// sweep_selftest.go — the reachability sweep, against a fixture with a known
// answer.
//
// The fixture is built with a real compiler so the DT_NEEDED entries are real
// ones, and it carries both directions: a library nothing needs, which must be
// reported unreachable, and a plugin plus its own dependency, which nothing
// links against and which must NOT be. An absence on its own would pass with a
// sweep that reported everything reachable.
//
// SPDX-License-Identifier: MIT
package bundle

import (
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/proc"
	"github.com/polaris0xff/glibc-research/internal/selftest"
)

// Selftest builds a bundle whose answer is known and sweeps it.
func Selftest() *selftest.Report {
	r := selftest.New("bundle-sweep")
	if !proc.Look("cc") {
		r.Skip("no C compiler, so no fixture with real DT_NEEDED can be built")
		return r
	}
	dir, err := os.MkdirTemp("", "pgb-sweep-selftest-")
	if err != nil {
		r.Fail("tempdir", err.Error(), "created")
		return r
	}
	defer os.RemoveAll(dir)

	src := filepath.Join(dir, "src")
	app := filepath.Join(dir, "AppDir")
	lib := filepath.Join(app, "lib")
	plugins := filepath.Join(lib, "plugins")
	quiet := filepath.Join(app, "quiet")
	for _, d := range []string{src, filepath.Join(app, "shared", "bin"), lib, plugins, quiet} {
		if err := os.MkdirAll(d, 0o755); err != nil {
			r.Fail("mkdir "+d, err.Error(), "created")
			return r
		}
	}

	write := func(name, body string) bool {
		if err := os.WriteFile(filepath.Join(src, name), []byte(body), 0o644); err != nil {
			r.Fail("write "+name, err.Error(), "created")
			return false
		}
		return true
	}
	ok := write("foo.c", "int foo(void){return 1;}\n") &&
		write("unused.c", "int unused_symbol(void){return 2;}\n") &&
		write("plugdep.c", "int plugdep(void){return 3;}\n") &&
		write("plug.c", "extern int plugdep(void); int plug(void){return plugdep();}\n") &&
		write("quietdep.c", "int quietdep(void){return 4;}\n") &&
		write("quiet.c", "extern int quietdep(void); int quiet(void){return quietdep();}\n") &&
		write("prog.c", "extern int foo(void); int main(void){return foo()-1;}\n")
	if !ok {
		return r
	}

	cc := func(args ...string) bool {
		res, err := proc.Quiet(append([]string{"cc"}, args...)...)
		return err == nil && !res.Failed()
	}
	built := cc("-shared", "-fPIC", "-o", filepath.Join(lib, "libfoo.so.1"), filepath.Join(src, "foo.c")) &&
		cc("-shared", "-fPIC", "-o", filepath.Join(lib, "libunused.so.1"), filepath.Join(src, "unused.c")) &&
		cc("-shared", "-fPIC", "-o", filepath.Join(lib, "libplugdep.so.1"), filepath.Join(src, "plugdep.c")) &&
		cc("-shared", "-fPIC", "-o", filepath.Join(lib, "libquietdep.so.1"), filepath.Join(src, "quietdep.c"))
	if !built {
		r.Skip("the fixture libraries did not build")
		return r
	}
	// The plugin links against libplugdep, so a correct sweep reaches
	// libplugdep only through the plugin directory.
	if !cc("-shared", "-fPIC", "-o", filepath.Join(plugins, "libplug.so"),
		filepath.Join(src, "plug.c"), "-L"+lib, "-l:libplugdep.so.1") {
		r.Skip("the fixture plugin did not build")
		return r
	}
	// A library reachable only through a directory an environment file names.
	if !cc("-shared", "-fPIC", "-o", filepath.Join(quiet, "libquiet.so"),
		filepath.Join(src, "quiet.c"), "-L"+lib, "-l:libquietdep.so.1") {
		r.Skip("the fixture env-named library did not build")
		return r
	}
	if !cc("-o", filepath.Join(app, "shared", "bin", "prog"),
		filepath.Join(src, "prog.c"), "-L"+lib, "-l:libfoo.so.1") {
		r.Skip("the fixture program did not build")
		return r
	}

	res, err := Sweep(SweepOptions{Dir: app})
	if err != nil {
		r.Fail("sweep", err.Error(), "no error")
		return r
	}
	unreach := map[string]bool{}
	for _, u := range res.Unreachable {
		unreach[filepath.Base(u)] = true
	}

	r.CheckBool("the program is a root", len(res.Roots) > 0, true)
	r.CheckBool("a library the program needs is reachable", unreach["libfoo.so.1"], false)
	r.CheckBool("a library nothing needs is UNREACHABLE", unreach["libunused.so.1"], true)
	r.CheckBool("a plugin nothing links against is reachable", unreach["libplug.so"], false)
	r.CheckBool("a plugin's own dependency is reachable", unreach["libplugdep.so.1"], false)
	r.CheckBool("the plugin directory was found", containsSuffix(res.PluginDirs, "plugins"), true)

	// The env-file rule, in both directions, which is what makes it a rule
	// rather than a coincidence of this fixture. Without the file, nothing
	// names the `quiet` directory, so the library only it needs is
	// unreachable; with it, both become reachable.
	r.CheckBool("without an env file, the env-only directory is not a plugin dir",
		containsSuffix(res.PluginDirs, "quiet"), false)
	r.CheckBool("without an env file, the library only it needs is UNREACHABLE",
		unreach["libquietdep.so.1"], true)
	r.Check("unreachable count without the env file", strconv.Itoa(res.UnreachFiles), "2")

	envFile := filepath.Join(app, ".env")
	if err := os.WriteFile(envFile, []byte("MY_PLUGIN_PATH=${SHARUN_DIR}/quiet\n"), 0o644); err != nil {
		r.Fail("write .env", err.Error(), "created")
		return r
	}
	resEnv, err := Sweep(SweepOptions{Dir: app, EnvFiles: []string{envFile}})
	if err != nil {
		r.Fail("sweep with the env file", err.Error(), "no error")
		return r
	}
	r.CheckBool("a directory an environment file names is a plugin dir",
		containsSuffix(resEnv.PluginDirs, "quiet"), true)
	unreachEnv := map[string]bool{}
	for _, u := range resEnv.Unreachable {
		unreachEnv[filepath.Base(u)] = true
	}
	r.CheckBool("and the library only it needs becomes reachable",
		unreachEnv["libquietdep.so.1"], false)
	r.Check("unreachable count with the env file", strconv.Itoa(resEnv.UnreachFiles), "1")
	return r
}

func containsSuffix(list []string, want string) bool {
	sorted := append([]string(nil), list...)
	sort.Strings(sorted)
	for _, s := range sorted {
		if strings.HasSuffix(s, want) {
			return true
		}
	}
	return false
}
