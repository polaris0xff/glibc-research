#!/bin/sh
# THE QUESTION
#
#   The operator's claim, and it is falsifiable: *"our static glibc binary and
#   a native musl static binary are at feature/standalone parity. No buts and
#   no ifs."* Is it true, axis by axis, on all eleven environments?
#
# ⛔ WHY THIS EXPERIMENT EXISTS. The evidence for that claim is spread across
# `experiments/20-`, `30-`, `40-`, `60-`, `61-`, `71-`, `75-`, `76-`, `82-`,
# `97-` and eleven POC result files, and ⛔ **the musl column is INFERRED in
# most of it rather than run**. T-078 asks for one matrix in which every cell
# is a measurement or a dash.
#
# -- THE THREE ARMS, AND WHY THEY ARE BUILT THE WAY THEY ARE -----------------
#
#   V  vanilla `cc -static`   ⭐ BUILT IN THE PINNED ENVIRONMENT, not on the
#                             host. This is the single-variable design
#                             `experiments/61-` insists on: V and P then
#                             differ ONLY by the mechanisms pgb injects —
#                             same gcc, same glibc, same sysroot. A
#                             host-built vanilla arm would differ by
#                             compiler AND glibc AND mechanisms at once, and
#                             a three-variable comparison measures none of
#                             them.
#   H  vanilla, HOST cc       ⚠ THE CONTROL FOR THAT CHOICE, not a column.
#                             It exists to answer "does building V in the
#                             pinned env change any capability answer?" If H
#                             and V agree the table is robust to the choice;
#                             if they disagree that is a finding and the
#                             table has to say which compiler it describes.
#   P  `pgb build`            all three default mechanisms, plus the opt-ins
#                             the axes below actually test.
#   M  `musl-gcc -static`     ⭐ THE COLUMN THE OPERATOR NAMED, and it is RUN
#                             here rather than inferred.
#
# ⛔ A SKIP IS NOT A DASH AND IT IS NOT A PASS. Every arm that cannot be built
# is reported with `exp_skip` and named in the summary, because a missing
# toolchain otherwise produces a green run with an empty column — which is
# exactly what `60-` and `61-` do and what T-078's Prove line warns about.
#
# -- ⭐ WHY EACH AXIS RUNS IN ITS OWN FORK -----------------------------------
#
# ⛔ MEASURED, NOT ANTICIPATED: in `experiments/82-` a vanilla static binary
# died with SIGFPE inside `gethostid()` on Arch, and because the probe was one
# process every axis after it was lost. Here each axis runs in a forked child,
# so one axis crashing costs that axis and nothing else, and the parent
# reports WHICH signal took it. A crash is a finding, not an absence.
#
# -- THE AXES ----------------------------------------------------------------
#
#   runs        the probe's own exit status, per environment
#   payload     host shared objects opened, by pid-attributed trace
#   nss         getpwuid(0) — does the name service answer
#   hostid      gethostid() — the NSS reach `82-` found, and the SIGFPE canary
#   services    getservbyname("http","tcp") — the eleventh row, `82-`
#   iconv       how many of 12 encodings iconv_open accepts
#   locale      setlocale(LC_ALL,"") + nl_langinfo(CODESET)
#   timezone    TZ=Europe/Berlin -> %Z %z
#   interp      PT_INTERP / DT_NEEDED — a static property of the artefact
#   size        bytes
#
# ⚠ FIVE AXES T-078 NAMES ARE NOT HERE, AND EACH IS A DASH WITH A REASON
# rather than a silent omission — see the closing notes: throughput (`61-`),
# startup and peak RSS (`40-`, and its own instrument says the difference is
# under its noise floor), dlopen of own plugins (`71-`), dlopen of host
# objects (`76-`), terminfo and the CA bundle (`75-`, `74-`).
#
# -- ⭐ PRE-REGISTERED EXPECTATION -------------------------------------------
#
# ⛔ WRITTEN AND COMMITTED BEFORE THE RUN. PROGRESS.md delivery rule 1.
#
#   Q1  V crashes on at least one environment (Arch, SIGFPE, via gethostid);
#       P and M crash on none.
#   Q2  services: V fails 3 of 11, M fails 3 of 11 — ⭐ THE SAME THREE, because
#       the file is absent from the HOST and neither libc carries one. ⛔ I
#       expect P to fail them too: pgb has no mechanism for this row. THIS IS
#       THE ROW THAT COMES OUT AGAINST US and it is the deliverable, not a
#       thing to soften.
#   Q3  locale: M reports a non-UTF-8 codeset on ALL 11 (musl has no locale
#       support); V reports non-UTF-8 on the 4 musl hosts; P reports UTF-8 on
#       11 of 11 with --embed-locale.
#   Q4  iconv: P accepts 12 of 12 everywhere. V accepts far fewer where the
#       host gconv tree does not match, and M accepts a middling fixed set
#       (musl's builtin conversions) uniformly.
#   Q5  timezone: P 11 of 11 with --embed-tzdata; V and M both fail where the
#       host has no zoneinfo — and `82-`'s presence table says that is 3 of
#       11 for /usr/share/zoneinfo.
#   Q6  payload: P loads zero host shared objects on 11 of 11. M likewise. V
#       does not.
#   Q7  ⭐ PARITY VERDICT: I expect P to be AHEAD of M on locale, iconv and
#       timezone, LEVEL on payload and runs, and LEVEL-AND-BOTH-FAILING on
#       services. I do not expect any axis where M beats P.
#
# -- ⛔ WHAT ACTUALLY HAPPENED, and two of the seven were wrong ---------------
#
# ⭐ THE PREDICTIONS ABOVE ARE LEFT EXACTLY AS THEY WERE WRITTEN. This block
# is added beside them, because a pre-registration that gets edited after the
# run is not one.
#
#   Q1  ⚠ SCORED AGAINST THE WRONG NUMBER, by this script's own defect. The
#       per-axis fork is what makes a crash survivable, so the PARENT always
#       exits 0 — and the counter was reading the parent's status. It
#       reported "crashed = 0" for the vanilla arm on a run whose rows read
#       `nss=SIG8`, `hostid=SIG8` and `iconv=SIG6`. Counting dead AXES
#       instead: vanilla 5 of 11, pgb 0, musl 0. ⭐ The prediction was right;
#       the instrument was not.
#   Q3  ⛔ FALSIFIED ON BOTH HALVES, and this is the one row where NATIVE
#       MUSL BEATS BOTH GLIBC COLUMNS. With no LANG set, musl answers UTF-8
#       on 11 of 11 and every glibc arm — pgb included — answers
#       ANSI_X3.4-1968 on 11 of 11. musl's minimal locale support does not
#       mean a poor codeset: its default charset IS UTF-8. Asked for
#       C.UTF-8 BY NAME, pgb answers UTF-8 on 11 of 11 and vanilla on 7.
#   Q7  ⛔ THEREFORE WRONG AS STATED: there IS an axis where M beats P, and
#       it is the environment-default codeset. The parity claim holds on
#       every other axis measured here and not on that one.
#
# ⚠ A THIRD DEFECT, and it is why `TWO RUNS OR IT IS NOT A NUMBER` is a rule:
# the UTF-8 counter globbed the whole output line, so adding the `locreq`
# axis put a second `UTF-8` on it and the glibc arms went 0 -> 7 and pgb
# 0 -> 11 between two runs with no change to the subject. Tokens are now
# extracted by name. The zeros were the true reading.
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "63 - the three-way parity matrix: vanilla gcc -static, pgb, native musl static"

