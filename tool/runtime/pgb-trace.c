/* pgb-trace.c -- a tracer small enough to carry into the thing being measured.
 *
 * THE PROBLEM
 * -------------------------------------------------------------------------
 * `pgb verify` decides docs/AGENTS.md §3 criterion 2 -- "loads no host shared
 * object" -- from a syscall trace. Under the chroot engine that trace is
 * taken by `strace` running OUTSIDE the target, and it works because the
 * target is an ordinary child process of the runner.
 *
 * Under the docker engine it is not. The subject runs inside the daemon's
 * namespaces, where a tracer on the build host cannot follow it, so that
 * column read `unmeasured` -- honest, and useless. And `strace` cannot simply
 * be installed in the eleven target images: they are pinned by digest and
 * adding a package to one changes what every result about it describes.
 *
 * ⭐ THE ANSWER IS THE PROJECT'S OWN. A tracer that must run on eleven
 * unknown userlands with nothing installed is precisely the artefact `pgb`
 * exists to produce, so this one is built by `pgb`, is a static ELF, and is
 * CARRIED IN beside the subject. experiments/70- measured that a carried-in
 * helper runs on 12 of 12 targets, which is what makes this viable at all.
 *
 * WHAT IT DOES, AND WHAT IT DELIBERATELY DOES NOT
 * -------------------------------------------------------------------------
 * It forks, PTRACE_TRACEME's, execs the subject, and reports every path the
 * subject OPENED SUCCESSFULLY through open/openat. That is all criterion 2
 * needs.
 *
 * ⛔ SUCCESSFULLY is the load-bearing word. It stops at both the entry and the
 * exit of each syscall: the path is only readable at entry, the result only at
 * exit, and reporting on entry alone counts every path the program merely
 * probed for. glibc probes several paths for a shared object and takes the
 * first that answers, so on the criterion-2 column that is a false positive --
 * a binary that loaded nothing failed for the paths that did not answer.
 *
 * ⚠ IT DOES NOT FOLLOW FORKS, and that is a decision rather than an omission.
 * The subject of `pgb verify` is a single statically linked ELF, and a
 * `dlopen` happens IN that process -- it cannot be delegated to a child. A
 * fork-follower is what a BUNDLE format needs, because a bundle extracts and
 * re-execs its payload elsewhere, and getting that wrong is recorded twice in
 * docs/history/corrections.md. This tracer is not for bundles, and it says so
 * rather than pretending to a generality it has not been tested for.
 *
 * ⛔ IT REPORTS WHAT IT SAW, AND SEPARATELY WHETHER IT SAW ANYTHING. A tracer
 * that fails to attach and prints no paths is indistinguishable, in its
 * output, from one that attached and found the binary clean. So it prints a
 * final status line, and the caller keys on that rather than on emptiness.
 *
 * Usage:  pgb-trace -- PROGRAM [ARGS...]
 * Output, on stderr, one per line:
 *          open <path>
 *          pgb-trace: status=<traced|failed> opens=<n> exit=<code>|signal=<n>
 * The subject's own exit status is this program's exit status, so a caller
 * still reads criterion 1 from it.
 *
 * SPDX-License-Identifier: MIT
 */

#define _GNU_SOURCE
#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ptrace.h>
#include <sys/types.h>
#include <sys/user.h>
#include <sys/wait.h>
#include <unistd.h>

#ifndef PTRACE_O_EXITKILL
#define PTRACE_O_EXITKILL 0x00100000
#endif
#ifndef PTRACE_O_TRACESYSGOOD
#define PTRACE_O_TRACESYSGOOD 0x00000001
#endif

/* x86_64 syscall numbers. ⚠ Hard-coded rather than taken from <sys/syscall.h>
 * so this file says out loud that it is architecture-specific; on aarch64
 * (TODO T-041) __NR_open does not exist at all and only openat does. */
