//! Resolver behaviour coverage (prefix RES).
//!
//! These tests close the gaps the inline `src/resolver/*` and `src/dep_tree.rs`
//! unit tests leave at the integration layer. The bulk drive the *public*
//! resolver API (`DepWalker::resolve` for install order, `DepTree::walk` for
//! the provenance graph: roots, nodes, per-node versions, and edges with their
//! kind / picked-alternative / original-constraint) against synthetic offline
//! catalogues built the public way through `Index::open_or_populate`
//! (`tests/resolver/synth.rs`). A handful that genuinely require the live
//! pipeline — the `FLATROOT_ARG_*` env-fallback closure equivalence and the
//! declared-vs-linker strategy split — run the binary against a real mirror.
//!
//! Because the synthetic builder mutates the process-global `FLATROOT_CACHE_HOME`
//! and `flatroot::path_index::PathIndex::open` caches parsed indices by path,
//! every test in this file is `#[serial]`.

mod resolver;

use std::collections::HashSet;

use flatroot::dep_tree::DepTree;
use flatroot::package::{DepKind, RichDep};
use flatroot::resolver::{DepWalker, ResolutionEnv};
use serial_test::serial;

use resolver::synth::{
  Family, SynthIndex, dep, dep_alternatives, dep_versioned, make_pkg, pkg, provides, provides_versioned, rich_pkg,
  rich_pkg_versioned, synth_index, synth_index_with_paths,
};

// ---------------------------------------------------------------------------
// ResolutionEnv builders over a synthetic catalogue.
// ---------------------------------------------------------------------------

/// Default config: no soft deps, no exclusions. The companion path index,
/// when the fixture recorded one, is vended by the catalogue itself.
fn cfg<'a>(synth: &'a SynthIndex) -> ResolutionEnv<'a> {
  ResolutionEnv {
    index: &synth.index,
    include_recommends: false,
    include_suggests: false,
    exclude: &[],
  }
}

/// Config widened with recommends/suggests.
fn cfg_soft<'a>(synth: &'a SynthIndex, recommends: bool, suggests: bool) -> ResolutionEnv<'a> {
  ResolutionEnv {
    index: &synth.index,
    include_recommends: recommends,
    include_suggests: suggests,
    exclude: &[],
  }
}

/// Config carrying an exclusion list.
fn cfg_exclude<'a>(synth: &'a SynthIndex, exclude: &'a [String]) -> ResolutionEnv<'a> {
  ResolutionEnv {
    index: &synth.index,
    include_recommends: false,
    include_suggests: false,
    exclude,
  }
}

fn seed(name: &str) -> Vec<String> {
  vec![name.to_string()]
}

fn seeds(names: &[&str]) -> Vec<String> {
  names.iter().map(|s| s.to_string()).collect()
}

/// The single outgoing edge of `parent` in a tree (panics if not exactly one).
fn only_edge<'a>(tree: &'a DepTree, parent: &str) -> &'a flatroot::resolver::DepEdge {
  let node = tree
    .nodes
    .get(parent)
    .unwrap_or_else(|| panic!("node {parent} absent: {:?}", tree.nodes.keys().collect::<Vec<_>>()));
  assert_eq!(node.edges.len(), 1, "{parent} must have exactly one edge: {:?}", node.edges);
  &node.edges[0]
}

/// Find the edge from `parent` to `child`.
fn edge_to<'a>(tree: &'a DepTree, parent: &str, child: &str) -> &'a flatroot::resolver::DepEdge {
  tree
    .nodes
    .get(parent)
    .unwrap_or_else(|| panic!("node {parent} absent"))
    .edges
    .iter()
    .find(|e| e.child == child)
    .unwrap_or_else(|| panic!("{parent} has no edge to {child}"))
}

// ===========================================================================
// Happy path: full expansion, order, and the trace graph shape.
// ===========================================================================

// covers: RES-001
#[test]
#[serial]
fn linear_chain_expands_fully_in_dep_first_bfs_order() {
  // app -> lib -> libc: a single linear chain.
  let synth = synth_index(
    Family::Dpkg,
    vec![
      make_pkg("app", "1.0", vec![dep("lib")], vec![], vec![]),
      make_pkg("lib", "1.0", vec![dep("libc")], vec![], vec![]),
      make_pkg("libc", "2.36", vec![], vec![], vec![]),
    ],
  );

  let resolved = DepWalker::resolve(&cfg(&synth), &seed("app")).unwrap();
  // Full expansion: every transitive package is present.
  assert_eq!(resolved.iter().collect::<HashSet<_>>().len(), 3, "no dups: {resolved:?}");
  for p in ["app", "lib", "libc"] {
    assert!(resolved.contains(&p.to_string()), "{p} missing: {resolved:?}");
  }
  // Breadth-first discovery order: the chain is reached one layer at a time,
  // so the parent precedes the child it pulled in throughout.
  assert_eq!(resolved, vec!["app", "lib", "libc"], "dep-first BFS order");
}

// covers: RES-002, RES-057
#[test]
#[serial]
fn trace_renders_chain_with_edge_kinds_and_leaf_has_zero_edges() {
  let synth = synth_index(
    Family::Dpkg,
    vec![
      make_pkg("app", "1.0", vec![dep("lib")], vec![], vec![]),
      make_pkg("lib", "1.0", vec![dep("libc")], vec![], vec![]),
      make_pkg("libc", "2.36", vec![], vec![], vec![]),
    ],
  );

  let tree = DepTree::walk(&cfg(&synth), &seed("app")).unwrap();

  // Root reflects the single requested package.
  assert_eq!(tree.roots, vec!["app".to_string()]);
  assert_eq!(tree.nodes.len(), 3);

  // Per-node versions come straight from the index.
  assert_eq!(tree.nodes.get("app").unwrap().version, "1.0");
  assert_eq!(tree.nodes.get("libc").unwrap().version, "2.36");

  // Edge kinds are Depends; single-alt slots record no picked_alternative.
  let app_edge = only_edge(&tree, "app");
  assert_eq!(app_edge.child, "lib");
  assert_eq!(app_edge.kind, DepKind::Depends);
  assert!(app_edge.picked_alternative.is_none());
  assert_eq!(only_edge(&tree, "lib").child, "libc");
  assert_eq!(only_edge(&tree, "lib").kind, DepKind::Depends);

  // libc is a leaf — zero outgoing edges.
  assert!(tree.nodes.get("libc").unwrap().edges.is_empty(), "libc is a leaf");
}

// covers: RES-004
#[test]
#[serial]
fn circular_dependency_terminates_with_both_nodes_once() {
  // a <-> b mutual dependency.
  let synth = synth_index(
    Family::Dpkg,
    vec![
      make_pkg("a", "1.0", vec![dep("b")], vec![], vec![]),
      make_pkg("b", "1.0", vec![dep("a")], vec![], vec![]),
    ],
  );

  let resolved = DepWalker::resolve(&cfg(&synth), &seed("a")).unwrap();
  assert!(resolved.contains(&"a".to_string()));
  assert!(resolved.contains(&"b".to_string()));
  assert_eq!(resolved.len(), 2, "cycle visited each node exactly once: {resolved:?}");
}