WORK="${PGB_EXP63_WORK:-/var/tmp/pgb-exp63}"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
CC="${CC:-cc}"
command -v "$CC" >/dev/null 2>&1 || { exp_note "no $CC on PATH"; exit 2; }
[ -d "$ENV_ROOT" ] || { exp_note "pinned env $ENV_ROOT absent; run pgb bootstrap"; exit 2; }

# ---------------------------------------------------------------------------
# The probe. One source, four builds.
# ---------------------------------------------------------------------------
cat > "$WORK/probe.c" <<'EOF'
/* One probe, one axis per forked child.

   ⛔ EACH AXIS FORKS. A vanilla static binary takes SIGFPE inside gethostid()
   on Arch (experiments/82-), and in a single-process probe that loses every
   axis after it. Forking costs one page and buys a complete row.

   ⛔ UNBUFFERED, for the same reason 82- is: stdout to a pipe is block
   buffered, so a crash discards answers already computed. */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <netdb.h>
#include <pwd.h>
#include <locale.h>
#include <langinfo.h>
#include <iconv.h>
#include <time.h>
#include <sys/wait.h>

static const char *ENC[] = {
  "UTF-8", "ISO-8859-1", "ISO-8859-15", "UTF-16LE", "UTF-16BE", "UTF-32LE",
  "CP1252", "KOI8-R", "EUC-JP", "SHIFT_JIS", "GB18030", "BIG5"
};
#define NENC ((int)(sizeof ENC / sizeof ENC[0]))

