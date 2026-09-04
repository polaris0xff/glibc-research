//! Domain-neutral HTTP retrieval: fetch a remote body with bounded
//! retries, serve recent answers from a local cache, and probe whether a
//! resource exists before paying to fetch it.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};

use crate::ui::RunVoice;

/// A non-success HTTP status as a typed error, so a caller that must
/// react to a specific status — the mirror probe treats 404 as an
/// answer, not a failure — can downcast instead of parsing error text.
#[derive(Debug)]
pub struct HttpError {
  /// The status the server answered with.
  pub status: reqwest::StatusCode,
}

impl std::fmt::Display for HttpError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "HTTP {}", self.status)
  }
}

impl std::error::Error for HttpError {}

/// The run's HTTP settings — retry count, cache directory, and cache
/// TTL — built once so every fetch follows the same rules.
pub struct HttpClient {
  /// How many extra attempts a failed request gets.
  retries: u32,
  /// Directory cached bodies are written into, keyed by a digest of the URL.
  path_dir_cache: PathBuf,
  /// How long a cached body is served before a refetch.
  ttl: Duration,
}

impl HttpClient {
  /// Cache TTL for index bodies: one hour bounds staleness while
  /// keeping repeat runs against the same source near-instant.
  pub const INDEX_CACHE_TTL: Duration = Duration::from_secs(3600);

  /// Builds the client with its retry count, cache directory, and cache
  /// TTL.
  pub fn new(path_dir_cache: PathBuf, retries: u32, ttl: Duration) -> Self {
    Self {
      retries,
      path_dir_cache,
      ttl,
    }
  }

