//! Alpine Linux (apk) resolver validation.
//!
//! Validates FlatRoot's dependency resolution against Docker's native package
//! manager by comparing installed package sets directly. Includes install-if
//! handling: FlatRoot-only packages are checked for install-if conditions
//! to distinguish legitimate triggers from phantoms.

use std::collections::{HashMap, HashSet};

use super::{DockerContainer, flatroot_resolved_list, is_legitimate_alternative, is_provided_by_flatroot};

/// Full validation for an Alpine Linux package.
pub fn validate_apk(remote: &str, image: &str, pkg: &str) {
  let (mut provides_map, all_installed, newly_installed, container) = docker_apk_install(image, pkg);
  let flatroot = flatroot_resolved_list(remote, pkg);

  // Query provides and install-if for FlatRoot-only packages.
  // FlatRoot may include packages via install-if triggers that Docker
  // also installs — or packages that are legitimate alternative providers.
  let flatroot_only: Vec<&str> = flatroot
    .iter()
    .filter(|f| !all_installed.contains(f.as_str()))
    .map(|f| f.as_str())
    .collect();

  // Track install-if conditions for phantom evaluation
  let mut install_if_map: HashMap<String, Vec<String>> = HashMap::new();

  if !flatroot_only.is_empty() {
    let extra_list = flatroot_only.join(" ");
    let extra_output = container.exec(&format!(
      r#"for p in {extra_list}; do
  echo "===PKG:$p==="
  echo "===PROVIDES==="
  apk info -a "$p" 2>/dev/null | sed -n '/provides:/,/^$/p'
  echo "===INSTALLIF==="
  apk info -a "$p" 2>/dev/null | sed -n '/has auto-install rule:/,/^$/p'
done"#,
      extra_list = extra_list
    ));

    let mut cur = String::new();
    let mut in_provides = false;
    let mut in_installif = false;

    for line in extra_output.lines() {
      let trimmed = line.trim();
      if let Some(pkg_name) = trimmed.strip_prefix("===PKG:").and_then(|s| s.strip_suffix("===")) {
        cur = pkg_name.to_string();
        in_provides = false;
        in_installif = false;
        continue;
      }
      if trimmed == "===PROVIDES===" {
        in_provides = true;
        in_installif = false;
        continue;
      }
      if trimmed == "===INSTALLIF===" {
        in_provides = false;
        in_installif = true;
        continue;
      }
      if cur.is_empty() || trimmed.is_empty() {
        if in_provides {
          in_provides = false;
        }
        if in_installif {
          in_installif = false;
        }
        continue;
      }

      if in_provides {
        if trimmed == "provides:" || trimmed.ends_with(" provides:") {
          continue;
        }
        let prov_name = strip_apk_version(trimmed);
        if !prov_name.is_empty() && prov_name != cur {
          let entry = provides_map.entry(prov_name).or_default();
          if !entry.contains(&cur) {
            entry.push(cur.clone());
          }
        }
      }

      if in_installif {
        if trimmed.ends_with(" has auto-install rule:") {
          continue;
        }
        let cond = strip_apk_version(trimmed);
        if !cond.is_empty() {
          install_if_map.entry(cur.clone()).or_default().push(cond);
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

  // Phantom: FlatRoot has it, Docker doesn't, it's not a legitimate
  // alternative, AND it's not a satisfied install-if trigger.
  let phantoms: Vec<&str> = flatroot
    .iter()
    .filter(|f| !all_installed.contains(f.as_str()))
    .filter(|f| !is_legitimate_alternative(f, &provides_map, &all_installed))
    .filter(|f| {
      // Check if this phantom is an install-if trigger with all conditions met
      if let Some(conditions) = install_if_map.get(f.as_str()) {
        let all_met = conditions.iter().all(|cond| {
          flatroot_set.contains(cond.as_str())
            || provides_map
              .get(cond)
              .is_some_and(|provs| provs.iter().any(|p| flatroot_set.contains(p.as_str())))
        });
        !all_met // keep as phantom only if conditions are NOT all met
      } else {
        true // no install-if rule — still a phantom
      }
    })
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
fn docker_apk_install(
  image: &str,
  pkg: &str,
) -> (HashMap<String, Vec<String>>, HashSet<String>, HashSet<String>, DockerContainer) {
  let container = DockerContainer::new(image);

  // Capture base image packages before installing
  let before_output = container.exec("apk info 2>/dev/null | sort");
  let base_packages: HashSet<String> = before_output
    .lines()
    .map(|l| l.trim().to_string())
    .filter(|l| !l.is_empty())
    .collect();

  // Install the package
  container.exec(&format!("apk update 2>/dev/null 1>/dev/null && apk add -q {} 2>/dev/null 1>/dev/null", pkg));

  // Capture full installed list after install
  let after_output = container.exec("apk info 2>/dev/null | sort");
  let all_installed: HashSet<String> = after_output
    .lines()
    .map(|l| l.trim().to_string())
    .filter(|l| !l.is_empty())
    .collect();

  let newly_installed: HashSet<String> = all_installed.difference(&base_packages).cloned().collect();

  // Build provides map from installed packages
  let mut provides_map: HashMap<String, Vec<String>> = HashMap::new();
  let prov_output = container.exec(
    r#"for p in $(apk info 2>/dev/null | sort); do echo "===PKG:$p==="; apk info -a "$p" 2>/dev/null | sed -n '/provides:/,/^$/p'; done"#,
  );

  let mut current_pkg = String::new();
  let mut in_provides = false;

  for line in prov_output.lines() {
    let trimmed = line.trim();
    if let Some(pkg_name) = trimmed.strip_prefix("===PKG:").and_then(|s| s.strip_suffix("===")) {
      current_pkg = pkg_name.to_string();
      in_provides = false;
      continue;
    }
    if trimmed == "provides:" || trimmed.ends_with(" provides:") {
      in_provides = true;
      // Extract package name from "busybox-1.37.0-r31 provides:" header
      if trimmed.ends_with(" provides:") {
        let prefix = trimmed.strip_suffix(" provides:").unwrap_or("");
        let mut sorted: Vec<&str> = all_installed.iter().map(|s| s.as_str()).collect();
        sorted.sort_by(|a, b| b.len().cmp(&a.len()));
        for installed in &sorted {
          if prefix.starts_with(&format!("{}-", installed)) {
            current_pkg = installed.to_string();
            break;
          }
        }
      }
      continue;
    }
    if in_provides {
      if trimmed.is_empty() {
        in_provides = false;
        continue;
      }
      let prov_name = strip_apk_version(trimmed);
      if !prov_name.is_empty() && prov_name != current_pkg {
        provides_map.entry(prov_name).or_default().push(current_pkg.clone());
      }
    }
  }

  for v in provides_map.values_mut() {
    v.sort();
    v.dedup();
  }

  (provides_map, all_installed, newly_installed, container)
}

/// Strip version constraints from an APK dep/provides string.
fn strip_apk_version(s: &str) -> String {
  let ops = [">=", "<=", "~=", ">", "<", "=", "~"];
  for op in &ops {
    if let Some(idx) = s.find(op) {
      return s[..idx].to_string();
    }
  }
  s.to_string()
}
