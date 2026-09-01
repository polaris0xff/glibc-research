#!/usr/bin/env python3
"""nix-index.py - the two indexes that turn a package NAME into a derivation
with NO nix and NO evaluation.

⛔ WHY THIS EXISTS. `scripts/common/nix-fetch.sh` resolves a name against the
channel's `store-paths.xz`, which is a list of store paths and nothing else.
`experiments/83-` measured what that costs: the narinfo `Deriver:` field that
names the producing `.drv` is present for **3%** of paths sampled by stride,
**1%** of paths with no output suffix and **47%** of twenty packages a person
would name. So the route works for some packages and silently does not for
most, and `docs/research/nix.md` records that a fallback is mandatory.

⛔ AND IT MEASURED SOMETHING WORSE THAT NOBODY NOTICED: `store-paths.xz` is
**every system the channel built**, not x86_64-linux. Resolving `nix-2.35.2`
by name in this tree returned an **aarch64-darwin** build -- a Mach-O
executable, fetched and verified and completely useless -- and nothing in the
route could tell. Its closure was 57 paths with no glibc in it, which is what
gave it away.

⭐ TWO INDEXES FIX BOTH, and both are plain HTTPS:

  1. `releases.nixos.org/nixpkgs/<pin>/packages.json.br`
     ⭐ Served with `Content-Encoding: br` and `Content-Type: application/json`,
     so `curl --compressed` decodes it and no brotli library is needed. 10 MB
     on the wire, ~400 MB of JSON, 149,812 attributes. Per attribute it gives
     `name`, `pname`, `version`, **`system`** and **`outputName`** -- which is
     how `bash` is known to be `bash-interactive-5.3p15` (the case
     `docs/research/nix.md` finding 3b says no name match can know) and how
     `jq`'s default output is known to be `bin` (the case the bundler got
     wrong once).

  2. `hydra.nixos.org/job/<project>/<jobset>/<attr>.<system>/latest-finished`
     ⭐ The build that produced the channel: **`drvpath`**, `system`, and every
     output's store path. That is name -> derivation directly, with no
     `Deriver:` field involved, so the 3%/47% availability ceiling does not
     apply to it.
     ⚠ It answers for the jobset's LATEST finished eval, which is not
     necessarily the revision the channel pinned. The caller must check that,
     and `nix-fetch.sh drv` does: the outputs it names have to be in the
     channel's own `store-paths.xz`.

⛔ THE 400 MB IS STREAMED, NEVER LOADED. `json.load` on packages.json costs
gigabytes of dict for six fields per package. `index` walks the top-level
`"packages"` object with `JSONDecoder.raw_decode`, one value at a time, and
writes a TSV of about 10 MB that every later lookup reads instead.

Usage:
  nix-index.py index PACKAGES_JSON OUT_TSV     # stream -> attr/name/system TSV
  nix-index.py hydra HYDRA_JSON [--system S]   # latest-finished -> key: value
  nix-index.py selftest

Exit: 0 did what was asked, 1 did not, 2 could not run.
SPDX-License-Identifier: MIT
"""
import io
import json
import os
import sys

DEC = json.JSONDecoder()


def _skip_ws(buf, i):
    while i < len(buf) and buf[i] in " \t\r\n":
        i += 1
    return i


