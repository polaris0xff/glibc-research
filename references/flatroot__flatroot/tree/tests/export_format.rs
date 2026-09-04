//! Format-selection, validation, and metadata contract tests for
//! `flatroot export`. These exercise the parts of the export pipeline
//! that do not need a real distro install: extension-based format
//! inference, the explicit-`--format` override, `--tag` handling, the
//! `FLATROOT_ARG_EXPORT_*` env fallbacks, the source/output validation
//! gates and their ordering, and the on-disk OCI/tar metadata the
//! exporters emit (platform, diff_ids, media types, USTAR headers,
//! reproducibility). A minimal synthetic rootfs is enough for all of
//! them, so they run in milliseconds and need no network.
//!
//! Every error string asserted here is copied verbatim from the cited
//! source: `src/commands/export/{mod,source,tar,oci}.rs` and
//! `src/arch.rs`.

#![allow(dead_code)]

mod export;

use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};

use crate::export::{
  build_rootfs, fresh_output_path, oci_layer_blob, run_export, run_export_no_format, run_export_raw,
};

/// Synthesize a 32-byte little-endian ELF header whose `e_machine`
/// field (bytes 18-19) names `machine`, write it as `bin/sh`, and add a
/// `.flatroot/` marker. The arch detector reads only the first 20 bytes
/// of `bin/sh`, so this is enough to drive `Arch::detect` to a chosen
/// architecture without any real cross-arch binary.
fn build_rootfs_arch(dir: &Path, machine: u16) {
  fs::create_dir_all(dir.join("bin")).unwrap();
  let mut header = vec![0u8; 32];
  header[0..4].copy_from_slice(b"\x7fELF");
  header[4] = 1; // EI_CLASS (unread, set to a plausible value)
  header[5] = 1; // EI_DATA = little-endian
  header[18] = (machine & 0xff) as u8;
  header[19] = (machine >> 8) as u8;
  fs::write(dir.join("bin/sh"), &header).unwrap();
  fs::create_dir_all(dir.join(".flatroot")).unwrap();
  fs::write(dir.join(".flatroot/packages"), b"").unwrap();
}

// ELF e_machine codes, matching src/arch.rs.
const EM_386: u16 = 3;
const EM_ARM: u16 = 40;
const EM_X86_64: u16 = 62;
const EM_AARCH64: u16 = 183;
const EM_RISCV: u16 = 243;

fn list_targz_entries(path: &Path) -> Vec<String> {
  let f = fs::File::open(path).unwrap();
  let gz = GzDecoder::new(BufReader::new(f));
  let mut archive = tar::Archive::new(gz);
  archive
    .entries()
    .unwrap()
    .map(|e| e.unwrap().path().unwrap().to_string_lossy().into_owned())
    .collect()
}

fn stderr_of(out: &std::process::Output) -> String {
  String::from_utf8_lossy(&out.stderr).into_owned()
}

fn assert_ok(out: &std::process::Output, label: &str) {
  assert!(out.status.success(), "{label}: export failed (exit={:?})\nstderr:\n{}", out.status.code(), stderr_of(out));
}

/// The real DwarFS image's entries, via `dwarfsck -l`. The spawn itself
/// fails loudly if `dwarfsck` is absent — a missing tool is an error, not
/// a skip.
fn list_dwarfs_entries(path: &Path) -> Vec<String> {
  let out = Command::new("dwarfsck")
    .arg("-l")
    .arg(path)
    .output()
    .expect("dwarfsck -l failed to execute");
  assert!(out.status.success(), "dwarfsck -l exited {:?}: {}", out.status.code(), String::from_utf8_lossy(&out.stderr));
  String::from_utf8_lossy(&out.stdout)
    .lines()
    .map(|s| s.trim_start_matches('/').to_string())
    .filter(|s| !s.is_empty())
    .collect()
}

/// The real SquashFS image's entries, via `unsquashfs -l` (its
/// `squashfs-root/` prefix stripped so the shape matches the other
/// backends).
fn list_sqfs_entries(path: &Path) -> Vec<String> {
  let out = Command::new("unsquashfs")
    .arg("-l")
    .arg(path)
    .output()
    .expect("unsquashfs -l failed to execute");
  assert!(
    out.status.success(),
    "unsquashfs -l exited {:?}: {}",
    out.status.code(),
    String::from_utf8_lossy(&out.stderr)
  );
  String::from_utf8_lossy(&out.stdout)
    .lines()
    .filter_map(|line| {
      let line = line.trim();
      if line.is_empty() || line == "squashfs-root" {
        return None;
      }
      Some(
        line
          .strip_prefix("squashfs-root/")
          .unwrap_or(line.trim_start_matches('/'))
          .to_string(),
      )
    })
    .collect()
}

/// The produced image must carry the planted payload and must never carry
/// the builder's private `.flatroot/` metadata dir.
fn assert_payload_and_exclusion(label: &str, entries: &[String]) {
  assert!(
    entries.iter().any(|p| p.trim_start_matches("./") == "bin/sh"),
    "{label}: payload bin/sh missing from the image:\n{entries:#?}"
  );
  assert!(
    !entries.iter().any(|p| {
      let n = p.trim_start_matches("./");
      n == ".flatroot" || n.starts_with(".flatroot/")
    }),
    "{label}: .flatroot leaked into the image:\n{entries:#?}"
  );
}

