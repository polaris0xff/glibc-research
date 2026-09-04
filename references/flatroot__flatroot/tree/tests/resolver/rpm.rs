//! CentOS/Fedora (RPM) resolver validation.
//!
//! Validates FlatRoot's dependency resolution against Docker's native package
//! manager by comparing installed package sets directly. Docker installs the
//! package natively, FlatRoot resolves independently, and the two sets are
//! compared with provides-based alternative detection.

use std::collections::{HashMap, HashSet};

use super::{DockerContainer, flatroot_resolved_list, is_legitimate_alternative, is_provided_by_flatroot};

/// RHEL-family base seed set — what flatroot's `centos`, `fedora`,
/// `alma`, and `rocky` source builders always add to the resolve set.
/// Mirrored from each distro module's `base_packages` so the
/// clean-root install reproduces flatroot's base-seeded closure.
const RPM_BASE_RHEL: &[&str] = &["filesystem", "basesystem", "bash", "coreutils", "glibc", "glibc-common"];

/// openSUSE base seed set — what flatroot's `opensuse` source builder
/// adds. openSUSE has no `basesystem` or `glibc-common` package; its
/// base set is smaller than the RHEL family's.
const RPM_BASE_OPENSUSE: &[&str] = &["filesystem", "bash", "coreutils", "glibc"];

/// Returns the base seed set flatroot uses for the given `--from`
/// remote, so the resolver test's clean-root install seeds the same
/// packages flatroot's resolver does.
fn rpm_base_set(remote: &str) -> &'static [&'static str] {
  if remote.starts_with("opensuse") {
    RPM_BASE_OPENSUSE
  } else {
    RPM_BASE_RHEL
  }
}

/// Container-side script that installs `__PKG__` (the placeholder
/// callers `str::replace` with the target package name) into a
/// blank `/tmp/cleanroot` so the resulting package list is the
/// from-scratch set dnf/yum/zypper would produce — the right
/// baseline to compare against flatroot's resolver, which itself
/// resolves from scratch.
///
/// Plain string concatenation, not `format!`, because the body
/// contains literal `${VAR}` shell expansions and `%{?macro}` rpm
/// expansions; both would clash with the `format!` argument
/// grammar.
const INSTALL_SCRIPT_RPM: &str = r#"
mkdir -p /tmp/cleanroot

# Rewrite dead mirror.centos.org URLs to vault.centos.org wherever they
# appear in the image. CentOS 7 ships a single CentOS-Base.repo; CentOS 8
# / Stream ship one repo file per component. The for-loop tolerates an
# unmatched glob via the inner `[ -f ]` test.
if [ -f /etc/yum.repos.d/CentOS-Base.repo ]; then
  sed -i 's|mirrorlist=|#mirrorlist=|g' /etc/yum.repos.d/CentOS-Base.repo
  sed -i 's|#baseurl=http://mirror.centos.org|baseurl=http://vault.centos.org|g' /etc/yum.repos.d/CentOS-Base.repo
fi
for f in /etc/yum.repos.d/CentOS-Linux-*.repo /etc/yum.repos.d/CentOS-Stream-*.repo; do
  [ -f "$f" ] || continue
  sed -i 's|mirrorlist=|#mirrorlist=|g' "$f"
  sed -i 's|#baseurl=http://mirror.centos.org|baseurl=http://vault.centos.org|g' "$f"
done

# Detect releasever for `dnf --installroot`. CentOS / RHEL define
# `%{rhel}`; Fedora defines `%{fedora}`; exactly one is defined per
# image, so concatenating them yields the right number.
RELEASEVER="$(rpm -E '%{?rhel}')$(rpm -E '%{?fedora}')"

if command -v zypper >/dev/null 2>&1; then
  # zypper --root reads repo config from the target tree, not the host,
  # so seed /etc/zypp inside the cleanroot from the running container's
  # config before invoking. PKI is copied alongside so signed repo
  # metadata still verifies. The leap repos template `$releasever` into
  # their baseurls; an empty cleanroot has no os-release for zypper to
  # read it from, so it is passed explicitly from the host's
  # /etc/os-release VERSION_ID.
  mkdir -p /tmp/cleanroot/etc/zypp/repos.d /tmp/cleanroot/etc/pki
  cp -r /etc/zypp/repos.d/. /tmp/cleanroot/etc/zypp/repos.d/
  cp -r /etc/pki/. /tmp/cleanroot/etc/pki/
  VERSION_ID="$(. /etc/os-release && echo "$VERSION_ID")"
  zypper --releasever "$VERSION_ID" --root /tmp/cleanroot --non-interactive --gpg-auto-import-keys install --no-recommends __PKG__
elif command -v yum >/dev/null 2>&1 && ! command -v dnf >/dev/null 2>&1; then
  yum --installroot=/tmp/cleanroot --releasever="$RELEASEVER" install -y __PKG__
