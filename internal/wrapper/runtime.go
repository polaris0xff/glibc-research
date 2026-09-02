// Package wrapper builds pgb's C runtime objects, decides the flags a build
// gets, and provides the compiler wrappers themselves.
//
// The wrappers are this same binary invoked under another name. They read one
// manifest written when the wrapper directory was created, then exec the real
// compiler: no shell is started, and nothing is re-parsed per invocation.
//
// SPDX-License-Identifier: MIT
package wrapper

import (
	"bytes"
	"fmt"
	"hash/crc32"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"

	assets "github.com/polaris0xff/glibc-research"
	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/proc"
)

var log = logx.New("wrapper")

// Builder compiles the runtime objects for one configuration.
type Builder struct {
	C  *cfg.Config
	CC string // the compiler to build the runtime with
}

// NewBuilder resolves the compiler the runtime objects are built by.
func NewBuilder(c *cfg.Config) *Builder {
	cc := firstNonEmpty(os.Getenv("PGB_INNER_CC"), os.Getenv("CC"), "cc")
	return &Builder{C: c, CC: cc}
}

// Dir is where the runtime objects for this compiler live.
//
// The key is the compiler's own identity, not the machine: $PGB_STATE is
// bind-mounted into the build environment, so an object compiled by the host's
// gcc would otherwise sit where the pinned environment's gcc looks.
func (b *Builder) Dir() string {
	id := fmt.Sprintf("%s\n%s\n%s",
		captureQuiet(b.CC, "-dumpmachine"),
		firstLine(captureQuiet(b.CC, "--version")),
		assets.Digest())
	return filepath.Join(b.C.State, fmt.Sprintf("runtime-%s-%d",
		unameMachine(), crc32.ChecksumIEEE([]byte(id))))
}

// srcDir materialises the embedded C sources and returns the directory.
func (b *Builder) srcDir() (string, error) {
	dir := b.C.RuntimeSrcDir()
	if err := assets.Materialise(dir); err != nil {
		return "", fail.Cannot("cannot write the carried C runtime to %s: %v", dir, err)
	}
	return dir, nil
}

// Build compiles everything this configuration needs and returns the runtime
// directory. Objects are rebuilt only when their source is newer.
func (b *Builder) Build() (string, error) {
	rd := b.Dir()
	if err := os.MkdirAll(rd, 0o755); err != nil {
		return "", fail.Cannot("cannot create %s: %v", rd, err)
	}
	src, err := b.srcDir()
	if err != nil {
		return "", err
	}

	if err := b.compileIfStale(src, "pgb-nssfix.c", filepath.Join(rd, "pgb-nssfix.o"), "-O2"); err != nil {
		return "", err
	}
	if b.C.UseIconv {
		arc := filepath.Join(rd, "libpgbruntime.a")
		if stale(filepath.Join(src, "pgb-iconv.c"), arc) {
			obj := filepath.Join(rd, "pgb-iconv.o")
			if err := b.compile(filepath.Join(src, "pgb-iconv.c"), obj, "-O2"); err != nil {
				return "", err
			}
			_ = os.Remove(arc)
			if r, err := proc.Run("ar", "rcs", arc, obj); err != nil || r.Failed() {
				return "", fail.Ran("ar failed for %s", arc)
			}
		}
	}
	if b.C.EmbedLocale {
		if err := b.buildLocaleData(rd, src); err != nil {
			return "", err
		}
	}
	if b.C.EmbedCacert {
		if err := b.buildCacertData(rd, src); err != nil {
			return "", err
		}
	}
	if b.C.EmbedTerminfo {
		if err := b.buildTerminfoData(rd, src); err != nil {
			return "", err
		}
	}
	if len(b.C.WrapDlopen) > 0 {
		if err := b.buildDlopenTable(rd, src); err != nil {
			return "", err
		}
	}
	return rd, nil
}

// BuildTracer compiles the carried-in ptrace tracer `pgb verify` uses where
// strace cannot follow the subject. It is linked plain `-static`: the
// instrument must not depend on the mechanisms it measures.
func (b *Builder) BuildTracer() (string, error) {
	rd := b.Dir()
	if err := os.MkdirAll(rd, 0o755); err != nil {
		return "", err
	}
	src, err := b.srcDir()
	if err != nil {
		return "", err
	}
	in := filepath.Join(src, "pgb-trace.c")
	out := filepath.Join(rd, "pgb-trace")
	if !stale(in, out) {
		return out, nil
	}
	cc := firstNonEmpty(os.Getenv("CC"), "cc")
	r, err := (&proc.Cmd{Argv: []string{cc, "-O2", "-static", "-o", out, in}, Subsys: "wrapper"}).Output()
	if err != nil || r.Failed() {
		return "", fail.Ran("cannot build the carried tracer: %s", strings.TrimSpace(string(r.Stderr)))
	}
	return out, nil
}

func (b *Builder) compileIfStale(srcDir, name, out string, flags ...string) error {
	in := filepath.Join(srcDir, name)
	if !stale(in, out) {
		return nil
	}
	return b.compile(in, out, flags...)
}

