//! Downloads a resolved package set's archives — verified against the
//! index's recorded checksums, fetched in parallel, and reused from a
//! local cache.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use futures::StreamExt;

use crate::db::Index;
use crate::internal::byte_size::ByteSize;
use crate::internal::fs::Fs;
use crate::manifest::Checksum;
use crate::remote::Remote;

/// One package's pending download: the URL, the cache path the archive
/// lands at, and the recorded checksum and size the result is verified
/// against. The checksum is carried in the encoding the source
/// published; `Checksum::infer` recovers the algorithm at verification
/// time.
struct DownloadJob {
  name_pkg: String,
  url: String,
  path_file_cache: PathBuf,
  checksum: String,
  size: u64,
}

impl DownloadJob {
  /// Whether the cache already holds this package's genuine archive: the file
  /// exists and its bytes agree with the recorded checksum. A hit is announced
  /// on the progress bar; a mismatch counts as a miss, sending the job to the
  /// download phase for a fresh copy. A package with no recorded checksum is
  /// always refetched, since a cached copy can never be proven genuine.
  fn cached_verify(&self, pb: &indicatif::ProgressBar) -> Result<bool> {
    if !self.path_file_cache.exists() || self.checksum.is_empty() {
      return Ok(false);
    }
    if !Checksum::infer(&self.checksum).verify(&self.path_file_cache)? {
      return Ok(false);
    }
    pb.set_message(format!("cached {}", self.name_pkg));
    pb.inc(1);
    Ok(true)
  }
}

/// The verdict of one transfer attempt: the file is fully on disk, or a
/// transient network failure worth retrying. Local filesystem errors are not
/// verdicts — they propagate immediately, since retrying cannot fix them.
enum StreamOutcome {
  Done,
  Retryable(String),
}

/// One build's downloader: HTTP client, cache directory, retry count,
/// and parallelism.
pub struct Downloader<'a> {
  /// Connection-pooled HTTP client, cheaply cloned into each task
  /// (clones share one connection pool).
  client: reqwest::Client,
  /// The package index each package's row is looked up in by name.
  index: &'a Index,
  /// The active `Remote`, turning a package filename into a download
  /// URL.
  remote: &'a dyn Remote,
  /// The per-source cache directory fetched archives land in and are
  /// reused from.
  path_dir_cache: &'a Path,
  /// How many extra attempts a failed transfer gets.
  retries: u32,
  /// Maximum archive downloads run in parallel.
  jobs: usize,
}

impl<'a> Downloader<'a> {
  /// Builds a downloader over the given index, remote, cache directory,
  /// retry count, and parallelism.
  pub fn new(
    index: &'a Index,
    remote: &'a dyn Remote,
    path_dir_cache: &'a Path,
    retries: u32,
    jobs: usize,
  ) -> Result<Self> {
    // Built once and cloned into each task; clones share the pool.
    let client = reqwest::Client::builder().build()?;
    Ok(Self {
      client,
      index,
      remote,
      path_dir_cache,
      retries,
      jobs,
    })
  }

  /// Downloads every requested package to a verified archive, reusing
  /// cached copies whose checksum still matches and fetching the rest
  /// in parallel. An archive that cannot be made to match its checksum
  /// is a hard failure, since corrupt content must not enter the
  /// rootfs.
  pub async fn fetch(&self, names: &[String], pb: &indicatif::ProgressBar) -> Result<HashMap<String, PathBuf>> {
    let mut downloaded = HashMap::new();

    // Cached hits are handled immediately; misses become download jobs.
    let mut to_download: Vec<DownloadJob> = Vec::new();
    for pkg_name in names {
      let job = self.job_build(pkg_name)?;
      if job.cached_verify(pb)? {
        downloaded.insert(job.name_pkg, job.path_file_cache);
        continue;
      }
      to_download.push(job);
    }

    if to_download.is_empty() {
      return Ok(downloaded);
    }

    // buffer_unordered runs up to `jobs` futures at once, completing them in
    // whatever order they finish (not in input order).
    let jobs = self.jobs.max(1);
    let results: Vec<Result<(String, PathBuf)>> = futures::stream::iter(to_download.into_iter().map(|job| {
      let pb = pb.clone();
      async move { self.job_fetch(job, &pb).await }
    }))
    .buffer_unordered(jobs)
    .collect()
    .await;

    for result in results {
      let (name_pkg, path_file_cache) = result?;
      downloaded.insert(name_pkg, path_file_cache);
    }

    Ok(downloaded)
  }

