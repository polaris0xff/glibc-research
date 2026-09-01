#!/bin/sh
# THE QUESTION
#
#   What is on this machine, and which of the things every later experiment
#   assumes is actually true here?
#
# Every other script in this directory depends on some of: a C toolchain that
# can link statically, root plus CAP_SYS_ADMIN for the chroot bed, the pinned
# root filesystems, and a network route. When one of those is missing the
# failure surfaces later as a confusing result rather than as a missing
# prerequisite, so this runs first and says which.
#
# ⚠ IT ALSO RECORDS THE HOST. A number in docs/ is only meaningful beside the
# machine that produced it; this is where that machine is written down.
#
# Exit: 0 the bed is complete, 1 the bed is incomplete (later experiments will
# skip rows), 2 could not run at all.

. "$(dirname "$0")/lib.sh"

exp_begin "10 - probe the host"

# --- the toolchain ---------------------------------------------------------
T=$(mktemp -d) || exit 2
trap 'rm -rf "$T"' EXIT INT TERM

printf 'int main(void){return 0;}\n' > "$T/t.c"
if ${CC:-cc} -static -o "$T/t" "$T/t.c" 2>/dev/null; then
  exp_check "cc can link -static" yes yes
else
  exp_check "cc can link -static" no yes
fi

if ${CC:-cc} -shared -fPIC -o "$T/t.so" "$T/t.c" 2>/dev/null; then
  exp_check "cc can build a shared object" yes yes
else
  exp_check "cc can build a shared object" no yes
fi

# The NSS override this project turns on is a real, versioned, PUBLIC glibc
# symbol rather than a GLIBC_PRIVATE one, and it has to be present in the
# STATIC archive, which is the case the whole design rests on.
if [ -n "${PGB_LIBC_A:-}" ]; then LIBC_A="$PGB_LIBC_A"; else
  LIBC_A=$(${CC:-cc} -print-file-name=libc.a 2>/dev/null)
fi
if [ -f "$LIBC_A" ] && nm -A "$LIBC_A" 2>/dev/null | grep -q 'T __nss_configure_lookup'; then
  exp_check "__nss_configure_lookup present in libc.a" yes yes
  exp_note "libc.a = $LIBC_A"
else
  exp_check "__nss_configure_lookup present in libc.a" no yes
  exp_note "libc.a = ${LIBC_A:-not found}"
fi

# --- the isolation bed -----------------------------------------------------
if [ "$(id -u)" = 0 ]; then
  exp_check "running as root" yes yes
else
  exp_check "running as root" no yes
fi

if command -v unshare >/dev/null 2>&1 && unshare --mount --propagation private true 2>/dev/null; then
  exp_check "unshare --mount works" yes yes
else
  exp_check "unshare --mount works" no yes
fi

if sh "$REPO_DIR/scripts/common/rootfs-run.sh" --selftest >/dev/null 2>&1; then
  exp_check "rootfs-run isolation selftest" pass pass
else
  exp_check "rootfs-run isolation selftest" fail pass
fi

if sh "$REPO_DIR/scripts/common/oci-pull.sh" --selftest >/dev/null 2>&1; then
  exp_check "oci-pull whiteout selftest" pass pass
else
  exp_check "oci-pull whiteout selftest" fail pass
fi

# --- container runtimes, which this project deliberately does not need -----
# ⚠ RECORDED, NOT REQUIRED. The bed is chroot-based precisely because neither
# of these is available here; this line exists so a reader on a machine that
# HAS a daemon knows the difference between their environment and the one the
# numbers came from.
if docker info >/dev/null 2>&1; then exp_note "docker daemon: reachable"
else exp_note "docker daemon: NOT reachable (the bed does not use one)"; fi
if command -v podman >/dev/null 2>&1; then exp_note "podman: present"
else exp_note "podman: absent (the bed does not use one)"; fi

# --- the test bed ----------------------------------------------------------
printf '\n  target root filesystems:\n'
have=0; missing=0
while read -r ref name libc digest; do
  case "$ref" in ''|\#*) continue ;; esac
  r=$(exp_rootfs "$name")
  if [ -n "$r" ]; then
    actual=$(exp_rootfs_libc "$name")
    if [ "$actual" = "$libc" ]; then
      printf '    ok    %-20s %-6s\n' "$name" "$actual"
      have=$((have+1))
    else
      # ⛔ The row says musl and the filesystem says glibc: the pin and the
      # unpack disagree, and every result taken against it would be mislabelled.
      printf '    WRONG %-20s declared=%s actual=%s\n' "$name" "$libc" "$actual"
      FAIL=$((FAIL+1))
    fi
  else
    printf '    --    %-20s absent\n' "$name"
    missing=$((missing+1))
  fi
done < "$REPO_DIR/scripts/common/rootfs-images.txt"

exp_check "target root filesystems missing" "$missing" 0
exp_note "run: sh scripts/common/fetch-rootfs.sh"

{
  exp_conditions
  printf '\nrootfs present: %s  missing: %s\n' "$have" "$missing"
} > "$EXP_OUT/conditions.txt" 2>&1

exp_finish
