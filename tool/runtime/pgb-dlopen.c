/* pgb-dlopen.c -- let a static binary load its OWN plugins, with no loader.
 *
 * THE PROBLEM, MEASURED IN experiments/50-host-plugin-feasibility.sh
 * -------------------------------------------------------------------------
 * A static glibc binary has no dynamic loader of its own, so dlopen() borrows
 * the HOST's ld.so and libc.so.6 -- and that pairing is what breaks. Across
 * the eleven pinned environments, dlopen of a shared object succeeds on two
 * and dies inside glibc's loader on the rest:
 *
 *   dl-call-libc-early-init.c:37  sym != NULL   Debian 11, Ubuntu 20.04
 *   dl-machine.h:487              assertion     Rocky 8
 *   signal 8                                    openSUSE Leap, Fedora 42
 *
 * ⛔ AND SUCCESS IS THE WORSE OUTCOME. Where it loads, a second libc and the
 * host loader are now in the process, which is the exact thing this project
 * exists to keep out. docs/limitations.md §1.
 *
 * THE SPLIT THAT MAKES THIS TRACTABLE
 * -------------------------------------------------------------------------
 * ⭐ Two different problems wear the same word. Loading a HOST plugin --
 * libnss_ldap, a distribution's GTK module -- means running code that was
 * built somewhere else against a libc that is not ours, and that is the hard
 * one. Loading the program's OWN plugins, shipped by the same build, is not
 * hard at all: the code is already available at link time. The only thing
 * dlopen is providing there is a NAME-TO-FUNCTION LOOKUP, and a lookup does
 * not need a loader.
 *
 * So this file answers dlopen/dlsym/dlclose/dlerror out of a table the build
 * compiled in. Nothing is mapped, nothing is relocated, no ld.so is consulted
 * and no second libc can enter. The plugins are ordinary objects in the link.
 *
 * PRIOR ART, AND WHAT WAS TAKEN FROM IT
 * -------------------------------------------------------------------------
 * references/allyourcodebase__pipewire/tree/src/wrap/dlfcn.zig does exactly
 * this for PipeWire's two plugin systems, by hand, in Zig. Taken from it:
 * the handle-is-a-pointer-to-the-table-entry representation, the no-op
 * dlclose, the insistence that dlerror is part of the interface and not an
 * extra, and its comptime assertion that no handle collides with RTLD_DEFAULT
 * or RTLD_NEXT -- reproduced here as a runtime check, since C cannot make it
 * at compile time.
 *
 * ⭐ WHAT IS DIFFERENT: that table is hand-written for one program. `pgb`
 * GENERATES this one with `nm` from the objects the build produced, which is
 * what turns a PipeWire-specific patch into a mechanism. POC 50 links
 * CPython's 49 extension modules in by hand through CPython's own
 * Modules/Setup.local; this is that, for programs with no such mechanism.
 *
 * ⭐ THE REDIRECTION IS A LINK-TIME ONE, SO NO SOURCE CHANGES, exactly as in
 * pgb-iconv.c: -Wl,--wrap=dlopen,... makes ld rewrite every UNDEFINED
 * reference to those names into __wrap_*, so calls from the application, from
 * any static library in the link, and from libraries compiled before this
 * tool existed are all caught, and none of them has to have seen a pgb header.
 *
 * ⭐ WHY THIS FILE IS IN AN ARCHIVE. An archive member is pulled in only when
 * something references a symbol it defines. A program that never calls dlopen
 * links none of this. Same reasoning as pgb-iconv.c.
 *
 * ⚠ WHAT THIS DOES NOT DO, stated rather than discovered later:
 *
 *   - it does not load a HOST plugin, and it is not trying to. That is
 *     docs/AGENTS.md §13 item 4, routes B and C.
 *   - dlopen(NULL), "a handle for the main program", cannot be answered from
 *     a generated table unless the build put a "@SELF" entry in it, because
 *     nothing here can enumerate the executable's own symbols. Unmatched, it
 *     reports so through dlerror() rather than returning a handle whose
 *     dlsym silently answers NULL for everything.
 *   - RTLD_DEFAULT and RTLD_NEXT are a search order over loaded objects.
 *     There is no such order here. They are reported through dlerror(),
 *     never faked.
 *   - dlvsym(), dlinfo() and dladdr() are not wrapped. A program that calls
 *     them keeps whatever glibc's static build does with them, which is the
 *     pre-existing behaviour rather than something this file changed.
 *
 * SPDX-License-Identifier: MIT
 */

#include "pgb-dlopen.h"

#include <string.h>

/* dlfcn.h is not included: this file must compile in an environment whose
 * dlfcn.h has already been seen by the wrapped program, and the only two
 * values needed are ABI constants that have never changed on Linux. */
#define PGB_RTLD_DEFAULT ((void *)0)
#define PGB_RTLD_NEXT    ((void *)-1L)

/* ⛔ dlerror() IS PART OF THE INTERFACE, NOT A COURTESY. dlopen is allowed to
 * return NULL for reasons a caller must be able to distinguish, and a program
 * that gets NULL with no message has been handed a silent failure. Every
 * failure path below sets this. */
