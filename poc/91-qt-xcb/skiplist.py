#!/usr/bin/env python3
"""Everything in the plan that is NOT on the keep list, as a NIX_DEP_SKIP set.

⛔ NAMING WHAT IS NEEDED IS CHECKABLE; NAMING WHAT IS NOT IS A GUESS THAT
GROWS. nixpkgs' qtbase has 61 build inputs and the xcb QPA plugin needs a
dozen; a skip list would have to be revisited every time nixpkgs adds one.

SPDX-License-Identifier: MIT
"""
import json, os, re, sys

plan = json.load(open(os.environ["QTPLAN"]))
keep = {k.lower() for k in os.environ.get("QT_KEEP", "").split()}
out = []
for d in plan.get("deps", []):
    name = d.get("name", "")
    short = re.sub(r"-[0-9].*$", "", re.sub(r"-(dev|lib|out|bin|man|doc)$", "", name))
    if short and short.lower() not in keep and short not in out:
        out.append(short)
print(" ".join(out))