// covers: RES-006
#[test]
#[serial]
fn provider_selection_is_deterministic_across_runs() {
  // Two providers of the virtual `awk`; provider_first orders by
  // (priority IS NULL), priority, name → gawk before mawk.
  let mut gawk = pkg("gawk", "5.2");
  gawk.provides = vec![provides("awk")];
  let mut mawk = pkg("mawk", "1.3.4");
  mawk.provides = vec![provides("awk")];
  let synth = synth_index(Family::Rpm, vec![make_pkg("app", "1.0", vec![dep("awk")], vec![], vec![]), gawk, mawk]);

  // The selected provider is identical across repeated resolutions.
  let mut chosen: Option<String> = None;
  for _ in 0..5 {
    let resolved = DepWalker::resolve(&cfg(&synth), &seed("app")).unwrap();
    let providers: Vec<&String> = resolved.iter().filter(|p| *p == "gawk" || *p == "mawk").collect();
    assert_eq!(providers.len(), 1, "exactly one awk provider: {resolved:?}");
    let this = providers[0].clone();
    if let Some(prev) = &chosen {
      assert_eq!(prev, &this, "provider selection must be stable");
    }
    chosen = Some(this);
  }
  assert_eq!(chosen.as_deref(), Some("gawk"), "first by (priority, name)");
}

// ===========================================================================
// Alternatives groups: first-fit, fall-through, picked_alternative.
// ===========================================================================

// covers: RES-007
#[test]
#[serial]
fn alternatives_first_fit_left_to_right_records_picked() {
  // app depends on (lib-a | lib-b); both satisfy → first (lib-a) wins.
  let synth = synth_index(
    Family::Dpkg,
    vec![
      make_pkg("app", "1.0", vec![dep_alternatives(&[("lib-a", None), ("lib-b", None)])], vec![], vec![]),
      make_pkg("lib-a", "1.0", vec![], vec![], vec![]),
      make_pkg("lib-b", "1.0", vec![], vec![], vec![]),
    ],
  );

  let tree = DepTree::walk(&cfg(&synth), &seed("app")).unwrap();
  assert!(tree.nodes.contains_key("lib-a"), "lib-a chosen first: {:?}", tree.nodes.keys().collect::<Vec<_>>());
  assert!(!tree.nodes.contains_key("lib-b"), "lib-b absent (first-fit stopped at lib-a)");

  let edge = only_edge(&tree, "app");
  assert_eq!(edge.child, "lib-a");
  // >1 alternative → picked_alternative records the chosen candidate.
  assert_eq!(edge.picked_alternative.as_deref(), Some("lib-a"));
}

// covers: RES-008
#[test]
#[serial]
fn alternatives_fall_through_when_first_fails_version() {
  // app depends on (lib-old >= 2.0 | lib-new): lib-old is 1.0 (fails),
  // so resolution falls through to the unconstrained lib-new.
  let synth = synth_index(
    Family::Dpkg,
    vec![
      make_pkg("app", "1.0", vec![dep_alternatives(&[("lib-old", Some(">= 2.0")), ("lib-new", None)])], vec![], vec![]),
      make_pkg("lib-old", "1.0", vec![], vec![], vec![]),
      make_pkg("lib-new", "3.0", vec![], vec![], vec![]),
    ],
  );

  let tree = DepTree::walk(&cfg(&synth), &seed("app")).unwrap();
  assert!(tree.nodes.contains_key("lib-new"), "lib-new chosen: {:?}", tree.nodes.keys().collect::<Vec<_>>());
  assert!(!tree.nodes.contains_key("lib-old"), "lib-old fails version, absent");

  let edge = only_edge(&tree, "app");
  assert_eq!(edge.child, "lib-new");
  assert_eq!(edge.picked_alternative.as_deref(), Some("lib-new"));
  // The chosen alternative (lib-new) carried no constraint.
  assert!(edge.original_constraint.is_none(), "lib-new has no constraint: {:?}", edge.original_constraint);
}

// covers: RES-010
#[test]
#[serial]
fn unsatisfiable_hard_constraint_no_alt_still_names_dep() {
  // app needs lib >= 5.0 but only lib 1.0 exists, no alternative.
  // A hard dep with no satisfying candidate is still named (lenient
  // include), so lib surfaces in the closure as the visible shortfall.
  let synth = synth_index(
    Family::Dpkg,
    vec![
      make_pkg("app", "1.0", vec![dep_versioned("lib", ">= 5.0")], vec![], vec![]),
      make_pkg("lib", "1.0", vec![], vec![], vec![]),
    ],
  );

  let resolved = DepWalker::resolve(&cfg(&synth), &seed("app")).unwrap();
  assert!(resolved.contains(&"lib".to_string()), "first-choice dep still named: {resolved:?}");
}

// covers: RES-011
#[test]
#[serial]
fn single_alt_slot_preserves_constraint_verbatim_and_picked_none() {
  // A single-alternative versioned slot: the original constraint string is
  // preserved verbatim on the edge and picked_alternative is None.
  let synth = synth_index(
    Family::Dpkg,
    vec![
      make_pkg("app", "1.0", vec![dep_versioned("lib", ">= 1.0")], vec![], vec![]),
      make_pkg("lib", "2.0", vec![], vec![], vec![]),
    ],
  );

  let tree = DepTree::walk(&cfg(&synth), &seed("app")).unwrap();
  let edge = only_edge(&tree, "app");
  assert_eq!(edge.child, "lib");
  assert_eq!(edge.original_constraint.as_deref(), Some(">= 1.0"), "verbatim constraint");
  assert!(edge.picked_alternative.is_none(), "single-alt slot reports None");
}

// covers: RES-012
#[test]
#[serial]
fn provides_with_version_judged_against_advertised_version() {
  // app needs libfoo >= 2.0. pkg-old provides libfoo=1.0 (reject),
  // pkg-new provides libfoo=3.0 (accept) — judged against the advertised
  // provides version, not the package's own version.
  let mut pkg_old = pkg("pkg-old", "9.9");
  pkg_old.provides = vec![provides_versioned("libfoo", "1.0")];
  let mut pkg_new = pkg("pkg-new", "0.1");
  pkg_new.provides = vec![provides_versioned("libfoo", "3.0")];
  let synth = synth_index(
    Family::Rpm,
    vec![
      make_pkg("app", "1.0", vec![dep_versioned("libfoo", ">= 2.0")], vec![], vec![]),
      pkg_old,
      pkg_new,
    ],
  );

  let resolved = DepWalker::resolve(&cfg(&synth), &seed("app")).unwrap();
  assert!(resolved.contains(&"pkg-new".to_string()), "pkg-new selected via provides 3.0: {resolved:?}");
  assert!(!resolved.contains(&"pkg-old".to_string()), "pkg-old rejected (provides 1.0): {resolved:?}");
}

// ===========================================================================
// File-path (PathIndex) dependency resolution.
// ===========================================================================

// covers: RES-013
#[test]
#[serial]
fn file_path_dependency_resolved_via_path_index() {
  // app declares a hard dep on the file path /usr/bin/foo, which the path
  // index maps to its owning package `owner`.
  let synth = synth_index_with_paths(
    Family::Rpm,
    vec![
      make_pkg("app", "1.0", vec![dep("/usr/bin/foo")], vec![], vec![]),
      make_pkg("owner", "2.0", vec![], vec![], vec![]),
    ],
    &[("/usr/bin/foo", "owner")],
  );

  let resolved = DepWalker::resolve(&cfg(&synth), &seed("app")).unwrap();
  assert!(resolved.contains(&"owner".to_string()), "file owner included via PathIndex: {resolved:?}");
}

// covers: RES-014
#[test]
#[serial]
fn file_path_dep_with_no_path_index_hard_bails() {
  // Alpine-style: no path index. A hard /path dep resolves to nothing and,
  // being mandatory, the first-choice name (the path) is re-queued and the
  // main walk bails when it too resolves to nothing.
  let synth = synth_index(Family::Dpkg, vec![make_pkg("app", "1.0", vec![dep("/missing/path")], vec![], vec![])]);

  let err = DepWalker::resolve(&cfg(&synth), &seed("app"))
    .err()
    .expect("expected an error");
  assert!(err.to_string().contains("not found in the index"), "hard /path dep with no path index must bail: {err}");
}

