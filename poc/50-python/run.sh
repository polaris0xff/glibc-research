#!/bin/sh
# POC: CPython
#
# WHY THIS PROJECT
#   CPython is the hardest case in this directory and it was chosen for that.
#   It is the project that breaks every simplifying assumption a "just link it
#   statically" story rests on:
#
#   dlopen    ⛔ CPython's normal answer to "import a C extension" is dlopen of
#             a .so out of the stdlib tree. A statically linked interpreter
#             cannot rely on that -- experiment and POC 10 both measured the
#             load succeeding on two distributions out of eleven and failing on
#             nine, which is the worst possible shape for a language runtime.
#             The fix is to build the extension modules INTO the interpreter,
#             which CPython supports through Modules/Setup, and this POC does.
#   data      the standard library is ~2000 files of Python source that the
#             interpreter finds by path at run time. Static linking says
#             nothing about it. This is the same class of problem as gconv,
#             locale, terminfo and the CA bundle, at a much larger scale.
#   libc      Python's os, socket, pwd, grp and time modules are a thin layer
#             over exactly the libc surfaces this project is about. socket
#             calls getaddrinfo; pwd calls getpwuid; both are NSS.
#   locale    Python 3 decides the filesystem encoding from the locale, so a
#             locale that has collapsed to ASCII changes how paths are decoded.
#
# NORMAL BUILD    ./configure && make && make install
# WHY STATIC GLIBC IS HARD HERE
#   Three separate reasons at once: the interpreter must be static, the C
#   extension modules must be inside it rather than dlopen'd, and the stdlib
#   must be reachable. Any one of the three left undone produces an
#   interpreter that starts and then fails on the first interesting import.

. "$(dirname "$0")/../common.sh"

POC_URL="https://www.python.org/ftp/python/3.12.7/Python-3.12.7.tgz"
POC_VERSION="3.12.7"
POC_SHA256="73ac8fe780227bf371add8373c3079f42a0dc62deff8d612cd15a618082ab623"
POC_NORMAL_BUILD="./configure && make && make install"
POC_STRESSES="dlopen extension modules, a large data tree, NSS via socket/pwd, locale"
POC_WHY="a language runtime: static interpreter, builtin extensions, stdlib as data"

poc_begin

SRC="$WORK/Python-$POC_VERSION"
PREFIX="$WORK/prefix-python"
BIN="$POC_OUT/python3"
STDLIB="$POC_OUT/pystdlib"