/* Run one axis in a child so a signal cannot take the rest of the row. */
static void axis(const char *name, void (*fn)(void)) {
  fflush(NULL);
  pid_t p = fork();
  if (p < 0) { printf(" %s=FORKFAIL", name); return; }
  if (p == 0) { fn(); fflush(NULL); _exit(0); }
  int st = 0;
  waitpid(p, &st, 0);
  if (WIFSIGNALED(st)) printf(" %s=SIG%d", name, WTERMSIG(st));
  else if (WEXITSTATUS(st) != 0) printf(" %s=EXIT%d", name, WEXITSTATUS(st));
}

static void a_nss(void) {
  struct passwd *pw = getpwuid(0);
  printf(" nss=%s", pw && pw->pw_name ? pw->pw_name : "NULL");
}
static void a_hostid(void)  { printf(" hostid=%ld", gethostid()); }
static void a_services(void) {
  struct servent *s = getservbyname("http", "tcp");
  printf(" services=%s", s ? "80" : "NULL");
}
static void a_iconv(void) {
  int ok = 0;
  for (int i = 0; i < NENC; i++) {
    iconv_t h = iconv_open(ENC[i], "UTF-8");
    if (h != (iconv_t)-1) { ok++; iconv_close(h); }
  }
  printf(" iconv=%d/%d", ok, NENC);
}
/* ⭐ TWO LOCALE AXES, AND THE FIRST ONE IS THE ONE WE LOSE.

   `locale`  asks for the ENVIRONMENT's locale — setlocale(LC_ALL, "") with no
             LANG set, which is what a program run from a bare container gets.
   `locreq`  asks for C.UTF-8 BY NAME, which is what exercises the
             --embed-locale mechanism.

   ⛔ THE FIRST IS KEPT EVEN THOUGH IT IS A LOSS. The pre-registered Q3 said
   musl would report a non-UTF-8 codeset and pgb would report UTF-8; the
   opposite happened on both halves. T-078's Prove line: a row that comes out
   against us IS the deliverable, and the axis must not be softened until it
   passes. So the losing row stays and the requesting row is ADDED beside it. */
static void a_locale(void) {
  setlocale(LC_ALL, "");
  const char *cs = nl_langinfo(CODESET);
  printf(" locale=%s", (cs && *cs) ? cs : "EMPTY");
}
static void a_locale_req(void) {
  const char *got = setlocale(LC_ALL, "C.UTF-8");
  const char *cs = got ? nl_langinfo(CODESET) : NULL;
  printf(" locreq=%s", (cs && *cs) ? cs : (got ? "EMPTY" : "REFUSED"));
}
static void a_tz(void) {
  setenv("TZ", "Europe/Berlin", 1);
  tzset();
  time_t t = 1720000000;            /* 2024-07-03, inside CEST */
  struct tm tm;
  localtime_r(&t, &tm);
  char buf[64];
  strftime(buf, sizeof buf, "%Z%z", &tm);
  printf(" tz=%s", buf);
}

