// Package envx builds and describes the pinned build environment.
//
// The stamp is what an environment was BUILT FROM, in one line. One function
// produces it and one consumes it, so the writer and the checker cannot drift.
// It matters because `pgb env create` builds for whichever engine is chosen at
// that moment and a later `pgb build` chooses again: the engines keep
// independent environments, and merely starting a container daemon changes
// which one a command uses.
//
// SPDX-License-Identifier: MIT
package envx

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/proc"
)

var log = logx.New("env")

// Stamp is the parsed form of an environment description.
type Stamp struct {
	Image    string // image@digest
	Iconv    bool
	Packages []string
	Pip      []string
}

// Want is the stamp these settings describe.
func Want(c *cfg.Config) Stamp {
	return Stamp{
		Image:    c.EnvImage + "@" + c.EnvDigest,
		Iconv:    c.UseIconv,
		Packages: sortedUnique(c.EnvPackages),
		Pip:      sortedUnique(c.EnvPip),
	}
}

// String renders the stamp in the order the fields already hold. It does not
// sort: Want and ParseStamp do that on the way in, which is what makes
// reordering PGB_ENV_PACKAGES not a difference. A Stamp built by hand renders
// whatever order it was given.
func (s Stamp) String() string {
	return fmt.Sprintf("%s iconv=%s packages=[%s] pip=[%s]",
		s.Image, boolBit(s.Iconv), strings.Join(s.Packages, " "), strings.Join(s.Pip, " "))
}

// ParseStamp reads the rendered form back. Fields are bounded by their own
// brackets, so a second bracketed field cannot be swallowed by the first.
func ParseStamp(s string) (Stamp, bool) {
	s = strings.TrimSpace(s)
	if s == "" {
		return Stamp{}, false
	}
	var st Stamp
	st.Image, _, _ = strings.Cut(s, " ")
	if v, ok := field(s, "iconv="); ok {
		st.Iconv = v == "1"
	}
	if v, ok := bracketField(s, "packages="); ok {
		st.Packages = sortedUnique(strings.Fields(v))
	}
	if v, ok := bracketField(s, "pip="); ok {
		st.Pip = sortedUnique(strings.Fields(v))
	}
	return st, st.Image != ""
}

func field(s, key string) (string, bool) {
	_, rest, ok := strings.Cut(s, key)
	if !ok {
		return "", false
	}
	if j := strings.IndexAny(rest, " \t"); j >= 0 {
		rest = rest[:j]
	}
	return rest, true
}

func bracketField(s, key string) (string, bool) {
	i := strings.Index(s, key+"[")
	if i < 0 {
		return "", false
	}
	rest := s[i+len(key)+1:]
	before, _, ok := strings.Cut(rest, "]")
	if !ok {
		return "", false
	}
	return before, true
}

// Have reads the stamp an engine's environment actually carries. An empty
// return means "no environment", never "matches".
func Have(c *cfg.Config, e cfg.Engine) Stamp {
	switch e {
	case cfg.EngineChroot:
		root := c.EnvRoot()
		if fi, err := os.Stat(root); err != nil || !fi.IsDir() {
			return Stamp{}
		}
		if b, err := os.ReadFile(filepath.Join(root, ".pgb-env-stamp")); err == nil {
			if st, ok := ParseStamp(string(b)); ok {
				return st
			}
		}
		// An environment created before the stamp existed is not unusable:
		// .pgb-env records the image, digest and package set, and the
		// archive's presence answers the iconv question.
		desc, err := readDescription(filepath.Join(root, ".pgb-env"))
		if err != nil {
			return Stamp{}
		}
		st := Stamp{
			Image:    desc["image"] + "@" + desc["digest"],
			Packages: sortedUnique(strings.Fields(desc["packages"])),
			Pip:      sortedUnique(strings.Fields(desc["pip"])),
		}
		if _, err := os.Stat(filepath.Join(root, c.LibiconvPrefix, "lib", "libiconv.a")); err == nil {
			st.Iconv = true
		}
		return st
	case cfg.EngineDocker, cfg.EnginePodman:
		out, code := proc.CaptureAllowFail(string(e), "image", "inspect",
			"--format", `{{index .Config.Labels "org.pgb.stamp"}}`, "pgb-env:"+cfg.Version)
		if code != 0 {
			return Stamp{}
		}
		out = strings.TrimSpace(out)
		if out == "" || out == "<no value>" {
			return Stamp{}
		}
		st, _ := ParseStamp(out)
		return st
	}
	return Stamp{}
}

// ServesNamedEnv reports whether an engine reads PGB_ENV_NAME. The chroot
// engine builds in RootfsDir/EnvName, so the name selects the environment; the
// container engines build from the image pgb-env:<version> and never look at
// it. Split out from RequireCurrent so the rule can be asserted without a
// daemon, a rootfs or a network.
func ServesNamedEnv(e cfg.Engine) bool { return e == cfg.EngineChroot }

