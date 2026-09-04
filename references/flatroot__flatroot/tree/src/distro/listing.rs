//! Discovers the releases a distribution currently offers — several publish
//! theirs only as directories on their mirrors — and presents them as the
//! table a user sees when asking what they can install.

use std::collections::HashSet;

use anyhow::{Context, Result, bail};

use crate::mirror::{ListMirror, Mirror};
use crate::ui::RunVoice;

use super::TableRelease;

/// How a distribution's releases are listed: a built-in table, or a
/// live walk of its mirrors' directory listings — chosen once per
/// distribution so callers need not know which.
pub enum ReleaseSource {
  /// Built-in fixed table — for distros that publish no walkable
  /// enumeration endpoint (arch, cachyos, centos, opensuse).
  Static(TableRelease),
  /// Walk the mirror chain's HTML index with a family strategy
  /// (debian, ubuntu, alpine, fedora, alma, rocky).
  Walk {
    /// Mirror chain to consult, primary first.
    mirrors: ListMirror,
    /// The per-family walk shape applied to each mirror listing.
    strategy: WalkStrategy,
  },
}

impl ReleaseSource {
  /// Produces the release table: a `Static` table is returned as-is; a
  /// `Walk` scrapes the mirrors now.
  pub fn collect(self) -> Result<TableRelease> {
    match self {
      ReleaseSource::Static(table) => Ok(table),
      ReleaseSource::Walk { mirrors, strategy } => ReleaseScraper::new(&mirrors).collect(&strategy),
    }
  }
}

/// The per-family rules of a mirror walk: what counts as a release
/// directory and how releases are ordered and validated.
pub enum WalkStrategy {
  /// Debian-family codename walk (Debian, Ubuntu).
  DebCodename,
  /// RHEL major-version walk, emitting one row per major against
  /// the supplied short mirror label (Alma, Rocky).
  RhelMajor {
    /// Short mirror display label shown in every emitted row.
    mirror_short: &'static str,
  },
  /// Numeric RPM walk, appending `extras` rows for non-numeric
  /// releases the distro surfaces (Fedora's `rawhide`).
  RpmNumeric {
    /// Extra `(release, mirror)` rows appended after the numbered
    /// ones — releases that cannot be discovered by number.
    extras: &'static [(&'static str, &'static str)],
  },
  /// Alpine `vX.Y` + `edge` walk (Alpine).
  ApkVersion,
}

impl WalkStrategy {
  /// Whether a directory name is a release under this family's rules,
  /// filtering out aliases like `stable` and the non-release
  /// directories that sit beside real releases.
  fn accept(&self, name: &str) -> bool {
    match self {
      WalkStrategy::DebCodename => {
        const SKIP: &[&str] = &[
          "stable",
          "testing",
          "unstable",
          "sid",
          "experimental",
          "stable-updates",
          "stable-proposed-updates",
          "testing-proposed-updates",
          "proposed-updates",
          "stable-backports",
          "testing-backports",
          "oldstable",
          "oldoldstable",
          "Debian",
          "devel",
        ];
        if SKIP.contains(&name) {
          return false;
        }
        !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase())
      }
      WalkStrategy::RhelMajor { .. } | WalkStrategy::RpmNumeric { .. } => {
        !name.is_empty() && name.chars().all(|c| c.is_ascii_digit())
      }
      WalkStrategy::ApkVersion => {
        if name == "edge" {
          return true;
        }
        let Some(tail) = name.strip_prefix('v') else {
          return false;
        };
        !tail.is_empty()
          && tail.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
          && tail.chars().all(|c| c.is_ascii_digit() || c == '.')
      }
    }
  }

  /// A (major, minor) sort key for a versioned release name, so the
  /// table sorts numerically rather than lexically.
  fn sort_key(&self, name: &str) -> (u32, u32) {
    let t = name.trim_start_matches('v');
    let mut parts = t.split('.');
    let major: u32 = parts.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    let minor: u32 = parts.next().and_then(|x| x.parse().ok()).unwrap_or(0);
    (major, minor)
  }
}

