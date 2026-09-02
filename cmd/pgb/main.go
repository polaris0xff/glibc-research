// Command pgb builds portable static glibc binaries.
//
// One statically linked executable: the driver, the compiler wrappers it puts
// on PATH, the planner, the verifier and the bundler. It carries the C runtime
// sources it compiles into a build, so there is nothing to clone and nothing
// to install beside it.
//
// SPDX-License-Identifier: MIT
package main

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/wrapper"
)

const usage = `pgb -- build portable static glibc binaries.

Produces an ordinary Linux ELF executable, statically linked against glibc,
that runs unchanged on glibc and musl distributions. No launcher, no AppDir,
no packaging format, no runtime beside it: one file you copy and run.

WHAT MAKES THIS DIFFERENT FROM ` + "`gcc -static`" + `, WHICH IS THE WHOLE POINT.
A plain ` + "`gcc -static`" + ` glibc binary is NOT self-contained. Measured across 11
pinned distributions in experiments/:

  - it reads the HOST /etc/nsswitch.conf and dlopen()s the modules named
    there, pulling a second libc into the process. That kills it outright on
    Arch Linux and openSUSE Leap 15.6;
  - it reaches every character encoding except a small builtin set through
    dlopen'd gconv modules, which kills it on Debian and Ubuntu and silently
    costs 11 of 12 tested encodings everywhere else;
  - it gets an ASCII charset from setlocale() on every musl host.

pgb closes all three. ` + "`pgb explain`" + ` prints exactly how, and ` + "`pgb verify`" + `
re-measures it on your binary rather than asking you to believe this text.

COMMANDS
  pgb doctor                 what this machine can and cannot do
  pgb explain                every flag injected and why. No black box
  pgb env create             build the pinned, controlled build environment
  pgb env info               what the environment is, exactly
  pgb bootstrap              a fresh machine: build env, test bed, in parallel
  pgb shell                  an interactive shell inside it
  pgb build [--] CMD...      run a build with the portable toolchain injected
  pgb cc-dir                 print the wrapper directory, for hand-driving
  pgb verify BIN             inspect a binary AND run it across the matrix
  pgb nix SUBCOMMAND         use nixpkgs as the planner: plan, fetch, build
  pgb bundle SUBCOMMAND      the nixpkgs bundler and its reachability sweep
  pgb elf SUBCOMMAND         needed, set-needed: DT_NEEDED inspection
  pgb rootfs SUBCOMMAND      fetch, run: the digest-pinned target bed
  pgb selftest               every carried-in selftest, offline

OPTIONS (build)
  --embed-cacert             find the host's TLS trust store wherever it is,
                             and carry a copy for hosts that have none
  --embed-terminfo           carry a handful of terminal descriptions, used
                             only when the host cannot describe $TERM
  --embed-locale             embed C.UTF-8, materialised only if the host
                             lacks it. Off by default: it is the one
                             mechanism that writes to the filesystem
  --no-iconv                 do not link GNU libiconv
  --arch-baseline LEVEL      x86-64 | x86-64-v2 | x86-64-v3 (default x86-64)
  --bind DIR[:DEST]          expose an extra path inside the environment,
                             repeatable. Needed when sources, a cache or an
                             install prefix live outside the current directory
  --wrap-dlopen NAME=OBJ[,OBJ...]
                             answer dlopen("NAME") from a table compiled in
                             from OBJ, instead of asking the host loader.
                             Repeatable. OBJ is any object or archive the
                             build produced; its exported symbols become the
                             plugin's dlsym table. For programs that load
                             their OWN plugins
  --host-dlopen              load a HOST shared object with pgb's own ELF
                             loader, resolving it against the static glibc
                             already in the binary. The host's ld.so is never
                             consulted and no second libc enters the process.
                             For programs that load somebody ELSE's plugins
  --engine chroot|docker|podman|host

DIAGNOSTICS
  -v, --verbose              narrate every command pgb runs
  --log LEVEL                error | warn | info | debug | trace
  --debug SUBSYS[,SUBSYS]    debug only these subsystems; 'all'; '-name' drops
                             one. PGB_DEBUG does the same from the environment
  --ts / --no-ts             timestamp streamed build output (PGB_TS)
  --ts-columns rel,delta     which stamps: rel delta wall iso epoch
  --ts-heartbeat 30s         say so when a build produces no output for this
                             long. 'off' disables

Exit codes: 0 ok, 1 the operation ran and failed, 2 it could not run.
`

