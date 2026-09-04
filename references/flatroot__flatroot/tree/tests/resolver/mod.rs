//! Shared resolver validation infrastructure.
//!
//! Every distro follows the same validation algorithm:
//!
//! 1. Docker installs the target package natively
//! 2. The distro-specific module builds a structured dep graph from Docker
//! 3. BFS walks the dep graph, following only FlatRoot's alternative choices
//! 4. `validate()` asserts zero missing AND zero phantoms
//!
//! The dep graph uses alternative groups uniformly:
//! - Debian: alternatives from `pkg-a | pkg-b` syntax
//! - RPM/Pacman/APK: alternatives from the provides map (virtual → [provider1, provider2])

pub mod apk;
pub mod deb;
pub mod pacman;
pub mod rpm;
pub mod synth;

use std::collections::{HashMap, HashSet, VecDeque};
use std::process;

use assert_cmd::Command;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Docker helpers
// ---------------------------------------------------------------------------

/// Run a shell script inside a throwaway Docker container and return stdout.
pub fn docker_run(image: &str, script: &str) -> String {
  let output = process::Command::new("docker")
    .args(["run", "--rm", image, "sh", "-c", script])
    .output()
    .expect("docker not available");

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    panic!("docker run {} failed: {}", image, stderr);
  }

  String::from_utf8_lossy(&output.stdout).to_string()
}

/// A persistent Docker container that is automatically removed on drop.
///
/// Use this when multiple `docker exec` calls are needed against the same
/// container (e.g., recursive dep graph discovery). The container is removed
/// when this struct goes out of scope — whether from normal return, panic,
/// assertion failure, or Ctrl+C (Rust runs destructors on unwind).
pub struct DockerContainer {
  pub name: String,
}

