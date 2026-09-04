//! Source-construction tests for the per-distro `--from <distro>:<release>[@<date>]`
//! resolution. These call the public `flatroot::distro` API directly and assert
//! the concrete `SourceDistro` a request resolves to — its mirrors, repository
//! layout, cache id, native arch, and base-package seed — entirely offline. No
//! network is touched: `DistroRegistry::source` and the per-distro `source`
//! builders only construct the description; fetching happens later. The build-time
//! refusals (unknown distro, unsupported arch, bad release, malformed `@date`)
//! are asserted with the exact `bail!`/`anyhow!` text from the source.

use std::sync::Arc;
use std::time::Duration;

use flatroot::arch::Arch;
use flatroot::distro::{DistroRegistry, FromSelector, LayoutRepository};
use flatroot::internal::http::HttpClient;
use flatroot::mirror::Mirror;
use flatroot::remote::{Remote, RemoteDistro};

/// An inert HTTP client. Source construction never fetches, so the cache dir
/// and retry budget are immaterial — this just satisfies the signature.
fn http() -> Arc<HttpClient> {
  Arc::new(HttpClient::new(std::env::temp_dir().join("flatroot-distro-source-test"), 3, Duration::from_secs(3600)))
}

/// Build a `SourceDistro` from a `<distro>:<release>` spec and target arch.
fn source(spec: &str, arch: Arch) -> anyhow::Result<flatroot::distro::SourceDistro> {
  let selector = FromSelector::parse(spec)?;
  DistroRegistry::source(&selector, arch, &http())
}

/// Collect the base URLs of every mirror across every repo of a source, in
/// repo-then-mirror-preference order.
fn mirror_bases(src: &flatroot::distro::SourceDistro) -> Vec<String> {
  let mut bases = Vec::new();
  for repo in &src.repos {
    for m in repo.mirrors.iter() {
      bases.push(m.url_base().to_string());
    }
  }
  bases
}

/// The `path_dir_repo` of a single RPM-layout repo by label substring.
fn rpm_repo_path(src: &flatroot::distro::SourceDistro, label_needle: &str) -> String {
  let repo = src
    .repos
    .iter()
    .find(|r| r.label.contains(label_needle))
    .unwrap_or_else(|| {
      panic!(
        "no repo with label containing '{}' in {:?}",
        label_needle,
        src.repos.iter().map(|r| &r.label).collect::<Vec<_>>()
      )
    });
  match &repo.layout {
    LayoutRepository::Rpm { path_dir_repo } => path_dir_repo.clone(),
    other => panic!("repo '{}' is not RPM layout: {:?}", repo.label, other),
  }
}

// ---------------------------------------------------------------------------
// --from parse + registry guards (DIST-029, DIST-030, DIST-031, DIST-050)
// ---------------------------------------------------------------------------

// covers: DIST-029
#[test]
fn from_selector_requires_colon() {
  // A colon-less spec is rejected by FromSelector::parse with the exact shape
  // message (distro/mod.rs:302-310). Source-building commands route through
  // RemoteDistro::from_str → FromSelector::parse.
  let err = RemoteDistro::from_str("debian", Arch::X86_64, http())
    .err()
    .expect("expected an error");
  assert_eq!(err.to_string(), "expected <distro>:<release>, got 'debian'", "missing-colon bail text");
}

// covers: DIST-030
#[test]
fn unknown_distro_prefix_rejected_with_supported_list() {
  // DistroRegistry::source bails "Unknown remote '<prefix>:<release>'. Supported: ..."
  // (distro/mod.rs:444-452) — distinct from by_prefix's "not a supported distro".
  let err = source("gentoo:stable", Arch::X86_64).err().expect("expected an error");
  let msg = err.to_string();
  assert_eq!(
    msg,
    "Unknown remote 'gentoo:stable'. Supported: debian, ubuntu, arch, alpine, centos, fedora, alma, rocky, opensuse, cachyos",
    "unknown-remote bail text"
  );
}

