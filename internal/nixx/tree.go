// tree.go — configure, build and install one source tree through pgb's
// static-glibc toolchain, adapting when the build says what is wrong.
//
// The build system is taken from the plan, not sniffed: nixpkgs already
// decided and says so through the setup hooks in nativeBuildInputs.
//
// SPDX-License-Identifier: MIT
package nixx

import (
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"regexp"
	"runtime"
	"slices"
	"strconv"
	"strings"
	"time"

	"github.com/polaris0xff/glibc-research/internal/fail"
	"github.com/polaris0xff/glibc-research/internal/logx"
	"github.com/polaris0xff/glibc-research/internal/proc"
)

// storePlaceholder matches nixpkgs' `builtins.placeholder` output: an absolute
// path of 52 characters from the nix alphabet that does not exist anywhere.
var storePlaceholder = regexp.MustCompile(`/[0-9a-df-np-sv-z]{52}`)

// makeVarPlaceholder matches the $(out) and ${dev} spellings nixpkgs' setup
// hooks expand and a plain configure takes literally.
var makeVarPlaceholder = regexp.MustCompile(`\$[({](out|dev|bin|lib|man|doc|info|devdoc)[)}]`)

// flagsFor picks the flag list the build system named. The plan carries all
// three; using configureFlags for a cmake or meson package silently drops
// every option nixpkgs chose.
func flagsFor(p *Plan) (string, []string) {
	for _, h := range p.BuildSystemHooks {
		switch h {
		case "cmake":
			return "cmakeFlags", p.CmakeFlags
		case "meson":
			return "mesonFlags", p.MesonFlags
		}
	}
	return "configureFlags", p.ConfigureFlags
}

// prepareFlags repoints or drops the flags that cannot survive outside nix.
func (b *Builder) prepareFlags(p *Plan) []string {
	key, raw := flagsFor(p)
	var out []string
	for _, f := range raw {
		// nixpkgs writes its output paths as placeholders, and a placeholder
		// is a valid-looking directory: passed through, a package configures
		// happily and installs itself somewhere that does not exist. Repointed
		// at our prefix rather than dropped, because the flag says where the
		// package should install and that answer is the prefix.
		repointed := makeVarPlaceholder.ReplaceAllString(f, b.Prefix)
		repointed = storePlaceholder.ReplaceAllString(repointed, b.Prefix)
		if repointed != f {
			logx.Say("repointed %s -> %s", f, repointed)
			f = repointed
		}
		name, value, hasValue := strings.Cut(f, "=")
		switch {
		case hasValue && strings.HasPrefix(value, "/nix/store/") &&
			(strings.HasPrefix(name, "--with-") || strings.HasPrefix(name, "--enable-")):
			// A store-path flag is repointed at our prefix before it is
			// dropped: it says the package WANTS this dependency and names
			// where nixpkgs put it. If the walk built it, the flag is still
			// right and only the path is wrong.
			dep := strings.TrimPrefix(strings.TrimPrefix(name, "--with-"), "--enable-")
			if _, err := os.Stat(filepath.Join(b.Prefix, ".built", dep)); err == nil {
				logx.Say("repointed %s -> %s=%s", f, name, b.Prefix)
				out = append(out, name+"="+b.Prefix)
			} else {
				logx.Warnf("dropped store-path flag (no %s in the static prefix): %s", dep, f)
			}
			continue
		case strings.Contains(f, "/nix/store/"):
			logx.Warnf("dropped store-path flag: %s", f)
			continue
		case f == "--with-shared", f == "--enable-shared", f == "--with-versioned-syms":
			// These arrive after pgb's own --disable-shared and therefore win,
			// so filtering them is the difference between a static build and a
			// shared one that then fails to link.
			logx.Warnf("dropped shared-library flag: %s", f)
			continue
		}
		out = append(out, f)
	}
	out = append(out, quirksFor(p.Pname)...)
	out = append(out, b.ConfigureExtra...)
	log.Debugf("flag list %s: %s", key, strings.Join(out, " "))
	return out
}

// prepareMakeFlags drops the store-path entries by the same rule.
func prepareMakeFlags(p *Plan) []string {
	var out []string
	for _, f := range p.MakeFlags {
		if strings.Contains(f, "/nix/store/") {
			continue
		}
		out = append(out, f)
	}
	return out
}