// covers: RES-015
#[test]
#[serial]
fn file_path_dep_with_empty_path_index_yields_no_candidate() {
  // The populate committed a companion record holding no facts, so the
  // catalogue's path lookup answers every query with nothing. candidates_get
  // / name_resolve yield nothing for the /path, so the hard dep bails the
  // same way.
  let synth = synth_index(Family::Rpm, vec![make_pkg("app", "1.0", vec![dep("/usr/lib/ghost.so")], vec![], vec![])]);

  let err = DepWalker::resolve(&cfg(&synth), &seed("app"))
    .err()
    .expect("expected an error");
  assert!(err.to_string().contains("not found in the index"), "no candidate from an empty path index: {err}");
}

// ===========================================================================
// Soft deps (--with) and advisory drop.
// ===========================================================================

// covers: RES-020
#[test]
#[serial]
fn with_recommends_and_suggests_includes_both_tiers() {
  let mut app = pkg("app", "1.0");
  app.depends = vec![dep("lib")];
  app.recommends = vec![dep("rec-pkg")];
  app.suggests = vec![dep("sug-pkg")];
  let synth = synth_index(Family::Dpkg, vec![app, pkg("lib", "1.0"), pkg("rec-pkg", "1.0"), pkg("sug-pkg", "1.0")]);

  // Both tiers on at once: both a recommends-tier and suggests-tier package
  // land in one closure.
  let resolved = DepWalker::resolve(&cfg_soft(&synth, true, true), &seed("app")).unwrap();
  assert!(resolved.contains(&"rec-pkg".to_string()), "recommends tier present: {resolved:?}");
  assert!(resolved.contains(&"sug-pkg".to_string()), "suggests tier present: {resolved:?}");
}

// covers: RES-021
#[test]
#[serial]
fn advisory_dep_that_cannot_be_satisfied_is_silently_dropped() {
  // app recommends a package that does not exist in the index. The advisory
  // (silent) dep yields nothing and the build still succeeds — no bail.
  let mut app = pkg("app", "1.0");
  app.depends = vec![dep("lib")];
  app.recommends = vec![dep("missing-rec")];
  let synth = synth_index(Family::Dpkg, vec![app, pkg("lib", "1.0")]);

  let resolved = DepWalker::resolve(&cfg_soft(&synth, true, false), &seed("app")).unwrap();
  assert!(resolved.contains(&"app".to_string()) && resolved.contains(&"lib".to_string()));
  assert!(!resolved.contains(&"missing-rec".to_string()), "unsatisfiable recommends dropped: {resolved:?}");
}

// ===========================================================================
// --exclude pruning.
// ===========================================================================

// covers: RES-022
#[test]
#[serial]
fn exclude_prunes_package_and_exclusively_transitive_deps() {
  // app -> mid -> leaf. Excluding `mid` prunes mid and its exclusively
  // transitive `leaf`, while app survives.
  let synth = synth_index(
    Family::Dpkg,
    vec![
      make_pkg("app", "1.0", vec![dep("mid")], vec![], vec![]),
      make_pkg("mid", "1.0", vec![dep("leaf")], vec![], vec![]),
      make_pkg("leaf", "1.0", vec![], vec![], vec![]),
    ],
  );

  let exclude = vec!["mid".to_string()];
  let resolved = DepWalker::resolve(&cfg_exclude(&synth, &exclude), &seed("app")).unwrap();
  assert!(resolved.contains(&"app".to_string()), "app kept: {resolved:?}");
  assert!(!resolved.contains(&"mid".to_string()), "mid pruned: {resolved:?}");
  assert!(!resolved.contains(&"leaf".to_string()), "leaf reachable only via mid, pruned: {resolved:?}");
}

// covers: RES-023
#[test]
#[serial]
fn exclude_keeps_package_reachable_via_another_path() {
  // app -> mid -> shared, and app -> other -> shared. Excluding `mid`
  // prunes mid, but `shared` survives because `other` still reaches it.
  let synth = synth_index(
    Family::Dpkg,
    vec![
      make_pkg("app", "1.0", vec![dep("mid"), dep("other")], vec![], vec![]),
      make_pkg("mid", "1.0", vec![dep("shared")], vec![], vec![]),
      make_pkg("other", "1.0", vec![dep("shared")], vec![], vec![]),
      make_pkg("shared", "1.0", vec![], vec![], vec![]),
    ],
  );

  let exclude = vec!["mid".to_string()];
  let resolved = DepWalker::resolve(&cfg_exclude(&synth, &exclude), &seed("app")).unwrap();
  assert!(!resolved.contains(&"mid".to_string()), "mid pruned: {resolved:?}");
  assert!(resolved.contains(&"other".to_string()), "other kept: {resolved:?}");
  assert!(resolved.contains(&"shared".to_string()), "shared survives via other: {resolved:?}");
}

// covers: RES-024
#[test]
#[serial]
fn exclude_applied_after_virtual_resolution() {
  // app depends on the virtual `awk`, provided by `mawk`. Excluding the
  // real provider name `mawk` drops it even though the dep is phrased
  // virtually (the exclude check runs against the resolved real_name).
  let mut mawk = pkg("mawk", "1.3.4");
  mawk.provides = vec![provides("awk")];
  let synth = synth_index(Family::Dpkg, vec![make_pkg("app", "1.0", vec![dep("awk")], vec![], vec![]), mawk]);

  let exclude = vec!["mawk".to_string()];
  let resolved = DepWalker::resolve(&cfg_exclude(&synth, &exclude), &seed("app")).unwrap();
  assert!(resolved.contains(&"app".to_string()), "app kept: {resolved:?}");
  assert!(!resolved.contains(&"mawk".to_string()), "excluded real_name skipped after virtual resolution: {resolved:?}");
}

// covers: RES-064
#[test]
#[serial]
fn exclude_of_top_level_requested_package_drops_it_at_queue_head() {
  // Both `app` and `lib` requested; `app` excluded. app is dropped at the
  // queue head before resolution while lib still resolves.
  let synth = synth_index(
    Family::Dpkg,
    vec![
      make_pkg("app", "1.0", vec![dep("app-only-dep")], vec![], vec![]),
      make_pkg("app-only-dep", "1.0", vec![], vec![], vec![]),
      make_pkg("lib", "1.0", vec![], vec![], vec![]),
    ],
  );

  let exclude = vec!["app".to_string()];
  let resolved = DepWalker::resolve(&cfg_exclude(&synth, &exclude), &seeds(&["app", "lib"])).unwrap();
  assert!(!resolved.contains(&"app".to_string()), "top-level app dropped: {resolved:?}");
  assert!(!resolved.contains(&"app-only-dep".to_string()), "app's exclusive dep never reached: {resolved:?}");
  assert!(resolved.contains(&"lib".to_string()), "co-requested lib still resolves: {resolved:?}");
}

// ===========================================================================
// Conflicts / Breaks: warn-but-include, applicability.
// ===========================================================================

// covers: RES-025
#[test]
#[serial]
fn conflicts_declaration_includes_both_packages() {
  // pkg-a Conflicts pkg-b. Both requested → both resolved (the resolver
  // only warns on a clash; it never drops a deliberately-requested pair).
  // The clash warning itself is emitted via eprintln! from library code
  // (process-global stderr) and is not capturable in-process.
  let synth = synth_index(
    Family::Dpkg,
    vec![
      make_pkg("pkg-a", "1.0", vec![], vec![dep("pkg-b")], vec![]),
      make_pkg("pkg-b", "1.0", vec![], vec![], vec![]),
    ],
  );

  let resolved = DepWalker::resolve(&cfg(&synth), &seeds(&["pkg-a", "pkg-b"])).unwrap();
  assert!(
    resolved.contains(&"pkg-a".to_string()) && resolved.contains(&"pkg-b".to_string()),
    "both included: {resolved:?}"
  );
}

