/* pgb-storefix.c -- resolve a compiled-in /nix/store path against the bundle.
 *
 * ⛔ WHAT THIS IS FOR, AND IT IS MEASURED RATHER THAN SUPPOSED.
 * `experiments/64-` arm G runs a GTK 3 application out of a nixpkgs closure on
 * eleven environments. GTK loads, GTK connects to a real X server, zero host
 * shared objects are opened -- and no window ever appears, because the program
 * has its own store path compiled into `.rodata`:
 *
 *     [galculator] Couldn't load /nix/store/<hash>-galculator-2.1.4/share/
 *                  galculator/ui/main_frame.ui
 *
 * ⭐ THAT FILE IS IN THE BUNDLE. `XDG_DATA_DIRS` points at the bundle's own
 * `share` and serves every application that LOOKS UP its data; it cannot serve
 * one with the path baked in. Arm C proved the diagnosis by binding the AppDir
 * at that store path -- 0 of 11 became 11 of 11 -- but a bind needs root and a
 * mount namespace, which a user double-clicking an AppImage does not have.
 *
 * ⛔ AND THE OBVIOUS ALTERNATIVE IS REFUSED ON SECURITY GROUNDS.
 * `/nix/store/` is 11 bytes and so is `/tmp/.pgbs/`, so a same-length prefix
 * substitution inside the ELF needs no relocation and no patchelf. It is also
 * unshippable: a fixed, predictable path under a world-writable directory is
 * squattable by any local user, and the tree it would serve is a program's
 * `share/` -- GTK `.ui` files naming GModules, a `loaders.cache` naming shared
 * objects to dlopen. Whoever controls that directory controls what the victim
 * loads. docs/design/store-paths.md §2 has the full argument and the answer,
 * which is no.
 *
 * -- HOW IT WORKS ------------------------------------------------------------
 *
 * The bundle carries `.preload`, which `Anylinux-sharun` reads and hands to the
 * loader as `--preload` (its `read_preload`, src/utils.rs). So this object is
 * in the link map before libc, and every path-taking call the application and
 * its shared libraries make THROUGH THE PLT lands here first.
 *
 * ⛔ THE REWRITE IS EXACT-MATCH, NOT A PATTERN. `pgb` computes the closure, so
 * it knows the finite set of store paths that are in this bundle and writes it
 * to `.storemap`. A path whose store component is in the set is rewritten; a
 * path whose store component is NOT is passed through untouched and reported.
 * ⚠ That is the difference between this and the field's five-regex cascade,
 * whose last rule is "replace any remaining store path with /" -- a guess that
 * destroys any store-shaped string it finds, including inside data.
 *
 * -- ⛔ WHAT IT CANNOT DO ----------------------------------------------------
 *
 *   - a STATICALLY LINKED program has no PLT to win. Not covered.
 *   - a program issuing RAW SYSCALLS (a Go binary is the usual case) does not
 *     go through libc at all. Not covered.
 *   - glibc's own internal calls do not go through the PLT either, which is
 *     why fopen/opendir/realpath are interposed BY NAME rather than left to
 *     the syscall layer to catch.
 *
 * ⚠ Each of those is a REPORT at build time, not a silent failure at run time.
 *
 * -- WHY dlsym(RTLD_NEXT) AND NOT RAW SYSCALLS -------------------------------
 *
 * `fopen` returns a `FILE *` this file cannot construct, and `opendir` a
 * `DIR *`. Forwarding is the only correct implementation for those, so the
 * object links against libc -- and `pgb` then CHECKS, against the bundle's own
 * libc, that every versioned symbol it imports is defined there. A build host
 * with a newer glibc than the closure is a finding, not a silent breakage.
 *
 * SPDX-License-Identifier: MIT
 */
#define _GNU_SOURCE
#include <dirent.h>
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

#define STORE_PREFIX     "/nix/store/"
#define STORE_PREFIX_LEN (sizeof STORE_PREFIX - 1)

/* One row of .storemap: the store path's base name, and the AppDir-relative
 * directory that now holds its tree. */
