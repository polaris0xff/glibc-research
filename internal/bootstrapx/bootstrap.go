// Package bootstrapx gets a fresh machine ready, in parallel, in one command.
//
// Nothing this project needs survives a fresh container: every session starts
// with 0 of 11 rootfs, no build environment, no static libiconv and no nix.
// Those steps are independent and mostly network-bound, so they run together
// and the caller reads the documentation while they do.
//
// Every "present" answer is a file on disk. A bootstrap that trusts a marker
// it wrote itself reports success after a step that half-ran.
//
// SPDX-License-Identifier: MIT
package bootstrapx

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"syscall"
	"time"

	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/envx"
	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/nixx"
	"github.com/polaris0xff/glibc-research/internal/proc"
	"github.com/polaris0xff/glibc-research/internal/rootfs"
	"github.com/polaris0xff/glibc-research/internal/selftest"
)

// Options controls one bootstrap run.
type Options struct {
	Nix, Bed, Env, Docker bool
	Detach                bool
	Check                 bool
	LogDir                string
	MinFreeGiB            int
}

// DefaultOptions turns everything on.
func DefaultOptions() Options {
	return Options{
		Nix: true, Bed: true, Env: true, Docker: true,
		LogDir: envOr("PGB_BOOTSTRAP_LOGS", "/var/tmp/pgb-bootstrap"),
		// Measured: the bed is 2.3 GiB, /nix reaches 1.4, the chroot
		// environment about 1.5, and a POC build wants several more.
		MinFreeGiB: 10,
	}
}

func envOr(name, def string) string {
	if v := os.Getenv(name); v != "" {
		return v
	}
	return def
}

// state is what this machine has.
type state struct {
	Nix       bool
	Env       bool
	Dockerd   bool
	DockerEnv bool
	BedHave   int
	BedTotal  int
	FreeGiB   int
}

func inspect(c *cfg.Config) state {
	var s state
	_, s.Nix = nixx.NixBin()
	if fi, err := os.Stat(filepath.Join(c.EnvRoot(), ".pgb-env-stamp")); err == nil && fi.Size() > 0 {
		s.Env = true
	}
	if proc.Look("docker") {
		if r, err := proc.Quiet("docker", "info"); err == nil && !r.Failed() {
			s.Dockerd = true
		}
	}
	if s.Dockerd {
		s.DockerEnv = envx.Have(c, cfg.EngineDocker).Image != ""
	}
	if images, err := cfg.ReadImages(c.ImagesFile()); err == nil {
		s.BedTotal = len(images)
		s.BedHave = rootfs.Present(images, c.RootfsDir)
	}
	s.FreeGiB = freeGiB("/")
	return s
}

func freeGiB(path string) int {
	var st syscall.Statfs_t
	if err := syscall.Statfs(path, &st); err != nil {
		return -1
	}
	return int(uint64(st.Bavail) * uint64(st.Bsize) / (1 << 30))
}

// Report prints what is present and reports whether the machine is ready.
//
// Ready does not require nix: the plan route works without one for a package
// whose derivation is cached, and a committed plan needs none at all. It does
// require that the engine detection will choose has an environment.
func Report(c *cfg.Config) bool {
	s := inspect(c)
	logx.Say("  %-22s %s", "nix", presentAbsent(s.Nix))
	logx.Say("  %-22s %s", "build env (chroot)", presentABSENT(s.Env))
	logx.Say("  %-22s %d of %d rootfs", "test bed", s.BedHave, s.BedTotal)
	logx.Say("  %-22s %s", "dockerd", runningOrNot(s.Dockerd))
	logx.Say("  %-22s %s", "build env (docker)", presentAbsent(s.DockerEnv))
	logx.Say("  %-22s %d GiB", "free disk", s.FreeGiB)

	ready := s.Env && s.BedTotal > 0 && s.BedHave == s.BedTotal
	if s.Dockerd && !s.DockerEnv {
		// Starting the daemon is what makes engine detection choose docker, so
		// a running daemon with no docker environment makes every build refuse.
		logx.Say("")
		logx.Warnf("dockerd is RUNNING and the docker environment is MISSING.")
		logx.Warnf("  engine detection prefers docker, so every pgb build will refuse.")
		logx.Warnf("  Fix it with either:")
		logx.Warnf("    pgb --engine docker env create")
		logx.Warnf("    pgb --engine chroot build ...   (per invocation)")
		ready = false
	}
	return ready
}