  /// Builds a one-shot blocking client: no ambient proxy, and no overall
  /// timeout, since a blanket deadline would sever a healthy download of
  /// a tens-of-megabytes listing partway through.
  fn transport() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
      .no_proxy()
      .build()
      .context("Failed to build HTTP transport")
  }

  /// Returns a cached body while it is within the TTL, otherwise fetches
  /// fresh and remembers it, so a second run against an unchanged source
  /// is near-instant.
  pub fn get_cached(&self, url: &str) -> Result<Vec<u8>> {
    let display_name = url.rsplit('/').next().unwrap_or(url);

    std::fs::create_dir_all(&self.path_dir_cache)?;
    let cache_file = self.path_file_cache(url);

    if let Ok(metadata) = cache_file.metadata()
      && let Ok(modified) = metadata.modified()
      && let Ok(age) = modified.elapsed()
      && age < self.ttl
    {
      eprintln!("  cached: {}", display_name);
      return std::fs::read(&cache_file).with_context(|| format!("Failed to read cached index for {}", url));
    }

    eprintln!("  fetching {}", url);
    let bytes = self.get_fresh(url)?;

    // Caching is best-effort — a failed write must not fail the fetch, but it
    // is worth a diagnostic because every later run pays the network cost again.
    if let Err(e) = std::fs::write(&cache_file, &bytes) {
      RunVoice::warn(format!("could not cache {}: {}", display_name, e));
    }

    Ok(bytes)
  }

  /// Fetches a body unconditionally, bypassing the cache, retrying a
  /// transfer that fails to start or breaks off partway, up to the
  /// retry count, so one transient failure does not sink the build.
  pub fn get_fresh(&self, url: &str) -> Result<Vec<u8>> {
    let display_name = url.rsplit('/').next().unwrap_or(url);
    let client = Self::transport()?;

    // One initial try plus `retries` additional attempts, so the run
    // always makes at least one request and `retries == 0` fails on the
    // first error.
    let attempts_total = self.retries + 1;
    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 0..=self.retries {
      let response = match client.get(url).send() {
        Ok(resp) if resp.status().is_success() => resp,
        Ok(resp) => {
          let status = resp.status();
          if attempt == self.retries {
            return Err(HttpError { status })
              .with_context(|| format!("Failed to fetch {} after {} attempts", url, attempts_total));
          }
          eprintln!("  warning: HTTP {} for {}, retry {}/{}...", status, url, attempt + 1, self.retries);
          continue;
        }
        Err(e) => {
          if attempt == self.retries {
            return Err(e).with_context(|| format!("Failed to fetch {} after {} attempts", url, attempts_total));
          }
          eprintln!("  warning: fetch failed for {}, retry {}/{}...", url, attempt + 1, self.retries);
          last_err = Some(e.into());
          continue;
        }
      };

      let pb = Self::progress_for(response.content_length(), display_name);
      let result = Self::body_stream(response, &pb);
      pb.finish_and_clear();

      match result {
        Ok(body) => return Ok(body),
        Err(e) if attempt == self.retries => {
          return Err(anyhow::anyhow!("Failed to download {} after {} attempts: {}", url, attempts_total, e));
        }
        Err(e) => {
          eprintln!("  warning: download interrupted for {}, retry {}/{}...", display_name, attempt + 1, self.retries);
          last_err = Some(e.into());
        }
      }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("Failed to fetch {}", url)))
  }

  /// The live download display: a bounded bar when the source declares a
  /// length, an unbounded byte counter otherwise. The variable-width `{msg}`
  /// goes to the end so the bar's left edge stays at a fixed column; the
  /// middle `{bytes}/{total_bytes}` width still shifts as the unit changes,
  /// but the bar itself is anchored.
  fn progress_for(total_size: Option<u64>, display_name: &str) -> ProgressBar {
    let pb = match total_size {
      Some(size) => {
        let pb = ProgressBar::new(size);
        pb.set_style(
          ProgressStyle::default_bar()
            .template("  [{bar:30}] {bytes}/{total_bytes} ({bytes_per_sec}) {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("=> "),
        );
        pb
      }
      None => {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
          ProgressStyle::default_spinner()
            .template("  {bytes} ({bytes_per_sec}) {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        pb
      }
    };
    pb.set_message(display_name.to_string());
    pb
  }

  /// Pull the whole body down in 8 KiB chunks, feeding the live display as
  /// bytes arrive.
  fn body_stream(mut response: reqwest::blocking::Response, pb: &ProgressBar) -> std::io::Result<Vec<u8>> {
    let mut body = Vec::new();
    if let Some(size) = response.content_length() {
      body.reserve(size as usize);
    }
    let mut buf = [0u8; 8192];
    loop {
      match std::io::Read::read(&mut response, &mut buf) {
        Ok(0) => return Ok(body),
        Ok(n) => {
          body.extend_from_slice(&buf[..n]);
          pb.inc(n as u64);
        }
        Err(e) => return Err(e),
      }
    }
  }

  /// Probes whether a resource exists without fetching it: a success
  /// status is yes, a definitive 404 is no, and an inconclusive reply
  /// is retried up to the retry count before counting as a failure.
  pub fn head(&self, url: &str) -> Result<bool> {
    let client = Self::transport()?;
    // One initial try plus `retries` additional attempts.
    let attempts_total = self.retries + 1;
    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 0..=self.retries {
      match client.head(url).send() {
        Ok(resp) if resp.status().is_success() => return Ok(true),
        Ok(resp) if resp.status() == reqwest::StatusCode::NOT_FOUND => return Ok(false),
        Ok(resp) => {
          let status = resp.status();
          if attempt == self.retries {
            bail!("HEAD {} returned HTTP {} after {} attempts", url, status, attempts_total);
          }
          eprintln!("  warning: HEAD HTTP {} for {}, retry {}/{}...", status, url, attempt + 1, self.retries);
          continue;
        }
        Err(e) => {
          if attempt == self.retries {
            return Err(e).with_context(|| format!("HEAD {} after {} attempts", url, attempts_total));
          }
          eprintln!("  warning: HEAD failed for {}, retry {}/{}...", url, attempt + 1, self.retries);
          last_err = Some(e.into());
          continue;
        }
      }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("HEAD {} failed", url)))
  }

  /// The configured retry count, so other network work (parallel
  /// package downloads) retries the same number of times.
  pub fn retries(&self) -> u32 {
    self.retries
  }

  /// Maps a URL to its one cache path via a digest of the URL: stable, so
  /// a later run finds what an earlier one saved, and collision-proof.
  fn path_file_cache(&self, url: &str) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let hash = hex::encode(hasher.finalize());
    self.path_dir_cache.join(hash)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::{ErrorKind, Write};
  use std::net::TcpListener;
  use std::sync::Arc;
  use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

  /// Stand up a local server that answers every request with `503` and
  /// closes the connection, counting how many connections the client
  /// opened. `503` is a retryable non-success and the close forces a
  /// fresh connection per attempt, so the accept count is exactly the
  /// number of attempts the client made. Never hangs: the server polls
  /// with a stop flag the client raises once it returns.
  fn attempts_count(retries: u32, run: impl FnOnce(&HttpClient, &str)) -> usize {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    listener.set_nonblocking(true).expect("nonblocking listener");
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{}/resource", addr);

    let count = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let count_srv = count.clone();
    let stop_srv = stop.clone();

    let server = std::thread::spawn(move || {
      loop {
        match listener.accept() {
          Ok((mut stream, _)) => {
            count_srv.fetch_add(1, Ordering::SeqCst);
            let _ =
              stream.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
          }
          Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
            if stop_srv.load(Ordering::SeqCst) {
              break;
            }
            std::thread::sleep(Duration::from_millis(5));
          }
          Err(_) => break,
        }
      }
    });

    let tmp = tempfile::tempdir().unwrap();
    let client = HttpClient::new(tmp.path().to_path_buf(), retries, HttpClient::INDEX_CACHE_TTL);
    run(&client, &url);

    stop.store(true, Ordering::SeqCst);
    server.join().unwrap();
    count.load(Ordering::SeqCst)
  }

  #[test]
  fn get_fresh_makes_one_attempt_when_no_retries() {
    let attempts = attempts_count(0, |client, url| {
      assert!(client.get_fresh(url).is_err());
    });
    assert_eq!(attempts, 1, "retries=0 must still make exactly one attempt");
  }

  #[test]
  fn get_fresh_attempts_initial_plus_retries() {
    let attempts = attempts_count(2, |client, url| {
      assert!(client.get_fresh(url).is_err());
    });
    assert_eq!(attempts, 3, "retries=2 must make one initial attempt plus two retries");
  }

  #[test]
  fn head_makes_one_attempt_when_no_retries() {
    let attempts = attempts_count(0, |client, url| {
      assert!(client.head(url).is_err());
    });
    assert_eq!(attempts, 1, "retries=0 must still make exactly one HEAD attempt");
  }

  #[test]
  fn head_attempts_initial_plus_retries() {
    let attempts = attempts_count(3, |client, url| {
      assert!(client.head(url).is_err());
    });
    assert_eq!(attempts, 4, "retries=3 must make one initial attempt plus three retries");
  }
}
