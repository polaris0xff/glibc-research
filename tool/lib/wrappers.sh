# tool/lib/wrappers.sh -- part of `pgb`. Sourced by it, never executed.
#
# ⛔ SOURCED, NOT RUN. `pgb build` re-enters itself inside the build
# environment as `pgb __inner-build`, and `pgb verify` enters every target
# rootfs. Both depend on that being ONE process: a library executed as a child
# would put a shell between `pgb` and the thing it is measuring, and the
# PGB_OPT_* handoff in `../../pgb` exists precisely because that boundary is
# already where options got lost once. So: `. "$PGB_SELF/tool/lib/wrappers.sh"`,
# no shebang, no `set -e`, no exec.
#
# ⚠ Every path here resolves from $PGB_SELF, which `../../pgb` sets from its
# own location. Nothing resolves from the caller's working directory.
#
# Holds: the runtime archive, the compiler wrapper directory, the flags they
#        inject, and `pgb explain`, which prints those flags and nothing else.
#
# SPDX-License-Identifier: MIT

# ---------------------------------------------------------------------------
# The runtime archive and the wrapper directory.
# ---------------------------------------------------------------------------
# ⛔ THE CACHE KEY IS THE COMPILER, NOT THE MACHINE.
#
# $PGB_STATE is bind-mounted into the build environment so the wrappers and
# runtime objects are visible there. That means an object compiled by the
# HOST's gcc 13.3 sits in the same directory the chrooted gcc 12.2 would look
# in, and a key of just $(uname -m) would hand the pinned environment an object
# built by the unpinned host -- silently reintroducing exactly the host
# contamination `pgb env create` exists to remove. Keying on the compiler's own
# identity makes the two coexist instead of colliding.
runtime_dir() {
  CCB="${PGB_INNER_CC:-${CC:-cc}}"
  id=$({ $CCB -dumpmachine 2>/dev/null; $CCB --version 2>/dev/null | head -1; } \
       | cksum | cut -d' ' -f1)
  printf '%s/runtime-%s-%s' "$PGB_STATE" "$(uname -m)" "${id:-unknown}"
}

build_runtime() {
  rd=$(runtime_dir); mkdir -p "$rd" || die "cannot create $rd" 2
  src="$PGB_SELF/tool/runtime"
  CCB="${PGB_INNER_CC:-${CC:-cc}}"

  if [ ! -f "$rd/pgb-nssfix.o" ] || [ "$src/pgb-nssfix.c" -nt "$rd/pgb-nssfix.o" ]; then
    vsay "compiling pgb-nssfix.o"
    $CCB -O2 -fno-lto -c -o "$rd/pgb-nssfix.o" "$src/pgb-nssfix.c" || die "nssfix build failed"
  fi
  if [ "$USE_ICONV" = 1 ] &&
     { [ ! -f "$rd/libpgbruntime.a" ] || [ "$src/pgb-iconv.c" -nt "$rd/libpgbruntime.a" ]; }; then
    vsay "compiling libpgbruntime.a"
    $CCB -O2 -fno-lto -c -o "$rd/pgb-iconv.o" "$src/pgb-iconv.c" || die "iconv shim build failed"
    rm -f "$rd/libpgbruntime.a"
    ar rcs "$rd/libpgbruntime.a" "$rd/pgb-iconv.o" || die "ar failed"
  fi
  if [ "$EMBED_LOCALE" = 1 ]; then
    build_locale_data "$rd" "$CCB"
  fi
  if [ "$EMBED_CACERT" = 1 ]; then
    build_cacert_data "$rd" "$CCB"
  fi
  if [ -n "${WRAP_DLOPEN:-}" ]; then
    build_dlopen_table "$rd" "$CCB"
  fi
  printf '%s' "$rd"
}

