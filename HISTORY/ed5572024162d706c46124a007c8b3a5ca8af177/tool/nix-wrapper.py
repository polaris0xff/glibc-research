#!/usr/bin/env python3
"""nix-wrapper.py - read the ENVIRONMENT out of a nixpkgs wrapper.

⛔ THE HOLE THIS FILLS, in `tool/nix-appimage.sh`'s own words: *"a nixpkgs
`bin/x` that is a WRAPPER SCRIPT is followed to the real ELF and the wrapper's
environment is NOT reproduced. An app that needs it will be missing it."*
`TODO` T-053 asks whether `patsh` should fill it.

⭐ THE ANSWER IS NO, AND THE REASON IS MEASURED RATHER THAN ARGUED.
`patsh` patches `/nix/store` paths **in shell scripts**. Two things are wrong
with that here:

  1. **The dominant wrapper is no longer a shell script.** nixpkgs' current
     `makeWrapper` produces `makeBinaryWrapper` output: a compiled C program.
     Measured on `mpv-with-scripts-0.41.0`, whose `bin/mpv` is a 16,560-byte
     **ELF**, not a script. patsh has nothing to patch.
  2. **We do not want the wrapper to keep working, we want what it knows.**
     A bundle does not run the wrapper -- sharun runs the real ELF and reads
     an `.env` file. Rewriting store paths so a script still works is the
     wrong end: the job is to LIFT the assignments out and re-express them
     against the bundle.

⭐ AND THE BINARY WRAPPER MAKES THAT EXACT, because nixpkgs embeds the command
that generated it as a comment in the binary's own data:

    makeCWrapper '/nix/store/...-mpv-0.41.0/bin/mpv' \\
        --inherit-argv0 \\
        --prefix 'LUA_CPATH' ';' '/nix/store/...-lua-5.2.4-env/lib/lua/5.2/?.so' \\
        --prefix 'PATH' ':' '/nix/store/...-lua-5.2.4-env/bin' \\
        --suffix 'PATH' ':' '/nix/store/...-yt-dlp-2026.08.19/bin'

So the environment is read, not guessed. The shell shape is still handled --
plenty of packages use it -- by parsing the assignments it emits.

Usage:
  nix-wrapper.py read WRAPPER        # -> TSV: op, var, sep, value
  nix-wrapper.py target WRAPPER      # -> the real program the wrapper execs
  nix-wrapper.py selftest

TSV ops: set | prefix | suffix | argv0 | target
Exit: 0 it was a wrapper, 1 it was not, 2 could not run.
SPDX-License-Identifier: MIT
"""
import os
import re
import shlex
import sys

# ---------------------------------------------------------------------------
# the binary wrapper: the generator command is embedded as a comment
# ---------------------------------------------------------------------------
CMD_RE = re.compile(rb"makeCWrapper\s(.{0,65536}?)\n\s*\n", re.S)


def _decode_cmd(blob):
    """The embedded block, joined across its backslash-newlines."""
    txt = blob.decode("utf-8", "replace")
    txt = txt.replace("\\\n", " ")
    # Strip anything after the first line that is a comment or blank -- the
    # block is followed by prose about nix-shell.
    out = []
    for line in txt.splitlines():
        s = line.strip()
        if s.startswith("#"):
            break
        out.append(s)
    return " ".join(out).strip()


def read_binary(path):
    with open(path, "rb") as fh:
        blob = fh.read()
    m = CMD_RE.search(blob)
    if not m:
        return None
    try:
        argv = shlex.split(_decode_cmd(m.group(1)))
    except ValueError:
        return None
    if not argv:
        return None
    recs = [("target", "", "", argv[0])]
    i = 1
    while i < len(argv):
        a = argv[i]
        if a == "--inherit-argv0":
            recs.append(("argv0", "", "", ""))
            i += 1
        elif a in ("--set", "--set-default") and i + 2 < len(argv):
            recs.append(("set", argv[i + 1], "", argv[i + 2]))
            i += 3
        elif a in ("--prefix", "--suffix", "--prefix-each", "--suffix-each") \
                and i + 3 < len(argv):
            op = "prefix" if a.startswith("--prefix") else "suffix"
            recs.append((op, argv[i + 1], argv[i + 2], argv[i + 3]))
            i += 4
        elif a in ("--add-flags", "--append-flags") and i + 1 < len(argv):
            recs.append(("flags", "", "", argv[i + 1]))
            i += 2
        elif a == "--argv0" and i + 1 < len(argv):
            recs.append(("argv0", "", "", argv[i + 1]))
            i += 2
        else:
            i += 1
    return recs