// RequireCurrent refuses a build against an environment that is not what the
// current settings describe, and names the difference.
//
// Each field is compared with the rule that field has:
//
//	image     differs at all  -> fatal, it is a different glibc
//	packages  wanted missing  -> fatal; extra -> a note
//	pip       wanted missing  -> fatal; extra -> a note
//	iconv     want 1, have 0  -> fatal; want 0, have 1 -> nothing, because
//	                             --no-iconv is a build option and an
//	                             environment that has libiconv serves it fine
func RequireCurrent(c *cfg.Config, e cfg.Engine) error {
	if e == cfg.EngineHost {
		return nil
	}
	// A named environment is a chroot directory, and the container engines
	// never read the name -- they build from the image pgb-env:<version>. So a
	// caller that named one and got a container engine has its choice dropped,
	// and the build runs in the DEFAULT environment while reporting nothing.
	if !ServesNamedEnv(e) && c.EnvName != cfg.DefaultEnvName {
		fmt.Fprintf(os.Stderr, "pgb: engine %s cannot serve the named environment %s.\n", e, c.EnvName)
		fmt.Fprintf(os.Stderr, "     PGB_ENV_NAME names a directory under %s, which only the\n", c.RootfsDir)
		fmt.Fprintf(os.Stderr, "     chroot engine reads; %s builds from the image pgb-env:%s.\n", e, cfg.Version)
		fmt.Fprintln(os.Stderr, "     The build would SUCCEED, in the environment this pin names by")
		fmt.Fprintln(os.Stderr, "     default -- which no output of it would ever show.")
		return fail.Cannot("name it for an engine that reads it: pgb --engine chroot build ...")
	}
	want := Want(c)
	have := Have(c, e)
	if have.Image == "" {
		var other []string
		for _, alt := range []cfg.Engine{cfg.EngineChroot, cfg.EngineDocker, cfg.EnginePodman} {
			if alt == e {
				continue
			}
			if Have(c, alt).Image != "" {
				other = append(other, string(alt))
			}
		}
		fmt.Fprintf(os.Stderr, "pgb: engine %s has no build environment.\n", e)
		fmt.Fprintf(os.Stderr, "     chosen engine: %s\n", e)
		if len(other) > 0 {
			fmt.Fprintf(os.Stderr,
				"     but these engines DO have one: %s -- pgb --engine <one of those> build ...\n",
				strings.Join(other, " "))
		}
		return fail.Cannot("run: pgb env create   (or pgb --engine ... build)")
	}
	if want.String() == have.String() {
		return nil
	}

	var notes []string
	fatal := false
	if want.Image != have.Image {
		fatal = true
		notes = append(notes,
			fmt.Sprintf("     image    wanted %s\n              have   %s", want.Image, have.Image))
	}
	if want.Iconv && !have.Iconv {
		fatal = true
		notes = append(notes,
			"     iconv    this build links static libiconv and the environment has none")
	}
	missing := onlyIn(want.Packages, have.Packages)
	extra := onlyIn(have.Packages, want.Packages)
	if len(missing) > 0 {
		fatal = true
		notes = append(notes, "     packages MISSING from the environment: "+strings.Join(missing, " "))
	}
	missingPip := onlyIn(want.Pip, have.Pip)
	if len(missingPip) > 0 {
		fatal = true
		notes = append(notes, "     pip packages MISSING from the environment: "+strings.Join(missingPip, " "))
	}

	if !fatal {
		if len(extra) > 0 {
			log.Infof("environment has packages these settings do not name: %s", strings.Join(extra, " "))
		}
		return nil
	}

	fmt.Fprintf(os.Stderr, "pgb: the %s build environment cannot serve these settings.\n", e)
	for _, n := range notes {
		fmt.Fprintln(os.Stderr, n)
	}
	if len(extra) > 0 {
		fmt.Fprintf(os.Stderr, "     (it also has, harmlessly: %s)\n", strings.Join(extra, " "))
	}
	// Two different consequences, so two different sentences. A missing tool
	// makes the build fail confusingly inside somebody else's makefile; a wrong
	// image makes it succeed against a glibc the pin does not describe.
	if want.Image != have.Image {
		fmt.Fprintln(os.Stderr, "     The build would SUCCEED, against a glibc this pin does not")
		fmt.Fprintln(os.Stderr, "     describe -- which no output of it would ever show.")
	} else {
		fmt.Fprintln(os.Stderr, "     A build would fail inside your build system with whatever the")
		fmt.Fprintln(os.Stderr, "     missing tool says, so it is refused here instead.")
	}
	// ⚠ THE REMEDY IS PER-ENGINE, and naming the wrong one sends the reader to
	// a directory that is already correct. The chroot environment lives under
	// a name derived from the pin, so a pin move gives it a NEW directory and
	// there is nothing to delete; the docker one is a single tagged image that
	// `env create` overwrites in place. Reported on 2026-09-02 when a real pin
	// move left the docker environment stale and this line pointed at a chroot
	// root that had just been built correctly.
	if e == cfg.EngineChroot {
		return fail.Cannot("rebuild it: pgb --engine chroot env create"+
			"   (if %s already exists at the old settings, delete it first)", c.EnvRoot())
	}
	return fail.Cannot("rebuild it: pgb --engine %s env create", e)
}

// onlyIn returns the words in a that are not in b. An empty b must not make
// every word match.
func onlyIn(a, b []string) []string {
	set := make(map[string]bool, len(b))
	for _, s := range b {
		set[s] = true
	}
	var out []string
	for _, s := range a {
		if s != "" && !set[s] {
			out = append(out, s)
		}
	}
	return out
}

func sortedUnique(in []string) []string {
	seen := map[string]bool{}
	out := make([]string, 0, len(in))
	for _, s := range in {
		if s == "" || seen[s] {
			continue
		}
		seen[s] = true
		out = append(out, s)
	}
	sort.Strings(out)
	return out
}

func boolBit(b bool) string {
	if b {
		return "1"
	}
	return "0"
}

// readDescription parses the human-readable .pgb-env file.
func readDescription(path string) (map[string]string, error) {
	b, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	out := map[string]string{}
	for line := range strings.SplitSeq(string(b), "\n") {
		k, v, ok := strings.Cut(line, ": ")
		if !ok {
			continue
		}
		if _, seen := out[k]; !seen {
			out[k] = strings.TrimSpace(v)
		}
	}
	return out, nil
}