#if defined(__x86_64__)
#define PGB_NR_OPEN    2
#define PGB_NR_OPENAT  257
#define PGB_ARG_PATH(r)  ((r).rdi)   /* open(path, ...) */
#define PGB_ARG_PATH2(r) ((r).rsi)   /* openat(dirfd, path, ...) */
#define PGB_SYSNO(r)     ((r).orig_rax)
#define PGB_SYSRET(r)    ((r).rax)      /* the return value, at syscall EXIT */
#elif defined(__aarch64__)
#define PGB_NR_OPEN    (-1)
#define PGB_NR_OPENAT  56
#define PGB_ARG_PATH(r)  ((r).regs[0])
#define PGB_ARG_PATH2(r) ((r).regs[1])
#define PGB_SYSNO(r)     ((r).regs[8])
#define PGB_SYSRET(r)    ((r).regs[0])  /* the return value, at syscall EXIT */
#else
#error "pgb-trace supports x86_64 and aarch64; add the syscall numbers for this one"
#endif

/* Read a NUL-terminated string out of the tracee, a word at a time.
 * PTRACE_PEEKDATA is the portable way; process_vm_readv would be fewer calls
 * and is not available under every seccomp profile a container may impose. */
static void read_str(pid_t pid, unsigned long addr, char *out, size_t max)
{
    size_t i = 0;
    while (i + sizeof(long) < max) {
        long word;
        errno = 0;
        word = ptrace(PTRACE_PEEKDATA, pid, addr + i, 0);
        if (errno != 0) break;
        memcpy(out + i, &word, sizeof(long));
        if (memchr(out + i, 0, sizeof(long))) return;
        i += sizeof(long);
    }
    out[i < max ? i : max - 1] = '\0';
}

