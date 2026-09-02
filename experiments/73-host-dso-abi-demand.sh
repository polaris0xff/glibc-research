#!/bin/sh
# 73-host-dso-abi-demand.sh
#
# -- THE QUESTION -----------------------------------------------------------
#
# docs/limitations.md §1 is the project's one measured, unfixed failure:
# dlopen() of a HOST shared object from a static glibc binary is
# host-dependent, and success is the worse outcome because the host's ld.so
# and libc.so.6 enter the process.
#
# experiments/50- tried to fix that by rewriting the host object. It did not
# work. experiments/72- then found something PRIOR to it: a static
# executable's .dynsym is empty, so even a perfect loader has nowhere to
# resolve a callback into the host program.
#
# ⭐ pg83/solo (references/pg83__solo, commit 79451211) answers both at once
# with a shape neither experiment tried: DO NOT USE THE HOST LOADER AT ALL.
# Compile an ELF loader INTO the binary, map the host object yourself, and
# resolve its imports against a table of the executable's own symbols. The
# empty .dynsym stops mattering because the loader's table is not .dynsym.
#
# ⛔ SOLO DOES THIS ON MUSL, and pays 5,948 lines of lib/glibc_shim.cpp to
# translate a guest's glibc imports onto a musl runtime. THIS PROJECT IS
# STATIC GLIBC, so the interesting question is whether that entire shim is
# unnecessary here — whether a host DSO's glibc imports can simply bind to
# the glibc already statically linked into our own binary.
#
# ⭐ THIS EXPERIMENT MEASURES THAT, AND ONLY THAT. It builds no loader. It
# answers the question that decides whether building one is worth it:
#
#     of everything real host shared objects import from glibc, how much
#     can a pgb binary's own statically linked glibc define?
#
# A loader is a large piece of work. Measuring the demand first is cheap, and
# if the answer were "half", the route would be closed before a line was
# written.
#
# -- WHAT IT DOES NOT ESTABLISH ---------------------------------------------
#
# ⛔ THIS IS A SYMBOL-AVAILABILITY MEASUREMENT, NOT A WORKING dlopen. It says
# the names can be bound. It does NOT say the bound code behaves: glibc's
# internal invariants (TLS layout, the stdio ABI, pthread object sizes, IFUNC
# resolution, RELRO) are not exercised by counting symbols, and solo's own
# README is explicit that "loading is the floor, not the claim".
#
# ⚠ It also cannot see symbols reached through dlsym at run time rather than
# through .dynsym, and it counts an object's demand once per object, not per
# call site.
#
# -- ARMS -------------------------------------------------------------------
#
#   control   Does an UNVERSIONED definition satisfy a VERSIONED reference?
#             The whole approach depends on it: `nm` over libc.a yields bare
#             names, and a host DSO asks for malloc@GLIBC_2.2.5. If ld.so
#             refused that pairing, matching by bare name would be inventing
#             semantics rather than reproducing them. Measured, not assumed.
#
#             ⛔ TWO CASES, AND THE FIRST VERSION OF THIS ARM MEASURED THE
#             WRONG ONE. Replacing the very library named in the reference's
#             DT_VERNEED entry with an unversioned build does NOT bind: glibc
#             prints "no version information available" and then ABORTS --
#             `dl-lookup.c:106: check_match: Assertion
#             'version->filename == NULL || ! _dl_name_match_p(...)' failed`.
#             glibc treats a named provider that lost its versions as a bug in
#             that library, not as a compatibility case.
#             ⭐ The case that matters here is the other one: the definition
#             comes from a DIFFERENT object than the one named in the verneed
#             entry -- which is exactly where a compiled-in provider table
#             sits. That one binds, and this arm measures both so the
#             difference is on the record rather than in somebody's memory.
#
#             ⚠ This also explains a result this project already has:
#             experiments/50- ported cross-libc-dlopen's cld_strip_versions()
#             and found no effect. Case 1 says what stripping versions off a
#             named provider does when it IS reached -- it makes the loader
#             assert. docs/limitations.md §1, TODO T-031.
#
#   demand    Every ELF shared object in every fetched rootfs, its undefined
#             .dynsym entries extracted by reading the file's bytes -- no
#             readelf, no nm inside the target -- and each one classified.
#
# -- THE CLASSIFICATION, AND WHY IT IS NOT A JUDGEMENT CALL -----------------
#
# Every symbol a pgb binary's own glibc cannot define is then asked two
# questions, both answered by the target environment itself:
#
#   A  the host's own ld.so exports it        -> LOADER-OWNED. Not a gap: a
#      compiled-in loader provides these itself, exactly as ld.so does.
#      __tls_get_addr, _rtld_global, _rtld_global_ro, __pointer_chk_guard.
#   B  the host's libc.so.6 has it at a GLIBC_ version NEWER than the pinned
#      build environment's        -> VERSION CEILING: the host object was
#      built against a newer glibc than ours and wants what ours has not got.
#   C  the host's libc.so.6 has it, the pinned environment's libc.so.6 does
#      NOT                        -> VERSION FLOOR: our glibc REMOVED it.
#   S  both libc.so.6 have it and our libc.a does not -> a symbol the SHARED
#      libc keeps and the STATIC one never had. Not a version question.
#   D  the host's libc.so.6 does not export it either -> the demand is met by
#      some other library there, or not at all.
#   E  anything left -> ⛔ AN UNEXPLAINED GAP, and the finding this arm exists
#      to catch. Expected empty; if it is not, that is the result.
#
# ⭐ B AND C TOGETHER ARE THE REAL RESULT, and they point in OPPOSITE
# directions. docs/AGENTS.md §14 already carries a FLOOR -- "do not build
# below glibc 2.34", because the NSS override needs it. B adds a CEILING and C
# adds a second floor of its own: build too old and a modern host object wants
# __isoc23_strtol; build too new and an older host object wants xdr_void,
# which glibc deleted in 2.32. ⛔ NO SINGLE PIN SATISFIES BOTH ENDS, so a
# loader that serves host objects on every environment needs a version-aware
# answer -- compat definitions, not a better choice of pin.
#
# Exit: 0 the measurement ran and matched, 1 it ran and did not, 2 could not run.
#
# SPDX-License-Identifier: MIT