# ---------------------------------------------------------------------------
# Generate the compiled-in plugin table from the objects the build produced.
#
# ⭐ THIS IS THE PART THAT MAKES IT A MECHANISM RATHER THAN A PATCH.
# references/allyourcodebase__pipewire/tree/src/wrap/dlfcn.zig hand-writes the
# equivalent table for one program, naming every plugin and every symbol.
# Here `nm` reads them out of the objects instead, so a project nobody has
# written a table for is served by the same command as one that has.
#
# Spec: NAME=OBJECT[,OBJECT...]
#   NAME    what the program passes to dlopen(). The matching rule in
#           tool/runtime/pgb-dlopen.c is exact-then-basename, so a bare
#           `foo.so` here matches a program that dlopens a long absolute path
#           ending in foo.so.
#   OBJECT  any .o or .a the build produced. Its DEFINED, EXTERNAL symbols
#           become the plugin's dlsym table.
#
# -- ⛔ WHY EVERY PLUGIN'S SYMBOLS ARE RENAMED, AND WHAT FOUND IT ------------
#
# A real dlopen gives each object its own namespace: two plugins may both
# define `foo` and neither sees the other's. This mechanism does the opposite
# by construction -- it puts the plugin objects in ONE executable -- so
# without help the second plugin does not fail at run time, it fails at LINK
# time and the whole build stops.
#
# ⭐ MEASURED, AND IT IS NOT AN EDGE CASE. SQLite's loadable-extension ABI
# requires every extension to carry `SQLITE_EXTENSION_INIT1`, which declares a
# file-scope, NON-static `const sqlite3_api_routines *sqlite3_api`. All 16 of
# the extensions in sqlite's own ext/misc define it, so ANY TWO of them
# collide:
#
#   ld: uuid.o:(.bss+0x0): multiple definition of `sqlite3_api';
#       series.o:(.bss+0x0): first defined here
#
# ⛔ And it is worse than one ABI's habit: sqlite derives an extension's entry
# point from its FILENAME, keeping only alphabetic characters, so base64.c and
# base85.c both define `sqlite3_base_init` ON PURPOSE. Two plugins colliding
# on their entry point is a thing upstreams deliberately do.
#
# ⭐ THE FIX IS THE NAMESPACE THE LOADER WOULD HAVE GIVEN THEM. Every symbol a
# plugin object DEFINES is renamed to a per-plugin prefix with
# `objcopy --redefine-syms`, and the table maps the ORIGINAL name to the
# renamed one. dlsym still answers `sqlite3_series_init`; nothing else in the
# link can see it. That is RTLD_LOCAL, reproduced at link time.
#
# ⚠ Only DEFINED symbols are renamed. A plugin's calls back into the host
# program are UNDEFINED references and are untouched, so they still bind.
#
# ⚠ THE BEHAVIOUR CHANGE, STATED: a plugin's symbols are no longer visible to
# the rest of the executable under their own names. That is what a separate
# .so loaded RTLD_LOCAL already does, so this makes the mechanism agree with
# what it is imitating -- but a program that called a plugin function
# DIRECTLY, without dlsym, would now fail to link. Nothing does: an object
# reached by dlopen is by definition reached by name.
build_dlopen_table() {
  rd="$1"; CCB="$2"
  gen="$rd/pgb-dlopen-table.c"
  : > "$gen"
  printf '/* GENERATED by pgb. Do not edit: rebuilt on every build. */\n' >> "$gen"
  printf '#include "%s/tool/runtime/pgb-dlopen.h"\n\n' "$PGB_SELF" >> "$gen"

  objs=""; idx=0; entries=""
  for spec in $WRAP_DLOPEN; do
    case "$spec" in
      *=*) : ;;
      *) die "--wrap-dlopen wants NAME=OBJECT[,OBJECT...], got: $spec" 2 ;;
    esac
    name=${spec%%=*}
    objlist=$(printf '%s' "${spec#*=}" | tr ',' ' ')

    syms=""
    for o in $objlist; do
      [ -f "$o" ] || die "--wrap-dlopen: no such object: $o" 2
      # ⛔ NOT added to $objs here. The link gets the NAMESPACED copy made
      # below, never the original -- linking both would reintroduce exactly
      # the duplicate-symbol collision the renaming exists to remove.
      # ⛔ DEFINED AND EXTERNAL ONLY. Without --defined-only the table would
      # carry the plugin's own UNDEFINED references -- every libc function it
      # calls -- and generating `extern void X; ... &X` for those makes the
      # link fail on symbols the plugin merely imports. Without
      # --extern-only it would carry file-local statics, which are not
      # addressable from another translation unit at all.
      s=$(nm --defined-only --extern-only "$o" 2>/dev/null \
          | awk '$2 ~ /^[TtDdBbRrWwGgSs]$/ { print $3 }' \
          | grep -E '^[A-Za-z_][A-Za-z0-9_]*$' || true)
      syms="$syms $s"
    done
    # ⚠ sort -u, because two objects of one plugin can both define a symbol
    # only one of them exports, and a duplicated table row would be a second
    # `extern` declaration of the same name in the generated file.
    syms=$(printf '%s\n' $syms | sort -u)
    [ -n "$syms" ] || die "--wrap-dlopen: $name has no defined external symbols in:$objlist" 1

    # ⭐ The namespace. Every defined symbol gets a per-plugin prefix, so two
    # plugins that both define `sqlite3_api` -- which every SQLite extension
    # does -- stop colliding. See the block above this function for what
    # found this and why the answer is renaming rather than localising:
    # localising would make the symbol unaddressable from the generated
    # table, which is the one translation unit that must still reach it.
    command -v objcopy >/dev/null 2>&1 || \
      die "--wrap-dlopen needs objcopy (binutils) to namespace plugin symbols" 2
    pfx="pgb_dl${idx}_"
    map="$rd/pgb-dlopen-renames-$idx.txt"
    : > "$map"
    for sym in $syms; do
      printf '%s %s%s\n' "$sym" "$pfx" "$sym" >> "$map"
    done
    nobjs=""
    ni=0
    for o in $objlist; do
      no="$rd/pgb-dl-$idx-$ni-$(basename "$o")"
      objcopy --redefine-syms="$map" "$o" "$no" \
        || die "--wrap-dlopen: could not namespace $o" 1
      nobjs="$nobjs $no"
      ni=$((ni+1))
    done
    objs="$objs $nobjs"

    for sym in $syms; do
      printf 'extern char %s%s[];\n' "$pfx" "$sym" >> "$gen"
    done
    printf '\nstatic const struct pgb_dl_sym pgb_dl_syms_%s[] = {\n' "$idx" >> "$gen"
    for sym in $syms; do
      printf '    { "%s", (void *)%s%s },\n' "$sym" "$pfx" "$sym" >> "$gen"
    done
    printf '    { NULL, NULL }\n};\n\n' >> "$gen"
    entries="$entries $idx:$name"
    idx=$((idx+1))
  done

  printf 'const struct pgb_dl_lib pgb_dlopen_libs[] = {\n' >> "$gen"
  for e in $entries; do
    printf '    { "%s", pgb_dl_syms_%s },\n' "${e#*:}" "${e%%:*}" >> "$gen"
  done
  printf '    { NULL, NULL }\n};\n' >> "$gen"

  vsay "generated $gen ($idx plugin(s))"
  $CCB -O2 -fno-lto -c -o "$rd/pgb-dlopen-table.o" "$gen" \
    || die "generated plugin table did not compile: $gen"

  # ⛔ THE WRAPPER GOES IN THE ARCHIVE, THE TABLE DOES NOT. The table defines
  # pgb_dlopen_libs, which the archive member only WEAKLY references, so an
  # archive member is not pulled in by it -- the table has to be a plain
  # object on the link line or it would silently not be there and every
  # dlopen would report "no plugin table was compiled in".
  if [ ! -f "$rd/pgb-dlopen.o" ] || [ "$PGB_SELF/tool/runtime/pgb-dlopen.c" -nt "$rd/pgb-dlopen.o" ]; then
    $CCB -O2 -fno-lto -c -o "$rd/pgb-dlopen.o" "$PGB_SELF/tool/runtime/pgb-dlopen.c" \
      || die "dlopen shim build failed"
  fi

  # The plugin objects themselves have to be ON the link line: the table takes
  # their addresses, and an object nobody references is not pulled from an
  # archive. Recorded for link_flags to emit.
  printf '%s\n' "$objs" > "$rd/pgb-dlopen-objs"
}

