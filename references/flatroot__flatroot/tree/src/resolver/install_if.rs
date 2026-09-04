//! Applies Alpine's install-if rules — a package auto-installed once a
//! set of other packages is present — re-running the sweep until a pass
//! adds nothing. A candidate whose hard dependencies cannot be
//! satisfied is skipped, not failed.

use std::collections::HashSet;

use anyhow::Result;

use crate::package::DepKind;
use crate::resolver::edge::DepEdge;

use super::DepWalker;

impl DepWalker<'_> {
  /// Adds every install-if candidate whose triggers are all present,
  /// re-sweeping until a pass adds nothing, since one addition can
  /// satisfy another's triggers. A candidate with an unsatisfiable
  /// closure is skipped with a warning and never reconsidered.
  pub(super) fn install_if_fixpoint(&mut self) -> Result<()> {
    let mut install_if_skipped: HashSet<String> = HashSet::new();
    loop {
      // Carry the triggers alongside the triggered package so
      // `install_if_edges_record` can attribute the install to every
      // trigger that is present.
      let mut newly_triggered: Vec<(String, Vec<String>)> = Vec::new();

      for (pkg_name, triggers) in self.cfg.index.packages().install_if()? {
        if self.state.visited.contains(&pkg_name) || install_if_skipped.contains(&pkg_name) {
          continue;
        }

        let all_met = triggers
          .iter()
          .try_fold(true, |acc, t| Ok::<_, anyhow::Error>(acc && self.trigger_satisfied(t)?))?;

        if all_met {
          newly_triggered.push((pkg_name, triggers));
        }
      }

      if newly_triggered.is_empty() {
        break;
      }

      // Resolve hard deps of each triggered package — soft semantics: skip
      // triggers whose transitive closure can't be satisfied (matches apk).
      for (trig_name, triggers) in newly_triggered {
        let resolved = self.transitive_resolve_optional(&trig_name)?;
        if !resolved {
          eprintln!(
            "  warning: skipping install-if trigger '{}': transitive hard dep not satisfiable in the current index",
            trig_name,
          );
          install_if_skipped.insert(trig_name);
        } else {
          self.install_if_edges_record(&trig_name, &triggers)?;
        }
      }
    }
    Ok(())
  }

  /// Whether one trigger is met: the named package is in the closure,
  /// or a provider of the virtual name is.
  fn trigger_satisfied(&self, trigger: &str) -> Result<bool> {
    let name = Trigger::bare_name(trigger);
    Ok(
      self.state.visited.contains(name)
        || self
          .provider_first(name)?
          .is_some_and(|p| self.state.visited.contains(&p)),
    )
  }

  /// Records an `InstallIf` edge from each present trigger (the package
  /// itself, or the provider that stood in for it) to the added
  /// candidate; an absent trigger records no edge.
  fn install_if_edges_record(&mut self, trig_name: &str, triggers: &[String]) -> Result<()> {
    for trigger in triggers {
      let name = Trigger::bare_name(trigger);
      let parent = if self.state.visited.contains(name) {
        Some(name.to_string())
      } else {
        self.provider_first(name)?.filter(|p| self.state.visited.contains(p))
      };
      if let Some(parent) = parent {
        self.state.edges.push((
          parent,
          DepEdge {
            child: trig_name.to_string(),
            kind: DepKind::InstallIf,
            picked_alternative: None,
            original_constraint: None,
          },
        ));
      }
    }
    Ok(())
  }
}

/// Alpine's trigger notation: a trigger name may carry a `~` version
/// pin.
struct Trigger;

impl Trigger {
  /// The package name with any `~` version pin stripped, so a pinned
  /// trigger matches the index's name-keyed lookups.
  fn bare_name(trigger: &str) -> &str {
    trigger.split('~').next().unwrap_or(trigger)
  }
}