static const char *pgb_dl_err;

/* Cleared by a read, as dlerror() is specified to do. */
char *__wrap_dlerror(void)
{
    const char *e = pgb_dl_err;
    pgb_dl_err = NULL;
    return (char *)e;
}

static const char *pgb_basename(const char *p)
{
    const char *s = strrchr(p, '/');
    return s ? s + 1 : p;
}

/* ⚠ THE MATCHING RULE IS NOT PLAIN EQUALITY, AND IT CANNOT BE.
 *
 * A program reaches its plugins by a path it built at run time -- from a
 * configure-time PKGLIBDIR, from an environment variable, from a directory
 * scan. The generated table knows what the BUILD produced. Requiring the two
 * strings to be equal would mean the table only ever matched programs that
 * hard-code a bare soname, which is the minority.
 *
 * So: exact match first, because a table entry written as a full path should
 * beat a basename collision; then basename, which is what actually fires for
 * `dlopen("/usr/lib/myapp/plugins/foo.so")` against a table built from
 * `foo.so`.
 *
 * ⛔ Basename matching is deliberately NOT extended to stripping "lib" or a
 * version suffix. `foo.so` and `foo.so.2` are different objects to the
 * program that named one of them, and a lookup that quietly returned the
 * other would be a silent wrong answer -- the failure mode this whole project
 * is about.
 */
static const struct pgb_dl_lib *pgb_find(const char *path)
{
    const struct pgb_dl_lib *l;

    if (&pgb_dlopen_libs[0] == NULL)
        return NULL;

    for (l = pgb_dlopen_libs; l->name; l++)
        if (strcmp(l->name, path) == 0)
            return l;

    {
        const char *base = pgb_basename(path);
        for (l = pgb_dlopen_libs; l->name; l++)
            if (strcmp(pgb_basename(l->name), base) == 0)
                return l;
    }
    return NULL;
}

void *__wrap_dlopen(const char *path, int flags)
{
    const struct pgb_dl_lib *l;

    (void)flags;   /* RTLD_LAZY/NOW/GLOBAL/LOCAL describe work that is already
                    * done: the code is in the executable and was bound by the
                    * static link. There is nothing left to defer or scope. */

    pgb_dl_err = NULL;

    if (&pgb_dlopen_libs[0] == NULL) {
        pgb_dl_err = "pgb: no plugin table was compiled in "
                     "(build with --wrap-dlopen)";
        return NULL;
    }

    if (path == NULL) {
        /* Only answerable if the build named an entry "@SELF". */
        l = pgb_find("@SELF");
        if (l == NULL) {
            pgb_dl_err = "pgb: dlopen(NULL) needs a \"@SELF\" entry in the "
                         "compiled-in table; this build has none";
            return NULL;
        }
    } else {
        l = pgb_find(path);
        if (l == NULL) {
            pgb_dl_err = "pgb: no such plugin in the compiled-in table";
            return NULL;
        }
    }

    /* ⛔ A HANDLE MUST NOT COLLIDE WITH A SENTINEL. pipewire's dlfcn.zig
     * asserts this at comptime; C cannot, so it is checked here. In practice
     * a table entry never lands at address 0 or -1, but "in practice" is how
     * a silent wrong answer gets shipped: dlsym would take the collision for
     * RTLD_DEFAULT and search the wrong thing. */
    if ((const void *)l == PGB_RTLD_DEFAULT || (const void *)l == PGB_RTLD_NEXT) {
        pgb_dl_err = "pgb: plugin table entry collides with an RTLD sentinel";
        return NULL;
    }

    return (void *)(size_t)l;
}

void *__wrap_dlsym(void *handle, const char *name)
{
    const struct pgb_dl_lib *l;
    const struct pgb_dl_sym *s;

    pgb_dl_err = NULL;

    if (handle == PGB_RTLD_DEFAULT || handle == PGB_RTLD_NEXT) {
        /* ⚠ Reported, never faked. Both are a search ORDER over loaded
         * objects and this binary has no such order -- everything is already
         * one image. Answering from the table would be a guess dressed as a
         * result. */
        pgb_dl_err = "pgb: RTLD_DEFAULT/RTLD_NEXT are not available in a "
                     "statically linked image";
        return NULL;
    }

    if (handle == NULL) {
        pgb_dl_err = "pgb: dlsym on a NULL handle";
        return NULL;
    }

    l = (const struct pgb_dl_lib *)handle;
    if (l->syms == NULL) {
        pgb_dl_err = "pgb: plugin has no symbol table";
        return NULL;
    }

    for (s = l->syms; s->name; s++)
        if (strcmp(s->name, name) == 0)
            return s->addr;

    pgb_dl_err = "pgb: symbol not found in the compiled-in table";
    return NULL;
}

int __wrap_dlclose(void *handle)
{
    /* Nothing was mapped, so nothing is unmapped. Returning 0 is not a lie:
     * dlclose's contract is that the caller may no longer use the handle,
     * and it may not. */
    (void)handle;
    pgb_dl_err = NULL;
    return 0;
}