# Turn the build environment's CA bundle into a C file of bytes.
#
# ⛔ THE BUNDLE COMES FROM THE ENVIRONMENT, NOT FROM THE HOST, and that matters
# for the same reason static libiconv does: it must be the pinned
# environment's ca-certificates snapshot, so two machines building the same
# source embed the same trust store. `pgb env create` installs
# ca-certificates, so this is always present inside the environment.
#
# ⚠ AND IT IS ONLY A FALLBACK. tool/runtime/pgb-cacert.c probes the host's own
# store first and materialises this copy only where the host has none -- the
# three minimal Debian/Ubuntu images in the matrix. A stale embedded bundle
# used in preference to a current host one would be a security regression, not
# a portability fix.
build_cacert_data() {
  rd="$1"; CCB="$2"
  [ -f "$rd/pgb-cacert.o" ] && [ -f "$rd/pgb-cacert-data.o" ] && return 0
  src=""
  for c in /etc/ssl/certs/ca-certificates.crt /etc/pki/tls/certs/ca-bundle.crt \
           /etc/ssl/ca-bundle.pem /etc/ssl/cert.pem; do
    [ -s "$c" ] && { src="$c"; break; }
  done
  [ -n "$src" ] || die "--embed-cacert found no CA bundle in the build environment (install ca-certificates)" 2
  vsay "embedding CA bundle from $src ($(wc -c < "$src") bytes)"
  gen="$rd/pgb-cacert-data.c"
  {
    printf '/* generated by pgb from %s -- do not edit */\n' "$src"
    printf 'const unsigned char pgb_cacert_data[] = {'
    od -An -v -tu1 < "$src" | tr -s ' ' | tr ' ' '\n' | grep -v '^$' | tr '\n' ','
    printf '};\n'
    printf 'const unsigned pgb_cacert_len = sizeof pgb_cacert_data;\n'
  } > "$gen"
  $CCB -O0 -fno-lto -c -o "$rd/pgb-cacert-data.o" "$gen" || die "CA bundle data build failed"
  $CCB -O2 -fno-lto -c -o "$rd/pgb-cacert.o" "$PGB_SELF/tool/runtime/pgb-cacert.c" \
    || die "CA bundle shim build failed"
}