// covers: RES-026, RES-065
#[test]
#[serial]
fn breaks_with_applicable_version_constraint_includes_both() {
  // pkg-a Breaks pkg-b << 2.0; pkg-b is 1.0 so the constraint applies
  // (recorded at the conflict_map registration moment). Both still resolve.
  let synth = synth_index(
    Family::Dpkg,
    vec![
      make_pkg("pkg-a", "1.0", vec![], vec![], vec![dep_versioned("pkg-b", "<< 2.0")]),
      make_pkg("pkg-b", "1.0", vec![], vec![], vec![]),
    ],
  );

  let resolved = DepWalker::resolve(&cfg(&synth), &seeds(&["pkg-a", "pkg-b"])).unwrap();
  assert!(
    resolved.contains(&"pkg-a".to_string()) && resolved.contains(&"pkg-b".to_string()),
    "both included: {resolved:?}"
  );
}

// covers: RES-027, RES-065
#[test]
#[serial]
fn breaks_with_non_applicable_version_constraint_includes_both() {
  // pkg-a Breaks pkg-b << 1.0; pkg-b is 2.0 so the constraint does NOT
  // apply — no clash registered — and both resolve cleanly.
  let synth = synth_index(
    Family::Dpkg,
    vec![
      make_pkg("pkg-a", "1.0", vec![], vec![], vec![dep_versioned("pkg-b", "<< 1.0")]),
      make_pkg("pkg-b", "2.0", vec![], vec![], vec![]),
    ],
  );

  let resolved = DepWalker::resolve(&cfg(&synth), &seeds(&["pkg-a", "pkg-b"])).unwrap();
  assert!(
    resolved.contains(&"pkg-a".to_string()) && resolved.contains(&"pkg-b".to_string()),
    "both included: {resolved:?}"
  );
}

// ===========================================================================
// RPM rich dependencies: If / Unless / And / Or / With / Without.
// ===========================================================================

// covers: RES-028
#[test]
#[serial]
fn rich_dep_if_payload_installed_and_edge_is_richif() {
  // app: If(extra-data) condition feature-pkg. With feature-pkg present,
  // extra-data is folded in and the edge carries kind RichIf.
  let mut app = pkg("app", "1.0");
  app.rich_deps = vec![RichDep::If {
    payload: Box::new(rich_pkg("extra-data")),
    condition: Box::new(rich_pkg("feature-pkg")),
  }];
  let synth = synth_index(Family::Rpm, vec![app, pkg("feature-pkg", "1.0"), pkg("extra-data", "1.0")]);

  let tree = DepTree::walk(&cfg(&synth), &seeds(&["app", "feature-pkg"])).unwrap();
  assert!(tree.nodes.contains_key("extra-data"), "payload present: {:?}", tree.nodes.keys().collect::<Vec<_>>());
  assert_eq!(edge_to(&tree, "app", "extra-data").kind, DepKind::RichIf);
}

// covers: RES-029
#[test]
#[serial]
fn rich_dep_if_payload_absent_when_condition_absent() {
  let mut app = pkg("app", "1.0");
  app.rich_deps = vec![RichDep::If {
    payload: Box::new(rich_pkg("extra-data")),
    condition: Box::new(rich_pkg("feature-pkg")),
  }];
  let synth = synth_index(Family::Rpm, vec![app, pkg("feature-pkg", "1.0"), pkg("extra-data", "1.0")]);

  // feature-pkg NOT requested → condition not met → payload not pulled in.
  let resolved = DepWalker::resolve(&cfg(&synth), &seed("app")).unwrap();
  assert!(!resolved.contains(&"extra-data".to_string()), "condition absent → payload not installed: {resolved:?}");
}

// covers: RES-030
#[test]
#[serial]
fn rich_dep_unless_fallback_installed_and_edge_is_richunless() {
  // app: Unless(fallback) condition preferred. preferred absent → fallback
  // installed, and the edge kind is RichUnless.
  let mut app = pkg("app", "1.0");
  app.rich_deps = vec![RichDep::Unless {
    payload: Box::new(rich_pkg("fallback")),
    condition: Box::new(rich_pkg("preferred")),
  }];
  let synth = synth_index(Family::Rpm, vec![app, pkg("preferred", "1.0"), pkg("fallback", "1.0")]);

  let tree = DepTree::walk(&cfg(&synth), &seed("app")).unwrap();
  assert!(tree.nodes.contains_key("fallback"), "fallback installed: {:?}", tree.nodes.keys().collect::<Vec<_>>());
  assert_eq!(edge_to(&tree, "app", "fallback").kind, DepKind::RichUnless);
}

// covers: RES-031
#[test]
#[serial]
fn rich_dep_unless_fallback_absent_when_guard_present() {
  let mut app = pkg("app", "1.0");
  app.rich_deps = vec![RichDep::Unless {
    payload: Box::new(rich_pkg("fallback")),
    condition: Box::new(rich_pkg("preferred")),
  }];
  let synth = synth_index(Family::Rpm, vec![app, pkg("preferred", "1.0"), pkg("fallback", "1.0")]);

  // preferred present → guard satisfied → fallback NOT installed.
  let resolved = DepWalker::resolve(&cfg(&synth), &seeds(&["app", "preferred"])).unwrap();
  assert!(!resolved.contains(&"fallback".to_string()), "guard present → no fallback: {resolved:?}");
}

// covers: RES-032
#[test]
#[serial]
fn rich_dep_fixpoint_cascades_across_passes() {
  // app: If(B) when feature present (feature requested).
  // B:   If(C) when B present.
  // Pass 1 admits B (feature present); pass 2 admits C (B now present).
  let mut app = pkg("app", "1.0");
  app.rich_deps = vec![RichDep::If {
    payload: Box::new(rich_pkg("B")),
    condition: Box::new(rich_pkg("feature")),
  }];
  let mut b = pkg("B", "1.0");
  b.rich_deps = vec![RichDep::If {
    payload: Box::new(rich_pkg("C")),
    condition: Box::new(rich_pkg("B")),
  }];
  let synth = synth_index(Family::Rpm, vec![app, b, pkg("feature", "1.0"), pkg("C", "1.0")]);

  let resolved = DepWalker::resolve(&cfg(&synth), &seeds(&["app", "feature"])).unwrap();
  assert!(resolved.contains(&"B".to_string()), "B in pass 1: {resolved:?}");
  assert!(resolved.contains(&"C".to_string()), "C cascaded in pass 2: {resolved:?}");
}

// covers: RES-033
#[test]
#[serial]
fn rich_dep_payload_brings_full_transitive_hard_closure() {
  // app: If(extra-data) when feature-pkg present. extra-data -> extra-lib.
  // The fired payload arrives with its whole transitive hard-dep closure.
  let mut app = pkg("app", "1.0");
  app.rich_deps = vec![RichDep::If {
    payload: Box::new(rich_pkg("extra-data")),
    condition: Box::new(rich_pkg("feature-pkg")),
  }];
  let synth = synth_index(
    Family::Rpm,
    vec![
      app,
      pkg("feature-pkg", "1.0"),
      make_pkg("extra-data", "1.0", vec![dep("extra-lib")], vec![], vec![]),
      pkg("extra-lib", "1.0"),
    ],
  );

  let resolved = DepWalker::resolve(&cfg(&synth), &seeds(&["app", "feature-pkg"])).unwrap();
  assert!(resolved.contains(&"extra-data".to_string()), "payload present: {resolved:?}");
  assert!(resolved.contains(&"extra-lib".to_string()), "payload's transitive hard dep present: {resolved:?}");
}