// covers: DIST-031
#[test]
fn arch_unsupported_by_distro_rejected() {
  // Arch Linux supports only x86_64 (distro/arch.rs:25). Requesting aarch64
  // bails with the display name + uname + the supported set (distro/mod.rs:453-461).
  let err = source("arch:rolling", Arch::Aarch64).err().expect("expected an error");
  assert_eq!(err.to_string(), "Arch Linux does not support architecture 'aarch64'. Supported: x86_64");

  // CachyOS likewise (distro/cachyos.rs:25).
  let err = source("cachyos:rolling", Arch::Aarch64)
    .err()
    .expect("expected an error");
  assert_eq!(err.to_string(), "CachyOS does not support architecture 'aarch64'. Supported: x86_64");
}

// covers: DIST-050
#[test]
fn unknown_arch_token_rejected_before_source_work() {
  // `--arch mips64` fails at Arch::from_uname (arch.rs:130-139) before any
  // source is built. main.rs parses --arch via from_uname for every command.
  let err = Arch::from_uname("mips64").err().expect("expected an error");
  assert_eq!(err.to_string(), "Unsupported architecture 'mips64'");
}

// ---------------------------------------------------------------------------
// Per-distro release-validation bails (DIST-032, DIST-035, DIST-037, DIST-038, DIST-040)
// ---------------------------------------------------------------------------

// covers: DIST-032
#[test]
fn arch_malformed_date_rejected() {
  // arch:rolling@2024-06 has a two-component date; distro/arch.rs:68-81 bails.
  let err = source("arch:rolling@2024-06", Arch::X86_64)
    .err()
    .expect("expected an error");
  assert_eq!(err.to_string(), "Invalid date format '2024-06'. Expected YYYY-MM-DD");
}

// covers: DIST-035
#[test]
fn cachyos_rejects_non_rolling_release() {
  // distro/cachyos.rs:73-76
  let err = source("cachyos:stable", Arch::X86_64).err().expect("expected an error");
  assert_eq!(err.to_string(), "CachyOS only supports 'rolling' release. Use: cachyos:rolling");
}

// covers: DIST-037
#[test]
fn alma_rejects_release_outside_8_9() {
  // distro/alma.rs:66-69
  let err = source("alma:10", Arch::X86_64).err().expect("expected an error");
  assert_eq!(err.to_string(), "Unknown AlmaLinux release '10'. Supported: 8, 9");
}

// covers: DIST-038
#[test]
fn rocky_rejects_release_outside_8_9() {
  // distro/rocky.rs:62-65
  let err = source("rocky:7", Arch::X86_64).err().expect("expected an error");
  assert_eq!(err.to_string(), "Unknown Rocky Linux release '7'. Supported: 8, 9");
}

// covers: DIST-040
#[test]
fn centos_rejects_unknown_release() {
  // distro/centos.rs:80
  let err = source("centos:9", Arch::X86_64).err().expect("expected an error");
  assert_eq!(err.to_string(), "Unknown CentOS release '9'. Supported: 7, 8, stream9");
}

// ---------------------------------------------------------------------------
// Debian / Ubuntu @date snapshot routing (DIST-044, DIST-046)
// ---------------------------------------------------------------------------

// covers: DIST-044
#[test]
fn debian_at_date_routes_to_single_snapshot_mirror() {
  // debian:bookworm@2024-06-15 → one snapshot mirror at snapshot.debian.org
  // with the stamp + T000000Z form, suite stripped of the @date, cache_id
  // keeping the @date verbatim (distro/debian.rs:73-110).
  let src = source("debian:bookworm@2024-06-15", Arch::X86_64).unwrap();
  assert_eq!(src.cache_id, "debian/bookworm@2024-06-15/amd64", "cache_id keeps the @date and native arch");
  assert_eq!(src.release, "bookworm@2024-06-15", "release carries the @date verbatim");

  let bases = mirror_bases(&src);
  assert_eq!(bases.len(), 1, "snapshot uses a single mirror, got {:?}", bases);
  assert_eq!(
    bases[0], "https://snapshot.debian.org/archive/debian/20240615T000000Z",
    "snapshot base must be stamp (dashes removed) + T000000Z"
  );

  // The repo's suite directory drops the @date — it is dists/<suite> only.
  let repo = &src.repos[0];
  match &repo.layout {
    LayoutRepository::Deb { path_dir_suite, .. } => {
      assert_eq!(path_dir_suite.as_str(), "dists/bookworm", "suite dir must strip the @date");
    }
    other => panic!("debian repo must be Deb layout: {:?}", other),
  }
}