/// After a successful export the destination must be the only file in its
/// directory — no staging sibling survives. Impl-agnostic: it asserts the
/// directory contents, not a `.partial` literal.
fn assert_only_destination_present(dir: &Path, output: &Path) {
  let mut entries: Vec<PathBuf> = fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
  entries.sort();
  assert_eq!(entries, vec![output.to_path_buf()], "export left stray files besides the destination:\n{entries:#?}");
}

// ---------------------------------------------------------------------------
// Extension inference (no --format) and explicit-format override.
// ---------------------------------------------------------------------------

// covers: EXP-001
#[test]
fn targz_inferred_from_extension() {
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());

  let (_keep, output) = fresh_output_path(".tar.gz");
  let out = run_export_no_format(rootfs.path(), &output, &[]);
  assert_ok(&out, "tar.gz inference");

  // Resolved to the Tar exporter: progress strings are the Tar ones.
  let err = stderr_of(&out);
  assert!(err.contains("Exporting tar.gz to"), "missing 'Exporting tar.gz' progress:\n{err}");
  assert!(err.contains("Done."), "missing 'Done.' progress:\n{err}");
  // And the artefact is a real gzip tarball carrying the payload.
  let entries = list_targz_entries(&output);
  assert!(entries.iter().any(|p| p == "bin/sh" || p == "./bin/sh"), "tar.gz missing payload:\n{entries:#?}");
}

// covers: EXP-002, EXP-050
#[test]
fn sqfs_inferred_produces_valid_excluded_image() {
  // No `--format`: the `.sqfs` extension selects the SquashFS exporter, and
  // the real mksquashfs produces a valid image carrying the payload with
  // `.flatroot/` excluded and no staging sibling left behind.
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  let (dir, output) = fresh_output_path(".sqfs");

  let out = run_export_no_format(rootfs.path(), &output, &[]);
  assert_ok(&out, "sqfs inference");
  assert!(stderr_of(&out).contains("Exporting SquashFS to"), "`.sqfs` did not select the SquashFS exporter");
  assert!(stderr_of(&out).contains("Done."), "missing 'Done.' progress");
  assert_payload_and_exclusion("sqfs", &list_sqfs_entries(&output));
  assert_only_destination_present(dir.path(), &output);
}

// covers: EXP-003
#[test]
fn squashfs_alias_produces_valid_image() {
  // `.squashfs` is the alias for `.sqfs`: it selects the SquashFS exporter
  // and produces a valid image with the payload present.
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  let (dir, output) = fresh_output_path(".squashfs");

  let out = run_export_no_format(rootfs.path(), &output, &[]);
  assert_ok(&out, "squashfs alias");
  assert!(stderr_of(&out).contains("Exporting SquashFS to"), "`.squashfs` did not select the SquashFS exporter");
  assert_payload_and_exclusion("squashfs", &list_sqfs_entries(&output));
  assert_only_destination_present(dir.path(), &output);
}

// covers: EXP-004, EXP-049
#[test]
fn dwarfs_inferred_produces_valid_excluded_image() {
  // No `--format`: the `.dwarfs` extension selects the DwarFS exporter, and
  // the real mkdwarfs produces a valid image carrying the payload with
  // `.flatroot/` excluded and no staging sibling left behind.
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  let (dir, output) = fresh_output_path(".dwarfs");

  let out = run_export_no_format(rootfs.path(), &output, &[]);
  assert_ok(&out, "dwarfs inference");
  assert!(stderr_of(&out).contains("Exporting DwarFS to"), "`.dwarfs` did not select the DwarFS exporter");
  assert!(stderr_of(&out).contains("Done."), "missing 'Done.' progress");
  assert_payload_and_exclusion("dwarfs", &list_dwarfs_entries(&output));
  assert_only_destination_present(dir.path(), &output);
}

// covers: EXP-008
#[test]
fn explicit_format_overrides_foreign_extension() {
  // `--format tar` must win over a `.bin` extension that infers nothing.
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  let (_keep, output) = fresh_output_path(".bin");
  let out = run_export(rootfs.path(), "tar", &output, &[]);
  assert_ok(&out, "tar over .bin");
  assert!(stderr_of(&out).contains("Exporting tar.gz to"), "explicit --format tar not honoured for .bin output");
  let entries = list_targz_entries(&output);
  assert!(entries.iter().any(|p| p == "bin/sh" || p == "./bin/sh"), "tar payload missing for .bin output");
}

// covers: EXP-010
#[test]
fn bare_tar_with_explicit_tar_succeeds() {
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  let (_keep, output) = fresh_output_path(".tar");
  let out = run_export(rootfs.path(), "tar", &output, &[]);
  assert_ok(&out, "bare .tar + --format tar");
  // The Tar exporter always gzips, even when the name lacks `.gz`.
  let entries = list_targz_entries(&output);
  assert!(entries.iter().any(|p| p == "bin/sh" || p == "./bin/sh"), "bare .tar gzip payload missing");
}