struct row {
    char *base; /* "<32-char hash>-<name>-<version>" */
    int   blen;
    char *dir;  /* "store/<name>-<version>", relative to the AppDir */
};

static struct row *rows;
static int         nrows;
static char        appdir[PATH_MAX];
static int         appdir_len;
static int         ready;      /* 1 once the map has been read (or found absent) */
static int         debug_on;   /* PGB_STOREFIX_DEBUG=1 */

/* ⛔ A MISS IS REPORTED, NOT SUBSTITUTED. Reported once per distinct store
 * path, because a GUI application stats its data files in a loop and an
 * unbounded warning stream is how a real finding gets scrolled away. */
#define MISS_MAX 32
static char *misses[MISS_MAX];
static int   nmisses;

static void note_miss(const char *base, int blen)
{
    int i;
    if (!debug_on)
        return;
    for (i = 0; i < nmisses; i++)
        if ((int)strlen(misses[i]) == blen && memcmp(misses[i], base, blen) == 0)
            return;
    if (nmisses < MISS_MAX) {
        char *copy = malloc(blen + 1);
        if (copy) {
            memcpy(copy, base, blen);
            copy[blen] = '\0';
            misses[nmisses++] = copy;
        }
    }
    fprintf(stderr, "pgb-storefix: NOT IN THIS BUNDLE: %s%.*s\n",
            STORE_PREFIX, blen, base);
}

/* where_am_i finds the AppDir. SHARUN_DIR is what sharun exports and is the
 * cheap answer; dladdr on this object is the one that still works when a child
 * process was started without it. */
static void where_am_i(void)
{
    const char *env = getenv("SHARUN_DIR");
    Dl_info info;
    char *slash;

    if (!env || !*env)
        env = getenv("APPDIR");
    if (env && *env) {
        snprintf(appdir, sizeof appdir, "%s", env);
        appdir_len = (int)strlen(appdir);
        return;
    }
    /* <appdir>/lib/libpgb-storefix.so -> <appdir> */
    if (dladdr((void *)where_am_i, &info) && info.dli_fname) {
        snprintf(appdir, sizeof appdir, "%s", info.dli_fname);
        slash = strrchr(appdir, '/');
        if (slash) {
            *slash = '\0';
            slash = strrchr(appdir, '/');
            if (slash) {
                *slash = '\0';
                appdir_len = (int)strlen(appdir);
                return;
            }
        }
    }
    appdir[0] = '\0';
    appdir_len = 0;
}

static void load_map(void)
{
    char path[PATH_MAX];
    char line[PATH_MAX * 2];
    FILE *f;
    int cap = 0;

    ready = 1;
    debug_on = getenv("PGB_STOREFIX_DEBUG") != NULL;
    where_am_i();
    if (!appdir_len)
        return;
    if (snprintf(path, sizeof path, "%s/.storemap", appdir) >= (int)sizeof path)
        return;
    /* ⚠ fopen HERE IS OUR OWN INTERPOSER'S fopen. It is safe because `ready`
     * is already 1, so the wrapper forwards without recursing. */
    f = fopen(path, "re");
    if (!f) {
        if (debug_on)
            fprintf(stderr, "pgb-storefix: no %s; nothing is rewritten\n", path);
        return;
    }
    while (fgets(line, sizeof line, f)) {
        char *tab, *nl;
        struct row r;
        if (line[0] == '#' || line[0] == '\n')
            continue;
        nl = strchr(line, '\n');
        if (nl)
            *nl = '\0';
        tab = strchr(line, '\t');
        if (!tab)
            continue;
        *tab = '\0';
        r.blen = (int)strlen(line);
        r.base = strdup(line);
        r.dir  = strdup(tab + 1);
        if (!r.base || !r.dir)
            break;
        if (nrows == cap) {
            int ncap = cap ? cap * 2 : 64;
            struct row *grown = realloc(rows, (size_t)ncap * sizeof *rows);
            if (!grown)
                break;
            rows = grown;
            cap = ncap;
        }
        rows[nrows++] = r;
    }
    fclose(f);
    if (debug_on)
        fprintf(stderr, "pgb-storefix: %d store path(s), AppDir %s\n", nrows, appdir);
}

