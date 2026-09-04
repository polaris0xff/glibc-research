// Package cfg holds pgb's settings: the pinned build environment, the paths,
// the per-build options, and the engine choice.
//
// Options cross the engine boundary through the environment rather than the
// argv, because `pgb build` re-enters itself inside the chroot or container
// with only the user's command. Export() writes them; Load() reads them back.
//
// SPDX-License-Identifier: MIT
package cfg

import (
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/proc"
)

// Version is the tool's version and the tag of the environment image it builds.
const Version = "0.1.0"

// Engine names where a build runs.
type Engine string

const (
	EngineChroot Engine = "chroot"
	EngineDocker Engine = "docker"
	EnginePodman Engine = "podman"
	EngineHost   Engine = "host"
)

// Defaults for the pinned build environment.
//
// ⛔ THE PIN IS A FLOOR AND A CEILING, AND THEY POINT IN OPPOSITE DIRECTIONS.
//
//	FLOOR    glibc 2.34 made the `files` and `dns` NSS services builtin.
//	         Below it __nss_configure_lookup only MOVES the dlopen, which
//	         experiments/21- measures against a 2.31 build.
//	CEILING  --host-dlopen needs a HOST object's imports satisfiable by OUR
//	         glibc. Every release the pin does not follow widens the set that
//	         are not: experiments/73-'s class B.
//
// ⭐ Nothing forces the pin to sit near the floor, and T-070 moved it off
// there: 2.36 -> 2.41, debian:12 -> debian:13. All four measured costs were
// zero -- the kernel floor a static binary declares stays at 3.2.0, class C
// (a symbol the newer glibc REMOVED) is empty on all eleven rows at both
// pins, the NSS floor still reports `none` with 21-'s 2.31 arm firing as the
// control, and all ten POCs build and pass under gcc 12.2.0 -> 14.2.0. What
// it buys is class B going 20 -> 5 distinct symbols, the whole __isoc23_*
// family at GLIBC_2.38 among them.
//
// ⚠ debian:13 and debian:trixie are the same release and resolved to this
// same manifest digest on 2026-09-02; the numbered tag is pinned to match
// debian:12's form. Nothing here is resolved at run time -- the digest is.
//
// ⛔ THESE THREE ARE THE ONLY COPIES IN THE TREE. TODO/check.sh asserts it,
// because the previous move found eight experiments and two CI jobs carrying
// their own, and a copy that does not follow measures the old glibc and says
// nothing.
const (
	DefaultEnvImage  = "debian:13"
	DefaultEnvDigest = "sha256:6788062a1b42ac281f053ac876170b79a3eaed5d61383b8ed7eaca6c6965f3b1"
	DefaultEnvName   = "pgb-env-debian13"
)

// DefaultEnvPackages is what an autotools, CMake or meson tarball needs beyond
// a compiler, including the parser generators a configure script asks for.
var DefaultEnvPackages = strings.Fields(`gcc g++ make cmake ninja-build meson
autoconf automake libtool libc6-dev binutils pkg-config file
xz-utils bzip2 patch perl ca-certificates curl zlib1g-dev libffi-dev
bison flex gettext texinfo gperf python3-pip`)

// DefaultEnvPip installs build tools over the distribution's, pinned. Debian
// 12 ships meson 1.0.1 and some projects require >= 1.1; /usr/local/bin
// precedes /usr/bin, so this shadows without removing.
var DefaultEnvPip = []string{"meson==1.9.1"}

// DefaultTerminfoEntries is what --embed-terminfo carries: a handful of
// descriptions, not a database.
var DefaultTerminfoEntries = strings.Fields(`xterm xterm-256color xterm-color
screen screen-256color tmux tmux-256color linux vt100 vt220 ansi rxvt putty dumb`)