// covers: DIST-044
#[test]
fn debian_declares_per_arch_and_all_contents_listings() {
  // Debian splits file ownership across Contents-<arch> (arch-specific
  // packages) and Contents-all (`Architecture: all` packages) on the live
  // and snapshot mirrors, so the fact-sheet declares both, in that order.
  // The long-term archive merges the two and drops Contents-all; its
  // absence is tolerated at fetch time, not modelled here.
  let src = source("debian:bookworm", Arch::X86_64).unwrap();
  let repo = &src.repos[0];
  match &repo.layout {
    LayoutRepository::Deb {
      paths_file_contents, ..
    } => {
      assert_eq!(
        paths_file_contents,
        &vec![
          "dists/bookworm/main/Contents-amd64.gz".to_string(),
          "dists/bookworm/main/Contents-all.gz".to_string(),
        ]
      );
    }
    other => panic!("debian repo must be Deb layout: {:?}", other),
  }
}

// covers: DIST-044
#[test]
fn debian_at_date_snapshot_across_arches() {
  // The snapshot routing is identical across every supported arch; only the
  // native (debian) arch naming changes. This exercises the non-x86_64 arch
  // axis for snapshots (distro/debian.rs:28 ARCHS_SUPPORTED).
  for (arch, deb) in [
    (Arch::X86_64, "amd64"),
    (Arch::Aarch64, "arm64"),
    (Arch::I686, "i386"),
    (Arch::Armv7l, "armhf"),
    (Arch::Riscv64, "riscv64"),
  ] {
    let src = source("debian:bookworm@2024-06-15", arch).unwrap();
    assert_eq!(src.native_arch, deb, "native arch for {:?}", arch);
    assert_eq!(src.cache_id, format!("debian/bookworm@2024-06-15/{deb}"));
    let bases = mirror_bases(&src);
    assert_eq!(bases, vec!["https://snapshot.debian.org/archive/debian/20240615T000000Z".to_string()]);
  }
}

// covers: DIST-046
#[test]
fn ubuntu_at_date_routes_archive_and_security_to_one_snapshot() {
  // ubuntu:noble@2024-06-15 routes BOTH the archive repos and the security
  // repos to snapshot.ubuntu.com (distro/ubuntu.rs:73-90), and cache_id keeps
  // the @date.
  let src = source("ubuntu:noble@2024-06-15", Arch::X86_64).unwrap();
  assert_eq!(src.cache_id, "ubuntu/noble@2024-06-15/amd64");

  let bases = mirror_bases(&src);
  assert!(!bases.is_empty(), "ubuntu must build repos");
  let snapshot = "https://snapshot.ubuntu.com/ubuntu/20240615T000000Z";
  assert!(
    bases.iter().all(|b| b.as_str() == snapshot),
    "every repo (archive + security) must point at the single snapshot mirror, got {:?}",
    bases
  );

  // The security repo is present and also snapshot-routed.
  let security = src
    .repos
    .iter()
    .find(|r| r.label.contains("-security/"))
    .expect("ubuntu @date must still build a -security repo");
  let sec_bases: Vec<&str> = security.mirrors.iter().map(|m| m.url_base()).collect();
  assert_eq!(sec_bases, vec![snapshot], "security repo must be snapshot-routed too");
}

// ---------------------------------------------------------------------------
// Ubuntu unpinned 6-repo structure (DIST-047)
// ---------------------------------------------------------------------------