set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "73 - what host shared objects import, and what a static glibc can define"

WORK="$EXP_OUT/build"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
RESULT="$EXP_OUT/RESULT.txt"

ENV_NAME="${PGB_ENV_NAME:-pgb-env-debian12}"
ENV_ROOT="$ROOTFS_DIR/$ENV_NAME"

command -v python3 >/dev/null 2>&1 || { exp_skip "python3" "absent"; exp_finish; }
command -v nm >/dev/null 2>&1      || { exp_skip "nm" "absent"; exp_finish; }

# ===========================================================================
# ARM 1 -- the control: does an unversioned definition satisfy a versioned
# reference?
#
# Built dynamically ON THE HOST, because it is a statement about the symbol
# resolution rule and not about static linking. Two shared objects with the
# same SONAME: one carries a version script, one does not. The executable is
# linked against the versioned one, so its reference is foo@PGBTEST_1.0. It
# is then RUN against the unversioned one.
# ===========================================================================
printf -- '-- control: unversioned definition vs versioned reference ----\n'

mkdir -p "$WORK/ctl/v" "$WORK/ctl/u"
cat > "$WORK/ctl/prov.c" <<'EOF'
int pgb_probe_symbol(void) { return 4242; }
EOF
# ⭐ A DIFFERENT value, so the run says WHICH object answered rather than only
# that something did. A shadow returning 4242 too would be indistinguishable
# from the versioned provider answering and would pass either way.
cat > "$WORK/ctl/shadow.c" <<'EOF'
int pgb_probe_symbol(void) { return 9999; }
EOF
cat > "$WORK/ctl/prov.map" <<'EOF'
PGBTEST_1.0 { global: pgb_probe_symbol; local: *; };
EOF
cat > "$WORK/ctl/main.c" <<'EOF'
#include <stdio.h>
int pgb_probe_symbol(void);
int main(void) { printf("%d\n", pgb_probe_symbol()); return 0; }
EOF

