#!/bin/sh
# THE QUESTION
#
#   A bundle carrying more than one program has to CHOOSE one. Does the
#   choice work, and does a SECOND program actually run out of a bundle on
#   all eleven environments?
#
# ⛔ WHY THIS EXISTS: THE MECHANISM IS SHIPPED AND HAS NEVER BEEN RUN.
# `internal/bundle/assemble.go` installs the entry point and then every other
# non-dot program in the same store path's `bin/`; `tool/runtime/pgb-apprun.c`
# is a static selector compiled in whenever a bundle carries more than one.
# docs/research/app-corpus.md rung 1 says it "works by construction" and marks
# the claim READ OFF THE SOURCE. ⭐ A source reading is not a result.
#
# -- ⭐ TWO ARMS, AND THEY ANSWER DIFFERENT HALVES ---------------------------
#
#   S  the SELECTOR, against a synthetic AppDir. Needs no nix, no bed and no
#      display, so it can run while another corpus has the machine. It
#      measures the dispatch TABLE: which of ARGV0 / argv[1] / argv[0] wins,
#      what is passed on, and what is refused.
#   B  the BUNDLE, end to end on all eleven. It measures that a second
#      program really comes out of a real closure through a real artefact.
#
# ⛔ NEITHER ARM SUBSTITUTES FOR THE OTHER. Arm S cannot see sharun, uruntime
# or the closure; arm B cannot tell WHICH rule fired. Run both:
#
#   sh scripts/common/run-experiment.sh 68          both arms
#   PGB_EXP68_ONLY=S sh experiments/68-multi-entry.sh   the selector only
#
# ⚠ A run that measures one arm says so in its summary and the verdict below
# fails on it, because a half-run that reports green is the thing this tree
# calls the worst possible answer (docs/AGENTS.md §0b).
#
# -- ⛔ PRE-REGISTERED EXPECTATIONS ------------------------------------------
#
# ⛔ COMMITTED BEFORE THE RUN. PROGRESS.md delivery rule 1.
#
#   E1  ARGV0's basename wins first. A bundle RENAMED or SYMLINKED to a
#       program's name runs that program -- this is the rule uruntime feeds,
#       because it sets ARGV0 to the AppImage's own path.
#   E2  ⭐ ARGV0 BEATS argv[1], AND argv[1] BEATS argv[0]. The file's own
#       header comment states the order as "argv[0] ... then argv[1]" and
#       names neither ARGV0's precedence nor argv[0]'s position. ⛔ THE
#       COMMENT IS PREDICTED WRONG AND THE CODE PREDICTED RIGHT; if the run
#       says otherwise the prediction was wrong and the record says so.
#   E3  a selected argv[1] is CONSUMED (the program does not see its own
#       name), and an argv[1] that did NOT select is passed through.
#   E4  argv[0] handed to the target is the ABSOLUTE path `<appdir>/bin/<n>`,
#       not the bare name. The source calls this load-bearing for sharun.
#   E5  ARGV0 is UNSET in the child, and APPDIR is SET to the AppDir.
#   E6  ⛔ A NAME CONTAINING '/' IS NEVER A PROGRAM. `./AppRun ../etc/passwd`
#       must not select, must not consume the argument, and must fall through
#       to the default. This is the one rule with a security reading.
#   E7  a name present in `shared/bin` but ABSENT from `bin/` is not a
#       program. Both halves are required and the source says so.
#   E8  ⭐ THE NEGATIVE CONTROL: a selector built with an EMPTY default and
#       given no matching name EXITS 127 and prints "no default program".
#       ⛔ Without this arm S cannot tell dispatch from a selector that runs
#       the same thing whatever it is told -- which is exactly the shape of
#       non-discriminating criterion `experiments/64-` was rejected for.
#   E9  arm B: a SECOND program runs out of a real bundle on 11 of 11, and
#       its assertion is the program's OWN identity, never a shared version
#       string. `mkvmerge --version` says "mkvmerge", `mkvextract --version`
#       says "mkvextract": a dispatch that always ran the default fails.
#   E10 arm B: the build PRINTS the entry-point count (`programs <p> + N
#       more`). That is the operator's question 3 answered by a number the
#       tool emits rather than by a reading of assemble.go.
#   E11 arm B: the second program loads ZERO host shared objects on 11 of 11,
#       exactly as the first one must.
#
# ⚠ E9 AND E11 ARE THE ONES THAT NEED THE BED. Everything else is arm S.
#
# -- ⭐ WHAT ARM S IS COMPARED AGAINST ---------------------------------------
#
# The field's `quick-sharun.sh` writes a SHELL AppRun whose rule is
#   ARG0="${ARGV0:-$0}"  ->  bin/${ARG0##*/}  ->  bin/$1 (shift)  ->  MAIN_BIN
# Ours is the same order with one addition: where theirs collapses ARGV0 and
# $0 into one test and so never re-checks $0 when ARGV0 is set-but-unmatched,
# ours falls through ARGV0 -> argv[1] -> argv[0]. ⚠ Ours is a SUPERSET, and
# E2 is what establishes that rather than asserting it.
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "68 - multi-entry dispatch: does a SECOND program come out of a bundle"

