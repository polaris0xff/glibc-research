//! Adds a triggered package and its full hard-dependency closure as one
//! unit: either the whole closure joins, or the walker state is left
//! exactly as it was.

use std::collections::VecDeque;

use anyhow::Result;

use crate::package::DepKind;

use super::DepWalker;

impl DepWalker<'_> {
  /// Adds a required (rich-dep) package with its full closure; any
  /// unsatisfiable part aborts the whole resolve.
  pub(super) fn transitive_resolve(&mut self, name: &str) -> Result<()> {
    self.transitive_resolve_impl(name, true)?;
    Ok(())
  }

  /// Adds an optional (install-if) candidate with its closure; an
  /// unsatisfiable closure rolls back and returns `false` instead of
  /// failing.
  pub(super) fn transitive_resolve_optional(&mut self, name: &str) -> Result<bool> {
    self.transitive_resolve_impl(name, false)
  }

  /// The shared implementation: adds a package and its whole
  /// hard-dependency chain, rolling back to a pre-walk snapshot when
  /// the chain cannot complete. `strict` decides whether an incomplete
  /// closure is an error or just returns `false`.
  fn transitive_resolve_impl(&mut self, name: &str, strict: bool) -> Result<bool> {
    if self.state.visited.contains(name) {
      return Ok(true);
    }

    let real_name = match self.name_resolve(name)? {
      Some(r) => r,
      None => return Ok(true), // Unknown package in conditional context — skip silently
    };

    if self.state.visited.contains(&real_name) {
      return Ok(true);
    }

    // Snapshot before the walk so a soft-fail (or strict-fail bail)
    // can roll back the walker. Cost is O(visited.len() + order.len()
    // + edges.len()) per call — negligible for typical install sizes.
    let snapshot = self.state.clone();

    self.state.visited.insert(real_name.clone());
    self.state.order.push(real_name.clone());

    // BFS through transitive hard deps. Edges are emitted along with
    // each queue push so the resulting tree records the hard-dep chain
    // that brought every transitive package into scope.
    let mut dep_queue: VecDeque<String> = VecDeque::new();
    self.hard_deps_queue(&real_name, &mut dep_queue)?;
    while let Some(dep_name) = dep_queue.pop_front() {
      if self.state.visited.contains(&dep_name) {
        continue;
      }
      // Hard dependencies must be resolvable — unlike the entry point
      // (which is called from conditional contexts), these are required
      // deps. In strict mode this is a hard error; in soft mode the
      // entire subtree (including the entry-point package) is dropped
      // atomically by restoring the snapshot before returning.
      let real = match self.name_resolve(&dep_name)? {
        Some(r) => r,
        None => {
          self.state = snapshot;
          if strict {
            anyhow::bail!("Package '{}' not found in the index", dep_name);
          }
          return Ok(false);
        }
      };
      if self.state.visited.contains(&real) {
        continue;
      }
      self.state.visited.insert(real.clone());
      self.hard_deps_queue(&real, &mut dep_queue)?;
      self.state.order.push(real);
    }

    Ok(true)
  }

  /// Queues one package's resolved hard dependencies, emitting the edge
  /// with each push so the tree records the chain that brought every
  /// transitive package in.
  fn hard_deps_queue(&mut self, name: &str, queue: &mut VecDeque<String>) -> Result<()> {
    for dep_group in self.cfg.index.dependencies().of(name, DepKind::Depends)? {
      if let Some(dep_name) = self.dep_resolve(&dep_group, false)? {
        self.dep_edge_emit(name, &dep_group, &dep_name, DepKind::Depends);
        queue.push_back(dep_name);
      }
    }
    Ok(())
  }
}
