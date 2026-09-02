/* pgb-apprun.c -- the multi-program selector, as a STATIC executable.
 *
 * ⛔ WHY THIS IS NOT A SHELL SCRIPT, AND THE MEASUREMENT THAT FORCED IT.
 * A bundle carrying several programs needs something to choose between them.
 * The obvious answer -- and the one `pkgforge-dev/Anylinux-AppImages` uses in
 * `useful-tools/AppRun.sh` -- is a shell script. But a script is run by the
 * HOST's interpreter, and that interpreter loads the HOST's libc:
 *
 *   experiments/90-, kdenlive: 1-4 host shared objects on every glibc row,
 *   none on the four musl ones. The competitor, whose AppRun is also a shell
 *   script, opened 10 on Rocky 8.
 *
 * ⭐ `QaidVoid/onelf` does not pay that: its `pack --entrypoint name=path`
 * dispatches from a runtime stub that is a static musl binary. This file is
 * the same idea reached from this project's own side -- a static ELF, built by
 * the toolchain this repository exists to build, with no interpreter and no
 * DT_NEEDED. ⭐ The shebang could never have pointed into the bundle anyway:
 * the mount path is not known until run time.
 *
 * The rule, in order, matching Anylinux's AppRun.sh so behaviour is familiar:
 *
 *   1. argv[0]'s basename names a program in shared/bin  -> run it
 *   2. argv[1] names a program in shared/bin             -> run it, drop argv[1]
 *   3. otherwise                                          -> run the default
 *
 * ⚠ It execs `<appdir>/bin/<name>`, which is a HARDLINK OF sharun, not the ELF
 * in shared/bin. sharun is what sets the library path and runs the bundled
 * loader; going straight to the payload would skip all of that and the binary
 * would look for its libraries on the host.
 *
 * ⚠ $APPDIR is taken from /proc/self/exe, not from argv[0] and not from the
 * environment: uruntime sets ARGV0 to the AppImage's own path, which is not
 * inside the mount at all.
 *
 * SPDX-License-Identifier: MIT
 */
#define _GNU_SOURCE
#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/stat.h>

#ifndef PGB_APPRUN_DEFAULT
#define PGB_APPRUN_DEFAULT ""
#endif

static const char *base_of(const char *p)
{
    const char *s = strrchr(p, '/');
    return s ? s + 1 : p;
}

/* shared/bin/<name> is the payload; bin/<name> is the sharun hardlink that
 * knows how to start it. Both must exist for a name to be a program. */
static int is_program(const char *appdir, const char *name)
{
    char p[PATH_MAX];
    struct stat st;
    if (!name || !*name || strchr(name, '/'))
        return 0;
    if (snprintf(p, sizeof p, "%s/shared/bin/%s", appdir, name) >= (int)sizeof p)
        return 0;
    if (stat(p, &st) != 0 || !S_ISREG(st.st_mode))
        return 0;
    if (snprintf(p, sizeof p, "%s/bin/%s", appdir, name) >= (int)sizeof p)
        return 0;
    return stat(p, &st) == 0;
}

int main(int argc, char **argv)
{
    char exe[PATH_MAX], appdir[PATH_MAX], target[PATH_MAX];
    const char *argv0 = getenv("ARGV0");
    const char *want = NULL;
    ssize_t n;
    char *slash;

    n = readlink("/proc/self/exe", exe, sizeof exe - 1);
    if (n < 0) {
        fprintf(stderr, "pgb-apprun: cannot read /proc/self/exe: %s\n",
                strerror(errno));
        return 127;
    }
    exe[n] = '\0';
    snprintf(appdir, sizeof appdir, "%s", exe);
    slash = strrchr(appdir, '/');
    if (!slash) {
        fprintf(stderr, "pgb-apprun: /proc/self/exe has no directory: %s\n", exe);
        return 127;
    }
    *slash = '\0';
    /* ⛔ ARGV0 is consumed here and unset for the child: sharun reads it too,
     * and leaving the AppImage's own path in it makes sharun look for a
     * program named after the image file. */
    if (argv0 && is_program(appdir, base_of(argv0)))
        want = base_of(argv0);
    unsetenv("ARGV0");

    if (!want && argc > 1 && is_program(appdir, argv[1])) {
        want = argv[1];
        argv++;
        argc--;
    }
    if (!want && argv[0] && is_program(appdir, base_of(argv[0])))
        want = base_of(argv[0]);
    if (!want)
        want = PGB_APPRUN_DEFAULT;

    if (!*want) {
        fprintf(stderr, "pgb-apprun: this bundle has no default program\n");
        return 127;
    }
    if (snprintf(target, sizeof target, "%s/bin/%s", appdir, want) >= (int)sizeof target) {
        fprintf(stderr, "pgb-apprun: path too long\n");
        return 127;
    }
    setenv("APPDIR", appdir, 1);
    argv[0] = (char *)want;
    execv(target, argv);
    fprintf(stderr, "pgb-apprun: cannot exec %s: %s\n", target, strerror(errno));
    return 127;
}