ctl_rule=could-not-run
ctl_named=could-not-run
if cc -shared -fPIC -o "$WORK/ctl/v/libpgbprobe.so.1" "$WORK/ctl/prov.c" \
      -Wl,--version-script="$WORK/ctl/prov.map" -Wl,-soname,libpgbprobe.so.1 2>/dev/null &&
   cc -shared -fPIC -o "$WORK/ctl/u/libpgbprobe.so.1" "$WORK/ctl/prov.c" \
      -Wl,-soname,libpgbprobe.so.1 2>/dev/null &&
   cc -shared -fPIC -o "$WORK/ctl/libpgbshadow.so" "$WORK/ctl/shadow.c" 2>/dev/null &&
   cc -o "$WORK/ctl/main" "$WORK/ctl/main.c" "$WORK/ctl/v/libpgbprobe.so.1" 2>/dev/null
then
  # ⛔ Prove the reference really is versioned before drawing any conclusion
  # from the run. A control that silently linked an unversioned reference
  # would "pass" while measuring nothing.
  ref=$(readelf -V "$WORK/ctl/main" 2>/dev/null | grep -c 'PGBTEST_1.0' || true)
  if [ "${ref:-0}" -gt 0 ]; then
    # Case 1: the provider named in DT_VERNEED, rebuilt without versions.
    if out=$(LD_LIBRARY_PATH="$WORK/ctl/u" "$WORK/ctl/main" 2>/dev/null) && [ "$out" = "4242" ]; then
      ctl_named=binds
    else
      ctl_named=$(LD_LIBRARY_PATH="$WORK/ctl/u" "$WORK/ctl/main" 2>&1 >/dev/null |
                    grep -qi 'assertion' && echo loader-asserts || echo refused)
    fi
    # Case 2: an unversioned definition in a DIFFERENT object, reached first
    # through the global scope. This is where a compiled-in provider sits.
    out=$(LD_LIBRARY_PATH="$WORK/ctl/v" LD_PRELOAD="$WORK/ctl/libpgbshadow.so" \
            "$WORK/ctl/main" 2>/dev/null)
    case "$out" in
      9999) ctl_rule=yes ;;
      4242) ctl_rule=no-versioned-definition-won ;;
      *)    ctl_rule=no ;;
    esac
  else
    ctl_rule=reference-not-versioned
  fi
fi
exp_check "unversioned definition in ANOTHER object satisfies foo@PGBTEST_1.0" "$ctl_rule" "yes"
exp_check "unversioned rebuild of the object NAMED in DT_VERNEED" "$ctl_named" "loader-asserts"
exp_note "the first is the rule that lets a table of bare names from \`nm\` answer"
exp_note "a host object's malloc@GLIBC_2.2.5. solo relies on it too:"
exp_note "references/pg83__solo/tree/lib/elf_loader.cpp:2070 (commit 79451211)"
exp_note "the second is why stripping versions off a named provider is not a fix"
exp_note "-- experiments/50-'s cld_strip_versions port, TODO T-031"
printf '\n'

# ===========================================================================
# ARM 2 -- the demand.
# ===========================================================================
printf -- '-- provider: what the pinned static glibc can define ----------\n'

[ -d "$ENV_ROOT" ] || { exp_skip "pinned build environment" "$ENV_ROOT absent; run: ./pgb env create"; exp_finish; }