// DefaultTzdataZones is what --embed-tzdata carries: a handful of zones, not
// a database. ⛔ tzdata is ~1,800 files and carrying all of them would
// multiply a 2 MB static binary, so this closes the case it carries and no
// other -- experiments/97- measures exactly that. Override with
// PGB_TZDATA_ZONES at build time.
var DefaultTzdataZones = strings.Fields(`UTC Etc/UTC
America/New_York America/Chicago America/Denver America/Los_Angeles
America/Sao_Paulo Europe/London Europe/Berlin Europe/Paris Europe/Moscow
Africa/Cairo Africa/Johannesburg Asia/Kolkata Asia/Shanghai Asia/Tokyo
Asia/Dubai Australia/Sydney Pacific/Auckland`)

// Config is the whole of pgb's settings for one invocation.
type Config struct {
	Self  string // the directory holding the pgb binary and the repository
	State string // PGB_STATE: wrappers, runtime objects, scratch

	RootfsDir      string
	LibiconvPrefix string

	EnvImage    string
	EnvDigest   string
	EnvName     string
	EnvPackages []string
	EnvPip      []string

	// Build options.
	Verbose       bool
	EmbedLocale   bool
	EmbedCacert   bool
	EmbedTerminfo bool
	EmbedTzdata   bool
	EmbedNetdb    bool
	// UTF8Default changes what an UNSET LANG means: C.UTF-8 instead of C.
	// ⛔ It is a change to a DOCUMENTED DEFAULT rather than a repair, which is
	// why it is separate from EmbedLocale and off unless asked for.
	// tool/runtime/pgb-locale.c, experiments/63-.
	UTF8Default  bool
	UseIconv     bool
	ArchBaseline string
	ExtraBinds   []string
	WrapDlopen   []string
	HostDlopen   bool

	// TLSReserve is --tls-reserve: bytes of the executable's OWN thread-local
	// storage set aside for initial-exec TLS in objects --host-dlopen loads.
	// ⛔ Zero by default because EVERY THREAD pays for it whether or not
	// anything is dlopen'd. glibc's own surplus -- ~3,168 bytes of headroom,
	// measured -- is a constant that cannot be enlarged, so this is the only
	// place a module wanting more than that can be put. T-072 route D.
	TLSReserve int

	// SharedWrappers reproduces the pre-T-058 single wrapper directory. Only
	// experiments/87- sets it; nothing in pgb does.
	SharedWrappers bool

	engine Engine // explicit --engine, empty means "detect"
}