__attribute__((constructor)) static void pgb_storefix_init(void)
{
    if (!ready)
        load_map();
}

/* fix rewrites one path, or returns it unchanged.
 *
 * ⛔ IT IS A PURE SUBSTITUTION AND IT SEARCHES NOTHING. `pgb` decided at build
 * time which directory holds each store path's tree; asking the filesystem
 * here would turn an exact answer into a guess, and a guess that happens to
 * hit is indistinguishable from one that happens to miss. */
static const char *fix(const char *p, char *buf, size_t bufsz)
{
    const char *rest;
    int blen, i;

    if (!p || p[0] != '/' || strncmp(p, STORE_PREFIX, STORE_PREFIX_LEN) != 0)
        return p;
    if (!ready)
        load_map();
    if (!nrows || !appdir_len)
        return p;

    p += STORE_PREFIX_LEN;
    rest = strchr(p, '/');
    blen = rest ? (int)(rest - p) : (int)strlen(p);
    if (!rest)
        rest = "";

    for (i = 0; i < nrows; i++) {
        if (rows[i].blen != blen || memcmp(rows[i].base, p, (size_t)blen) != 0)
            continue;
        if ((size_t)snprintf(buf, bufsz, "%s/%s%s", appdir, rows[i].dir, rest) >= bufsz)
            return p - STORE_PREFIX_LEN;
        if (debug_on)
            fprintf(stderr, "pgb-storefix: %s%.*s%s -> %s\n",
                    STORE_PREFIX, blen, p, rest, buf);
        return buf;
    }
    note_miss(p, blen);
    return p - STORE_PREFIX_LEN;
}

/* ⚠ EVERY FORWARD IS RESOLVED LAZILY, not in the constructor. A preloaded
 * object's constructor is not guaranteed to have run before every call it will
 * ever see -- libc's own initialisation opens files -- so nothing here may
 * assume it has. */
#define FIXED(p) char _b[PATH_MAX * 2]; const char *_p = fix((p), _b, sizeof _b)

/* -- open ---------------------------------------------------------------- */

typedef int (*open_fn)(const char *, int, ...);

static int do_open(const char *name, const char *path, int flags, mode_t mode)
{
    static open_fn cached_open, cached_open64;
    open_fn *slot = name[4] == '6' ? &cached_open64 : &cached_open;
    char b[PATH_MAX * 2];
    const char *p = fix(path, b, sizeof b);
    if (!*slot)
        *slot = (open_fn)dlsym(RTLD_NEXT, name);
    if (!*slot) {
        errno = ENOSYS;
        return -1;
    }
    return (*slot)(p, flags, mode);
}

int open(const char *path, int flags, ...)
{
    mode_t mode = 0;
    if (flags & (O_CREAT | O_TMPFILE)) {
        va_list ap;
        va_start(ap, flags);
        mode = (mode_t)va_arg(ap, int);
        va_end(ap);
    }
    return do_open("open", path, flags, mode);
}

int open64(const char *path, int flags, ...)
{
    mode_t mode = 0;
    if (flags & (O_CREAT | O_TMPFILE)) {
        va_list ap;
        va_start(ap, flags);
        mode = (mode_t)va_arg(ap, int);
        va_end(ap);
    }
    return do_open("open64", path, flags, mode);
}

typedef int (*openat_fn)(int, const char *, int, ...);

static int do_openat(const char *name, int dirfd, const char *path, int flags, mode_t mode)
{
    static openat_fn cached_openat, cached_openat64;
    openat_fn *slot = name[6] == '6' ? &cached_openat64 : &cached_openat;
    char b[PATH_MAX * 2];
    const char *p = fix(path, b, sizeof b);
    if (!*slot)
        *slot = (openat_fn)dlsym(RTLD_NEXT, name);
    if (!*slot) {
        errno = ENOSYS;
        return -1;
    }
    return (*slot)(dirfd, p, flags, mode);
}

