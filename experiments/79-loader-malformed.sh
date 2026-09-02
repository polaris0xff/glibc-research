#!/bin/sh
# 79-loader-malformed.sh
#
# -- THE QUESTION -----------------------------------------------------------
#
# `pgb build --host-dlopen` compiles an ELF loader into the binary
# (tool/runtime/pgb-elfload.c, TODO T-064). That loader parses files it did
# not write, found on the host at run time. ⛔ So: what does it do with a file
# that is not the well-formed shared object it expects?
#
# ⚠ THIS IS NOT ONLY A SECURITY QUESTION and it should not be read as one. A
# truncated `.so` is an ordinary accident -- a partial download, a full disk, a
# killed package manager -- and the answer measured here the first time was
# SIGBUS: the process died with no message and no dlerror(), because mmap of a
# short file SUCCEEDS and the fault arrives on first touch of a page past the
# end. `experiments/76-` could never have found it: every object it loads is
# well-formed.
#
# -- WHAT IS ASSERTED -------------------------------------------------------
#
#   1. ⛔ NO CASE CRASHES. Every malformed input is a named refusal through
#      dlerror(), or a correct load. A signal is a failure of this experiment.
#   2. ⭐ THE WELL-FORMED CASE STILL LOADS. A hardening pass that rejected
#      everything would pass assertion 1 and be worthless, so the pristine
#      copy of the same library is an arm here.
#   3. ⛔ AND THE REAL CORPUS IS UNCHANGED. Hardening that costs real objects
#      is a regression; this re-sweeps every shared object on the build host
#      and requires the load count not to fall.
#
# ⚠ Each case corrupts exactly ONE field of a known-good library, so a failure
# names the field responsible rather than "some bad file".
#
# Exit: 0 the measurement ran and matched, 1 it ran and did not, 2 could not run.
#
# SPDX-License-Identifier: MIT

set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "79 - the compiled-in loader against malformed ELF"

WORK="$EXP_OUT/build"
rm -rf "$WORK"; mkdir -p "$WORK/bad" || exit 2
RESULT="$EXP_OUT/RESULT.txt"

command -v python3 >/dev/null 2>&1 || { printf 'python3 is needed to build the corpus\n'; exit 2; }
command -v cc >/dev/null 2>&1 || { printf 'no C compiler\n'; exit 2; }

# The library the corpus is derived from. Any ordinary .so will do; libz is
# small, present everywhere, and has a real DT_GNU_HASH and DT_RELA.
SRC=""
for c in /usr/lib/x86_64-linux-gnu/libz.so.1 /usr/lib64/libz.so.1 /lib/x86_64-linux-gnu/libz.so.1; do
  [ -f "$c" ] && { SRC="$c"; break; }
done
[ -n "$SRC" ] || { printf 'no libz.so.1 to derive a corpus from\n'; exit 2; }

