#!/usr/bin/env python3
"""onelf-recipe.py - our AppDir's sharun `.env` as an `onelf.toml` recipe.

⛔ WHY THIS EXISTS. `experiments/90-` compares three PACKERS on ONE payload, so
every arm has to be given the same information. Ours reaches sharun through a
`.env` file; `QaidVoid/onelf` reaches its runtime through `[env]` in
`onelf.toml`. Handing onelf a payload without the environment would be
measuring our configuration against their lack of it.

Two translations, and both matter:

  ${SHARUN_DIR}  ->  ${ONELF_DIR}   the package root, expanded at run time
  ${VAR}         ->  $${VAR}        onelf expands ${VAR} at RECIPE-LOAD time
                                    against the packer's environment; `$$` is
                                    its escape for "expand at run time"

⛔ AND REPEATED KEYS HAVE TO BE FOLDED. sharun's `.env` may set `XDG_DATA_DIRS`
fifteen times, each line prepending to the last; TOML cannot repeat a key. The
lines are replayed in order against an accumulator, so the single value that
comes out is what sharun would have ended up with.

⚠ `[compression] level` is set to match what our own packer uses. onelf's
default is 12 and `tool/nix-appimage.sh` packs dwarfs at zstd:19; comparing
those two would measure a default rather than a design.

⭐ AND WRITING THIS FOUND A DEFECT IN OUR OWN `.env`. The wrapper-lifting step
appends a record per program, and kdenlive, melt, ffmpeg and kdenlive_render
each carry the same QT_PLUGIN_PATH prefixes -- so the bundle asked Qt to scan
**sixty plugin directories, most of them four times over**, on every start.
`--fold-env` rewrites a `.env` with repeated keys folded and repeated path
components dropped, keeping first occurrence order.

Usage: onelf-recipe.py APPDIR MAIN_PROGRAM [--level N] > onelf.toml
       onelf-recipe.py --fold-env PATH
SPDX-License-Identifier: MIT
"""
import os
import re
import sys


def fold_env(path):
    """Replay `.env` in order; return {key: value} with ${KEY} resolved."""
    acc = {}
    order = []
    if not os.path.exists(path):
        return order, acc
    for raw in open(path, encoding="utf-8", errors="replace"):
        line = raw.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        k, _, v = line.partition("=")
        k = k.strip()
        if not k:
            continue
        if k not in acc:
            order.append(k)
            # First mention: ${K} means the LIVE variable.
            acc[k] = v.replace("${%s}" % k, "\0LIVE\0")
        else:
            acc[k] = v.replace("${%s}" % k, acc[k])
    return order, acc


def dedupe_path(v, sep=":"):
    """Drop repeated components, keeping the first occurrence's position."""
    if sep not in v:
        return v
    seen = set()
    out = []
    for part in v.split(sep):
        if part in seen:
            continue
        seen.add(part)
        out.append(part)
    return sep.join(out)


def cmd_fold_env(path):
    order, acc = fold_env(path)
    if not order:
        return 0
    lines = []
    for k in order:
        v = acc[k]
        sep = ";" if (";" in v and ":" not in v) else ":"
        v = dedupe_path(v, sep)
        lines.append("%s=%s" % (k, v.replace("\0LIVE\0", "${%s}" % k)))
    with open(path + ".part", "w") as w:
        w.write("\n".join(lines) + "\n")
    os.replace(path + ".part", path)
    sys.stderr.write("env-fold: %d keys\n" % len(order))
    return 0


def to_onelf(v):
    v = v.replace("${SHARUN_DIR}", "${ONELF_DIR}")
    # Any remaining ${VAR} is a live variable for onelf, which needs $$.
    v = re.sub(r"\$\{(?!ONELF_DIR\b)([A-Za-z_][A-Za-z0-9_]*)\}", r"$${\1}", v)
    v = v.replace("\0LIVE\0", "$${%s}" % "PLACEHOLDER")
    return v


def main(argv):
    if len(argv) < 2:
        sys.stderr.write(__doc__)
        return 2
    if argv[0] == "--fold-env":
        return cmd_fold_env(argv[1])
    appdir, main_prog = argv[0], argv[1]
    level = 19
    if "--level" in argv:
        level = int(argv[argv.index("--level") + 1])

    print("[package]")
    print('name = "%s"' % main_prog)
    print('command = "bin/%s"' % main_prog)
    print()
    binpath = os.path.join(appdir, "shared", "bin")
    for name in sorted(os.listdir(binpath)) if os.path.isdir(binpath) else []:
        if name == main_prog:
            continue
        print("[[entrypoint]]")
        print('name = "%s"' % name)
        print('path = "bin/%s"' % name)
        print()
    print("[compression]")
    print("level = %d" % level)
    print()
    print("[bundle]")
    print("skip = true")
    print()
    order, acc = fold_env(os.path.join(appdir, ".env"))
    if order:
        print("[env]")
        for k in order:
            sep = ";" if (";" in acc[k] and ":" not in acc[k]) else ":"
            v = to_onelf(dedupe_path(acc[k], sep)).replace("$${PLACEHOLDER}", "$${%s}" % k)
            # TOML basic string: escape backslashes and quotes.
            v = v.replace("\\", "\\\\").replace('"', '\\"')
            print('%s = "%s"' % (k, v))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