int openat(int dirfd, const char *path, int flags, ...)
{
    mode_t mode = 0;
    if (flags & (O_CREAT | O_TMPFILE)) {
        va_list ap;
        va_start(ap, flags);
        mode = (mode_t)va_arg(ap, int);
        va_end(ap);
    }
    return do_openat("openat", dirfd, path, flags, mode);
}

int openat64(int dirfd, const char *path, int flags, ...)
{
    mode_t mode = 0;
    if (flags & (O_CREAT | O_TMPFILE)) {
        va_list ap;
        va_start(ap, flags);
        mode = (mode_t)va_arg(ap, int);
        va_end(ap);
    }
    return do_openat("openat64", dirfd, path, flags, mode);
}

/* -- stdio --------------------------------------------------------------- */

FILE *fopen(const char *path, const char *mode)
{
    typedef FILE *(*fn_t)(const char *, const char *);
    static fn_t real;
    FIXED(path);
    if (!real)
        real = (fn_t)dlsym(RTLD_NEXT, "fopen");
    if (!real) {
        errno = ENOSYS;
        return NULL;
    }
    return real(_p, mode);
}

FILE *fopen64(const char *path, const char *mode)
{
    typedef FILE *(*fn_t)(const char *, const char *);
    static fn_t real;
    FIXED(path);
    if (!real)
        real = (fn_t)dlsym(RTLD_NEXT, "fopen64");
    if (!real)
        return fopen(_p, mode);
    return real(_p, mode);
}

FILE *freopen(const char *path, const char *mode, FILE *stream)
{
    typedef FILE *(*fn_t)(const char *, const char *, FILE *);
    static fn_t real;
    FIXED(path);
    if (!real)
        real = (fn_t)dlsym(RTLD_NEXT, "freopen");
    if (!real) {
        errno = ENOSYS;
        return NULL;
    }
    return real(_p, mode, stream);
}

/* -- stat ---------------------------------------------------------------- */

int stat(const char *path, struct stat *st)
{
    typedef int (*fn_t)(const char *, struct stat *);
    static fn_t real;
    FIXED(path);
    if (!real)
        real = (fn_t)dlsym(RTLD_NEXT, "stat");
    if (!real)
        return fstatat(AT_FDCWD, _p, st, 0);
    return real(_p, st);
}

int lstat(const char *path, struct stat *st)
{
    typedef int (*fn_t)(const char *, struct stat *);
    static fn_t real;
    FIXED(path);
    if (!real)
        real = (fn_t)dlsym(RTLD_NEXT, "lstat");
    if (!real)
        return fstatat(AT_FDCWD, _p, st, AT_SYMLINK_NOFOLLOW);
    return real(_p, st);
}

int fstatat(int dirfd, const char *path, struct stat *st, int flags)
{
    typedef int (*fn_t)(int, const char *, struct stat *, int);
    static fn_t real;
    FIXED(path);
    if (!real)
        real = (fn_t)dlsym(RTLD_NEXT, "fstatat");
    if (!real) {
        errno = ENOSYS;
        return -1;
    }
    return real(dirfd, _p, st, flags);
}

/* ⚠ THE __xstat FAMILY IS glibc BEFORE 2.33 AND IS NOT DEAD CODE HERE. The
 * bundle's libc decides which of the two shapes its own libraries call, not
 * the host this object was compiled on, so both are defined and each forwards
 * to whatever the bundle actually has. */
int __xstat(int ver, const char *path, struct stat *st)
{
    typedef int (*fn_t)(int, const char *, struct stat *);
    static fn_t real;
    FIXED(path);
    if (!real)
        real = (fn_t)dlsym(RTLD_NEXT, "__xstat");
    if (!real)
        return fstatat(AT_FDCWD, _p, st, 0);
    return real(ver, _p, st);
}

int __lxstat(int ver, const char *path, struct stat *st)
{
    typedef int (*fn_t)(int, const char *, struct stat *);
    static fn_t real;
    FIXED(path);
    if (!real)
        real = (fn_t)dlsym(RTLD_NEXT, "__lxstat");
    if (!real)
        return fstatat(AT_FDCWD, _p, st, AT_SYMLINK_NOFOLLOW);
    return real(ver, _p, st);
}

