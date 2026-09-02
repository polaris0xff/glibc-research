#!/usr/bin/env python3
"""elf-needed.py - read, and shorten, a DT_NEEDED that names an absolute path.

⛔ WHY THIS EXISTS, AND WHY IT IS NOT A NEW ELF LIBRARY. docs/AGENTS.md §14
says not to write an ELF analyser before checking `references/`, so:
`leleliu008/elftool` IS vendored there and it is a READER --
`print-needed.c`, `print-rpath.c`, `print-soname.c`, `print-interpreter.c`,
nothing that writes. `patchelf` would do the job and is not on this machine
and is not a dependency this project wants. What is needed here is one
narrow, safe edit, and it is about sixty lines.

-- THE PROBLEM, MEASURED ----------------------------------------------------

A nixpkgs closure bundled the Anylinux way runs its own loader with
`--library-path`, which resolves a DT_NEEDED by NAME. But nixpkgs links some
libraries with an absolute path, so the entry is not `libsqlite3.so` but
`/nix/store/fqkp26idpnpqk5l2cjfb51jdn6nj5bam-sqlite-3.53.3/lib/libsqlite3.so`.
An absolute DT_NEEDED is opened as a path and the search path is never
consulted, so the bundle fails with

    error while loading shared libraries:
    /nix/store/...-sqlite-3.53.3/lib/libsqlite3.so:
    cannot open shared object file: No such file or directory

on a machine that has that library sitting right beside the binary.

-- THE EDIT, AND WHY IT IS SAFE ---------------------------------------------

⭐ A BASENAME IS ALWAYS SHORTER THAN THE PATH IT CAME FROM, so it is written
OVER the original string at the same `.dynstr` offset with its own NUL. The
DT_NEEDED entry still points at that offset; the bytes after the NUL become
unreachable padding. ⛔ Nothing moves, no section grows, no offset in the file
changes -- which is what makes this safe to do to a file that other tools
have already laid out, and what separates it from a general ELF editor.

⚠ RPATH AND RUNPATH ARE LEFT ALONE, deliberately. A stale /nix/store entry in
them is searched, found missing, and skipped, so it costs a failed stat and
nothing else. Rewriting them would be a bigger edit for no observed problem.

Usage:
  elf-needed.py print   FILE...
  elf-needed.py shorten FILE...      # rewrites in place, reports what changed
  elf-needed.py selftest
"""

import os
import struct
import sys

DT_NEEDED, DT_STRTAB, DT_STRSZ, DT_NULL = 1, 5, 10, 0
PT_LOAD, PT_DYNAMIC = 1, 2


class NotElf(Exception):
    pass


def _read(path):
    with open(path, "rb") as fh:
        return bytearray(fh.read())


def parse(data):
    """Returns (dynamic entries, vaddr->offset mapper, strtab file offset)."""
    if data[:4] != b"\x7fELF":
        raise NotElf("not an ELF file")
    if data[4] != 2:
        # ⛔ REFUSED, NOT GUESSED. A 32-bit ELF has a different header layout
        # and every offset below would be wrong while still parsing.
        raise NotElf("only ELF64 is handled; this is ELF32")
    if data[5] != 1:
        raise NotElf("only little-endian is handled")
    e_phoff, = struct.unpack_from("<Q", data, 0x20)
    e_phentsize, e_phnum = struct.unpack_from("<HH", data, 0x36)

    loads, dyn_off, dyn_size = [], None, 0
    for i in range(e_phnum):
        off = e_phoff + i * e_phentsize
        p_type, = struct.unpack_from("<I", data, off)
        p_offset, p_vaddr = struct.unpack_from("<QQ", data, off + 8)
        p_filesz, = struct.unpack_from("<Q", data, off + 32)
        if p_type == PT_LOAD:
            loads.append((p_vaddr, p_offset, p_filesz))
        elif p_type == PT_DYNAMIC:
            dyn_off, dyn_size = p_offset, p_filesz
    if dyn_off is None:
        raise NotElf("no PT_DYNAMIC: this is a static binary or an object file")

    def v2o(vaddr):
        # ⚠ THE MAPPING IS PER-SEGMENT. Assuming a constant bias between
        # vaddr and file offset is right for most ELFs and wrong for exactly
        # the ones with more than one PT_LOAD, which is all of them since
        # RELRO.
        for v, o, sz in loads:
            if v <= vaddr < v + sz:
                return o + (vaddr - v)
        return None

    entries = []
    strtab_v = None
    pos = dyn_off
    while pos + 16 <= dyn_off + dyn_size:
        d_tag, d_val = struct.unpack_from("<Qq", data, pos)
        if d_tag == DT_NULL:
            break
        if d_tag == DT_STRTAB:
            strtab_v = d_val
        entries.append((pos, d_tag, d_val))
        pos += 16
    if strtab_v is None:
        raise NotElf("no DT_STRTAB")
    strtab_o = v2o(strtab_v)
    if strtab_o is None:
        raise NotElf("DT_STRTAB vaddr 0x%x is in no PT_LOAD" % strtab_v)
    return entries, strtab_o


def cstr(data, off):
    end = data.index(b"\0", off)
    return data[off:end].decode("utf-8", "replace")


def needed(path):
    data = _read(path)
    entries, strtab = parse(data)
    return [
        (strtab + d_val, cstr(data, strtab + d_val))
        for _, d_tag, d_val in entries
        if d_tag == DT_NEEDED
    ]