// covers: DIST-047
#[test]
fn ubuntu_unpinned_builds_six_repos_contents_only_on_base_main() {
  // Unpinned ubuntu:noble builds suite + suite-updates + suite-security, each
  // crossed with main/universe → 6 repos. Only the base-suite main repo carries
  // the suite-level Contents path; base_packages is empty (the Essential flag
  // drives the seed) (distro/ubuntu.rs:92-133).
  let src = source("ubuntu:noble", Arch::X86_64).unwrap();
  assert_eq!(
    src.repos.len(),
    6,
    "noble must build 6 repos: {:?}",
    src.repos.iter().map(|r| &r.label).collect::<Vec<_>>()
  );
  assert!(src.base_packages.is_empty(), "ubuntu base_packages must be empty (Essential drives the seed)");

  let labels: Vec<&str> = src.repos.iter().map(|r| r.label.as_str()).collect();
  for expected in [
    "ubuntu:noble/main",
    "ubuntu:noble/universe",
    "ubuntu:noble-updates/main",
    "ubuntu:noble-updates/universe",
    "ubuntu:noble-security/main",
    "ubuntu:noble-security/universe",
  ] {
    assert!(labels.contains(&expected), "missing repo {}: {:?}", expected, labels);
  }

  // Exactly one repo carries Contents listings, and it is the base
  // suite/main one. Ubuntu folds `Architecture: all` packages into its
  // per-arch Contents, so the suite-level listing is the single entry —
  // there is no separate `Contents-all` to declare.
  let mut with_contents = Vec::new();
  for repo in &src.repos {
    if let LayoutRepository::Deb {
      paths_file_contents, ..
    } = &repo.layout
      && !paths_file_contents.is_empty()
    {
      with_contents.push(repo.label.clone());
    }
  }
  assert_eq!(with_contents, vec!["ubuntu:noble/main".to_string()], "Contents must be on base suite main only");

  let base_main = src.repos.iter().find(|r| r.label == "ubuntu:noble/main").unwrap();
  if let LayoutRepository::Deb {
    paths_file_contents, ..
  } = &base_main.layout
  {
    assert_eq!(paths_file_contents, &vec!["dists/noble/Contents-amd64.gz".to_string()]);
  }
}

// covers: DIST-047
#[test]
fn ubuntu_unpinned_archive_falls_back_to_old_releases() {
  // The archive repos carry the ordered fallback archive.ubuntu.com →
  // old-releases.ubuntu.com; security points only at security.ubuntu.com.
  let src = source("ubuntu:jammy", Arch::X86_64).unwrap();
  let main = src.repos.iter().find(|r| r.label == "ubuntu:jammy/main").unwrap();
  let main_bases: Vec<&str> = main.mirrors.iter().map(|m| m.url_base()).collect();
  assert_eq!(
    main_bases,
    vec![
      "https://archive.ubuntu.com/ubuntu",
      "https://old-releases.ubuntu.com/ubuntu"
    ],
    "archive repo must fall back archive → old-releases"
  );
  let security = src
    .repos
    .iter()
    .find(|r| r.label == "ubuntu:jammy-security/main")
    .unwrap();
  let sec_bases: Vec<&str> = security.mirrors.iter().map(|m| m.url_base()).collect();
  assert_eq!(sec_bases, vec!["https://security.ubuntu.com/ubuntu"], "security repo is a single mirror");
}

// ---------------------------------------------------------------------------
// Arch @date snapshot routing (DIST-033)
// ---------------------------------------------------------------------------

// covers: DIST-033
#[test]
fn arch_at_date_routes_to_archive_repos_y_m_d() {
  // arch:rolling@2024-06-15 → archive.archlinux.org/repos/2024/06/15 as a
  // single snapshot mirror; the live geo mirror is unused; cache_id keeps the
  // @date (distro/arch.rs:66-84).
  let src = source("arch:rolling@2024-06-15", Arch::X86_64).unwrap();
  assert_eq!(src.cache_id, "arch/rolling@2024-06-15/x86_64", "cache_id keeps the @date");
  assert_eq!(src.native_arch, "x86_64");

  let bases = mirror_bases(&src);
  assert!(!bases.is_empty(), "arch must build repos");
  let snapshot = "https://archive.archlinux.org/repos/2024/06/15";
  assert!(bases.iter().all(|b| b.as_str() == snapshot), "every repo must point at the dated snapshot, got {:?}", bases);
  assert!(
    !bases.iter().any(|b| b.contains("geo.mirror.pkgbuild.com")),
    "the live mirror must not be used for a dated request: {:?}",
    bases
  );
}

// ---------------------------------------------------------------------------
// CentOS per-release host + layout, arch axis (DIST-039)
// ---------------------------------------------------------------------------

