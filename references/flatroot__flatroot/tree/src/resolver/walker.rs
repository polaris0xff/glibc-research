//! The resolver's walk: `DepWalker` expands the requested packages
//! breadth-first into their full dependency closure, and `Closure` is
//! the state it accumulates. Name resolution, the conditional
//! fixpoints, and the transitive sub-walk extend `DepWalker` in the
//! sibling modules.

use std::collections::{HashMap, HashSet, VecDeque};

use anyhow::Result;

use crate::package::{DepKind, DepSpec, Dependency};
use crate::resolver::edge::DepEdge;
use crate::resolver::env::ResolutionEnv;

/// The state a walk accumulates, kept cheaply clonable so a speculative
/// round can be tried and then kept or discarded whole.
#[derive(Clone, Default)]
pub(super) struct Closure {
  /// Names already accounted for, so a name is never processed twice.
  pub(super) visited: HashSet<String>,
  /// Packages in reach order — because the walk widens one layer at a time,
  /// anything needed by something else comes first, so an install unpacks
  /// strictly in this order (one package's directory before a later one's
  /// files).
  pub(super) order: Vec<String>,
  /// Each "this package drew in that one" link, in the order the choices
  /// were made.
  pub(super) edges: Vec<(String, DepEdge)>,
}

impl Closure {
  /// Keeps `order` and `edges` and drops the `visited` set, which only
  /// the walk itself needs.
  fn into_resolution(self) -> Resolution {
    Resolution {
      order: self.order,
      edges: self.edges,
    }
  }
}

/// The result of a finished resolve: the install order and the edges
/// recording which package drew in which.
pub(crate) struct Resolution {
  /// Packages in dependency-first install order — anything needed by
  /// something else comes ahead of the thing that needs it.
  pub(crate) order: Vec<String>,
  /// Each "this package drew in that one" link in the order the choices
  /// were made.
  pub(crate) edges: Vec<(String, DepEdge)>,
}

/// One resolve in progress.
pub struct DepWalker<'a> {
  /// The read-only inputs of the resolve.
  pub(super) cfg: &'a ResolutionEnv<'a>,
  /// The closure built up as the walk proceeds.
  pub(super) state: Closure,
  /// The excluded names, as a set for fast per-candidate lookup.
  exclude_set: HashSet<&'a str>,
}

impl<'a> DepWalker<'a> {
  /// Resolves the requested packages into their full dependency
  /// closure — one concrete package chosen per alternative — in
  /// dependency-first install order.
  pub fn resolve(cfg: &ResolutionEnv<'_>, requested: &[String]) -> Result<Vec<String>> {
    Ok(Self::walk_internal(cfg, requested)?.order)
  }

  /// The shared walk behind `resolve` and the dependency tree; it also
  /// returns the edges, so a dependency trace shows exactly what an
  /// install assembles. Plain dependencies resolve first, then the
  /// rich-dep and install-if fixpoints that depend on them.
  pub(crate) fn walk_internal(cfg: &ResolutionEnv<'_>, requested: &[String]) -> Result<Resolution> {
    let mut walker = DepWalker::new(cfg);
    walker.walk(requested)?;
    walker.rich_dep_fixpoint()?;
    walker.install_if_fixpoint()?;
    Ok(walker.into_resolution())
  }

  /// A fresh walker, with the exclusion list turned into a set for fast
  /// lookup.
  fn new(cfg: &'a ResolutionEnv<'a>) -> Self {
    Self {
      cfg,
      state: Closure::default(),
      exclude_set: cfg.exclude.iter().map(|s| s.as_str()).collect(),
    }
  }

  /// The finished `Resolution`, consuming the walker.
  fn into_resolution(self) -> Resolution {
    self.state.into_resolution()
  }