LIBDIR=""
for d in "$ENV_ROOT/usr/lib/x86_64-linux-gnu" "$ENV_ROOT/usr/lib64" "$ENV_ROOT/usr/lib"; do
  [ -f "$d/libc.a" ] && { LIBDIR="$d"; break; }
done
[ -n "$LIBDIR" ] || { exp_skip "libc.a in the pinned environment" "not found under $ENV_ROOT"; exp_finish; }

# ⛔ THE DEFECT THIS LOOP EXISTS TO CATCH, and it produced a wrong number here
# before it was written: on Debian 12 `libm.a` IS NOT AN ARCHIVE. It is a GNU
# ld script reading `GROUP ( libm-2.36.a libmvec.a )`. `nm` on it yields
# NOTHING, silently and with exit 0, so the provider set came out missing all
# of libm -- and the run then reported sin, cos, tan, log2, pow and two dozen
# more as symbols a static glibc "cannot define". Every one of them was a
# measurement artefact. Resolve the script.
resolve_archive() {  # path -> echoes one or more real archive paths
  if [ "$(head -c 2 "$1" 2>/dev/null)" = "/*" ]; then
    tr -d '\n' < "$1" | sed 's/.*GROUP *( *//; s/ *).*//' | tr ' ' '\n' |
      while read -r m; do
        [ -n "$m" ] || continue
        printf '%s\n' "$ENV_ROOT$m"
      done
  else
    printf '%s\n' "$1"
  fi
}

: > "$WORK/archives.txt"
for a in libc.a libm.a libpthread.a librt.a libdl.a libutil.a libresolv.a \
         libanl.a libcrypt.a libc_nonshared.a; do
  [ -f "$LIBDIR/$a" ] || continue
  resolve_archive "$LIBDIR/$a" >> "$WORK/archives.txt"
done
# libgcc: the GCC_* half of the demand.
for g in "$ENV_ROOT"/usr/lib/gcc/x86_64-linux-gnu/*/libgcc.a \
         "$ENV_ROOT"/usr/lib/gcc/x86_64-linux-gnu/*/libgcc_eh.a; do
  [ -f "$g" ] && printf '%s\n' "$g" >> "$WORK/archives.txt"
done

# shellcheck disable=SC2046
nm --defined-only --extern-only $(tr '\n' ' ' < "$WORK/archives.txt") 2>/dev/null |
  awk '$2 ~ /^[TWDBRVSGiu]$/ {print $3}' | sort -u > "$WORK/provider.txt"

nprov=$(wc -l < "$WORK/provider.txt" | tr -d ' ')
exp_check "provider symbols extracted from the pinned glibc" "$([ "$nprov" -gt 3000 ] && echo many || echo "$nprov")" "many"
# The libm guard: if the linker script is ever mishandled again, this fails
# instead of quietly deflating every coverage figure below.
exp_check "provider set includes libm (linker-script guard)" \
          "$(grep -cx 'tan' "$WORK/provider.txt")" "1"
exp_note "provider archives: $(wc -l < "$WORK/archives.txt" | tr -d ' '), symbols: $nprov"
exp_note "pinned environment: $ENV_NAME"
printf '\n'

cat > "$WORK/demand.py" <<'PYEOF'
"""Read ELF bytes directly. An independent oracle: nothing in the target
rootfs is executed, and no binutils inside it is trusted or required."""
import os, sys, struct, collections, json

def sections(data):
    if len(data) < 64 or data[:4] != b'\x7fELF' or data[4] != 2:
        return None
    e_shoff, = struct.unpack_from('<Q', data, 0x28)
    e_shentsize, e_shnum, e_shstrndx = struct.unpack_from('<HHH', data, 0x3a)
    if e_shoff == 0 or e_shnum == 0 or e_shstrndx >= e_shnum:
        return None
    secs = []
    for i in range(e_shnum):
        off = e_shoff + i * e_shentsize
        if off + 64 > len(data):
            return None
        name, typ, flags, addr, offset, size, link, info, align, entsize = \
            struct.unpack_from('<IIQQQQIIQQ', data, off)
        secs.append(dict(name=name, offset=offset, size=size))
    shstr = secs[e_shstrndx]
    out = {}
    for s in secs:
        b = data[shstr['offset'] + s['name']:]
        nul = b.find(b'\0')
        if nul < 0:
            continue
        out[b[:nul].decode('utf-8', 'replace')] = s
    return out

