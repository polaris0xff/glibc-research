#!/bin/sh
# THE QUESTION
#
#   glibc or musl -- which is actually FASTER at the work a program does after
#   it has started? And does a pgb binary keep glibc's answer on a musl host?
#
# -- WHY THIS EXISTS, AND WHAT IT IS CORRECTING ------------------------------
#
# ⛔ experiments/40- AND 60- MEASURED STARTUP AND SIZE AND CALLED IT
# PERFORMANCE. That is the wrong axis, and picking it produced a wrong
# headline: 60- reported that a static musl binary "ties pgb on coverage and
# beats it on startup and size", and concluded pgb was not better than the
# alternatives. Startup and size are the two axes on which musl wins BY
# CONSTRUCTION -- it is a smaller libc with a smaller startup path, and no
# amount of glibc engineering changes that.
#
# ⭐ THE PROJECT'S PREMISE IS THE OTHER AXIS. tmp/START.md asks for static
# binaries "using GLIBC rather than MUSL ... while avoiding the usual drawbacks
# and portability problems". Nobody reaches for glibc to start faster. They
# reach for it because of what it does once running: a malloc with per-thread
# arenas instead of a contended one, string and memory routines dispatched
# through IFUNC to the widest SIMD the CPU has, a faster qsort and printf.
# Those are what "performs better" means here, and until this script none of
# them had ever been measured in this repository.
#
# -- THE THREE COMPARISONS, EACH WITH ONE VARIABLE ---------------------------
#
# ⛔ A BENCHMARK WITH TWO VARIABLES MEASURES NEITHER. 60-'s musl arm was built
# by Alpine's gcc 14.2 and its glibc arms by the pinned Debian's gcc 12.2, so
# any difference was libc AND compiler. Each comparison here moves one thing:
#
#   A  libc gap        same host, SAME compiler, glibc -static vs musl -static
#   B  pgb's own cost  same pinned env, same compiler, plain -static vs pgb
#   C  does it travel  pgb vs static musl, run on all 11 environments
#
# ⭐ C IS THE ONE THAT MATTERS TO THE PROJECT. A on its own only repeats what
# is already known about the two libcs. The question this tool exists for is
# whether you can have glibc's numbers on a machine that ships musl -- which is
# exactly what an Alpine row of C answers.
#
# -- HOW THE WORKLOADS WERE CHOSEN -------------------------------------------
#
# Each is a libc facility a real program leans on, timed in nanoseconds per
# operation, and reported SEPARATELY so a reader can see where each libc wins
# rather than being handed one blended number:
#
#   malloc1   allocator, single thread, 14 size classes recycled
#   malloc4   the same on 4 threads at once -- the contention case
#   memcpy    8 B to 256 KiB, so it spans the dispatch-bound and the
#             bandwidth-bound ends and does not flatter either libc
#   strops    strlen + strchr + strstr, the IFUNC-dispatched routines
#   qsort     libc's own sort, 4096 ints
#   snprintf  formatting
#   math      pow/exp/log/sin
#
# ⚠ THE WORK IS SUNK INTO A `volatile` ACCUMULATOR on purpose. Without it -O2
# deletes most of these loops and the benchmark measures an empty for-loop.
#
# Exit: 0 measured, 1 an arm did not build or run, 2 could not measure.

. "$(dirname "$0")/lib.sh"

exp_begin "61 - libc throughput: what glibc buys once the program is running"

ROUNDS="${PGB_BENCH_ROUNDS:-3}"
SCALE="${PGB_BENCH_SCALE:-1}"
MATRIX_SCALE="${PGB_BENCH_MATRIX_SCALE:-1}"

ENV_ROOT="$ROOTFS_DIR/${PGB_ENV_NAME:-pgb-env-debian12}"
RR="$REPO_DIR/scripts/common/rootfs-run.sh"

B="$EXP_OUT/build"
rm -rf "$B"; mkdir -p "$B" || exit 2

cat > "$B/bench.c" <<'EOF'
/* Steady-state libc throughput. Each workload reports ns per operation.
 *
 * ⚠ `sink` is volatile and every workload feeds it. Without that, -O2 proves
 * the results unused and deletes the loops, and the benchmark reports the cost
 * of an empty for-loop as the cost of malloc.
 */