WORK="${PGB_EXP68_WORK:-/var/tmp/t068}"
ONLY="${PGB_EXP68_ONLY:-SB}"
RUN_TIMEOUT="${PGB_EXP68_TIMEOUT:-150}"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2

ARMS_RUN=""
command -v cc >/dev/null 2>&1 || { exp_note "no cc on PATH"; exit 2; }

# ===========================================================================
# ARM S -- the selector, against a synthetic AppDir
# ===========================================================================
#
# ⭐ THE APPDIR IS SYNTHETIC ON PURPOSE. A real bundle's bin/<name> is a
# hardlink of sharun, which sets a library path and runs the bundled loader --
# none of which this arm is asking about. Replacing it with a program that
# prints its own identity is what makes "which one ran" observable at all.
#
# ⚠ STATED LIMIT: the stand-in programs here are ordinary host binaries, so
# arm S says NOTHING about host shared objects. That is arm B's E11.
case "$ONLY" in *S*)
  ARMS_RUN="$ARMS_RUN S"
  AD="$WORK/AppDir"
  mkdir -p "$AD/shared/bin" "$AD/bin" || exit 2

  # Each stand-in prints its identity, its own argv[0] and its arguments, and
  # whether ARGV0/APPDIR reached it. Every expectation below reads one of
  # those lines, so a program that ran but was handed the wrong argv fails.
  cat > "$WORK/stand-in.c" <<'EOF'
#include <stdio.h>
#include <stdlib.h>
int main(int argc, char **argv)
{
    const char *a0 = getenv("ARGV0"), *ad = getenv("APPDIR");
    int i;
    printf("PROGRAM=%s\n", PGB_NAME);
    printf("ARGV0_SET=%s\n", a0 ? "yes" : "no");
    printf("APPDIR=%s\n", ad ? ad : "(unset)");
    printf("ARGV0_IS=%s\n", argv[0] ? argv[0] : "(null)");
    printf("NARGS=%d\n", argc - 1);
    for (i = 1; i < argc; i++) printf("ARG%d=%s\n", i, argv[i]);
    return 0;
}
EOF
  for n in alpha beta; do
    cc -O0 -DPGB_NAME="\"$n\"" -o "$AD/bin/$n" "$WORK/stand-in.c" 2>>"$WORK/cc.log" || exit 2
    # shared/bin/<name> must be a REGULAR FILE for is_program to accept it.
    printf 'payload %s\n' "$n" > "$AD/shared/bin/$n"
  done
  # E7's subject: present in shared/bin, absent from bin/.
  printf 'payload gamma\n' > "$AD/shared/bin/gamma"

  SRC="$REPO_DIR/tool/runtime/pgb-apprun.c"
  [ -f "$SRC" ] || { exp_note "missing $SRC"; exit 2; }
  cc -O2 -DPGB_APPRUN_DEFAULT='"alpha"' -o "$AD/AppRun" "$SRC" 2>>"$WORK/cc.log" \
    || { exp_note "could not build the selector; see $WORK/cc.log"; exit 2; }
  # ⛔ E8's negative control LIVES IN THE SAME AppDir, and the first version of
  # this file put it in $WORK. The selector derives its AppDir from
  # /proc/self/exe, so a control sitting outside the tree sees an AppDir with
  # no programs in it at all -- it exited 127 for the wrong reason and the
  # check went green. A control that passes because it can see nothing is not
  # a control. docs/AGENTS.md §0b: verify before you trust.
  cc -O2 -DPGB_APPRUN_DEFAULT='""' -o "$AD/AppRun-nodefault" "$SRC" \
    2>>"$WORK/cc.log" || exit 2

  # ⭐ argv[0] MUST BE SETTABLE INDEPENDENTLY OF THE PATH EXECUTED, or the
  # argv[0] rule cannot be tested at all. POSIX sh has no way to do that --
  # `exec -a` and `"${@:3}"` are bash, and this tree's experiments are sh --
  # so the helper is eleven lines of C. It execs argv[2] with argv[1] as the
  # child's argv[0].
  cat > "$WORK/execas.c" <<'EOF'
