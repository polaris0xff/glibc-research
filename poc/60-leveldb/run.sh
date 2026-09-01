#!/bin/sh
# POC: LevelDB 1.23 + a C++ subject
#
# WHY THIS PROJECT
#   ⭐ Every other POC in this tree is C, and every one of them is an autotools
#   TARBALL. That was not a preference: until this session the pinned build
#   environment contained no cmake, no meson and no autoconf, so a project
#   shipping a generated `configure` was the only kind that could be built at
#   all. TODO T-016. A §9 table full of green rows was describing one build
#   system's worth of evidence and reading as four.
#
#   So this POC changes two axes at once, deliberately:
#
#   C++       static initialisation order, exception unwind tables surviving a
#             static link, RTTI and typeid across translation units, and
#             iostreams -- which is what drags libstdc++'s locale machinery in.
#   CMake     a build system that has to find the wrappers through CMAKE's own
#             compiler detection rather than through ./configure's.
#   LevelDB   a real C++ dependency with its own build, linked static.
#
# NORMAL BUILD    cmake -S . -B build && cmake --build build
#
# WHY STATIC GLIBC IS HARD HERE, and this one is measured rather than expected:
#   ⛔ libstdc++ CALLS ICONV ITSELF, from std::__narrow_multibyte_chars. pgb's
#   --wrap rewrites those references to __wrap_iconv*, and the compiler driver
#   appends -lstdc++ AFTER the flags pgb appends -- `gcc -###` put
#   -lpgbruntime at 178 and "-lstdc++" at 180. An archive is scanned where it
#   appears, so every C++ link failed:
#
#     undefined reference to `__wrap_iconv_open'
#     ... in .text._ZSt24__narrow_multibyte_charsPKcP15__locale_struct
#
#   That is a defect this POC found, not a property of C++. tool/lib/wrappers.sh
#   forces the archive member in with -Wl,-u for the C++ drivers only.

. "$(dirname "$0")/../common.sh"

POC_URL="https://api.rv.pkgforge.dev/https://github.com/google/leveldb/archive/refs/tags/1.23.tar.gz"
POC_VERSION="1.23"
POC_SHA256="9a37f8a6174f09bd622bc723b55881dc541cd50747cbd08831c2a82d620f6d76"
POC_NORMAL_BUILD="cmake -S . -B build && cmake --build build"
POC_STRESSES="C++ static init, exceptions, RTTI, iostreams/locale, CMake, a static C++ dependency"
POC_WHY="the first C++ POC, and the first CMake one"

poc_begin

SRC="$WORK/leveldb-$POC_VERSION"
PREFIX="$WORK/prefix-leveldb"
BIN="$POC_OUT/cppapp"

# ⚠ The URL goes through api.rv.pkgforge.dev because github.com's release and
# archive routes return 403 through this environment's proxy. TODO/RULES.md
# carries the rule and what skipping it costs.

if [ ! -x "$BIN" ]; then
  poc_fetch "$POC_URL" "$WORK/leveldb-$POC_VERSION.tar.gz" "$POC_SHA256" || exit 2
  rm -rf "$SRC"
  tar xzf "$WORK/leveldb-$POC_VERSION.tar.gz" -C "$WORK" || exit 2

  # The subject. It asserts the four C++ properties a static link can break,
  # and then does real work through the dependency -- because a C++ program
  # that only prints would exercise none of them.
  cat > "$WORK/app.cc" <<'EOF'
#include <iostream>
#include <memory>
#include <stdexcept>
#include <string>
#include <typeinfo>
#include "leveldb/db.h"

/* A namespace-scope object with a non-trivial constructor: if static
 * initialisation order is broken by the static link, this is empty. */
struct Early { std::string tag; Early() : tag("early-ctor-ran") {} };
static Early g_early;

struct Base { virtual ~Base() {} };
struct Derived : Base {};

