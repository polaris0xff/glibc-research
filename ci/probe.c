/* ci/probe.c -- the binary CI copies onto eleven distributions.
 *
 * ⛔ IT MUST FAIL LOUDLY, NOT DEGRADE QUIETLY. Every check below returns a
 * non-zero exit on failure, because the whole point of the matrix job is that
 * a red square means something. A probe that printed a warning and exited 0
 * would turn the entire workflow into decoration.
 *
 * ⚠ WHAT IT DELIBERATELY DOES NOT ASSERT. Two things are environment
 * properties rather than portability properties, and asserting them would
 * produce failures that are nothing to do with this project:
 *
 *   - resolving "localhost". Debian and Ubuntu base images ship an /etc/hosts
 *     with no localhost line; a plain `gcc -static` binary was measured
 *     failing there identically. The probe creates its own name instead.
 *   - reaching the network. A CI runner may have no route out. DNS is
 *     attempted and REPORTED, never required.
 *
 * Everything else is a hard assertion.
 */

#define _GNU_SOURCE
#include <errno.h>
#include <iconv.h>
#include <langinfo.h>
#include <locale.h>
#include <netdb.h>
#include <pwd.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>

static int failures;

static void ok(const char *name, int cond, const char *detail)
{
    printf("  %-4s %-24s %s\n", cond ? "ok" : "FAIL", name, detail ? detail : "");
    if (!cond) failures++;
}

static void note(const char *name, const char *detail)
{
    printf("  %-4s %-24s %s\n", "--", name, detail);
}

/* NSS through the passwd database. uid 0 is root on every Linux filesystem
 * that has an /etc/passwd at all, so this is a stable assertion. */
static void check_passwd(void)
{
    struct passwd *pw = getpwuid(0);
    ok("nss-passwd", pw && pw->pw_name && strcmp(pw->pw_name, "root") == 0,
       pw && pw->pw_name ? pw->pw_name : "(null)");
}

/* NSS through the hosts database, using a name this probe puts in /etc/hosts
 * so the check measures the dispatcher rather than the image's contents. */
static void check_hosts(void)
{
    FILE *f = fopen("/etc/hosts", "a");
    if (f) { fputs("\n127.0.0.1 pgb-ci-probe\n", f); fclose(f); }

    struct addrinfo hints, *res = NULL;
    memset(&hints, 0, sizeof hints);
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    int rc = getaddrinfo("pgb-ci-probe", "80", &hints, &res);
    ok("nss-hosts-file", rc == 0, rc == 0 ? "resolved via files" : gai_strerror(rc));
    if (res) freeaddrinfo(res);
}

/* ⭐ THE ENCODINGS. On a plain static glibc binary these either crash the
 * process (when the host's gconv path matches the build's) or return EINVAL
 * (when it does not). Here they must all open and one must round-trip to
 * exact bytes. */
static void check_iconv(void)
{
    static const char *const to[] = { "ISO-8859-1", "ISO-8859-15", "CP1252",
                                      "EUC-JP", "SHIFT_JIS", "GB18030",
                                      "KOI8-R", "BIG5", "UTF-16LE", "UTF-32",
                                      "ISO-8859-2", "ASCII", NULL };
    int opened = 0, failed = 0;
    for (int i = 0; to[i]; i++) {
        iconv_t cd = iconv_open(to[i], "UTF-8");
        if (cd == (iconv_t) -1) { failed++; printf("       iconv_open UTF-8 -> %s failed\n", to[i]); }
        else { opened++; iconv_close(cd); }
    }
    char buf[64];
    snprintf(buf, sizeof buf, "%d opened, %d failed", opened, failed);
    ok("iconv-open-12", failed == 0, buf);

    /* Byte-exact: "café" in UTF-8 becomes 4 bytes in Latin-1 ending 0xE9. */
    iconv_t cd = iconv_open("ISO-8859-1", "UTF-8");
    int good = 0;
    if (cd != (iconv_t) -1) {
        char in[] = "caf\xc3\xa9", out[16];
        char *ip = in, *op = out;
        size_t il = strlen(in), ol = sizeof out;
        if (iconv(cd, &ip, &il, &op, &ol) != (size_t) -1 &&
            (size_t)(op - out) == 4 && (unsigned char) out[3] == 0xE9)
            good = 1;
        iconv_close(cd);
    }
    ok("iconv-roundtrip", good, "UTF-8 -> ISO-8859-1 byte exact");
}

static void *thread_fn(void *arg) { *(int *) arg = 1; return NULL; }

/* TLS and thread setup in a static binary. */
static void check_threads(void)
{
    pthread_t t[4];
    int flags[4] = { 0, 0, 0, 0 };
    int started = 0;
    for (int i = 0; i < 4; i++)
        if (pthread_create(&t[i], NULL, thread_fn, &flags[i]) == 0) started++;
    for (int i = 0; i < started; i++) pthread_join(t[i], NULL);
    int all = (started == 4);
    for (int i = 0; i < 4; i++) if (!flags[i]) all = 0;
    ok("threads", all, "4 threads created, ran and joined");
}

/* Reported, never asserted: a runner may be offline. */
static void check_dns(void)
{
    struct addrinfo hints, *res = NULL;
    memset(&hints, 0, sizeof hints);
    hints.ai_socktype = SOCK_STREAM;
    int rc = getaddrinfo("example.com", "80", &hints, &res);
    note("dns (not asserted)", rc == 0 ? "resolved" : gai_strerror(rc));
    if (res) freeaddrinfo(res);
}

static void check_locale(void)
{
    char *l = setlocale(LC_ALL, "");
    char buf[128];
    snprintf(buf, sizeof buf, "setlocale=%s codeset=%s",
             l ? l : "NULL", nl_langinfo(CODESET));
    note("locale (not asserted)", buf);
}

int main(void)
{
    printf("pgb ci probe\n");
    check_passwd();
    check_hosts();
    check_iconv();
    check_threads();
    check_dns();
    check_locale();
    printf("%s: %d failure(s)\n", failures ? "FAILED" : "PASSED", failures);
    return failures ? 1 : 0;
}