/// Scrapes a distribution's releases from its mirrors' directory
/// listings, for distributions whose releases live only on their
/// mirrors.
pub struct ReleaseScraper<'a> {
  /// Mirror chain to consult, primary first.
  mirrors: &'a ListMirror,
}

impl<'a> ReleaseScraper<'a> {
  /// Binds the scraper to an ordered set of mirrors to walk.
  pub fn new(mirrors: &'a ListMirror) -> ReleaseScraper<'a> {
    ReleaseScraper { mirrors }
  }

  /// Scrapes the releases, dispatching to the family-specific walk
  /// `strategy` names; returns a deduplicated release-and-mirror table.
  pub fn collect(&self, strategy: &WalkStrategy) -> Result<TableRelease> {
    match strategy {
      WalkStrategy::DebCodename => self.collect_deb(strategy),
      WalkStrategy::RhelMajor { mirror_short } => self.collect_rhel_majors(strategy, mirror_short),
      WalkStrategy::RpmNumeric { extras } => self.collect_rpm_numeric(strategy, extras),
      WalkStrategy::ApkVersion => self.collect_apk(strategy),
    }
  }

  /// Discovers Debian-family releases: reads codenames from each mirror and
  /// confirms each publishes a real release before admitting it.
  fn collect_deb(&self, strategy: &WalkStrategy) -> Result<TableRelease> {
    eprintln!("Fetching releases...");

    let mut rows: Vec<(String, String)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    let mirror_iter: Vec<&dyn Mirror> = self.mirrors.iter().collect();
    for mirror in &mirror_iter {
      let text_html = mirror
        .fetch_listing("dists/")
        .with_context(|| format!("listing dists/ at {}", mirror.url_base()))?;

      let url_short = ReleaseScraper::mirror_label(mirror.url_base());

      for name_codename in self.listing_parse(&text_html, strategy) {
        if seen.contains(&name_codename) {
          continue;
        }
        if !self.codename_validate(*mirror, &name_codename)? {
          continue;
        }
        seen.insert(name_codename.clone());
        rows.push((name_codename, url_short.clone()));
      }
    }

    if rows.is_empty() {
      bail!("no Debian-family releases discovered — every mirror returned an empty listing");
    }

    rows.sort_by(|a, b| a.0.cmp(&b.0));

    let rows_display: Vec<Vec<String>> = rows
      .into_iter()
      .map(|(codename, mirror)| vec![codename, mirror])
      .collect();

    Ok(TableRelease {
      header: vec!["Codename", "Mirror"],
      rows: rows_display,
    })
  }

  /// Discovers RHEL-rebuild releases: the bare major versions the mirror
  /// publishes, one row each against the mirror label.
  fn collect_rhel_majors(&self, strategy: &WalkStrategy, mirror_short: &str) -> Result<TableRelease> {
    eprintln!("Fetching releases...");

    let first = self
      .mirrors
      .iter()
      .next()
      .ok_or_else(|| anyhow::anyhow!("RHEL-major scrape needs at least one mirror"))?;
    let text_html = first
      .fetch_listing("/")
      .with_context(|| format!("listing at {}", first.url_base()))?;

    let mut majors: Vec<u32> = self
      .listing_parse(&text_html, strategy)
      .into_iter()
      .filter_map(|name| name.parse().ok())
      .collect();
    majors.sort();
    majors.dedup();

    if majors.is_empty() {
      bail!("{}: no major-release directories discovered", first.url_base());
    }

    let rows: Vec<Vec<String>> = majors
      .iter()
      .map(|major| vec![major.to_string(), mirror_short.to_string()])
      .collect();

    Ok(TableRelease {
      header: vec!["Release", "Mirror"],
      rows,
    })
  }

  /// Discovers numbered RPM releases from the mirrors and appends the
  /// named extras that cannot be discovered by number (Fedora's
  /// `rawhide`).
  fn collect_rpm_numeric(&self, strategy: &WalkStrategy, extras: &[(&str, &str)]) -> Result<TableRelease> {
    eprintln!("Fetching releases...");

    let mut seen: HashSet<u32> = HashSet::new();
    let mut releases: Vec<(u32, String)> = Vec::new();

    for mirror in self.mirrors.iter() {
      let text_html = mirror
        .fetch_listing("/")
        .with_context(|| format!("listing root at {}", mirror.url_base()))?;
      let url_short = ReleaseScraper::mirror_label(mirror.url_base());
      for name in self.listing_parse(&text_html, strategy) {
        let num: u32 = match name.parse() {
          Ok(n) => n,
          Err(_) => continue,
        };
        if seen.insert(num) {
          releases.push((num, url_short.clone()));
        }
      }
    }

    releases.sort_by_key(|(num, _)| *num);

    // A reachable-but-empty listing must fail rather than answer with only
    // the static extras (such as `rawhide`): an empty numeric discovery is
    // never silently presented as a usable release set.
    if releases.is_empty() {
      bail!("no numbered releases discovered — every mirror returned an empty listing");
    }

    let mut rows: Vec<Vec<String>> = releases
      .into_iter()
      .map(|(num, mirror)| vec![num.to_string(), mirror])
      .collect();
    for (extra_name, extra_mirror) in extras {
      rows.push(vec![(*extra_name).to_string(), (*extra_mirror).to_string()]);
    }

    Ok(TableRelease {
      header: vec!["Release", "Mirror"],
      rows,
    })
  }

  /// Discovers Alpine's versioned releases and its rolling `edge`,
  /// confirming each serves the package indexes an install reads, and marks
  /// which is which.
  fn collect_apk(&self, strategy: &WalkStrategy) -> Result<TableRelease> {
    eprintln!("Fetching releases...");

    let first = self
      .mirrors
      .iter()
      .next()
      .ok_or_else(|| anyhow::anyhow!("APK scrape needs at least one mirror"))?;
    let text_html = first
      .fetch_listing("/")
      .with_context(|| format!("listing at {}", first.url_base()))?;

    let mut names = self.listing_parse(&text_html, strategy);
    let has_edge = names.iter().any(|n| n == "edge");
    names.retain(|n| n != "edge");
    names.sort_by_key(|v| strategy.sort_key(v));

    let mirror_label = ReleaseScraper::mirror_label(first.url_base());
    let mut rows = Vec::new();
    for name_release in &names {
      if !self.apk_validate(first, name_release)? {
        continue;
      }
      rows.push(vec![name_release.clone(), "stable".into(), mirror_label.clone()]);
    }
    if has_edge && self.apk_validate(first, "edge")? {
      rows.push(vec!["edge".into(), "rolling".into(), mirror_label.clone()]);
    }

    // A reachable listing that yields no version serving an installable
    // index must fail rather than exit silently with nothing.
    if rows.is_empty() {
      bail!("{}: no Alpine releases discovered — no published version serves an installable index", first.url_base());
    }

    Ok(TableRelease {
      header: vec!["Release", "Type", "Mirror"],
      rows,
    })
  }

  /// Extracts the directory names from a mirror's HTML listing, keeping
  /// only those `strategy` accepts, deduplicated.
  fn listing_parse(&self, text_html: &str, strategy: &WalkStrategy) -> Vec<String> {
    const HREF_OPEN: &str = "href=\"";
    let mut names = Vec::new();
    for line in text_html.lines() {
      let Some(start) = line.find(HREF_OPEN) else {
        continue;
      };
      let rest = &line[start + HREF_OPEN.len()..];
      let Some(end) = rest.find('/') else {
        continue;
      };
      let name = &rest[..end];
      if !name.is_empty() && strategy.accept(name) {
        names.push(name.to_string());
      }
    }
    names.sort();
    names.dedup();
    names
  }

  /// Confirms a plausible codename actually publishes its `Release`
  /// metadata, so only installable releases survive into the answer.
  fn codename_validate(&self, mirror: &dyn Mirror, name_codename: &str) -> Result<bool> {
    let path = format!("dists/{name_codename}/Release");
    match mirror.exists(&path) {
      Ok(b) => Ok(b),
      Err(e) => {
        RunVoice::warn(format!("HEAD failed for {name_codename} at {}: {e}", mirror.url_base()));
        Err(e)
      }
    }
  }

  /// Confirms an Alpine release serves both its main and community
  /// `APKINDEX` files, so only buildable releases are offered.
  fn apk_validate(&self, mirror: &dyn Mirror, name_release: &str) -> Result<bool> {
    let path_main = format!("{name_release}/main/x86_64/APKINDEX.tar.gz");
    if !mirror
      .exists(&path_main)
      .with_context(|| format!("HEAD failed for {} at {}", path_main, mirror.url_base()))?
    {
      return Ok(false);
    }
    let path_community = format!("{name_release}/community/x86_64/APKINDEX.tar.gz");
    if !mirror
      .exists(&path_community)
      .with_context(|| format!("HEAD failed for {} at {}", path_community, mirror.url_base()))?
    {
      return Ok(false);
    }
    Ok(true)
  }

  /// A mirror URL reduced to its host, used as the mirror column in the
  /// table.
  fn mirror_label(url: &str) -> String {
    let no_proto = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    no_proto.split('/').next().unwrap_or(no_proto).to_string()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// A `Mirror` whose directory listing, candidate existence, and
  /// reachability are all fixed by the test, so each walk strategy can
  /// be driven through a normal listing, a reachable-but-empty listing,
  /// and an unreachable mirror without touching the network.
  #[derive(Clone)]
  struct MockMirror {
    url: String,
    listing: String,
    exists: bool,
    listing_fails: bool,
  }

  impl MockMirror {
    fn new(listing: &str) -> Self {
      Self {
        url: "https://mock.test/repo".to_string(),
        listing: listing.to_string(),
        exists: true,
        listing_fails: false,
      }
    }

    fn with_exists(mut self, exists: bool) -> Self {
      self.exists = exists;
      self
    }

    fn unreachable() -> Self {
      Self {
        url: "https://mock.test/repo".to_string(),
        listing: String::new(),
        exists: false,
        listing_fails: true,
      }
    }
  }

  impl Mirror for MockMirror {
    fn fetch(&self, _rel_path: &str) -> Result<Vec<u8>> {
      anyhow::bail!("mock mirror does not serve content")
    }

    fn probe(&self, _rel_path: &str) -> Result<bool> {
      Ok(self.exists)
    }

    fn exists(&self, _rel_path: &str) -> Result<bool> {
      Ok(self.exists)
    }

    fn fetch_listing(&self, _rel_path: &str) -> Result<String> {
      if self.listing_fails {
        anyhow::bail!("mock mirror unreachable");
      }
      Ok(self.listing.clone())
    }

    fn url_base(&self) -> &str {
      &self.url
    }

    fn clone_box(&self) -> Box<dyn Mirror> {
      Box::new(self.clone())
    }
  }

  /// Render directory names as the anchor markup the listing parser
  /// reads, one `href="<name>/"` per entry.
  fn listing_html(names: &[&str]) -> String {
    names.iter().map(|n| format!("<a href=\"{n}/\">{n}/</a>\n")).collect()
  }

  fn scrape(mirror: MockMirror, strategy: &WalkStrategy) -> Result<TableRelease> {
    let mirrors = ListMirror::primary(Box::new(mirror));
    ReleaseScraper::new(&mirrors).collect(strategy)
  }

  // ── DebCodename ───────────────────────────────────────────────────────

  #[test]
  fn deb_codename_normal_listing_yields_rows() {
    let mirror = MockMirror::new(&listing_html(&["bookworm", "bullseye", "stable"]));
    let table = scrape(mirror, &WalkStrategy::DebCodename).unwrap();
    // `stable` is a lifecycle alias filtered by `accept`; the two real
    // codenames survive, validated by the mock's `exists`.
    assert_eq!(table.header, vec!["Codename", "Mirror"]);
    assert_eq!(table.rows.len(), 2);
  }

  #[test]
  fn deb_codename_empty_listing_is_error() {
    let err = scrape(MockMirror::new(""), &WalkStrategy::DebCodename).unwrap_err();
    assert!(err.to_string().contains("no Debian-family releases discovered"));
  }

  #[test]
  fn deb_codename_unreachable_mirror_is_fetch_error() {
    let err = scrape(MockMirror::unreachable(), &WalkStrategy::DebCodename).unwrap_err();
    assert!(err.to_string().contains("listing dists/"));
  }

  // ── RhelMajor (Alma, Rocky) ───────────────────────────────────────────

  #[test]
  fn rhel_major_normal_listing_yields_rows() {
    let mirror = MockMirror::new(&listing_html(&["8", "9", "SRPMS"]));
    let table = scrape(mirror, &WalkStrategy::RhelMajor { mirror_short: "vault" }).unwrap();
    assert_eq!(table.rows, vec![vec!["8", "vault"], vec!["9", "vault"]]);
  }

  #[test]
  fn rhel_major_empty_listing_is_error() {
    let err = scrape(MockMirror::new(""), &WalkStrategy::RhelMajor { mirror_short: "vault" }).unwrap_err();
    assert!(err.to_string().contains("no major-release directories discovered"));
  }

  #[test]
  fn rhel_major_unreachable_mirror_is_fetch_error() {
    let err = scrape(MockMirror::unreachable(), &WalkStrategy::RhelMajor { mirror_short: "vault" }).unwrap_err();
    assert!(err.to_string().contains("listing at"));
  }

  // ── RpmNumeric (Fedora) ───────────────────────────────────────────────

  const FEDORA_EXTRAS: &[(&str, &str)] = &[("rawhide", "dl.fedoraproject.org")];

  #[test]
  fn rpm_numeric_normal_listing_yields_rows_plus_extras() {
    let mirror = MockMirror::new(&listing_html(&["40", "41"]));
    let table = scrape(mirror, &WalkStrategy::RpmNumeric { extras: FEDORA_EXTRAS }).unwrap();
    // Two discovered numeric releases plus the appended `rawhide` extra.
    assert_eq!(table.rows.len(), 3);
    assert_eq!(table.rows[2][0], "rawhide");
  }

  /// An empty numeric discovery must fail rather than answer with only
  /// the unprobed `rawhide` extra.
  #[test]
  fn rpm_numeric_empty_listing_is_error_not_rawhide_only() {
    let err = scrape(MockMirror::new(""), &WalkStrategy::RpmNumeric { extras: FEDORA_EXTRAS }).unwrap_err();
    assert!(err.to_string().contains("no numbered releases discovered"));
  }

  #[test]
  fn rpm_numeric_unreachable_mirror_is_fetch_error() {
    let err = scrape(MockMirror::unreachable(), &WalkStrategy::RpmNumeric { extras: FEDORA_EXTRAS }).unwrap_err();
    assert!(err.to_string().contains("listing root at"));
  }

  // ── ApkVersion (Alpine) ───────────────────────────────────────────────

  #[test]
  fn apk_version_normal_listing_yields_rows() {
    let mirror = MockMirror::new(&listing_html(&["v3.20", "v3.21", "edge"]));
    let table = scrape(mirror, &WalkStrategy::ApkVersion).unwrap();
    assert_eq!(table.header, vec!["Release", "Type", "Mirror"]);
    // Two stable versions plus edge, all validated by the mock.
    assert_eq!(table.rows.len(), 3);
    assert_eq!(table.rows[2][1], "rolling");
  }

  /// An empty listing must fail rather than exit silently with zero
  /// rows.
  #[test]
  fn apk_version_empty_listing_is_error() {
    let err = scrape(MockMirror::new(""), &WalkStrategy::ApkVersion).unwrap_err();
    assert!(err.to_string().contains("no Alpine releases discovered"));
  }

  /// A reachable listing whose every candidate fails index validation
  /// is just as empty an answer, and must fail too.
  #[test]
  fn apk_version_all_candidates_unvalidated_is_error() {
    let mirror = MockMirror::new(&listing_html(&["v3.20", "v3.21"])).with_exists(false);
    let err = scrape(mirror, &WalkStrategy::ApkVersion).unwrap_err();
    assert!(err.to_string().contains("no Alpine releases discovered"));
  }

  #[test]
  fn apk_version_unreachable_mirror_is_fetch_error() {
    let err = scrape(MockMirror::unreachable(), &WalkStrategy::ApkVersion).unwrap_err();
    assert!(err.to_string().contains("listing at"));
  }
}