// Load builds a Config from the environment. Command-line options are applied
// afterwards by the caller.
func Load(self string) *Config {
	c := &Config{
		Self:           self,
		State:          envOr("PGB_STATE", filepath.Join(xdgState(), "pgb")),
		RootfsDir:      envOr("PGB_ROOTFS_DIR", "/var/lib/pgb-rootfs"),
		LibiconvPrefix: envOr("PGB_LIBICONV_PREFIX", "/opt/pgb-libiconv"),
		EnvImage:       envOr("PGB_ENV_IMAGE", DefaultEnvImage),
		EnvDigest:      envOr("PGB_ENV_DIGEST", DefaultEnvDigest),
		EnvName:        envOr("PGB_ENV_NAME", DefaultEnvName),
		EnvPackages:    fieldsOr("PGB_ENV_PACKAGES", DefaultEnvPackages),
		EnvPip:         fieldsOr("PGB_ENV_PIP", DefaultEnvPip),

		Verbose:       logx.EnvBool("PGB_OPT_VERBOSE", false),
		EmbedLocale:   logx.EnvBool("PGB_OPT_EMBED_LOCALE", false),
		EmbedCacert:   logx.EnvBool("PGB_OPT_EMBED_CACERT", false),
		EmbedTerminfo: logx.EnvBool("PGB_OPT_EMBED_TERMINFO", false),
		EmbedTzdata:   logx.EnvBool("PGB_OPT_EMBED_TZDATA", false),
		EmbedNetdb:    logx.EnvBool("PGB_OPT_EMBED_NETDB", false),
		UTF8Default:   logx.EnvBool("PGB_OPT_UTF8_DEFAULT", false),
		UseIconv:      logx.EnvBool("PGB_OPT_USE_ICONV", true),
		ArchBaseline:  os.Getenv("PGB_OPT_BASELINE"),
		ExtraBinds:    strings.Fields(os.Getenv("PGB_OPT_BINDS")),
		WrapDlopen:    strings.Fields(os.Getenv("PGB_OPT_WRAP_DLOPEN")),
		HostDlopen:    logx.EnvBool("PGB_OPT_HOST_DLOPEN", false),
		TLSReserve:    envInt("PGB_OPT_TLS_RESERVE", 0),

		SharedWrappers: os.Getenv("PGB_T058_SHARED_WRAPPERS") != "",
	}
	// PGB_ENGINE is the environment's form of --engine, and it exists because
	// the shell harnesses cannot always pass a flag: poc/common.sh builds
	// through one entry point whose only flag slot is claimed by the POC
	// itself. Without it, naming a candidate environment on a machine running
	// dockerd silently built against the default one. An unusable value is
	// ignored here rather than fatal, because Load has no way to report; the
	// flag path validates and the detected engine is a safe fallback.
	//
	// ⛔ NOT in OptVars: exporting it across an engine boundary would make the
	// re-entered pgb try to enter a second container.
	if e := os.Getenv("PGB_ENGINE"); e != "" {
		_ = c.SetEngine(e)
	}
	// ⛔ --utf8-default NEEDS THE EMBEDDED LOCALE TO FALL BACK ON, and the
	// implication is enforced HERE as well as at the flag: a build re-entered
	// across an engine boundary reads PGB_OPT_* out of the environment, and
	// setting only the one variable would link an object referencing
	// pgb_utf8_default with nothing defining it.
	if c.UTF8Default {
		c.EmbedLocale = true
	}
	return c
}

// OptVars are the variables that carry options across an engine boundary. One
// list, so the exporter and the container argument builder cannot drift.
var OptVars = []string{
	"PGB_OPT_VERBOSE", "PGB_OPT_EMBED_LOCALE", "PGB_OPT_EMBED_CACERT",
	"PGB_OPT_EMBED_TERMINFO", "PGB_OPT_EMBED_TZDATA", "PGB_OPT_EMBED_NETDB",
	"PGB_OPT_UTF8_DEFAULT",
	"PGB_OPT_USE_ICONV", "PGB_OPT_BASELINE",
	"PGB_OPT_BINDS", "PGB_OPT_WRAP_DLOPEN", "PGB_OPT_HOST_DLOPEN",
	"PGB_OPT_TLS_RESERVE",
	"PGB_STATE", "PGB_LIBICONV_PREFIX", "PGB_T058_SHARED_WRAPPERS",
	"PGB_LOG", "PGB_DEBUG", "PGB_TS", "PGB_TS_COLUMNS", "PGB_TS_HEARTBEAT",
}

// Export sets the option variables in this process's environment so a
// re-entered pgb inherits the same decisions.
func (c *Config) Export() {
	set := func(k, v string) { _ = os.Setenv(k, v) }
	set("PGB_OPT_VERBOSE", bit(c.Verbose))
	set("PGB_OPT_EMBED_LOCALE", bit(c.EmbedLocale))
	set("PGB_OPT_EMBED_CACERT", bit(c.EmbedCacert))
	set("PGB_OPT_EMBED_TERMINFO", bit(c.EmbedTerminfo))
	set("PGB_OPT_EMBED_TZDATA", bit(c.EmbedTzdata))
	set("PGB_OPT_EMBED_NETDB", bit(c.EmbedNetdb))
	set("PGB_OPT_UTF8_DEFAULT", bit(c.UTF8Default))
	set("PGB_OPT_USE_ICONV", bit(c.UseIconv))
	set("PGB_OPT_BASELINE", c.ArchBaseline)
	set("PGB_OPT_BINDS", strings.Join(c.ExtraBinds, " "))
	set("PGB_OPT_WRAP_DLOPEN", strings.Join(c.WrapDlopen, " "))
	set("PGB_OPT_HOST_DLOPEN", bit(c.HostDlopen))
	set("PGB_OPT_TLS_RESERVE", strconv.Itoa(c.TLSReserve))
	set("PGB_STATE", c.State)
	set("PGB_LIBICONV_PREFIX", c.LibiconvPrefix)
	if spec := logx.SubsysSpec(); spec != "" {
		set("PGB_DEBUG", spec)
	}
	set("PGB_LOG", logx.CurrentLevel().String())
}

