//! Debian/Ubuntu resolver validation.
//!
//! Builds a **complete** dep graph from `apt-cache depends` by recursively
//! following every branch (including all alternatives). Uses a persistent
//! Docker container with Rust driving the BFS — each batch of newly discovered
//! packages is queried via `docker exec`, and Rust parses the output to find
//! more packages to query. The graph covers the full universe of possible
//! dependency paths. The test BFS then walks this graph, picking only the
//! alternative FlatRoot chose at each multi-option node.

use std::collections::{HashMap, HashSet, VecDeque};

use super::{DockerContainer, bfs_reachable, flatroot_resolved_list, validate};

/// Full validation for a Debian or Ubuntu package.
pub fn validate_deb(remote: &str, image: &str, pkg: &str) {
  let (dep_graph, provides_map, docker_installed, essential_packages) = docker_apt_graph(image, pkg);
  let flatroot = flatroot_resolved_list(remote, pkg);

  let flatroot_set: HashSet<&str> = flatroot.iter().map(|s| s.as_str()).collect();
  let installed_set: HashSet<&str> = docker_installed.iter().map(|s| s.as_str()).collect();

  // Seed BFS with the target package + all Essential packages.
  // This mirrors what FlatRoot does — it resolves from [essentials + user-requested].
  let mut seeds = essential_packages;
  if !seeds.contains(&pkg.to_string()) {
    seeds.push(pkg.to_string());
  }

  let reachable = bfs_reachable(&seeds, &dep_graph, &flatroot_set, &installed_set);

  validate(remote, pkg, &reachable, &docker_installed, &flatroot, &provides_map);
}

/// Build a complete dep graph using a persistent Docker container with
/// Rust-driven recursive discovery.
///
/// 1. Create and start a container (auto-removed on drop)
/// 2. Install the target package, get installed list + essential list
/// 3. Rust BFS: for each batch of unqueried packages, `docker exec` to
///    query `apt-cache depends`, parse the output, discover new packages
/// 4. Repeat until no new packages are discovered
///
/// Returns: (dep_graph, provides_map, all_installed, essential_packages)
fn docker_apt_graph(
  image: &str,
  pkg: &str,
) -> (HashMap<String, Vec<Vec<String>>>, HashMap<String, Vec<String>>, HashSet<String>, Vec<String>) {
  let container = DockerContainer::new(image);

  // Install the package and get metadata
  let setup_output = container.exec(&format!(
    r#"
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq 2>/dev/null
apt-get install -y -qq {pkg} 2>/dev/null 1>/dev/null

echo '===INSTALLED==='
dpkg -l 2>/dev/null | grep '^ii' | awk '{{print $2}}' | cut -d: -f1 | sort -u

echo '===ESSENTIAL==='
apt-cache dumpavail 2>/dev/null | grep -B10 "^Essential: yes" | grep "^Package:" | sed 's/Package: //' | sort -u
"#,
    pkg = pkg
  ));

  // Parse installed and essential lists
  let mut all_installed: HashSet<String> = HashSet::new();
  let mut essential_packages: Vec<String> = Vec::new();
  let mut section = 0;

  for line in setup_output.lines() {
    let trimmed = line.trim();
    if trimmed == "===INSTALLED===" {
      section = 1;
      continue;
    }
    if trimmed == "===ESSENTIAL===" {
      section = 2;
      continue;
    }
    match section {
      1 => {
        if !trimmed.is_empty() {
          all_installed.insert(trimmed.to_string());
        }
      }
      2 => {
        if !trimmed.is_empty() {
          essential_packages.push(trimmed.to_string());
        }
      }
      _ => {}
    }
  }

  // Rust-driven recursive BFS to build the complete dep graph
  let mut dep_graph: HashMap<String, Vec<Vec<String>>> = HashMap::new();
  let mut provides_map: HashMap<String, Vec<String>> = HashMap::new();
  let mut queried: HashSet<String> = HashSet::new();

  // Seed with essential packages + target
  let mut queue: VecDeque<String> = essential_packages.iter().cloned().collect();
  if !queue.contains(&pkg.to_string()) {
    queue.push_back(pkg.to_string());
  }

  while !queue.is_empty() {
    // Collect unqueried packages for this batch
    let mut batch: Vec<String> = Vec::new();
    while let Some(name) = queue.pop_front() {
      if queried.contains(&name) {
        continue;
      }
      queried.insert(name.clone());
      batch.push(name);
    }

    if batch.is_empty() {
      break;
    }

    // Query apt-cache depends for the batch via docker exec
    let pkg_list = batch.join(" ");
    let script = format!(
      r#"for p in {}; do echo "===PKG:$p==="; apt-cache depends --no-recommends --no-suggests --no-conflicts --no-breaks --no-replaces --no-enhances "$p" 2>/dev/null; done"#,
      pkg_list
    );
    let output = container.exec(&script);

    // Parse output and discover new packages
    let (batch_graph, batch_provides, discovered) = parse_apt_batch(&output);

    for (pkg_name, groups) in batch_graph {
      dep_graph.insert(pkg_name, groups);
    }
    for (virt, providers) in batch_provides {
      let entry = provides_map.entry(virt).or_default();
      for p in providers {
        if !entry.contains(&p) {
          entry.push(p);
        }
      }
    }

    // Queue newly discovered packages
    for name in discovered {
      if !queried.contains(&name) {
        queue.push_back(name);
      }
    }
  }

  // Container is automatically removed when `container` goes out of scope (Drop).

  // Build cross-provides from alternative groups
  let all_groups: Vec<Vec<String>> = dep_graph.values().flat_map(|groups| groups.iter().cloned()).collect();
  for group in &all_groups {
    if group.len() > 1 {
      for member in group {
        for other in group {
          if other != member {
            let entry = provides_map.entry(other.clone()).or_default();
            if !entry.contains(member) {
              entry.push(member.clone());
            }
          }
        }
      }
    }
  }

  (dep_graph, provides_map, all_installed, essential_packages)
}