def stream_packages(fh, emit, chunk=1 << 20):
    """Walk the top-level {"packages": {ATTR: {...}, ...}} object.

    ⛔ Reads in chunks and decodes ONE package value at a time. The file is
    400 MB and the whole point is never to hold it.

    ⚠ `chunk` is a parameter ONLY so the selftest can force the refill paths to
    run many times over a small document. Production reads 1 MiB.
    """
    buf = fh.read(chunk)
    # Find the opening of the packages object.
    key = '"packages"'
    while key not in buf:
        more = fh.read(1 << 20)
        if not more:
            raise ValueError('no "packages" key in this document')
        buf += more
    i = buf.index(key) + len(key)
    i = _skip_ws(buf, i)
    if i >= len(buf) or buf[i] != ':':
        raise ValueError('"packages" is not an object member')
    i += 1
    i = _skip_ws(buf, i)
    if i >= len(buf) or buf[i] != '{':
        raise ValueError('"packages" is not an object')
    i += 1
    n = 0
    while True:
        # Make sure there is enough buffered to decode a name and a value.
        while True:
            i = _skip_ws(buf, i)
            if i < len(buf):
                break
            more = fh.read(chunk)
            if not more:
                return n
            # ⛔ THE REFILL MUST KEEP WHAT IT JUST READ. This line was
            # `buf = buf[i:]` with `more` dropped on the floor, so every time
            # the walk ran out of buffer it silently discarded a whole chunk:
            # 103,571 attributes out of 149,812 -- a 31% loss that looked like
            # a complete index. Found by comparing against `json.load`'s count
            # on the same file, which is the control this reader needs.
            buf = buf[i:] + more
            i = 0
        if buf[i] == '}':
            return n
        if buf[i] == ',':
            i += 1
            continue
        # The attribute name, then its value. raw_decode needs the whole value
        # in the buffer, so grow until it stops raising.
        while True:
            try:
                attr, j = DEC.raw_decode(buf, i)
                j = _skip_ws(buf, j)
                if j >= len(buf) or buf[j] != ':':
                    raise ValueError
                # ⛔ raw_decode DOES NOT SKIP LEADING WHITESPACE, and
                # `json.dumps` puts a space after every colon. Decoding from
                # `j + 1` therefore fails on a document a human would call
                # well-formed, the retry loop reads to EOF, and what comes out
                # is "truncated packages.json" -- a message about the input for
                # a defect in the reader. The selftest's filler value exists to
                # keep this honest across a read boundary.
                val, k = DEC.raw_decode(buf, _skip_ws(buf, j + 1))
                break
            except Exception:
                more = fh.read(chunk)
                if not more:
                    raise ValueError("truncated packages.json")
                buf += more
        emit(attr, val)
        n += 1
        i = k
        # Keep the buffer from growing without bound.
        if i > max(1 << 22, chunk * 4):
            buf = buf[i:]
            i = 0


def cmd_index(argv):
    if len(argv) < 2:
        sys.stderr.write("index needs PACKAGES_JSON and OUT_TSV\n")
        return 2
    src, out = argv[0], argv[1]
    n = 0
    with open(out + ".part", "w") as w:
        w.write("# attr\tname\tpname\tversion\tsystem\toutputName\toutputs\n")

        def emit(attr, val):
            nonlocal n
            if not isinstance(val, dict):
                return
            outs = val.get("outputs") or {}
            w.write("\t".join([
                attr,
                str(val.get("name", "")),
                str(val.get("pname", "")),
                str(val.get("version", "")),
                str(val.get("system", "")),
                str(val.get("outputName", "")),
                ",".join(sorted(outs.keys())) if isinstance(outs, dict) else "",
            ]) + "\n")
            n += 1

        with open(src, "r", encoding="utf-8", errors="replace") as fh:
            stream_packages(fh, emit)
    os.replace(out + ".part", out)
    sys.stderr.write("nix-index: %d attributes -> %s\n" % (n, out))
    return 0


def cmd_hydra(argv):
    want_system = "x86_64-linux"
    args = []
    i = 0
    while i < len(argv):
        if argv[i] == "--system":
            i += 1
            want_system = argv[i] if i < len(argv) else want_system
        else:
            args.append(argv[i])
        i += 1
    if not args:
        sys.stderr.write("hydra needs a JSON file\n")
        return 2
    try:
        d = json.load(open(args[0]))
    except Exception as e:
        sys.stderr.write("hydra: not JSON: %s\n" % e)
        return 1
    drv = d.get("drvpath") or ""
    system = d.get("system") or ""
    if not drv:
        sys.stderr.write("hydra: the reply names no drvpath\n")
        return 1
    # ⛔ A REPLY FOR THE WRONG SYSTEM IS A FAILURE, NOT A RESULT. This is the
    # exact defect the store-paths.xz route had and could not see.
    if system != want_system:
        sys.stderr.write("hydra: reply is for %s, wanted %s\n" % (system, want_system))
        return 1
    print("Drv: %s" % drv)
    print("System: %s" % system)
    print("Nixname: %s" % (d.get("nixname") or ""))
    print("Job: %s" % (d.get("job") or ""))
    print("Finished: %s" % (d.get("finished")))
    print("Buildstatus: %s" % (d.get("buildstatus")))
    evs = [e for e in (d.get("jobsetevals") or []) if isinstance(e, int)]
    if evs:
        # ⚠ ONE LINE, NOT ONE PER EVAL. A popular job is in dozens of evals and
        # the list buries the three fields a caller actually reads.
        print("Evals: %d" % len(evs))
        print("EvalLatest: %d" % max(evs))
    outs = d.get("buildoutputs") or {}
    for name in sorted(outs):
        p = (outs[name] or {}).get("path") or ""
        if p:
            print("Out.%s: %s" % (name, p))
    return 0