int main(int argc, char **argv)
{
    pid_t child;
    int i = 1, status, in_syscall = 0, pending_sig = 0;
    long opens = 0;
    /* The path seen at syscall entry, held until the exit stop says whether
     * the open actually succeeded. */
    static char pending[4096];

    if (i < argc && strcmp(argv[i], "--") == 0) i++;
    if (i >= argc) {
        fprintf(stderr, "usage: pgb-trace -- PROGRAM [ARGS...]\n");
        return 2;
    }

    child = fork();
    if (child < 0) { perror("fork"); return 2; }

    if (child == 0) {
        if (ptrace(PTRACE_TRACEME, 0, 0, 0) < 0) { perror("PTRACE_TRACEME"); _exit(127); }
        raise(SIGSTOP);
        execvp(argv[i], &argv[i]);
        perror("execvp");
        _exit(127);
    }

    if (waitpid(child, &status, 0) < 0) { perror("waitpid"); return 2; }

    /* ⛔ EXITKILL, because a tracer that dies leaving the subject stopped in
     * ptrace-stop wedges the container. docs/history/corrections.md records
     * exactly that costing a run: a tracee left in ptrace-stop where SIGKILL
     * to the tracer alone did not reap it. */
    /* ⛔ TRACESYSGOOD, AND IT IS NOT OPTIONAL -- THE FIRST VERSION WITHOUT IT
     * WAS WRONG. Without it, a syscall stop and an ordinary SIGTRAP stop are
     * indistinguishable, and `execve` under PTRACE_TRACEME delivers exactly
     * such an extra SIGTRAP. A bare entry/exit toggle therefore flips one time
     * too many at the very first syscall and reads every argument at the exit
     * stop and every result at the entry stop for the whole rest of the run.
     * Measured: /bin/true, which unmistakably loads libc.so.6, was reported as
     * opening nothing at all. With this option a syscall stop is SIGTRAP|0x80
     * and nothing else is, so the toggle cannot drift. */
    if (ptrace(PTRACE_SETOPTIONS, child, 0,
               PTRACE_O_EXITKILL | PTRACE_O_TRACESYSGOOD) < 0) {
        /* Not fatal on its own -- report it in the status line, do not lie. */
        fprintf(stderr, "pgb-trace: status=failed opens=0 exit=-1 (SETOPTIONS: %s)\n",
                strerror(errno));
        kill(child, SIGKILL);
        waitpid(child, &status, 0);
        return 2;
    }

    for (;;) {
        /* ⛔ THE PENDING SIGNAL IS RE-INJECTED, AND LEAVING IT OUT HANGS THE
         * TRACER FOREVER ON EXACTLY THE BINARIES THIS TOOL EXISTS TO CATCH.
         *
         * When the tracee takes SIGFPE or SIGSEGV, ptrace stops it to report
         * the signal and the tracer decides what is delivered. Resuming with
         * 0 SUPPRESSES it: the faulting instruction re-executes, faults
         * again, is suppressed again, and the process never dies.
         *
         * Measured on a CI runner: `pgb verify --engine docker` on the plain
         * -static control -- which dies with SIGFPE on Arch and openSUSE and
         * SIGSEGV on Debian 11 and Ubuntu -- got through four of eleven rows
         * in 18 minutes and was still going. The portable binary, which
         * faults nowhere, finished all eleven in 43 seconds.
         *
         * ⚠ SIGTRAP is the exception and must NOT be re-injected: it is
         * ptrace's own stop signal (execve raises one under PTRACE_TRACEME),
         * and delivering it would kill the process the tracer is measuring. */
        if (ptrace(PTRACE_SYSCALL, child, 0, (void *)(long)pending_sig) < 0) break;
        pending_sig = 0;
        if (waitpid(child, &status, 0) < 0) break;
        if (WIFEXITED(status) || WIFSIGNALED(status)) break;
        if (!WIFSTOPPED(status)) continue;

        {
            int sig = WSTOPSIG(status);
            if (sig != (SIGTRAP | 0x80)) {          /* not a syscall stop */
                if (sig != SIGTRAP) pending_sig = sig;   /* deliver it */
                continue;
            }
        }

        in_syscall = !in_syscall;
        {
            struct user_regs_struct r;
            if (ptrace(PTRACE_GETREGS, child, 0, &r) < 0) continue;

            if (in_syscall) {                /* ENTRY: the args are intact */
                long no = (long)PGB_SYSNO(r);
                unsigned long p = 0;
                if (no == PGB_NR_OPENAT)     p = (unsigned long)PGB_ARG_PATH2(r);
                else if (no == PGB_NR_OPEN)  p = (unsigned long)PGB_ARG_PATH(r);
                pending[0] = '\0';
                if (p) read_str(child, p, pending, sizeof pending);
            } else if (pending[0]) {         /* EXIT: the RESULT is available */
                /* ⛔ ONLY AN OPEN THAT SUCCEEDED IS A LOAD, AND THE FIRST
                 * VERSION OF THIS FILE GOT IT WRONG. Reporting at syscall
                 * ENTRY counts every path the program merely PROBED FOR.
                 * Measured: the docker arm reported /etc/nsswitch.conf read on
                 * alpine-3.10, where that file does not exist -- the open
                 * returned ENOENT. The chroot instrument filters those and the
                 * two arms disagreed, which is how this was found.
                 *
                 * ⛔ On the criterion-2 column that is not a cosmetic
                 * difference, it is a FALSE POSITIVE: glibc probes several
                 * paths for a shared object and takes the first that answers,
                 * so a binary that loaded nothing would have been failed for
                 * the ones that did not. */
                long ret = (long)PGB_SYSRET(r);
                if (ret >= 0) { fprintf(stderr, "open %s\n", pending); opens++; }
                pending[0] = '\0';
            }
        }
    }

    if (WIFEXITED(status)) {
        fprintf(stderr, "pgb-trace: status=traced opens=%ld exit=%d\n",
                opens, WEXITSTATUS(status));
        return WEXITSTATUS(status);
    }
    if (WIFSIGNALED(status)) {
        fprintf(stderr, "pgb-trace: status=traced opens=%ld signal=%d\n",
                opens, WTERMSIG(status));
        return 128 + WTERMSIG(status);
    }
    fprintf(stderr, "pgb-trace: status=failed opens=%ld exit=-1\n", opens);
    return 2;
}
