//! Regenerates the content-derived caches — trust stores, icon and
//! font caches, schema and module registries — that a live system keeps
//! current automatically. Each rebuild runs only when its inputs are
//! present, so a minimal rootfs skips it.

use std::fs;

use anyhow::Result;
use indicatif::ProgressBar;

use super::rootfs::Rootfs;
use super::step_report::StepReport;
use crate::sandbox::Sandbox;
use crate::ui::RunVoice;

/// The image-loader cache builder's names in preference order, plain first,
/// then the `-64`/`-32` multilib variants.
const GDK_PIXBUF_QUERY_LOADERS: &[&str] = &[
  "gdk-pixbuf-query-loaders",
  "gdk-pixbuf-query-loaders-64",
  "gdk-pixbuf-query-loaders-32",
];
/// The I/O-module registry builder's names in preference order, plain
/// first, then the multilib variants.
const GIO_QUERYMODULES: &[&str] = &["gio-querymodules", "gio-querymodules-64", "gio-querymodules-32"];

/// The input-method registry builder's names in preference order, plain
/// first, then the multilib variants.
const GTK_QUERY_IMMODULES_3_0: &[&str] = &[
  "gtk-query-immodules-3.0",
  "gtk-query-immodules-3.0-64",
  "gtk-query-immodules-3.0-32",
];

/// How a cache rebuilder must be invoked — run as-is, pointed at a
/// directory, gated on a file's presence, or with an input seeded first.
enum Argv {
  /// Append a fixed tail to the resolved binary (e.g.
  /// `--update-cache`, `/usr/share/mime`).
  Fixed(&'static [&'static str]),
  /// `gio-querymodules` takes the located `gio/modules`
  /// directory, falling back to the flat default.
  GioModules,
  /// `gtk-update-icon-cache` runs only when the hicolor theme
  /// index is present, then over `/usr/share/icons/hicolor`.
  IconCache,
  /// `locale-gen` seeds `/etc/locale.gen` when absent, then runs
  /// with no further arguments.
  LocaleGen,
}

impl Argv {
  /// Builds the concrete command to run in the rootfs, seeding any
  /// input file the rebuilder expects. `None` when the inputs it would
  /// summarize are absent — that cache is skipped while the rest still
  /// rebuild.
  fn build(&self, rootfs: &Rootfs, bin: &str) -> Result<Option<Vec<String>>> {
    let argv = match self {
      Argv::Fixed(tail) => {
        let mut argv = vec![bin.to_string()];
        argv.extend(tail.iter().map(|s| s.to_string()));
        argv
      }
      Argv::GioModules => {
        let modules_dir = rootfs
          .lib_dir_find("gio/modules")
          .unwrap_or_else(|| "/usr/lib/gio/modules".to_string());
        vec![bin.to_string(), modules_dir]
      }
      Argv::IconCache => {
        if !rootfs.path().join("usr/share/icons/hicolor/index.theme").exists() {
          return Ok(None);
        }
        vec![
          bin.to_string(),
          "-q".to_string(),
          "/usr/share/icons/hicolor".to_string(),
        ]
      }
      Argv::LocaleGen => {
        let locale_gen = rootfs.path().join("etc/locale.gen");
        if !locale_gen.exists() {
          fs::create_dir_all(rootfs.path().join("etc"))?;
          fs::write(&locale_gen, "en_US.UTF-8 UTF-8\n")?;
        }
        vec![bin.to_string()]
      }
    };
    Ok(Some(argv))
  }
}

/// One cache rebuild described as data: the builder binary's candidate
/// names, where it lives, a label, and how it is invoked.
struct CacheHook {
  /// Candidate names, including ELF-class `-64`/`-32` variants.
  basenames: &'static [&'static str],
  /// Plugin-scanner subdir (`gdk-pixbuf-2.0`, `glib-2.0`,
  /// `libgtk-3-0`) or absent when the binary lives on the PATH.
  lib_subdir: Option<&'static str>,
  /// Human label for progress/report.
  label: &'static str,
  /// How to turn the resolved binary into a sandbox argv.
  argv: Argv,
}