def _selftest():
    bad = 0

    def check(label, got, want):
        nonlocal bad
        if got == want:
            print("  ok    %-44s = %s" % (label, got))
        else:
            print("  FAIL  %-44s = %r, expected %r" % (label, got, want))
            bad = 1

    # ⭐ The streaming walk, on a document shaped like the real one -- including
    # a value big enough to cross the read boundary, which is the only part of
    # this that can go wrong quietly.
    filler = "x" * 3000
    doc = json.dumps({
        "packages": {
            "jq": {"name": "jq-1.8.2", "pname": "jq", "version": "1.8.2",
                   "system": "x86_64-linux", "outputName": "bin",
                   "outputs": {"bin": None, "out": None},
                   "meta": {"description": filler}},
            "bash": {"name": "bash-interactive-5.3p15", "pname": "bash-interactive",
                     "version": "5.3p15", "system": "x86_64-linux",
                     "outputName": "out", "outputs": {"out": None},
                     "meta": {"description": filler}},
        },
        "version": 2,
    })
    got = []
    stream_packages(io.StringIO(doc), lambda a, v: got.append((a, v.get("name"), v.get("outputName"))))
    check("streamed attribute count", len(got), 2)

    # ⛔ THE CONTROL THAT CAUGHT THE 31% LOSS, and it is the whole reason
    # `chunk` is a parameter. With a chunk far smaller than the document the
    # refill path runs on nearly every package, and the count must still equal
    # what `json.loads` sees. The first version of this reader dropped a whole
    # chunk on each refill and produced 103,571 of 149,812 attributes -- a
    # number that looks like an index rather than like a bug.
    big = json.dumps({"packages": {
        "p%04d" % k: {"name": "p%04d-1.0" % k, "system": "x86_64-linux",
                      "outputName": "out", "meta": {"d": "y" * 200}}
        for k in range(500)}, "version": 2})
    truth = len(json.loads(big)["packages"])
    for csize in (16, 64, 997, 65536):
        seen = []
        stream_packages(io.StringIO(big), lambda a, v: seen.append(a), chunk=csize)
        check("chunk=%-5d loses nothing against json.loads" % csize, len(seen), truth)
    check("bash resolves to its real name", dict((g[0], g[1]) for g in got).get("bash"),
          "bash-interactive-5.3p15")
    check("jq's default output", dict((g[0], g[2]) for g in got).get("jq"), "bin")

    # ⛔ A REFUSAL CASE. A hydra reply for another system must fail, not be
    # reported. This is the defect that made a Mach-O binary look like a result.
    import tempfile
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as t:
        json.dump({"drvpath": "/nix/store/a-b.drv", "system": "aarch64-darwin"}, t)
        name = t.name
    check("hydra refuses a reply for another system", cmd_hydra([name]), 1)
    check("hydra accepts the system it was asked for",
          cmd_hydra([name, "--system", "aarch64-darwin"]), 0)
    os.unlink(name)

    # A truncated document must raise, not return a short list quietly.
    try:
        stream_packages(io.StringIO('{"packages":{"a":{"name":"x"'), lambda a, v: None)
        check("truncated document refused", "returned", "raised")
    except ValueError:
        check("truncated document refused", "raised", "raised")

    print("nix-index selftest: %s" % ("all checks pass." if not bad else "FAILURES above."))
    return bad


def main(argv):
    if not argv:
        sys.stderr.write(__doc__)
        return 2
    cmd, rest = argv[0], argv[1:]
    if cmd == "index":
        return cmd_index(rest)
    if cmd == "hydra":
        return cmd_hydra(rest)
    if cmd in ("selftest", "--selftest"):
        return _selftest()
    sys.stderr.write("unknown command: %s\n" % cmd)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
