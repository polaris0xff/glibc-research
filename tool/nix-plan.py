#!/usr/bin/env python3
"""nix-plan.py - turn a nixpkgs derivation into a pgb build plan.

Reads `nix derivation show --recursive` JSON on stdin. Writes one JSON object.

⛔ THE DERIVATION IS THE SOURCE OF TRUTH, NOT THE .nix EXPRESSION. A derivation
is what nix decided after every override, overlay and conditional in nixpkgs
has run. The expression is what somebody wrote. Reading the expression means
re-implementing nix's evaluator badly, which is exactly the work the operator's
ruling says not to do.

⚠ WHAT A PLAN DELIBERATELY DOES NOT CARRY: nixpkgs' builder script. The plan is
the INPUTS -- source, patches, flags, dependency names -- not the recipe, and
pgb's own build drives the recipe. Carrying stdenv's setup hooks would mean
carrying stdenv, and stdenv is where the /nix/store paths get baked in.

Usage:
  nix derivation show DRV --recursive | nix-plan.py ATTR DRVPATH [--nix-prefix P]
"""

import json
import os
import subprocess
import sys


def sh(cmd):
    try:
        return subprocess.run(
            cmd, capture_output=True, text=True, timeout=60
        ).stdout.strip()
    except Exception:
        return ""


def split_ws(s):
    """Normalise a derivation attribute to a list of strings.

    ⚠ THE SAME ATTRIBUTE IS A SPACE-JOINED STRING IN ONE DERIVATION SHAPE AND
    A JSON ARRAY IN THE OTHER. Treating the array as a string gives you its
    python repr, one 'flag' per plan and every one of them wrong.
    """
    if s is None:
        return []
    if isinstance(s, list):
        return [str(x) for x in s if str(x)]
    if isinstance(s, bool):
        return []
    return [x for x in str(s).split() if x]



def dep_names_of(env, key):
    """Store paths -> readable names, hash prefix stripped."""
    out = []
    for p in split_ws(env.get(key)):
        b = os.path.basename(p)
        out.append(b.split("-", 1)[1] if "-" in b else b)
    return out


def full_store(p):
    """⛔ `nix derivation show` KEYS BY BASENAME, and every nix command that
    takes a derivation wants an absolute store path. Emitting the basename put
    unusable paths in every plan: `nix derivation show
    bi27b...-ncurses-6.6.drv` resolves it against the CURRENT DIRECTORY and
    reports "No such file or directory", so the recursive dependency build
    failed on every dependency and reported each one as unbuildable.
    """
    if not p:
        return p
    return p if p.startswith("/") else "/nix/store/" + p