int main(void) {
  setvbuf(stdout, NULL, _IONBF, 0);
  printf("probe");
  axis("nss",      a_nss);
  axis("hostid",   a_hostid);
  axis("services", a_services);
  axis("iconv",    a_iconv);
  axis("locale",   a_locale);
  axis("locreq",   a_locale_req);
  axis("tz",       a_tz);
  printf("\n");
  return 0;
}
EOF

BUILD_LOG="$WORK/build.log"; : > "$BUILD_LOG"

# --- arm H: vanilla, HOST cc (the control for the build-location choice) ----
H_OK=no
if "$CC" -static -O2 -o "$WORK/probe-H" "$WORK/probe.c" >>"$BUILD_LOG" 2>&1; then
  H_OK=yes
else
  exp_skip "arm H (vanilla, host cc)" "static link failed; see $BUILD_LOG"
fi

# --- arm V: vanilla, inside the PINNED environment --------------------------
#
# ⛔ `pgb rootfs run` ON THE ENVIRONMENT ROOT, NOT `pgb build`. `pgb build`
# puts the wrappers on PATH and injects the mechanisms — that is arm P. This
# arm needs the SAME environment with NONE of them, which is plain `cc` inside
# the same tree.
V_OK=no
if "$REPO_DIR/pgb" rootfs run "$ENV_ROOT" --bind "$WORK:/w" -- \
     /bin/sh -c 'cc -static -O2 -o /w/probe-V /w/probe.c' >>"$BUILD_LOG" 2>&1 \
   && [ -x "$WORK/probe-V" ]; then
  V_OK=yes
else
  exp_skip "arm V (vanilla, pinned env)" "cc -static in $ENV_ROOT failed"
fi

# --- arm P: pgb, with the opt-ins the axes test -----------------------------
P_OK=no
if "$REPO_DIR/pgb" --bind "$WORK" build --embed-locale --embed-tzdata -- \
     /bin/sh -c "\$CC -O2 -o '$WORK/probe-P' '$WORK/probe.c'" >>"$BUILD_LOG" 2>&1 \
   && [ -x "$WORK/probe-P" ]; then
  P_OK=yes
else
  exp_skip "arm P (pgb)" "pgb build failed; see $BUILD_LOG"
fi

# --- arm M: native musl static ----------------------------------------------
#
# ⛔ THE COLUMN THE OPERATOR NAMED. If musl-gcc is absent this SKIPS and the
# summary says so — it does not quietly produce a table with an empty column.
M_OK=no
if command -v musl-gcc >/dev/null 2>&1; then
  if musl-gcc -static -O2 -o "$WORK/probe-M" "$WORK/probe.c" >>"$BUILD_LOG" 2>&1; then
    M_OK=yes
  else
    exp_skip "arm M (native musl static)" "musl-gcc link failed; see $BUILD_LOG"
  fi
else
  exp_skip "arm M (native musl static)" \
    "musl-gcc absent (Debian/Ubuntu: apt-get install musl-tools)"
fi

exp_check "arm H built (control)"          "$H_OK" yes
exp_check "arm V built (vanilla, pinned)"  "$V_OK" yes
exp_check "arm P built (pgb)"              "$P_OK" yes
exp_check "arm M built (native musl)"      "$M_OK" yes

printf '\n'
printf -- '-- the artefacts themselves ----------------------------------------\n'
printf '  %-28s %12s  %-10s %s\n' ARM 'SIZE (B)' PT_INTERP DT_NEEDED
for a in H V P M; do
  f="$WORK/probe-$a"
  [ -x "$f" ] || { printf '  %-28s %12s  %-10s %s\n' "$a" - - -; continue; }
  interp=$(readelf -l "$f" 2>/dev/null | grep -c INTERP || true)
  needed=$(readelf -d "$f" 2>/dev/null | grep -c NEEDED || true)
  case "$a" in
    H) lbl="H vanilla (host cc)" ;;
    V) lbl="V vanilla (pinned env)" ;;
    P) lbl="P pgb" ;;
    M) lbl="M native musl static" ;;
  esac
  printf '  %-28s %12s  %-10s %s\n' "$lbl" "$(wc -c < "$f")" \
    "$([ "$interp" = 0 ] && echo none || echo "$interp")" \
    "$([ "$needed" = 0 ] && echo none || echo "$needed")"
