// Package verifyx inspects a binary and runs it across the pinned matrix.
//
// The static inspection is reported but is not the test. The test is criterion
// 2: the binary runs, and loads no host shared object, decided from a syscall
// trace attributed to its own pid.
//
// SPDX-License-Identifier: MIT
package verifyx

import (
	"bytes"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"time"

	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/elfx"
	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/proc"
	"github.com/polaris0xff/glibc-research/internal/rootfs"
	"github.com/polaris0xff/glibc-research/internal/wrapper"
)

var log = logx.New("verify")

// Row is one environment's result.
type Row struct {
	Name   string
	Libc   string
	Result string   // ok | exitN | SIGN | -
	Libs   []string // host shared objects the binary opened
	Data   []string // host data files it read; reported, never asserted
	// Unmeasured records that nobody looked, which is neither a pass nor a
	// failure. Reporting "none" here would turn "we did not look" into "we
	// looked and it was clean".
	Unmeasured bool
	Missing    bool
}

// Verify runs the whole report and returns a process exit code.
func Verify(c *cfg.Config, bin string, args []string) error {
	if bin == "" {
		return fail.Cannot("pgb verify NEEDS a binary")
	}
	if fi, err := os.Stat(bin); err != nil || fi.IsDir() {
		return fail.Cannot("pgb verify NEEDS a binary")
	}
	logx.Say("pgb verify: %s", bin)
	logx.Say("")
	logx.Say("  -- static inspection (NOT the success criterion) -------------------")
	printStatic(bin)
	logx.Say("")
	logx.Say("    None of the above is the test. A binary can satisfy every line")
	logx.Say("    here and still die on Arch. The next section is the test.")
	logx.Say("")

	mode := c.Engine()
	if mode == cfg.EngineHost {
		// The host engine has no bed of its own: fall back to the chroot bed,
		// which is what every committed number was measured through.
		mode = cfg.EngineChroot
	}
	images, err := cfg.ReadImages(c.ImagesFile())
	if err != nil {
		return err
	}

	logx.Say("  -- runtime, across the pinned matrix -------------------------------")
	logx.Say("     engine: %s", mode)
	logx.Say("    %-20s %-6s %-8s %-26s %s", "ENVIRONMENT", "LIBC", "RESULT", "HOST .so LOADED", "HOST DATA READ")

	failed := false
	sawUnmeasured := false
	any := false
	for _, img := range images {
		var row Row
		switch mode {
		case cfg.EngineChroot:
			row = runChroot(c, img, bin, args)
		default:
			row = runContainer(c, string(mode), img, bin, args)
		}
		if row.Missing {
			logx.Say("    %-20s %-6s %-8s %s", img.Name, img.Libc, "-", "not fetched")
			continue
		}
		any = true
		logx.Say("    %-20s %-6s %-8s %-26s %s", row.Name, row.Libc, row.Result,
			column(row.Libs, row.Unmeasured), column(row.Data, row.Unmeasured))
		if row.Result != "ok" {
			failed = true
		}
		// Only a host shared object is a failure. Reading host data is not:
		// glibc still opens /etc/nsswitch.conf under the NSS override, and a
		// program honouring the host's locale where one exists is correct.
		if row.Unmeasured {
			sawUnmeasured = true
		} else if len(row.Libs) > 0 {
			failed = true
		}
	}
	if !any {
		logx.Say("    nothing to run against. pgb rootfs fetch")
		return fail.Cannot("no target environments are on disk")
	}

	logx.Say("")
	switch {
	case !failed && sawUnmeasured:
		logx.Say("  VERDICT: ran correctly on every environment.")
		logx.Say("  CRITERION 2 WAS NOT CHECKED on at least one row. Where the")
		logx.Say("    host-object column reads 'unmeasured' the tracer did not attach,")
		logx.Say("    so that row says nothing about whether a host shared object was")
		logx.Say("    loaded. --cap-add=SYS_PTRACE is what it needs.")
	case !failed:
		logx.Say("  VERDICT: ran on every fetched environment and loaded no host shared object.")
	default:
		logx.Say("  VERDICT: NOT portable as built. A non-zero exit or a host object")
		logx.Say("           above is a real failure, not a labelling quibble.")
	}
	if failed {
		return fail.Exit(1)
	}
	return nil
}