#define _GNU_SOURCE
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <math.h>

static volatile uint64_t sink;

static double now_ns(void) {
    struct timespec ts; clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec * 1e9 + (double)ts.tv_nsec;
}

/* ---- allocator ------------------------------------------------------
 * 14 size classes, recycled through a 64-slot ring so the allocator is
 * asked to reuse rather than only to grow. Both ends of each block are
 * touched, so a lazily mapped page is actually faulted in.
 */
static const size_t SZ[] = {16,32,48,64,96,128,192,256,384,512,1024,2048,4096,8192};
#define NSZ (sizeof SZ / sizeof SZ[0])

static void *malloc_loop(void *arg) {
    long n = *(long *)arg;
    void *keep[64] = {0};
    uint64_t acc = 0;
    for (long i = 0; i < n; i++) {
        size_t s = SZ[i % NSZ];
        int slot = (int)(i % 64);
        free(keep[slot]);
        keep[slot] = malloc(s);
        if (keep[slot]) { ((char *)keep[slot])[0] = (char)i; ((char*)keep[slot])[s-1] = (char)i;
                          acc += (uint64_t)((unsigned char *)keep[slot])[0]; }
    }
    for (int i = 0; i < 64; i++) free(keep[i]);
    sink += acc;
    return NULL;
}

static double bench_malloc(long n, int threads) {
    pthread_t t[16];
    long per = n / threads;
    double t0 = now_ns();
    if (threads == 1) { malloc_loop(&per); }
    else {
        for (int i = 0; i < threads; i++) pthread_create(&t[i], NULL, malloc_loop, &per);
        for (int i = 0; i < threads; i++) pthread_join(t[i], NULL);
    }
    double t1 = now_ns();
    return (t1 - t0) / (double)(per * threads);
}

/* ---- memcpy: 8 B to 256 KiB ----------------------------------------- */
static double bench_mem(long n) {
    size_t cap = 1 << 20;
    char *a = malloc(cap), *b = malloc(cap);
    memset(a, 1, cap);
    static const size_t len[] = {8,32,64,128,512,4096,32768,262144};
    double t0 = now_ns();
    for (long i = 0; i < n; i++) {
        size_t l = len[i % 8];
        memcpy(b, a, l);
        sink += (unsigned char)b[l - 1];
    }
    double t1 = now_ns();
    free(a); free(b);
    return (t1 - t0) / (double)n;
}

/* ---- strlen / strchr / strstr --------------------------------------- */
static double bench_str(long n) {
    size_t cap = 4096;
    char *s = malloc(cap);
    memset(s, 'a', cap - 1); s[cap - 1] = 0;
    s[cap - 200] = 'q'; s[cap - 199] = 'z';
    double t0 = now_ns();
    for (long i = 0; i < n; i++) {
        sink += strlen(s);
        sink += (uint64_t)(uintptr_t)strchr(s, 'q');
        sink += (uint64_t)(uintptr_t)strstr(s, "qz");
    }
    double t1 = now_ns();
    free(s);
    return (t1 - t0) / (double)n;
}

/* ---- qsort ---------------------------------------------------------- */
static int cmpint(const void *a, const void *b) {
    int x = *(const int *)a, y = *(const int *)b;
    return (x > y) - (x < y);
}
static double bench_qsort(long n) {
    int cnt = 4096;
    int *v = malloc((size_t)cnt * sizeof *v);
    double total = 0;
    long rounds = n / cnt; if (rounds < 1) rounds = 1;
    for (long r = 0; r < rounds; r++) {
        uint32_t x = 12345u + (uint32_t)r;
        for (int i = 0; i < cnt; i++) { x = x * 1103515245u + 12345u; v[i] = (int)(x >> 8); }
        double t0 = now_ns();
        qsort(v, (size_t)cnt, sizeof *v, cmpint);
        total += now_ns() - t0;
        sink += (uint64_t)v[0];
    }
    free(v);
    return total / (double)(rounds * cnt);
}

/* ---- snprintf ------------------------------------------------------- */
static double bench_snprintf(long n) {
    char buf[256];
    double t0 = now_ns();
    for (long i = 0; i < n; i++) {
        snprintf(buf, sizeof buf, "%d %s %.6f %x", (int)i, "portable", (double)i * 1.5, (unsigned)i);
        sink += (unsigned char)buf[0];
    }
    double t1 = now_ns();
    return (t1 - t0) / (double)n;
}