# ---------------------------------------------------------------------------
# ⭐ PYTHONHOME points the interpreter at the stdlib tree that travels beside
# it. That tree is copied into each target as /pgb-py, so the test is that
# CPython works from a plain directory on a foreign distribution -- not that
# the target happens to have a Python installation.
# ---------------------------------------------------------------------------
poc_functional_test() {
cat <<'TEST'
set -u
fail=0
PY="/python3"
export PYTHONHOME=/pgb-py
export PYTHONDONTWRITEBYTECODE=1
t() { if [ "$2" = "$3" ]; then printf '  ok   %-30s %s\n' "$1" "$2"
      else printf '  FAIL %-30s got [%s] want [%s]\n' "$1" "$2" "$3"; fail=1; fi; }

# 1. the interpreter starts and is the version we built
t version "$($PY -c 'import sys;print("%d.%d"%sys.version_info[:2])' 2>&1)" "3.12"

# 2. pure-Python stdlib: proves the data tree was found and is readable
t stdlib-json "$($PY -c 'import json;print(json.dumps({"a":[1,2]},separators=(",",":")))' 2>&1)" '{"a":[1,2]}'
t stdlib-re "$($PY -c 'import re;print(re.sub(r"\d+","N","a1b22c"))' 2>&1)" "aNbNc"

# 3. ⭐ C EXTENSION MODULES, the whole point. Each of these is a C module that
#    a normal CPython dlopens from lib-dynload. If they import here they were
#    linked INTO the interpreter, which is what makes this binary portable.
for m in zlib binascii math _socket select unicodedata _csv array _struct; do
  if $PY -c "import $m" 2>/dev/null; then
    printf '  ok   %-30s builtin\n' "ext:$m"
  else
    printf '  FAIL %-30s not importable\n' "ext:$m"; fail=1
  fi
done

# 4. ⛔ NO .so MAY BE LOADED. A module that imports because the interpreter
#    found a shared object on THIS host is not portable, it is lucky. This
#    asserts every imported C module came from inside the binary.
t no-dynload "$($PY -c '
import sys
bad=[m.__name__ for m in sys.modules.values()
     if getattr(m,"__file__","") and str(getattr(m,"__file__","")).endswith(".so")]
print(len(bad))' 2>&1)" "0"

# 5. zlib actually compressing, not merely importing
t zlib-roundtrip "$($PY -c '
import zlib
d=b"pgb"*100
print(zlib.decompress(zlib.compress(d))==d)' 2>&1)" "True"

# 6. ⭐ NSS THROUGH PYTHON. socket.getaddrinfo IS glibc getaddrinfo and
#    pwd.getpwuid IS getpwuid -- the calls that crash a naive static binary on
#    Arch and openSUSE.
#
#    ⛔ THE NAME IS ONE THIS TEST CREATES, NOT "localhost", AND THAT IS A
#    CORRECTION. Debian 11, Debian 12 and Ubuntu 20.04 ship a root filesystem
#    whose /etc/hosts has NO localhost line at all -- Docker injects one when
#    it starts a container, and an unpacked image has none. Resolving
#    "localhost" there fails with EAI_NONAME.
#
#    ⚠ That is NOT caused by the NSS override, and the attribution matters:
#    a plain `gcc -static` binary was measured failing identically on those
#    three, so this is a property of the test bed. Using a name the test
#    itself puts in /etc/hosts measures the NSS dispatcher rather than the
#    image's contents. Whether bare "localhost" resolves is recorded in the
#    observation arm instead.
printf '127.0.0.1 pgb-py-probe\n' >> /etc/hosts 2>/dev/null || true
t nss-getaddrinfo "$($PY -c '
import socket
print(socket.getaddrinfo("pgb-py-probe",80,proto=socket.IPPROTO_TCP)[0][4][0])' 2>&1)" "127.0.0.1"
t nss-getpwuid "$($PY -c 'import pwd;print(pwd.getpwuid(0).pw_name)' 2>&1)" "root"

# 7. unicode and the filesystem encoding, which Python derives from the locale
t unicode-len "$($PY -c 'print(len("café"))' 2>&1)" "4"
t unicode-encode "$($PY -c 'print("café".encode("utf-8").hex())' 2>&1)" "636166c3a9"

# 8. codecs: Python has its own encoders and does NOT use glibc gconv, so this
#    must keep working on hosts where glibc's own iconv would have nothing.
t codec-latin1 "$($PY -c 'print("café".encode("latin-1").hex())' 2>&1)" "636166e9"
t codec-utf16 "$($PY -c 'print("ab".encode("utf-16-le").hex())' 2>&1)" "61006200"

# 9. real work: file I/O, subprocess, and the os layer
t file-io "$($PY -c '
open("/tmp/p.txt","w").write("x"*10)
print(len(open("/tmp/p.txt").read()))' 2>&1)" "10"
t subprocess "$($PY -c '
import subprocess
print(subprocess.run(["/bin/echo","hi"],capture_output=True,text=True).stdout.strip())' 2>&1)" "hi"

# 10. threading, which needs TLS to be set up correctly in a static binary
t threading "$($PY -c '
import threading
r=[]
ts=[threading.Thread(target=lambda i=i: r.append(i)) for i in range(8)]
[t.start() for t in ts]; [t.join() for t in ts]
print(sum(r))' 2>&1)" "28"

rm -f /tmp/p.txt
exit $fail
TEST
}