def shorten(path):
    """Returns the list of (old, new) it rewrote."""
    data = _read(path)
    entries, strtab = parse(data)
    changed = []
    for _, d_tag, d_val in entries:
        if d_tag != DT_NEEDED:
            continue
        off = strtab + d_val
        s = cstr(data, off)
        if not s.startswith("/"):
            continue
        base = os.path.basename(s)
        if not base or len(base) >= len(s):
            continue
        b = base.encode()
        data[off:off + len(b)] = b
        data[off + len(b)] = 0
        changed.append((s, base))
    if changed:
        # ⚠ WRITTEN THROUGH A TEMPORARY AND RENAMED, so an interrupted run
        # leaves the original rather than a half-written ELF that every later
        # step then reports as corrupt.
        tmp = path + ".pgbtmp"
        with open(tmp, "wb") as fh:
            fh.write(data)
        os.chmod(tmp, os.stat(path).st_mode & 0o7777)
        os.replace(tmp, path)
    return changed


def selftest():
    import shutil
    import subprocess
    import tempfile

    fails = []

    def check(name, cond, detail=""):
        print(("  ok    " if cond else "  FAIL  ") + name + ((" " + detail) if detail else ""))
        if not cond:
            fails.append(name)

    tmp = tempfile.mkdtemp()
    try:
        # ⭐ THE FIXTURE IS BUILT BY THE REAL TOOLCHAIN, so the thing under
        # test faces a real ELF rather than one this file wrote.
        lib = os.path.join(tmp, "libpgbtest.so")
        src = os.path.join(tmp, "l.c")
        with open(src, "w") as fh:
            fh.write("int pgb_test(void){return 42;}\n")
        r = subprocess.run(["cc", "-shared", "-fPIC", "-o", lib, src],
                           capture_output=True)
        if r.returncode != 0:
            print("  skip  no working cc to build a fixture")
            return 0
        prog = os.path.join(tmp, "prog")
        psrc = os.path.join(tmp, "p.c")
        with open(psrc, "w") as fh:
            fh.write("int pgb_test(void); int main(void){return pgb_test()-42;}\n")
        # ⛔ LINKED BY ABSOLUTE PATH ON PURPOSE: that is what makes the linker
        # write an absolute DT_NEEDED, which is the case being fixed.
        r = subprocess.run(["cc", "-o", prog, psrc, lib], capture_output=True)
        if r.returncode != 0:
            print("  skip  could not link the fixture: " + r.stderr.decode()[:120])
            return 0

        before = [n for _, n in needed(prog)]
        check("the fixture really has an absolute DT_NEEDED",
              any(n.startswith("/") for n in before), str(before))

        # It must not run once the library is moved out of the way, because
        # the absolute path is what it is looking for.
        os.rename(lib, os.path.join(tmp, "moved.so"))
        rc_before = subprocess.run([prog], capture_output=True,
                                   env={"LD_LIBRARY_PATH": tmp}).returncode
        check("before the edit it cannot start", rc_before != 0, "rc=%d" % rc_before)

        os.rename(os.path.join(tmp, "moved.so"), lib)
        changed = shorten(prog)
        check("shorten rewrote one entry", len(changed) == 1, str(changed))
        after = [n for _, n in needed(prog)]
        check("no absolute DT_NEEDED is left", not any(n.startswith("/") for n in after),
              str(after))
        check("the basename survived", "libpgbtest.so" in after, str(after))

        # ⭐ AND THE POINT: it now resolves by NAME out of a search path, which
        # is exactly what the bundled loader's --library-path provides.
        elsewhere = os.path.join(tmp, "libs")
        os.makedirs(elsewhere, exist_ok=True)
        shutil.move(lib, os.path.join(elsewhere, "libpgbtest.so"))
        rc = subprocess.run([prog], capture_output=True,
                            env={"LD_LIBRARY_PATH": elsewhere}).returncode
        check("after the edit it runs from a search path", rc == 0, "rc=%d" % rc)

        # A file with no absolute entries must be left byte-identical.
        digest_before = open(prog, "rb").read()
        again = shorten(prog)
        check("a second pass changes nothing", not again and
              open(prog, "rb").read() == digest_before)

        # A static binary has no PT_DYNAMIC and must be refused, not corrupted.
        st = os.path.join(tmp, "static")
        r = subprocess.run(["cc", "-static", "-o", st, psrc, lib], capture_output=True)
        if r.returncode == 0:
            refused = False
            try:
                shorten(st)
            except NotElf:
                refused = True
            check("a static binary is refused", refused)
        else:
            print("  skip  static link unavailable for the refusal case")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    print("elf-needed --selftest: %s" %
          ("all checks pass." if not fails else "%d FAILED." % len(fails)))
    return 1 if fails else 0


def main(argv):
    if len(argv) < 2:
        print(__doc__)
        return 2
    cmd = argv[1]
    if cmd == "selftest":
        return selftest()
    rc = 0
    for path in argv[2:]:
        try:
            if cmd == "print":
                for _, n in needed(path):
                    print("%s\t%s" % (path, n))
            elif cmd == "shorten":
                for old, new in shorten(path):
                    print("%s\t%s -> %s" % (path, old, new))
            else:
                print("elf-needed: unknown subcommand: " + cmd, file=sys.stderr)
                return 2
        except NotElf:
            # ⚠ NOT AN ERROR WHEN SWEEPING A TREE. A bundle directory holds
            # static binaries, scripts and data beside the shared objects, and
            # a run over all of them must not fail on the ones with no dynamic
            # section.
            pass
        except Exception as e:
            print("elf-needed: %s: %s" % (path, e), file=sys.stderr)
            rc = 1
    return rc


if __name__ == "__main__":
    sys.exit(main(sys.argv))