func presentAbsent(b bool) string {
	if b {
		return "present"
	}
	return "absent"
}

func presentABSENT(b bool) string {
	if b {
		return "present"
	}
	return "ABSENT"
}

func runningOrNot(b bool) string {
	if b {
		return "running"
	}
	return "not running"
}

// Run prepares the machine.
func Run(c *cfg.Config, o Options) error {
	if o.Check {
		logx.Say("pgb bootstrap: what this machine has")
		if Report(c) {
			logx.Say("  VERDICT: ready to build and verify.")
			return nil
		}
		logx.Say("  VERDICT: NOT ready. Run: pgb bootstrap")
		return fail.Exit(1)
	}

	logx.Say("pgb bootstrap: preparing a fresh machine")
	logx.Say("")
	if err := preflight(c, o); err != nil {
		return err
	}
	if err := os.MkdirAll(o.LogDir, 0o755); err != nil {
		return fail.Cannot("cannot write %s: %v", o.LogDir, err)
	}

	s := inspect(c)
	// dockerd first: it takes seconds, and whether it comes up decides whether
	// the docker environment is one of the parallel jobs.
	if o.Docker && proc.Look("docker") && !s.Dockerd {
		startDockerd(o)
		s = inspect(c)
	}

	type job struct {
		name string
		run  func(logFile string) error
	}
	var jobs []job

	if o.Nix {
		switch {
		case s.Nix:
			logx.Say("  nix already present, skipping")
		case nixInstallIsSafe(s.Nix):
			// A third-party script fetched over the network and run as root,
			// here only because the operator authorised it by name. Its first
			// action is `rm -rf /nix`, which is correct on a machine with no
			// nix and destructive on one with a half-installed store.
			logx.Say("  starting nix     third-party installer, run as ROOT:")
			logx.Say("                   pkgforge/devscripts Linux/install_nix.sh")
			jobs = append(jobs, job{"nix", installNix})
		default:
			logx.Warnf("/nix exists but nix does not run: NOT running the installer.")
			logx.Warnf("  its first action is 'rm -rf /nix', which would destroy a store")
			logx.Warnf("  this command did not create. Remove /nix by hand if that is")
			logx.Warnf("  really what you want, then re-run.")
		}
	}

	// The chroot environment is created with the engine NAMED. Calling a
	// detection that prefers docker right after starting dockerd builds a
	// second docker environment and leaves the chroot one absent.
	if o.Env {
		if s.Env {
			logx.Say("  chroot environment already present, skipping")
		} else {
			jobs = append(jobs, job{"env", func(string) error {
				sub := *c
				_ = sub.SetEngine(string(cfg.EngineChroot))
				return envx.Create(&sub)
			}})
		}
	}
	if o.Bed {
		if s.BedTotal > 0 && s.BedHave == s.BedTotal {
			logx.Say("  test bed already complete, skipping")
		} else {
			jobs = append(jobs, job{"bed", func(string) error {
				images, err := cfg.ReadImages(c.ImagesFile())
				if err != nil {
					return err
				}
				return rootfs.Fetch(images, rootfs.FetchOptions{
					Dest: c.RootfsDir, IfMissing: true,
				})
			}})
		}
	}
	if o.Docker && s.Dockerd {
		if s.DockerEnv {
			logx.Say("  docker environment already present, skipping")
		} else {
			jobs = append(jobs, job{"env-docker", func(string) error {
				sub := *c
				_ = sub.SetEngine(string(cfg.EngineDocker))
				return envx.Create(&sub)
			}})
		}
	}

	if len(jobs) == 0 {
		logx.Say("")
		logx.Say("nothing to do -- this machine is already set up.")
		if Report(c) {
			return nil
		}
		return fail.Exit(1)
	}

	if o.Detach {
		// Detaching means re-execing this same binary without --detach, with
		// its output going to the log directory.
		return detach(o)
	}

	logx.Say("")
	logx.Say("  running in parallel. Read docs/AGENTS.md while this happens --")
	logx.Say("  that is the point of doing it this way.")
	logx.Say("")

	var wg sync.WaitGroup
	results := make([]error, len(jobs))
	for i, j := range jobs {
		wg.Add(1)
		go func(i int, j job) {
			defer wg.Done()
			logFile := filepath.Join(o.LogDir, j.name+".log")
			f, err := os.Create(logFile)
			if err != nil {
				results[i] = err
				return
			}
			defer f.Close()
			logx.Say("  starting %s  (log: %s)", j.name, logFile)
			results[i] = runCaptured(f, func() error { return j.run(logFile) })
		}(i, j)
	}
	wg.Wait()

	failed := false
	for i, j := range jobs {
		if results[i] == nil {
			logx.Say("  %-22s ok", j.name)
			continue
		}
		logx.Say("  %-22s FAILED: %v -- last lines:", j.name, results[i])
		tail(filepath.Join(o.LogDir, j.name+".log"), 8)
		failed = true
	}

	logx.Say("")
	if Report(c) && !failed {
		logx.Say("  VERDICT: ready to build and verify.")
		logx.Say("  engine pgb will pick: %s", c.Engine())
		return nil
	}
	logx.Say("  VERDICT: NOT ready. The logs above say which step, and re-running")
	logx.Say("  this command repeats only what is missing.")
	return fail.Exit(1)
}