def _strfn(data, sec):
    tab = data[sec['offset']:sec['offset'] + sec['size']]
    def f(o):
        e = tab.find(b'\0', o)
        return tab[o:e].decode('utf-8', 'replace') if e >= 0 else ''
    return f

def _verneed(data, secs, sstr):
    """version index -> version string, from .gnu.version_r (imports)."""
    m = {}
    gvr = secs.get('.gnu.version_r')
    if not gvr or not gvr['size']:
        return m
    p, end = gvr['offset'], gvr['offset'] + gvr['size']
    while p < end:
        vn_version, vn_cnt, vn_file, vn_aux, vn_next = struct.unpack_from('<HHIII', data, p)
        ap = p + vn_aux
        for _ in range(vn_cnt):
            vda_hash, vda_flags, vda_other, vda_name, vda_next = struct.unpack_from('<IHHII', data, ap)
            m[vda_other & 0x7fff] = sstr(vda_name)
            if vda_next == 0:
                break
            ap += vda_next
        if vn_next == 0:
            break
        p += vn_next
    return m

def _verdef(data, secs, sstr):
    """version index -> version string, from .gnu.version_d (exports)."""
    m = {}
    gvd = secs.get('.gnu.version_d')
    if not gvd or not gvd['size']:
        return m
    p, end = gvd['offset'], gvd['offset'] + gvd['size']
    while p < end:
        vd_version, vd_flags, vd_ndx, vd_cnt, vd_hash, vd_aux, vd_next = \
            struct.unpack_from('<HHHHIII', data, p)
        if vd_cnt:
            vda_name, vda_next = struct.unpack_from('<II', data, p + vd_aux)
            m[vd_ndx & 0x7fff] = sstr(vda_name)
        if vd_next == 0:
            break
        p += vd_next
    return m

