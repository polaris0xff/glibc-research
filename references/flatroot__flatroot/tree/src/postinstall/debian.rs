//! Debian-family post-install: stubs for the tools its maintainer
//! scripts call, so the parts that shape the rootfs take effect and the
//! privileged parts succeed as no-ops.

use std::fs;
use std::os::unix::fs::PermissionsExt;

use anyhow::Result;

use super::rootfs::Rootfs;
use super::script::{ScriptEntry, ScriptFlavour, ScriptReplay};
use super::stubs::StubSet;
use crate::internal::fs::Fs;
use crate::sandbox::Sandbox;

/// The Debian-family post-install behavior, shared unchanged by the RPM
/// families.
pub struct DebPostInstall;

impl DebPostInstall {
  /// Installs the stubs the maintainer scripts look for, so a script
  /// that hits a privileged or interactive command still runs to
  /// completion.
  pub fn stubs_install(&self, rootfs: &Rootfs) -> Result<()> {
    let stubs = StubSet::new(*rootfs);

    stubs.noop(NOOP_STUBS)?;

    // dpkg stub: handle --compare-versions, no-op everything else
    stubs.script("dpkg", DPKG_STUB)?;

    // update-alternatives stub: create actual symlinks
    stubs.script("update-alternatives", ALTERNATIVES_STUB)?;

    // Stub debconf confmodule — sourced by scripts via `. /usr/share/debconf/confmodule`.
    stubs.seed("usr/share/debconf/confmodule", CONFMODULE_STUB, false)?;
    stubs.seed("usr/share/debconf/frontend", "#!/bin/sh\nexit 0\n", true)?;

    // Create /etc/shells if missing — bash and dash postinst call add-shell which needs it
    stubs.seed_keep("etc/shells", "/bin/sh\n/bin/bash\n/bin/dash\n/usr/bin/bash\n/usr/bin/sh\n")?;

    // add-shell / remove-shell stubs — append to /etc/shells
    stubs.script("add-shell", ADD_SHELL_STUB)?;
    stubs.script("remove-shell", "#!/bin/sh\nexit 0\n")?;

    Ok(())
  }

  pub fn scripts_run(&self, rootfs: &Rootfs, sandbox: &Sandbox, arch: &str) -> Result<()> {
    ScriptReplay::new(*rootfs, sandbox, DebianFlavour, arch).run()
  }
}

/// Commands whose outcome does not shape the rootfs; each gets a stub
/// that just succeeds, so a script does not abort at them.
const NOOP_STUBS: &[&str] = &[
  "chown",
  "chgrp",
  "chmod",
  "dpkg-trigger",
  "dpkg-divert",
  "invoke-rc.d",
  "systemctl",
  "update-rc.d",
  "start-stop-daemon",
  "adduser",
  "addgroup",
  "deluser",
  "delgroup",
  "pam-auth-update",
  "mount",
  "umount",
  "pycompile",
  "py3compile",
  "pyclean",
  "py3clean",
  "dpkg-query",
  "dpkg-statoverride",
  "dpkg-maintscript-helper",
];

/// A `dpkg` stub that really answers `--compare-versions` and reports
/// any queried package as installed — the two answers scripts branch
/// on — while every other invocation exits 0.
const DPKG_STUB: &str = r#"#!/bin/sh
case "$1" in
    --compare-versions)
        # $2 = version_a, $3 = op, $4 = version_b
        # Use sort -V for basic version comparison
        va="$2"; op="$3"; vb="$4"
        case "$op" in
            lt|lt-nl)
                [ "$(printf '%s\n%s' "$va" "$vb" | sort -V | head -1)" = "$va" ] && [ "$va" != "$vb" ]
                exit $?;;
            le|le-nl)
                [ "$(printf '%s\n%s' "$va" "$vb" | sort -V | head -1)" = "$va" ]
                exit $?;;
            eq)
                [ "$va" = "$vb" ]
                exit $?;;
            ge|ge-nl)
                [ "$(printf '%s\n%s' "$va" "$vb" | sort -V | tail -1)" = "$va" ]
                exit $?;;
            gt|gt-nl)
                [ "$(printf '%s\n%s' "$va" "$vb" | sort -V | tail -1)" = "$va" ] && [ "$va" != "$vb" ]
                exit $?;;
            *)
                exit 0;;
        esac;;
    -s|--status)
        # Pretend the package is installed
        echo "Status: install ok installed"
        exit 0;;
    *)
        exit 0;;
esac
"#;