// runCaptured points stdout and stderr at a log file for the duration of one
// job, so a parallel step's output lands in its own file.
func runCaptured(f *os.File, fn func() error) error {
	oldOut, oldErr := os.Stdout, os.Stderr
	os.Stdout, os.Stderr = f, f
	defer func() { os.Stdout, os.Stderr = oldOut, oldErr }()
	return fn()
}

func preflight(c *cfg.Config, o Options) error {
	fatal := false
	for _, t := range []string{"curl", "tar", "xz"} {
		if !proc.Look(t) {
			logx.Warnf("missing: %s", t)
			fatal = true
		}
	}
	if _, err := os.Stat(c.ImagesFile()); err != nil {
		logx.Warnf("missing: %s", c.ImagesFile())
		fatal = true
	}
	if o.Env || o.Bed {
		if os.Geteuid() != 0 {
			logx.Warnf("the chroot bed needs root (euid = %d)", os.Geteuid())
			fatal = true
		}
		if !proc.Look("unshare") {
			logx.Warnf("missing: unshare")
			fatal = true
		}
	}
	free := freeGiB("/")
	if free >= 0 && free < o.MinFreeGiB {
		logx.Warnf("only %d GiB free on /; this needs about %d.", free, o.MinFreeGiB)
		logx.Warnf("the bed is 2.3 GiB, /nix reaches 1.4, the chroot env about 1.5,")
		logx.Warnf("and a POC build wants several more.")
		fatal = true
	}
	if fatal {
		return fail.Cannot("refusing to start. Nothing was changed.")
	}
	logx.Say("  preflight ok: %d GiB free, root, unshare, curl", free)
	return nil
}

func startDockerd(o Options) {
	logFile := filepath.Join(o.LogDir, "dockerd.log")
	_ = os.MkdirAll(o.LogDir, 0o755)
	f, err := os.Create(logFile)
	if err != nil {
		logx.Warnf("cannot write %s: %v", logFile, err)
		return
	}
	logx.Say("  starting dockerd  (log: %s)", logFile)
	cmd := &proc.Cmd{Argv: []string{"dockerd"}, Stdout: f, Stderr: f, Subsys: "bootstrap"}
	go func() {
		defer f.Close()
		_, _ = cmd.Run()
	}()
	for w := 0; w < 30; w++ {
		time.Sleep(time.Second)
		if r, err := proc.Quiet("docker", "info"); err == nil && !r.Failed() {
			logx.Say("  dockerd up after %ds", w+1)
			return
		}
	}
	// Not fatal: a container without the right capabilities cannot run
	// dockerd, and the chroot engine is what every committed number was
	// measured through anyway.
	logx.Warnf("dockerd did not come up in 30s -- continuing with the chroot engine")
	tail(logFile, 3)
}

// nixInstallIsSafe refuses when a store exists but nix does not run: the
// installer's first action would destroy a store this command did not create.
func nixInstallIsSafe(haveNix bool) bool {
	if haveNix {
		return false
	}
	if _, err := os.Stat("/nix"); err == nil {
		return false
	}
	return true
}

const nixInstaller = "https://raw.githubusercontent.com/pkgforge/devscripts/refs/heads/main/Linux/install_nix.sh"

func installNix(string) error {
	dir, err := os.MkdirTemp("", "pgb-nix-install-")
	if err != nil {
		return err
	}
	defer os.RemoveAll(dir)
	script := filepath.Join(dir, "install_nix.sh")
	if r, err := proc.Run("curl", "-qfsSL", nixInstaller, "-o", script); err != nil || r.Failed() {
		return fmt.Errorf("could not fetch the installer")
	}
	if err := os.Chmod(script, 0o755); err != nil {
		return err
	}
	r, err := proc.Run("bash", script)
	if err != nil {
		return err
	}
	if r.Failed() {
		return fmt.Errorf("the installer exited %d", r.Code)
	}
	return nil
}