# Turn a glibc locale directory into a C file of byte arrays.
build_locale_data() {
  rd="$1"; CCB="$2"
  [ -f "$rd/pgb-locale.o" ] && [ -f "$rd/pgb-locale-data.o" ] && return 0
  srcdir=""
  for c in /usr/lib/locale/C.utf8 /usr/lib/locale/C.UTF-8; do
    [ -d "$c" ] && { srcdir="$c"; break; }
  done
  [ -n "$srcdir" ] || die "--embed-locale needs a compiled C.UTF-8 on this machine (locale-gen C.UTF-8)" 2
  vsay "embedding locale from $srcdir"
  gen="$rd/pgb-locale-data.c"
  {
    printf '/* generated by pgb from %s -- do not edit */\n' "$srcdir"
    printf 'struct pgb_locale_file { const char *name; const unsigned char *data; unsigned len; };\n'
    # ⛔ `find -type f`, NOT a top-level glob. A glibc locale is a TREE:
    # LC_MESSAGES is a directory holding SYS_LC_MESSAGES. A glob plus
    # `[ -f ]` silently drops it, and a locale missing one category fails
    # the whole LC_ALL composite at run time with no diagnostic anywhere.
    # The name stored is RELATIVE to the locale root so the separator
    # survives into the extracted tree.
    files=$(cd "$srcdir" && find . -type f | sed 's|^\./||' | sort)
    i=0
    for f in $files; do
      printf 'static const unsigned char d%d[] = {' "$i"
      od -An -v -tu1 < "$srcdir/$f" | tr -s ' ' | tr ' ' '\n' | grep -v '^$' | tr '\n' ','
      printf '};\n'
      i=$((i+1))
    done
    printf 'const struct pgb_locale_file pgb_locale_files[] = {\n'
    i=0
    for f in $files; do
      printf '  { "%s", d%d, sizeof d%d },\n' "$f" "$i" "$i"
      i=$((i+1))
    done
    printf '};\n'
    printf 'const unsigned pgb_locale_nfiles = %d;\n' "$i"
    printf 'const char pgb_locale_name[] = "%s";\n' "$(basename "$srcdir")"
  } > "$gen"
  $CCB -O0 -fno-lto -c -o "$rd/pgb-locale-data.o" "$gen" || die "locale data build failed"
  $CCB -O2 -fno-lto -c -o "$rd/pgb-locale.o" "$PGB_SELF/tool/runtime/pgb-locale.c" || die "locale shim build failed"
}