#include <stdio.h>
#include <string.h>
#include <unistd.h>
int main(int argc, char **argv)
{
    char *path;
    if (argc < 3) { fprintf(stderr, "execas ARGV0 PATH [args...]\n"); return 2; }
    path = argv[2];         /* what is actually exec'd */
    argv[2] = argv[1];      /* ...while the child's argv[0] is the name given */
    execv(path, &argv[2]);
    perror("execas");
    return 127;
}
EOF
  cc -O2 -o "$WORK/execas" "$WORK/execas.c" 2>>"$WORK/cc.log" || exit 2

  # sel <tag> [VAR=VAL...] <argv...>   ; output lands in $WORK/<tag>.out
  # ⛔ NO `--` IN THE CALLS. `env -u ARGV0 VAR=VAL -- prog` treats `--` as the
  # PROGRAM, because it is no longer in option position once an assignment has
  # been seen. Six checks read `(none)` before that was found, and the three
  # that happened to pass did so because their `--` did follow the options.
  sel() {
    _tag=$1; shift
    env -u ARGV0 "$@" > "$WORK/$_tag.out" 2>"$WORK/$_tag.err"
    printf '%s' $? > "$WORK/$_tag.code"
  }
  # field <tag> <KEY> -> the value of KEY= in that run's output, or "(none)"
  field() {
    _v=$(grep -m1 "^$2=" "$WORK/$1.out" 2>/dev/null | cut -d= -f2-)
    printf '%s' "${_v:-(none)}"
  }

  run_as_argv0() {   # tag argv0 [args-to-AppRun...]
    _tag=$1; _a0=$2; shift 2
    env -u ARGV0 "$WORK/execas" "$_a0" "$AD/AppRun" "$@" \
      > "$WORK/$_tag.out" 2>"$WORK/$_tag.err"
    printf '%s' $? > "$WORK/$_tag.code"
  }

  printf '\n-- arm S: the dispatch table -------------------------------------\n'

  # E1/E2: ARGV0 names beta, and it wins even though argv[1] names alpha.
  sel e1 ARGV0=/wherever/it/came/from/beta "$AD/AppRun"
  exp_check "E1  ARGV0's basename selects the program" "$(field e1 PROGRAM)" beta

  sel e2 ARGV0=/x/beta "$AD/AppRun" alpha
  exp_check "E2  ⭐ ARGV0 BEATS argv[1]" "$(field e2 PROGRAM)" beta
  # ...and because ARGV0 won, argv[1] was NOT consumed.
  exp_check "E3  an argv[1] that did not select is passed on" "$(field e2 ARG1)" alpha

  # E2b: with no ARGV0, argv[1] selects -- and it beats argv[0], which the
  # header comment states in the opposite order.
  run_as_argv0 e2b /some/path/alpha beta
  exp_check "E2  ⭐ argv[1] BEATS argv[0]" "$(field e2b PROGRAM)" beta

  # E3: a selected argv[1] is consumed, and later arguments survive.
  sel e3 "$AD/AppRun" beta --flag value
  exp_check "E3  a selected argv[1] is CONSUMED" "$(field e3 ARG1)" --flag
  exp_check "E3  the arguments after it survive" "$(field e3 NARGS)" 2

  # E2c: argv[0] selects when nothing else does.
  run_as_argv0 e2c /some/path/beta
  exp_check "E2  argv[0]'s basename selects when it is alone" "$(field e2c PROGRAM)" beta

  # the default, with nothing naming anything
  run_as_argv0 edef /some/path/AppRun
  exp_check "E2  otherwise the DEFAULT runs" "$(field edef PROGRAM)" alpha

  # E4/E5
  exp_check "E4  the child's argv[0] is the ABSOLUTE path" \
    "$(field e1 ARGV0_IS)" "$AD/bin/beta"
  exp_check "E5  ARGV0 is UNSET in the child" "$(field e1 ARGV0_SET)" no
  exp_check "E5  APPDIR is set to the AppDir" "$(field e1 APPDIR)" "$AD"

  # E6 -- a name with a '/' is not a program, is not consumed, and the
  # default runs with it still in argv.
  sel e6 "$AD/AppRun" ../../etc/passwd
  exp_check "E6  ⛔ a name containing '/' does not select" "$(field e6 PROGRAM)" alpha
  exp_check "E6  ⛔ ...and is NOT consumed from argv" "$(field e6 ARG1)" ../../etc/passwd

  # E7 -- shared/bin only is not enough
  sel e7 "$AD/AppRun" gamma
  exp_check "E7  shared/bin without bin/ is not a program" "$(field e7 PROGRAM)" alpha
  exp_check "E7  ...and its name stays in argv" "$(field e7 ARG1)" gamma

  # E8 -- ⭐ THE NEGATIVE CONTROL. Same source, empty default, nothing named.
  # ⛔ If this passes, arm S proves nothing: it would mean the selector runs
  # something no matter what it is told.
  env -u ARGV0 "$AD/AppRun-nodefault" > "$WORK/e8.out" 2>"$WORK/e8.err"
  e8code=$?
  exp_check "E8  ⭐ CONTROL: no default and no name EXITS 127" "$e8code" 127
  if grep -q "no default program" "$WORK/e8.err" 2>/dev/null; then e8msg=yes; else e8msg=no; fi
  exp_check "E8  ⭐ CONTROL: ...and says why" "$e8msg" yes
  # and the same binary DOES dispatch when told a name -- so 127 above is the
  # absence of a default, not a broken build.
  env -u ARGV0 "$AD/AppRun-nodefault" beta > "$WORK/e8b.out" 2>&1
  exp_check "E8  ⭐ CONTROL: the same binary still dispatches" \
    "$(grep -m1 '^PROGRAM=' "$WORK/e8b.out" | cut -d= -f2-)" beta
  ;;