// detach re-execs this binary in the background with the same options minus
// --detach, so the caller gets its prompt back.
func detach(o Options) error {
	self, err := os.Executable()
	if err != nil {
		return fail.Cannot("cannot locate the running pgb: %v", err)
	}
	argv := []string{self, "bootstrap"}
	if !o.Nix {
		argv = append(argv, "--no-nix")
	}
	if !o.Bed {
		argv = append(argv, "--no-bed")
	}
	if !o.Env {
		argv = append(argv, "--no-env")
	}
	if !o.Docker {
		argv = append(argv, "--no-docker")
	}
	logFile := filepath.Join(o.LogDir, "bootstrap.log")
	f, err := os.Create(logFile)
	if err != nil {
		return fail.Cannot("cannot write %s: %v", logFile, err)
	}
	cmd := &proc.Cmd{Argv: argv, Stdout: f, Stderr: f, Subsys: "bootstrap"}
	go func() {
		defer f.Close()
		_, _ = cmd.Run()
	}()
	// Give the child time to start before this process exits.
	time.Sleep(time.Second)
	logx.Say("")
	logx.Say("started in the background (log: %s). Read docs/AGENTS.md and", logFile)
	logx.Say("TODO/PROGRESS.md, then: pgb bootstrap --check")
	return nil
}

func tail(path string, n int) {
	b, err := os.ReadFile(path)
	if err != nil {
		return
	}
	lines := strings.Split(strings.TrimRight(string(b), "\n"), "\n")
	if len(lines) > n {
		lines = lines[len(lines)-n:]
	}
	for _, l := range lines {
		fmt.Fprintf(os.Stderr, "      %s\n", l)
	}
}

// Selftest exercises the decisions offline, with no side effects.
func Selftest(c *cfg.Config) *selftest.Report {
	r := selftest.New("bootstrap")

	dir, err := os.MkdirTemp("", "pgb-bootstrap-selftest-")
	if err != nil {
		r.Fail("tempdir", err.Error(), "created")
		return r
	}
	defer os.RemoveAll(dir)

	// The image list must ignore comments and blank lines.
	images := filepath.Join(dir, "images.txt")
	_ = os.WriteFile(images, []byte("# a comment\n\nrepo:tag  alpha  musl  sha256:x\nrepo:tag  beta  glibc  sha256:y\n"), 0o644)
	rows, err := cfg.ReadImages(images)
	if err != nil {
		r.Fail("read the image list", err.Error(), "two rows")
		return r
	}
	r.Check("image list ignores comments and blanks", itoa(len(rows)), "2")

	roots := filepath.Join(dir, "roots")
	_ = os.MkdirAll(filepath.Join(roots, "alpha"), 0o755)
	r.Check("bed counts what is on disk", itoa(rootfs.Present(rows, roots)), "1")
	_ = os.MkdirAll(filepath.Join(roots, "beta"), 0o755)
	r.Check("bed count after the second appears", itoa(rootfs.Present(rows, roots)), "2")

	// An environment that half-built has the directory and no stamp, and
	// treating it as present is how a session builds against a rootfs with no
	// compiler in it.
	sub := *c
	sub.RootfsDir = roots
	sub.EnvName = "e"
	_ = os.MkdirAll(filepath.Join(roots, "e"), 0o755)
	r.CheckBool("half-built environment is not present", inspect(&sub).Env, false)
	_ = os.WriteFile(filepath.Join(roots, "e", ".pgb-env-stamp"), []byte("stamp\n"), 0o644)
	r.CheckBool("environment with a stamp is present", inspect(&sub).Env, true)

	// The destructive-install guard is the only check here that protects data
	// rather than a result.
	if _, err := os.Stat("/nix"); err == nil {
		r.CheckBool("installer refused when /nix already exists", nixInstallIsSafe(false), false)
	} else {
		r.CheckBool("installer allowed when there is no /nix", nixInstallIsSafe(false), true)
	}
	r.CheckBool("installer never runs when nix already works", nixInstallIsSafe(true), false)
	return r
}

func itoa(n int) string { return fmt.Sprintf("%d", n) }