# link_flags [cxx]
#
# ⛔ THE `cxx` ARGUMENT EXISTS BECAUSE C++ DOES NOT LINK WITHOUT IT, and it was
# found by building the first C++ project this project has ever built.
#
# libstdc++ calls iconv itself, from std::__narrow_multibyte_chars. --wrap
# rewrites those undefined references to __wrap_iconv*, exactly as it does for
# the application's. But the wrappers append pgb's flags to the END of the
# user's argv, and the compiler driver then appends ITS OWN libraries after
# that -- `gcc -###` on this machine puts -lpgbruntime at 178 and "-lstdc++"
# at 180. An archive is scanned once, where it appears, so by the time
# libstdc++ introduces those references the archive holding them is behind the
# linker and every C++ link fails:
#
#   undefined reference to `__wrap_iconv_open'
#   ... in .text._ZSt24__narrow_multibyte_charsPKcP15__locale_struct
#
# ⭐ -u forces the member in AT -lpgbruntime, so the symbols are already
# defined when libstdc++ is scanned. Same technique this file already uses for
# pgb_runtime_anchor, and for the same reason: an archive member nothing has
# referenced yet is not loaded.
#
# ⚠ THE COST, STATED RATHER THAN HIDDEN: a C++ program now links the iconv
# shim whether or not it calls iconv, so it pays libiconv's ~900 KiB. That is
# why this is NOT applied to C links -- docs/AGENTS.md §10 measures 940 KiB vs
# 2.1 MiB for a C program that does not call iconv, and that property is kept.
# ⚠ In practice most C++ programs pay it anyway: anything that instantiates a
# locale-aware facet pulls __narrow_multibyte_chars in.
#
# -- --eh-frame-hdr, AND WHY IT IS ON EVERY LINK ----------------------------
#
# ⛔ GCC SUPPRESSES `--eh-frame-hdr` FOR EVERY `-static` LINK. Its spec reads
# `%{!static|static-pie:--eh-frame-hdr}` (gcc/config/gnu-user.h), so GNU ld
# leaves a static executable with NO `.eh_frame_hdr` section and NO
# `PT_GNU_EH_FRAME` segment. Measured here, and it is not a pgb behaviour --
# plain `g++ -static` on this machine produces the same:
#
#   dynamic c++            PT_GNU_EH_FRAME: 1
#   g++ -static            PT_GNU_EH_FRAME: 0
#   pgb build -- c++       PT_GNU_EH_FRAME: 0   (before this flag)
#
# ⚠ NOTHING IS BROKEN TODAY AND THAT IS THE POINT. With the GNU toolchain the
# unwinder never needs the segment: `crtbeginT.o` registers `__EH_FRAME_BEGIN__`
# through `__register_frame_info`, and `_Unwind_Find_FDE` -- which `nm` finds in
# the binary -- reads that registry first. Exceptions work. `poc/60-leveldb`
# throws, catches, and asserts on the payload, and it passes on all eleven.
#
# ⛔ THE HAZARD IS THAT THE FALLBACK IS THE GNU RUNTIME'S, NOT THE FORMAT'S. An
# unwinder that discovers tables ONLY through PT_GNU_EH_FRAME finds nothing, and
# the failure mode is `std::terminate` at the first throw with `catch (...)` in
# main never running -- silent at build time, fatal at run time. pg83/solo hit
# exactly this with static LLVM libunwind (its PR #3, at commit 79451211;
# docs/research/solo.md), and its own CI missed it because the one leg using
# gcc did not run the smoke test.
#
# ⭐ The flag is a NO-OP where the header is already emitted, and it makes the
# unwind tables discoverable by anything that walks program headers --
# backtrace(), profilers, and any loader compiled into the binary later.
#
# ⚠ THE COST, MEASURED RATHER THAN GUESSED. `.eh_frame_hdr` is a binary-search
# table over every FDE in the link, so on a static glibc binary it is not
# small: 16,004 bytes of section, and on this machine
#
#   ci/probe.c    2,161,056 -> 2,177,568   (+16,512)
#   a C++ throw   2,265,744 -> 2,286,344   (+20,600)
#
# ⭐ The size property docs/AGENTS.md §10 rests on is unaffected: a C program
# that never calls iconv is 952,536 bytes with the flag, still under 1 MiB
# against the 2.1 MiB of one that does.
link_flags() {
  rd=$(runtime_dir)
  printf -- '-static -Wl,--eh-frame-hdr %s -Wl,-u,pgb_runtime_anchor' "$rd/pgb-nssfix.o"
  if [ "$USE_ICONV" = 1 ]; then
    printf -- ' -Wl,--wrap=iconv_open,--wrap=iconv,--wrap=iconv_close'
    # ⛔ AND THE SAME FORCING IS NEEDED WHENEVER --wrap-dlopen IS IN PLAY,
    # for a C program, and this was measured on MLT linked against a static
    # ffmpeg. The wrapper re-emits the caller's own -l after pgb's flags (see
    # make_wrappers), so a caller archive is scanned AFTER -lpgbruntime -- and
    # ffmpeg's libavformat calls iconv, so --wrap rewrites those to
    # __wrap_iconv* and the archive that defines them is already behind the
    # linker:
    #
    #   libavformat.a(mpegts.o): undefined reference to `__wrap_iconv_open'
    #
    # ⭐ -u pulls the defining member in AT -lpgbruntime, so the symbols exist
    # before the repeated libraries are scanned. Identical technique, identical
    # reason, one layer further out than the C++ case above.
    { [ "${1:-}" = cxx ] || [ -n "${WRAP_DLOPEN:-}" ]; } && \
      printf -- ' -Wl,-u,__wrap_iconv_open -Wl,-u,__wrap_iconv -Wl,-u,__wrap_iconv_close'
    printf -- ' -L%s -lpgbruntime -L%s/lib -liconv' "$rd" "$PGB_LIBICONV_PREFIX"
  fi
  if [ "$EMBED_LOCALE" = 1 ]; then
    printf -- ' -Wl,--wrap=setlocale %s %s' "$rd/pgb-locale.o" "$rd/pgb-locale-data.o"
  fi
  if [ "$EMBED_CACERT" = 1 ]; then
    # ⛔ -u, for the same reason as pgb_runtime_anchor: this is a CONSTRUCTOR
    # and nothing references it, so without forcing the symbol the linker is
    # entitled to drop it -- and a trust-store fix that silently did not link
    # is the exact failure the file exists to prevent.
    printf -- ' -Wl,-u,pgb_cacert_anchor %s %s' "$rd/pgb-cacert.o" "$rd/pgb-cacert-data.o"
  fi
  if [ -n "${WRAP_DLOPEN:-}" ] && [ -f "$rd/pgb-dlopen-table.o" ]; then
    printf -- ' -Wl,--wrap=dlopen,--wrap=dlsym,--wrap=dlclose,--wrap=dlerror'
    printf -- ' %s %s' "$rd/pgb-dlopen.o" "$rd/pgb-dlopen-table.o"
    [ -f "$rd/pgb-dlopen-objs" ] && printf -- ' %s' "$(cat "$rd/pgb-dlopen-objs")"
  fi
}

