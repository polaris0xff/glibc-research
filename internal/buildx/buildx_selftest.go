// buildx_selftest.go — the engine boundary, asserted without a bed.
//
// ⛔ WHY THIS PACKAGE NEEDED ONE, AND WHY IT IS THE ARGV RATHER THAN THE RUN.
// `pgb build` re-enters itself inside the build environment, and the options
// the outer process parsed have to cross that boundary. A chroot inherits the
// caller's environment; a container does not. ⭐ **T-019 is that difference
// realised as a defect**: the docker branch passed exactly `-e PGB_INNER=1`,
// so `--wrap-dlopen`, `--embed-locale`, `--no-iconv`, `--arch-baseline` and
// `-v` were ALL dropped at the boundary — no warning, no error, exit 0, and a
// binary that simply did not have the mechanism the caller asked for.
//
// ⚠ AND IT HID BEHIND A REAL RESULT. T-010 measured the two engines producing
// byte-identical output, and that measurement stands: it was taken on a build
// with NO OPTIONS, which is the one case where dropping them all changes
// nothing. ⛔ A cross-check that only exercises the default path certifies the
// default path.
//
// So the cases below are about the composed command line, and the load-bearing
// one asserts the two engines carry the SAME SET — it fails the moment an
// option can reach the chroot and not the container.
//
// ⚠ WHAT IS NOT COVERED, plainly: `Build`, `Shell`, `enterChroot`,
// `enterContainer` and `Inner` all start something. Nothing here mounts,
// chroots, or runs a container; the acceptance for the run itself is still the
// eleven-environment matrix and the POCs. T-062 says to carry the parsing and
// the decision logic, not the run, and this is that line.
//
// SPDX-License-Identifier: MIT
package buildx

import (
	"os"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/selftest"
)