func column(v []string, unmeasured bool) string {
	if unmeasured {
		return "unmeasured"
	}
	if len(v) == 0 {
		return "none"
	}
	return strings.Join(v, " ")
}

func printStatic(bin string) {
	fi, err := os.Stat(bin)
	if err == nil {
		logx.Say("    %-22s %d bytes", "size", fi.Size())
	}
	info, err := elfx.Inspect(bin)
	if err != nil {
		logx.Say("    %-22s %s", "file", "not an ELF file: "+err.Error())
		return
	}
	logx.Say("    %-22s %s %s %s", "file", info.Class, info.Machine, info.Type)
	logx.Say("    %-22s %s", "PT_INTERP", presence(info.Interp != ""))
	logx.Say("    %-22s %d", "DT_NEEDED entries", len(info.Needed))
	logx.Say("    %-22s %s", "PT_GNU_EH_FRAME", presence(info.HasEHFrame))
	if hasRuntimeAnchor(bin) {
		logx.Say("    %-22s %s", "nssfix linked in", "yes")
	} else {
		logx.Say("    %-22s %s", "nssfix linked in", "NO -- not a pgb binary")
	}
}

func presence(b bool) string {
	if b {
		return "present"
	}
	return "absent"
}

// hasRuntimeAnchor looks for the NSS constructor's marker string as a whole
// NUL-delimited run, rather than as a substring of something longer.
func hasRuntimeAnchor(bin string) bool {
	b, err := os.ReadFile(bin)
	if err != nil {
		return false
	}
	needle := []byte("pgb-runtime")
	for i := 0; ; {
		j := bytes.Index(b[i:], needle)
		if j < 0 {
			return false
		}
		at := i + j
		before := at == 0 || !isPrintable(b[at-1])
		after := at+len(needle) >= len(b) || !isPrintable(b[at+len(needle)])
		if before && after {
			return true
		}
		i = at + 1
	}
}

func isPrintable(c byte) bool { return c >= 0x20 && c < 0x7f }

// runChroot runs the subject inside a target filesystem, then traces it.
func runChroot(c *cfg.Config, img cfg.ImageRow, bin string, args []string) Row {
	row := Row{Name: img.Name, Libc: img.Libc}
	root := filepath.Join(c.RootfsDir, img.Name)
	if fi, err := os.Stat(root); err != nil || !fi.IsDir() {
		row.Missing = true
		return row
	}
	base := filepath.Base(bin)
	inner := "/" + base
	dst := filepath.Join(root, base)
	if err := copyExecutable(bin, dst); err != nil {
		row.Result = "copy-failed"
		return row
	}
	defer os.Remove(dst)

	argv := append([]string{inner}, args...)
	code, err := rootfs.Run(rootfs.Options{
		Root:   root,
		Stdin:  devNull(),
		Stdout: discard(),
		Stderr: discard(),
	}, argv)
	if err != nil {
		row.Result = "runner-error"
		return row
	}
	row.Result = resultOf(code)

	libs, data, ok := straceTrace(c, root, argv)
	if !ok {
		row.Unmeasured = true
		return row
	}
	row.Libs, row.Data = libs, data
	return row
}

func resultOf(code int) string {
	switch {
	case code == 0:
		return "ok"
	case code >= 129 && code <= 169:
		return fmt.Sprintf("SIG%d", code-128)
	default:
		return fmt.Sprintf("exit%d", code)
	}
}