/* ---- math ----------------------------------------------------------- */
static double bench_math(long n) {
    double acc = 0;
    double t0 = now_ns();
    for (long i = 1; i <= n; i++) {
        double x = (double)i * 0.000001;
        acc += pow(x, 1.5) + exp(x) + log(x + 1.0) + sin(x);
    }
    double t1 = now_ns();
    sink += (uint64_t)acc;
    return (t1 - t0) / (double)n;
}

int main(int argc, char **argv) {
    const char *only = argc > 1 ? argv[1] : "all";
    long scale = argc > 2 ? atol(argv[2]) : 1;
    struct { const char *name; double (*fn)(long); long n; } tests[] = {
        { "memcpy",   bench_mem,      2000000L },
        { "strops",   bench_str,       200000L },
        { "qsort",    bench_qsort,    2000000L },
        { "snprintf", bench_snprintf, 1000000L },
        { "math",     bench_math,     2000000L },
    };
    if (!strcmp(only, "all") || !strcmp(only, "malloc1"))
        printf("malloc1 %.2f\n", bench_malloc(3000000L * scale, 1));
    if (!strcmp(only, "all") || !strcmp(only, "malloc4"))
        printf("malloc4 %.2f\n", bench_malloc(3000000L * scale, 4));
    for (size_t i = 0; i < sizeof tests / sizeof tests[0]; i++)
        if (!strcmp(only, "all") || !strcmp(only, tests[i].name))
            printf("%s %.2f\n", tests[i].name, tests[i].fn(tests[i].n * scale));
    if (sink == 0x1234567) fprintf(stderr, "impossible\n");
    return 0;
}
EOF

WORKLOADS="malloc1 malloc4 memcpy strops qsort snprintf math"

# best (lowest) ns/op across ROUNDS, per workload, from a binary run locally
run_local() {  # binary scale -> "name value" lines, best of ROUNDS
  _bin="$1"; _sc="$2"; _r=0
  : > "$B/acc.$$"
  while [ "$_r" -lt "$ROUNDS" ]; do
    "$_bin" all "$_sc" >> "$B/acc.$$" 2>/dev/null
    _r=$((_r+1))
  done
  awk '{ if (!(($1) in m) || $2 < m[$1]) m[$1] = $2 }
       END { for (k in m) printf "%s %s\n", k, m[k] }' "$B/acc.$$"
  rm -f "$B/acc.$$"
}

get() { # "name value" lines on stdin, name -> value
  awk -v k="$1" '$1 == k { print $2; exit }'
}

ratio() { # a b -> b/a to 2dp, or "-"
  awk -v a="$1" -v b="$2" 'BEGIN { if (a+0 > 0 && b+0 > 0) printf "%.2f", b/a; else printf "-" }'
}

# ---------------------------------------------------------------------------
# A -- the libc gap, same machine, same compiler
# ---------------------------------------------------------------------------
# ⭐ SAME COMPILER IS THE WHOLE POINT OF THIS ARM. Both binaries come from this
# host's cc; only the libc behind them differs. `musl-gcc` is a wrapper that
# hands the same gcc a musl sysroot, which is exactly the single-variable build
# this needs -- and is why arm A is built here rather than in the Alpine rootfs
# the way experiments/60- built its musl arm.
A_OK=no
if command -v musl-gcc >/dev/null 2>&1; then
  if ${CC:-cc} -O2 -static -o "$B/a-glibc" "$B/bench.c" -lm -lpthread >>"$B/build.log" 2>&1 \
     && musl-gcc -O2 -static -o "$B/a-musl" "$B/bench.c" -lm -lpthread >>"$B/build.log" 2>&1; then
    A_OK=yes
  else
    A_WHY="one of the two host builds failed; see build.log"
  fi
else
  A_WHY="musl-gcc absent (Debian/Ubuntu: musl-tools; Fedora: musl-gcc)"
fi

exp_check "A: both host arms built (same compiler)" "$A_OK" yes