// covers: DIST-039
#[test]
fn centos_per_release_host_and_layout_across_arches() {
  // 7/8 live on the vault with their frozen point-version paths; stream9 lives
  // on the stream host with its 9-stream path. The arch token threads into
  // every path_dir_repo (distro/centos.rs:62-81). aarch64 is the untested axis.
  for arch in [Arch::X86_64, Arch::Aarch64] {
    let uname = arch.as_uname();

    let c7 = source("centos:7", arch).unwrap();
    assert_eq!(rpm_repo_path(&c7, "centos:7/os"), format!("centos/7/os/{uname}"));
    assert_eq!(rpm_repo_path(&c7, "centos:7/updates"), format!("centos/7/updates/{uname}"));
    assert!(
      mirror_bases(&c7)
        .iter()
        .all(|b| b.as_str() == "https://vault.centos.org"),
      "centos:7 must use the vault"
    );

    let c8 = source("centos:8", arch).unwrap();
    assert_eq!(rpm_repo_path(&c8, "centos:8/BaseOS"), format!("centos/8.5.2111/BaseOS/{uname}/os"));
    assert_eq!(rpm_repo_path(&c8, "centos:8/AppStream"), format!("centos/8.5.2111/AppStream/{uname}/os"));
    assert!(
      mirror_bases(&c8)
        .iter()
        .all(|b| b.as_str() == "https://vault.centos.org"),
      "centos:8 must use the vault"
    );

    let s9 = source("centos:stream9", arch).unwrap();
    assert_eq!(rpm_repo_path(&s9, "centos:stream9/BaseOS"), format!("9-stream/BaseOS/{uname}/os"));
    assert_eq!(rpm_repo_path(&s9, "centos:stream9/AppStream"), format!("9-stream/AppStream/{uname}/os"));
    assert!(
      mirror_bases(&s9)
        .iter()
        .all(|b| b.as_str() == "https://mirror.stream.centos.org"),
      "centos:stream9 must use the stream host"
    );
  }
}

// ---------------------------------------------------------------------------
// Fedora rawhide vs numbered, arch axis + archive fallback (DIST-041, DIST-042, DIST-074)
// ---------------------------------------------------------------------------

// covers: DIST-041
#[test]
fn fedora_rawhide_single_mirror_one_repo_across_arches() {
  // rawhide → a single primary mirror and one development/rawhide repo, for
  // every supported arch (distro/fedora.rs:68-104, ARCHS_SUPPORTED line 24).
  for arch in [Arch::X86_64, Arch::Aarch64, Arch::I686] {
    let uname = arch.as_uname();
    let src = source("fedora:rawhide", arch).unwrap();
    assert_eq!(src.repos.len(), 1, "rawhide must build exactly one repo");
    assert_eq!(rpm_repo_path(&src, "fedora:rawhide/Everything"), format!("development/rawhide/Everything/{uname}/os"));
    let bases = mirror_bases(&src);
    assert_eq!(
      bases,
      vec!["https://dl.fedoraproject.org/pub/fedora/linux".to_string()],
      "rawhide is a single primary mirror, no archive fallback"
    );
  }
}

// covers: DIST-042
#[test]
fn fedora_numbered_uses_primary_then_archive_fallback() {
  // A numbered release builds two repos (releases + updates), each carrying the
  // ordered fallback primary → archives.fedoraproject.org (distro/fedora.rs:70-104).
  // fedora:30 is the deeply-archived release the gap-spec calls out.
  for arch in [Arch::X86_64, Arch::Aarch64, Arch::I686] {
    let uname = arch.as_uname();
    let src = source("fedora:30", arch).unwrap();
    assert_eq!(src.repos.len(), 2, "numbered fedora must build releases + updates");
    assert_eq!(rpm_repo_path(&src, "fedora:30/releases"), format!("releases/30/Everything/{uname}/os"));
    assert_eq!(rpm_repo_path(&src, "fedora:30/updates"), format!("updates/30/Everything/{uname}"));

    let releases = src.repos.iter().find(|r| r.label == "fedora:30/releases").unwrap();
    let bases: Vec<&str> = releases.mirrors.iter().map(|m| m.url_base()).collect();
    assert_eq!(
      bases,
      vec![
        "https://dl.fedoraproject.org/pub/fedora/linux",
        "https://archives.fedoraproject.org/pub/archive/fedora/linux"
      ],
      "numbered fedora repo must fall back primary → archive"
    );
  }
}

// covers: DIST-074
#[test]
fn fedora_arbitrary_release_builds_without_bail() {
  // Fedora does not validate the release string at build time; a nonexistent
  // release (fedora:99) builds a literal source whose fetch would 404 later
  // (distro/fedora.rs:68-104 has no release guard).
  let src = source("fedora:99", Arch::X86_64).unwrap();
  assert_eq!(src.cache_id, "fedora/99/x86_64");
  assert_eq!(rpm_repo_path(&src, "fedora:99/releases"), "releases/99/Everything/x86_64/os");
}