compile_flags() {
  bl="${ARCH_BASELINE:-$(default_baseline)}"
  [ -n "$bl" ] && printf -- '-march=%s ' "$bl"
  printf -- '-fno-plt'
}

make_wrappers() {
  wd="$PGB_STATE/bin"; rm -rf "$wd"; mkdir -p "$wd" || die "cannot create $wd" 2
  cf=$(compile_flags)
  for pair in "cc:cc" "gcc:gcc" "c++:c++" "g++:g++" "cpp:cpp"; do
    name=${pair%%:*}; real=${pair##*:}
    # The C++ drivers get the forcing flags; the C ones must not -- see
    # link_flags() for what that costs and why the split is where it is.
    case "$name" in c++|g++) lf=$(link_flags cxx) ;; *) lf=$(link_flags) ;; esac
    # ⛔ SET ONLY WHEN --wrap-dlopen IS IN PLAY, so a build without it produces
    # a byte-identical wrapper to before this existed.
    pgbdl=""; [ -n "${WRAP_DLOPEN:-}" ] && pgbdl=1
    realpath_=$(command -v "$real" 2>/dev/null) || realpath_=""
    [ -n "$realpath_" ] || continue
    case "$realpath_" in "$wd"/*) continue ;; esac
    cat > "$wd/$name" <<EOF
#!/bin/sh
# pgb compiler wrapper -- generated, readable on purpose.
# Real compiler: $realpath_
REAL="$realpath_"
CF="$cf"
LF="$lf"
PGBDL="$pgbdl"
mode=link
for a in "\$@"; do
  case "\$a" in
    -c|-E|-S|-M|-MM)          mode=compile ;;
    -shared|-dynamiclib)      mode=shared ;;
    # ⛔ A QUERY MUST NEVER BE DECORATED. configure, libtool and CMake all
    # parse the OUTPUT of these; adding flags changes what they read and the
    # build then makes decisions on a value pgb invented.
    -print-*|--print-*|-dumpmachine|-dumpversion|-dumpspecs|--version|-V)
      exec "\$REAL" "\$@" ;;
  esac
done
case "\$mode" in
  compile) [ -n "\${PGB_VERBOSE:-}" ] && echo "pgb[compile] \$REAL \$* \$CF" >&2
           exec "\$REAL" "\$@" \$CF ;;
  shared)  [ -n "\${PGB_VERBOSE:-}" ] && echo "pgb[shared,passthrough] \$REAL \$*" >&2
           exec "\$REAL" "\$@" ;;
  link)
           # ⛔ THE PLUGIN OBJECTS ARE APPENDED AFTER THE CALLER'S LIBRARIES,
           # and an object's undefined references can only be resolved by a
           # library that comes AFTER it. So a plugin that calls pow() links
           # against a -lm the caller already spent:
           #
           #   filter_lumaliftgaingamma.c:(.text+0x1e1):
           #       undefined reference to \`pow'
           #
           # ⛔ AND THE CALLER CANNOT FIX IT, because the caller does not
           # control where pgb puts those objects. Measured on MLT, whose
           # modules are exactly this shape. Same defect class as the
           # -lstdc++ ordering above, one layer further out.
           #
           # ⭐ The fix is to re-emit the caller's own -l/-L after them.
           # Repeating a -l is safe: an archive member is pulled in once, and
           # a repeated shared library is one DT_NEEDED either way. Only the
           # caller's libraries are repeated -- nothing is invented, so a
           # plugin needing a library the program never named still fails,
           # loudly, which is correct.
           RELIBS=""
           if [ -n "\$PGBDL" ]; then
             _next=""
             for a in "\$@"; do
               if [ -n "\$_next" ]; then RELIBS="\$RELIBS \$_next \$a"; _next=""; continue; fi
               case "\$a" in
                 -l|-L)   _next=\$a ;;
                 -l*|-L*) RELIBS="\$RELIBS \$a" ;;
               esac
             done
           fi
           [ -n "\${PGB_VERBOSE:-}" ] && echo "pgb[link] \$REAL \$* \$CF \$LF \$RELIBS" >&2
           exec "\$REAL" "\$@" \$CF \$LF \$RELIBS ;;
esac
EOF
    chmod +x "$wd/$name"
  done
  printf '%s' "$wd"
}

# ---------------------------------------------------------------------------
# explain -- the anti-black-box command
# ---------------------------------------------------------------------------
cmd_explain() {
  bl="${ARCH_BASELINE:-$(default_baseline)}"
  cat <<EOF
pgb $PGB_VERSION injects exactly this, and nothing else.

WHERE IT IS INJECTED
  Not into your source and not into your build files. pgb puts a directory of
  compiler wrappers first on PATH and sets CC/CXX to them, so autotools,
  CMake, meson and plain make all pick it up without knowing pgb exists.
  \`pgb cc-dir\` prints that directory; every wrapper is a readable shell
  script you can cat.

  Each wrapper looks at its own argv and decides:
    -c / -E / -S / -M      a compile.  compile flags only
    -shared                a shared library.  PASSED THROUGH UNCHANGED
    anything else          an executable link.  link flags added
  Passing -shared through is what lets a normal ./configure run: its shared
  library probes must keep working or the build fails before it starts.

COMPILE FLAGS
  -fno-plt                       no procedure linkage table indirection
  -march=$bl$(printf '%*s' $((22 - ${#bl})) '')CPU baseline. NOT -march=native. A binary built
                                 with -march=native runs on the machine that
                                 built it and crashes elsewhere with SIGILL,
                                 which looks exactly like a portability bug in
                                 this tool and is not one.

LINK FLAGS
  -static                        no interpreter, no DT_NEEDED, nothing to
                                 resolve on the host
  -Wl,--eh-frame-hdr             gcc DROPS this for every -static link, so a
                                 static executable normally has no
                                 PT_GNU_EH_FRAME. GNU libgcc does not need it
                                 -- crtbeginT.o registers the frames instead --
                                 but an unwinder that reads only the segment
                                 finds nothing, and the failure is
                                 std::terminate at the first throw. +16 KiB
  <pgb-nssfix.o>                 passed as a plain object, not from an archive
  -Wl,-u,pgb_runtime_anchor      forces it to stay. A constructor with no
                                 referenced symbol is dropped by the linker,
                                 and an nssfix that silently did not link is
                                 the exact failure it exists to prevent
$( [ "$USE_ICONV" = 1 ] && cat <<EOF2
  -Wl,--wrap=iconv_open          redirect the three public iconv entry points
  -Wl,--wrap=iconv               to GNU libiconv, statically linked. Applies
  -Wl,--wrap=iconv_close         to every object in the link, including static
                                 libraries built before pgb existed
  -lpgbruntime -liconv           the shim lives in an archive, so a program
                                 that never calls iconv_open links none of it
                                 and pays none of the ~900 KiB
EOF2
)
$( [ "$EMBED_CACERT" = 1 ] && cat <<EOFCA
  -Wl,-u,pgb_cacert_anchor       forces the CA-store constructor to stay. It
                                 probes the host's own trust store in nine
                                 known locations and points SSL_CERT_FILE,
                                 CURL_CA_BUNDLE and SSL_CERT_DIR at it.
  <pgb-cacert.o>                 ⛔ it NEVER overrides a variable you set, and
                                 it never disables verification. Where no host
                                 store exists it writes the embedded copy to
                                 \$TMPDIR and points at that -- the only
                                 filesystem write, and only there.
  <pgb-cacert-data.o>            the pinned environment's ca-certificates
                                 snapshot, $( [ -s /etc/ssl/certs/ca-certificates.crt ] && wc -c < /etc/ssl/certs/ca-certificates.crt || echo '?') bytes.
                                 ⚠ IT AGES. Roots are revoked and expire, so
                                 the host's store is always preferred and this
                                 is a fallback, never a pre-emption
EOFCA
)$( [ "$EMBED_LOCALE" = 1 ] && cat <<EOF3
  -Wl,--wrap=setlocale           embedded C.UTF-8, materialised ONLY when the
  <pgb-locale-data.o>            host cannot answer a UTF-8 setlocale. A
                                 program that never calls setlocale writes
                                 nothing and touches no directory
EOF3
)$( [ -n "${WRAP_DLOPEN:-}" ] && cat <<EOF4
  -Wl,--wrap=dlopen              answer the program's OWN plugin loads from a
  -Wl,--wrap=dlsym               table compiled in from the objects you named,
  -Wl,--wrap=dlclose             instead of asking the host's ld.so. Nothing
  -Wl,--wrap=dlerror             is mapped, and no second libc can enter
  <pgb-dlopen-table.o>           GENERATED for this build. Its symbols come
                                 from 'nm --defined-only --extern-only' over
                                 each object you named, so a file-local static
                                 is not in the table and will not resolve
  <your plugin objects>          on the link line, because the table takes
                                 their addresses
                                 ⚠ this does NOT load a HOST plugin and is not
                                 trying to. docs/AGENTS.md §13 item 4
  requested                      $WRAP_DLOPEN
EOF4
)

WHAT EACH ONE IS FOR, WITH THE MEASUREMENT
  NSS      __nss_configure_lookup() pins every database to services glibc
           2.34+ implements inside libc, so the host's /etc/nsswitch.conf
           names nothing that can be dlopen'd.
           experiments/20-static-glibc-nss-dlopen.sh
  iconv    glibc's gconv modules are dlopen'd and carry DT_NEEDED libc.so.6.
           GNU libiconv carries the same tables as archive code.
           experiments/30-gconv-and-locale.sh
  locale   glibc's C.UTF-8 is files on disk, not code in libc.
           experiments/30-gconv-and-locale.sh

WHAT pgb DOES NOT DO, AND WILL NOT PRETEND TO
  - It does not make dlopen() of host plugins work. A static binary that
    dlopens a host shared object pulls a second libc in, which is the failure
    this tool exists to avoid. A program whose CORE function is loading host
    plugins is outside the class pgb serves. docs/limitations.md is explicit.
  - It does not give you NSS data from LDAP, SSSD, NIS or systemd-resolved.
    Keeping those modules out is the fix, and losing them is its cost.
  - It does not lower the kernel ABI floor. Your build glibc decides that.
EOF
}