# ---------------------------------------------------------------------------
# OBSERVATION: how many C modules ended up inside the binary, and does the
# interpreter still believe it can load shared ones?
# ---------------------------------------------------------------------------
poc_observation_probe() {
cat <<'PROBE'
export PYTHONHOME=/pgb-py PYTHONDONTWRITEBYTECODE=1
lh=$(/python3 -c '
import socket
try:
    socket.getaddrinfo("localhost",80,proto=socket.IPPROTO_TCP); print("resolves")
except Exception: print("NO")' 2>/dev/null)
n=$(/python3 -c 'import sys;print(len(sys.builtin_module_names))' 2>/dev/null)
d=$(/python3 -c '
import importlib.machinery as m
print(len(m.EXTENSION_SUFFIXES))' 2>/dev/null)
s=$(/python3 -c '
import sys
print(sum(1 for x in sys.path if x.endswith("lib-dynload")))' 2>/dev/null)
echo "builtin=$n localhost=$lh dynload-on-path=$s"
PROBE
}

# ---------------------------------------------------------------------------
if [ ! -x "$BIN" ] || [ "${POC_REBUILD:-0}" = 1 ]; then
  poc_fetch "$POC_URL" "$WORK/Python-$POC_VERSION.tgz" "$POC_SHA256" || exit 2
  rm -rf "$SRC" "$PREFIX"
  tar xzf "$WORK/Python-$POC_VERSION.tgz" -C "$WORK" || exit 2

  # ⭐ THE ONE PIECE OF PROJECT-SPECIFIC CONFIGURATION IN THE WHOLE POC SET,
  # and it is CPython's own documented mechanism rather than a patch.
  #
  # configure generates Modules/Setup.stdlib listing every stdlib extension
  # module whose build dependencies it found, with the correct source files,
  # and a `*shared*` marker at the top. That file says, in its own comment:
  #
  #     # The file is not used by default yet. For testing do:
  #     #     ln -sfr Modules/Setup.stdlib Modules/Setup.local
  #
  # So the entire change is: install it as Setup.local with `*shared*` turned
  # into `*static*`. Every module then links INTO the interpreter instead of
  # becoming a .so under lib-dynload, which is the difference between an
  # `import zlib` that is a dlopen of a host-adjacent object and one that is a
  # symbol already in the binary.
  #
  # ⛔ NO CPYTHON SOURCE FILE IS MODIFIED, and the module list is not
  # hand-maintained. An earlier version of this POC did hand-write the list
  # and got the source filenames wrong -- _asyncio's source is
  # _asynciomodule.c, not _asyncio.c -- which failed the build outright. Using
  # configure's own output cannot drift from the version being built.
  # docs/patches.md records this as configuration, and it is the only entry.
  # ⛔ py_cv_module_nis=n/a IS NOT COSMETIC. A static link pulls WHOLE
  # archives, and CPython's `nis` module links libtirpc, whose
  # svc_auth_gss.o references GSSAPI symbols that are not there. The failure
  # lands on Programs/_freeze_module, a BUILD-TIME tool, so the error reads as
  # a broken toolchain rather than as one unwanted stdlib module:
  #
  #   libtirpc.a(libtirpc_la-svc_auth_gss.o): undefined reference to
  #   `gss_import_name` ... make: *** [Programs/_freeze_module] Error 1
  #
  # ⚠ THIS IS A GENERAL PROPERTY OF -static, not a CPython defect. Dynamic
  # linking defers the unresolved symbols to a library that is never loaded;
  # static linking resolves them at link time and they are simply absent. Any
  # optional module with an incompletely-static dependency does the same, and
  # docs/limitations.md states it once rather than per project.
  #
  # py_cv_module_<name>=n/a is CPython's own supported way to skip a module,
  # not a patch.
  #
  # --disable-test-modules is needed for a related reason. Making ALL of
  # Setup.stdlib static also makes the _testcapi family static, and
  # _testinternalcapi references _Py_Get_Getpath_CodeObject, which exists only
  # in the interpreter -- so the build-time Programs/_freeze_module fails to
  # link. The test modules are not wanted in a shipped binary anyway.
  poc_in_env "cd '$SRC' && \
      ./configure --prefix='$PREFIX' --disable-shared --with-ensurepip=no \
        py_cv_module_nis=n/a --disable-test-modules \
      >'$POC_OUT/configure.log' 2>&1" \
    || { poc_note "python configure failed"; tail -20 "$POC_OUT/configure.log" 2>/dev/null; exit 1; }

  sed 's/^\*shared\*/*static*/' "$SRC/Modules/Setup.stdlib" > "$SRC/Modules/Setup.local" || exit 2
  poc_note "static stdlib modules requested: $(grep -cE '^[a-z_]+ ' "$SRC/Modules/Setup.local")"

  poc_in_env "cd '$SRC' && make -j\$(nproc) >'$POC_OUT/make.log' 2>&1 && \
      make install >'$POC_OUT/install.log' 2>&1" \
    || { poc_note "python build failed"; tail -30 "$POC_OUT/make.log" 2>/dev/null; exit 1; }

  cp "$PREFIX/bin/python3.12" "$BIN" 2>/dev/null || cp "$PREFIX/bin/python3" "$BIN" || exit 2

  # The stdlib tree that travels with the binary. Trimmed of the parts a POC
  # does not need, and the trim is DELETION so every remaining path is the
  # path CPython expects.
  rm -rf "$STDLIB"; mkdir -p "$STDLIB/lib"
  cp -a "$PREFIX/lib/python3.12" "$STDLIB/lib/" || exit 2
  rm -rf "$STDLIB/lib/python3.12/test" "$STDLIB/lib/python3.12/idlelib" \
         "$STDLIB/lib/python3.12/tkinter" "$STDLIB/lib/python3.12/turtledemo" \
         "$STDLIB/lib/python3.12/__pycache__"
  find "$STDLIB" -name '__pycache__' -type d -exec rm -rf {} + 2>/dev/null
  poc_note "stdlib tree: $(du -"$STDLIB" | cut -f1), $(find "$STDLIB" -name '*.py' | wc -l) .py files"
  poc_note "lib-dynload .so files remaining: $(find "$STDLIB" -name '*.so' | wc -l)"
fi

poc_check "built" "$([ -x "$BIN" ] && echo yes || echo no)" yes
poc_inspect "$BIN"
poc_matrix "$BIN" "$STDLIB:/pgb-py"
poc_observe "$BIN" "how many C modules are inside the interpreter" "$STDLIB:/pgb-py"
poc_finish
