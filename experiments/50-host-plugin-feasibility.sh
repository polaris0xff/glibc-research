#!/bin/sh
# THE QUESTION
#
#   Can the class of programs that must load HOST plugins be brought inside
#   this tool's guarantee -- and specifically, does cross-libc-dlopen's
#   symbol-version rewrite help a STATIC glibc binary do it?
#
# -- WHY THIS IS WORTH MEASURING RATHER THAN REASONING ------------------------
#
# docs/limitations.md §1 records that a static glibc binary CAN dlopen a host
# object, that it succeeded on 2 of 11 environments and was refused on 9, and
# that the class of programs needing host plugins is therefore out of scope.
#
# ⛔ WHAT THAT ENTRY DOES NOT SAY IS *WHY* THE NINE REFUSED. Without the
# reason, "adopt cross-libc-dlopen's rewrite and the nine become two hundred"
# is an untested hope. The rewrite fixes exactly one failure class -- symbol
# VERSION requirements the process cannot satisfy. If the nine are failing for
# a different reason, the rewrite cannot help and the extension is dead before
# it is written.
#
# So this experiment collects the dlerror() STRING, per environment, per arm.
#
#   arm A  plain dlopen of a host shared object
#   arm B  the same object, rewritten first: DT_VERSYM, DT_VERNEED, DT_VERDEF
#          and DT_VERDEFNUM neutralised in a private copy, then dlopen'd
#
# ⭐ ARM B IS A FAITHFUL PORT OF ONE FUNCTION FROM PRIOR ART, and it is cited
# rather than reinvented: cld_strip_versions(), at
# references/pkgforge-dev__cross-libc-dlopen/tree/src/cross-libc-dlopen.c
# lines 811-817, commit 1cecf50ef603ed146dfcebda8553ff1558470965. Its comment
# at :809 records the constraint this port keeps: every version tag must go
# together, because a verdef left without its versym table segfaults ld.so.
#
# ⚠ WHAT THIS EXPERIMENT CANNOT SETTLE. cross-libc-dlopen is an LD_PRELOAD for
# a process that already carries a bundled libc AND its loader. This port runs
# in a STATIC binary, which carries neither. So a failure here is evidence
# about the static case only, and specifically NOT evidence that
# cross-libc-dlopen does not work in the setting it was built for.
#
# Exit: 0 the measurement ran, 2 it could not. ⛔ NOTHING IS ASSERTED -- this
# is a feasibility probe, and an expectation would be the guess it exists to
# replace.

. "$(dirname "$0")/lib.sh"

exp_begin "50 - can host-plugin loading be brought in scope? the dlerror evidence"

B="$EXP_OUT/build"
rm -rf "$B"; mkdir -p "$B" || exit 2

cat > "$B/probe.c" <<'EOF'
/* Find a host shared object, try to load it two ways, print what happened. */
#define _GNU_SOURCE
#include <dirent.h>
#include <dlfcn.h>
#include <elf.h>
#include <fcntl.h>
#include <link.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <unistd.h>

/* Unknown d_tag values are ignored by ld.so. Same constant and same idea as
 * cross-libc-dlopen's CLD_NEUTRAL_TAG. */
#define NEUTRAL_TAG 0x50474231 /* 'PGB1' */

/* Port of cld_strip_versions(), cross-libc-dlopen.c:811-817 @ 1cecf50e.
 * Reads `src`, neutralises ALL FOUR version tags in a private copy, writes
 * `dst`. Returns 1 when a copy was written. */
static int strip_versions(const char *src, const char *dst)
{
    int fd = open(src, O_RDONLY);
    if (fd < 0) return 0;
    struct stat st;
    if (fstat(fd, &st) != 0 || st.st_size < (off_t) sizeof(ElfW(Ehdr))) { close(fd); return 0; }
    char *buf = malloc((size_t) st.st_size);
    if (!buf) { close(fd); return 0; }
    if (read(fd, buf, (size_t) st.st_size) != (ssize_t) st.st_size) { free(buf); close(fd); return 0; }
    close(fd);

    ElfW(Ehdr) *eh = (ElfW(Ehdr) *) buf;
    if (memcmp(eh->e_ident, ELFMAG, SELFMAG) != 0) { free(buf); return 0; }
    ElfW(Phdr) *ph = (ElfW(Phdr) *) (buf + eh->e_phoff);

    int stripped = 0;
    for (int i = 0; i < eh->e_phnum; i++) {
        if (ph[i].p_type != PT_DYNAMIC) continue;
        ElfW(Dyn) *dyn = (ElfW(Dyn) *) (buf + ph[i].p_offset);
        size_t n = ph[i].p_filesz / sizeof(ElfW(Dyn));
        for (size_t j = 0; j < n && dyn[j].d_tag != DT_NULL; j++) {
            if (dyn[j].d_tag == DT_VERSYM || dyn[j].d_tag == DT_VERNEED ||
                dyn[j].d_tag == DT_VERDEF || dyn[j].d_tag == DT_VERDEFNUM) {
                dyn[j].d_tag = NEUTRAL_TAG;
                stripped++;
            }
        }
    }
    int out = open(dst, O_WRONLY | O_CREAT | O_TRUNC, 0700);
    if (out < 0) { free(buf); return 0; }
    ssize_t w = write(out, buf, (size_t) st.st_size);
    close(out); free(buf);
    if (w != (ssize_t) st.st_size) return 0;
    printf("    (rewrote %d version tag(s))\n", stripped);
    return 1;
}

