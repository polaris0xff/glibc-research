// cfg_selftest.go — the settings, and the one property nothing else can check.
//
// ⛔ WHY THIS EXISTS. `pgb build` re-enters ITSELF inside the chroot or the
// container with only the user's command, so every build option crosses that
// boundary through the ENVIRONMENT and not through the argv. Export() writes
// them; Load() reads them back; OptVars is the list the container argument
// builder renders. ⚠ Three places, one truth, and nothing asserted that they
// agreed — an option added to two of the three is silently dropped at the
// boundary, and what the user sees is a build that succeeded without the flag
// they passed. That is T-019's defect in a different costume.
//
// ⭐ THE ROUND TRIP IS THE ASSERTION, not a list of names: set every option to
// a value distinguishable from its default, Export, re-Load, compare. An
// option Export forgets comes back as its default and fails here.
//
// Offline: it reads and writes only this process's own environment, and puts
// it back.
//
// SPDX-License-Identifier: MIT
package cfg

import (
	"os"
	"path/filepath"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/selftest"
)

// Selftest asserts the option handoff, the pinned image list, and the paths.
func Selftest() *selftest.Report {
	r := selftest.New("cfg")

	// ⚠ EVERY OptVar IS SAVED AND RESTORED. This process is `pgb selftest`,
	// and leaving PGB_OPT_* set behind would change what a later command in
	// the same shell builds.
	saved := map[string]*string{}
	for _, v := range OptVars {
		if s, ok := os.LookupEnv(v); ok {
			c := s
			saved[v] = &c
		} else {
			saved[v] = nil
		}
		_ = os.Unsetenv(v)
	}
	defer func() {
		for v, s := range saved {
			if s == nil {
				_ = os.Unsetenv(v)
			} else {
				_ = os.Setenv(v, *s)
			}
		}
	}()

	// --- ⭐ the round trip -------------------------------------------------
	// Every value differs from the default, so an option Export drops comes
	// back as the default and the comparison fails.
	out := &Config{
		Self:           "/self",
		State:          "/state",
		LibiconvPrefix: "/iconv",
		Verbose:        true,
		EmbedLocale:    true,
		EmbedCacert:    true,
		EmbedTerminfo:  true,
		EmbedTzdata:    true,
		UseIconv:       false, // the default is TRUE, so false is the signal
		ArchBaseline:   "x86-64-v3",
		ExtraBinds:     []string{"/a:/a", "/b:/b"},
		WrapDlopen:     []string{"p=x.o", "q=y.o"},
		HostDlopen:     true,
		TLSReserve:     65536,
	}
	// The environment as it stands, so the set Export() touches can be DERIVED
	// rather than listed. A listed set is a copy of OptVars and would agree
	// with it by construction.
	before := map[string]string{}
	for _, kv := range os.Environ() {
		if k, v, ok := strings.Cut(kv, "="); ok {
			before[k] = v
		}
	}
	out.Export()
	var exported []string
	for _, kv := range os.Environ() {
		k, v, ok := strings.Cut(kv, "=")
		if !ok {
			continue
		}
		if old, had := before[k]; !had || old != v {
			exported = append(exported, k)
		}
	}
	back := Load("/self")

	r.CheckBool("--verbose survives the engine boundary", back.Verbose, true)
	r.CheckBool("--embed-locale survives", back.EmbedLocale, true)
	r.CheckBool("--embed-cacert survives", back.EmbedCacert, true)
	r.CheckBool("--embed-terminfo survives", back.EmbedTerminfo, true)
	r.CheckBool("--embed-tzdata survives", back.EmbedTzdata, true)
	r.CheckBool("⛔ --no-iconv survives (the default is ON, so this is the hard one)",
		back.UseIconv, false)
	r.CheckBool("--host-dlopen survives", back.HostDlopen, true)
	r.Check("--baseline survives", back.ArchBaseline, "x86-64-v3")
	r.Check("--tls-reserve survives", itoa(back.TLSReserve), "65536")
	r.Check("--bind survives, and keeps its order",
		strings.Join(back.ExtraBinds, " "), "/a:/a /b:/b")
	r.Check("--wrap-dlopen survives, and keeps its order",
		strings.Join(back.WrapDlopen, " "), "p=x.o q=y.o")
	r.Check("PGB_STATE survives", back.State, "/state")
	r.Check("PGB_LIBICONV_PREFIX survives", back.LibiconvPrefix, "/iconv")

	// ⛔ AND THE CONTAINER ARGUMENT BUILDER MUST RENDER EVERYTHING Export()
	// WROTE. A variable Export writes and OptVars omits crosses a chroot and
	// not a container — a difference between engines that no output of either
	// would show.
	//
	// ⚠ THE DIRECTION MATTERS AND THE FIRST VERSION OF THIS HAD IT BACKWARDS.
	// It iterated OptVars and asked whether each was rendered, which is
	// trivially true because ContainerEnvArgs iterates OptVars too. Removing
	// PGB_OPT_BINDS from OptVars left the suite green — a check with nothing
	// in it. What binds is the OTHER direction: the set Export() actually
	// touched, DERIVED by diffing the environment across the call, must be
	// contained in OptVars.
	args := out.ContainerEnvArgs()
	rendered := map[string]bool{}
	for i := 0; i+1 < len(args); i += 2 {
		if args[i] == "-e" {
			rendered[args[i+1]] = true
		}
	}
	orphan := ""
	for _, v := range exported {
		if !rendered[v] {
			orphan += v + " "
		}
	}
	r.Check("⛔ every variable Export() writes is rendered as a container -e argument",
		strings.TrimSpace(orphan), "")
	r.CheckBool("and Export() wrote something at all", len(exported) > 5, true)
	r.CheckBool("an UNSET variable is not rendered", rendered["PGB_TS_HEARTBEAT"], false)

	// ⛔ PGB_ENGINE IS DELIBERATELY NOT IN OptVars, and asserting it keeps
	// somebody from "fixing" the omission: exporting it across an engine
	// boundary would make the re-entered pgb try to enter a second container.
	inOpt := false
	for _, v := range OptVars {
		if v == "PGB_ENGINE" {
			inOpt = true
		}
	}
	r.CheckBool("PGB_ENGINE is NOT carried across the boundary", inOpt, false)

	// --- the engine, both directions --------------------------------------
	c := &Config{}
	r.CheckBool("--engine chroot is accepted", c.SetEngine("chroot") == nil, true)
	r.Check("and is what Engine() returns", string(c.Engine()), "chroot")
	r.CheckBool("EngineExplicit says so", c.EngineExplicit(), true)
	r.CheckBool("--engine nonsense is refused", c.SetEngine("nonsense") != nil, true)
	r.CheckBool("the empty engine means detect", func() bool {
		d := &Config{}
		return d.SetEngine("") == nil && !d.EngineExplicit()
	}(), true)

	// --- paths -------------------------------------------------------------
	// ⚠ AbsBindspec runs for EVERY engine, not only chroot: a relative bind
	// source is a NAMED VOLUME to docker, not a directory.
	wd, _ := os.Getwd()
	r.Check("a bare bind source becomes SRC:SRC, absolute",
		AbsBindspec("/x"), "/x:/x")
	r.Check("SRC:DEST keeps both sides", AbsBindspec("/x:/y"), "/x:/y")
	r.Check("a relative source is made absolute",
		AbsBindspec("./rel:/y"), filepath.Join(wd, "rel")+":/y")
	r.Check("EnvRoot is RootfsDir/EnvName",
		(&Config{RootfsDir: "/r", EnvName: "e"}).EnvRoot(), "/r/e")

	// --- the pinned image list ---------------------------------------------
	// ⚠ A row with fewer than four fields is an ERROR rather than a silently
	// shorter matrix, which is the property worth asserting.
	dir, err := os.MkdirTemp("", "pgb-cfg-selftest")
	if err != nil {
		r.Skip("cannot create a temporary directory: " + err.Error())
		return r
	}
	defer os.RemoveAll(dir)
	good := filepath.Join(dir, "good.txt")
	_ = os.WriteFile(good, []byte(
		"# a comment\n\nalpine:3.22  alpine-3.22  musl  sha256:aa\n"+
			"debian:12  debian-12  glibc  sha256:bb\n"), 0o644)
	rows, err := ReadImages(good)
	r.CheckBool("a well-formed images file parses", err == nil, true)
	r.Check("and skips comments and blank lines", itoa(len(rows)), "2")
	if len(rows) == 2 {
		r.Check("the local name is column 2", rows[0].Name, "alpine-3.22")
		r.Check("the libc is column 3", rows[0].Libc, "musl")
		r.Check("Repo() strips the tag", rows[1].Repo(), "debian")
	}
	short := filepath.Join(dir, "short.txt")
	_ = os.WriteFile(short, []byte("alpine:3.22 alpine-3.22 musl\n"), 0o644)
	_, err = ReadImages(short)
	r.CheckBool("⛔ a three-column row is an ERROR, not a shorter matrix",
		err != nil, true)
	empty := filepath.Join(dir, "empty.txt")
	_ = os.WriteFile(empty, []byte("# only a comment\n"), 0o644)
	_, err = ReadImages(empty)
	r.CheckBool("a file naming no environments is an error", err != nil, true)
	_, err = ReadImages(filepath.Join(dir, "absent.txt"))
	r.CheckBool("a missing images file is an error", err != nil, true)

	// --- envInt, through the environment ------------------------------------
	// ⚠ An unparsable or negative value falls back to the DEFAULT rather than
	// failing, because Load has no way to report. The flag path is where a bad
	// value is rejected out loud, and this asserts the quiet half.
	for _, bad := range []string{"", "-1", "banana"} {
		_ = os.Setenv("PGB_OPT_TLS_RESERVE", bad)
		r.Check("PGB_OPT_TLS_RESERVE="+quoteEmpty(bad)+" falls back to the default",
			itoa(Load("/self").TLSReserve), "0")
	}
	_ = os.Setenv("PGB_OPT_TLS_RESERVE", "4096")
	r.Check("and a good value is taken", itoa(Load("/self").TLSReserve), "4096")

	return r
}

func itoa(n int) string {
	if n == 0 {
		return "0"
	}
	var b []byte
	neg := n < 0
	if neg {
		n = -n
	}
	for n > 0 {
		b = append([]byte{byte('0' + n%10)}, b...)
		n /= 10
	}
	if neg {
		return "-" + string(b)
	}
	return string(b)
}

func quoteEmpty(s string) string {
	if s == "" {
		return "(empty)"
	}
	return s
}