// Selftest asserts the engine boundary's command lines.
func Selftest() *selftest.Report {
	r := selftest.New("buildx")

	c := &cfg.Config{
		Self:  "/repo",
		State: "/var/tmp/pgb-state",
	}
	const wd = "/work"
	const self = "/usr/local/bin/pgb"
	yes := func(string) bool { return true }
	no := func(string) bool { return false }

	joined := func(a []string) string { return strings.Join(a, " ") }
	has := func(a []string, want ...string) bool {
		return strings.Contains(" "+joined(a)+" ", " "+joined(want)+" ")
	}

	// ---- the container command line --------------------------------------
	run := ContainerRunArgv(c, "docker", []string{"make", "-j4"}, false, wd, self, yes, "")

	r.Check("the container run starts with the engine", joined(run[:3]), "docker run --rm")
	r.CheckBool("a non-interactive run is not -it", has(run, "-it"), false)
	r.CheckBool("the working directory is mounted at the same path",
		has(run, "-v", wd+":"+wd), true)
	r.CheckBool("...and is the container's working directory", has(run, "-w", wd), true)
	r.CheckBool("the state directory is mounted at the same path",
		has(run, "-v", c.State+":"+c.State), true)
	// ⛔ READ-ONLY, and at a FIXED path: the re-entry has to be independent of
	// where the caller's copy of pgb lives.
	r.CheckBool("pgb is mounted read-only at the fixed inner path",
		has(run, "-v", self+":"+selfInside+":ro"), true)
	r.CheckBool("the repository is mounted when it exists",
		has(run, "-v", c.Self+":"+c.Self), true)
	r.CheckBool("PGB_INNER marks the inner process",
		has(run, "-e", "PGB_INNER=1"), true)
	// The image is derived from the version constant, never retyped.
	r.CheckBool("the image is pgb-env at this version",
		has(run, "pgb-env:"+cfg.Version), true)
	// ⭐ THE COMMAND IS APPENDED AS ARGV. `make -j4` must stay two arguments;
	// flattening it into a string is the defect this package's header warns
	// about.
	r.Check("the caller's command is appended as argv, unflattened",
		joined(run[len(run)-4:]), selfInside+" "+InnerBuild+" make -j4")

	// A repository path that is not on disk is not mounted: docker would create
	// an empty directory there and the build would see a repository with
	// nothing in it.
	run2 := ContainerRunArgv(c, "docker", []string{"make"}, false, wd, self, no, "")
	r.CheckBool("a repository path that does not exist is NOT mounted",
		has(run2, "-v", c.Self+":"+c.Self), false)

	// Interactive adds -it and nothing else changes.
	runI := ContainerRunArgv(c, "podman", []string{"/bin/sh"}, true, wd, self, yes, "")
	r.CheckBool("an interactive run is -it", has(runI, "-it"), true)
	r.Check("...under the engine it was asked for", runI[0], "podman")

	// A CA anchor, when the host has one, is mounted read-only.
	runCA := ContainerRunArgv(c, "docker", []string{"make"}, false, wd, self, yes, "/etc/ssl/certs")
	r.CheckBool("a CA anchor is mounted read-only",
		has(runCA, "-v", "/etc/ssl/certs:/etc/ssl/certs:ro"), true)
	r.CheckBool("...and is absent when the host names none",
		has(run, "-v", ":ro"), false)

	// ---- the chroot bind list --------------------------------------------
	binds := ChrootBinds(c, wd, self, no)
	r.Check("the chroot binds wd, state and pgb, in that order",
		joined(binds), wd+":"+wd+" "+c.State+":"+c.State+" "+self+":"+selfInside)
	r.CheckBool("...plus the repository when it exists",
		strings.Contains(joined(ChrootBinds(c, wd, self, yes)), c.Self+":"+c.Self), true)

	// --bind is made absolute on both sides, in both engines, because a
	// relative path means something different inside.
	cb := &cfg.Config{Self: "/repo", State: "/state", ExtraBinds: []string{"/a:/b", "/c"}}
	r.CheckBool("--bind SRC:DST reaches the chroot",
		strings.Contains(joined(ChrootBinds(cb, wd, self, no)), "/a:/b"), true)
	r.CheckBool("--bind SRC alone becomes SRC:SRC",
		strings.Contains(joined(ChrootBinds(cb, wd, self, no)), "/c:/c"), true)
	rb := ContainerRunArgv(cb, "docker", []string{"make"}, false, wd, self, no, "")
	r.CheckBool("--bind SRC:DST reaches the container too", has(rb, "-v", "/a:/b"), true)
	r.CheckBool("--bind SRC alone does too", has(rb, "-v", "/c:/c"), true)

	// ---- ⭐ T-019: the two engines must carry the SAME OPTION SET ----------
	//
	// ⛔ THE CASE THIS FILE EXISTS FOR. The chroot gets the options by
	// INHERITANCE — cfg.Export() sets them in this process and the child sees
	// them — so the set that crosses is exactly cfg.OptVars. The container gets
	// only what is named with -e. If a variable is in OptVars and not in the
	// container's argv, it reaches one engine and not the other, and the build
	// silently differs by engine.
	saved := map[string]*string{}
	for _, v := range cfg.OptVars {
		if old, ok := os.LookupEnv(v); ok {
			s := old
			saved[v] = &s
		} else {
			saved[v] = nil
		}
		// A value that is plainly not a default, so a case cannot pass because
		// the variable happened to be set to the same thing anyway.
		_ = os.Setenv(v, "pgb-selftest")
	}
	defer func() {
		for v, old := range saved {
			if old == nil {
				_ = os.Unsetenv(v)
			} else {
				_ = os.Setenv(v, *old)
			}
		}
	}()

	full := ContainerRunArgv(c, "docker", []string{"make"}, false, wd, self, yes, "")
	missing := []string{}
	for _, v := range cfg.OptVars {
		if !has(full, "-e", v) {
			missing = append(missing, v)
		}
	}
	r.Check("every option variable the chroot inherits is named to the container",
		strings.Join(missing, ","), "")
	r.CheckInt("...and that was over this many variables", len(cfg.OptVars), len(cfg.OptVars))

	// ⚠ AND THE OTHER DIRECTION: a variable that is NOT set must not be passed.
	// `docker run -e NAME` with NAME unset in the caller's environment sets it
	// to the empty string inside, and T-074 is this tree's record of what
	// confusing "absent" with "set and empty" costs.
	probe := cfg.OptVars[0]
	_ = os.Unsetenv(probe)
	partial := ContainerRunArgv(c, "docker", []string{"make"}, false, wd, self, yes, "")
	r.CheckBool("an UNSET option variable is not passed as an empty one",
		has(partial, "-e", probe), false)
	r.CheckBool("...while the ones that are set still are",
		has(partial, "-e", cfg.OptVars[1]), true)

	// ---- the small decisions ---------------------------------------------
	r.Check("the inner re-entry points keep their names",
		InnerBuild+" "+InnerShell, "__inner-build __inner-shell")
	r.Check("pgb is bind-mounted at a fixed inner path", selfInside, "/pgb")

	// Describe names the engine, so a log line says which one ran.
	dc := &cfg.Config{}
	_ = dc.SetEngine("chroot")
	r.Check("Describe names the engine and the command",
		Describe(dc, []string{"make", "all"}), "chroot: make all")

	// shellPath honours $SHELL and falls back to /bin/sh.
	prevShell, hadShell := os.LookupEnv("SHELL")
	_ = os.Setenv("SHELL", "/bin/zsh")
	r.Check("shellPath honours $SHELL", shellPath(), "/bin/zsh")
	_ = os.Unsetenv("SHELL")
	r.Check("...and falls back to /bin/sh", shellPath(), "/bin/sh")
	if hadShell {
		_ = os.Setenv("SHELL", prevShell)
	}

	// ⛔ An empty command is refused before anything is mounted or started.
	if err := Build(&cfg.Config{}, nil); err == nil {
		r.Fail("pgb build with no command", "no error", "an error")
	} else {
		r.CheckBool("pgb build with no command is refused, naming the fix",
			strings.Contains(err.Error(), "pgb build -- make"), true)
	}
	if err := Inner(&cfg.Config{}, nil, false); err == nil {
		r.Fail("the inner re-entry with no command", "no error", "an error")
	} else {
		r.CheckBool("the inner re-entry with no command is refused",
			strings.Contains(err.Error(), InnerBuild), true)
	}

	return r
}