/* ⛔ EACH ARM RUNS IN ITS OWN CHILD, AND THAT IS A BUG FIX, NOT TIDINESS.
 * The first version ran both arms in one process. On five of the eleven
 * environments arm A does not merely fail -- it ABORTS or takes SIGFPE inside
 * glibc's loader -- so the process died and arm B, the entire point of the
 * experiment, never executed. The run then looked like "B was not reached"
 * when the question asked was "does B work". Forking makes the arms
 * independent, so a crash in one is a datum rather than a missing row. */
static void try_load(const char *what, const char *path)
{
    fflush(stdout);
    pid_t pid = fork();
    if (pid == 0) {
        dlerror();
        void *h = dlopen(path, RTLD_NOW | RTLD_LOCAL);
        if (h) { printf("  %-8s LOADED   %s\n", what, path); fflush(stdout); _exit(0); }
        const char *e = dlerror();
        printf("  %-8s refused  %s\n           -> %s\n", what, path, e ? e : "(dlerror gave nothing)");
        fflush(stdout);
        _exit(1);
    }
    int status = 0;
    waitpid(pid, &status, 0);
    if (WIFSIGNALED(status))
        printf("  %-8s KILLED   %s\n           -> signal %d inside the loader, before any dlerror\n",
               what, path, WTERMSIG(status));
}

/* First regular .so under a set of directories the distributions actually use. */
static int find_host_so(char *out, size_t outsz)
{
    static const char *dirs[] = { "/usr/lib/x86_64-linux-gnu", "/usr/lib64",
                                  "/usr/lib", "/lib/x86_64-linux-gnu", "/lib64",
                                  "/lib", NULL };
    for (int i = 0; dirs[i]; i++) {
        DIR *d = opendir(dirs[i]);
        if (!d) continue;
        struct dirent *de;
        while ((de = readdir(d))) {
            if (!strstr(de->d_name, ".so")) continue;
            /* ⛔ Never pick libc or the loader: those are the objects
             * cross-libc-dlopen's own cld_never_touch list refuses, and
             * loading one would measure something else entirely. */
            if (strncmp(de->d_name, "libc.", 5) == 0) continue;
            if (strncmp(de->d_name, "ld-", 3) == 0) continue;
            if (strstr(de->d_name, "libpthread")) continue;
            char p[4096];
            snprintf(p, sizeof p, "%s/%s", dirs[i], de->d_name);
            struct stat st;
            if (stat(p, &st) == 0 && S_ISREG(st.st_mode) && st.st_size > 4096) {
                snprintf(out, outsz, "%s", p);
                closedir(d);
                return 1;
            }
        }
        closedir(d);
    }
    return 0;
}

int main(void)
{
    char host[4096];
    if (!find_host_so(host, sizeof host)) { printf("  no host shared object found\n"); return 0; }

    /* The rewrite is done FIRST, in the parent, so that arm A crashing a
     * child cannot prevent arm B from being attempted. */
    char copy[4096];
    snprintf(copy, sizeof copy, "/tmp/pgb-stripped-%ld.so", (long) getpid());
    int have_copy = strip_versions(host, copy);

    try_load("A plain", host);
    if (have_copy) { try_load("B strip", copy); unlink(copy); }
    else printf("  B strip  could not rewrite %s\n", host);
    return 0;
}
EOF

# Built through pgb, so this is the ACTUAL portable binary asking the question,
# not a differently-linked stand-in.
if ! sh "$REPO_DIR/pgb" --bind "$B" build -- /bin/sh -c \
      "\$CC -O2 -o '$B/plugin-probe' '$B/probe.c'" >"$B/build.log" 2>&1; then
  exp_note "build failed"; tail -5 "$B/build.log"; exit 2
fi
exp_check "probe built through pgb" yes yes

printf '\n  a static pgb binary trying to dlopen a host shared object:\n\n'
while read -r ref name libc digest; do
  case "$ref" in ''|\#*) continue ;; esac
  r=$(exp_rootfs "$name")
  [ -n "$r" ] || { exp_skip "$name" "not fetched"; continue; }
  printf '  == %s (%s)\n' "$name" "$libc"
  cp "$B/plugin-probe" "$r/pgb-plugin-probe"
  sh "$REPO_DIR/scripts/common/rootfs-run.sh" "$r" -- /pgb-plugin-probe 2>&1 \
    | sed 's/^/  /' | tee -a "$EXP_OUT/dlerror.txt"
  rm -f "$r/pgb-plugin-probe"
  printf '\n'
done < "$REPO_DIR/scripts/common/rootfs-images.txt"

exp_note "Read the -> lines. A 'version ... not found' is the class the"
exp_note "  cross-libc-dlopen rewrite removes. Anything naming libc.so.6, the"
exp_note "  loader, or static TLS is a class it does NOT remove, because that"
exp_note "  object needs a loader and a libc this process does not carry."
exp_finish