done

# ---------------------------------------------------------------------------
# The matrix: every built arm on every fetched environment.
# ---------------------------------------------------------------------------
ENVS=$(awk '!/^#/ && NF {print $2}' "$REPO_DIR/scripts/common/rootfs-images.txt")

# Per-arm tallies, so the summary is derived and not retyped.
for a in H V P M; do
  eval "CRASH_$a=0; UTF8_$a=0; LREQ_$a=0; TZOK_$a=0; SVC_$a=0; NSSOK_$a=0; ICONV_$a=0; ROWS_$a=0"
done

printf '\n'
printf -- '-- the matrix ------------------------------------------------------\n'
for name in $ENVS; do
  r=$(exp_rootfs "$name") || true
  [ -n "$r" ] || { exp_skip "$name" "not fetched"; continue; }
  libc=$(exp_rootfs_libc "$name")
  printf '\n  %s  (%s)\n' "$name" "$libc"
  for a in H V P M; do
    f="$WORK/probe-$a"
    [ -x "$f" ] || continue
    "$REPO_DIR/pgb" rootfs run "$r" --copy "$f:/probe" -- /probe \
      > "$WORK/o.$a.$name" 2>/dev/null
    st=$?
    out=$(tr -d '\r' < "$WORK/o.$a.$name" | head -1)
    eval "ROWS_$a=\$((ROWS_$a+1))"
    # ⛔ COUNT THE AXES THAT DIED, NOT THE PROCESS'S EXIT STATUS.
    #
    # ⚠ THIS IS A DEFECT THIS EXPERIMENT SHIPPED AND THEN CAUGHT. The
    # per-axis fork is what makes a crash survivable — the parent reaps the
    # child and carries on, so the PARENT always exits 0. The first version
    # counted `$st` and reported "crashed = 0" for the vanilla arm on a run
    # whose own rows plainly read `nss=SIG8`, `hostid=SIG8` and `iconv=SIG6`.
    # The pre-registered Q1 was therefore scored against the wrong number.
    # A signal token in the output is the measurement; $st is kept only so a
    # probe that dies BEFORE forking anything is still visible.
    _sigs=$(printf '%s' "$out" | grep -o 'SIG[0-9]*' | wc -l | tr -d ' ')
    if [ "$_sigs" -gt 0 ] || [ "$st" != 0 ]; then
      eval "CRASH_$a=\$((CRASH_$a+1))"
    fi
    # ⛔ EXTRACT THE TOKEN BY NAME. DO NOT GLOB THE WHOLE LINE.
    #
    # ⚠ THIS IS THE THIRD DEFECT THIS EXPERIMENT CAUGHT IN ITSELF, and it is
    # the one that proves the "two runs or it is not a number" rule. The
    # counter was `case "$out" in *locale=*UTF-8*)`, which reads as "contains
    # locale=, and later contains UTF-8". Adding the `locreq` axis put a
    # second UTF-8 on the same line, so a row printing
    #     locale=ANSI_X3.4-1968 locreq=UTF-8
    # started counting as a UTF-8 environment. The glibc arms went 0 -> 7 and
    # pgb went 0 -> 11 BETWEEN TWO RUNS OF THE SAME EXPERIMENT with no change
    # to the subject. The first run's zeros were the true ones.
    #
    # ⭐ `tok` splits on spaces and matches the token's own name, so a value
    # can never be read out of a neighbouring field.
    tok() { printf '%s' "$2" | tr ' ' '\n' | sed -n "s/^$1=//p" | head -1; }
    case "$(tok locreq "$out")" in UTF-8|utf8|UTF8) eval "LREQ_$a=\$((LREQ_$a+1))";; esac
    case "$(tok locale "$out")" in UTF-8|utf8|UTF8) eval "UTF8_$a=\$((UTF8_$a+1))";; esac
    [ "$(tok tz "$out")"       = "CEST+0200" ] && eval "TZOK_$a=\$((TZOK_$a+1))"
    [ "$(tok services "$out")" = "80" ]        && eval "SVC_$a=\$((SVC_$a+1))"
    [ "$(tok nss "$out")"      = "root" ]      && eval "NSSOK_$a=\$((NSSOK_$a+1))"
    _ic=$(tok iconv "$out" | cut -d/ -f1)
    case "$_ic" in ''|*[!0-9]*) : ;; *) eval "ICONV_$a=\$((ICONV_$a+_ic))";; esac
    printf '    %-2s %-84s [exit %s]\n' "$a" "${out:-<none>}" "$st"
  done