// covers: EXP-011
#[test]
fn bare_tar_with_explicit_oci_succeeds() {
  // The same bare `.tar` name that is ambiguous without --format is
  // accepted once --format oci disambiguates it.
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  let (_keep, output) = fresh_output_path(".tar");
  let out = run_export(rootfs.path(), "oci", &output, &["--tag", "app:1"]);
  assert_ok(&out, "bare .tar + --format oci");
  // An OCI outer tar carries oci-layout/index.json at its root.
  let f = fs::File::open(&output).unwrap();
  let names: Vec<String> = tar::Archive::new(BufReader::new(f))
    .entries()
    .unwrap()
    .map(|e| {
      e.unwrap()
        .path()
        .unwrap()
        .to_string_lossy()
        .trim_start_matches("./")
        .to_string()
    })
    .collect();
  assert!(names.iter().any(|n| n == "oci-layout"), "oci outer tar missing oci-layout:\n{names:#?}");
  assert!(names.iter().any(|n| n == "index.json"), "oci outer tar missing index.json:\n{names:#?}");
}

// ---------------------------------------------------------------------------
// --tag handling.
// ---------------------------------------------------------------------------

// covers: EXP-006
#[test]
fn oci_without_tag_is_refused() {
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  let (_keep, output) = fresh_output_path(".oci");
  let out = run_export(rootfs.path(), "oci", &output, &[]);
  assert!(!out.status.success(), "oci export without --tag unexpectedly succeeded");
  assert!(
    stderr_of(&out).contains("--tag is required for OCI format. Example: --tag myapp:v1.0"),
    "wrong/missing OCI tag-required error:\n{}",
    stderr_of(&out)
  );
  assert!(!output.exists(), "no output should be written when the tag check fails");
}

// covers: EXP-007
#[test]
fn tag_is_ignored_for_non_oci_formats() {
  // Passing --tag to tar/sqfs/dwarfs must be a silent no-op: the export
  // still succeeds and still produces a valid image carrying the payload.
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());

  let (_keep_a, out_a) = fresh_output_path(".tar.gz");
  let r_a = run_export(rootfs.path(), "tar", &out_a, &["--tag", "ignored:tag"]);
  assert_ok(&r_a, "tar with --tag");
  assert_payload_and_exclusion("tar", &list_targz_entries(&out_a));

  let (_keep_s, out_s) = fresh_output_path(".sqfs");
  let r_s = run_export(rootfs.path(), "sqfs", &out_s, &["--tag", "ignored:tag"]);
  assert_ok(&r_s, "sqfs with --tag");
  assert_payload_and_exclusion("sqfs", &list_sqfs_entries(&out_s));

  let (_keep_d, out_d) = fresh_output_path(".dwarfs");
  let r_d = run_export(rootfs.path(), "dwarfs", &out_d, &["--tag", "ignored:tag"]);
  assert_ok(&r_d, "dwarfs with --tag");
  assert_payload_and_exclusion("dwarfs", &list_dwarfs_entries(&out_d));
}

// covers: EXP-037
#[test]
fn oci_short_tag_alias_works() {
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  let (_keep, output) = fresh_output_path(".tar");
  let out = run_export(rootfs.path(), "oci", &output, &["-t", "app:1"]);
  assert_ok(&out, "oci -t alias");
  // The docker manifest.json RepoTags must echo the -t value.
  assert_eq!(oci_repo_tags(&output), vec!["app:1".to_string()], "-t alias did not set RepoTags");
}

// ---------------------------------------------------------------------------
// Validation errors and ordering.
// ---------------------------------------------------------------------------

// covers: EXP-009
#[test]
fn bare_tar_without_format_is_ambiguous() {
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  let (_keep, output) = fresh_output_path(".tar");
  let out = run_export_no_format(rootfs.path(), &output, &[]);
  assert!(!out.status.success(), "bare .tar without --format unexpectedly succeeded");
  assert!(
    stderr_of(&out)
      .contains("Ambiguous output extension `.tar` — use `--format tar` or `--format oci` to disambiguate"),
    "wrong/missing ambiguity error:\n{}",
    stderr_of(&out)
  );
  assert!(!output.exists(), "no output on ambiguous extension");
}

// covers: EXP-012
#[test]
fn unknown_extension_without_format_is_refused() {
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  // A name with no recognisable suffix and no --format: inference fails.
  let dir = tempfile::tempdir().unwrap();
  let output = dir.path().join("output");
  let out = run_export_no_format(rootfs.path(), &output, &[]);
  assert!(!out.status.success(), "unknown-extension export unexpectedly succeeded");
  assert!(
    stderr_of(&out).contains("Cannot infer format from output extension; specify --format explicitly"),
    "wrong/missing inference error:\n{}",
    stderr_of(&out)
  );
}

// covers: EXP-013
#[test]
fn output_with_no_file_name_is_refused() {
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  // `/` has no file_name component; resolution refuses it.
  let out = run_export_no_format(rootfs.path(), Path::new("/"), &[]);
  assert!(!out.status.success(), "export to `/` unexpectedly succeeded");
  assert!(
    stderr_of(&out).contains("Output path has no file name:"),
    "wrong/missing no-file-name error:\n{}",
    stderr_of(&out)
  );
}

// covers: EXP-014
#[test]
fn missing_source_directory_is_refused() {
  let dir = tempfile::tempdir().unwrap();
  let missing = dir.path().join("does-not-exist");
  let (_keep, output) = fresh_output_path(".tar.gz");
  let out = run_export(&missing, "tar", &output, &[]);
  assert!(!out.status.success(), "export of a missing source unexpectedly succeeded");
  assert!(
    stderr_of(&out).contains("does not exist or is not a directory"),
    "wrong/missing source error:\n{}",
    stderr_of(&out)
  );
  assert!(!output.exists(), "no output should be written when the source is missing");
}