// covers: RES-034
#[test]
#[serial]
fn rich_dep_payload_with_unsatisfiable_hard_dep_aborts_strictly() {
  // The fired payload's required closure cannot be satisfied → strict abort
  // with a "not found" bail; the state is rolled back (the whole resolve
  // returns Err rather than a partial set).
  let mut app = pkg("app", "1.0");
  app.rich_deps = vec![RichDep::If {
    payload: Box::new(rich_pkg("extra-data")),
    condition: Box::new(rich_pkg("feature-pkg")),
  }];
  let synth = synth_index(
    Family::Rpm,
    vec![
      app,
      pkg("feature-pkg", "1.0"),
      make_pkg("extra-data", "1.0", vec![dep("ghost-lib")], vec![], vec![]),
    ],
  );

  let err = DepWalker::resolve(&cfg(&synth), &seeds(&["app", "feature-pkg"]))
    .err()
    .expect("expected an error");
  assert!(err.to_string().contains("not found in the index"), "strict rich-dep payload abort: {err}");
}

// covers: RES-035
#[test]
#[serial]
fn rich_dep_or_prefers_already_satisfied_branch() {
  // app: If( Or(left, right) ) when feature present. left is already in the
  // set (requested) → the Or yields the left branch; right is not pulled in.
  let mut app = pkg("app", "1.0");
  app.rich_deps = vec![RichDep::If {
    payload: Box::new(RichDep::Or(Box::new(rich_pkg("left")), Box::new(rich_pkg("right")))),
    condition: Box::new(rich_pkg("feature")),
  }];
  let synth = synth_index(Family::Rpm, vec![app, pkg("feature", "1.0"), pkg("left", "1.0"), pkg("right", "1.0")]);

  let resolved = DepWalker::resolve(&cfg(&synth), &seeds(&["app", "feature", "left"])).unwrap();
  assert!(resolved.contains(&"left".to_string()), "left taken (already resolved): {resolved:?}");
  assert!(!resolved.contains(&"right".to_string()), "right not pulled in by the Or: {resolved:?}");
}

// covers: RES-036
#[test]
#[serial]
fn rich_dep_and_unions_both_payload_sides() {
  // app: If( And(X, Y) ) when feature present → both X and Y folded in.
  let mut app = pkg("app", "1.0");
  app.rich_deps = vec![RichDep::If {
    payload: Box::new(RichDep::And(Box::new(rich_pkg("X")), Box::new(rich_pkg("Y")))),
    condition: Box::new(rich_pkg("feature")),
  }];
  let synth = synth_index(Family::Rpm, vec![app, pkg("feature", "1.0"), pkg("X", "1.0"), pkg("Y", "1.0")]);

  let resolved = DepWalker::resolve(&cfg(&synth), &seeds(&["app", "feature"])).unwrap();
  assert!(resolved.contains(&"X".to_string()) && resolved.contains(&"Y".to_string()), "And unions both: {resolved:?}");
}

// covers: RES-037
#[test]
#[serial]
fn rich_dep_with_without_payload_uses_left_only() {
  // With(L, R) and Without(L, R) payloads reduce to the left side only.
  let mut app = pkg("app", "1.0");
  app.rich_deps = vec![
    RichDep::If {
      payload: Box::new(RichDep::With(Box::new(rich_pkg("L1")), Box::new(rich_pkg("R1")))),
      condition: Box::new(rich_pkg("feature")),
    },
    RichDep::If {
      payload: Box::new(RichDep::Without(Box::new(rich_pkg("L2")), Box::new(rich_pkg("R2")))),
      condition: Box::new(rich_pkg("feature")),
    },
  ];
  let synth = synth_index(
    Family::Rpm,
    vec![
      app,
      pkg("feature", "1.0"),
      pkg("L1", "1.0"),
      pkg("R1", "1.0"),
      pkg("L2", "1.0"),
      pkg("R2", "1.0"),
    ],
  );

  let resolved = DepWalker::resolve(&cfg(&synth), &seeds(&["app", "feature"])).unwrap();
  assert!(resolved.contains(&"L1".to_string()), "With → left contributes: {resolved:?}");
  assert!(resolved.contains(&"L2".to_string()), "Without → left contributes: {resolved:?}");
  assert!(!resolved.contains(&"R1".to_string()), "With right side ignored: {resolved:?}");
  assert!(!resolved.contains(&"R2".to_string()), "Without right side ignored: {resolved:?}");
}

// covers: RES-038
#[test]
#[serial]
fn rich_dep_condition_with_version_judged_against_present_package() {
  // app: If(payload-hi) when cond-pkg >= 2.0; cond-pkg present at 3.0 → fires.
  // app: If(payload-lo) when cond-pkg >= 5.0; cond-pkg 3.0 fails → suppressed.
  let mut app = pkg("app", "1.0");
  app.rich_deps = vec![
    RichDep::If {
      payload: Box::new(rich_pkg("payload-hi")),
      condition: Box::new(rich_pkg_versioned("cond-pkg", ">= 2.0")),
    },
    RichDep::If {
      payload: Box::new(rich_pkg("payload-lo")),
      condition: Box::new(rich_pkg_versioned("cond-pkg", ">= 5.0")),
    },
  ];
  let synth = synth_index(
    Family::Rpm,
    vec![
      app,
      pkg("cond-pkg", "3.0"),
      pkg("payload-hi", "1.0"),
      pkg("payload-lo", "1.0"),
    ],
  );

  let resolved = DepWalker::resolve(&cfg(&synth), &seeds(&["app", "cond-pkg"])).unwrap();
  assert!(resolved.contains(&"payload-hi".to_string()), "satisfying version → fires: {resolved:?}");
  assert!(!resolved.contains(&"payload-lo".to_string()), "failing version → suppressed: {resolved:?}");
}

// covers: RES-039
#[test]
#[serial]
fn rich_dep_condition_satisfied_by_virtual_provider() {
  // app: If(payload) when condition `cap` (a virtual). provider-pkg provides
  // `cap` and is in the set → is_resolved sees the provider → condition met.
  let mut app = pkg("app", "1.0");
  app.rich_deps = vec![RichDep::If {
    payload: Box::new(rich_pkg("payload")),
    condition: Box::new(rich_pkg("cap")),
  }];
  let mut provider_pkg = pkg("provider-pkg", "1.0");
  provider_pkg.provides = vec![provides("cap")];
  let synth = synth_index(Family::Rpm, vec![app, provider_pkg, pkg("payload", "1.0")]);

  let resolved = DepWalker::resolve(&cfg(&synth), &seeds(&["app", "provider-pkg"])).unwrap();
  assert!(resolved.contains(&"payload".to_string()), "virtual-provider condition fires payload: {resolved:?}");
}

// covers: RES-040
#[test]
#[serial]
fn rich_dep_nested_condition_unmet_precondition_is_vacuously_true() {
  // app: If(payload) when condition = If(inner) when absent-guard.
  // The inner If's precondition (absent-guard) is unmet → the nested
  // condition is vacuously true → the outer payload is evaluated and fires.
  let mut app = pkg("app", "1.0");
  app.rich_deps = vec![RichDep::If {
    payload: Box::new(rich_pkg("payload")),
    condition: Box::new(RichDep::If {
      payload: Box::new(rich_pkg("inner-never")),
      condition: Box::new(rich_pkg("absent-guard")),
    }),
  }];
  let synth = synth_index(
    Family::Rpm,
    vec![
      app,
      pkg("payload", "1.0"),
      pkg("inner-never", "1.0"),
      pkg("absent-guard", "1.0"),
    ],
  );

  // absent-guard NOT requested → nested If precondition unmet → vacuously
  // true → outer payload fires.
  let resolved = DepWalker::resolve(&cfg(&synth), &seed("app")).unwrap();
  assert!(resolved.contains(&"payload".to_string()), "nested-unmet guard is vacuously true: {resolved:?}");
}