done

# ---------------------------------------------------------------------------
# ⛔ THE PAYLOAD AXIS IS TRACED, NOT ASSERTED, and it is criterion 2 of
# docs/AGENTS.md §3: a binary loads no HOST SHARED OBJECT. `ldd` is not a
# test here and neither is `file`.
# ---------------------------------------------------------------------------
printf '\n'
printf -- '-- payload: host shared objects opened, by pid-attributed trace ------\n'
for a in H V P M; do eval "OBJ_$a=0"; done
if command -v strace >/dev/null 2>&1; then
  for name in $ENVS; do
    r=$(exp_rootfs "$name") || true
    [ -n "$r" ] || continue
    line="  $name"
    for a in H V P M; do
      f="$WORK/probe-$a"
      [ -x "$f" ] || { line="$line  $a=-"; continue; }
      cp "$f" "$r/probe63" 2>/dev/null || { line="$line  $a=-"; continue; }
      n=$(exp_trace_libs "$r" /probe63 "$WORK/tr.$a.$name" | wc -l | tr -d ' ')
      rm -f "$r/probe63"
      line="$line  $a=$n"
      [ "$n" -gt 0 ] && eval "OBJ_$a=\$((OBJ_$a+1))"
    done
    printf '%s\n' "$line"
  done
else
  exp_skip "payload trace" "no strace on PATH"
fi

# ---------------------------------------------------------------------------
# The summary the deliverable is built from.
# ---------------------------------------------------------------------------
printf '\n'
printf -- '-- summary ---------------------------------------------------------\n'
printf '  %-22s %8s %8s %8s %8s\n' AXIS 'H host' 'V pinned' 'P pgb' 'M musl'
srow() { # label var-prefix
  eval "printf '  %-22s %8s %8s %8s %8s\n' '$1' \"\$$2_H\" \"\$$2_V\" \"\$$2_P\" \"\$$2_M\""
}
srow "rows measured"           ROWS
srow "rows with a dead axis"   CRASH
srow "nss=root"                NSSOK
srow "services resolved"       SVC
srow "UTF-8 by environment"    UTF8
srow "UTF-8 when REQUESTED"    LREQ
srow "timezone CEST+0200"      TZOK
srow "iconv encodings (sum)"   ICONV
srow "envs loading host .so"   OBJ

printf '\n'

# ⭐ THE CONTROL FOR THE BUILD-LOCATION CHOICE. If H and V disagree on any
# capability axis, the vanilla column is not a single thing and the table has
# to say which compiler it describes.
if [ "$H_OK" = yes ] && [ "$V_OK" = yes ]; then
  hv_same=yes
  for v in CRASH NSSOK SVC UTF8 TZOK ICONV OBJ; do
    eval "_h=\$${v}_H; _v=\$${v}_V"
    [ "$_h" = "$_v" ] || hv_same=no
  done
  exp_check "control: host-built and pinned-built vanilla agree" "$hv_same" yes
  exp_note "⚠ if this ever reads no, the 'vanilla' column is two different"
  exp_note "   things and docs/comparison.md must name the compiler."
else
  exp_skip "control: host-built and pinned-built vanilla agree" "an arm did not build"