// covers: EXP-015
#[test]
fn source_that_is_a_file_is_refused() {
  let dir = tempfile::tempdir().unwrap();
  let file = dir.path().join("somefile.txt");
  fs::write(&file, b"not a rootfs").unwrap();
  let (_keep, output) = fresh_output_path(".tar.gz");
  let out = run_export(&file, "tar", &output, &[]);
  assert!(!out.status.success(), "export of a regular-file source unexpectedly succeeded");
  assert!(
    stderr_of(&out).contains("does not exist or is not a directory"),
    "wrong/missing not-a-directory error:\n{}",
    stderr_of(&out)
  );
}

// covers: EXP-016, EXP-043
#[test]
fn source_without_detectable_arch_is_refused() {
  // EXP-016: a directory with no detectable arch binary.
  // EXP-043: a directory containing ONLY a top-level .flatroot/ (no
  // payload) must fail arch detection, not produce an empty tarball.
  let empty = tempfile::tempdir().unwrap();
  let (_keep_a, out_a) = fresh_output_path(".tar.gz");
  let r_a = run_export(empty.path(), "tar", &out_a, &[]);
  assert!(!r_a.status.success(), "export of an empty dir unexpectedly succeeded");
  assert!(
    stderr_of(&r_a).contains("Failed to detect rootfs architecture"),
    "EXP-016: wrong/missing arch-detection error:\n{}",
    stderr_of(&r_a)
  );

  let only_flatroot = tempfile::tempdir().unwrap();
  fs::create_dir_all(only_flatroot.path().join(".flatroot/files")).unwrap();
  fs::write(only_flatroot.path().join(".flatroot/packages"), b"").unwrap();
  let (_keep_b, out_b) = fresh_output_path(".tar.gz");
  let r_b = run_export(only_flatroot.path(), "tar", &out_b, &[]);
  assert!(!r_b.status.success(), "EXP-043: .flatroot-only export unexpectedly succeeded");
  assert!(
    stderr_of(&r_b).contains("Failed to detect rootfs architecture"),
    "EXP-043: wrong/missing arch-detection error:\n{}",
    stderr_of(&r_b)
  );
  assert!(!out_b.exists(), "EXP-043: no tarball should be written when arch detection fails");
}

// covers: EXP-041
#[test]
fn source_validation_runs_before_format_and_tag() {
  // Missing source + an OCI request with no --tag and an ambiguous .tar
  // name: the SOURCE gate (RootfsSource::open) is first in run(), so the
  // source error wins over the format/tag errors.
  let dir = tempfile::tempdir().unwrap();
  let missing = dir.path().join("missing");
  let output = dir.path().join("badname");
  let out = run_export(&missing, "oci", &output, &[]);
  assert!(!out.status.success(), "export unexpectedly succeeded");
  let err = stderr_of(&out);
  assert!(err.contains("does not exist or is not a directory"), "SOURCE error did not fire first:\n{err}");
  assert!(!err.contains("--tag is required"), "tag error fired before the source gate:\n{err}");
}

// ---------------------------------------------------------------------------
// Env-var fallbacks (FLATROOT_ARG_EXPORT_*).
// ---------------------------------------------------------------------------

// covers: EXP-038
#[test]
fn env_export_format_selects_format() {
  // FLATROOT_ARG_EXPORT_FORMAT=tar drives a `.tar` (ambiguous without
  // help) name to the Tar exporter with no --format flag at all.
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  let (_keep, output) = fresh_output_path(".tar");
  let out = run_export_raw(
    &[rootfs.path().to_str().unwrap(), output.to_str().unwrap()],
    &[("FLATROOT_ARG_EXPORT_FORMAT", std::ffi::OsStr::new("tar"))],
  );
  assert_ok(&out, "env format=tar");
  assert!(stderr_of(&out).contains("Exporting tar.gz to"), "env FLATROOT_ARG_EXPORT_FORMAT=tar not honoured");
  assert!(
    list_targz_entries(&output)
      .iter()
      .any(|p| p == "bin/sh" || p == "./bin/sh"),
    "tar payload missing"
  );
}

// covers: EXP-039
#[test]
fn env_export_tag_satisfies_oci_requirement() {
  // FLATROOT_ARG_EXPORT_TAG supplies the OCI tag with no --tag flag.
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  let (_keep, output) = fresh_output_path(".tar");
  let out = run_export_raw(
    &[
      rootfs.path().to_str().unwrap(),
      output.to_str().unwrap(),
      "--format",
      "oci",
    ],
    &[("FLATROOT_ARG_EXPORT_TAG", std::ffi::OsStr::new("envtag:9"))],
  );
  assert_ok(&out, "env tag for oci");
  assert!(!stderr_of(&out).contains("--tag is required"), "env tag did not satisfy the OCI requirement");
  assert_eq!(oci_repo_tags(&output), vec!["envtag:9".to_string()], "env tag not reflected in RepoTags");
}

