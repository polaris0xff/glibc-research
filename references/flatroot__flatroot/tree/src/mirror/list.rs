//! `ListMirror`: several equivalent mirrors treated as one source — a
//! fetch fails only when none can answer, and a healthy fallback covers
//! for an unreachable preferred mirror.

use anyhow::{Context, Result, bail};

use crate::internal::http::HttpError;

use super::Mirror;

/// Several interchangeable mirrors behind one fetch interface, in
/// preference order — the first preferred, the rest fallbacks.
pub struct ListMirror {
  /// Mirrors in preference order — `mirrors[0]` is canonical, later
  /// entries are fallbacks tried in turn.
  mirrors: Vec<Box<dyn Mirror>>,
}

impl ListMirror {
  /// Builds the list in the caller's preference order — first
  /// preferred, the rest fallbacks.
  pub fn new(mirrors: Vec<Box<dyn Mirror>>) -> Self {
    Self { mirrors }
  }

  /// A list holding a single mirror, so callers always work through
  /// `ListMirror`.
  pub fn primary(mirror: Box<dyn Mirror>) -> Self {
    Self { mirrors: vec![mirror] }
  }

  /// Fetches `rel_path`, succeeding as long as any mirror can serve it.
  pub fn fetch(&self, rel_path: &str) -> Result<Vec<u8>> {
    let (bytes, _) = self.fetch_with_origin(rel_path)?;
    Ok(bytes)
  }

  /// The content plus which source served it. For a package index the
  /// origin must travel with the body, since the entries it lists are
  /// downloaded later from that same source, not the preferred one.
  pub fn fetch_with_origin(&self, rel_path: &str) -> Result<(Vec<u8>, String)> {
    let mut err_last: Option<anyhow::Error> = None;
    for mirror in &self.mirrors {
      match mirror.fetch(rel_path) {
        Ok(bytes_body) => return Ok((bytes_body, mirror.url_base().to_string())),
        Err(e) => err_last = Some(e),
      }
    }
    bail!(
      "all {} mirror(s) failed for {}: {}",
      self.mirrors.len(),
      rel_path,
      err_last.map(|e| e.to_string()).unwrap_or_default()
    )
  }

  /// Like `fetch_with_origin` but tolerating absence: `None` only when
  /// every source answered a definitive not-found, since an unreachable
  /// mirror says nothing about whether the content exists.
  pub fn fetch_with_origin_optional(&self, rel_path: &str) -> Result<Option<(Vec<u8>, String)>> {
    let mut err_last: Option<anyhow::Error> = None;
    let mut all_not_found = true;
    for mirror in &self.mirrors {
      match mirror.fetch(rel_path) {
        Ok(bytes_body) => return Ok(Some((bytes_body, mirror.url_base().to_string()))),
        Err(e) => {
          all_not_found = all_not_found
            && e
              .downcast_ref::<HttpError>()
              .is_some_and(|h| h.status == reqwest::StatusCode::NOT_FOUND);
          err_last = Some(e);
        }
      }
    }
    if all_not_found && err_last.is_some() {
      return Ok(None);
    }
    bail!(
      "all {} mirror(s) failed for {}: {}",
      self.mirrors.len(),
      rel_path,
      err_last.map(|e| e.to_string()).unwrap_or_default()
    )
  }

  /// Visits every source's content, treating them as distinct contributors
  /// rather than substitutes, so one unreachable source fails the request.
  pub fn fetch_all<F>(&self, rel_path: &str, mut visit: F) -> Result<()>
  where
    F: FnMut(&dyn Mirror, Vec<u8>) -> Result<()>,
  {
    for mirror in &self.mirrors {
      let bytes_body = mirror
        .fetch(rel_path)
        .with_context(|| format!("listing {} at {}", rel_path, mirror.url_base()))?;
      visit(mirror.as_ref(), bytes_body)?;
    }
    Ok(())
  }

  /// Visits every source's directory listing, so any source that cannot
  /// produce its listing fails the request rather than narrowing the
  /// picture.
  pub fn fetch_listings<F>(&self, rel_path: &str, mut visit_listing: F) -> Result<()>
  where
    F: FnMut(&dyn Mirror, String) -> Result<()>,
  {
    for mirror in &self.mirrors {
      let text = mirror
        .fetch_listing(rel_path)
        .with_context(|| format!("listing {} at {}", rel_path, mirror.url_base()))?;
      visit_listing(mirror.as_ref(), text)?;
    }
    Ok(())
  }

  /// The first mirror that can serve `rel_path`, probed without
  /// fetching, for a caller that must commit to one mirror ahead of
  /// time.
  pub fn select(&self, rel_path: &str) -> Result<&dyn Mirror> {
    for mirror in &self.mirrors {
      if mirror.probe(rel_path)? {
        return Ok(mirror.as_ref());
      }
    }
    bail!("no mirror serves {}", rel_path)
  }

  /// The sources in preference order, for callers reasoning about them
  /// individually.
  pub fn iter(&self) -> impl Iterator<Item = &dyn Mirror> {
    self.mirrors.iter().map(|m| m.as_ref())
  }
}

impl Clone for ListMirror {
  fn clone(&self) -> Self {
    Self {
      mirrors: self.mirrors.iter().map(|m| m.clone_box()).collect(),
    }
  }
}

impl std::fmt::Debug for ListMirror {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ListMirror")
      .field("mirrors", &self.mirrors.iter().map(|m| m.url_base()).collect::<Vec<_>>())
      .finish()
  }
}
