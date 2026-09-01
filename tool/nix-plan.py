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
    return [x for x in (s or "").split() if x]


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
    env = top.get("env", {})

    # ⭐ INDEX THE FIXED-OUTPUT DERIVATIONS: those are the fetchurl calls, and
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
        rec = {"drv": path, "outputHash": h, "urls": urls}
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
        drv = ""
        if nixpfx:
            drv = sh([os.path.join(nixpfx, "nix-store"), "-q", "--deriver", store_path])
        if drv and drv in drvs:
            d = drvs[drv]
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
        out = []
        for p in split_ws(env.get(key)):
            b = os.path.basename(p)
            out.append(b.split("-", 1)[1] if "-" in b else b)
        return out

    src = env.get("src", "")
    plan = {
        "schema": "pgb-nix-plan/1",
        "attr": attr,
        "pname": env.get("pname") or env.get("name") or attr,
        "version": env.get("version", ""),
        "drv": drvpath,
        "system": top.get("system", ""),
        "src": resolve(src) if src else {},
        "patches": [resolve(p) for p in split_ws(env.get("patches"))],
        "configureFlags": split_ws(env.get("configureFlags")),
        "cmakeFlags": split_ws(env.get("cmakeFlags")),
        "mesonFlags": split_ws(env.get("mesonFlags")),
        "makeFlags": split_ws(env.get("makeFlags")),
        "buildInputs": dep_names("buildInputs"),
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
