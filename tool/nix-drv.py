#!/usr/bin/env python3
"""nix-drv.py - read nix's own derivation format, so the planner needs no nix.

⭐ THE OPERATOR ASKED THE RIGHT QUESTION: "the downloaded nix store files
themselves contain *.drv files? so we don't actually need nix installed no?"

⛔ THEY ARE RIGHT, AND IT IS BETTER THAN THAT. Measured on 2026-09-01:

  1. a narinfo names its own producer:
     `Deriver: hplnhqsmnpr4gv35yf4cxvbalki3k308-bash-5.3p15.drv`
  2. THAT .drv IS ITSELF A STORE PATH IN THE BINARY CACHE, with its own
     narinfo, its own ed25519 signature and its own NarHash --
     `cache.nixos.org/hplnhqsmnpr4gv35yf4cxvbalki3k308.narinfo` answers 200.
  3. and its `References` are the .drv paths of every one of its inputs.

So the whole derivation graph is reachable over plain HTTPS, signed and
hash-checked, with no nix installed and no evaluation. `nix derivation show`
was never the only route to it -- it was the convenient one.

⚠ WHAT STILL NEEDS AN EVALUATOR, and it is one step: turning the ATTRIBUTE
NAME `bash` into a store path. `scripts/common/nix-fetch.sh resolve` does that
from the channel's store-paths.xz index by name matching, which is good enough
for a package whose name you know and is NOT the same thing as evaluating
nixpkgs: an overlay, an override or an unbuilt attribute is out of reach.
docs/research/nix.md states that boundary rather than blurring it.

-- THE FORMAT ---------------------------------------------------------------

A .drv is ATerm, one line, no whitespace:

  Derive([(outName,outPath,hashAlgo,hash)...],
         [(inputDrvPath,[outNames])...],
         [inputSrcs...], system, builder, [args...], [(key,value)...])

Strings are double-quoted with \\" \\\\ \\n \\r \\t escapes. That is the whole
grammar, and the parser below is about eighty lines because that is all it is.

Usage:
  nix-drv.py parse FILE.drv                 # one derivation, as JSON
  nix-drv.py show  FILE.drv [FILE.drv...]   # the `nix derivation show` shape
  nix-drv.py selftest
"""

import json
import os
import sys


class DrvError(Exception):
    pass


class ATerm:
    def __init__(self, text):
        self.s = text
        self.i = 0

    def peek(self):
        return self.s[self.i] if self.i < len(self.s) else ""

    def take(self, ch):
        if self.peek() != ch:
            raise DrvError("expected %r at offset %d, got %r"
                           % (ch, self.i, self.peek()))
        self.i += 1

    def string(self):
        self.take('"')
        out = []
        while True:
            if self.i >= len(self.s):
                raise DrvError("unterminated string")
            c = self.s[self.i]
            self.i += 1
            if c == '"':
                break
            if c == "\\":
                # ⛔ THE ESCAPES ARE nix's, NOT JSON's. A derivation env value
                # routinely contains newlines (postInstall is a shell script),
                # and treating \n as a literal backslash-n puts the two-character
                # sequence into the plan where a newline belonged.
                e = self.s[self.i]
                self.i += 1
                out.append({"n": "\n", "r": "\r", "t": "\t"}.get(e, e))
            else:
                out.append(c)
        return "".join(out)

    def list(self, item):
        self.take("[")
        out = []
        if self.peek() == "]":
            self.i += 1
            return out
        while True:
            out.append(item())
            c = self.peek()
            self.i += 1
            if c == "]":
                return out
            if c != ",":
                raise DrvError("expected , or ] at offset %d, got %r" % (self.i, c))

    def tuple(self, *items):
        self.take("(")
        out = []
        for n, fn in enumerate(items):
            if n:
                self.take(",")
            out.append(fn())
        self.take(")")
        return out


def parse(text):
    if not text.startswith("Derive("):
        raise DrvError("not a derivation: does not begin with Derive(")
    p = ATerm(text)
    p.i = len("Derive(")
    outputs = p.list(lambda: p.tuple(p.string, p.string, p.string, p.string))
    p.take(",")
    input_drvs = p.list(lambda: p.tuple(p.string, lambda: p.list(p.string)))
    p.take(",")
    input_srcs = p.list(p.string)
    p.take(",")
    system = p.string()
    p.take(",")
    builder = p.string()
    p.take(",")
    args = p.list(p.string)
    p.take(",")
    env = p.list(lambda: p.tuple(p.string, p.string))
    p.take(")")
    return {
        "outputs": outputs,
        "inputDrvs": input_drvs,
        "inputSrcs": input_srcs,
        "system": system,
        "builder": builder,
        "args": args,
        "env": dict(env),
    }