func main() {
	if code := run(); code != 0 {
		os.Exit(code)
	}
}

func run() int {
	// A wrapper invocation is decided by argv[0] before anything else, because
	// the wrappers are this same binary under another name and a build system
	// calls them thousands of times.
	if name := filepath.Base(os.Args[0]); wrapper.IsWrapperName(name) {
		return wrapper.Dispatch(name, os.Args[1:])
	}

	err := dispatch(os.Args[1:])
	if err == nil {
		return 0
	}
	if code, silent := fail.IsSilent(err); silent {
		return code
	}
	fmt.Fprintf(os.Stderr, "pgb: %s\n", err)
	return fail.Code(err)
}

// structuredCommands take subcommand words rather than a user's argv, so
// global options stay recognised among their positionals. For build, verify,
// shell and rootfs the remaining argv belongs to the subcommand or to the
// caller's own program and is passed through untouched — `pgb rootfs run`
// has a --bind of its own, and a global one would eat it.
var structuredCommands = map[string]bool{
	"env": true, "nix": true, "bundle": true, "elf": true,
	"selftest": true, "doctor": true, "explain": true, "cc-dir": true,
	"bootstrap": true, "help": true, "debug": true, "build-root": true,
}

// parser threads the argv walk so option handlers can consume a value.
type parser struct {
	argv []string
	i    int
	c    *cfg.Config

	verbose   bool
	logLevel  string
	debugSpec string
	engine    string
	tsSet     bool
	tsOn      bool
	tsColumns string
	tsBeat    string
	helped    bool
}

// value consumes the argument after the current flag.
func (p *parser) value(flag string) (string, error) {
	if p.i+1 >= len(p.argv) {
		return "", fail.Cannot("%s needs a value", flag)
	}
	p.i++
	return p.argv[p.i], nil
}

// option handles one global flag. It reports whether the flag was recognised.
func (p *parser) option(a string) (bool, error) {
	var err error
	var v string
	switch a {
	case "-v", "--verbose":
		p.verbose = true
	case "--embed-locale":
		p.c.EmbedLocale = true
	case "--embed-cacert":
		p.c.EmbedCacert = true
	case "--embed-terminfo":
		p.c.EmbedTerminfo = true
	case "--no-iconv":
		p.c.UseIconv = false
	case "--ts":
		p.tsSet, p.tsOn = true, true
	case "--no-ts":
		p.tsSet, p.tsOn = true, false
	case "-h", "--help":
		fmt.Print(usage)
		p.helped = true
	case "--version":
		logx.Say("pgb %s", cfg.Version)
		p.helped = true
	case "--arch-baseline":
		if v, err = p.value(a); err == nil {
			p.c.ArchBaseline = v
		}
	case "--bind":
		if v, err = p.value(a); err == nil {
			p.c.ExtraBinds = append(p.c.ExtraBinds, v)
		}
	case "--wrap-dlopen":
		if v, err = p.value(a); err == nil {
			p.c.WrapDlopen = append(p.c.WrapDlopen, v)
		}
	case "--host-dlopen":
		p.c.HostDlopen = true
	case "--engine":
		if v, err = p.value(a); err == nil {
			p.engine = v
		}
	case "--log":
		if v, err = p.value(a); err == nil {
			p.logLevel = v
		}
	case "--debug":
		if v, err = p.value(a); err == nil {
			p.debugSpec = v
		}
	case "--ts-columns":
		if v, err = p.value(a); err == nil {
			p.tsColumns = v
		}
	case "--ts-heartbeat":
		if v, err = p.value(a); err == nil {
			p.tsBeat = v
		}
	default:
		return false, nil
	}
	return true, err
}