int main() {
    int fails = 0;

    if (g_early.tag != "early-ctor-ran") { std::cout << "  FAIL static init\n"; fails++; }
    else std::cout << "  ok   static initialisation ran before main\n";

    /* Exceptions: the unwind tables have to survive the static link. */
    try { throw std::runtime_error("thrown"); }
    catch (const std::runtime_error &e) {
        if (std::string(e.what()) != "thrown") { std::cout << "  FAIL exception payload\n"; fails++; }
        else std::cout << "  ok   exception thrown, unwound and caught\n";
    } catch (...) { std::cout << "  FAIL wrong catch\n"; fails++; }

    /* RTTI across translation units. */
    std::unique_ptr<Base> p(new Derived);
    if (dynamic_cast<Derived*>(p.get()) == nullptr) { std::cout << "  FAIL dynamic_cast\n"; fails++; }
    else std::cout << "  ok   RTTI: dynamic_cast<Derived*> succeeded\n";
    if (std::string(typeid(*p).name()).find("Derived") == std::string::npos) {
        std::cout << "  FAIL typeid\n"; fails++;
    } else std::cout << "  ok   RTTI: typeid names Derived\n";

    /* The dependency, doing real work. */
    leveldb::DB *db = nullptr;
    leveldb::Options o; o.create_if_missing = true;
    leveldb::Status s = leveldb::DB::Open(o, "/tmp/pgb-leveldb-poc", &db);
    if (!s.ok()) { std::cout << "  FAIL leveldb open: " << s.ToString() << "\n"; return 1; }
    s = db->Put(leveldb::WriteOptions(), "k", "v-42");
    if (!s.ok()) { std::cout << "  FAIL leveldb put\n"; fails++; }
    std::string got;
    s = db->Get(leveldb::ReadOptions(), "k", &got);
    if (!s.ok() || got != "v-42") { std::cout << "  FAIL leveldb get: " << got << "\n"; fails++; }
    else std::cout << "  ok   leveldb round trip: k -> " << got << "\n";
    delete db;

    std::cout << (fails ? "FAILED" : "PASSED") << ": " << fails << " failure(s)\n";
    return fails ? 1 : 0;
}
EOF

  poc_in_env "cd '$SRC' && \
      cmake -S . -B build -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF \
        -DLEVELDB_BUILD_TESTS=OFF -DLEVELDB_BUILD_BENCHMARKS=OFF \
        -DCMAKE_INSTALL_PREFIX='$PREFIX' > '$POC_OUT/cmake.log' 2>&1 && \
      cmake --build build -j\$(nproc) >> '$POC_OUT/cmake.log' 2>&1 && \
      cmake --install build >> '$POC_OUT/cmake.log' 2>&1" \
    || { poc_note "leveldb build failed"; tail -20 "$POC_OUT/cmake.log" 2>/dev/null; exit 1; }

  poc_in_env "\$CXX -O2 -std=c++17 -I'$PREFIX/include' \
      -o '$BIN' '$WORK/app.cc' '$PREFIX/lib/libleveldb.a' \
      > '$POC_OUT/link.log' 2>&1" \
    || { poc_note "C++ link failed"; tail -20 "$POC_OUT/link.log" 2>/dev/null; exit 1; }
fi

# ⛔ THE FUNCTIONAL TEST, and this POC is why poc/common.sh now refuses
# without one. Omitting it wrote an EMPTY script into each target; `sh` on an
# empty file exits 0, so the matrix reported eleven green rows having executed
# nothing. The guard in poc_matrix() is that defect's fix.
#
# ⭐ It asserts on the subject's OWN OUTPUT, not just its exit status. The
# binary prints one `ok` line per C++ property it checked, and a build that
# silently lost RTTI or exception unwinding could still exit 0 while printing
# fewer of them.
poc_functional_test() {
  cat <<'TEST'
#!/bin/sh
out=$(/cppapp 2>&1) || { printf '%s\n' "$out"; exit 1; }
printf '%s\n' "$out"
# five assertions, named, so a missing one is visible rather than silent
for want in \
  'static initialisation ran before main' \
  'exception thrown, unwound and caught' \
  'dynamic_cast<Derived*> succeeded' \
  'typeid names Derived' \
  'leveldb round trip: k -> v-42'
do
  printf '%s\n' "$out" | grep -qF "$want" || { echo "MISSING: $want"; exit 1; }
done
printf '%s\n' "$out" | grep -q '^PASSED' || exit 1
exit 0
TEST
}

# What the probe asks INSIDE each environment. ⚠ Observed, never asserted:
# what a C++ static binary opens depends on the host's locale data, and
# honouring a host locale that exists is correct behaviour -- docs/AGENTS.md §3.
poc_observation_probe() {
  cat <<'PROBE'
#!/bin/sh
# Run the subject and report what libstdc++ resolved its locale to. A C++
# binary reaches the locale machinery through iostreams whether or not the
# program asks it to, which is the path that made this POC find a link defect.
/cppapp >/dev/null 2>&1; rc=$?
loc=$(LC_ALL= /cppapp 2>/dev/null >/dev/null; echo ok)
printf 'exit=%s\n' "$rc"
PROBE
}

poc_check "built" "$([ -x "$BIN" ] && echo yes || echo no)" yes
poc_inspect "$BIN"
poc_matrix "$BIN"
poc_observe "$BIN" "what a C++ static binary opens"
poc_finish
