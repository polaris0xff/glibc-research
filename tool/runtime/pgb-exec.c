/* pgb-exec.c -- a script entry point, as a STATIC ELF the bundle can start.
 *
 * ⛔ WHY A BUNDLE NEEDS THIS AT ALL, AND IT IS NOT A meld QUIRK.
 * nixpkgs wraps a Python program as a `makeBinaryWrapper` ELF whose target is
 * a Python SCRIPT. `sharun` reads `shared/bin/<name>` as an ELF and exits when
 * it is not one, so a script can never be a bundle's entry point — and
 * `internal/bundle/appimage.go`'s reader used to oscillate between the two
 * shapes for five hops and then report `no entry point`. NO Python GUI
 * application bundled at all. That is the standard nixpkgs shape.
 *
 * ⭐ WHAT RESOLVES IT: a script entry point is not one file, it is
 * INTERPRETER + SCRIPT ARGUMENT. This trampoline is what turns the pair back
 * into the single ELF the layout requires.
 *
 *   PGB_EXEC_INTERP   the interpreter's name in shared/bin, e.g. "python3.14"
 *   PGB_EXEC_SCRIPT   the script, relative to the AppDir
 *
 * ⚠ IT EXECS `<appdir>/bin/<interp>`, WHICH IS A HARDLINK OF sharun, not the
 * ELF in shared/bin. sharun is what runs the bundled loader with the bundled
 * library path; going straight to the interpreter would have it look for its
 * libraries on the host. Same rule, same reason, as pgb-apprun.c.
 *
 * ⚠ AND IT IS STATIC ON PURPOSE. sharun execs a static binary DIRECTLY
 * (`src/main.rs`, `if is_static_bin`), so this file needs no loader
 * cooperation and cannot drag a host libc in — which a `#!/bin/sh` wrapper
 * would, measured at 1-4 host shared objects per glibc row in experiments/90-.
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

#ifndef PGB_EXEC_INTERP
#define PGB_EXEC_INTERP ""
#endif
#ifndef PGB_EXEC_SCRIPT
#define PGB_EXEC_SCRIPT ""
#endif

int main(int argc, char **argv)
{
    char exe[PATH_MAX], appdir[PATH_MAX], interp[PATH_MAX], script[PATH_MAX];
    char **out;
    ssize_t n;
    char *slash;
    int i;

    /* <appdir>/shared/bin/<prog> -> <appdir>. Taken from /proc/self/exe and
     * not from argv[0]: uruntime sets ARGV0 to the AppImage's own path, which
     * is not inside the mount at all. */
    n = readlink("/proc/self/exe", exe, sizeof exe - 1);
    if (n < 0) {
        fprintf(stderr, "pgb-exec: cannot read /proc/self/exe: %s\n", strerror(errno));
        return 127;
    }
    exe[n] = '\0';
    snprintf(appdir, sizeof appdir, "%s", exe);
    for (i = 0; i < 3; i++) {
        slash = strrchr(appdir, '/');
        if (!slash) {
            fprintf(stderr, "pgb-exec: %s is not inside an AppDir\n", exe);
            return 127;
        }
        *slash = '\0';
    }

    if (!*PGB_EXEC_INTERP || !*PGB_EXEC_SCRIPT) {
        fprintf(stderr, "pgb-exec: built with no interpreter or no script\n");
        return 127;
    }
    if (snprintf(interp, sizeof interp, "%s/bin/%s", appdir, PGB_EXEC_INTERP) >= (int)sizeof interp ||
        snprintf(script, sizeof script, "%s/%s", appdir, PGB_EXEC_SCRIPT) >= (int)sizeof script) {
        fprintf(stderr, "pgb-exec: path too long\n");
        return 127;
    }

    out = calloc((size_t)argc + 3, sizeof *out);
    if (!out) {
        fprintf(stderr, "pgb-exec: out of memory\n");
        return 127;
    }
    /* ⛔ argv[0] IS THE FULL PATH. sharun derives its own directory from
     * argv[0] as well as from /proc/self/exe, and handing it a bare name is
     * half of the `Failed to find ARG0 dir!` failure pgb-apprun.c records. */
    out[0] = interp;
    out[1] = script;
    for (i = 1; i < argc; i++)
        out[i + 1] = argv[i];
    out[argc + 1] = NULL;

    setenv("APPDIR", appdir, 1);
    setenv("SHARUN_DIR", appdir, 1);
    execv(interp, out);
    fprintf(stderr, "pgb-exec: cannot exec %s: %s\n", interp, strerror(errno));
    return 127;
}