// BuildTree runs the configure/build rounds, adapting at most once per round
// and at most MaxRounds times, so a build that cannot be fixed fails with its
// own error rather than looping.
func (b *Builder) BuildTree(top string, p *Plan, work string, install bool) error {
	logFile := filepath.Join(work, "build.log")
	adaptations := filepath.Join(work, "adaptations.txt")
	_ = os.WriteFile(adaptations, nil, 0o644)

	flags := b.prepareFlags(p)
	makeFlags := prepareMakeFlags(p)
	hooks := p.BuildSystemHooks
	if len(hooks) > 0 {
		key, _ := flagsFor(p)
		logx.Say("build system: %s (%s)", strings.Join(hooks, " "), key)
	}
	var env []string
	seen := map[string]bool{}

	for round := 1; round <= b.MaxRounds; round++ {
		logx.Say("round %d: configure %s", round, strings.Join(flags, " "))
		err := b.tryBuild(top, flags, makeFlags, hooks, logFile, env, install)
		if err == nil {
			logx.Say("round %d: built", round)
			return nil
		}
		fix := diagnose(logFile, flags)
		if fix == "" {
			logx.Say("")
			logx.Warnf("the build failed and pgb has no adaptation for it. Last 30 lines:")
			showTail(logFile, 30)
			return fail.Ran("build failed")
		}
		// The same adaptation twice is not progress, it is a loop: a fix
		// already applied means the diagnosis is wrong about this failure, so
		// the honest move is to stop and print the real error.
		if seen[fix] {
			logx.Say("")
			logx.Warnf("adaptation %q was already applied and the build failed the same way.", fix)
			logx.Warnf("That is a wrong diagnosis, not a missing round. Last 30 lines:")
			showTail(logFile, 30)
			return fail.Ran("build failed")
		}
		seen[fix] = true
		appendLine(adaptations, fmt.Sprintf("round %d: %s", round, fix))
		logx.Say("round %d: FAILED -> %s", round, fix)

		verb, arg, _ := strings.Cut(fix, ":")
		switch verb {
		case "drop":
			var kept []string
			for _, f := range flags {
				if f != arg {
					kept = append(kept, f)
				}
			}
			flags = kept
		case "add":
			flags = append(flags, arg)
		case "env":
			env = append(env, arg)
		}
	}
	logx.Warnf("gave up after %d rounds", b.MaxRounds)
	return fail.Ran("build failed")
}

