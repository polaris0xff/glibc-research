//! Arch Linux (pacman) resolver validation.
//!
//! Validates FlatRoot's dependency resolution against Docker's native package
//! manager by comparing installed package sets directly.

use std::collections::{HashMap, HashSet};

use super::{DockerContainer, flatroot_resolved_list, is_legitimate_alternative, is_provided_by_flatroot};

/// Full validation for an Arch Linux package.
pub fn validate_pacman(remote: &str, image: &str, pkg: &str) {
  let (mut provides_map, all_installed, newly_installed, container) = docker_pacman_install(image, pkg);
  let flatroot = flatroot_resolved_list(remote, pkg);

  // Query provides for FlatRoot-only packages
  let flatroot_only: Vec<&str> = flatroot
    .iter()
    .filter(|f| !all_installed.contains(f.as_str()))
    .map(|f| f.as_str())
    .collect();

  if !flatroot_only.is_empty() {
    let extra_list = flatroot_only.join(" ");
    let prov_output = container.exec(&format!(
      r#"for p in {extra_list}; do echo "===PKG:$p==="; pacman -Si "$p" 2>/dev/null | grep "^Provides" || true; done"#,
      extra_list = extra_list
    ));

    let mut cur = String::new();
    for line in prov_output.lines() {
      let trimmed = line.trim();
      if let Some(pkg_name) = trimmed.strip_prefix("===PKG:").and_then(|s| s.strip_suffix("===")) {
        cur = pkg_name.to_string();
        continue;
      }
      if cur.is_empty() || trimmed.is_empty() {
        continue;
      }
      if let Some(rest) = trimmed.strip_prefix("Provides        : ") {
        if rest.trim() != "None" {
          for prov in rest.split_whitespace() {
            let clean = strip_pacman_version(prov);
            if !clean.is_empty() && clean != cur {
              let entry = provides_map.entry(clean).or_default();
              if !entry.contains(&cur) {
                entry.push(cur.clone());
              }
            }
          }
        }
      }
    }
  }

  let flatroot_set: HashSet<&str> = flatroot.iter().map(|s| s.as_str()).collect();

  // Missing: Docker installed it for this package (not pre-existing in base
  // image), FlatRoot doesn't have it, and no FlatRoot package provides it.
  let missing: Vec<&String> = newly_installed
    .iter()
    .filter(|p| !flatroot_set.contains(p.as_str()))
    .filter(|p| !is_provided_by_flatroot(p, &provides_map, &flatroot_set))
    .collect();

  // Phantom: FlatRoot has it, Docker doesn't, and it's not a legitimate
  // alternative provider.
  let phantoms: Vec<&str> = flatroot
    .iter()
    .filter(|f| !all_installed.contains(f.as_str()))
    .filter(|f| !is_legitimate_alternative(f, &provides_map, &all_installed))
    .map(|f| f.as_str())
    .collect();

  eprintln!(
    "  {} {}: all_installed={}, newly_installed={}, flatroot={}, missing={}, phantoms={}",
    remote,
    pkg,
    all_installed.len(),
    newly_installed.len(),
    flatroot.len(),
    missing.len(),
    phantoms.len()
  );
  if !missing.is_empty() {
    eprintln!("    MISSING: {:?}", missing);
  }
  if !phantoms.is_empty() {
    eprintln!("    PHANTOMS: {:?}", phantoms);
  }

  assert!(missing.is_empty(), "{} {}: flatroot MISSING {} deps: {:?}", remote, pkg, missing.len(), missing);
  assert!(phantoms.is_empty(), "{} {}: flatroot has {} PHANTOM packages: {:?}", remote, pkg, phantoms.len(), phantoms);
}

/// Install a package in Docker and return provides map + installed set.
fn docker_pacman_install(
  image: &str,
  pkg: &str,
) -> (HashMap<String, Vec<String>>, HashSet<String>, HashSet<String>, DockerContainer) {
  let container = DockerContainer::new(image);

  // `-Syu` (sync repos + upgrade everything) before any `-S <pkg>`
  // install. A bare `-Sy` would leave installed packages on the
  // older base-image versions, which can be ABI-incompatible with
  // the freshly-rebuilt target package.
  container.exec("pacman -Syu --noconfirm 1>/dev/null");

  let pre_install_output = container.exec("pacman -Q 2>/dev/null | awk '{print $1}'");
  let pre_install_packages: HashSet<String> = pre_install_output
    .lines()
    .map(|l| l.trim().to_string())
    .filter(|l| !l.is_empty())
    .collect();

  container.exec(&format!("pacman -S --noconfirm {} 1>/dev/null", pkg));

  let after_output = container.exec("pacman -Q 2>/dev/null | awk '{print $1}'");
  let all_installed: HashSet<String> = after_output
    .lines()
    .map(|l| l.trim().to_string())
    .filter(|l| !l.is_empty())
    .collect();

  // Delta against the post-upgrade snapshot, so packages added by
  // `-Syu` are credited to the upgrade transaction rather than to
  // the target package's deps.
  let newly_installed: HashSet<String> = all_installed.difference(&pre_install_packages).cloned().collect();

  // Build provides map from installed packages via pacman -Qi
  let mut provides_map: HashMap<String, Vec<String>> = HashMap::new();
  let qi_output = container.exec("pacman -Qi 2>/dev/null");

  let mut current_name = String::new();
  for line in qi_output.lines() {
    if let Some(rest) = line.strip_prefix("Name            : ") {
      current_name = rest.trim().to_string();
    } else if let Some(rest) = line.strip_prefix("Provides        : ") {
      if rest.trim() != "None" && !current_name.is_empty() {
        for prov in rest.split_whitespace() {
          let clean = strip_pacman_version(prov);
          if !clean.is_empty() && clean != current_name {
            provides_map.entry(clean).or_default().push(current_name.clone());
          }
        }
      }
    }
  }

  for v in provides_map.values_mut() {
    v.sort();
    v.dedup();
  }

  (provides_map, all_installed, newly_installed, container)
}

/// Strip version constraints from a pacman dep/provides string.
fn strip_pacman_version(s: &str) -> String {
  if let Some(idx) = s.find(|c: char| c == '>' || c == '<' || c == '=') {
    s[..idx].to_string()
  } else {
    s.to_string()
  }
}