func (b *Builder) compile(in, out string, flags ...string) error {
	log.Infof("compiling %s", filepath.Base(out))
	argv := append([]string{b.CC}, flags...)
	argv = append(argv, "-fno-lto", "-c", "-o", out, in)
	r, err := (&proc.Cmd{Argv: argv, Subsys: "wrapper"}).Output()
	if err != nil {
		return fail.Cannot("%s: %v", b.CC, err)
	}
	if r.Failed() {
		return fail.Ran("%s did not compile:\n%s", filepath.Base(in),
			strings.TrimSpace(string(r.Stderr)))
	}
	return nil
}

// stale reports whether out is missing or older than in.
func stale(in, out string) bool {
	oi, err := os.Stat(out)
	if err != nil {
		return true
	}
	ii, err := os.Stat(in)
	if err != nil {
		return false
	}
	return ii.ModTime().After(oi.ModTime())
}

// ---------------------------------------------------------------------------
// Embedded data: terminfo, CA bundle, locale.
// ---------------------------------------------------------------------------

// terminfoRoots are searched in order, and every root is searched for every
// entry: a distribution can ship an empty /usr/share/terminfo and keep the
// real entries in /lib/terminfo.
var terminfoRoots = []string{"/usr/share/terminfo", "/lib/terminfo", "/etc/terminfo", "/usr/lib/terminfo"}

func (b *Builder) buildTerminfoData(rd, src string) error {
	if exists(filepath.Join(rd, "pgb-terminfo.o")) && exists(filepath.Join(rd, "pgb-terminfo-data.o")) {
		return nil
	}
	var roots []string
	for _, r := range terminfoRoots {
		if isDir(r) {
			roots = append(roots, r)
		}
	}
	if len(roots) == 0 {
		return fail.Cannot("--embed-terminfo needs a terminfo database in the build environment (install ncurses-base)")
	}
	entries := cfg.DefaultTerminfoEntries
	if v := os.Getenv("PGB_TERMINFO_ENTRIES"); v != "" {
		entries = strings.Fields(v)
	}

	var buf bytes.Buffer
	fmt.Fprintf(&buf, "/* generated by pgb from %s */\n", strings.Join(roots, " "))
	buf.WriteString("struct pgb_ti_file { const char *name; const unsigned char *data; unsigned len; };\n")
	var found []string
	n := 0
	for _, t := range entries {
		path := findTerminfo(roots, t)
		if path == "" {
			continue
		}
		data, err := os.ReadFile(path)
		if err != nil {
			continue
		}
		fmt.Fprintf(&buf, "static const unsigned char t%d[] = {", n)
		writeBytes(&buf, data)
		buf.WriteString("};\n")
		found = append(found, fmt.Sprintf("%s:%d", t, n))
		n++
	}
	if n == 0 {
		return fail.Cannot("--embed-terminfo found none of %s under %s",
			strings.Join(entries, " "), strings.Join(roots, " "))
	}
	buf.WriteString("const struct pgb_ti_file pgb_ti_files[] = {\n")
	for i, f := range found {
		fmt.Fprintf(&buf, "  { \"%s\", t%d, sizeof t%d },\n", strings.SplitN(f, ":", 2)[0], i, i)
	}
	buf.WriteString("};\n")
	fmt.Fprintf(&buf, "const unsigned pgb_ti_nfiles = %d;\n", n)

	log.Infof("embedding terminfo entries: %s", strings.Join(found, " "))
	gen := filepath.Join(rd, "pgb-terminfo-data.c")
	if err := os.WriteFile(gen, buf.Bytes(), 0o644); err != nil {
		return err
	}
	if err := b.compile(gen, filepath.Join(rd, "pgb-terminfo-data.o"), "-O0"); err != nil {
		return err
	}
	return b.compile(filepath.Join(src, "pgb-terminfo.c"), filepath.Join(rd, "pgb-terminfo.o"), "-O2")
}

func findTerminfo(roots []string, term string) string {
	if term == "" {
		return ""
	}
	first := term[:1]
	for _, r := range roots {
		// Two layouts exist: a letter directory, and a hex-of-the-first-byte
		// directory on distributions that use it.
		for _, sub := range []string{first, fmt.Sprintf("%02x", term[0])} {
			p := filepath.Join(r, sub, term)
			if fi, err := os.Stat(p); err == nil && !fi.IsDir() {
				return p
			}
		}
	}
	return ""
}

// caBundleCandidates are the build environment's own trust stores, searched in
// order. The embedded copy is a fallback the runtime uses only where the host
// has none.
var caBundleCandidates = []string{
	"/etc/ssl/certs/ca-certificates.crt", "/etc/pki/tls/certs/ca-bundle.crt",
	"/etc/ssl/ca-bundle.pem", "/etc/ssl/cert.pem",
}