if [ "$A_OK" = yes ]; then
  run_local "$B/a-glibc" "$SCALE" > "$B/a-glibc.txt"
  run_local "$B/a-musl"  "$SCALE" > "$B/a-musl.txt"
  printf '\n-- A: same machine, same compiler, libc is the only difference ----\n'
  printf '   cc: %s\n' "$({ ${CC:-cc} --version 2>/dev/null; } | head -1)"
  printf '   %s rounds, best round; ns per operation, lower is better\n\n' "$ROUNDS"
  printf '   %-10s %14s %14s   %s\n' WORKLOAD 'glibc static' 'musl static' 'musl / glibc'
  for w in $WORKLOADS; do
    g=$(get "$w" < "$B/a-glibc.txt"); m=$(get "$w" < "$B/a-musl.txt")
    printf '   %-10s %14s %14s   %sx\n' "$w" "${g:--}" "${m:--}" "$(ratio "$g" "$m")"
  done
  printf '\n'
  {
    printf '# arm A: host, same compiler\n'
    for w in $WORKLOADS; do
      printf 'A %s glibc=%s musl=%s\n' "$w" "$(get "$w" < "$B/a-glibc.txt")" "$(get "$w" < "$B/a-musl.txt")"
    done
  } > "$EXP_OUT/throughput.txt"
fi

# ---------------------------------------------------------------------------
# B -- what pgb costs on top of plain static glibc, on the SAME workload
# ---------------------------------------------------------------------------
# experiments/40- answered this for STARTUP and found no measurable difference.
# It never asked whether the NSS constructor and the static libiconv change
# steady-state throughput. They should not -- neither is on any path these
# workloads take -- but "should not" is the kind of sentence this repository
# exists to replace with a number.
B_OK=no
if [ -d "$ENV_ROOT" ]; then
  if sh "$RR" "$ENV_ROOT" --bind "$B:$B" --workdir "$B" -- /bin/sh -c \
       "gcc -O2 -static -o $B/b-static $B/bench.c -lm -lpthread" </dev/null >>"$B/build.log" 2>&1 \
     && ( cd "$B" && sh "$REPO_DIR/pgb" --bind "$B" build -- /bin/sh -c \
            "\$CC -O2 -o $B/b-pgb $B/bench.c -lm -lpthread" ) >>"$B/build.log" 2>&1; then
    B_OK=yes
  else
    B_WHY="a pinned-env build failed; see build.log"
  fi
else
  B_WHY="no build environment: sh pgb env create"
fi

exp_check "B: both pinned-env arms built" "$B_OK" yes

if [ "$B_OK" = yes ]; then
  run_local "$B/b-static" "$SCALE" > "$B/b-static.txt"
  run_local "$B/b-pgb"    "$SCALE" > "$B/b-pgb.txt"
  printf -- '-- B: plain gcc -static vs pgb, same pinned env, same compiler ----\n'
  printf '   %s rounds, best round; ns per operation, lower is better\n\n' "$ROUNDS"
  printf '   %-10s %14s %14s   %s\n' WORKLOAD 'plain static' 'pgb' 'pgb / plain'
  for w in $WORKLOADS; do
    s=$(get "$w" < "$B/b-static.txt"); p=$(get "$w" < "$B/b-pgb.txt")
    printf '   %-10s %14s %14s   %sx\n' "$w" "${s:--}" "${p:--}" "$(ratio "$s" "$p")"
  done
  printf '\n'
  {
    printf '# arm B: pinned env, plain static vs pgb\n'
    for w in $WORKLOADS; do
      printf 'B %s static=%s pgb=%s\n' "$w" "$(get "$w" < "$B/b-static.txt")" "$(get "$w" < "$B/b-pgb.txt")"
    done
  } >> "$EXP_OUT/throughput.txt"
fi