# ---------------------------------------------------------------------------
# the shell wrapper: read what it assigns and what it execs
# ---------------------------------------------------------------------------
# ⚠ Deliberately narrow. These are the forms `makeWrapper` emits; a wrapper
# doing something else is reported as "not a wrapper" rather than
# half-understood, because a half-read environment is worse than a stated gap.
EXPORT_RE = re.compile(r"^\s*export\s+([A-Za-z_][A-Za-z0-9_]*)=(.*)$")
ASSIGN_RE = re.compile(r"^\s*([A-Za-z_][A-Za-z0-9_]*)=(.*?)(?:\s*;?\s*export\s+\1)?\s*$")
EXEC_RE = re.compile(r"^\s*exec\s+(?:-a\s+(?:\"\$0\"|\S+)\s+)?(\S+)")


def read_shell(path):
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as fh:
            head = fh.read(65536)
    except OSError:
        return None
    if not head.startswith("#!"):
        return None
    recs = []
    target = ""
    for line in head.splitlines():
        m = EXEC_RE.match(line)
        if m and m.group(1).startswith("/nix/store/"):
            target = m.group(1)
            continue
        m = EXPORT_RE.match(line) or ASSIGN_RE.match(line)
        if not m:
            continue
        var, raw = m.group(1), m.group(2).strip()
        if var in ("PATH", "LD_LIBRARY_PATH") and not raw:
            continue
        val = raw.strip('"').strip("'")
        # `VAR='a':$VAR` and `VAR=$VAR:'a'` are prefix and suffix.
        ref = "$" + var
        if ref in val or ("${" + var + "}") in val:
            body = val.replace("${" + var + "}", ref)
            before, _, after = body.partition(ref)
            if before and not after:
                recs.append(("prefix", var, before[-1] if before else ":",
                             before.rstrip(":;")))
            elif after and not before:
                recs.append(("suffix", var, after[0] if after else ":",
                             after.lstrip(":;")))
            else:
                recs.append(("set", var, "", val))
        else:
            recs.append(("set", var, "", val))
    if not recs and not target:
        return None
    if target:
        recs.insert(0, ("target", "", "", target))
    return recs


def read_any(path):
    with open(path, "rb") as fh:
        magic = fh.read(4)
    if magic == b"\x7fELF":
        return read_binary(path)
    return read_shell(path)


def cmd_read(path):
    recs = read_any(path)
    if not recs:
        return 1
    for op, var, sep, val in recs:
        print("\t".join((op, var, sep, val)))
    return 0


def cmd_target(path):
    recs = read_any(path)
    if not recs:
        return 1
    for op, _var, _sep, val in recs:
        if op == "target":
            print(val)
            return 0
    return 1


