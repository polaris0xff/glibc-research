//! Keeps the reachable closure of requested packages in graph form,
//! rather than a flat list, so a reader can see who pulled in each
//! package and why.

use std::collections::{BTreeMap, HashMap, HashSet};

use anyhow::Result;

use crate::resolver::{DepEdge, DepWalker, Resolution, ResolutionEnv};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// The reachable closure of the requested packages as a graph, keeping
/// the roots separate so a reader can tell what the user asked for from
/// what was pulled in as a dependency.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DepTree {
  /// Resolved names of the packages the walk started from, in input
  /// order — each may differ from the input string after virtual-name
  /// resolution, and a fanned-out request contributes one per
  /// deduplicated package.
  pub roots: Vec<String>,
  /// Every reachable package keyed by resolved name: the roots plus every
  /// transitive dependency the resolver visited.
  pub nodes: BTreeMap<String, DepNode>,
}

/// One package in the closure: its identity and the dependency edges it
/// owns. No edges marks a leaf.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DepNode {
  /// Resolved package name as the index reports it.
  pub name: String,
  /// Version as the index records it.
  pub version: String,
  /// Outgoing dependency edges in resolver-visit order. Empty for leaves.
  pub edges: Vec<DepEdge>,
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

impl DepTree {
  /// Resolves `roots` and their full transitive closure into a graph;
  /// each edge records its kind, the alternative taken, and the
  /// declared version constraint. An empty `roots` is an error rather
  /// than an empty tree.
  pub fn walk(cfg: &ResolutionEnv<'_>, roots: &[String]) -> Result<DepTree> {
    if roots.is_empty() {
      return Err(anyhow::anyhow!("DepTree::walk: at least one seed is required"));
    }
    let Resolution { order, mut edges } = DepWalker::walk_internal(cfg, roots)?;

    // The BFS pushes every seed onto the queue before processing any of
    // them, so the resolver emits the resolved seed names to `order`
    // before any of their dependencies. The first `roots.len()` entries
    // of `order` are therefore the resolved seeds — except when two
    // inputs resolve to the same package, which callers deduplicate
    // upstream.
    let resolved_count = roots.len().min(order.len());
    let roots_resolved: Vec<String> = order[..resolved_count].to_vec();

    // `dep_resolve` records an edge's child as the raw dependency name. For a
    // versioned virtual whose providers miss the constraint that is the bare
    // virtual name (e.g. `fedora-release-identity`), not the provider the walk
    // actually pulled into the closure (`fedora-release-identity-basic`). Left
    // unmapped the rendered tree descends into virtual names absent from the
    // node set and never reaches the resolved providers, so it would disagree
    // with the flat renderings over the very same closure. Remap each such child
    // to the concrete closure node it resolved to — mirroring `name_resolve`'s
    // provider preference — so every rendering covers the identical package set.
    let node_names: HashSet<&str> = order.iter().map(String::as_str).collect();
    for (_parent, edge) in edges.iter_mut() {
      if node_names.contains(edge.child.as_str()) {
        continue;
      }
      let Some(provider) = cfg.index.providers().first(&edge.child)? else {
        continue;
      };
      if node_names.contains(provider.as_str()) {
        edge.child = provider;
      }
    }

    // Group edges by parent for fast attachment to nodes.
    let mut edges_by_parent: HashMap<String, Vec<DepEdge>> = HashMap::new();
    for (parent, edge) in edges {
      edges_by_parent.entry(parent).or_default().push(edge);
    }

    let mut nodes: BTreeMap<String, DepNode> = BTreeMap::new();
    for name in &order {
      let pkg = cfg
        .index
        .packages()
        .get(name)?
        .ok_or_else(|| anyhow::anyhow!("DepTree::walk: resolved package '{}' missing from index", name))?;
      let edges = edges_by_parent.remove(name).unwrap_or_default();
      nodes.insert(
        name.clone(),
        DepNode {
          name: pkg.name,
          version: pkg.version,
          edges,
        },
      );
    }

    Ok(DepTree {
      roots: roots_resolved,
      nodes,
    })
  }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::Index;
  use crate::package::fixtures::{dep, dep_alts, dep_ver, pkg, pkg_with_deps};
  use crate::package::{DepKind, DepSpec, Package, RichDep};