// tryBuild composes the build script and runs it inside the pgb build
// environment. The fetch already happened outside: the chroot has no network
// by design, and a build that reaches the network is not reproducible anyway.
func (b *Builder) tryBuild(top string, flags, makeFlags, hooks []string, logFile string, extraEnv []string, install bool) error {
	jobs := strconv.Itoa(runtime.NumCPU())
	q := logx.Quote

	// The static prefix goes on every search path, and pkg-config is pointed
	// at both lib/pkgconfig and share/pkgconfig: an architecture-independent
	// package puts its .pc file in the second, and with only the first on the
	// path a dependency built one directory over is invisible.
	envLines := []string{
		fmt.Sprintf(`CPPFLAGS="-I%s ${CPPFLAGS:-}"`, b.Prefix),
		fmt.Sprintf(`LDFLAGS="-L%s/lib -L%s/lib64 ${LDFLAGS:-}"`, b.Prefix, b.Prefix),
		fmt.Sprintf(`PKG_CONFIG_PATH="%s/lib/pkgconfig:%s/share/pkgconfig:%s/lib64/pkgconfig"`,
			b.Prefix, b.Prefix, b.Prefix),
		fmt.Sprintf(`PATH="%s/bin:$PATH"`, b.Prefix),
	}
	envLines = append(envLines, extraEnv...)

	flagStr := strings.Join(quoteAll(flags), " ")
	makeStr := strings.Join(quoteAll(makeFlags), " ")
	instSuffix := ""
	if install {
		instSuffix = " && make install"
	}

	var script strings.Builder
	fmt.Fprintf(&script, "set -u\ncd %s || exit 1\n", q(top))
	for _, e := range envLines {
		fmt.Fprintf(&script, "export %s\n", e)
	}
	// An autoreconf hook GENERATES ./configure, so the decision about which
	// build shape this is must be taken inside the script, after it has run.
	if hasHook(hooks, "autoreconf") {
		script.WriteString("[ -x ./configure ] || { ./autogen.sh || ./bootstrap || autoreconf -fi; } || exit 1\n")
	}

	switch {
	case hasHook(hooks, "cmake"):
		fmt.Fprintf(&script, "cd \"$(%s build-root CMakeLists.txt)\" || exit 1\n", selfInEnv)
		fmt.Fprintf(&script, "cmake -S . -B _pgbbuild -DBUILD_SHARED_LIBS=OFF -DCMAKE_BUILD_TYPE=Release "+
			"-DCMAKE_INSTALL_PREFIX=%s -DCMAKE_INSTALL_LIBDIR=lib -DCMAKE_PREFIX_PATH=%s %s || exit 1\n",
			q(b.Prefix), q(b.Prefix), flagStr)
		fmt.Fprintf(&script, "cmake --build _pgbbuild -j %s || exit 1\n", jobs)
		if install {
			script.WriteString("cmake --install _pgbbuild || exit 1\n")
		}
	case hasHook(hooks, "meson"):
		// --libdir=lib is not cosmetic: meson defaults it to lib64 on x86_64,
		// and the search paths above name lib. A dependency that builds,
		// installs and is invisible is the worst of the three outcomes.
		fmt.Fprintf(&script, "cd \"$(%s build-root meson.build)\" || exit 1\n", selfInEnv)
		fmt.Fprintf(&script, "meson setup _pgbbuild --default-library=static --prefer-static "+
			"--prefix=%s --libdir=lib %s || exit 1\n", q(b.Prefix), flagStr)
		fmt.Fprintf(&script, "ninja -C _pgbbuild || exit 1\n")
		if install {
			script.WriteString("ninja -C _pgbbuild install || exit 1\n")
		}
	default:
		fmt.Fprintf(&script, "cd \"$(%s build-root configure Makefile Makefile.in bootstrap.sh CMakeLists.txt)\" || exit 1\n",
			selfInEnv)
		script.WriteString(b.plainBuildScript(q(b.Prefix), flagStr, makeStr, jobs, instSuffix, install))
	}

	log.Debugf("build script:\n%s", script.String())

	lf, err := os.Create(logFile)
	if err != nil {
		return err
	}
	defer lf.Close()

	argv := []string{b.pgbPath()}
	if b.C.EngineExplicit() {
		// The engine has to travel with the call: an inner `pgb build` that
		// re-detects would use whichever engine a running daemon makes it pick,
		// so the engine the operator named would be used for nothing.
		argv = append(argv, "--engine", string(b.C.Engine()))
	}
	argv = append(argv, "build",
		"--bind", top+":"+top,
		"--bind", b.Prefix+":"+b.Prefix,
		"--", "sh", "-c", script.String())

	r, err := (&proc.Cmd{Argv: argv, Stdout: lf, Stderr: lf, Subsys: "nix"}).Run()
	if err != nil {
		return err
	}
	if r.Failed() {
		return fmt.Errorf("build exited %d", r.Code)
	}
	return nil
}

// selfInEnv is where pgb is reachable inside the build environment.
const selfInEnv = "/pgb"

