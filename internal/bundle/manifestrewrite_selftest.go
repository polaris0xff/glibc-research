// manifestrewrite_selftest.go — the two halves of the manifest rule must agree.
//
// ⛔ THE DEFECT THIS EXISTS FOR, AND IT WAS A HALF-FIX. The sweep learned that
// share/vulkan/explicit_layer.d and etc/OpenCL/vendors name libraries, so the
// libraries they name stopped being deleted. The REWRITE never learned it, so
// those same files kept the `/nix/store` path nixpkgs wrote into them. The
// bundle then carries the library and a manifest pointing at a directory that
// does not exist on the target — and nothing notices, because a path inside a
// JSON file is not a DT_NEEDED and the integrity check has no edge to walk.
//
// ⭐ SO THE FIXTURE IS GENERATED FROM manifestGlobs ITSELF rather than from a
// list written out here. A glob added to the sweep's list gets a fixture, an
// assertion and a failure automatically; a second hand-maintained list would
// be the same defect one level up.
//
// SPDX-License-Identifier: MIT
package bundle

import (
	"os"
	"path/filepath"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/selftest"
)

// ManifestRewriteSelftest writes one fixture per manifest glob, each naming a
// library by absolute store path and a data file by absolute store path, and
// asserts the rewrite fixes the first, leaves the second, and leaves nothing
// the sweep can no longer read.
func ManifestRewriteSelftest() *selftest.Report {
	r := selftest.New("bundle-manifest-rewrite")

	dir, err := os.MkdirTemp("", "pgb-manifest-rewrite-")
	if err != nil {
		r.Fail("tempdir", err.Error(), "created")
		return r
	}
	defer os.RemoveAll(dir)
	app := filepath.Join(dir, "AppDir")

	const store = "/nix/store/00000000000000000000000000000000-fixture"
	// ⛔ The negative arm, and it is in EVERY fixture rather than off to one
	// side: a store path that does not name a shared object must survive. The
	// rule is "point the manifest at the soname beside it", not "strip every
	// absolute path out of somebody else's data file".
	dataPath := store + "/share/table.dat"

	type fixture struct{ path, soname string }
	var fixtures []fixture
	for i, g := range manifestGlobs {
		soname := "libfixture" + string(rune('a'+i)) + ".so.1"
		libPath := store + "/lib/" + soname
		ext := filepath.Ext(g)
		var body string
		switch ext {
		case ".json":
			// The JSON manifests carry the library under "library_path".
			body = `{"file_format_version":"1.0.0",` +
				`"ICD":{"library_path":"` + libPath + `"},` +
				`"data_path":"` + dataPath + `"}` + "\n"
		default:
			// An OpenCL .icd is not JSON at all: one library per line.
			body = libPath + "\n" + dataPath + "\n"
		}
		f := filepath.Join(app, filepath.Dir(g), "fixture"+ext)
		if err := os.MkdirAll(filepath.Dir(f), 0o755); err != nil {
			r.Fail("mkdir "+filepath.Dir(f), err.Error(), "created")
			return r
		}
		if err := os.WriteFile(f, []byte(body), 0o644); err != nil {
			r.Fail("write "+f, err.Error(), "created")
			return r
		}
		fixtures = append(fixtures, fixture{f, soname})
	}

	r.Check("the rewrite visited one file per manifest glob",
		itoa(rewriteManifestPaths(app)), itoa(len(manifestGlobs)))

	// ⭐ The assertion that cannot drift: no library store path survives in
	// anything the SWEEP treats as a manifest.
	for _, fx := range fixtures {
		rel, _ := filepath.Rel(app, fx.path)
		data, err := os.ReadFile(fx.path)
		if err != nil {
			r.Fail("read "+rel, err.Error(), "read back")
			continue
		}
		r.CheckBool(rel+": the library is named bare",
			strings.Contains(string(data), `"`+fx.soname+`"`) ||
				strings.Contains(string(data), "\n"+fx.soname+"\n") ||
				strings.HasPrefix(string(data), fx.soname+"\n"), true)
		r.CheckBool(rel+": no store path to a library survives",
			strings.Contains(string(data), store+"/lib/"), false)
		r.CheckBool(rel+": a store path naming a DATA file is left alone",
			strings.Contains(string(data), dataPath), true)
	}

	// And the sweep still reads the same library names out of the rewritten
	// files, which is what makes them roots. ⚠ This is the half that was
	// already working; it is asserted so a change to the rewrite cannot fix
	// the paths and break the roots in the same motion.
	got := map[string]bool{}
	for _, s := range librariesNamedInManifests(app) {
		got[s] = true
	}
	var missing []string
	for _, fx := range fixtures {
		if !got[fx.soname] {
			missing = append(missing, fx.soname)
		}
	}
	r.Check("the sweep still finds every rewritten library",
		strings.Join(missing, " "), "")
	return r
}