  fn walk_cfg(index: &Index, recommends: bool, suggests: bool) -> ResolutionEnv<'_> {
    ResolutionEnv {
      index,
      include_recommends: recommends,
      include_suggests: suggests,
      exclude: &[],
    }
  }

  fn build_db(packages: Vec<Package>) -> Index {
    Index::in_memory(packages)
  }

  // ---------------------------------------------------------------------------
  // build
  // ---------------------------------------------------------------------------

  fn seed(name: &str) -> Vec<String> {
    vec![name.to_string()]
  }

  #[test]
  fn build_simple_chain() {
    let conn = build_db(vec![
      pkg_with_deps("app", "1.0", vec![dep("lib")]),
      pkg_with_deps("lib", "1.0", vec![dep("libc")]),
      pkg("libc", "2.36"),
    ]);

    let tree = DepTree::walk(&walk_cfg(&conn, false, false), &seed("app")).unwrap();

    assert_eq!(tree.roots, vec!["app".to_string()]);
    assert_eq!(tree.nodes.len(), 3, "tree must contain app, lib, libc: {:?}", tree.nodes.keys().collect::<Vec<_>>());

    let app = tree.nodes.get("app").unwrap();
    assert_eq!(app.version, "1.0");
    assert_eq!(app.edges.len(), 1);
    assert_eq!(app.edges[0].child, "lib");
    assert_eq!(app.edges[0].kind, DepKind::Depends);

    let lib = tree.nodes.get("lib").unwrap();
    assert_eq!(lib.edges.len(), 1);
    assert_eq!(lib.edges[0].child, "libc");
    assert_eq!(lib.edges[0].kind, DepKind::Depends);

    let libc = tree.nodes.get("libc").unwrap();
    assert!(libc.edges.is_empty(), "libc has no outgoing edges: {:?}", libc.edges);
  }

  #[test]
  fn build_alternatives_record_picked() {
    let conn = build_db(vec![
      pkg_with_deps("app", "1.0", vec![dep_alts(&[("lib-old", Some(">= 2.0")), ("lib-new", None)])]),
      pkg("lib-old", "1.0"),
      pkg("lib-new", "3.0"),
    ]);

    let tree = DepTree::walk(&walk_cfg(&conn, false, false), &seed("app")).unwrap();
    let app_edge = &tree.nodes.get("app").unwrap().edges[0];
    assert_eq!(app_edge.child, "lib-new", "should pick lib-new (lib-old fails version)");
    assert_eq!(app_edge.picked_alternative.as_deref(), Some("lib-new"));
    assert!(
      app_edge.original_constraint.is_none(),
      "lib-new alt has no constraint: {:?}",
      app_edge.original_constraint
    );
  }

  #[test]
  fn build_recommends_gated_by_flag() {
    let mut app = pkg("app", "1.0");
    app.depends = vec![dep("lib")];
    app.recommends = vec![dep("rec-pkg")];
    let conn = build_db(vec![app, pkg("lib", "1.0"), pkg("rec-pkg", "1.0")]);

    let off = DepTree::walk(&walk_cfg(&conn, false, false), &seed("app")).unwrap();
    assert!(!off.nodes.contains_key("rec-pkg"), "rec-pkg out without flag: {:?}", off.nodes.keys().collect::<Vec<_>>());

    let on = DepTree::walk(&walk_cfg(&conn, true, false), &seed("app")).unwrap();
    assert!(on.nodes.contains_key("rec-pkg"), "rec-pkg in with flag: {:?}", on.nodes.keys().collect::<Vec<_>>());
    let rec_edge = on
      .nodes
      .get("app")
      .unwrap()
      .edges
      .iter()
      .find(|e| e.child == "rec-pkg")
      .expect("app must have edge to rec-pkg");
    assert_eq!(rec_edge.kind, DepKind::Recommends);
  }

  #[test]
  fn build_records_version_constraint() {
    let conn = build_db(vec![
      pkg_with_deps("app", "1.0", vec![dep_ver("lib", ">= 1.0")]),
      pkg("lib", "2.0"),
    ]);

    let tree = DepTree::walk(&walk_cfg(&conn, false, false), &seed("app")).unwrap();
    let edge = &tree.nodes.get("app").unwrap().edges[0];
    assert_eq!(edge.child, "lib");
    assert_eq!(edge.original_constraint.as_deref(), Some(">= 1.0"));
    assert!(edge.picked_alternative.is_none(), "single-alt slot reports None: {:?}", edge.picked_alternative);
  }

  #[test]
  fn build_rich_if_records_richif_kind() {
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
    app.depends = vec![dep("feature-pkg")];

    let conn = build_db(vec![app, pkg("feature-pkg", "1.0"), pkg("extra-data", "1.0")]);
    let tree = DepTree::walk(&walk_cfg(&conn, false, false), &seed("app")).unwrap();

    assert!(tree.nodes.contains_key("extra-data"), "extra-data pulled in: {:?}", tree.nodes.keys().collect::<Vec<_>>());
    let app_edges = &tree.nodes.get("app").unwrap().edges;
    let rich_edge = app_edges
      .iter()
      .find(|e| e.child == "extra-data")
      .expect("app must have a rich-if edge to extra-data");
    assert_eq!(rich_edge.kind, DepKind::RichIf);
  }

  #[test]
  fn build_multi_seed_unions_closures() {
    // Two independent seeds with overlapping transitive dep (libc) —
    // the closure must contain both seeds and the shared dep deduped.
    let conn = build_db(vec![
      pkg_with_deps("app1", "1.0", vec![dep("libc")]),
      pkg_with_deps("app2", "2.0", vec![dep("libc")]),
      pkg("libc", "2.36"),
    ]);

    let tree = DepTree::walk(&walk_cfg(&conn, false, false), &["app1".to_string(), "app2".to_string()]).unwrap();

    assert_eq!(tree.roots, vec!["app1".to_string(), "app2".to_string()]);
    assert_eq!(
      tree.nodes.len(),
      3,
      "tree must contain app1, app2, libc once each: {:?}",
      tree.nodes.keys().collect::<Vec<_>>()
    );
    assert!(tree.nodes.contains_key("libc"));
  }

  #[test]
  fn build_rejects_empty_seed_slice() {
    let conn = build_db(vec![pkg("app", "1.0")]);
    let err = DepTree::walk(&walk_cfg(&conn, false, false), &[]).unwrap_err();
    assert!(err.to_string().contains("at least one seed"), "{err}");
  }
}
