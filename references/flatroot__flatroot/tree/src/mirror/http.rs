//! A network package source addressed by a base location: the live
//! archive that serves the current contents, or a frozen date-pinned
//! snapshot. A snapshot adds only a human-facing label of the day it was
//! pinned to; the day is already baked into where it points.

use std::sync::Arc;

use anyhow::{Context, Result};

use crate::internal::http::{HttpClient, HttpError};

use super::Mirror;

/// One network source: where the packages live, the transport that
/// reaches them, and whether this is a frozen snapshot or the live archive.
#[derive(Clone)]
pub struct HttpMirror {
  /// HTTPS base URL; the slash is normalised at join time. For a snapshot
  /// this already encodes the date.
  url_base: String,
  /// The user-supplied `@<date>` for a snapshot, for diagnostics only;
  /// absent for a live mirror.
  date: Option<String>,
  /// Shared client carrying the retry budget and index cache; cheap to
  /// clone (one client shared by reference).
  http: Arc<HttpClient>,
}

impl HttpMirror {
  /// A live mirror at `url_base`, serving the archive's current
  /// contents.
  pub fn live(url_base: impl Into<String>, http: Arc<HttpClient>) -> Self {
    Self {
      url_base: url_base.into(),
      date: None,
      http,
    }
  }

  /// A frozen point-in-time copy as a source, so a build reproduces a past
  /// day. The date is remembered only for self-description; the location
  /// already resolves to that day.
  pub fn snapshot(url_base: impl Into<String>, date: impl Into<String>, http: Arc<HttpClient>) -> Self {
    Self {
      url_base: url_base.into(),
      date: Some(date.into()),
      http,
    }
  }

  /// The day a snapshot is pinned to, for diagnostics; `None` for the live
  /// archive.
  pub fn date(&self) -> Option<&str> {
    self.date.as_deref()
  }

  /// Joins the base URL and a relative path into one full URL, with
  /// exactly one separator however each half carries it.
  fn url_join(&self, rel: &str) -> String {
    format!("{}/{}", self.url_base.trim_end_matches('/'), rel.trim_start_matches('/'))
  }
}

impl Mirror for HttpMirror {
  fn fetch(&self, rel_path: &str) -> Result<Vec<u8>> {
    let url_target = self.url_join(rel_path);
    self
      .http
      .get_cached(&url_target)
      .with_context(|| format!("fetch {url_target}"))
  }

  fn probe(&self, rel_path: &str) -> Result<bool> {
    let url_target = self.url_join(rel_path);
    match self.http.get_cached(&url_target) {
      Ok(_) => Ok(true),
      // A definitive 404 answers the probe's question — the resource is
      // absent — recognised by the typed status the fetch carries rather
      // than by sniffing formatted error text.
      Err(e)
        if e
          .downcast_ref::<HttpError>()
          .is_some_and(|h| h.status == reqwest::StatusCode::NOT_FOUND) =>
      {
        Ok(false)
      }
      Err(e) => Err(e).with_context(|| format!("probe {url_target}")),
    }
  }

  fn exists(&self, rel_path: &str) -> Result<bool> {
    let url_target = self.url_join(rel_path);
    self
      .http
      .head(&url_target)
      .with_context(|| format!("exists {url_target}"))
  }

  fn fetch_listing(&self, rel_path: &str) -> Result<String> {
    let url_target = self.url_join(rel_path);
    let bytes = self
      .http
      .get_fresh(&url_target)
      .with_context(|| format!("listing {url_target}"))?;
    String::from_utf8(bytes).with_context(|| format!("listing at {url_target} not utf-8"))
  }

  fn url_base(&self) -> &str {
    &self.url_base
  }

  fn clone_box(&self) -> Box<dyn Mirror> {
    Box::new(self.clone())
  }
}