func dispatch(argv []string) error {
	self, err := selfDir()
	if err != nil {
		return fail.Cannot("cannot locate the pgb tree: %v", err)
	}
	c := cfg.Load(self)
	p := &parser{argv: argv, c: c, verbose: c.Verbose}

	var cmd string
	var rest []string

	for p.i = 0; p.i < len(argv); p.i++ {
		a := argv[p.i]
		if a == "--" {
			// A structured command parses its own `--`, so it is passed
			// through; for build, verify and shell it separates pgb's options
			// from the caller's command and is consumed here.
			if cmd != "" && structuredCommands[cmd] {
				rest = append(rest, "--")
			}
			rest = append(rest, argv[p.i+1:]...)
			p.i = len(argv)
			break
		}
		if !strings.HasPrefix(a, "-") {
			if cmd == "" {
				cmd = a
				// The hidden re-entry points carry their own flags and are
				// composed by pgb itself, so their argv is taken verbatim.
				if strings.HasPrefix(cmd, "__") {
					rest = append(rest, argv[p.i+1:]...)
					p.i = len(argv)
					break
				}
				continue
			}
			if structuredCommands[cmd] {
				rest = append(rest, a)
				continue
			}
			rest = append(rest, argv[p.i:]...)
			p.i = len(argv)
			break
		}
		known, err := p.option(a)
		if err != nil {
			return err
		}
		if p.helped {
			return nil
		}
		if !known {
			// Once a command is named, an option pgb does not know is the
			// subcommand's to parse or to refuse. Before that there is nobody
			// to hand it to.
			if cmd == "" {
				return fail.Cannot("unknown option: %s", a)
			}
			rest = append(rest, a)
		}
	}

	if err := logx.Configure(p.verbose, p.logLevel, p.debugSpec); err != nil {
		return fail.Cannot("%v", err)
	}
	c.Verbose = p.verbose
	if p.engine != "" {
		if err := c.SetEngine(p.engine); err != nil {
			return err
		}
	}
	if p.tsSet {
		_ = os.Setenv("PGB_TS", map[bool]string{true: "1", false: "0"}[p.tsOn])
	}
	if p.tsColumns != "" {
		_ = os.Setenv("PGB_TS_COLUMNS", p.tsColumns)
	}
	if p.tsBeat != "" {
		_ = os.Setenv("PGB_TS_HEARTBEAT", p.tsBeat)
	}
	if err := os.MkdirAll(c.State, 0o755); err != nil && !errors.Is(err, os.ErrExist) {
		logx.Warnf("cannot create %s: %v", c.State, err)
	}
	logx.Debugf("config: %s", c.Describe())

	return runCommand(c, cmd, rest)
}

// selfDir is the directory the repository lives in. A pgb installed on its own
// still works: only commands that read the tree (experiments, references) need
// it, and they say so when it is absent.
func selfDir() (string, error) {
	exe, err := os.Executable()
	if err != nil {
		return "", err
	}
	if resolved, err := filepath.EvalSymlinks(exe); err == nil {
		exe = resolved
	}
	dir := filepath.Dir(exe)
	// A binary built into the repository root, or installed beside it, sees
	// the tree directly. One built into ./bin or ./out sees it one level up.
	for _, cand := range []string{dir, filepath.Dir(dir)} {
		if _, err := os.Stat(filepath.Join(cand, "tool", "runtime")); err == nil {
			return cand, nil
		}
	}
	if v := os.Getenv("PGB_SELF"); v != "" {
		return v, nil
	}
	return dir, nil
}