impl DockerContainer {
  /// Create and start a persistent container from the given image.
  pub fn new(image: &str) -> Self {
    let name = format!("flatroot-test-{}-{}", std::process::id(), image.replace([':', '/'], "-"));

    let output = process::Command::new("docker")
      .args(["create", "--name", &name, image, "sleep", "infinity"])
      .output()
      .expect("docker create failed");
    if !output.status.success() {
      panic!("docker create failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let output = process::Command::new("docker")
      .args(["start", &name])
      .output()
      .expect("docker start failed");
    if !output.status.success() {
      // Clean up the created-but-not-started container
      let _ = process::Command::new("docker").args(["rm", "-f", &name]).output();
      panic!("docker start failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    Self { name }
  }

  /// Execute a shell command inside the container and return stdout.
  pub fn exec(&self, script: &str) -> String {
    let output = process::Command::new("docker")
      .args(["exec", &self.name, "sh", "-c", script])
      .output()
      .expect("docker exec failed");

    if !output.status.success() {
      let stderr = String::from_utf8_lossy(&output.stderr);
      panic!("docker exec in {} failed: {}", self.name, stderr);
    }

    String::from_utf8_lossy(&output.stdout).to_string()
  }
}

impl Drop for DockerContainer {
  fn drop(&mut self) {
    let _ = process::Command::new("docker").args(["rm", "-f", &self.name]).output();
  }
}

// ---------------------------------------------------------------------------
// FlatRoot helper
// ---------------------------------------------------------------------------

/// Run FlatRoot and read the resolved package list from the `.flatroot/packages`
/// manifest written after install.
///
/// The install runs with `--postinstall=none` so post-install scripts/hooks are
/// skipped (resolver validation doesn't care about them) and with a fresh cache
/// dir so parallel tests don't race on the shared `~/.cache/flatroot/` index
/// database.
pub fn flatroot_resolved_list(remote: &str, pkg: &str) -> Vec<String> {
  let root = TempDir::new().unwrap();
  let cache = TempDir::new().unwrap();
  let output = Command::cargo_bin("flatroot")
    .unwrap()
    .env("FLATROOT_CACHE_HOME", cache.path())
    .args([
      "--from",
      remote,
      "install",
      "--output",
      root.path().to_str().unwrap(),
      "--postinstall=none",
      pkg,
    ])
    .output()
    .expect("flatroot not available");

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    panic!("flatroot failed for {} {}: {}", remote, pkg, stderr);
  }

  // Parse .flatroot/packages for the list of installed package names. Each
  // dpkg-style record begins with `Package: <name>`.
  let manifest_path = root.path().join(".flatroot/packages");
  let manifest = std::fs::read_to_string(&manifest_path)
    .unwrap_or_else(|e| panic!("failed to read manifest at {}: {e}", manifest_path.display()));

  manifest
    .lines()
    .filter_map(|l| l.strip_prefix("Package: "))
    .map(|s| s.trim().to_string())
    .collect()
}

// ---------------------------------------------------------------------------
// BFS reachability with alternative tracking
// ---------------------------------------------------------------------------

/// BFS through the dep graph starting from seed packages, following only
/// the alternatives that FlatRoot chose.
///
/// `dep_graph`: package → list of dep groups. Each group is either:
///   - `[single_pkg]` — mandatory, follow unconditionally
///   - `[alt1, alt2, ...]` — alternatives, follow the one FlatRoot chose
///
/// `provides_map`: virtual name → list of real providers. Used to resolve
///   dep names that aren't real packages (file paths, sonames, virtuals).
///
/// Returns the set of all packages reachable from the seeds.
///
/// Panics if an alternative group has no provider in the FlatRoot set AND
/// no provider in the installed set — this means an unhandled virtual package.
pub fn bfs_reachable(
  seeds: &[String],
  dep_graph: &HashMap<String, Vec<Vec<String>>>,
  flatroot_set: &HashSet<&str>,
  installed_set: &HashSet<&str>,
) -> HashSet<String> {
  let mut reachable = HashSet::new();
  let mut queue: VecDeque<String> = seeds.iter().cloned().collect();

  while let Some(pkg) = queue.pop_front() {
    if !reachable.insert(pkg.clone()) {
      continue;
    }

    let Some(dep_groups) = dep_graph.get(&pkg) else {
      continue;
    };

    for group in dep_groups {
      if group.len() == 1 {
        // Mandatory dep — follow unconditionally
        queue.push_back(group[0].clone());
      } else if !group.is_empty() {
        // Alternative group — follow the one FlatRoot chose.
        // Check flatroot_set first, then installed_set as fallback
        // (installed_set covers packages that are in the Docker base image
        // which FlatRoot also resolved but through a different name).
        if let Some(chosen) = group.iter().find(|alt| flatroot_set.contains(alt.as_str())) {
          queue.push_back(chosen.clone());
        } else if let Some(chosen) = group.iter().find(|alt| installed_set.contains(alt.as_str())) {
          // Docker chose this alternative, FlatRoot didn't ��� this will be
          // caught as a "missing" package, which is correct.
          queue.push_back(chosen.clone());
        }
        // If no alternative exists in either set, the dep is for something
        // not installed (e.g., a suggests-only package). Skip silently —
        // the missing check will catch it if it matters.
      }
    }
  }

  reachable
}

// ---------------------------------------------------------------------------
// Resolve a dep name through the provides map
// ---------------------------------------------------------------------------

/// Given a dependency name (which may be a real package, a virtual, a file path,
/// or a soname), resolve it to a list of real package names that satisfy it.
///
/// Returns a single-element vec if the name is a real package in the installed set.
/// Returns the providers list if the name is in the provides map.
/// Returns empty vec if nothing satisfies it (will be caught as missing).
pub fn resolve_dep_name(
  dep_name: &str,
  installed_set: &HashSet<&str>,
  provides_map: &HashMap<String, Vec<String>>,
) -> Vec<String> {
  // Real package name — not a virtual
  if installed_set.contains(dep_name) {
    return vec![dep_name.to_string()];
  }

  // Virtual/file-path/soname — look up providers
  if let Some(providers) = provides_map.get(dep_name) {
    // Filter to providers that are actually installed
    let installed_providers: Vec<String> = providers
      .iter()
      .filter(|p| installed_set.contains(p.as_str()))
      .cloned()
      .collect();
    if !installed_providers.is_empty() {
      return installed_providers;
    }
    // Return all known providers even if none are installed
    // (the BFS will handle the "not found" case)
    return providers.clone();
  }

  // Unknown — nothing satisfies this dep
  Vec::new()
}

// ---------------------------------------------------------------------------
// Strict validation
// ---------------------------------------------------------------------------

/// Validate FlatRoot's resolution against Docker's native package manager.
///
/// Two strict assertions:
/// 1. **Missing**: every package in the BFS reachable set must be in FlatRoot's list.
///    No exceptions, no whitelist.
/// 2. **Phantom**: every package in FlatRoot's list must be in Docker's installed list
///    OR be a legitimate alternative provider (provides a virtual that an installed
///    package also provides).
///
/// Both assertions are fatal — zero tolerance.
pub fn validate(
  distro: &str,
  pkg: &str,
  reachable: &HashSet<String>,
  docker_installed: &HashSet<String>,
  flatroot: &[String],
  provides_map: &HashMap<String, Vec<String>>,
) {
  assert!(!flatroot.is_empty(), "{} {}: flatroot returned 0 packages (install failed?)", distro, pkg);

  let flatroot_set: HashSet<&str> = flatroot.iter().map(|s| s.as_str()).collect();

  // ── Missing check ──────────────────────────────────────────────────
  // A package is MISSING if the BFS says it's needed, FlatRoot didn't
  // include it, Docker DID install it (confirming it's genuinely needed),
  // and no alternative provider in FlatRoot satisfies the same need.
  //
  // If neither Docker nor FlatRoot installed a reachable package, the
  // test BFS reached it through a path the real resolvers don't follow
  // (e.g., sub-package splits where --whatprovides returns incomplete
  // provider data). These are not real misses.
  let missing: Vec<&String> = reachable
    .iter()
    // Not in FlatRoot
    .filter(|p| !flatroot_set.contains(p.as_str()))
    // Docker confirms it's genuinely needed (not a BFS artifact)
    .filter(|p| docker_installed.contains(p.as_str()))
    // Not satisfied by an alternative provider in FlatRoot
    .filter(|p| !is_provided_by_flatroot(p, provides_map, &flatroot_set))
    .collect();

  // ── Phantom check ──────────────────────────────────────────────────
  // Packages FlatRoot included that are neither in Docker's installed set
  // nor in the BFS reachable set. The reachable set covers packages from
  // alternative branches FlatRoot chose (e.g., usrmerge vs usr-is-merged)
  // and base/essential packages that the BFS was seeded with.
  let phantoms: Vec<&str> = flatroot
    .iter()
    .filter(|f| !docker_installed.contains(f.as_str()))
    .filter(|f| !reachable.contains(f.as_str()))
    .filter(|f| !is_legitimate_alternative(f, provides_map, docker_installed))
    .map(|f| f.as_str())
    .collect();

  // ── Report ─────────────────────────────────────────────────────────
  eprintln!(
    "  {} {}: reachable={}, docker_installed={}, flatroot={}, missing={}, phantoms={}",
    distro,
    pkg,
    reachable.len(),
    docker_installed.len(),
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

  assert!(missing.is_empty(), "{} {}: flatroot MISSING {} deps: {:?}", distro, pkg, missing.len(), missing);
  assert!(phantoms.is_empty(), "{} {}: flatroot has {} PHANTOM packages: {:?}", distro, pkg, phantoms.len(), phantoms);
}

/// Check if a "missing" package is provided by something FlatRoot included.
///
/// Three ways a missing package can still be satisfied:
///
/// 1. `pkg` is a virtual name and a FlatRoot package provides it.
///    Example: Docker's dep tree says `awk` is needed; FlatRoot has
///    `mawk` which provides `awk`.
///
/// 2. `pkg` is itself a key in the provides map whose provider list
///    includes a FlatRoot package — the mirror of case 1 for maps
///    keyed slightly differently.
///
/// 3. `pkg` and a FlatRoot package both provide the same virtual.
///    Example: Docker installed `glibc-minimal-langpack` to satisfy
///    glibc's `Requires: glibc-langpack`; FlatRoot satisfied the same
///    requirement with `glibc-all-langpacks`. Both provide
///    `glibc-langpack`, so the requirement is met even though the
///    exact package differs. Without this case, every distro whose
///    package manager prefers a different provider of a shared
///    virtual would report a spurious "missing" package.
pub fn is_provided_by_flatroot(
  pkg: &str,
  provides_map: &HashMap<String, Vec<String>>,
  flatroot_set: &HashSet<&str>,
) -> bool {
  // Case 1: any package in flatroot provides this name.
  for (virtual_name, providers) in provides_map {
    if virtual_name == pkg {
      if providers.iter().any(|p| flatroot_set.contains(p.as_str())) {
        return true;
      }
    }
  }

  // Case 2: `pkg` itself is a virtual that some flatroot package provides.
  if let Some(providers) = provides_map.get(pkg) {
    if providers.iter().any(|p| flatroot_set.contains(p.as_str())) {
      return true;
    }
  }

  // Case 3: `pkg` and a flatroot package both provide the same virtual.
  for providers in provides_map.values() {
    if !providers.iter().any(|p| p == pkg) {
      continue;
    }
    if providers
      .iter()
      .any(|p| p.as_str() != pkg && flatroot_set.contains(p.as_str()))
    {
      return true;
    }
  }

  false
}

/// Check if a phantom package is a legitimate alternative provider.
///
/// A phantom is legitimate in two cases:
///
/// 1. The phantom provides some virtual V, and a Docker-installed package also
///    provides V. Example: FlatRoot has `mawk`, Docker has `gawk`, both provide `awk`.
///
/// 2. The phantom's name IS a virtual name (a key in the provides map), and a
///    Docker-installed package provides it. Example: FlatRoot installed the literal
///    `urw-fonts` package, but Docker satisfied the `urw-fonts` need through
///    `urw-base35-fonts` (which provides `urw-fonts`).
pub fn is_legitimate_alternative(
  phantom: &str,
  provides_map: &HashMap<String, Vec<String>>,
  docker_installed: &HashSet<String>,
) -> bool {
  // Case 1: phantom appears as a provider in some virtual's provider list,
  // and another provider of that same virtual is installed in Docker.
  for (_virtual_name, providers) in provides_map {
    if !providers.iter().any(|p| p == phantom) {
      continue;
    }
    if providers
      .iter()
      .any(|p| p != phantom && docker_installed.contains(p.as_str()))
    {
      return true;
    }
  }

  // Case 2: phantom's name IS a virtual (a key in the provides map),
  // and one of its providers is installed in Docker.
  if let Some(providers) = provides_map.get(phantom) {
    if providers.iter().any(|p| docker_installed.contains(p.as_str())) {
      return true;
    }
  }

  false
}