// ---------------------------------------------------------------------------
// openSUSE native vs ports path selection, arch axis (DIST-043, DIST-074)
// ---------------------------------------------------------------------------

// covers: DIST-043
#[test]
fn opensuse_native_vs_ports_path_selection() {
  // tumbleweed on x86_64 uses the native repo path; on aarch64 it uses the
  // ports/<arch>/... path. Leap releases mirror the same split (distro/opensuse.rs:59-79).
  // aarch64 (ports) is the untested branch.
  let tw_x86 = source("opensuse:tumbleweed", Arch::X86_64).unwrap();
  assert_eq!(rpm_repo_path(&tw_x86, "opensuse:tumbleweed/oss"), "tumbleweed/repo/oss", "native x86_64 path");

  let tw_arm = source("opensuse:tumbleweed", Arch::Aarch64).unwrap();
  assert_eq!(rpm_repo_path(&tw_arm, "opensuse:tumbleweed/oss"), "ports/aarch64/tumbleweed/repo/oss", "ports path");

  let leap_x86 = source("opensuse:15.6", Arch::X86_64).unwrap();
  assert_eq!(rpm_repo_path(&leap_x86, "opensuse:15.6/oss"), "distribution/leap/15.6/repo/oss", "native leap path");

  let leap_arm = source("opensuse:15.6", Arch::Aarch64).unwrap();
  assert_eq!(
    rpm_repo_path(&leap_arm, "opensuse:15.6/oss"),
    "ports/aarch64/distribution/leap/15.6/repo/oss",
    "ports leap path"
  );

  // Single mirror for every openSUSE source.
  assert_eq!(mirror_bases(&tw_arm), vec!["https://download.opensuse.org".to_string()]);
}

// covers: DIST-074
#[test]
fn opensuse_arbitrary_release_builds_without_bail() {
  // openSUSE accepts any release string at build time; the failure is deferred
  // to a fetch 404 (distro/opensuse.rs:59-79 has no release guard). A bogus
  // leap number builds the literal leap path.
  let src = source("opensuse:99.9", Arch::X86_64).unwrap();
  assert_eq!(rpm_repo_path(&src, "opensuse:99.9/oss"), "distribution/leap/99.9/repo/oss");
}

// ---------------------------------------------------------------------------
// Alpine x86 ↔ i686 ↔ native 'x86', arbitrary release (DIST-049, DIST-075)
// ---------------------------------------------------------------------------

// covers: DIST-049
#[test]
fn alpine_x86_and_i686_both_resolve_to_native_x86() {
  // Both kernel tokens x86 and i686 parse to Arch::I686 (arch.rs:130-139),
  // which projects to Alpine's native 'x86' (arch.rs:97-105). The source built
  // for Arch::I686 names its packages dir and apkindex under 'x86'
  // (distro/alpine.rs:21-25, 67-93).
  assert_eq!(Arch::from_uname("x86").unwrap(), Arch::I686);
  assert_eq!(Arch::from_uname("i686").unwrap(), Arch::I686);

  let src = source("alpine:v3.21", Arch::I686).unwrap();
  assert_eq!(src.native_arch, "x86", "Alpine native arch for I686 is 'x86'");
  assert_eq!(src.cache_id, "alpine/v3.21/x86");

  let main = src.repos.iter().find(|r| r.label.contains("/main")).unwrap();
  match &main.layout {
    LayoutRepository::Apk {
      path_file_apkindex,
      path_dir_packages,
    } => {
      assert_eq!(path_file_apkindex.as_str(), "main/x86/APKINDEX.tar.gz");
      assert_eq!(path_dir_packages.as_str(), "main/x86");
    }
    other => panic!("alpine repo must be Apk layout: {:?}", other),
  }
}