impl CacheHook {
  /// Rebuilds one cache, or does nothing when its builder or inputs are
  /// absent (the normal case, not an error). Runs isolated in the rootfs
  /// and reports the outcome; one cache's trouble never derails the others.
  fn resolve_and_run(&self, rootfs: &Rootfs, sandbox: &Sandbox, pb: &ProgressBar) -> Result<()> {
    let bin = match rootfs.binary_find(self.basenames, self.lib_subdir) {
      Some(b) => b,
      None => return Ok(()),
    };
    let argv = match self.argv.build(rootfs, &bin)? {
      Some(a) => a,
      None => return Ok(()),
    };

    pb.set_message(format!("hook: {}", self.label));

    let env: Vec<(&str, &str)> = vec![
      ("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"),
      ("HOME", "/root"),
      ("TERM", "dumb"),
    ];

    let cmd: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
    let output = sandbox.run(&cmd, &env);
    StepReport::print(pb, "hook", self.label, &output);
    Ok(())
  }
}

/// The best-effort cache-rebuild stage: considers every cache it knows,
/// rebuilds the ones whose source is present, and treats the rest as not
/// applicable rather than failures.
pub struct CacheHooks;

impl CacheHooks {
  /// Every content-derived cache in a deliberate order — some depend on
  /// others being refreshed first (trust stores before network validators,
  /// registries before their consumers).
  const TABLE: &'static [CacheHook] = &[
    // CA trust store (Arch, CachyOS, Fedora, CentOS, Alma, Rocky).
    // Populates /etc/ssl/certs/ca-bundle.crt and the hashed cert symlinks
    // applications consult for TLS validation.
    CacheHook {
      basenames: &["update-ca-trust"],
      lib_subdir: None,
      label: "ca-trust",
      argv: Argv::Fixed(&[]),
    },
    // CA certificates (Debian, Ubuntu, Alpine, openSUSE).
    CacheHook {
      basenames: &["update-ca-certificates"],
      lib_subdir: None,
      label: "ca-certificates",
      argv: Argv::Fixed(&[]),
    },
    // locale-gen: create /etc/locale.gen if missing
    CacheHook {
      basenames: &["locale-gen"],
      lib_subdir: None,
      label: "locale-gen",
      argv: Argv::LocaleGen,
    },
    // gdk-pixbuf loader cache
    CacheHook {
      basenames: GDK_PIXBUF_QUERY_LOADERS,
      lib_subdir: Some("gdk-pixbuf-2.0"),
      label: "gdk-pixbuf",
      argv: Argv::Fixed(&["--update-cache"]),
    },
    // MIME type database
    CacheHook {
      basenames: &["update-mime-database"],
      lib_subdir: None,
      label: "mime-database",
      argv: Argv::Fixed(&["/usr/share/mime"]),
    },
    // GSettings schemas
    CacheHook {
      basenames: &["glib-compile-schemas"],
      lib_subdir: Some("glib-2.0"),
      label: "glib-schemas",
      argv: Argv::Fixed(&["/usr/share/glib-2.0/schemas"]),
    },
    // GIO modules
    CacheHook {
      basenames: GIO_QUERYMODULES,
      lib_subdir: Some("glib-2.0"),
      label: "gio-modules",
      argv: Argv::GioModules,
    },
    // Icon cache
    CacheHook {
      basenames: &["gtk-update-icon-cache"],
      lib_subdir: None,
      label: "icon-cache",
      argv: Argv::IconCache,
    },
    // Desktop file database
    CacheHook {
      basenames: &["update-desktop-database"],
      lib_subdir: None,
      label: "desktop-database",
      argv: Argv::Fixed(&["--quiet"]),
    },
    // Font cache
    CacheHook {
      basenames: &["fc-cache"],
      lib_subdir: None,
      label: "font-cache",
      argv: Argv::Fixed(&["-s"]),
    },
    // GTK3 input method modules
    CacheHook {
      basenames: GTK_QUERY_IMMODULES_3_0,
      lib_subdir: Some("libgtk-3-0"),
      label: "gtk3-immodules",
      argv: Argv::Fixed(&["--update-cache"]),
    },
    // dconf database
    CacheHook {
      basenames: &["dconf"],
      lib_subdir: None,
      label: "dconf",
      argv: Argv::Fixed(&["update"]),
    },
  ];

  /// Rebuilds every applicable cache, best-effort: progress is
  /// reported, and one cache's failure never stops the others.
  pub fn run(rootfs: &Rootfs, sandbox: &Sandbox) -> Result<()> {
    let pb = RunVoice::spinner("Running cache hooks");

    for hook in Self::TABLE {
      hook.resolve_and_run(rootfs, sandbox, &pb)?;
    }

    RunVoice::finish_ok(&pb, "Cache hooks completed");
    Ok(())
  }
}