// covers: EXP-040
#[test]
fn cli_format_beats_env_format() {
  // FLATROOT_ARG_EXPORT_FORMAT=oci but --format tar on the CLI: the flag
  // wins (clap env fallback only applies when the flag is absent).
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  let (_keep, output) = fresh_output_path(".tar.gz");
  let out = run_export_raw(
    &[
      rootfs.path().to_str().unwrap(),
      output.to_str().unwrap(),
      "--format",
      "tar",
    ],
    &[("FLATROOT_ARG_EXPORT_FORMAT", std::ffi::OsStr::new("oci"))],
  );
  assert_ok(&out, "cli format beats env");
  assert!(stderr_of(&out).contains("Exporting tar.gz to"), "CLI --format tar did not beat env oci");
  // A real gzip tarball, not an OCI outer tar.
  assert!(
    list_targz_entries(&output)
      .iter()
      .any(|p| p == "bin/sh" || p == "./bin/sh"),
    "tar payload missing"
  );
}

// ---------------------------------------------------------------------------
// stdout silence (progress only on stderr).
// ---------------------------------------------------------------------------

// covers: EXP-045
#[test]
fn export_writes_nothing_to_stdout() {
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());

  // tar + oci need no external tool.
  for (format, suffix, extra) in [("tar", ".tar.gz", Vec::new()), ("oci", ".tar", vec!["--tag", "app:1"])] {
    let (_keep, output) = fresh_output_path(suffix);
    let out = run_export(rootfs.path(), format, &output, &extra);
    assert_ok(&out, format);
    assert!(out.stdout.is_empty(), "{format}: export wrote to stdout: {:?}", String::from_utf8_lossy(&out.stdout));
    assert!(stderr_of(&out).contains("Done."), "{format}: progress not on stderr");
  }

  // dwarfs + sqfs via the real tools.
  for (format, suffix) in [("dwarfs", ".dwarfs"), ("sqfs", ".sqfs")] {
    let (_keep, output) = fresh_output_path(suffix);
    let out = run_export(rootfs.path(), format, &output, &[]);
    assert_ok(&out, format);
    assert!(out.stdout.is_empty(), "{format}: export wrote to stdout: {:?}", String::from_utf8_lossy(&out.stdout));
  }
}

// ---------------------------------------------------------------------------
// OCI metadata contracts (parse the produced archive directly).
// ---------------------------------------------------------------------------

/// Unpack an OCI outer tar to a tempdir and return its root path.
fn oci_unpack(outer_tar: &Path) -> tempfile::TempDir {
  let unpack = tempfile::tempdir().unwrap();
  let f = fs::File::open(outer_tar).unwrap();
  tar::Archive::new(BufReader::new(f))
    .unpack(unpack.path())
    .expect("unpack OCI outer tar");
  unpack
}

/// Read the docker `manifest.json` RepoTags from an OCI outer tar.
fn oci_repo_tags(outer_tar: &Path) -> Vec<String> {
  let unpack = oci_unpack(outer_tar);
  let bytes = fs::read(unpack.path().join("manifest.json")).expect("read manifest.json");
  let v: serde_json::Value = serde_json::from_slice(&bytes).expect("parse manifest.json");
  v[0]["RepoTags"]
    .as_array()
    .expect("RepoTags array")
    .iter()
    .map(|t| t.as_str().unwrap().to_string())
    .collect()
}

/// Read `index.json` and follow the manifest blob -> config blob,
/// returning (index_platform_json, manifest_json, config_json).
fn oci_docs(outer_tar: &Path) -> (serde_json::Value, serde_json::Value, serde_json::Value) {
  let unpack = oci_unpack(outer_tar);
  let blobs = unpack.path().join("blobs/sha256");
  let read_blob = |digest: &str| -> Vec<u8> {
    let bare = digest.strip_prefix("sha256:").unwrap();
    fs::read(blobs.join(bare)).unwrap_or_else(|e| panic!("read blob {bare}: {e}"))
  };

  let index: serde_json::Value = serde_json::from_slice(&fs::read(unpack.path().join("index.json")).unwrap()).unwrap();
  let manifest_digest = index["manifests"][0]["digest"].as_str().unwrap().to_string();
  let manifest: serde_json::Value = serde_json::from_slice(&read_blob(&manifest_digest)).unwrap();
  let config_digest = manifest["config"]["digest"].as_str().unwrap().to_string();
  let config: serde_json::Value = serde_json::from_slice(&read_blob(&config_digest)).unwrap();
  (index, manifest, config)
}

// covers: EXP-033, EXP-034
#[test]
fn oci_platform_and_detected_arch_across_arches() {
  // (e_machine, expected uname, expected goarch, expected oci variant)
  let cases: [(u16, &str, &str, Option<&str>); 5] = [
    (EM_X86_64, "x86_64", "amd64", None),
    (EM_386, "i686", "386", None),
    (EM_AARCH64, "aarch64", "arm64", None),
    (EM_ARM, "armv7l", "arm", Some("v7")),
    (EM_RISCV, "riscv64", "riscv64", None),
  ];

  for (machine, uname, goarch, variant) in cases {
    let rootfs = tempfile::tempdir().unwrap();
    build_rootfs_arch(rootfs.path(), machine);
    let (_keep, output) = fresh_output_path(".tar");
    let out = run_export(rootfs.path(), "oci", &output, &["--tag", "app:1"]);
    assert_ok(&out, uname);

    // EXP-034: stderr names the detected architecture as "<uname> (<goarch>)".
    assert!(
      stderr_of(&out).contains(&format!("detected architecture: {uname} ({goarch})")),
      "{uname}: missing detected-architecture stderr line:\n{}",
      stderr_of(&out)
    );

    // EXP-033: index platform + config architecture carry the goarch and,
    // for armv7l, variant=v7.
    let (index, _manifest, config) = oci_docs(&output);
    let platform = &index["manifests"][0]["platform"];
    assert_eq!(platform["architecture"].as_str(), Some(goarch), "{uname}: index platform.architecture");
    assert_eq!(platform["os"].as_str(), Some("linux"), "{uname}: index platform.os");
    assert_eq!(platform["variant"].as_str(), variant, "{uname}: index platform.variant");
    assert_eq!(config["architecture"].as_str(), Some(goarch), "{uname}: config architecture");
    assert_eq!(config["os"].as_str(), Some("linux"), "{uname}: config os");
  }
}