/// An `update-alternatives` stub that creates the requested symlinks —
/// skipping it would leave generic command names pointing nowhere — and
/// ignores the bookkeeping options.
const ALTERNATIVES_STUB: &str = r#"#!/bin/sh
case "$1" in
    --install)
        # --install <link> <name> <path> <priority> [--slave <link> <name> <path>]...
        link="$2"
        target="$4"  # capture the primary path before the shifts consume it
        shift 4  # skip --install <link> <name> <path>
        shift    # skip <priority>
        # Create the primary symlink
        ln -sf "$target" "$link" 2>/dev/null || true
        # Process --slave entries
        while [ $# -gt 0 ]; do
            case "$1" in
                --slave)
                    slave_link="$2"
                    shift 2  # skip --slave <link>
                    shift    # skip <name>
                    slave_path="$1"
                    shift    # skip <path>
                    ln -sf "$slave_path" "$slave_link" 2>/dev/null || true
                    ;;
                *)
                    shift
                    ;;
            esac
        done
        ;;
esac
exit 0
"#;

/// A debconf confmodule stub defining the same `db_*` calls but
/// answering each immediately and emptily, so an unattended install
/// never blocks on a prompt.
const CONFMODULE_STUB: &str = r#"# flatroot stub: no-op debconf confmodule
db_input() { return 0; }
db_go() { return 0; }
db_get() { RET=""; return 0; }
db_set() { return 0; }
db_subst() { return 0; }
db_register() { return 0; }
db_unregister() { return 0; }
db_purge() { return 0; }
db_metaget() { RET=""; return 0; }
db_fget() { RET=""; return 0; }
db_fset() { return 0; }
db_title() { return 0; }
db_beginblock() { return 0; }
db_endblock() { return 0; }
db_capb() { return 0; }
db_stop() { return 0; }
db_settitle() { return 0; }
db_previous_module() { return 0; }
db_info() { return 0; }
db_progress() { return 0; }
db_version() { return 0; }
db_reset() { return 0; }
"#;

/// An `add-shell` stub that really appends to `/etc/shells` — an
/// unlisted shell is not accepted as a login shell — skipping shells
/// already listed, so repeated runs stay correct.
const ADD_SHELL_STUB: &str = r#"#!/bin/sh
for shell in "$@"; do
    if ! grep -qx "$shell" /etc/shells 2>/dev/null; then
        echo "$shell" >> /etc/shells
    fi
done
"#;

/// Debian's `ScriptFlavour`: runs each `postinst` with `sh` in the
/// `configure` phase under dpkg's environment variables. `libc6` is
/// excluded — its postinst assumes a live system — as are `.lua`
/// scriptlets, which this runner cannot interpret.
pub struct DebianFlavour;

impl ScriptFlavour for DebianFlavour {
  fn interpreter(&self) -> &'static str {
    "sh"
  }

  fn eligible(&self, entry: &ScriptEntry) -> Result<bool> {
    if entry.identity.name == "libc6" {
      return Ok(false);
    }
    if entry.script("postinst.lua").exists() {
      return Ok(false);
    }
    Ok(entry.script("postinst").exists())
  }

  fn stage(&self, rootfs: &Rootfs, entry: &ScriptEntry, _interpreter: &str) -> Result<Option<Vec<String>>> {
    let postinst = entry.script("postinst");

    // Copy the script into the rootfs so the sandbox can access it
    let script_dest = rootfs.path().join(".flatroot/current-postinst");
    fs::copy(&postinst, &script_dest)?;
    fs::set_permissions(&script_dest, fs::Permissions::from_mode(0o755))?;

    Ok(Some(vec![
      "/bin/sh".to_string(),
      "-e".to_string(),
      "/.flatroot/current-postinst".to_string(),
      "configure".to_string(),
      "".to_string(),
    ]))
  }

  fn env(&self, entry: &ScriptEntry) -> Vec<(String, String)> {
    vec![
      ("PATH".to_string(), "/.flatroot/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string()),
      ("DPKG_MAINTSCRIPT_NAME".to_string(), "postinst".to_string()),
      ("DPKG_MAINTSCRIPT_PACKAGE".to_string(), entry.identity.name.clone()),
      ("DPKG_MAINTSCRIPT_ARCH".to_string(), entry.identity.arch.clone()),
      ("DPKG_ROOT".to_string(), "".to_string()),
      ("DEBIAN_FRONTEND".to_string(), "noninteractive".to_string()),
      ("HOME".to_string(), "/root".to_string()),
      ("TERM".to_string(), "dumb".to_string()),
    ]
  }

  fn label(&self) -> &'static str {
    "postinst"
  }

  fn cleanup(&self, rootfs: &Rootfs) {
    Fs::remove_lenient(&rootfs.path().join(".flatroot/current-postinst"));
  }

  fn finish_message(&self) -> &'static str {
    "Postinst scripts completed"
  }
}