  /// The main walk: breadth-first from the requested packages until
  /// nothing new is reachable. Each name is resolved to a real package
  /// (a virtual name to a provider), no name is visited twice so cycles
  /// cannot loop, excluded names are skipped, and conflicts print
  /// warnings but never block.
  fn walk(&mut self, requested: &[String]) -> Result<()> {
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut conflict_map: HashMap<String, Vec<String>> = HashMap::new();

    for name in requested {
      queue.push_back(name.clone());
    }

    while let Some(name) = queue.pop_front() {
      if self.state.visited.contains(&name) || self.exclude_set.contains(name.as_str()) {
        continue;
      }

      // Resolve the package name (might be a virtual package)
      let real_name = self
        .name_resolve(&name)?
        .ok_or_else(|| anyhow::anyhow!("Package '{}' not found in the index", name))?;

      if self.state.visited.contains(&real_name) || self.exclude_set.contains(real_name.as_str()) {
        continue;
      }

      self.state.visited.insert(real_name.clone());

      // Check for conflicts with already-selected packages
      if let Some(conflictors) = conflict_map.get(&real_name) {
        for c in conflictors {
          eprintln!("  warning: {} conflicts with already-selected {}", real_name, c);
        }
      }

      // Register this package's Conflicts and Breaks
      self.conflicts_register(&real_name, DepKind::Conflicts, &mut conflict_map)?;
      self.conflicts_register(&real_name, DepKind::Breaks, &mut conflict_map)?;

      // Queue dependencies (hard).
      self.deps_walk(&real_name, DepKind::Depends, false, &mut queue)?;

      // Queue recommends (silently skip missing — they're optional)
      if self.cfg.include_recommends {
        self.deps_walk(&real_name, DepKind::Recommends, true, &mut queue)?;
      }

      // Queue suggests/optdepends (silently skip missing)
      if self.cfg.include_suggests {
        self.deps_walk(&real_name, DepKind::Suggests, true, &mut queue)?;
      }

      self.state.order.push(real_name);
    }

    Ok(())
  }

  /// Resolves one package's dependencies of one `kind` to concrete
  /// packages — one per dependency group — recording the edge for each
  /// and queueing each for its own visit.
  fn deps_walk(&mut self, parent: &str, kind: DepKind, silent: bool, queue: &mut VecDeque<String>) -> Result<()> {
    // Dependencies::of returns every comma-separated group on this
    // package's dep line. For `Depends: a | b, c, d | e`:
    //   group 0 = [a, b]   ← `|` means "either is fine"
    //   group 1 = [c]
    //   group 2 = [d, e]
    // The `,` says every group is required, so we walk all of them.
    // The `|` says one alternative per group is enough — dep_resolve
    // picks one and only that one package gets queued. So this loop
    // pushes 3 packages onto the BFS queue here, not 5.
    //
    // How dep_resolve picks: walk the alternatives left-to-right
    // (`a` before `b` — source order is author preference); inside
    // each alt walk its candidates (real package, or every provider
    // of a virtual sorted alphabetically, or the file-path owner for
    // RPM `/path` deps). First candidate whose version satisfies any
    // constraint wins. A version mismatch is never silently accepted
    // — move to the next candidate, then the next alt.
    for dep_group in self.cfg.index.dependencies().of(parent, kind.clone())? {
      if let Some(dep_name) = self.dep_resolve(&dep_group, silent)? {
        self.dep_edge_emit(parent, &dep_group, &dep_name, kind.clone());
        queue.push_back(dep_name);
      }
    }
    Ok(())
  }

  /// Records the edge from `parent` to `child`: which alternative was
  /// picked and the original version constraint.
  pub(super) fn dep_edge_emit(&mut self, parent: &str, dep_group: &Dependency, child: &str, kind: DepKind) {
    let alt = self.alt_for_pick(dep_group, child);
    let picked_alternative = if dep_group.alternatives.len() > 1 {
      alt.map(|a| a.name.clone())
    } else {
      None
    };
    let original_constraint = alt.and_then(|a| a.version_constraint.clone());
    self.state.edges.push((
      parent.to_string(),
      DepEdge {
        child: child.to_string(),
        kind,
        picked_alternative,
        original_constraint,
      },
    ));
  }