// covers: EXP-036
#[test]
fn oci_repo_tags_echo_exact_tag() {
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  let (_keep, output) = fresh_output_path(".tar");
  let out = run_export(rootfs.path(), "oci", &output, &["--tag", "custom/name:tag123"]);
  assert_ok(&out, "oci repotags");
  assert_eq!(oci_repo_tags(&output), vec!["custom/name:tag123".to_string()], "RepoTags must echo the exact --tag");
}

#[test]
fn oci_index_names_image_for_every_loader() {
  // Each loader takes the image's name from a different annotation dialect:
  // podman reads `org.opencontainers.image.ref.name` as the full name:tag,
  // while docker's containerd image store reads `io.containerd.image.name`
  // and resolves lookups only against the fully-qualified form (docker
  // expands a bare `app:1` to `docker.io/library/app:1` before consulting
  // the store, so the stored name must already be in that form or the image
  // is reachable only by ID). The index entry must carry both: ref.name
  // echoing the tag verbatim, and the containerd name normalized exactly
  // the way docker normalizes lookups — bare and namespaced names gain the
  // docker.io prefix, registry-qualified names (dot, port, or `localhost`
  // in the first component) pass through untouched.
  let cases = [
    ("app:1", "docker.io/library/app:1"),
    ("custom/name:tag123", "docker.io/custom/name:tag123"),
    ("ghcr.io/org/app:v1", "ghcr.io/org/app:v1"),
    ("localhost:5000/app:v1", "localhost:5000/app:v1"),
  ];
  for (tag, containerd_name) in cases {
    let rootfs = tempfile::tempdir().unwrap();
    build_rootfs(rootfs.path());
    let (_keep, output) = fresh_output_path(".tar");
    let out = run_export(rootfs.path(), "oci", &output, &["--tag", tag]);
    assert_ok(&out, tag);

    let (index, _manifest, _config) = oci_docs(&output);
    let annotations = &index["manifests"][0]["annotations"];
    assert_eq!(annotations["org.opencontainers.image.ref.name"].as_str(), Some(tag), "{tag}: ref.name annotation");
    assert_eq!(
      annotations["io.containerd.image.name"].as_str(),
      Some(containerd_name),
      "{tag}: containerd name annotation"
    );
  }
}

// covers: EXP-042, EXP-052
#[test]
fn oci_layer_media_type_and_diff_id() {
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  let (_keep, output) = fresh_output_path(".tar");
  let out = run_export(rootfs.path(), "oci", &output, &["--tag", "app:1"]);
  assert_ok(&out, "oci media type");

  let (_index, manifest, config) = oci_docs(&output);

  // EXP-042: layer mediaType is tar+gzip, and the (compressed) layer
  // digest differs from the (uncompressed) diff_id.
  let layer = &manifest["layers"][0];
  assert_eq!(
    layer["mediaType"].as_str(),
    Some("application/vnd.oci.image.layer.v1.tar+gzip"),
    "OCI layer mediaType wrong"
  );
  let layer_digest = layer["digest"].as_str().unwrap().to_string();
  let diff_id = config["rootfs"]["diff_ids"][0].as_str().unwrap().to_string();
  assert_ne!(layer_digest, diff_id, "compressed layer digest must differ from uncompressed diff_id");

  // EXP-052: the config diff_id is the sha256 of the DECOMPRESSED layer.
  let (_unpack, blob) = oci_layer_blob(&output);
  let mut gz = GzDecoder::new(BufReader::new(fs::File::open(&blob).unwrap()));
  let mut decompressed = Vec::new();
  gz.read_to_end(&mut decompressed).unwrap();
  let computed = format!("sha256:{}", hex::encode(Sha256::digest(&decompressed)));
  assert_eq!(diff_id, computed, "diff_id must equal sha256 of the decompressed layer");
}

// covers: EXP-030
#[test]
fn oci_diff_id_is_deterministic() {
  // The same tree exported twice yields the same diff_id and the same
  // config-blob digest (config carries only arch + diff_id, both stable).
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());

  let (_keep_a, out_a) = fresh_output_path(".tar");
  let r_a = run_export(rootfs.path(), "oci", &out_a, &["--tag", "app:1"]);
  assert_ok(&r_a, "oci a");
  let (_keep_b, out_b) = fresh_output_path(".tar");
  let r_b = run_export(rootfs.path(), "oci", &out_b, &["--tag", "app:1"]);
  assert_ok(&r_b, "oci b");

  let (_ia, ma, ca) = oci_docs(&out_a);
  let (_ib, mb, cb) = oci_docs(&out_b);
  assert_eq!(
    ca["rootfs"]["diff_ids"][0].as_str(),
    cb["rootfs"]["diff_ids"][0].as_str(),
    "diff_id is not deterministic across runs"
  );
  assert_eq!(
    ma["config"]["digest"].as_str(),
    mb["config"]["digest"].as_str(),
    "config-blob digest is not deterministic across runs"
  );
}