// plainBuildScript covers the trees with no cmake and no meson. Four shapes
// turn up and each is chosen inside the script, from the tree as it is after
// any autoreconf: Boost.Build, oconfigure's KEY=VALUE form, an ordinary
// ./configure, openssl's ./config, and a bare Makefile.
func (b *Builder) plainBuildScript(prefix, flagStr, makeStr, jobs, instSuffix string, install bool) string {
	var s strings.Builder
	installArg := ""
	if install {
		installArg = "install"
	}
	s.WriteString("if [ -x ./bootstrap.sh ] && { [ -f ./Jamroot ] || [ -f ./bootstrap.jam ] || [ -f ./boost-build.jam ]; }; then\n")
	fmt.Fprintf(&s, "  ./bootstrap.sh --prefix=%s --without-libraries=python || exit 1\n", prefix)
	// b2 returns non-zero when any target failed and a library set always has
	// one, so the success test is what landed rather than what b2 thought.
	fmt.Fprintf(&s, "  ./b2 -j %s link=static runtime-link=static threading=multi variant=release --prefix=%s %s || true\n",
		jobs, prefix, installArg)
	fmt.Fprintf(&s, "  ls %s/lib/libboost_system.a >/dev/null 2>&1 || exit 1\n", prefix)
	s.WriteString("elif [ -x ./configure ] && grep -q oconfigure ./configure 2>/dev/null; then\n")
	fmt.Fprintf(&s, "  ./configure PREFIX=%s %s || exit 1\n", prefix, flagStr)
	fmt.Fprintf(&s, "  make -j %s%s || exit 1\n", jobs, instSuffix)
	s.WriteString("elif [ -x ./configure ]; then\n")
	fmt.Fprintf(&s, "  ./configure --prefix=%s --disable-shared --enable-static %s || exit 1\n", prefix, flagStr)
	fmt.Fprintf(&s, "  make -j %s%s || exit 1\n", jobs, instSuffix)
	s.WriteString("elif [ -x ./config ] && [ -f ./Configure ]; then\n")
	fmt.Fprintf(&s, "  ./config no-shared no-tests --prefix=%s --openssldir=%s/ssl --libdir=lib || exit 1\n", prefix, prefix)
	fmt.Fprintf(&s, "  make -j %s build_sw || exit 1\n", jobs)
	if install {
		s.WriteString("  make install_sw || exit 1\n")
	}
	s.WriteString("else\n")
	fmt.Fprintf(&s, "  make -j %s SHARED=no prefix=%s PREFIX=%s %s || exit 1\n", jobs, prefix, prefix, makeStr)
	if install {
		fmt.Fprintf(&s, "  make install SHARED=no prefix=%s PREFIX=%s %s || exit 1\n", prefix, prefix, makeStr)
	}
	s.WriteString("fi\n")
	return s.String()
}

func (b *Builder) pgbPath() string {
	if p, err := os.Executable(); err == nil {
		return p
	}
	return "pgb"
}

func hasHook(hooks []string, want string) bool {
	return slices.Contains(hooks, want)
}

func quoteAll(in []string) []string {
	out := make([]string, len(in))
	for i, s := range in {
		out[i] = logx.Quote(s)
	}
	return out
}

func appendLine(path, line string) {
	f, err := os.OpenFile(path, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o644)
	if err != nil {
		return
	}
	defer f.Close()
	fmt.Fprintln(f, line)
}

func showTail(path string, n int) {
	b, err := os.ReadFile(path)
	if err != nil {
		return
	}
	lines := strings.Split(strings.TrimRight(string(b), "\n"), "\n")
	if len(lines) > n {
		lines = lines[len(lines)-n:]
	}
	for _, l := range lines {
		fmt.Fprintf(os.Stderr, "  %s\n", l)
	}
}

// ---------------------------------------------------------------------------
// fetch helpers
// ---------------------------------------------------------------------------

func downloadFile(url, dst string) error {
	client := &http.Client{Timeout: 30 * time.Minute}
	resp, err := client.Get(url)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("%s: HTTP %d", url, resp.StatusCode)
	}
	if err := os.MkdirAll(filepath.Dir(dst), 0o755); err != nil {
		return err
	}
	f, err := os.Create(dst)
	if err != nil {
		return err
	}
	if _, err := io.Copy(f, resp.Body); err != nil {
		f.Close()
		return err
	}
	return f.Close()
}

// checkFileHash accepts every encoding a derivation writes a fixed-output hash
// in. An unrecognised shape is a refusal rather than a skipped check.
func checkFileHash(path, want string) (bool, error) {
	if want == "" {
		return false, fmt.Errorf("no expected hash")
	}
	f, err := os.Open(path)
	if err != nil {
		return false, err
	}
	defer f.Close()
	h := sha256.New()
	if _, err := io.Copy(h, f); err != nil {
		return false, err
	}
	sum := h.Sum(nil)
	got := hex.EncodeToString(sum)
	switch {
	case NormaliseHash(want) == got:
		return true, nil
	case want == base64.StdEncoding.EncodeToString(sum):
		return true, nil
	case want == "sha256-"+base64.StdEncoding.EncodeToString(sum):
		return true, nil
	case want == NixBase32(sum), want == "sha256:"+NixBase32(sum):
		return true, nil
	}
	return false, nil
}