int __fxstatat(int ver, int dirfd, const char *path, struct stat *st, int flags)
{
    typedef int (*fn_t)(int, int, const char *, struct stat *, int);
    static fn_t real;
    FIXED(path);
    if (!real)
        real = (fn_t)dlsym(RTLD_NEXT, "__fxstatat");
    if (!real)
        return fstatat(dirfd, _p, st, flags);
    return real(ver, dirfd, _p, st, flags);
}

int statx(int dirfd, const char *restrict path, int flags, unsigned int mask,
          struct statx *restrict stx)
{
    typedef int (*fn_t)(int, const char *, int, unsigned int, struct statx *);
    static fn_t real;
    FIXED(path);
    if (!real)
        real = (fn_t)dlsym(RTLD_NEXT, "statx");
    if (!real) {
        errno = ENOSYS;
        return -1;
    }
    return real(dirfd, _p, flags, mask, stx);
}

/* -- access, directories, links ------------------------------------------ */

int access(const char *path, int mode)
{
    typedef int (*fn_t)(const char *, int);
    static fn_t real;
    FIXED(path);
    if (!real)
        real = (fn_t)dlsym(RTLD_NEXT, "access");
    if (!real) {
        errno = ENOSYS;
        return -1;
    }
    return real(_p, mode);
}

int faccessat(int dirfd, const char *path, int mode, int flags)
{
    typedef int (*fn_t)(int, const char *, int, int);
    static fn_t real;
    FIXED(path);
    if (!real)
        real = (fn_t)dlsym(RTLD_NEXT, "faccessat");
    if (!real) {
        errno = ENOSYS;
        return -1;
    }
    return real(dirfd, _p, mode, flags);
}

DIR *opendir(const char *path)
{
    typedef DIR *(*fn_t)(const char *);
    static fn_t real;
    FIXED(path);
    if (!real)
        real = (fn_t)dlsym(RTLD_NEXT, "opendir");
    if (!real) {
        errno = ENOSYS;
        return NULL;
    }
    return real(_p);
}

ssize_t readlink(const char *path, char *buf, size_t sz)
{
    typedef ssize_t (*fn_t)(const char *, char *, size_t);
    static fn_t real;
    FIXED(path);
    if (!real)
        real = (fn_t)dlsym(RTLD_NEXT, "readlink");
    if (!real) {
        errno = ENOSYS;
        return -1;
    }
    return real(_p, buf, sz);
}

char *realpath(const char *path, char *out)
{
    typedef char *(*fn_t)(const char *, char *);
    static fn_t real;
    FIXED(path);
    if (!real)
        real = (fn_t)dlsym(RTLD_NEXT, "realpath");
    if (!real) {
        errno = ENOSYS;
        return NULL;
    }
    return real(_p, out);
}

/* -- exec and dlopen ------------------------------------------------------ */

int execve(const char *path, char *const argv[], char *const envp[])
{
    typedef int (*fn_t)(const char *, char *const[], char *const[]);
    static fn_t real;
    FIXED(path);
    if (!real)
        real = (fn_t)dlsym(RTLD_NEXT, "execve");
    if (!real) {
        errno = ENOSYS;
        return -1;
    }
    return real(_p, argv, envp);
}

int execv(const char *path, char *const argv[])
{
    extern char **environ;
    return execve(path, argv, environ);
}

/* ⭐ dlopen IS THE ONE THAT IS NOT ABOUT DATA FILES. A gdk-pixbuf
 * `loaders.cache` names its loaders by absolute store path, and libglvnd's
 * vendor JSON does the same for the GL implementation. Those are rewritten at
 * build time where the file is ours to rewrite; this catches the ones assembled
 * at run time. */
void *dlopen(const char *path, int flags)
{
    typedef void *(*fn_t)(const char *, int);
    static fn_t real;
    char b[PATH_MAX * 2];
    const char *p = path ? fix(path, b, sizeof b) : NULL;
    if (!real)
        real = (fn_t)dlsym(RTLD_NEXT, "dlopen");
    if (!real)
        return NULL;
    return real(p, flags);
}