// ContainerEnvArgs renders OptVars as `-e NAME` arguments. The value is taken
// from this process's environment by the container runtime, so a value
// containing spaces cannot be torn apart on the way.
func (c *Config) ContainerEnvArgs() []string {
	out := make([]string, 0, len(OptVars)*2)
	for _, v := range OptVars {
		if _, ok := os.LookupEnv(v); !ok {
			continue
		}
		out = append(out, "-e", v)
	}
	return out
}

// SetEngine records an explicit --engine.
func (c *Config) SetEngine(e string) error {
	switch Engine(e) {
	case EngineChroot, EngineDocker, EnginePodman, EngineHost:
		c.engine = Engine(e)
		return nil
	case "":
		c.engine = ""
		return nil
	}
	return fail.Cannot("unknown engine: %s (chroot docker podman host)", e)
}

// Engine returns the explicit choice, or detects one. Detection prefers podman,
// then docker, then chroot; starting a container daemon therefore changes what
// a later command picks, which is why env_stamp checks what the chosen engine
// actually holds.
func (c *Config) Engine() Engine {
	if c.engine != "" {
		return c.engine
	}
	if proc.Look("podman") {
		if r, err := proc.Quiet("podman", "info"); err == nil && !r.Failed() {
			return EnginePodman
		}
	}
	if proc.Look("docker") {
		if r, err := proc.Quiet("docker", "info"); err == nil && !r.Failed() {
			return EngineDocker
		}
	}
	if os.Geteuid() == 0 && proc.Look("unshare") {
		return EngineChroot
	}
	return EngineHost
}

// EngineExplicit reports whether --engine was given.
func (c *Config) EngineExplicit() bool { return c.engine != "" }

// EnvRoot is where the chroot build environment lives.
func (c *Config) EnvRoot() string { return filepath.Join(c.RootfsDir, c.EnvName) }

// ImagesFile is the digest-pinned list of target environments.
func (c *Config) ImagesFile() string {
	return filepath.Join(c.Self, "scripts", "common", "rootfs-images.txt")
}

// RuntimeSrcDir is where the C runtime sources are materialised.
func (c *Config) RuntimeSrcDir() string { return filepath.Join(c.State, "runtime-src") }

// DefaultBaseline is the CPU baseline for this architecture.
func DefaultBaseline() string {
	switch runtime.GOARCH {
	case "amd64":
		return "x86-64"
	case "arm64":
		return "armv8-a"
	}
	return ""
}

// Baseline is the configured baseline or the architecture default.
func (c *Config) Baseline() string {
	if c.ArchBaseline != "" {
		return c.ArchBaseline
	}
	return DefaultBaseline()
}

// AbsPath makes a path absolute without resolving symlinks, so a bind source
// keeps the name the caller used.
func AbsPath(p string) string {
	if filepath.IsAbs(p) {
		return filepath.Clean(p)
	}
	if fi, err := os.Stat(p); err == nil && fi.IsDir() {
		if abs, err := filepath.Abs(p); err == nil {
			return abs
		}
	}
	wd, err := os.Getwd()
	if err != nil {
		return p
	}
	return filepath.Join(wd, strings.TrimPrefix(p, "./"))
}