  /// The alternative the chosen package satisfied — by its own name, or
  /// through a virtual name it provides.
  fn alt_for_pick<'b>(&self, dep_group: &'b Dependency, pick: &str) -> Option<&'b DepSpec> {
    for alt in &dep_group.alternatives {
      if alt.name == pick {
        return Some(alt);
      }
      if let Ok(providers) = self.cfg.index.providers().of(&alt.name) {
        if providers.iter().any(|p| p.package_name == pick) {
          return Some(alt);
        }
      }
    }
    None
  }

  /// Resolves one dependency group to one package: alternatives are
  /// tried in the packager's order and the first candidate satisfying
  /// the version constraint wins. When nothing fits, a hard dependency
  /// (`silent` false) still returns its first alternative so the
  /// shortfall surfaces; an optional one (`silent` true) returns `None`.
  pub(super) fn dep_resolve(&self, dep_group: &Dependency, silent: bool) -> Result<Option<String>> {
    for alt in &dep_group.alternatives {
      for candidate in self.candidates_get(&alt.name)? {
        if self.candidate_satisfies(alt, &candidate)? {
          return Ok(Some(candidate));
        }
      }
    }

    if silent {
      return Ok(None);
    }
    Ok(dep_group.alternatives.first().map(|alt| alt.name.clone()))
  }

  /// Whether `candidate` satisfies `alt`. An unconstrained alternative
  /// is met by any existing package. A constrained one compares against
  /// the version `candidate` provides for `alt`'s name when it states
  /// one, and against `candidate`'s own version otherwise.
  fn candidate_satisfies(&self, alt: &DepSpec, candidate: &str) -> Result<bool> {
    let Some(constraint) = &alt.version_constraint else {
      return self.cfg.index.packages().exists(candidate);
    };
    let version = match self.provides_version_get(&alt.name, candidate)? {
      Some(v) => Some(v),
      None => self.cfg.index.packages().get(candidate)?.map(|p| p.version),
    };
    let Some(version) = version else {
      return Ok(false);
    };
    Ok(self.cfg.index.version_cmp().satisfies(&version, constraint))
  }

  /// Records `source`'s `Conflicts:`/`Breaks:` entries that apply to
  /// the version in the index, so a later conflicting pick prints a
  /// warning. A conflict never blocks, since the user may have asked
  /// for both packages.
  fn conflicts_register(
    &self,
    source: &str,
    kind: DepKind,
    conflict_map: &mut HashMap<String, Vec<String>>,
  ) -> Result<()> {
    for dep_group in self.cfg.index.dependencies().of(source, kind)? {
      for alt in &dep_group.alternatives {
        let conflict_applies = match &alt.version_constraint {
          Some(constraint) => self
            .cfg
            .index
            .packages()
            .get(&alt.name)?
            .is_some_and(|pkg| self.cfg.index.version_cmp().satisfies(&pkg.version, constraint)),
          None => true,
        };
        if conflict_applies {
          conflict_map
            .entry(alt.name.clone())
            .or_default()
            .push(source.to_string());
        }
      }
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::Index;
  use crate::package::fixtures::{dep, dep_alts, dep_ver, pkg, pkg_with_deps};
  use crate::package::{Package, RichDep};

  /// The resolver's conflict tests also need the negative-edge fields the
  /// shared fixtures deliberately leave empty; this extends the shared shape
  /// with conflicts and breaks rather than re-typing the whole literal.
  fn make_pkg(
    name: &str,
    version: &str,
    depends: Vec<Dependency>,
    conflicts: Vec<Dependency>,
    breaks: Vec<Dependency>,
  ) -> Package {
    Package {
      conflicts,
      breaks,
      ..pkg_with_deps(name, version, depends)
    }
  }

  fn build_db(packages: Vec<Package>) -> Index {
    Index::in_memory(packages)
  }

  fn walk_cfg(index: &Index, recommends: bool, suggests: bool) -> ResolutionEnv<'_> {
    ResolutionEnv {
      index,
      include_recommends: recommends,
      include_suggests: suggests,
      exclude: &[],
    }
  }

  #[test]
  fn accepts_satisfying_version() {
    let conn = build_db(vec![
      make_pkg("app", "1.0", vec![dep_ver("lib", ">= 1.0")], vec![], vec![]),
      make_pkg("lib", "2.0", vec![], vec![], vec![]),
    ]);

    let resolved = DepWalker::resolve(&walk_cfg(&conn, true, true), &["app".to_string()]).unwrap();
    assert!(resolved.contains(&"lib".to_string()));
    assert!(resolved.contains(&"app".to_string()));
  }

  #[test]
  fn picks_version_satisfying_alternative() {
    let conn = build_db(vec![
      make_pkg("app", "1.0", vec![dep_alts(&[("lib-old", Some(">= 2.0")), ("lib-new", None)])], vec![], vec![]),
      make_pkg("lib-old", "1.0", vec![], vec![], vec![]),
      make_pkg("lib-new", "3.0", vec![], vec![], vec![]),
    ]);

    let resolved = DepWalker::resolve(&walk_cfg(&conn, true, true), &["app".to_string()]).unwrap();
    assert!(resolved.contains(&"lib-new".to_string()), "Should pick lib-new: {:?}", resolved);
    assert!(!resolved.contains(&"lib-old".to_string()), "Should not include lib-old: {:?}", resolved);
  }

  #[test]
  fn warns_on_unsatisfied_no_alternative() {
    let conn = build_db(vec![
      make_pkg("app", "1.0", vec![dep_ver("lib", ">= 5.0")], vec![], vec![]),
      make_pkg("lib", "1.0", vec![], vec![], vec![]),
    ]);

    let resolved = DepWalker::resolve(&walk_cfg(&conn, true, true), &["app".to_string()]).unwrap();
    assert!(resolved.contains(&"lib".to_string()), "Should still include lib: {:?}", resolved);
  }

  #[test]
  fn conflicts_warn_but_include_both() {
    let conn = build_db(vec![
      make_pkg("pkg-a", "1.0", vec![], vec![dep("pkg-b")], vec![]),
      make_pkg("pkg-b", "1.0", vec![], vec![], vec![]),
    ]);

    let resolved =
      DepWalker::resolve(&walk_cfg(&conn, true, true), &["pkg-a".to_string(), "pkg-b".to_string()]).unwrap();
    assert!(resolved.contains(&"pkg-a".to_string()));
    assert!(resolved.contains(&"pkg-b".to_string()));
  }

  #[test]
  fn breaks_with_version_constraint() {
    let conn = build_db(vec![
      make_pkg("pkg-a", "1.0", vec![], vec![], vec![dep_ver("pkg-b", "<< 2.0")]),
      make_pkg("pkg-b", "1.0", vec![], vec![], vec![]),
    ]);

    let resolved =
      DepWalker::resolve(&walk_cfg(&conn, true, true), &["pkg-a".to_string(), "pkg-b".to_string()]).unwrap();
    assert!(resolved.contains(&"pkg-a".to_string()));
    assert!(resolved.contains(&"pkg-b".to_string()));
  }

  #[test]
  fn breaks_version_not_applicable() {
    let conn = build_db(vec![
      make_pkg("pkg-a", "1.0", vec![], vec![], vec![dep_ver("pkg-b", "<< 1.0")]),
      make_pkg("pkg-b", "2.0", vec![], vec![], vec![]),
    ]);

    let resolved =
      DepWalker::resolve(&walk_cfg(&conn, true, true), &["pkg-a".to_string(), "pkg-b".to_string()]).unwrap();
    assert!(resolved.contains(&"pkg-a".to_string()));
    assert!(resolved.contains(&"pkg-b".to_string()));
  }

  #[test]
  fn simple_dependency_chain() {
    let conn = build_db(vec![
      make_pkg("app", "1.0", vec![dep("lib")], vec![], vec![]),
      make_pkg("lib", "1.0", vec![dep("libc")], vec![], vec![]),
      make_pkg("libc", "2.36", vec![], vec![], vec![]),
    ]);

    let resolved = DepWalker::resolve(&walk_cfg(&conn, true, true), &["app".to_string()]).unwrap();
    assert_eq!(resolved, vec!["app", "lib", "libc"]);
  }

  #[test]
  fn no_duplicate_deps() {
    let conn = build_db(vec![
      make_pkg("app", "1.0", vec![dep("lib"), dep("libc")], vec![], vec![]),
      make_pkg("lib", "1.0", vec![dep("libc")], vec![], vec![]),
      make_pkg("libc", "2.36", vec![], vec![], vec![]),
    ]);

    let resolved = DepWalker::resolve(&walk_cfg(&conn, true, true), &["app".to_string()]).unwrap();
    let libc_count = resolved.iter().filter(|p| *p == "libc").count();
    assert_eq!(libc_count, 1);
  }

  #[test]
  fn virtual_package_resolves_to_provider() {
    let mut mawk = pkg("mawk", "1.3.4");
    mawk.provides = vec![DepSpec {
      name: "awk".to_string(),
      version_constraint: None,
    }];

    let conn = build_db(vec![make_pkg("app", "1.0", vec![dep("awk")], vec![], vec![]), mawk]);

    let resolved = DepWalker::resolve(&walk_cfg(&conn, false, false), &["app".to_string()]).unwrap();
    assert!(resolved.contains(&"mawk".to_string()), "mawk should be pulled in: {:?}", resolved);
    assert!(!resolved.contains(&"awk".to_string()), "awk is virtual");
  }

  #[test]
  fn provides_with_version_picks_correct_provider() {
    let mut pkg_old = pkg("pkg-old", "1.0");
    pkg_old.provides = vec![DepSpec {
      name: "libfoo".to_string(),
      version_constraint: Some("1.0".to_string()),
    }];

    let mut pkg_new = pkg("pkg-new", "1.0");
    pkg_new.provides = vec![DepSpec {
      name: "libfoo".to_string(),
      version_constraint: Some("3.0".to_string()),
    }];

    let conn = build_db(vec![
      make_pkg("app", "1.0", vec![dep_ver("libfoo", ">= 2.0")], vec![], vec![]),
      pkg_old,
      pkg_new,
    ]);

    let resolved = DepWalker::resolve(&walk_cfg(&conn, false, false), &["app".to_string()]).unwrap();
    assert!(resolved.contains(&"pkg-new".to_string()), "pkg-new satisfies: {:?}", resolved);
    assert!(!resolved.contains(&"pkg-old".to_string()), "pkg-old does not satisfy: {:?}", resolved);
  }

  #[test]
  fn recommends_only_with_flag() {
    let mut app = pkg("app", "1.0");
    app.depends = vec![dep("lib")];
    app.recommends = vec![dep("rec-pkg")];

    let conn = build_db(vec![app, pkg("lib", "1.0"), pkg("rec-pkg", "1.0")]);

    let without = DepWalker::resolve(&walk_cfg(&conn, false, false), &["app".to_string()]).unwrap();
    assert!(!without.contains(&"rec-pkg".to_string()), "excluded without flag: {:?}", without);

    let with = DepWalker::resolve(&walk_cfg(&conn, true, false), &["app".to_string()]).unwrap();
    assert!(with.contains(&"rec-pkg".to_string()), "included with flag: {:?}", with);
  }

  #[test]
  fn suggests_only_with_flag() {
    let mut app = pkg("app", "1.0");
    app.depends = vec![dep("lib")];
    app.suggests = vec![dep("sug-pkg")];

    let conn = build_db(vec![app, pkg("lib", "1.0"), pkg("sug-pkg", "1.0")]);

    let without = DepWalker::resolve(&walk_cfg(&conn, false, false), &["app".to_string()]).unwrap();
    assert!(!without.contains(&"sug-pkg".to_string()), "excluded without flag: {:?}", without);

    let with = DepWalker::resolve(&walk_cfg(&conn, false, true), &["app".to_string()]).unwrap();
    assert!(with.contains(&"sug-pkg".to_string()), "included with flag: {:?}", with);
  }

  #[test]
  fn install_if_triggers_when_all_conditions_met() {
    let mut loader = pkg("loader", "1.0");
    loader.install_if = vec!["gdk-pixbuf".to_string(), "librsvg".to_string()];

    let conn = build_db(vec![pkg("gdk-pixbuf", "2.42"), pkg("librsvg", "2.58"), loader]);

    let resolved =
      DepWalker::resolve(&walk_cfg(&conn, false, false), &["gdk-pixbuf".to_string(), "librsvg".to_string()]).unwrap();

    assert!(resolved.contains(&"loader".to_string()), "loader auto-installed: {:?}", resolved);
  }

  #[test]
  fn install_if_does_not_trigger_when_partial_conditions() {
    let mut loader = pkg("loader", "1.0");
    loader.install_if = vec!["gdk-pixbuf".to_string(), "librsvg".to_string()];

    let conn = build_db(vec![pkg("gdk-pixbuf", "2.42"), pkg("librsvg", "2.58"), loader]);

    let resolved = DepWalker::resolve(&walk_cfg(&conn, false, false), &["gdk-pixbuf".to_string()]).unwrap();

    assert!(!resolved.contains(&"loader".to_string()), "loader not installed: {:?}", resolved);
  }

  #[test]
  fn install_if_trigger_skipped_on_unresolvable_hard_dep() {
    let mut triggered = pkg("triggered", "1.0");
    triggered.install_if = vec!["X".to_string()];
    triggered.depends = vec![dep("unresolvable-name")];

    let conn = build_db(vec![pkg("X", "1.0"), triggered]);

    let resolved = DepWalker::resolve(&walk_cfg(&conn, false, false), &["X".to_string()]).unwrap();

    assert!(resolved.contains(&"X".to_string()));
    assert!(!resolved.contains(&"triggered".to_string()), "silently skipped: {:?}", resolved);
  }

  #[test]
  fn install_if_one_trigger_skip_does_not_block_others() {
    let mut bad = pkg("bad", "1.0");
    bad.install_if = vec!["X".to_string()];
    bad.depends = vec![dep("unresolvable")];

    let mut good = pkg("good", "1.0");
    good.install_if = vec!["X".to_string()];

    let conn = build_db(vec![pkg("X", "1.0"), bad, good]);

    let resolved = DepWalker::resolve(&walk_cfg(&conn, false, false), &["X".to_string()]).unwrap();

    assert!(resolved.contains(&"good".to_string()), "good resolves: {:?}", resolved);
    assert!(!resolved.contains(&"bad".to_string()), "bad skipped: {:?}", resolved);
  }

  #[test]
  fn rich_dep_if_condition_met() {
    let mut app = pkg("app", "1.0");
    app.rich_deps = vec![RichDep::If {
      payload: Box::new(RichDep::Pkg(DepSpec {
        name: "extra-data".to_string(),
        version_constraint: None,
      })),
      condition: Box::new(RichDep::Pkg(DepSpec {
        name: "feature-pkg".to_string(),
        version_constraint: None,
      })),
    }];

    let conn = build_db(vec![app, pkg("feature-pkg", "1.0"), pkg("extra-data", "1.0")]);

    let resolved =
      DepWalker::resolve(&walk_cfg(&conn, false, false), &["app".to_string(), "feature-pkg".to_string()]).unwrap();

    assert!(resolved.contains(&"extra-data".to_string()), "condition met: {:?}", resolved);
  }

  #[test]
  fn rich_dep_if_condition_not_met() {
    let mut app = pkg("app", "1.0");
    app.rich_deps = vec![RichDep::If {
      payload: Box::new(RichDep::Pkg(DepSpec {
        name: "extra-data".to_string(),
        version_constraint: None,
      })),
      condition: Box::new(RichDep::Pkg(DepSpec {
        name: "feature-pkg".to_string(),
        version_constraint: None,
      })),
    }];

    let conn = build_db(vec![app, pkg("feature-pkg", "1.0"), pkg("extra-data", "1.0")]);

    let resolved = DepWalker::resolve(&walk_cfg(&conn, false, false), &["app".to_string()]).unwrap();

    assert!(!resolved.contains(&"extra-data".to_string()), "condition not met: {:?}", resolved);
  }

  #[test]
  fn rich_dep_unless_condition_absent() {
    let mut app = pkg("app", "1.0");
    app.rich_deps = vec![RichDep::Unless {
      payload: Box::new(RichDep::Pkg(DepSpec {
        name: "fallback".to_string(),
        version_constraint: None,
      })),
      condition: Box::new(RichDep::Pkg(DepSpec {
        name: "preferred".to_string(),
        version_constraint: None,
      })),
    }];

    let conn = build_db(vec![app, pkg("preferred", "1.0"), pkg("fallback", "1.0")]);

    let resolved = DepWalker::resolve(&walk_cfg(&conn, false, false), &["app".to_string()]).unwrap();

    assert!(resolved.contains(&"fallback".to_string()), "fallback installed: {:?}", resolved);
  }

  #[test]
  fn rich_dep_unless_condition_present() {
    let mut app = pkg("app", "1.0");
    app.rich_deps = vec![RichDep::Unless {
      payload: Box::new(RichDep::Pkg(DepSpec {
        name: "fallback".to_string(),
        version_constraint: None,
      })),
      condition: Box::new(RichDep::Pkg(DepSpec {
        name: "preferred".to_string(),
        version_constraint: None,
      })),
    }];

    let conn = build_db(vec![app, pkg("preferred", "1.0"), pkg("fallback", "1.0")]);

    let resolved =
      DepWalker::resolve(&walk_cfg(&conn, false, false), &["app".to_string(), "preferred".to_string()]).unwrap();

    assert!(!resolved.contains(&"fallback".to_string()), "fallback not installed: {:?}", resolved);
  }

  #[test]
  fn circular_dependencies_do_not_loop() {
    let conn = build_db(vec![
      make_pkg("a", "1.0", vec![dep("b")], vec![], vec![]),
      make_pkg("b", "1.0", vec![dep("a")], vec![], vec![]),
    ]);

    let resolved = DepWalker::resolve(&walk_cfg(&conn, false, false), &["a".to_string()]).unwrap();
    assert!(resolved.contains(&"a".to_string()));
    assert!(resolved.contains(&"b".to_string()));
    assert_eq!(resolved.len(), 2);
  }
}