// ---------------------------------------------------------------------------
// USTAR / no GNU-sparse, and tar reproducibility.
// ---------------------------------------------------------------------------

/// Type-flag byte for every entry in a plain (decompressed) tar. The
/// flag sits at offset 156 of each 512-byte header block. GNU sparse is
/// 'S' (0x53 == 83). We assert none appears.
fn tar_type_flags(decompressed: &[u8]) -> Vec<u8> {
  let mut flags = Vec::new();
  let mut off = 0usize;
  while off + 512 <= decompressed.len() {
    let block = &decompressed[off..off + 512];
    // Two consecutive zero blocks mark end-of-archive.
    if block.iter().all(|&b| b == 0) {
      break;
    }
    let name_first = block[0];
    if name_first == 0 {
      off += 512;
      continue;
    }
    let typeflag = block[156];
    flags.push(typeflag);
    // Size is an octal string at offset 124, 12 bytes.
    let size_str = std::str::from_utf8(&block[124..136])
      .unwrap_or("")
      .trim_matches(['\0', ' ']);
    let size = u64::from_str_radix(size_str, 8).unwrap_or(0);
    let data_blocks = size.div_ceil(512) as usize;
    off += 512 + data_blocks * 512;
  }
  flags
}

// covers: EXP-032
#[test]
fn targz_has_no_gnu_sparse_entries() {
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  let (_keep, output) = fresh_output_path(".tar.gz");
  let out = run_export(rootfs.path(), "tar", &output, &[]);
  assert_ok(&out, "tar sparse");

  let mut gz = GzDecoder::new(BufReader::new(fs::File::open(&output).unwrap()));
  let mut decompressed = Vec::new();
  gz.read_to_end(&mut decompressed).unwrap();
  let flags = tar_type_flags(&decompressed);
  assert!(!flags.is_empty(), "no tar entries parsed");
  assert!(!flags.contains(&b'S'), "tar.gz contains a GNU-sparse (type-S) entry: {flags:?}");
}

// covers: EXP-031
#[test]
fn oci_layer_has_no_gnu_sparse_entries() {
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  let (_keep, output) = fresh_output_path(".tar");
  let out = run_export(rootfs.path(), "oci", &output, &["--tag", "app:1"]);
  assert_ok(&out, "oci sparse");

  // The layer blob (gzip) must have no type-S entries.
  let (_unpack, blob) = oci_layer_blob(&output);
  let mut gz = GzDecoder::new(BufReader::new(fs::File::open(&blob).unwrap()));
  let mut layer = Vec::new();
  gz.read_to_end(&mut layer).unwrap();
  let layer_flags = tar_type_flags(&layer);
  assert!(!layer_flags.is_empty(), "no OCI layer tar entries parsed");
  assert!(!layer_flags.contains(&b'S'), "OCI layer has a GNU-sparse (type-S) entry: {layer_flags:?}");

  // The OCI outer tar (uncompressed) must also be plain USTAR.
  let outer = fs::read(&output).unwrap();
  let outer_flags = tar_type_flags(&outer);
  assert!(!outer_flags.is_empty(), "no OCI outer tar entries parsed");
  assert!(!outer_flags.contains(&b'S'), "OCI outer tar has a GNU-sparse (type-S) entry: {outer_flags:?}");
}

// covers: EXP-029
#[test]
fn tar_and_oci_entries_are_name_sorted_and_reproducible() {
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());

  // tar: byte-identical artefact across two runs of the same tree.
  let (_ka, ta) = fresh_output_path(".tar.gz");
  assert_ok(&run_export(rootfs.path(), "tar", &ta, &[]), "tar a");
  let (_kb, tb) = fresh_output_path(".tar.gz");
  assert_ok(&run_export(rootfs.path(), "tar", &tb, &[]), "tar b");
  let entries_a = list_targz_entries(&ta);
  let entries_b = list_targz_entries(&tb);
  assert_eq!(entries_a, entries_b, "tar top-level ordering differs across runs");

  // The sequence of distinct first-path-components is name-sorted (the
  // append-order `Tar::append_dir_sorted` imposes). For our fixture that
  // is `bin` before `usr`.
  let top_components = |entries: &[String]| -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for e in entries {
      let n = e.trim_start_matches("./");
      let first = n.split('/').next().unwrap_or("").to_string();
      if !first.is_empty() && !seen.contains(&first) {
        seen.push(first);
      }
    }
    seen
  };
  let order = top_components(&entries_a);
  let mut sorted = order.clone();
  sorted.sort();
  assert_eq!(order, sorted, "tar top-level components are not name-sorted: {order:?}");
  assert!(
    order.contains(&"bin".to_string()) && order.contains(&"usr".to_string()),
    "unexpected top-level set: {order:?}"
  );

  // oci: the layer blob ordering is reproducible too.
  let (_kc, oa) = fresh_output_path(".tar");
  assert_ok(&run_export(rootfs.path(), "oci", &oa, &["--tag", "app:1"]), "oci a");
  let (_kd, ob) = fresh_output_path(".tar");
  assert_ok(&run_export(rootfs.path(), "oci", &ob, &["--tag", "app:1"]), "oci b");
  let (_ua, la) = oci_layer_blob(&oa);
  let (_ub, lb) = oci_layer_blob(&ob);
  assert_eq!(list_targz_entries(&la), list_targz_entries(&lb), "oci layer ordering differs across runs");
}