else
  # DNF5 (Fedora 41+) requires --use-host-config so --installroot picks
  # up the running container's repos; DNF4 (CentOS 8 / older Fedora)
  # reads /etc/yum.repos.d by default and rejects the flag. Presence of
  # the `dnf5` binary is the version discriminator.
  if command -v dnf5 >/dev/null 2>&1; then
    HOST_CFG="--use-host-config"
  else
    HOST_CFG=""
  fi
  dnf install -y --installroot=/tmp/cleanroot $HOST_CFG --releasever="$RELEASEVER" --setopt=install_weak_deps=False --disablerepo='*cisco*' __PKG__
fi
"#;

/// Full validation for a CentOS or Fedora package.
pub fn validate_rpm(remote: &str, image: &str, pkg: &str) {
  // Docker installs with --setopt=install_weak_deps=False and --disablerepo='*cisco*'
  // to match FlatRoot's configuration (no Recommends, no third-party repos).
  //
  // Two installed sets:
  // - `all_installed` (base + new) — for phantom check. FlatRoot's base packages
  //   (filesystem, glibc, etc.) are pre-installed in Docker's base image, so
  //   they shouldn't be flagged as phantoms.
  // - `newly_installed` (new only) — for missing check. Base image packages
  //   (pam, gzip, readline) aren't deps of the target package — they're
  //   pre-existing. dnf didn't install them for this package, so they're not
  //   expected in FlatRoot's output.
  let (mut provides_map, all_installed, newly_installed, container) =
    docker_rpm_install(image, pkg, rpm_base_set(remote));
  let flatroot = flatroot_resolved_list(remote, pkg);

  // Query provides for FlatRoot-only packages (not installed by Docker).
  // Without this, the provides map only knows about Docker's installed
  // packages' provides, so legitimate alternatives chosen by FlatRoot
  // (e.g., glibc-all-langpacks vs glibc-minimal-langpack) can't be matched.
  let flatroot_only: Vec<&str> = flatroot
    .iter()
    .filter(|f| !all_installed.contains(f.as_str()))
    .map(|f| f.as_str())
    .collect();

  if !flatroot_only.is_empty() {
    let extra_list = flatroot_only.join(" ");
    let prov_output = container.exec(&format!(
      r#"for p in {extra_list}; do echo "===PKG:$p==="; dnf repoquery --provides "$p" 2>/dev/null || repoquery --provides "$p" 2>/dev/null; done"#,
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
      let prov_name = strip_rpm_provides(trimmed);
      if !prov_name.is_empty() {
        let entry = provides_map.entry(prov_name).or_default();
        if !entry.contains(&cur) {
          entry.push(cur.clone());
        }
      }
    }
  }

  let flatroot_set: HashSet<&str> = flatroot.iter().map(|s| s.as_str()).collect();

  // Missing: Docker installed it for this package, FlatRoot doesn't have it,
  // and no FlatRoot package provides it.
  // gpg-pubkey is filtered — it's an RPM database entry for imported signing
  // keys, not a real package from the repository.
  let missing: Vec<&String> = newly_installed
    .iter()
    .filter(|p| p.as_str() != "gpg-pubkey")
    .filter(|p| !flatroot_set.contains(p.as_str()))
    .filter(|p| !is_provided_by_flatroot(p, &provides_map, &flatroot_set))
    .collect();

  // Phantom: FlatRoot has it, Docker doesn't (even in base image), and
  // it's not a legitimate alternative provider.
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

/// Install a package in Docker and return provides map + installed sets.
///
/// 1. Capture base image packages (before install)
/// 2. Install the target package (matching FlatRoot's configuration)
/// 3. Capture full installed list (after install)
/// 4. Build provides map from all installed packages
///
/// Returns: (provides_map, all_installed, newly_installed, container)
///
/// The container is returned so the caller can make additional queries
/// (e.g., querying provides for FlatRoot-only packages).
fn docker_rpm_install(
  image: &str,
  pkg: &str,
  base: &[&str],
) -> (HashMap<String, Vec<String>>, HashSet<String>, HashSet<String>, DockerContainer) {
  let container = DockerContainer::new(image);

  // Step 1: install the target package into an empty `--installroot`,
  // not on top of the container's own filesystem.
  //
  // The container's base image ships its own selection of packages —
  // CentOS 8's base ships `libcurl-minimal` instead of the full
  // `libcurl`, for example — and a plain `dnf install` against that
  // baseline only records what dnf incrementally added on top of the
  // pre-existing state. flatroot resolves from scratch, so its set
  // includes every transitive dep regardless of base-image content.
  // Comparing flatroot's from-scratch set against a base-image-plus-
  // delta set produces spurious phantoms (`libssh`, `brotli`,
  // `libpsl` on CentOS 8) that are real deps flatroot correctly pulls
  // but the docker test environment happens to already satisfy.
  //
  // Installing into `/tmp/cleanroot` instead yields an apples-to-
  // apples baseline: the from-scratch set dnf would install on a
  // blank system, which is exactly what flatroot's resolver produces.
  //
  // Branches:
  //
  // - zypper present (openSUSE Leap/Tumbleweed): `--root` accepts an
  //   empty target; `--no-recommends` mirrors flatroot's no-weak-dep
  //   default.
  //
  // - CentOS 7 layout (`CentOS-Base.repo` present): rewrite dead
  //   `mirror.centos.org` URLs to `vault.centos.org` and install
  //   with `yum --installroot`. CentOS 7 predates weak-dep semantics.
  //
  // - CentOS 8 / Stream layout (`CentOS-Linux-*.repo` or
  //   `CentOS-Stream-*.repo` present): same vault rewrite against the
  //   per-component repo files, then install with `dnf --installroot`
  //   + weak-deps + cisco-repo flags. `--releasever` is supplied
  //   explicitly because dnf cannot infer it from an empty rootfs.
  //
  // - Default (modern Fedora and anything else with dnf): plain
  //   `dnf install --installroot` with the same flags.
  //
  // No `2>/dev/null` suppression — a failed install must surface
  // immediately, not be papered over and turn into a confusing
  // follow-up `dnf repoquery` failure.
  //
  // The install list is the distro's base seed set plus the target.
  // flatroot always seeds the base set and resolves its full closure
  // regardless of the requested package; installing the same base set
  // into the cleanroot keeps the two baselines comparable. Without it,
  // `dnf install bash` alone produces a far smaller set than flatroot's
  // base-seeded resolution and every base-closure package reads as a
  // spurious phantom.
  let pkg_list = format!("{} {}", base.join(" "), pkg);
  let install_script = INSTALL_SCRIPT_RPM.replace("__PKG__", &pkg_list);
  container.exec(&install_script);

  // Step 2: capture the clean-root installed list. This IS the
  // baseline — `all_installed` equals `newly_installed` for the
  // clean-root strategy because every package in the root was
  // installed for this target.
  let after_output = container.exec("rpm -qa --root=/tmp/cleanroot --qf '%{NAME}\\n' | sort -u");
  let all_installed: HashSet<String> = after_output
    .lines()
    .map(|l| l.trim().to_string())
    .filter(|l| !l.is_empty())
    .collect();
  let newly_installed = all_installed.clone();

  // Step 3: build provides map from the clean-root packages.
  let mut provides_map: HashMap<String, Vec<String>> = HashMap::new();

  let provides_output = container.exec(
    r#"for p in $(rpm -qa --root=/tmp/cleanroot --qf '%{NAME}\n' | sort -u); do echo "===PKG:$p==="; rpm -q --root=/tmp/cleanroot --provides "$p" 2>/dev/null; done"#
  );

  let mut current_pkg = String::new();
  for line in provides_output.lines() {
    let trimmed = line.trim();
    if let Some(pkg_name) = trimmed.strip_prefix("===PKG:").and_then(|s| s.strip_suffix("===")) {
      current_pkg = pkg_name.to_string();
      continue;
    }
    if current_pkg.is_empty() || trimmed.is_empty() {
      continue;
    }
    let prov_name = strip_rpm_provides(trimmed);
    if !prov_name.is_empty() {
      let entry = provides_map.entry(prov_name).or_default();
      if !entry.contains(&current_pkg) {
        entry.push(current_pkg.clone());
      }
    }
  }

  // Deduplicate
  for v in provides_map.values_mut() {
    v.sort();
    v.dedup();
  }

  (provides_map, all_installed, newly_installed, container)
}

/// Strip version constraints and arch markers from an RPM provides line.
///
/// Examples:
/// - `"libfoo.so.6()(64bit)"` → `"libfoo.so.6"`
/// - `"pkg = 1.0-1.fc42"` → `"pkg"`
/// - `"libc.so.6(GLIBC_2.17)(64bit)"` → `"libc.so.6(GLIBC_2.17)"`
fn strip_rpm_provides(raw: &str) -> String {
  let s = raw.trim();

  // Strip "()(64bit)" suffix
  let s = if let Some(idx) = s.find("()(64bit)") {
    &s[..idx]
  } else if s.ends_with("(64bit)") {
    s.trim_end_matches("(64bit)")
  } else {
    s
  };

  // Strip version constraints: " >= 1.0", " = 1.0", " < 2.0"
  if let Some(idx) = s
    .find(" >= ")
    .or(s.find(" <= "))
    .or(s.find(" = "))
    .or(s.find(" > "))
    .or(s.find(" < "))
  {
    return s[..idx].trim().to_string();
  }

  s.trim().to_string()
}