fi

# --- the pre-registered predictions, as assertions --------------------------
exp_check "Q1  P has no dead axis on any row"       "$CRASH_P" 0
exp_check "Q1  M has no dead axis on any row"       "$CRASH_M" 0
exp_check "Q1  V has a dead axis somewhere"         "$([ "$CRASH_V" -gt 0 ] && echo yes || echo no)" yes
exp_check "Q5  P resolves the zone on every row"    "$TZOK_P" "$ROWS_P"
exp_check "Q6  P loaded a host object on no row"    "$OBJ_P" 0
exp_check "Q6  M loaded a host object on no row"    "$OBJ_M" 0
exp_check "Q2  P and M fail services on the same count" "$SVC_P" "$SVC_M"

# ---------------------------------------------------------------------------
# ⛔ Q3 WAS FALSIFIED, ON BOTH HALVES, AND THE AXIS IS NOT SOFTENED.
#
# Pre-registered (commit e7fdc180, before the run): *"M reports a non-UTF-8
# codeset on ALL 11 (musl has no locale support); P reports UTF-8 on 11 of 11
# with --embed-locale."* ⛔ THE OPPOSITE HAPPENED ON BOTH HALVES:
#
#   - musl reports UTF-8 on 11 of 11. musl's minimal locale support does not
#     mean a poor codeset — its default charset IS UTF-8, unconditionally.
#   - EVERY glibc arm, pgb included, reports ANSI_X3.4-1968 when asked for the
#     ENVIRONMENT's locale with no LANG set, which is what a bare container
#     gives a program.
#
# ⭐ SO THIS IS A ROW WHERE NATIVE MUSL STATIC BEATS BOTH GLIBC COLUMNS, and
# it is the one axis in this matrix where that is true. `--embed-locale`
# answers a REQUEST for UTF-8; it does not change what an unset LANG means,
# and nothing in this tree claimed it did — but nothing had measured the
# unset-LANG case either, so the parity claim had a hole exactly here.
#
# ⚠ THE ASSERTIONS BELOW ENCODE THE MEASURED RESULT so this becomes a
# regression test, which is what `experiments/97-` does with its own numbers.
# ⛔ That is not the same as softening the axis: the losing row is still
# printed, still counted, and is carried into docs/comparison.md as a loss.
exp_check "Q3 FALSIFIED: M has UTF-8 by environment on every row" "$UTF8_M" "$ROWS_M"
exp_check "Q3 FALSIFIED: P has UTF-8 by environment on no row"    "$UTF8_P" 0
exp_check "Q3 FALSIFIED: V has UTF-8 by environment on no row"    "$UTF8_V" 0
exp_check "and P DOES have UTF-8 when it is REQUESTED"            "$LREQ_P" "$ROWS_P"
exp_note "⛔ THE LOCALE ROW IS A LOSS AGAINST NATIVE MUSL. With no LANG set,"
exp_note "   musl answers UTF-8 on $UTF8_M of $ROWS_M and pgb answers"
exp_note "   ANSI_X3.4-1968 on all $ROWS_P. Requesting C.UTF-8 by name, pgb"
exp_note "   answers UTF-8 on $LREQ_P of $ROWS_P."
exp_note "⚠ The pre-registration predicted this row the other way round, on"
exp_note "   both halves. It is recorded rather than rewritten: the header's"
exp_note "   Q3 stands and the block above says what actually happened."

exp_note "⛔ THE ROW THAT COMES OUT AGAINST US IS \`services\`, and it is"
exp_note "   REPORTED rather than softened: P resolves it on $SVC_P of $ROWS_P and"
exp_note "   M on $SVC_M of $ROWS_M. pgb has no mechanism for /etc/services;"
exp_note "   experiments/82- is where that row was found. T-079."
exp_note "⚠ SKIPS: an arm that did not build is a SKIP above, never a dash in"
exp_note "   the table and never a pass. skip=\$SKIP at the end is the number"
exp_note "   to read before believing any column."

exp_finish