# ---------------------------------------------------------------------------
# C -- does the glibc advantage travel? pgb vs static musl, on all 11
# ---------------------------------------------------------------------------
# ⭐ THE ROWS THAT DECIDE WHETHER THIS PROJECT IS WORTH ANYTHING ARE THE MUSL
# ONES. On a glibc host, "use glibc" needs no tool. On Alpine, the ordinary
# choice is a musl build and musl's numbers; a pgb binary running there is the
# claim that you can have glibc's numbers on a machine that does not ship
# glibc. Every environment gets both binaries so the comparison is per-row.
C_MUSL=no
if [ -d "$ROOTFS_DIR/alpine-3.22" ]; then
  if sh "$RR" "$ROOTFS_DIR/alpine-3.22" --bind "$B:$B" --workdir "$B" -- /bin/sh -c \
       "apk add --no-cache gcc musl-dev >/dev/null 2>&1 && gcc -O2 -static -o $B/c-musl $B/bench.c -lm -lpthread" \
       </dev/null >>"$B/build.log" 2>&1 && [ -x "$B/c-musl" ]; then
    C_MUSL=yes
  fi
fi
exp_check "C: static musl arm built" "$C_MUSL" yes

if [ "$B_OK" = yes ] && [ "$C_MUSL" = yes ]; then
  printf -- '-- C: the same two binaries, run on every environment ------------\n'
  printf '   ns per operation, 1 round at matrix scale %s; lower is better\n' "$MATRIX_SCALE"
  printf '   ⚠ arm C is one round, not %s -- it is a per-environment check that\n' "$ROUNDS"
  printf '     the gap travels, not a re-measurement of its size.\n\n'
  printf '   %-19s %-6s %10s %10s %10s %10s\n' ENVIRONMENT LIBC 'P mal4' 'M mal4' 'P strops' 'M strops'
  ENVS=0
  while read -r ref name libc digest; do
    case "$ref" in ''|\#*) continue ;; esac
    root=$(exp_rootfs "$name")
    [ -n "$root" ] || { exp_skip "$name" "not fetched"; continue; }
    ENVS=$((ENVS+1))
    cp "$B/b-pgb" "$root/pgb-bench-p"; cp "$B/c-musl" "$root/pgb-bench-m"
    chmod +x "$root/pgb-bench-p" "$root/pgb-bench-m"
    sh "$RR" "$root" -- /pgb-bench-p all "$MATRIX_SCALE" </dev/null > "$B/c.$name.P" 2>/dev/null
    sh "$RR" "$root" -- /pgb-bench-m all "$MATRIX_SCALE" </dev/null > "$B/c.$name.M" 2>/dev/null
    pm=$(get malloc4 < "$B/c.$name.P"); mm=$(get malloc4 < "$B/c.$name.M")
    ps=$(get strops  < "$B/c.$name.P"); ms=$(get strops  < "$B/c.$name.M")
    printf '   %-19s %-6s %10s %10s %10s %10s\n' "$name" "$libc" \
      "${pm:--}" "${mm:--}" "${ps:--}" "${ms:--}"
    {
      printf 'C %s libc=%s\n' "$name" "$libc"
      for w in $WORKLOADS; do
        printf '  %-9s pgb=%-10s musl=%s\n' "$w" \
          "$(get "$w" < "$B/c.$name.P")" "$(get "$w" < "$B/c.$name.M")"
      done
    } >> "$EXP_OUT/per-environment.txt"
    rm -f "$root/pgb-bench-p" "$root/pgb-bench-m"
  done < "$REPO_DIR/scripts/common/rootfs-images.txt"
  printf '\n'
  exp_check "C: ran on every fetched environment" "$([ "$ENVS" -gt 0 ] && echo yes || echo no)" yes
  printf '   full per-workload figures per environment: %s\n\n' "$EXP_OUT/per-environment.txt"
fi

[ "$A_OK" = yes ] || exp_skip "arm A" "${A_WHY:-not built}"
[ "$B_OK" = yes ] || exp_skip "arm B" "${B_WHY:-not built}"

exp_note "⛔ READ THE RATIO COLUMN, NOT THE ABSOLUTE NUMBERS. The ns/op figures"
exp_note "  are one machine on one day and mean nothing elsewhere; the ratio"
exp_note "  between two binaries measured back to back on the same machine is"
exp_note "  the part that carries."
exp_note ""
exp_note "⚠ THIS DOES NOT MAKE musl A BAD LIBC. It is smaller and starts faster,"
exp_note "  measured in experiments/40- and 60-, and those are real advantages"
exp_note "  for short-lived processes. The two sets of measurements are about"
exp_note "  different things and neither cancels the other."
exp_note ""
exp_note "⚠ One machine, one day, 4 cores. See the conditions block above."
exp_finish