esac

# ===========================================================================
# ARM B -- a real bundle, a real second program, eleven environments
# ===========================================================================
#
# ⭐ THE SUBJECT IS CHOSEN FOR A DISCRIMINATING ASSERTION, NOT FOR SIZE.
# `mkvtoolnix` ships mkvmerge, mkvinfo, mkvextract and mkvpropedit in one
# store path, and each prints its OWN name in --version. ⛔ A dispatch that
# silently ran the default would print "mkvmerge" where "mkvextract" is
# required, so the criterion fails for the right reason (delivery rule 6).
# A shared version string -- imagemagick's `convert`/`identify`, which are
# symlinks to one binary -- could not tell those two apart.
ATTR="${PGB_EXP68_ATTR:-mkvtoolnix}"
ENTRY="${PGB_EXP68_ENTRY:-mkvmerge}"
SECOND="${PGB_EXP68_SECOND:-mkvextract}"

case "$ONLY" in *B*)
  ARMS_RUN="$ARMS_RUN B"
  printf '\n-- arm B: a second program out of a real bundle --------------------\n'
  IMG="$WORK/multi.AppImage"
  BLOG="$WORK/build.log"
  PGB_APPIMAGE_CACHE="$WORK/cache" "$REPO_DIR/pgb" bundle appimage "$ATTR" \
    --out "$IMG" --name "$ENTRY" >"$BLOG" 2>&1 || true

  if [ ! -s "$IMG" ]; then
    # ⛔ UNRESOLVED IS NOT A PASS AND IS NOT A FAILURE OF THE MECHANISM.
    why=$(grep -aoE "nixpkgs has no attribute [^ ]*|no entry point in [^ ]*|could not fetch the closure[^\"]*|--name [^ ]* names no program" "$BLOG" 2>/dev/null | head -1)
    exp_note "⛔ UNRESOLVED: ${why:-see $BLOG}"
    exp_note "   arm B did not run. That is a gap in this measurement, not"
    exp_note "   evidence about the mechanism -- and the verdict below fails"
    exp_note "   on it rather than reporting green on arm S alone."
    ARMS_RUN=$(printf '%s' "$ARMS_RUN" | sed 's/ B//')
  else
    # E10 -- the entry-point count, printed by the build.
    progline=$(grep -a -m1 '^programs ' "$BLOG" | sed 's/  */ /g')
    nmore=$(printf '%s' "$progline" | grep -oE '\+ [0-9]+ more' | grep -oE '[0-9]+')
    exp_note "build says: ${progline:-(no 'programs' line)}"
    exp_check "E10 the build PRINTS extra entry points" \
      "$([ -n "$nmore" ] && [ "$nmore" -gt 0 ] && echo yes || echo no)" yes
    # and the selector was compiled in rather than a shell script written
    if grep -qa '^apprun *pgb-apprun' "$BLOG"; then sel_static=yes; else sel_static=no; fi
    exp_check "E10 the selector is the STATIC one, not a shell" "$sel_static" yes

    ENVS=$(awk '!/^#/ && NF {print $2}' "$REPO_DIR/scripts/common/rootfs-images.txt")
    NENV=$(printf '%s\n' "$ENVS" | wc -l | tr -d ' ')
    ok2=0; clean2=0; ok1=0; rows=0
    for name in $ENVS; do
      root=$(exp_rootfs "$name") || true
      [ -n "$root" ] || { exp_skip "B/$name" "rootfs not fetched"; continue; }
      rows=$((rows+1))
      # ⭐ THE ARTEFACT IS INSTALLED UNDER THE SECOND PROGRAM'S NAME, which is
      # the rename/symlink route E1 measures -- uruntime sets ARGV0 from it.
      rm -f "$root/$SECOND" "$root/$ENTRY"
      cp "$IMG" "$root/$SECOND" 2>/dev/null; chmod +x "$root/$SECOND"
      cp "$IMG" "$root/$ENTRY" 2>/dev/null;  chmod +x "$root/$ENTRY"

      tr2="$WORK/tr.second.$name"
      strace -f -e trace=openat,open,execve,clone,clone3,vfork -o "$tr2" \
        timeout "$RUN_TIMEOUT" "$REPO_DIR/pgb" rootfs run "$root" -- \
          /bin/sh -c "APPIMAGE_EXTRACT_AND_RUN=1 /$SECOND --version" \
        >"$WORK/out.second.$name" 2>"$WORK/err.second.$name"
      st2=$?
      # ⚠ reap BEFORE reading anything: docs/AGENTS.md §6, delivery rule 7.
      for _p in /proc/[0-9]*; do
        _pid=${_p#/proc/}; _rt=$(readlink "/proc/$_pid/root" 2>/dev/null) || continue
        case "$_rt" in "$root"|"$root"/*) kill -9 "$_pid" 2>/dev/null ;; esac
      done
      out2=$(cat "$WORK/out.second.$name" "$WORK/err.second.$name" 2>/dev/null)
      # E9 -- the SECOND program's own identity, not a shared version string.
      if [ "$st2" = 0 ] && printf '%s' "$out2" | grep -qE "^$SECOND v?[0-9]"; then
        ok2=$((ok2+1))
      fi
      nh=$(exp_classify_trace "$tr2" "/$SECOND" | grep -c '^host ' || true)
      [ "$nh" = 0 ] && clean2=$((clean2+1))
      rm -f "$tr2"

      # the ENTRY point, through the same artefact, as the within-row control:
      # if this fails too the bundle is broken and E9 is not about dispatch.
      timeout "$RUN_TIMEOUT" "$REPO_DIR/pgb" rootfs run "$root" -- \
        /bin/sh -c "APPIMAGE_EXTRACT_AND_RUN=1 /$ENTRY --version" \
        >"$WORK/out.first.$name" 2>"$WORK/err.first.$name"
      st1=$?
      for _p in /proc/[0-9]*; do
        _pid=${_p#/proc/}; _rt=$(readlink "/proc/$_pid/root" 2>/dev/null) || continue
        case "$_rt" in "$root"|"$root"/*) kill -9 "$_pid" 2>/dev/null ;; esac
      done
      out1=$(cat "$WORK/out.first.$name" "$WORK/err.first.$name" 2>/dev/null)
      [ "$st1" = 0 ] && printf '%s' "$out1" | grep -qE "^$ENTRY v?[0-9]" && ok1=$((ok1+1))
      rm -f "$root/$SECOND" "$root/$ENTRY"
    done

    exp_note "rows measured: $rows of $NENV"
    exp_check "E9  ⭐ the SECOND program ($SECOND) runs, by its OWN name" "$ok2" "$rows"
    exp_check "E9  the ENTRY program ($ENTRY) still runs (within-row control)" "$ok1" "$rows"
    exp_check "E11 the second program loads ZERO host shared objects" "$clean2" "$rows"
    rm -f "$IMG"; rm -rf "$WORK/cache"
  fi
  ;;
esac

printf '\n'
# ⛔ A HALF-RUN MUST NOT REPORT GREEN. Arm S needs no bed and arm B does, so
# the failure mode this catches is real: a session runs S while a corpus holds
# the machine, reads a green verdict, and never comes back for B.
exp_check "both arms ran (S = the selector, B = the bundle)" \
  "$(printf '%s' "$ARMS_RUN" | tr -d ' ')" "SB"
exp_note "⚠ arm S uses ordinary host stand-in binaries, so it says NOTHING"
exp_note "  about host shared objects; that is arm B's E11 and only E11."
exp_note "⚠ arm B measures ONE closure. A second multi-program subject is a"
exp_note "  different measurement, not a repeat of this one."

exp_finish