def main(argv):
    attr = argv[1] if len(argv) > 1 else "?"
    drvpath = argv[2] if len(argv) > 2 else ""
    nixpfx = ""
    if "--nix-prefix" in argv:
        nixpfx = argv[argv.index("--nix-prefix") + 1]
    doc = json.load(sys.stdin)
    drvs = doc.get("derivations", doc)
    if drvpath not in drvs:
        # ⚠ `nix derivation show` keys by the FULL store path; a caller that
        # passed a bare basename should still work rather than get an empty plan.
        cand = [k for k in drvs if k.endswith(os.path.basename(drvpath))]
        if not cand:
            print("nix-plan: %s is not in the document" % drvpath, file=sys.stderr)
            return 1
        drvpath = cand[0]
    top = drvs[drvpath]

    # ⛔ TWO DERIVATION SHAPES, AND THE NEW ONE CARRIES NOTHING IN `env`.
    # nixpkgs is migrating to __structuredAttrs, where the attributes are
    # proper JSON under a top-level `structuredAttrs` key and `env` holds only
    # the output names. Measured on this machine: bash is the old shape and
    # tmux is the new one, and reading only `env` gave tmux a plan with no
    # source at all -- "the plan has no source", which reads like a nixpkgs
    # defect rather than a missing branch here.
    #
    # ⚠ AND THE TYPES DIFFER: the old shape space-joins lists into a string,
    # the new one keeps them as JSON arrays. `attrs()` below normalises both,
    # which is why nothing downstream has to know which shape it came from.
    sattrs = top.get("structuredAttrs")
    env = top.get("env", {})
    if isinstance(sattrs, dict) and sattrs:
        env = dict(sattrs)

    # ⭐ INDEX EVERY DERIVATION BY THE OUTPUTS IT PRODUCES. The recursive
    # document already contains the whole graph, so an output path can be
    # mapped back to the derivation that makes it WITHOUT the local store
    # holding anything.
    #
    # ⛔ THIS REPLACED `nix-store -q --deriver` AS THE PRIMARY ROUTE BECAUSE
    # THAT ROUTE ANSWERS `unknown-deriver` FOR A PATH THAT IS NOT REALISED,
    # which is every dependency of a package nobody has built here. Measured:
    # all five of htop's buildInputs came back with no derivation, so the
    # dependency graph -- the entire point of using nixpkgs as the planner --
    # was empty on the first package that needed one.
    #
    # ⚠ The document writes output paths as BASENAMES and buildInputs as full
    # /nix/store paths. Matching without normalising finds nothing, silently.
    out_index = {}
    for path, d in drvs.items():
        for oname, o in (d.get("outputs") or {}).items():
            op = o.get("path")
            if op:
                out_index[os.path.basename(op)] = full_store(path)

    # INDEX THE FIXED-OUTPUT DERIVATIONS TOO: those are the fetchurl calls, and
    # they are the only place the UPSTREAM URL and its hash exist. Everything
    # else in the graph is built from them.
    by_name = {}
    for path, d in drvs.items():
        outs = d.get("outputs", {})
        out = outs.get("out", {})
        h = out.get("hash") or d.get("env", {}).get("outputHash")
        if not h:
            continue
        e = d.get("env", {})
        urls = split_ws(e.get("urls")) or split_ws(e.get("url"))
        rec = {"drv": full_store(path), "outputHash": h, "urls": urls}
        by_name.setdefault(d.get("name") or e.get("name") or "", []).append(rec)

    def resolve(store_path):
        """store path -> {store, urls, outputHash}.

        ⛔ TWO ROUTES, AND THE FIRST IS THE ONLY EXACT ONE. `nix-store -q
        --deriver` names the derivation that produced this exact path. The
        name index is a fallback for a path nix does not have locally, and it
        can be ambiguous: nixpkgs really does contain several fixed-output
        derivations with one name. An ambiguous fallback keeps every candidate
        URL rather than picking one, because a wrong URL that hashes wrong is
        caught at fetch time and a silently dropped one is not.
        """
        rec = {"store": store_path, "urls": [], "outputHash": ""}
        base = os.path.basename(store_path)
        name = base.split("-", 1)[1] if "-" in base else base
        drv = out_index.get(base, "")
        if not drv and nixpfx:
            drv = sh([os.path.join(nixpfx, "nix-store"), "-q", "--deriver", store_path])
        if drv and os.path.basename(drv) in drvs:
            d = drvs[os.path.basename(drv)]
            e = d.get("env", {})
            rec["urls"] = split_ws(e.get("urls")) or split_ws(e.get("url"))
            rec["outputHash"] = (
                d.get("outputs", {}).get("out", {}).get("hash")
                or e.get("outputHash")
                or ""
            )
            if rec["urls"]:
                return rec
        for c in by_name.get(name, []):
            for u in c["urls"]:
                if u not in rec["urls"]:
                    rec["urls"].append(u)
            rec["outputHash"] = rec["outputHash"] or c["outputHash"]
        return rec

    def dep_names(key):
        return dep_names_of(env, key)

    def dep_records(key):
        """⭐ THE DEPENDENCY GRAPH, WITH ENOUGH TO ACT ON IT.

        A name is not actionable: `ncurses-6.6-dev` is not a nixpkgs attribute
        and guessing the attribute from a store-path name gets it wrong on
        anything with a version suffix, a multiple output, or a rename. The
        DERIVATION is actionable -- it can be planned recursively exactly like
        the top-level one -- so each record carries it.

        ⚠ `nix-store -q --deriver` needs the path present or substitutable. A
        record whose drv could not be resolved is kept with an empty drv
        rather than dropped, because a dependency this tool cannot plan is
        something the caller has to be told about, not something to hide.
        """
        out = []
        for p in split_ws(env.get(key)):
            b = os.path.basename(p)
            drv = out_index.get(b, "")
            if not drv and nixpfx:
                drv = sh([os.path.join(nixpfx, "nix-store"), "-q", "--deriver", p])
                if drv.startswith("unknown"):
                    drv = ""
            out.append(
                {
                    "name": b.split("-", 1)[1] if "-" in b else b,
                    "out": p,
                    "drv": drv,
                }
            )
        return out

    # ⭐ nixpkgs ALREADY KNOWS WHAT BUILD SYSTEM THIS IS, and says so by which
    # setup hooks it puts in nativeBuildInputs. Reading them off is free and it
    # is strictly better than sniffing the unpacked tree: a source that ships
    # both `configure.ac` and a `CMakeLists.txt` is ambiguous to a sniffer and
    # not to the derivation. `autoreconf-hook` in particular is nixpkgs saying
    # "this tarball has no ./configure, generate one" -- htop's source is a
    # bare git export, and without this the build ran `make` in a tree with no
    # Makefile and reported "No targets specified".
    natives = " ".join(dep_names_of(env, "nativeBuildInputs"))
    hooks = []
    for hook, flag in (
        ("autoreconf-hook", "autoreconf"),
        ("cmake-", "cmake"),
        ("meson-", "meson"),
        ("ninja-", "ninja"),
        ("pkg-config-wrapper", "pkg-config"),
        ("rustc", "cargo"),
        ("cargo-", "cargo"),
        ("go-", "go"),
    ):
        if hook in natives and flag not in hooks:
            hooks.append(flag)

    src = env.get("src", "")
    plan = {
        "buildSystemHooks": hooks,
        "schema": "pgb-nix-plan/1",
        "attr": attr,
        "pname": env.get("pname") or env.get("name") or attr,
        "version": env.get("version", ""),
        "drv": full_store(drvpath),
        "system": top.get("system", ""),
        "src": resolve(src) if src else {},
        "patches": [resolve(p) for p in split_ws(env.get("patches"))],
        "configureFlags": split_ws(env.get("configureFlags")),
        "cmakeFlags": split_ws(env.get("cmakeFlags")),
        "mesonFlags": split_ws(env.get("mesonFlags")),
        "makeFlags": split_ws(env.get("makeFlags")),
        "buildInputs": dep_names("buildInputs"),
        "deps": dep_records("buildInputs") + dep_records("propagatedBuildInputs"),
        "nativeBuildInputs": dep_names("nativeBuildInputs"),
        "propagatedBuildInputs": dep_names("propagatedBuildInputs"),
        "outputs": split_ws(env.get("outputs")) or ["out"],
        # ⚠ CARRIED AND NOT ACTED ON, deliberately. These are nixpkgs' own
        # shell fragments; running them needs stdenv's environment, which is
        # the thing a pgb build is trying not to be inside. They are in the
        # plan so a human debugging a failed build can see what nixpkgs did
        # that pgb did not.
        "nix_only": {
            k: env[k]
            for k in ("postPatch", "preConfigure", "postInstall", "prePatch",
                      "preBuild", "postBuild", "patchFlags", "NIX_CFLAGS_COMPILE",
                      "hardeningDisable", "dontDisableStatic", "env")
            if k in env and env[k] not in ("", "1", "0")
        },
        # ⛔ RECORDED SO A PLAN CANNOT BE SILENTLY RE-READ AGAINST ANOTHER
        # nixpkgs. Two plans for one attribute at different revisions describe
        # different sources, and nothing else in the file would say so.
        "nixpkgs": sh(
            [os.path.join(nixpfx, "nix-instantiate"), "--eval", "--expr",
             "(import <nixpkgs> {}).lib.version"]
        ).strip('"') if nixpfx else "",
    }
    json.dump(plan, sys.stdout, indent=1, sort_keys=True)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