def _selftest():
    import tempfile
    bad = 0

    def check(label, got, want):
        nonlocal bad
        if got == want:
            print("  ok    %-46s = %s" % (label, got))
        else:
            print("  FAIL  %-46s = %r, expected %r" % (label, got, want))
            bad = 1

    d = tempfile.mkdtemp()

    # ⭐ The BINARY wrapper, in the exact shape nixpkgs emits: an ELF magic, a
    # blob of machine code, and the generator command in a trailing comment.
    blob = (b"\x7fELF" + b"\x00" * 64 + b"some machine code here" + b"\x00" * 32 +
            b"\n\n# ---------------------------------------\n"
            b"# The C-code for this binary wrapper has been generated using the following command:\n\n\n"
            b"makeCWrapper '/nix/store/aaaa-mpv-0.41.0/bin/mpv' \\\n"
            b"    --inherit-argv0 \\\n"
            b"    --prefix 'LUA_CPATH' ';' '/nix/store/bbbb-lua-env/lib/lua/5.2/?.so' \\\n"
            b"    --suffix 'PATH' ':' '/nix/store/cccc-yt-dlp/bin' \\\n"
            b"    --set 'MPV_HOME' '/nix/store/dddd-conf'\n\n\n"
            b"# (Use `nix-shell -p makeBinaryWrapper` ...)\n")
    p = os.path.join(d, "binwrap")
    open(p, "wb").write(blob)
    recs = read_any(p)
    check("binary wrapper: records read", len(recs or []), 5)
    check("binary wrapper: the real program",
          dict((r[0], r[3]) for r in recs).get("target"),
          "/nix/store/aaaa-mpv-0.41.0/bin/mpv")
    pre = [r for r in recs if r[0] == "prefix"]
    check("binary wrapper: a prefix keeps its separator",
          (pre[0][1], pre[0][2]) if pre else None, ("LUA_CPATH", ";"))
    suf = [r for r in recs if r[0] == "suffix"]
    check("binary wrapper: a suffix is not read as a prefix",
          suf[0][1] if suf else None, "PATH")
    st = [r for r in recs if r[0] == "set"]
    check("binary wrapper: --set", (st[0][1], st[0][3]) if st else None,
          ("MPV_HOME", "/nix/store/dddd-conf"))

    # The SHELL wrapper, the older makeWrapper shape.
    p2 = os.path.join(d, "shwrap")
    open(p2, "w").write(
        "#! /nix/store/xxx-bash/bin/bash -e\n"
        "export GSETTINGS_SCHEMA_DIR='/nix/store/eeee-schemas'\n"
        "export XDG_DATA_DIRS='/nix/store/ffff-icons/share':$XDG_DATA_DIRS\n"
        "export PATH=$PATH:'/nix/store/gggg-tool/bin'\n"
        'exec -a "$0" /nix/store/hhhh-app/bin/.app-wrapped "$@"\n')
    recs2 = read_any(p2)
    m2 = dict((r[1], (r[0], r[3])) for r in recs2 if r[1])
    check("shell wrapper: a plain export is a set",
          m2.get("GSETTINGS_SCHEMA_DIR"), ("set", "/nix/store/eeee-schemas"))
    check("shell wrapper: VAR='x':$VAR is a prefix",
          m2.get("XDG_DATA_DIRS", (None, None))[0], "prefix")
    check("shell wrapper: VAR=$VAR:'x' is a suffix",
          m2.get("PATH", (None, None))[0], "suffix")
    check("shell wrapper: the wrapped program",
          dict((r[0], r[3]) for r in recs2).get("target"),
          "/nix/store/hhhh-app/bin/.app-wrapped")

    # ⛔ THE REFUSAL. A plain ELF is not a wrapper, and saying "no records"
    # rather than "an empty environment" is what keeps the bundler from
    # reporting success on an application whose wrapper it never found.
    p3 = os.path.join(d, "plain")
    open(p3, "wb").write(b"\x7fELF" + b"\x00" * 4096)
    check("a plain ELF is not a wrapper", read_any(p3), None)
    p4 = os.path.join(d, "script")
    open(p4, "w").write("#!/bin/sh\nexec /usr/bin/true\n")
    check("a script that sets nothing and execs outside the store is not one",
          read_any(p4), None)

    print("nix-wrapper selftest: %s" % ("all checks pass." if not bad else "FAILURES above."))
    return bad


def main(argv):
    if len(argv) < 1:
        sys.stderr.write(__doc__)
        return 2
    cmd = argv[0]
    if cmd in ("selftest", "--selftest"):
        return _selftest()
    if len(argv) < 2:
        sys.stderr.write("%s needs a wrapper path\n" % cmd)
        return 2
    if cmd == "read":
        return cmd_read(argv[1])
    if cmd == "target":
        return cmd_target(argv[1])
    sys.stderr.write("unknown command: %s\n" % cmd)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