// covers: RES-041
#[test]
#[serial]
fn rich_dep_fixpoint_only_evaluates_rich_deps_of_installed_packages() {
  // X carries a rich-dep that would pull `would-fire` (condition trivially
  // met by feature). But X is never installed, so its conditionals are never
  // evaluated and would-fire stays out.
  let mut x = pkg("X", "1.0");
  x.rich_deps = vec![RichDep::If {
    payload: Box::new(rich_pkg("would-fire")),
    condition: Box::new(rich_pkg("feature")),
  }];
  let synth = synth_index(Family::Rpm, vec![pkg("app", "1.0"), x, pkg("feature", "1.0"), pkg("would-fire", "1.0")]);

  // Only app + feature requested; X absent from the set.
  let resolved = DepWalker::resolve(&cfg(&synth), &seeds(&["app", "feature"])).unwrap();
  assert!(!resolved.contains(&"X".to_string()), "X not installed: {resolved:?}");
  assert!(!resolved.contains(&"would-fire".to_string()), "X's conditionals never evaluated: {resolved:?}");
}

// covers: RES-042
#[test]
#[serial]
fn rich_dep_payload_resolving_to_absent_name_is_skipped_not_fatal() {
  // app: If(ghost) when feature-pkg present. `ghost` is not in the index, so
  // name_resolve returns None → the payload is skipped (continue), no error.
  let mut app = pkg("app", "1.0");
  app.rich_deps = vec![RichDep::If {
    payload: Box::new(rich_pkg("ghost")),
    condition: Box::new(rich_pkg("feature-pkg")),
  }];
  let synth = synth_index(Family::Rpm, vec![app, pkg("feature-pkg", "1.0")]);

  let resolved = DepWalker::resolve(&cfg(&synth), &seeds(&["app", "feature-pkg"])).unwrap();
  assert!(resolved.contains(&"app".to_string()) && resolved.contains(&"feature-pkg".to_string()));
  assert!(!resolved.contains(&"ghost".to_string()), "ghost payload skipped: {resolved:?}");
}

// ===========================================================================
// Alpine install-if triggers.
// ===========================================================================

// covers: RES-043
#[test]
#[serial]
fn install_if_triggers_only_when_all_conditions_present() {
  let mut loader = pkg("loader", "1.0");
  loader.install_if = vec!["gdk-pixbuf".to_string(), "librsvg".to_string()];
  let synth = synth_index(Family::Dpkg, vec![pkg("gdk-pixbuf", "2.42"), pkg("librsvg", "2.58"), loader]);

  let resolved = DepWalker::resolve(&cfg(&synth), &seeds(&["gdk-pixbuf", "librsvg"])).unwrap();
  assert!(resolved.contains(&"loader".to_string()), "all triggers met → loader auto-installed: {resolved:?}");
}

// covers: RES-044
#[test]
#[serial]
fn install_if_not_triggered_when_partially_met() {
  let mut loader = pkg("loader", "1.0");
  loader.install_if = vec!["gdk-pixbuf".to_string(), "librsvg".to_string()];
  let synth = synth_index(Family::Dpkg, vec![pkg("gdk-pixbuf", "2.42"), pkg("librsvg", "2.58"), loader]);

  // Only gdk-pixbuf present → conjunctive trigger not satisfied.
  let resolved = DepWalker::resolve(&cfg(&synth), &seed("gdk-pixbuf")).unwrap();
  assert!(!resolved.contains(&"loader".to_string()), "partial conditions → loader absent: {resolved:?}");
}

// covers: RES-045
#[test]
#[serial]
fn install_if_fixpoint_cascades_across_passes() {
  // A requested. B install-if [A]. C install-if [B].
  // Pass 1 admits B (A present); pass 2 admits C (B now present).
  let mut b = pkg("B", "1.0");
  b.install_if = vec!["A".to_string()];
  let mut c = pkg("C", "1.0");
  c.install_if = vec!["B".to_string()];
  let synth = synth_index(Family::Dpkg, vec![pkg("A", "1.0"), b, c]);

  let resolved = DepWalker::resolve(&cfg(&synth), &seed("A")).unwrap();
  assert!(resolved.contains(&"B".to_string()), "B admitted in pass 1: {resolved:?}");
  assert!(resolved.contains(&"C".to_string()), "C cascaded in pass 2: {resolved:?}");
}

// covers: RES-046, RES-050
#[test]
#[serial]
fn install_if_trigger_with_unsatisfiable_hard_dep_dropped_and_build_succeeds() {
  // `triggered` install-if [X], but depends on an unresolvable name. The
  // trigger's transitive closure is unsatisfiable → it is dropped (soft
  // semantics, rolled back to the snapshot), the build still succeeds, and
  // the skip is remembered so the package never re-enters.
  let mut triggered = pkg("triggered", "1.0");
  triggered.install_if = vec!["X".to_string()];
  triggered.depends = vec![dep("unresolvable-name")];
  let synth = synth_index(Family::Dpkg, vec![pkg("X", "1.0"), triggered]);

  let resolved = DepWalker::resolve(&cfg(&synth), &seed("X")).unwrap();
  assert!(resolved.contains(&"X".to_string()), "build succeeds, X present: {resolved:?}");
  assert!(!resolved.contains(&"triggered".to_string()), "unsatisfiable trigger declined: {resolved:?}");
  assert!(!resolved.contains(&"unresolvable-name".to_string()), "no half-admitted residue (rolled back): {resolved:?}");
}

// covers: RES-047
#[test]
#[serial]
fn install_if_one_trigger_skip_does_not_block_others() {
  // bad and good both install-if [X]. bad's hard dep is unresolvable (skip);
  // good has none (admitted). One skip must not block the other.
  let mut bad = pkg("bad", "1.0");
  bad.install_if = vec!["X".to_string()];
  bad.depends = vec![dep("unresolvable")];
  let mut good = pkg("good", "1.0");
  good.install_if = vec!["X".to_string()];
  let synth = synth_index(Family::Dpkg, vec![pkg("X", "1.0"), bad, good]);

  let resolved = DepWalker::resolve(&cfg(&synth), &seed("X")).unwrap();
  assert!(resolved.contains(&"good".to_string()), "good admitted: {resolved:?}");
  assert!(!resolved.contains(&"bad".to_string()), "bad skipped independently: {resolved:?}");
}

// covers: RES-048
#[test]
#[serial]
fn install_if_trigger_satisfied_by_virtual_provider() {
  // candidate install-if [cap], where `cap` is a virtual provided by
  // prov-pkg. With prov-pkg in the set, trigger_satisfied resolves the
  // capability via provider_first and the candidate auto-installs.
  let mut prov_pkg = pkg("prov-pkg", "1.0");
  prov_pkg.provides = vec![provides("cap")];
  let mut candidate = pkg("candidate", "1.0");
  candidate.install_if = vec!["cap".to_string()];
  let synth = synth_index(Family::Dpkg, vec![prov_pkg, candidate]);

  let resolved = DepWalker::resolve(&cfg(&synth), &seed("prov-pkg")).unwrap();
  assert!(resolved.contains(&"candidate".to_string()), "trigger met via virtual provider: {resolved:?}");
}

// covers: RES-049
#[test]
#[serial]
fn install_if_trigger_with_version_pin_matches_on_bare_name() {
  // candidate install-if ["base-pkg~1.2.3"]. The '~version' pin is stripped
  // to the bare name `base-pkg`, which is present → trigger satisfied.
  let mut candidate = pkg("candidate", "1.0");
  candidate.install_if = vec!["base-pkg~1.2.3".to_string()];
  let synth = synth_index(Family::Dpkg, vec![pkg("base-pkg", "1.2.3"), candidate]);

  let resolved = DepWalker::resolve(&cfg(&synth), &seed("base-pkg")).unwrap();
  assert!(resolved.contains(&"candidate".to_string()), "pinned trigger matched on bare name: {resolved:?}");
}