var (
	// A shared object ends in .so or .so.N. Matching the substring ".so"
	// anywhere also matches /etc/ld.so.cache, which is an index and not an
	// object, and this value decides pass or fail.
	soPattern   = regexp.MustCompile(`"([^"]*\.so(?:\.[0-9]+)*)"`)
	openPattern = regexp.MustCompile(`\bopen(?:at)?\(`)
	execPattern = regexp.MustCompile(`\bexecve\("([^"]*)"`)
	pidPattern  = regexp.MustCompile(`^(\d+)\s`)
	dataPattern = regexp.MustCompile(`"([^"]*gconv[^"]*|/usr/lib/locale[^"]*|/etc/nsswitch\.conf|[^"]*terminfo[^"]*)"`)
)

// straceTrace runs the subject under strace and attributes opens to the pid
// that exec'd it, so nothing another process opened is charged to the binary.
func straceTrace(c *cfg.Config, root string, argv []string) (libs, data []string, ok bool) {
	if !proc.Look("strace") {
		return nil, nil, false
	}
	tmp, err := os.CreateTemp("", "pgb-strace-")
	if err != nil {
		return nil, nil, false
	}
	tmp.Close()
	defer os.Remove(tmp.Name())

	self, err := os.Executable()
	if err != nil {
		return nil, nil, false
	}
	cmd := append([]string{"strace", "-f", "-e", "trace=openat,open,execve", "-o", tmp.Name(),
		self, "rootfs", "run", root, "--"}, argv...)
	r, err := (&proc.Cmd{Argv: cmd, Stdin: devNull(), Stdout: discard(), Stderr: discard(), Subsys: "verify"}).Run()
	if err != nil {
		return nil, nil, false
	}
	_ = r
	out, err := os.ReadFile(tmp.Name())
	if err != nil {
		return nil, nil, false
	}
	return parseStrace(string(out), argv[0])
}

// parseStrace extracts the target pid's successful opens.
func parseStrace(text, want string) (libs, data []string, ok bool) {
	target := ""
	seen := false
	libSet := map[string]bool{}
	dataSet := map[string]bool{}
	for line := range strings.SplitSeq(text, "\n") {
		pid := ""
		if m := pidPattern.FindStringSubmatch(line); m != nil {
			pid = m[1]
		}
		if m := execPattern.FindStringSubmatch(line); m != nil && m[1] == want {
			target, seen = pid, true
			continue
		}
		if !seen || pid != target {
			continue
		}
		if !openPattern.MatchString(line) {
			continue
		}
		if strings.Contains(line, "ENOENT") || strings.Contains(line, "= -1") {
			continue
		}
		for _, m := range soPattern.FindAllStringSubmatch(line, -1) {
			libSet[m[1]] = true
		}
		for _, m := range dataPattern.FindAllStringSubmatch(line, -1) {
			dataSet[classifyData(m[1])] = true
		}
	}
	if !seen {
		return nil, nil, false
	}
	return sortedKeys(libSet), sortedKeys(dataSet), true
}

func classifyData(p string) string {
	switch {
	case strings.Contains(p, "gconv"):
		return "gconv-cfg"
	case strings.HasPrefix(p, "/usr/lib/locale"):
		return "/usr/lib/locale/*"
	case strings.Contains(p, "terminfo"):
		return "terminfo"
	default:
		return p
	}
}

func sortedKeys(m map[string]bool) []string {
	out := make([]string, 0, len(m))
	for k := range m {
		out = append(out, k)
	}
	sort.Strings(out)
	return out
}