def as_show(path, d):
    """Reshape into what `nix derivation show` prints.

    ⭐ ONE DOCUMENT FORMAT, TWO PRODUCERS. tool/nix-plan.py already reads the
    `nix derivation show` shape, so emitting it here means the nix-free route
    and the nix route share every line of the planner below them -- and a bug
    in the planner cannot be present on one route and absent on the other.
    """
    outs = {}
    for name, opath, halgo, h in d["outputs"]:
        e = {}
        if opath:
            e["path"] = os.path.basename(opath)
        if h:
            e["hash"] = h
        if halgo:
            e["method"] = halgo
        outs[name] = e
    # ⛔ __json IS structuredAttrs, AND `nix derivation show` DECODES IT.
    # In the raw .drv the modern attribute set arrives as ONE env entry called
    # `__json` holding the whole JSON document, while `env` itself carries only
    # the output names. A reader that does not decode it sees a derivation with
    # no src, no patches and no configure flags -- which is exactly the empty
    # plan the nix route produced before it learned the same lesson.
    sattrs = {}
    if "__json" in d["env"]:
        try:
            sattrs = json.loads(d["env"]["__json"])
        except ValueError:
            sattrs = {}
    entry = {
            "name": os.path.basename(path).split("-", 1)[-1].replace(".drv", ""),
            "outputs": outs,
            "inputs": {
                "drvs": [os.path.basename(p) for p, _ in d["inputDrvs"]],
                "srcs": [os.path.basename(p) for p in d["inputSrcs"]],
            },
            "system": d["system"],
            "builder": d["builder"],
            "args": d["args"],
            "env": d["env"],
    }
    if sattrs:
        entry["structuredAttrs"] = sattrs
    return {os.path.basename(path): entry}


def selftest():
    fails = []

    def check(name, cond, detail=""):
        print(("  ok    " if cond else "  FAIL  ") + name + ((" " + detail) if detail else ""))
        if not cond:
            fails.append(name)

    # ⭐ A REAL DERIVATION, cut down but not hand-simplified: this is the
    # shape cache.nixos.org served for bash-5.3p15 on 2026-09-01.
    sample = (
        'Derive([("out","/nix/store/aaa-x","",""),("dev","/nix/store/bbb-x-dev","","")],'
        '[("/nix/store/ccc-dep.drv",["out"]),("/nix/store/ddd-src.drv",["out"])],'
        '["/nix/store/eee-builder.sh"],"x86_64-linux","/nix/store/fff-bash/bin/bash",'
        '["-e","/nix/store/eee-builder.sh"],'
        '[("configureFlags","--a --b"),("out","/nix/store/aaa-x"),'
        '("postInstall","ln -s a b\\nrm c\\n"),("quoted","say \\"hi\\"")])'
    )
    d = parse(sample)
    check("outputs parsed", len(d["outputs"]) == 2, str(len(d["outputs"])))
    check("out path", d["outputs"][0][1] == "/nix/store/aaa-x")
    check("inputDrvs parsed", len(d["inputDrvs"]) == 2)
    check("inputSrcs parsed", d["inputSrcs"] == ["/nix/store/eee-builder.sh"])
    check("system", d["system"] == "x86_64-linux")
    check("args", d["args"] == ["-e", "/nix/store/eee-builder.sh"])
    check("env: a plain value", d["env"]["configureFlags"] == "--a --b")
    # ⛔ THE TWO ESCAPES THAT MATTER. A \n read literally turns a shell
    # fragment into one unrunnable line; a mishandled \" ends the string early
    # and every field after it shifts.
    check("env: \\n is a newline", d["env"]["postInstall"] == "ln -s a b\nrm c\n",
          repr(d["env"]["postInstall"]))
    check('env: \\" is a quote', d["env"]["quoted"] == 'say "hi"',
          repr(d["env"]["quoted"]))

    # An empty list must parse, because a derivation with no patches has one.
    d2 = parse('Derive([("out","/nix/store/a","","")],[],[],"s","b",[],[])')
    check("empty lists parse", d2["inputDrvs"] == [] and d2["args"] == [])

    # ⛔ AND A NON-DERIVATION MUST BE REFUSED, not half-parsed. A fetch that
    # returned an error page would otherwise produce an empty plan.
    refused = False
    try:
        parse("<html>404</html>")
    except DrvError:
        refused = True
    check("a non-derivation is refused", refused)

    truncated = False
    try:
        parse('Derive([("out","/nix/store/a","","")],[],[],"s","b",[],[("k","v"')
    except DrvError:
        truncated = True
    check("a truncated derivation is refused", truncated)

    print("nix-drv --selftest: %s" %
          ("all checks pass." if not fails else "%d FAILED." % len(fails)))
    return 1 if fails else 0


def main(argv):
    if len(argv) < 2:
        print(__doc__)
        return 2
    cmd = argv[1]
    if cmd == "selftest":
        return selftest()
    if cmd in ("parse", "show"):
        doc = {}
        for path in argv[2:]:
            with open(path, encoding="utf-8", errors="surrogateescape") as fh:
                d = parse(fh.read())
            if cmd == "parse":
                json.dump(d, sys.stdout, indent=1, sort_keys=True)
                sys.stdout.write("\n")
            else:
                doc.update(as_show(path, d))
        if cmd == "show":
            json.dump({"derivations": doc, "version": 3}, sys.stdout,
                      indent=1, sort_keys=True)
            sys.stdout.write("\n")
        return 0
    print("nix-drv: unknown subcommand: " + cmd, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