// ===========================================================================
// Transitive resolve primitives (idempotency / silent skip).
// ===========================================================================

// covers: RES-051
#[test]
#[serial]
fn transitive_resolve_of_already_visited_is_noop() {
  // extra-data is both a hard dep of app (visited in the main walk) AND the
  // rich-dep payload fired by feature-pkg. The second admission via
  // transitive_resolve is a no-op: extra-data appears exactly once, no
  // duplicate order entry, and the build succeeds.
  let mut app = pkg("app", "1.0");
  app.depends = vec![dep("extra-data")];
  app.rich_deps = vec![RichDep::If {
    payload: Box::new(rich_pkg("extra-data")),
    condition: Box::new(rich_pkg("feature-pkg")),
  }];
  let synth = synth_index(Family::Rpm, vec![app, pkg("feature-pkg", "1.0"), pkg("extra-data", "1.0")]);

  let resolved = DepWalker::resolve(&cfg(&synth), &seeds(&["app", "feature-pkg"])).unwrap();
  let count = resolved.iter().filter(|p| *p == "extra-data").count();
  assert_eq!(count, 1, "already-visited transitive resolve is a no-op (no dup): {resolved:?}");
}

// covers: RES-052
#[test]
#[serial]
fn transitive_resolve_of_unknown_name_in_conditional_context_is_silent() {
  // A rich-dep payload that name-resolves to nothing is skipped (continue)
  // in the conditional context, contrasting the main walk's hard bail for
  // an unknown hard dep. The build succeeds.
  let mut app = pkg("app", "1.0");
  app.rich_deps = vec![RichDep::If {
    payload: Box::new(rich_pkg("nonexistent-ghost")),
    condition: Box::new(rich_pkg("feature-pkg")),
  }];
  let synth = synth_index(Family::Rpm, vec![app, pkg("feature-pkg", "1.0")]);

  let resolved = DepWalker::resolve(&cfg(&synth), &seeds(&["app", "feature-pkg"])).unwrap();
  assert!(resolved.contains(&"app".to_string()), "Ok(true), build succeeds: {resolved:?}");
  assert!(
    !resolved.contains(&"nonexistent-ghost".to_string()),
    "unknown conditional name skipped silently: {resolved:?}"
  );
}

// ===========================================================================
// Three-phase ordering & subsystem composition.
// ===========================================================================

// covers: RES-053
#[test]
#[serial]
fn three_phase_ordering_walk_then_rich_dep_then_install_if() {
  // Phase 1 (main BFS): app -> base, with feature requested.
  // Phase 2 (rich-dep): app If(rich-payload) when feature present.
  // Phase 3 (install-if): courtesy install-if [rich-payload] — can only fire
  //   AFTER phase 2 added rich-payload, proving rich_dep runs before
  //   install_if (and both after the main walk).
  let mut app = pkg("app", "1.0");
  app.depends = vec![dep("base")];
  app.rich_deps = vec![RichDep::If {
    payload: Box::new(rich_pkg("rich-payload")),
    condition: Box::new(rich_pkg("feature")),
  }];
  let mut courtesy = pkg("courtesy", "1.0");
  courtesy.install_if = vec!["rich-payload".to_string()];
  let synth = synth_index(
    Family::Rpm,
    vec![
      app,
      pkg("base", "1.0"),
      pkg("feature", "1.0"),
      pkg("rich-payload", "1.0"),
      courtesy,
    ],
  );

  let resolved = DepWalker::resolve(&cfg(&synth), &seeds(&["app", "feature"])).unwrap();
  assert!(resolved.contains(&"base".to_string()), "phase 1 hard dep present: {resolved:?}");
  assert!(resolved.contains(&"rich-payload".to_string()), "phase 2 rich payload present: {resolved:?}");
  assert!(
    resolved.contains(&"courtesy".to_string()),
    "phase 3 install-if fires on the phase-2 payload, proving rich_dep precedes install_if: {resolved:?}"
  );
}

// covers: RES-061
#[test]
#[serial]
fn resolver_order_honors_filesystem_first_bfs_layering() {
  // Mirrors how install.rs seeds base/dir-creating packages ahead of the
  // user's package: `filesystem` requested first, then `coreutils`. The
  // resolved order places the earlier-seeded dir-creating package first,
  // and a shared transitive dep is reached at its breadth-first depth
  // (after its first-level requirers), never out of layer.
  let synth = synth_index(
    Family::Dpkg,
    vec![
      make_pkg("filesystem", "1.0", vec![], vec![], vec![]),
      make_pkg("coreutils", "1.0", vec![dep("libc")], vec![], vec![]),
      make_pkg("libc", "2.36", vec![], vec![], vec![]),
    ],
  );

  let resolved = DepWalker::resolve(&cfg(&synth), &seeds(&["filesystem", "coreutils"])).unwrap();
  let pos = |name: &str| resolved.iter().position(|p| p == name).unwrap();
  assert!(pos("filesystem") < pos("coreutils"), "dir-creating base ordered earlier: {resolved:?}");
  assert!(pos("coreutils") < pos("libc"), "transitive libc reached one layer later: {resolved:?}");
}

// ===========================================================================
// DepTree edge cases: empty seed, leaf, roots, node-build bail.
// ===========================================================================

// covers: RES-054
#[test]
#[serial]
fn deptree_walk_rejects_empty_seed_slice() {
  let synth = synth_index(Family::Dpkg, vec![pkg("app", "1.0")]);
  let err = DepTree::walk(&cfg(&synth), &[]).err().expect("expected an error");
  assert!(err.to_string().contains("at least one seed is required"), "{err}");
}

// covers: RES-056
#[test]
#[serial]
fn deptree_roots_reflect_resolved_post_virtual_names_in_input_order() {
  // Two virtual seeds resolving to providers, requested in a fixed order.
  // The tree's roots carry the resolved provider names in input order.
  let mut mawk = pkg("mawk", "1.3.4");
  mawk.provides = vec![provides("awk")];
  let mut openssl = pkg("openssl-pkg", "3.0");
  openssl.provides = vec![provides("crypto")];
  let synth = synth_index(Family::Dpkg, vec![mawk, openssl]);

  let tree = DepTree::walk(&cfg(&synth), &seeds(&["awk", "crypto"])).unwrap();
  assert_eq!(
    tree.roots,
    vec!["mawk".to_string(), "openssl-pkg".to_string()],
    "roots hold resolved providers in input order"
  );
}

// covers: RES-058
#[test]
#[serial]
fn deptree_walk_bails_when_resolved_name_missing_from_index_at_node_build() {
  // app hard-depends on the file path /usr/bin/orphan. The path index maps
  // that path to an owning package name that was deliberately NOT inserted
  // into the packages table. name_resolve accepts the owner (the path-index
  // leg never consults `packages`), the walk records it in `order`, but
  // DepTree node-build's `packages().get` finds nothing → the documented
  // "resolved package '<name>' missing from index" bail.
  let synth = synth_index_with_paths(
    Family::Rpm,
    vec![make_pkg("app", "1.0", vec![dep("/usr/bin/orphan")], vec![], vec![])],
    &[("/usr/bin/orphan", "orphan-owner")],
  );
  let err = DepTree::walk(&cfg(&synth), &seed("app"))
    .err()
    .expect("expected an error");
  assert!(
    err.to_string().contains("missing from index"),
    "node-build bail when resolved name absent from packages: {err}"
  );
}