// runContainer runs the subject in the pinned image with the carried tracer as
// the entrypoint, so the subject is its only child and nothing of the image's
// choosing runs.
func runContainer(c *cfg.Config, engine string, img cfg.ImageRow, bin string, args []string) Row {
	row := Row{Name: img.Name, Libc: img.Libc}
	image := img.Repo() + "@" + img.Digest
	binDir, err := filepath.Abs(filepath.Dir(bin))
	if err != nil {
		row.Result = "path-error"
		return row
	}
	base := filepath.Base(bin)

	timeout := 120 * time.Second
	if v := os.Getenv("PGB_VERIFY_TIMEOUT"); v != "" {
		if d, err := time.ParseDuration(v + "s"); err == nil {
			timeout = d
		}
	}

	b := wrapper.NewBuilder(c)
	tracer, terr := b.BuildTracer()
	if terr != nil {
		log.Debugf("no carried tracer: %v", terr)
		// Bounded, always: a tracer defect must cost one row and a visible
		// exit124, never an unbounded wait.
		argv := append([]string{"timeout", fmt.Sprintf("%d", int(timeout.Seconds())),
			engine, "run", "--rm", "-v", binDir + ":/pgb-verify:ro",
			"--entrypoint", "/pgb-verify/" + base, image}, args...)
		r, err := (&proc.Cmd{Argv: argv, Stdout: discard(), Stderr: discard(), Subsys: "verify"}).Run()
		if err != nil {
			row.Result = "runner-error"
		} else {
			row.Result = resultOf(r.Code)
		}
		row.Unmeasured = true
		return row
	}

	tdir := filepath.Dir(tracer)
	argv := append([]string{"timeout", fmt.Sprintf("%d", int(timeout.Seconds())),
		engine, "run", "--rm", "--cap-add=SYS_PTRACE",
		"-v", binDir + ":/pgb-verify:ro", "-v", tdir + ":/pgb-tracer:ro",
		"--entrypoint", "/pgb-tracer/" + filepath.Base(tracer), image,
		"--", "/pgb-verify/" + base}, args...)

	var stderr bytes.Buffer
	r, err := (&proc.Cmd{Argv: argv, Stdout: discard(), Stderr: &stderr, Subsys: "verify"}).Run()
	if err != nil {
		row.Result = "runner-error"
		row.Unmeasured = true
		return row
	}
	row.Result = resultOf(r.Code)

	// The tracer says whether it looked, and the caller keys on that: a tracer
	// that failed to attach prints no paths, which is byte for byte what a
	// clean binary produces.
	out := stderr.String()
	if !strings.Contains(out, "status=traced") {
		row.Unmeasured = true
		return row
	}
	row.Libs, row.Data = classifyTracerOutput(out)
	return row
}

// classifyTracerOutput reads the carried tracer's `open <path>` lines.
func classifyTracerOutput(out string) (libs, data []string) {
	libSet := map[string]bool{}
	dataSet := map[string]bool{}
	for line := range strings.SplitSeq(out, "\n") {
		p, ok := strings.CutPrefix(line, "open ")
		if !ok {
			continue
		}
		p = strings.TrimSpace(p)
		switch {
		case isSharedObject(p):
			libSet[p] = true
		case strings.Contains(p, "gconv"):
			dataSet["gconv-cfg"] = true
		case strings.HasPrefix(p, "/usr/lib/locale"):
			dataSet["/usr/lib/locale/*"] = true
		case p == "/etc/nsswitch.conf":
			dataSet[p] = true
		case strings.Contains(p, "terminfo"):
			dataSet["terminfo"] = true
		}
	}
	return sortedKeys(libSet), sortedKeys(dataSet)
}

var soSuffix = regexp.MustCompile(`\.so(\.[0-9]+)*$`)

// IsSharedObject reports whether a path names a shared object rather than
// something merely containing ".so", such as /etc/ld.so.cache.
func IsSharedObject(p string) bool { return isSharedObject(p) }

func isSharedObject(p string) bool { return soSuffix.MatchString(p) }

func copyExecutable(src, dst string) error {
	b, err := os.ReadFile(src)
	if err != nil {
		return err
	}
	return os.WriteFile(dst, b, 0o755)
}

func devNull() *os.File {
	f, err := os.Open(os.DevNull)
	if err != nil {
		return nil
	}
	return f
}

func discard() *os.File {
	f, err := os.OpenFile(os.DevNull, os.O_WRONLY, 0)
	if err != nil {
		return nil
	}
	return f
}