func (b *Builder) buildCacertData(rd, src string) error {
	if exists(filepath.Join(rd, "pgb-cacert.o")) && exists(filepath.Join(rd, "pgb-cacert-data.o")) {
		return nil
	}
	var chosen string
	for _, c := range caBundleCandidates {
		if fi, err := os.Stat(c); err == nil && fi.Size() > 0 {
			chosen = c
			break
		}
	}
	if chosen == "" {
		return fail.Cannot("--embed-cacert found no CA bundle in the build environment (install ca-certificates)")
	}
	data, err := os.ReadFile(chosen)
	if err != nil {
		return fail.Cannot("cannot read %s: %v", chosen, err)
	}
	log.Infof("embedding CA bundle from %s (%d bytes)", chosen, len(data))
	var buf bytes.Buffer
	fmt.Fprintf(&buf, "/* generated by pgb from %s */\n", chosen)
	buf.WriteString("const unsigned char pgb_cacert_data[] = {")
	writeBytes(&buf, data)
	buf.WriteString("};\n")
	buf.WriteString("const unsigned pgb_cacert_len = sizeof pgb_cacert_data;\n")
	gen := filepath.Join(rd, "pgb-cacert-data.c")
	if err := os.WriteFile(gen, buf.Bytes(), 0o644); err != nil {
		return err
	}
	if err := b.compile(gen, filepath.Join(rd, "pgb-cacert-data.o"), "-O0"); err != nil {
		return err
	}
	return b.compile(filepath.Join(src, "pgb-cacert.c"), filepath.Join(rd, "pgb-cacert.o"), "-O2")
}

func (b *Builder) buildLocaleData(rd, src string) error {
	if exists(filepath.Join(rd, "pgb-locale.o")) && exists(filepath.Join(rd, "pgb-locale-data.o")) {
		return nil
	}
	var dir string
	for _, c := range []string{"/usr/lib/locale/C.utf8", "/usr/lib/locale/C.UTF-8"} {
		if isDir(c) {
			dir = c
			break
		}
	}
	if dir == "" {
		return fail.Cannot("--embed-locale needs a compiled C.UTF-8 in the build environment (locale-gen C.UTF-8)")
	}
	// A glibc locale is a tree: LC_MESSAGES is a directory holding
	// SYS_LC_MESSAGES, and a locale missing one category fails the whole
	// LC_ALL composite at run time with no diagnostic.
	var files []string
	err := filepath.Walk(dir, func(p string, fi os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if fi.Mode().IsRegular() {
			rel, err := filepath.Rel(dir, p)
			if err != nil {
				return err
			}
			files = append(files, rel)
		}
		return nil
	})
	if err != nil {
		return fail.Cannot("cannot read %s: %v", dir, err)
	}
	sort.Strings(files)
	if len(files) == 0 {
		return fail.Cannot("--embed-locale found no files under %s", dir)
	}
	log.Infof("embedding locale from %s (%d files)", dir, len(files))

	var buf bytes.Buffer
	fmt.Fprintf(&buf, "/* generated by pgb from %s */\n", dir)
	buf.WriteString("struct pgb_locale_file { const char *name; const unsigned char *data; unsigned len; };\n")
	for i, f := range files {
		data, err := os.ReadFile(filepath.Join(dir, f))
		if err != nil {
			return err
		}
		fmt.Fprintf(&buf, "static const unsigned char d%d[] = {", i)
		writeBytes(&buf, data)
		buf.WriteString("};\n")
	}
	buf.WriteString("const struct pgb_locale_file pgb_locale_files[] = {\n")
	for i, f := range files {
		fmt.Fprintf(&buf, "  { \"%s\", d%d, sizeof d%d },\n", f, i, i)
	}
	buf.WriteString("};\n")
	fmt.Fprintf(&buf, "const unsigned pgb_locale_nfiles = %d;\n", len(files))
	fmt.Fprintf(&buf, "const char pgb_locale_name[] = \"%s\";\n", filepath.Base(dir))

	gen := filepath.Join(rd, "pgb-locale-data.c")
	if err := os.WriteFile(gen, buf.Bytes(), 0o644); err != nil {
		return err
	}
	if err := b.compile(gen, filepath.Join(rd, "pgb-locale-data.o"), "-O0"); err != nil {
		return err
	}
	return b.compile(filepath.Join(src, "pgb-locale.c"), filepath.Join(rd, "pgb-locale.o"), "-O2")
}

// writeBytes emits a comma-separated decimal byte list.
func writeBytes(buf *bytes.Buffer, data []byte) {
	for _, c := range data {
		fmt.Fprintf(buf, "%d,", c)
	}
}

func exists(p string) bool { _, err := os.Stat(p); return err == nil }

func isDir(p string) bool { fi, err := os.Stat(p); return err == nil && fi.IsDir() }

func firstNonEmpty(v ...string) string {
	for _, s := range v {
		if s != "" {
			return s
		}
	}
	return ""
}

func firstLine(s string) string {
	if i := strings.IndexByte(s, '\n'); i >= 0 {
		return s[:i]
	}
	return s
}

func captureQuiet(argv ...string) string {
	out, _ := proc.CaptureAllowFail(argv...)
	return out
}

func unameMachine() string {
	if out, err := exec.Command("uname", "-m").Output(); err == nil {
		return strings.TrimSpace(string(out))
	}
	return "unknown"
}
