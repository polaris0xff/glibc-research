//! `Rootfs`: lookups over an assembled rootfs — where a tool or plugin
//! directory lives — answered as the absolute path the rootfs itself
//! would use once mounted at `/`.

use std::path::Path;

/// A handle onto an assembled rootfs, answering lookups as
/// rootfs-absolute paths whatever layout the distro chose.
#[derive(Clone, Copy)]
pub struct Rootfs<'a> {
  root: &'a Path,
}

impl<'a> Rootfs<'a> {
  /// The program directories, in the order a lookup prefers — merged-`/usr`
  /// locations first, so the canonical copy wins when a name appears twice.
  const BIN_DIRS: &'static [&'static str] = &["usr/bin", "usr/sbin", "usr/local/bin", "usr/local/sbin", "bin", "sbin"];

  /// The library directories, from architecture-qualified multiarch down to
  /// a flat `lib`, so a per-package helper is found however this distro
  /// arranges its libraries.
  const LIB_DIRS: &'static [&'static str] = &[
    "usr/lib64",
    "usr/lib32",
    "usr/lib/x86_64-linux-gnu",
    "usr/lib/aarch64-linux-gnu",
    "usr/lib/i386-linux-gnu",
    "usr/lib/arm-linux-gnueabihf",
    "usr/lib/riscv64-linux-gnu",
    "usr/lib",
    "lib64",
    "lib/x86_64-linux-gnu",
    "lib/aarch64-linux-gnu",
    "lib/i386-linux-gnu",
    "lib/arm-linux-gnueabihf",
    "lib/riscv64-linux-gnu",
    "lib",
  ];

  /// Binds lookups to the rootfs at `root`.
  pub fn new(root: &'a Path) -> Self {
    Self { root }
  }

  /// Where the rootfs lives on the host, for a sandbox to mount as `/`.
  pub fn path(&self) -> &Path {
    self.root
  }

  /// Resolves a tool to its rootfs-absolute path, trying each candidate
  /// basename across the known bin layouts (and, with `lib_subdir`, the
  /// lib layouts) in preference order. `None` when the rootfs does not
  /// ship it.
  pub fn binary_find(&self, basenames: &[&str], lib_subdir: Option<&str>) -> Option<String> {
    for basename in basenames {
      for dir in Self::BIN_DIRS {
        let path = self.root.join(dir).join(basename);
        if path.symlink_metadata().is_ok() {
          return Some(format!("/{}/{}", dir, basename));
        }
      }
      if let Some(subdir) = lib_subdir {
        for dir in Self::LIB_DIRS {
          let path = self.root.join(dir).join(subdir).join(basename);
          if path.symlink_metadata().is_ok() {
            return Some(format!("/{}/{}/{}", dir, subdir, basename));
          }
        }
      }
    }
    None
  }

  /// Finds a plugin directory across the library layouts, to hand a cache
  /// rebuilder that must be pointed at the exact directory that changed.
  pub fn lib_dir_find(&self, sub_path: &str) -> Option<String> {
    for dir in Self::LIB_DIRS {
      let path = self.root.join(dir).join(sub_path);
      if path.symlink_metadata().is_ok() {
        return Some(format!("/{}/{}", dir, sub_path));
      }
    }
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use std::os::unix::fs::PermissionsExt;
  use tempfile::TempDir;

  fn touch(root: &Path, rel: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(&p, b"#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
  }

  #[test]
  fn binary_find_locates_in_usr_bin() {
    let tmp = TempDir::new().unwrap();
    touch(tmp.path(), "usr/bin/fc-cache");
    assert_eq!(Rootfs::new(tmp.path()).binary_find(&["fc-cache"], None), Some("/usr/bin/fc-cache".to_string()));
  }

  #[test]
  fn binary_find_locates_in_usr_sbin() {
    let tmp = TempDir::new().unwrap();
    touch(tmp.path(), "usr/sbin/update-ca-certificates");
    assert_eq!(
      Rootfs::new(tmp.path()).binary_find(&["update-ca-certificates"], None),
      Some("/usr/sbin/update-ca-certificates".to_string())
    );
  }

  #[test]
  fn binary_find_prefers_usr_bin_over_bin() {
    let tmp = TempDir::new().unwrap();
    touch(tmp.path(), "bin/sh");
    touch(tmp.path(), "usr/bin/sh");
    assert_eq!(Rootfs::new(tmp.path()).binary_find(&["sh"], None), Some("/usr/bin/sh".to_string()));
  }

  #[test]
  fn binary_find_walks_every_multiarch_triplet() {
    for triplet in &[
      "x86_64-linux-gnu",
      "aarch64-linux-gnu",
      "i386-linux-gnu",
      "arm-linux-gnueabihf",
      "riscv64-linux-gnu",
    ] {
      let tmp = TempDir::new().unwrap();
      let rel = format!("usr/lib/{}/gdk-pixbuf-2.0/gdk-pixbuf-query-loaders", triplet);
      touch(tmp.path(), &rel);
      assert_eq!(
        Rootfs::new(tmp.path()).binary_find(&["gdk-pixbuf-query-loaders"], Some("gdk-pixbuf-2.0")),
        Some(format!("/{}", rel)),
        "triplet {triplet} not resolved"
      );
    }
  }

  #[test]
  fn binary_find_locates_in_flat_usr_lib() {
    let tmp = TempDir::new().unwrap();
    touch(tmp.path(), "usr/lib/glib-2.0/gio-querymodules");
    assert_eq!(
      Rootfs::new(tmp.path()).binary_find(&["gio-querymodules"], Some("glib-2.0")),
      Some("/usr/lib/glib-2.0/gio-querymodules".to_string())
    );
  }

  #[test]
  fn binary_find_locates_in_usr_lib64() {
    let tmp = TempDir::new().unwrap();
    touch(tmp.path(), "usr/lib64/glib-2.0/gio-querymodules");
    assert_eq!(
      Rootfs::new(tmp.path()).binary_find(&["gio-querymodules"], Some("glib-2.0")),
      Some("/usr/lib64/glib-2.0/gio-querymodules".to_string())
    );
  }

  #[test]
  fn binary_find_matches_arch_class_suffixed_name() {
    let tmp = TempDir::new().unwrap();
    touch(tmp.path(), "usr/bin/gdk-pixbuf-query-loaders-64");
    let result = Rootfs::new(tmp.path()).binary_find(
      &[
        "gdk-pixbuf-query-loaders",
        "gdk-pixbuf-query-loaders-64",
        "gdk-pixbuf-query-loaders-32",
      ],
      Some("gdk-pixbuf-2.0"),
    );
    assert_eq!(result, Some("/usr/bin/gdk-pixbuf-query-loaders-64".to_string()));
  }

  #[test]
  fn binary_find_returns_first_listed_basename_when_both_exist() {
    let tmp = TempDir::new().unwrap();
    touch(tmp.path(), "usr/bin/gio-querymodules");
    touch(tmp.path(), "usr/bin/gio-querymodules-64");
    let result = Rootfs::new(tmp.path()).binary_find(&["gio-querymodules", "gio-querymodules-64"], Some("glib-2.0"));
    assert_eq!(result, Some("/usr/bin/gio-querymodules".to_string()));
  }

  #[test]
  fn binary_find_skips_lib_dirs_when_subdir_is_none() {
    let tmp = TempDir::new().unwrap();
    touch(tmp.path(), "usr/lib/glib-2.0/gio-querymodules");
    assert_eq!(Rootfs::new(tmp.path()).binary_find(&["gio-querymodules"], None), None);
  }

  #[test]
  fn binary_find_returns_none_when_nothing_matches() {
    let tmp = TempDir::new().unwrap();
    touch(tmp.path(), "usr/bin/something-else");
    assert_eq!(Rootfs::new(tmp.path()).binary_find(&["fc-cache"], None), None);
  }

  #[test]
  fn binary_find_resolves_dirent_with_broken_absolute_symlink_target() {
    use std::os::unix::fs::symlink;
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("usr/bin")).unwrap();
    symlink("/nonexistent/host/path", tmp.path().join("usr/bin/sh")).unwrap();
    assert_eq!(Rootfs::new(tmp.path()).binary_find(&["sh"], None), Some("/usr/bin/sh".to_string()));
  }

  #[test]
  fn lib_dir_find_locates_multiarch_directory() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("usr/lib/x86_64-linux-gnu/gio/modules")).unwrap();
    assert_eq!(
      Rootfs::new(tmp.path()).lib_dir_find("gio/modules"),
      Some("/usr/lib/x86_64-linux-gnu/gio/modules".to_string())
    );
  }

  #[test]
  fn lib_dir_find_locates_lib64_directory() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("usr/lib64/gio/modules")).unwrap();
    assert_eq!(Rootfs::new(tmp.path()).lib_dir_find("gio/modules"), Some("/usr/lib64/gio/modules".to_string()));
  }

  #[test]
  fn lib_dir_find_locates_flat_lib_directory() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("usr/lib/gio/modules")).unwrap();
    assert_eq!(Rootfs::new(tmp.path()).lib_dir_find("gio/modules"), Some("/usr/lib/gio/modules".to_string()));
  }

  #[test]
  fn lib_dir_find_returns_none_when_nothing_matches() {
    let tmp = TempDir::new().unwrap();
    assert_eq!(Rootfs::new(tmp.path()).lib_dir_find("gio/modules"), None);
  }
}