// AbsBindspec normalises SRC[:DEST] with both sides absolute. A relative bind
// source is a named volume to docker, not a directory, so this runs for every
// engine rather than only for chroot.
func AbsBindspec(spec string) string {
	src, dst, ok := strings.Cut(spec, ":")
	if !ok {
		dst = src
	}
	return AbsPath(src) + ":" + AbsPath(dst)
}

// caVars are the variables a caller uses to name a TLS trust anchor.
var caVars = []string{
	"CURL_CA_BUNDLE", "SSL_CERT_FILE", "GIT_SSL_CAINFO",
	"REQUESTS_CA_BUNDLE", "NODE_EXTRA_CA_CERTS",
}

// CAAnchor returns the trust anchor the caller's own environment names, or "".
// Only that one file is carried into a build environment; verification is
// never disabled and nothing else is trusted.
func CAAnchor() string {
	for _, v := range caVars {
		p := os.Getenv(v)
		if p == "" {
			continue
		}
		if fi, err := os.Stat(p); err == nil && !fi.IsDir() {
			return p
		}
	}
	return ""
}

// ImageRow is one line of rootfs-images.txt.
type ImageRow struct {
	Ref    string // e.g. alpine:3.22
	Name   string // e.g. alpine-3.22
	Libc   string // glibc | musl
	Digest string // sha256:...
}

// Repo is the reference with any tag removed, for composing repo@digest.
func (r ImageRow) Repo() string {
	if i := strings.LastIndexByte(r.Ref, ':'); i > strings.LastIndexByte(r.Ref, '/') {
		return r.Ref[:i]
	}
	return r.Ref
}

// ReadImages parses the pinned environment list. Blank lines and comments are
// skipped; a row with fewer than four fields is an error rather than a silently
// shorter matrix.
func ReadImages(path string) ([]ImageRow, error) {
	b, err := os.ReadFile(path)
	if err != nil {
		return nil, fail.Cannot("missing %s", path)
	}
	var out []ImageRow
	for n, line := range strings.Split(string(b), "\n") {
		s := strings.TrimSpace(line)
		if s == "" || strings.HasPrefix(s, "#") {
			continue
		}
		f := strings.Fields(s)
		if len(f) < 4 {
			return nil, fail.Cannot("%s:%d: want 'ref name libc digest', got %q", path, n+1, s)
		}
		out = append(out, ImageRow{Ref: f[0], Name: f[1], Libc: f[2], Digest: f[3]})
	}
	if len(out) == 0 {
		return nil, fail.Cannot("%s names no environments", path)
	}
	return out, nil
}

func envOr(name, def string) string {
	if v := os.Getenv(name); v != "" {
		return v
	}
	return def
}

// envInt reads a non-negative integer setting. ⚠ An unparsable or negative
// value falls back to the default rather than failing: Load has no way to
// report, and the flag path is where a bad value is rejected out loud.
func envInt(name string, def int) int {
	v, ok := os.LookupEnv(name)
	if !ok || v == "" {
		return def
	}
	n, err := strconv.Atoi(v)
	if err != nil || n < 0 {
		return def
	}
	return n
}

func fieldsOr(name string, def []string) []string {
	v, ok := os.LookupEnv(name)
	if !ok {
		return def
	}
	return strings.Fields(v)
}

func xdgState() string {
	if v := os.Getenv("XDG_STATE_HOME"); v != "" {
		return v
	}
	home, err := os.UserHomeDir()
	if err != nil {
		home = "/root"
	}
	return filepath.Join(home, ".local", "state")
}

func bit(b bool) string {
	if b {
		return "1"
	}
	return "0"
}

// Describe renders the settings for the debug log. It reports the engine only
// when one was named: detecting it runs `docker info`, and this is called on
// every invocation including each compiler wrapper's.
func (c *Config) Describe() string {
	engine := "(detect)"
	if c.engine != "" {
		engine = string(c.engine)
	}
	return fmt.Sprintf("self=%s state=%s rootfs=%s engine=%s iconv=%v baseline=%s",
		c.Self, c.State, c.RootfsDir, engine, c.UseIconv, c.Baseline())
}