/// Parse a batch of `===PKG:name===` sections from apt-cache depends output.
///
/// Returns: (dep_graph_entries, provides_entries, newly_discovered_package_names)
fn parse_apt_batch(text: &str) -> (HashMap<String, Vec<Vec<String>>>, HashMap<String, Vec<String>>, Vec<String>) {
  let mut dep_graph: HashMap<String, Vec<Vec<String>>> = HashMap::new();
  let mut provides_map: HashMap<String, Vec<String>> = HashMap::new();
  let mut discovered: Vec<String> = Vec::new();

  let mut current_pkg = String::new();
  let mut current_deps: Vec<Vec<String>> = Vec::new();
  let mut current_alt_group: Vec<String> = Vec::new();
  let mut in_virtual = false;
  let mut alt_from_pipe = false;

  for line in text.lines() {
    // New package section
    if let Some(pkg_name) = line.trim().strip_prefix("===PKG:").and_then(|s| s.strip_suffix("===")) {
      flush_pkg(&mut dep_graph, &mut current_pkg, &mut current_deps, &mut current_alt_group);
      current_pkg = pkg_name.to_string();
      current_deps.clear();
      current_alt_group.clear();
      in_virtual = false;
      alt_from_pipe = false;
      continue;
    }

    // Alternative dependency: " |Depends: name" or " |PreDepends: name"
    if line.starts_with(" |Depends: ") || line.starts_with(" |PreDepends: ") {
      let name = extract_dep_name(line);
      if !name.is_empty() {
        current_alt_group.push(name.clone());
        discovered.push(name);
        in_virtual = line.contains('<');
        alt_from_pipe = true;
      }
      continue;
    }

    // Provider line (4-space indent) under a virtual dep
    if line.starts_with("    ") && (in_virtual || !current_alt_group.is_empty()) {
      let name = clean_dep_name(line.trim());
      if !name.is_empty() {
        current_alt_group.push(name.clone());
        discovered.push(name);
      }
      continue;
    }

    // Regular dependency: "  Depends: name" or "  PreDepends: name"
    if line.starts_with("  Depends: ") || line.starts_with("  PreDepends: ") {
      let name = extract_dep_name(line);
      if name.is_empty() {
        continue;
      }
      discovered.push(name.clone());

      if !current_alt_group.is_empty() && alt_from_pipe {
        // Active alt group started by "|Depends:". This non-pipe Depends is
        // the final member of the same alternative group.
        current_alt_group.push(name.clone());
        if line.contains('<') {
          in_virtual = true;
          provides_map.entry(name).or_default();
        } else {
          in_virtual = false;
          alt_from_pipe = false;
          flush_alt_group(&mut current_deps, &mut current_alt_group);
        }
      } else if !current_alt_group.is_empty() && !alt_from_pipe {
        // Active alt group from a virtual "<name>" — this Depends is a NEW dep.
        flush_alt_group(&mut current_deps, &mut current_alt_group);
        in_virtual = false;
        if line.contains('<') {
          current_alt_group.push(name.clone());
          in_virtual = true;
          provides_map.entry(name).or_default();
        } else {
          current_deps.push(vec![name]);
        }
      } else {
        // No active alt group — standalone dependency
        in_virtual = false;
        if line.contains('<') {
          current_alt_group.push(name.clone());
          in_virtual = true;
          provides_map.entry(name).or_default();
        } else {
          current_deps.push(vec![name]);
        }
      }
      continue;
    }

    // Any other non-indented line — flush alt group
    if !line.starts_with(' ') && !line.starts_with('\t') {
      flush_alt_group(&mut current_deps, &mut current_alt_group);
      in_virtual = false;
      alt_from_pipe = false;
    }
  }

  flush_pkg(&mut dep_graph, &mut current_pkg, &mut current_deps, &mut current_alt_group);

  discovered.sort();
  discovered.dedup();

  (dep_graph, provides_map, discovered)
}

/// Flush the current package's deps into the dep graph.
fn flush_pkg(
  dep_graph: &mut HashMap<String, Vec<Vec<String>>>,
  current_pkg: &mut String,
  current_deps: &mut Vec<Vec<String>>,
  current_alt_group: &mut Vec<String>,
) {
  flush_alt_group(current_deps, current_alt_group);
  if !current_pkg.is_empty() {
    dep_graph.insert(current_pkg.clone(), current_deps.clone());
  }
  current_pkg.clear();
  current_deps.clear();
}

/// Extract and clean a dependency name from an apt-cache depends line.
fn extract_dep_name(line: &str) -> String {
  let raw = line.split(": ").nth(1).unwrap_or("").trim();
  clean_dep_name(raw)
}

/// Clean angle brackets and :any suffix from a dep name.
fn clean_dep_name(raw: &str) -> String {
  raw
    .trim_start_matches('<')
    .trim_end_matches('>')
    .trim_end_matches(":any")
    .to_string()
}

/// Flush a pending alternative group into the dep list.
fn flush_alt_group(deps: &mut Vec<Vec<String>>, alt_group: &mut Vec<String>) {
  if !alt_group.is_empty() {
    alt_group.sort();
    alt_group.dedup();
    deps.push(alt_group.clone());
    alt_group.clear();
  }
}