def dynsyms(path):
    """-> (undefined [(name, version)], defined {name: version})"""
    try:
        with open(path, 'rb') as f:
            data = f.read()
    except OSError:
        return None
    secs = sections(data)
    if secs is None:
        return None
    dynsym, dynstr = secs.get('.dynsym'), secs.get('.dynstr')
    if not dynsym or not dynstr or not dynsym['size']:
        return ([], {})
    sstr = _strfn(data, dynstr)
    need, defs = _verneed(data, secs, sstr), _verdef(data, secs, sstr)
    vidx = None
    gv = secs.get('.gnu.version')
    if gv and gv['size']:
        n = min(gv['size'] // 2, (len(data) - gv['offset']) // 2)
        vidx = struct.unpack_from('<%dH' % n, data, gv['offset'])
    und, dfn = [], {}
    for i in range(dynsym['size'] // 24):
        o = dynsym['offset'] + i * 24
        if o + 24 > len(data):
            break
        st_name, st_info, st_other, st_shndx, st_value, st_size = \
            struct.unpack_from('<IBBHQQ', data, o)
        if st_name == 0:
            continue
        nm = sstr(st_name)
        if not nm:
            continue
        vi = (vidx[i] & 0x7fff) if (vidx and i < len(vidx)) else None
        if st_shndx == 0:
            und.append((nm, need.get(vi)))
        else:
            dfn[nm] = defs.get(vi) or dfn.get(nm)
    return (und, dfn)

def objects(root):
    out = set()
    for base in ('lib', 'lib64', 'usr/lib', 'usr/lib64', 'usr/libexec'):
        d = os.path.join(root, base)
        if not os.path.isdir(d):
            continue
        for dirpath, dirnames, filenames in os.walk(d):
            dirnames[:] = [x for x in dirnames
                           if not os.path.islink(os.path.join(dirpath, x))]
            for fn in filenames:
                if '.so' not in fn:
                    continue
                p = os.path.join(dirpath, fn)
                if not os.path.islink(p):
                    out.add(p)
    return sorted(out)

def find_one(root, names):
    for base in ('lib', 'lib64', 'usr/lib', 'usr/lib64'):
        d = os.path.join(root, base)
        if not os.path.isdir(d):
            continue
        for dirpath, dirnames, filenames in os.walk(d):
            for fn in filenames:
                if fn in names:
                    return os.path.join(dirpath, fn)
    return None

def vernum(v):
    if not v or not v.startswith('GLIBC_'):
        return None
    try:
        return tuple(int(x) for x in v[len('GLIBC_'):].split('.'))
    except ValueError:
        return None

def main():
    provider_file, pin, root, envroot = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
    provided = set(l.strip() for l in open(provider_file) if l.strip())
    pinv = tuple(int(x) for x in pin.split('.'))

    ldso = find_one(root, {'ld-linux-x86-64.so.2', 'ld-musl-x86_64.so.1',
                           'ld-linux.so.2', 'ld64.so.1'})
    libc = find_one(root, {'libc.so.6'})
    loader_exports = set()
    if ldso:
        r = dynsyms(ldso)
        if r:
            loader_exports = set(r[1])
    libc_exports = {}
    if libc:
        r = dynsyms(libc)
        if r:
            libc_exports = r[1]
    # The pinned environment's OWN shared libc: the discriminator between
    # "our glibc removed it" and "our glibc has it, but only in libc.so.6".
    envlibc = find_one(envroot, {'libc.so.6'})
    env_exports = {}
    if envlibc:
        r = dynsyms(envlibc)
        if r:
            env_exports = r[1]

    demand = collections.Counter()
    nobj = 0
    for p in objects(root):
        r = dynsyms(p)
        if r is None:
            continue
        nobj += 1
        for nm, v in r[0]:
            if v and (v.startswith('GLIBC_') or v.startswith('GCC_')):
                demand[nm] += 1

    served, cls = 0, collections.defaultdict(list)
    for nm, cnt in demand.items():
        if nm in provided:
            served += 1
            continue
        if nm in loader_exports:
            cls['A'].append((cnt, nm, ''))
            continue
        v = libc_exports.get(nm)
        n = vernum(v)
        if n is None:
            cls['D'].append((cnt, nm, v or '-'))
        elif n > pinv:
            cls['B'].append((cnt, nm, v))
        elif nm not in env_exports:
            cls['C'].append((cnt, nm, v))
        else:
            cls['S'].append((cnt, nm, v))
    for k in cls:
        cls[k].sort(reverse=True)
    print(json.dumps({
        'root': os.path.basename(root), 'objects': nobj,
        'demand': len(demand), 'served': served,
        'ldso': os.path.basename(ldso) if ldso else None,
        'libc': os.path.basename(libc) if libc else None,
        'envlibc': envlibc,
        'A': cls['A'], 'B': cls['B'], 'C': cls['C'],
        'S': cls['S'], 'D': cls['D'], 'E': cls['E'],
    }))

main()
PYEOF

# The pinned environment's glibc version, read from the archive name it ships.
PIN=$(ls "$LIBDIR"/libm-*.a 2>/dev/null | head -1 | sed 's/.*libm-//; s/\.a$//')
[ -n "$PIN" ] || PIN=2.36
exp_note "pinned glibc: $PIN (from $(basename "$(ls "$LIBDIR"/libm-*.a 2>/dev/null | head -1)" 2>/dev/null))"
printf '\n'

printf -- '-- demand: every shared object in every fetched environment ---\n'
printf '  %-20s %-6s %-7s %-7s %-6s  %s\n' ENVIRONMENT OBJS DEMAND SERVED PCT "UNSERVED BY CLASS"

: > "$WORK/rows.txt"
TOTAL_E=0
NROWS=0
NGLIBC=0
# ⛔ THE COLUMN ORDER IS image, name, libc, digest -- NOT name first. Reading
# it the other way round made every row look up a rootfs directory named
# `alpine:3.22`, so all eleven reported "rootfs absent" and the run skipped
# the entire measurement while still printing a table header.
while read -r image name libc rest; do
  case "$image" in ''|\#*) continue;; esac
  [ -n "$name" ] || continue
  r=$(exp_rootfs "$name")
  [ -n "$r" ] || { exp_skip "$name" "rootfs absent"; continue; }
  j="$WORK/$name.json"
  if ! python3 "$WORK/demand.py" "$WORK/provider.txt" "$PIN" "$r" "$ENV_ROOT" > "$j" 2>"$WORK/$name.err"; then
    exp_skip "$name" "demand scan failed: $(head -1 "$WORK/$name.err")"
    continue
  fi
  eval "$(python3 - "$j" <<'PYEOF'
import json, sys
d = json.load(open(sys.argv[1]))
# ⚠ A musl environment's objects import nothing GLIBC_-versioned, so the
# denominator is zero and a percentage would be a fabrication. Say so.
pct = ("%.1f%%" % (100.0 * d['served'] / d['demand'])) if d['demand'] else "n/a"
print("ROW_OBJ=%d; ROW_DEM=%d; ROW_SRV=%d; ROW_PCT=%s; ROW_A=%d; ROW_B=%d; ROW_C=%d; ROW_S=%d; ROW_D=%d; ROW_E=%d" % (
    d['objects'], d['demand'], d['served'], pct,
    len(d['A']), len(d['B']), len(d['C']), len(d['S']), len(d['D']), len(d['E'])))
PYEOF
)"
  printf '  %-20s %-6s %-7s %-7s %-6s  A=%s B=%s C=%s S=%s D=%s E=%s\n' \
    "$name" "$ROW_OBJ" "$ROW_DEM" "$ROW_SRV" "$ROW_PCT" \
    "$ROW_A" "$ROW_B" "$ROW_C" "$ROW_S" "$ROW_D" "$ROW_E"
  printf '%s %s %s %s %s %s %s %s %s %s %s\n' "$name" "$ROW_OBJ" "$ROW_DEM" "$ROW_SRV" \
    "$ROW_PCT" "$ROW_A" "$ROW_B" "$ROW_C" "$ROW_S" "$ROW_D" "$ROW_E" >> "$WORK/rows.txt"
  TOTAL_E=$((TOTAL_E + ROW_E))
  NROWS=$((NROWS + 1))
  [ "$ROW_DEM" -gt 0 ] && NGLIBC=$((NGLIBC + 1))
done < "$REPO_DIR/scripts/common/rootfs-images.txt"

printf '\n'
exp_note "A = the host ld.so exports it: a compiled-in loader owns these"
exp_note "B = the host libc.so.6 has it at a GLIBC_ version NEWER than $PIN"
exp_note "C = the host libc.so.6 has it and the pinned glibc REMOVED it"
exp_note "S = both shared libcs have it; the static libc.a never did"
exp_note "D = the host libc.so.6 does not define it either"
exp_note "E = none of the above -- UNEXPLAINED"
printf '\n'

exp_check "environments measured" "$([ "$NROWS" -ge 8 ] && echo enough || echo "$NROWS")" "enough"
# ⚠ A glibc-versioned demand of zero on the four musl rows is the correct
# answer, not a failure to look: a musl distribution's shared objects import
# from musl. Assert that the glibc rows were reached, or eleven skipped rows
# and four genuinely-empty ones would be indistinguishable.
exp_check "environments with glibc-versioned demand" \
          "$([ "$NGLIBC" -ge 7 ] && echo the-glibc-seven || echo "$NGLIBC")" "the-glibc-seven"
# ⛔ THE ASSERTION THAT MATTERS. Every symbol a pgb binary's own glibc cannot
# define must fall into a class with a named reason. A non-empty class E is a
# demand this project has not accounted for, and it is the finding.
#
# ⛔ AND IT MUST NOT PASS VACUOUSLY. The first run of this script read the
# rootfs table's columns in the wrong order, skipped all eleven rows, and then
# reported "unexplained gaps = 0" as a PASS -- zero symbols in class E because
# zero symbols had been looked at. docs/methodology/experiments.md: an absence
# is not a zero. A run that measured nothing SKIPS this, it does not pass it.
if [ "$NROWS" -eq 0 ]; then
  exp_skip "unexplained gaps (class E)" "no environment was scanned; 0 here would be an absence, not a zero"
else
  exp_check "unexplained gaps (class E), $NROWS environments" "$TOTAL_E" "0"
fi

printf '\n'
printf -- '-- the two version constraints, and they point opposite ways ---\n'
python3 - "$WORK" <<'PYEOF' | tee "$WORK/classes.txt"
import json, glob, os, sys, collections
work = sys.argv[1]
for key, title in (('B', 'CLASS B -- ceiling: the host glibc is NEWER than the pin'),
                   ('C', 'CLASS C -- floor: the pinned glibc REMOVED it'),
                   ('S', 'CLASS S -- in libc.so.6, never in libc.a'),
                   ('E', 'CLASS E -- unexplained')):
    agg, vers, where = collections.Counter(), {}, collections.defaultdict(set)
    for j in sorted(glob.glob(os.path.join(work, '*.json'))):
        d = json.load(open(j))
        for cnt, nm, v in d[key]:
            agg[nm] += cnt
            vers[nm] = v
            where[nm].add(d['root'])
    print("  %s" % title)
    if not agg:
        print("    empty on every environment.")
    else:
        print("    %-32s %-12s %-6s %s" % ("SYMBOL", "AT", "OBJS", "ENVIRONMENTS"))
        for nm, cnt in agg.most_common(10):
            print("    %-32s %-12s %-6d %d" % (nm, vers[nm] or '-', cnt, len(where[nm])))
        print("    ... %d distinct symbols" % len(agg))
    print("")
PYEOF

{
  printf 'experiment 73 - host DSO ABI demand against a static glibc\n\n'
  printf 'pinned build glibc : %s (%s)\n' "$PIN" "$ENV_NAME"
  printf 'provider symbols   : %s\n' "$nprov"
  printf 'version rule       : an unversioned definition in another object\n'
  printf '                     satisfies a versioned reference = %s\n' "$ctl_rule"
  printf '                     the object NAMED in DT_VERNEED, unversioned = %s\n\n' "$ctl_named"
  printf '%-20s %-6s %-7s %-7s %-7s %s\n' ENVIRONMENT OBJS DEMAND SERVED PCT 'A / B / C / S / D / E'
  while read -r n o d s p a b c ss dd e; do
    printf '%-20s %-6s %-7s %-7s %-7s %s / %s / %s / %s / %s / %s\n' \
      "$n" "$o" "$d" "$s" "$p" "$a" "$b" "$c" "$ss" "$dd" "$e"
  done < "$WORK/rows.txt"
  printf '\nA = host ld.so exports it (a compiled-in loader owns these)\n'
  printf 'B = host libc.so.6 has it at a GLIBC_ version newer than %s\n' "$PIN"
  printf 'C = host libc.so.6 has it and the pinned glibc removed it\n'
  printf 'S = both shared libcs have it; the static libc.a never did\n'
  printf 'D = host libc.so.6 does not define it either\n'
  printf 'E = none of the above -- unexplained\n\n'
  cat "$WORK/classes.txt"
} > "$RESULT"

exp_note "written: $RESULT"
exp_finish