// covers: RES-062
#[test]
#[serial]
fn leaf_package_resolves_to_just_itself_with_no_edges() {
  // A genuinely-empty-dependency leaf resolves to exactly itself, the tree
  // has a single node with zero edges.
  let synth = synth_index(Family::Dpkg, vec![pkg("leaf-pkg", "1.0")]);

  let resolved = DepWalker::resolve(&cfg(&synth), &seed("leaf-pkg")).unwrap();
  assert_eq!(resolved, vec!["leaf-pkg".to_string()], "only the package itself");

  let tree = DepTree::walk(&cfg(&synth), &seed("leaf-pkg")).unwrap();
  assert_eq!(tree.roots, vec!["leaf-pkg".to_string()]);
  assert_eq!(tree.nodes.len(), 1);
  assert!(tree.nodes.get("leaf-pkg").unwrap().edges.is_empty(), "leaf has no edges");
}

// ===========================================================================
// Per-distro version comparator drives every constraint check.
// ===========================================================================

// covers: RES-066
#[test]
#[serial]
fn per_distro_comparator_drives_constraint_checks() {
  // The resolver is comparator-agnostic: it asks `index.version_cmp()`.
  // RPM accepts the single-char `<` operator that dpkg does not, and each
  // family orders versions its own way. Same graph, two families, two
  // verdicts — proving the dispatch is driven by the active comparator.

  // RPM single-char `<` operator: lib 1.0 satisfies `< 2.0`, so the
  // alternative is taken (not the fall-through). dpkg would reject `<`.
  let rpm = synth_index(
    Family::Rpm,
    vec![
      make_pkg("app", "1.0", vec![dep_alternatives(&[("lib", Some("< 2.0")), ("fallback", None)])], vec![], vec![]),
      make_pkg("lib", "1.0", vec![], vec![], vec![]),
      make_pkg("fallback", "1.0", vec![], vec![], vec![]),
    ],
  );
  let rpm_resolved = DepWalker::resolve(&cfg(&rpm), &seed("app")).unwrap();
  assert!(rpm_resolved.contains(&"lib".to_string()), "rpm comparator accepts single-char `<`: {rpm_resolved:?}");
  assert!(
    !rpm_resolved.contains(&"fallback".to_string()),
    "rpm: first alt satisfied, no fall-through: {rpm_resolved:?}"
  );

  // dpkg `<<` strict-less ordering: lib 2.0 does NOT satisfy `<< 2.0`, so
  // the alternative falls through to `fallback`.
  let deb = synth_index(
    Family::Dpkg,
    vec![
      make_pkg("app", "1.0", vec![dep_alternatives(&[("lib", Some("<< 2.0")), ("fallback", None)])], vec![], vec![]),
      make_pkg("lib", "2.0", vec![], vec![], vec![]),
      make_pkg("fallback", "1.0", vec![], vec![], vec![]),
    ],
  );
  let deb_resolved = DepWalker::resolve(&cfg(&deb), &seed("app")).unwrap();
  assert!(
    deb_resolved.contains(&"fallback".to_string()),
    "dpkg: lib 2.0 fails `<< 2.0`, falls through: {deb_resolved:?}"
  );
}

// ===========================================================================
// Live mirror: declared-vs-linker strategy split (RES-059) and the
// FLATROOT_ARG_* env-fallback closure equivalence (RES-063).
// ===========================================================================

mod live {
  use assert_cmd::Command;
  use serde_json::Value;
  use serial_test::serial;
  use std::collections::BTreeSet;
  use tempfile::TempDir;

  /// Reasons present in an `analyze trace --format=json` run with the given
  /// extra args against debian:bookworm.
  fn trace_reasons(extra: &[&str]) -> BTreeSet<String> {
    let cache = TempDir::new().unwrap();
    let mut full: Vec<&str> = vec!["--from", "debian:bookworm", "analyze", "trace", "bash"];
    full.extend_from_slice(extra);
    full.push("--format=json");
    let output = Command::cargo_bin("flatroot")
      .unwrap()
      .env("FLATROOT_CACHE_HOME", cache.path())
      .args(&full)
      .output()
      .expect("flatroot binary failed to spawn");
    assert!(
      output.status.success(),
      "flatroot analyze {:?} exited {:?}\nstderr:\n{}",
      extra,
      output.status.code(),
      String::from_utf8_lossy(&output.stderr)
    );
    let v: Value = serde_json::from_slice(&output.stdout).expect("analyze json parses");
    v.as_array()
      .expect("analyze top is an array")
      .iter()
      .filter_map(|e| e["reason"].as_str().map(str::to_string))
      .collect()
  }

  // covers: RES-059
  #[test]
  #[serial(flatroot_cache)]
  fn analyze_strategy_declared_vs_linker_changes_traced_edges() {
    // The declared strategy classifies via metadata edges; the linker
    // strategy via DT_NEEDED. Run each alone — the reason vocabulary
    // differs: declared yields `declared` reasons, linker yields `linker`
    // reasons, and neither produces the other's.
    let declared = trace_reasons(&["--strategy", "declared"]);
    let linker = trace_reasons(&["--strategy", "linker"]);

    assert!(declared.contains("declared"), "declared-only run must carry `declared` reasons: {declared:?}");
    assert!(!declared.contains("linker"), "declared-only run must not carry bare `linker`: {declared:?}");

    assert!(linker.contains("linker"), "linker-only run must carry `linker` reasons: {linker:?}");
    assert!(!linker.contains("declared"), "linker-only run must not carry bare `declared`: {linker:?}");
  }

  /// Read the resolved package set from a `.flatroot/packages` manifest.
  fn resolved_set_from(root: &std::path::Path) -> BTreeSet<String> {
    let manifest =
      std::fs::read_to_string(root.join(".flatroot/packages")).unwrap_or_else(|e| panic!("read manifest: {e}"));
    manifest
      .lines()
      .filter_map(|l| l.strip_prefix("Package: "))
      .map(|s| s.trim().to_string())
      .collect()
  }

  /// Run an install of `nano` on debian:bookworm. When `use_env` is set the
  /// resolver-driving inputs come from `FLATROOT_ARG_*` env vars with no
  /// matching CLI flags; otherwise they come from CLI flags. Returns the
  /// resolved closure read from the manifest.
  fn install_closure(use_env: bool) -> BTreeSet<String> {
    let root = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();
    let root_str = root.path().to_str().unwrap().to_string();

    let mut cmd = Command::cargo_bin("flatroot").unwrap();
    cmd.env("FLATROOT_CACHE_HOME", cache.path());
    if use_env {
      cmd
        .env("FLATROOT_ARG_FROM", "debian:bookworm")
        .env("FLATROOT_ARG_INSTALL_OUTPUT", &root_str)
        .env("FLATROOT_ARG_INSTALL_WITH", "recommends")
        .args(["install", "--postinstall=none", "nano"]);
    } else {
      cmd.args([
        "--from",
        "debian:bookworm",
        "install",
        "-o",
        root_str.as_str(),
        "--with",
        "recommends",
        "--postinstall=none",
        "nano",
      ]);
    }
    let output = cmd.output().expect("flatroot binary failed to spawn");
    assert!(
      output.status.success(),
      "install (use_env={use_env}) exited {:?}\nstderr:\n{}",
      output.status.code(),
      String::from_utf8_lossy(&output.stderr)
    );
    resolved_set_from(root.path())
  }

  // covers: RES-063
  #[test]
  #[serial(flatroot_cache)]
  fn flatroot_arg_env_fallback_drives_resolver_identically_to_flags() {
    // The same resolver-flag inputs supplied via FLATROOT_ARG_* env vars
    // must yield the identical resolved closure as the CLI-flag form.
    let via_env = install_closure(true);
    let via_flags = install_closure(false);
    assert!(!via_env.is_empty(), "env-driven install resolved nothing");
    assert_eq!(via_env, via_flags, "env-fallback closure must equal CLI-flag closure");
  }
}
