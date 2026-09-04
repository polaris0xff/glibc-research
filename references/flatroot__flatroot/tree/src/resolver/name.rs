//! Resolves a requested name to a concrete package: a real package
//! name, a virtual name some package provides, or a file path some
//! package owns.

use anyhow::Result;

use super::DepWalker;

impl DepWalker<'_> {
  /// Resolves `name` to the one package that satisfies it, trying in
  /// order:
  /// - a real package with that name
  /// - the first provider of the virtual name
  /// - the owner of the file path
  ///
  /// `None` when nothing matches.
  pub(super) fn name_resolve(&self, name: &str) -> Result<Option<String>> {
    if self.cfg.index.packages().exists(name)? {
      return Ok(Some(name.to_string()));
    }
    if let Some(provider) = self.provider_first(name)? {
      return Ok(Some(provider));
    }
    self.path_owner_get(name)
  }

  /// The first provider of a virtual name, chosen by a fixed ordering
  /// so the same provider wins every run.
  pub(super) fn provider_first(&self, name: &str) -> Result<Option<String>> {
    self.cfg.index.providers().first(name)
  }

  /// Every candidate that could satisfy `name`, in preference order:
  /// the package itself, else every provider of the virtual name, else
  /// the file path's owner.
  pub(super) fn candidates_get(&self, name: &str) -> Result<Vec<String>> {
    if self.cfg.index.packages().exists(name)? {
      return Ok(vec![name.to_string()]);
    }
    let providers = self.cfg.index.providers().of(name)?;
    if !providers.is_empty() {
      return Ok(providers.into_iter().map(|p| p.package_name).collect());
    }
    Ok(self.path_owner_get(name)?.into_iter().collect())
  }

  /// The owner of a dependency spelled as a file path (`/usr/bin/foo`),
  /// looked up in the path index when this source publishes one. `None`
  /// when the name is not a path, no path index exists, or nothing owns
  /// the file.
  fn path_owner_get(&self, name: &str) -> Result<Option<String>> {
    if !name.starts_with('/') {
      return Ok(None);
    }
    let Some(index) = self.cfg.index.paths()? else {
      return Ok(None);
    };
    index.query(name)
  }

  /// The version `provider_name` declares for `virtual_name` — often
  /// not its own version — so a constraint is checked against what the
  /// provides entry actually promises.
  pub(super) fn provides_version_get(&self, virtual_name: &str, provider_name: &str) -> Result<Option<String>> {
    self.cfg.index.providers().version(virtual_name, provider_name)
  }
}