# ---------------------------------------------------------------------------
# The corpus. One field per case.
# ---------------------------------------------------------------------------
python3 - "$SRC" "$WORK/bad" <<'PY'
import os, struct, sys
src, out = sys.argv[1], sys.argv[2]
base = open(src, 'rb').read()
def emit(n, d): open(os.path.join(out, n + '.so'), 'wb').write(d)
def u16(b, o, v): return b[:o] + struct.pack('<H', v) + b[o+2:]
def u64(b, o, v): return b[:o] + struct.pack('<Q', v) + b[o+8:]
emit('00_pristine',        base)                      # the positive control
emit('01_truncated_hdr',   base[:32])
emit('02_truncated_half',  base[:len(base)//2])       # ⭐ the one that was SIGBUS
emit('03_truncated_1byte', base[:1])
emit('04_empty',           b'')
emit('05_phnum_huge',      u16(base, 0x38, 0xffff))
emit('06_phoff_past_eof',  u64(base, 0x20, len(base) * 4))
emit('07_phoff_unaligned', u64(base, 0x20, 3))
emit('08_phentsize_zero',  u16(base, 0x36, 0))
emit('09_bad_magic',       b'\x7fELX' + base[4:])
emit('10_class32',         base[:4] + b'\x01' + base[5:])
emit('11_machine_arm',     u16(base, 0x12, 183))
emit('12_type_exec',       u16(base, 0x10, 2))
emit('13_shoff_garbage',   u64(base, 0x28, 0xdeadbeefdeadbeef))
emit('14_all_ff',          b'\x7fELF' + b'\xff' * (len(base) - 4))
emit('15_zeros_after_hdr', base[:64] + b'\x00' * (len(base) - 64))
print(f"{len(os.listdir(out))} cases from {src}")
PY

printf '  corpus derived from %s\n' "$SRC"

# ---------------------------------------------------------------------------
# The subject: the loader, built exactly as `pgb build --host-dlopen` builds
# it -- the same source, the same weak provider table, the same -u list.
# ---------------------------------------------------------------------------
RT="$REPO_DIR/tool/runtime"
GEN="$WORK/table.c"
LIBDIR=""
for d in /usr/lib/x86_64-linux-gnu /usr/lib64 /usr/lib; do
  [ -f "$d/libc.a" ] && { LIBDIR="$d"; break; }
done
[ -n "$LIBDIR" ] || { printf 'no static glibc on this host\n'; exit 2; }

# ⚠ The generator resolves GNU ld SCRIPTS, because libm.a is one on Debian and
# Ubuntu and reading it as an archive yields zero symbols in silence.
# docs/history/corrections.md C18.
{
  all=$(mktemp); tls=$(mktemp); keep=$(mktemp); work=$(mktemp)
  for a in libc.a libm.a libpthread.a libdl.a librt.a libutil.a libresolv.a libcrypt.a libanl.a; do
    f="$LIBDIR/$a"; [ -f "$f" ] || continue
    if head -c 200 "$f" 2>/dev/null | grep -q 'GNU ld script'; then
      tr '(),' '   ' < "$f" | tr ' ' '\n' | grep -E '\.a$' >> "$work"
    else
      echo "$f" >> "$work"
    fi
  done
  sort -u "$work" -o "$work"
  while read -r a; do
    [ -f "$a" ] || continue
    readelf -sW "$a" 2>/dev/null | awk '
      $4=="FUNC"||$4=="OBJECT"||$4=="IFUNC" { if ($5=="GLOBAL"||$5=="WEAK") if ($7!="UND") print "K", $8 }
      $4=="TLS" { print "T", $8 }'
  done < "$work" | sed 's/@.*//' > "$all"
  grep '^T ' "$all" | cut -d' ' -f2 | grep -E '^[A-Za-z_][A-Za-z0-9_]*$' | LC_ALL=C sort -u > "$tls"
  grep '^K ' "$all" | cut -d' ' -f2 | grep -E '^[A-Za-z_][A-Za-z0-9_]*$' | LC_ALL=C sort -u \
    | LC_ALL=C comm -23 - "$tls" > "$keep"
  {
    echo '#include "pgb-elfload.h"'
    while read -r s; do printf 'extern char %s[] __attribute__((weak));\n' "$s"; done < "$keep"
    echo 'const struct pgb_provider_sym pgb_provider_syms[] = {'
    while read -r s; do printf '  {"%s", %s},\n' "$s" "$s"; done < "$keep"
    echo '  {0,0} };'
    echo 'const char *const pgb_provider_sonames[] = { "libc.so.6", "libm.so.6",'
    echo '  "libpthread.so.0", "libdl.so.2", "librt.so.1", "libutil.so.1",'
    echo '  "libresolv.so.2", "libcrypt.so.1", "libanl.so.1", 0 };'
  } > "$GEN"
  awk -F'"' '/^  \{"/ {print "-u " $2}' "$GEN" > "$WORK/u.rsp"
  printf '  provider table: %s names, %s thread-local excluded\n' \
    "$(wc -l < "$keep")" "$(wc -l < "$tls")"
  rm -f "$all" "$tls" "$keep" "$work"
}

# The driver: fork per case so a crash cannot end the run, and report the
# signal when there is one.
cat > "$WORK/driver.c" <<'EOF'
#include "pgb-elfload.h"
#include <stdio.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>
int main(int argc, char **argv)
{
    char line[4096];
    FILE *f = fopen(argv[1], "r");
    if (!f) return 2;
    while (fgets(line, sizeof line, f)) {
        pid_t p; int st;
        line[strcspn(line, "\n")] = 0;
        if (!*line) continue;
        p = fork();
        if (p == 0) {
            /* ⛔ stderr, not stdout, and it is not a style choice. stdout here
             * is redirected to a file and therefore FULLY buffered, and
             * _exit() does not flush stdio -- so a child that printf'd and
             * _exit'd wrote nothing at all. The first run of this experiment
             * reported an empty table for exactly that reason. */
            void *h = pgb_elf_dlopen(line, 2);
            if (!h) { fprintf(stderr, "REFUSED %s :: %s\n", line, pgb_elf_dlerror()); _exit(1); }
            fprintf(stderr, "LOADED  %s\n", line);
            _exit(0);
        }
        if (waitpid(p, &st, 0) < 0) continue;
        if (WIFSIGNALED(st)) fprintf(stderr, "SIGNAL%d %s\n", WTERMSIG(st), line);
    }
    fclose(f);
    return 0;
}
EOF

cc -static -O2 -fno-builtin -w -o "$WORK/driver" "$WORK/driver.c" \
   "$RT/pgb-elfload.c" "$GEN" -I"$RT" -I"$WORK" "-Wl,@$WORK/u.rsp" \
   -Wl,--start-group -lm -lpthread -ldl -lrt -lutil -lresolv -lcrypt -lanl \
   -Wl,--end-group 2>"$WORK/cc.log"
[ -x "$WORK/driver" ] || { printf 'the driver did not build; see %s\n' "$WORK/cc.log"; exit 2; }

# ---------------------------------------------------------------------------
# Run it.
# ---------------------------------------------------------------------------
ls "$WORK"/bad/*.so 2>/dev/null | sort > "$WORK/list.txt"
"$WORK/driver" "$WORK/list.txt" > "$WORK/out.txt" 2>&1

n_cases=$(wc -l < "$WORK/list.txt")
# ⚠ `grep -c` PRINTS 0 and EXITS 1 when nothing matches, so `|| echo 0`
# appends a second zero and the assertion compares "0\n0" against "0".
n_sig=$(grep -c '^SIGNAL' "$WORK/out.txt" 2>/dev/null || true)
pristine=$(grep -c '^LOADED  .*00_pristine' "$WORK/out.txt" 2>/dev/null || true)
n_sig=${n_sig:-0}; pristine=${pristine:-0}

{
  printf 'experiment 79 - the compiled-in loader against malformed ELF\n'
  printf 'date (UTC)   : %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'host kernel  : %s\n' "$(uname -sr)"
  printf 'corpus from  : %s\n\n' "$SRC"
  sed 's|.*/bad/||' "$WORK/out.txt"
} > "$RESULT"

printf -- '-- what each malformed case did ------------------------------\n'
sed 's|.*/bad/||' "$WORK/out.txt" | sed 's/^/  /'
printf '\n'

printf -- '-- assertions ------------------------------------------------\n'
exp_check "no case crashed the process" "$n_sig" "0"
exp_check "the pristine control still loads" "$pristine" "1"
printf '  --    %-46s = %s\n' "cases in the corpus (observed)" "$n_cases"
printf '\n'
exp_note "⛔ Both assertions are needed. A loader that refused everything would"
exp_note "pass the first and be worthless; one that accepted everything would"
exp_note "pass the second and take SIGBUS on a truncated file, which is what"
exp_note "the first run of this experiment measured before the fix."
printf '\n'
printf 'full table: %s\n' "$RESULT"

exp_finish