// covers: DIST-075
#[test]
fn debian_ubuntu_alpine_accept_arbitrary_release_no_build_bail() {
  // A nonexistent codename/version builds a literal suite/version source with
  // no build-time bail; the index 404 surfaces later (distro/debian.rs:70-110,
  // distro/ubuntu.rs:70-135, distro/alpine.rs:67-108).
  let deb = source("debian:nonexistent", Arch::X86_64).unwrap();
  match &deb.repos[0].layout {
    LayoutRepository::Deb { path_dir_suite, .. } => assert_eq!(path_dir_suite.as_str(), "dists/nonexistent"),
    other => panic!("debian repo must be Deb: {:?}", other),
  }

  let ubu = source("ubuntu:nonexistent", Arch::X86_64).unwrap();
  assert!(ubu.repos.iter().any(|r| r.label == "ubuntu:nonexistent/main"));

  // Alpine prefixes a bare release with 'v'; a literal already-v string is kept.
  let alp = source("alpine:v9.99", Arch::X86_64).unwrap();
  assert_eq!(alp.release, "v9.99");
  let alp_bare = source("alpine:9.99", Arch::X86_64).unwrap();
  assert_eq!(alp_bare.release, "v9.99", "bare numeric alpine release gains a 'v' prefix");
}

// ---------------------------------------------------------------------------
// --arch default is the host arch (DIST-051)
// ---------------------------------------------------------------------------

// covers: DIST-051
#[test]
fn arch_default_equals_from_host_as_uname() {
  // cli.rs:40-41 defaults --arch to Arch::from_host().as_uname(). Building a
  // source with that exact default produces a native arch consistent with the
  // host arch, proving the default round-trips through from_uname.
  let host = Arch::from_host();
  let default_token = host.as_uname();
  assert_eq!(Arch::from_uname(default_token).unwrap(), host, "host default token must parse back to the host arch");

  let src = source("debian:bookworm", host).unwrap();
  assert_eq!(src.native_arch, host.as_debian(), "default-arch source uses the host's debian arch naming");
}

// ---------------------------------------------------------------------------
// download_url unprefixed-filename refusal (DIST-072)
// ---------------------------------------------------------------------------

// covers: DIST-072
#[test]
fn download_url_errors_on_unprefixed_filename() {
  // index_fetch encodes every filename as `<url_base>|<relative_path>`. A
  // filename lacking the `|` prefix is an index-population invariant violation
  // and download_url refuses it as an error. Build a real RemoteDistro offline
  // and call download_url with a bare filename.
  let remote = RemoteDistro::from_str("debian:bookworm", Arch::X86_64, http()).unwrap();
  let err = remote
    .download_url("bash_5.2.deb")
    .err()
    .expect("download_url must refuse an unprefixed filename");
  let msg = err.to_string();
  assert!(
    msg.contains("download_url received unprefixed filename 'bash_5.2.deb'"),
    "error message must name the unprefixed filename, got: {msg}"
  );
}

// ===========================================================================
// DIST-078 — download_url base-trim/rel-trim joins with EXACTLY one slash
// (remote/mod.rs:480 — `format!("{}/{}", base.trim_end_matches('/'),
// rest.trim_start_matches('/'))`)
//
// Called directly with a well-formed `<base>|<rel>` filename: a trailing slash
// on the base and a leading slash on the rel must collapse to a single
// separator, never zero and never two.
// ===========================================================================

// covers: DIST-078
#[test]
fn download_url_joins_with_exactly_one_slash() {
  let remote = RemoteDistro::from_str("debian:bookworm", Arch::X86_64, http()).unwrap();

  // Every combination of trailing-base-slash × leading-rel-slash must yield the
  // identical single-slash join.
  let expected = "https://mirror.example/debian/pool/main/b/bash/bash_5.2.deb";
  let cases: &[&str] = &[
    "https://mirror.example/debian|pool/main/b/bash/bash_5.2.deb",
    "https://mirror.example/debian/|pool/main/b/bash/bash_5.2.deb",
    "https://mirror.example/debian|/pool/main/b/bash/bash_5.2.deb",
    "https://mirror.example/debian/|/pool/main/b/bash/bash_5.2.deb",
  ];
  for filename in cases {
    let url = remote.download_url(filename).unwrap();
    assert_eq!(url, expected, "single-slash join failed for {filename:?}");
    // No double slash in the path portion (after the scheme's `https://`).
    let after_scheme = url.strip_prefix("https://").unwrap();
    assert!(!after_scheme.contains("//"), "path must carry exactly one separator: {url}");
  }

  // Multiple trailing/leading slashes still collapse to one (trim_end/trim_start
  // strip them all).
  let many = remote
    .download_url("https://mirror.example/debian///|///pool/x.deb")
    .unwrap();
  assert_eq!(many, "https://mirror.example/debian/pool/x.deb", "repeated slashes must trim to one");
}