  /// The `DownloadJob` for one resolved package: its index row looked
  /// up, its URL derived, and its cache path fixed (the basename of the
  /// repo path: "pool/main/b/bash/bash_5.2.deb" → "bash_5.2.deb").
  fn job_build(&self, pkg_name: &str) -> Result<DownloadJob> {
    let pkg = self
      .index
      .packages()
      .get(pkg_name)?
      .ok_or_else(|| anyhow::anyhow!("Package '{}' not found in database", pkg_name))?;
    let url = self.remote.download_url(&pkg.filename)?;
    let path_file_cache = self
      .path_dir_cache
      .join(pkg.filename.rsplit('/').next().unwrap_or(&pkg.name));
    Ok(DownloadJob {
      name_pkg: pkg_name.to_string(),
      url,
      path_file_cache,
      checksum: pkg.checksum,
      size: pkg.size,
    })
  }

  /// Downloads one package and verifies it. A checksum mismatch is
  /// retried once — a CDN can briefly serve a stale file against a
  /// newer index entry — and a copy that still disagrees is a hard
  /// failure, because corrupt content must never enter the rootfs.
  async fn job_fetch(&self, job: DownloadJob, pb: &indicatif::ProgressBar) -> Result<(String, PathBuf)> {
    pb.set_message(format!("downloading {} ({})", job.name_pkg, ByteSize(job.size)));
    self.stream_to(&job.url, &job.path_file_cache, &job.name_pkg).await?;

    if !self.checksum_settle(&job, pb).await? {
      bail!("Checksum mismatch for {} after retry: expected {}", job.name_pkg, job.checksum);
    }
    pb.inc(1);
    Ok((job.name_pkg, job.path_file_cache))
  }

  /// Verifies a downloaded archive against its recorded checksum,
  /// refetching once on mismatch. `true` when the bytes match (or no
  /// checksum was recorded to check against); `false` when even the
  /// retry disagreed.
  async fn checksum_settle(&self, job: &DownloadJob, pb: &indicatif::ProgressBar) -> Result<bool> {
    if job.checksum.is_empty() || Checksum::infer(&job.checksum).verify(&job.path_file_cache)? {
      return Ok(true);
    }
    Fs::remove_lenient(&job.path_file_cache);
    pb.println(format!("  retrying: {} (checksum mismatch)", job.name_pkg));
    self.stream_to(&job.url, &job.path_file_cache, &job.name_pkg).await?;

    if Checksum::infer(&job.checksum).verify(&job.path_file_cache)? {
      return Ok(true);
    }
    Fs::remove_lenient(&job.path_file_cache);
    Ok(false)
  }

  /// Fetches one archive to disk, retrying transient failures up to the
  /// retry count, each attempt starting from a fresh file so a partial
  /// copy never contaminates the next. The checksum check happens
  /// afterwards.
  async fn stream_to(&self, url: &str, path_file_dest: &Path, name_display: &str) -> Result<()> {
    // One initial try plus `retries` additional attempts, so a single
    // archive is always fetched at least once and `retries == 0` fails
    // on the first stumble.
    let attempts_total = self.retries + 1;
    for attempt in 0..=self.retries {
      match self.stream_attempt(url, path_file_dest).await? {
        StreamOutcome::Done => return Ok(()),
        StreamOutcome::Retryable(reason) if attempt < self.retries => {
          eprintln!("  warning: {} for {}, retry {}/{}...", reason, name_display, attempt + 1, self.retries);
        }
        StreamOutcome::Retryable(reason) => {
          bail!("Failed to download {}: {} after {} attempts", name_display, reason, attempts_total);
        }
      }
    }
    bail!("Failed to download {} after {} attempts", name_display, attempts_total)
  }

  /// One transfer attempt, started clean: send the request, check the status,
  /// stream the body to disk chunk by chunk. A transient failure names the
  /// stage that broke so the retry warning can say so.
  async fn stream_attempt(&self, url: &str, path_file_dest: &Path) -> Result<StreamOutcome> {
    let response = match self.client.get(url).send().await {
      Ok(r) => r,
      Err(e) => return Ok(StreamOutcome::Retryable(format!("download failed ({e})"))),
    };
    if !response.status().is_success() {
      return Ok(StreamOutcome::Retryable(format!("HTTP {}", response.status())));
    }

    let mut file = fs::File::create(path_file_dest)?;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
      match chunk {
        Ok(data) => file.write_all(&data)?,
        Err(e) => return Ok(StreamOutcome::Retryable(format!("stream error ({e})"))),
      }
    }
    Ok(StreamOutcome::Done)
  }
}