#[test]
fn oci_layer_entries_are_root_owned() {
  // The rootfs is built by whatever account runs the export, but the image
  // is consumed where that account means nothing: an engine applying the
  // layer in a user namespace with a single mapped id (rootless podman
  // without subuid ranges) rejects any owner it cannot map. Every layer
  // entry — file, directory, and symlink — must therefore record numeric
  // owner 0:0 with no owner names, regardless of who built the tree.
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  std::os::unix::fs::symlink("sh", rootfs.path().join("bin/ash")).unwrap();

  let (_keep, output) = fresh_output_path(".tar");
  let out = run_export(rootfs.path(), "oci", &output, &["--tag", "app:1"]);
  assert_ok(&out, "oci root-owned");

  let (_unpack, blob) = oci_layer_blob(&output);
  let gz = GzDecoder::new(BufReader::new(fs::File::open(&blob).unwrap()));
  let mut archive = tar::Archive::new(gz);
  let mut paths: Vec<String> = Vec::new();
  for entry in archive.entries().unwrap() {
    let entry = entry.unwrap();
    let header = entry.header();
    let path = entry.path().unwrap().display().to_string();
    assert_eq!(header.uid().unwrap(), 0, "{path}: layer entry not owned by uid 0");
    assert_eq!(header.gid().unwrap(), 0, "{path}: layer entry not owned by gid 0");
    assert_eq!(header.username().unwrap(), Some(""), "{path}: layer entry carries a user name");
    assert_eq!(header.groupname().unwrap(), Some(""), "{path}: layer entry carries a group name");
    paths.push(path);
  }
  assert!(paths.iter().any(|p| p.trim_start_matches("./") == "bin/ash"), "symlink missing from layer: {paths:?}");
  assert!(paths.iter().any(|p| p.trim_start_matches("./") == "usr/bin/hello"), "file missing from layer: {paths:?}");
}

// covers: EXP-026
#[test]
fn tar_and_oci_exclude_top_level_flatroot_only() {
  // Top-level .flatroot/ is dropped; a nested usr/share/.flatroot is kept.
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());
  fs::create_dir_all(rootfs.path().join("usr/share/.flatroot")).unwrap();
  fs::write(rootfs.path().join("usr/share/.flatroot/keepme"), b"nested\n").unwrap();

  let has_top = |entries: &[String]| {
    entries.iter().any(|p| {
      let n = p.trim_start_matches("./");
      n == ".flatroot" || n.starts_with(".flatroot/")
    })
  };
  let has_nested = |entries: &[String]| {
    entries.iter().any(|p| {
      let n = p.trim_start_matches("./");
      n == "usr/share/.flatroot" || n.starts_with("usr/share/.flatroot/")
    })
  };

  let (_kt, t) = fresh_output_path(".tar.gz");
  assert_ok(&run_export(rootfs.path(), "tar", &t, &[]), "tar");
  let tar_entries = list_targz_entries(&t);
  assert!(!has_top(&tar_entries), "tar: top-level .flatroot leaked:\n{tar_entries:#?}");
  assert!(has_nested(&tar_entries), "tar: nested usr/share/.flatroot was removed:\n{tar_entries:#?}");

  let (_ko, o) = fresh_output_path(".tar");
  assert_ok(&run_export(rootfs.path(), "oci", &o, &["--tag", "app:1"]), "oci");
  let (_unpack, layer) = oci_layer_blob(&o);
  let oci_entries = list_targz_entries(&layer);
  assert!(!has_top(&oci_entries), "oci: top-level .flatroot leaked:\n{oci_entries:#?}");
  assert!(has_nested(&oci_entries), "oci: nested usr/share/.flatroot was removed:\n{oci_entries:#?}");
}

// covers: EXP-044
#[test]
fn oci_leaves_no_partial_output_on_failure() {
  // Force the OCI build to fail late: make the output path unwritable so
  // `seal` (File::create) fails after all staging is done. The staging
  // tempdir must be cleaned up and no partial file left at the output.
  let rootfs = tempfile::tempdir().unwrap();
  build_rootfs(rootfs.path());

  // Output lives under a path whose parent does not exist -> File::create
  // fails inside seal, after the (tempdir) staging is fully built.
  let dir = tempfile::tempdir().unwrap();
  let output = dir.path().join("nope/out.tar");
  let out = run_export(rootfs.path(), "oci", &output, &["--tag", "app:1"]);
  assert!(!out.status.success(), "oci export to an unwritable path unexpectedly succeeded");
  assert!(
    stderr_of(&out).contains(&format!("Failed to create {}", output.display())),
    "missing 'Failed to create' context:\n{}",
    stderr_of(&out)
  );
  assert!(!output.exists(), "no partial OCI output should remain after a seal failure");
}
