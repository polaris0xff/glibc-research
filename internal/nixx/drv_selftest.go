// drv_selftest.go — the ATerm reader's cases.
//
// The sample is a real derivation cut down but not hand-simplified: it is the
// shape cache.nixos.org served for bash-5.3p15.
//
// SPDX-License-Identifier: MIT
package nixx

import (
	"strconv"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/selftest"
)

const drvSample = `Derive([("out","/nix/store/aaa-x","",""),("dev","/nix/store/bbb-x-dev","","")],` +
	`[("/nix/store/ccc-dep.drv",["out"]),("/nix/store/ddd-src.drv",["out"])],` +
	`["/nix/store/eee-builder.sh"],"x86_64-linux","/nix/store/fff-bash/bin/bash",` +
	`["-e","/nix/store/eee-builder.sh"],` +
	`[("configureFlags","--a --b"),("out","/nix/store/aaa-x"),` +
	`("postInstall","ln -s a b\nrm c\n"),("quoted","say \"hi\"")])`

// DrvSelftest exercises the derivation reader.
func DrvSelftest() *selftest.Report {
	r := selftest.New("nix-drv")

	d, err := ParseDrv(drvSample)
	if err != nil {
		r.Fail("parse the sample derivation", err.Error(), "no error")
		return r
	}
	r.Check("outputs parsed", strconv.Itoa(len(d.Outputs)), "2")
	r.Check("out path", d.Outputs[0].Path, "/nix/store/aaa-x")
	r.Check("inputDrvs parsed", strconv.Itoa(len(d.InputDrvs)), "2")
	r.Check("inputSrcs parsed", strings.Join(d.InputSrcs, " "), "/nix/store/eee-builder.sh")
	r.Check("system", d.System, "x86_64-linux")
	r.Check("args", strings.Join(d.Args, " "), "-e /nix/store/eee-builder.sh")
	r.Check("env: a plain value", d.Env["configureFlags"], "--a --b")
	// A \n read literally turns a shell fragment into one unrunnable line; a
	// mishandled \" ends the string early and every field after it shifts.
	r.Check(`env: \n is a newline`, quoteVisible(d.Env["postInstall"]), `ln -s a b\nrm c\n`)
	r.Check(`env: \" is a quote`, d.Env["quoted"], `say "hi"`)

	d2, err := ParseDrv(`Derive([("out","/nix/store/a","","")],[],[],"s","b",[],[])`)
	if err != nil {
		r.Fail("empty lists parse", err.Error(), "no error")
	} else {
		r.CheckBool("empty lists parse", len(d2.InputDrvs) == 0 && len(d2.Args) == 0, true)
	}

	// A fetch that returned an error page must be refused, not half-parsed.
	_, err = ParseDrv("<html>404</html>")
	r.CheckBool("a non-derivation is refused", err != nil, true)

	_, err = ParseDrv(`Derive([("out","/nix/store/a","","")],[],[],"s","b",[],[("k","v"`)
	r.CheckBool("a truncated derivation is refused", err != nil, true)

	// structuredAttrs must be decoded, or the plan below sees a derivation
	// with no src, no patches and no configure flags.
	d3, err := ParseDrv(`Derive([("out","/nix/store/a","","")],[],[],"s","b",[],` +
		`[("__json","{\"pname\":\"jq\",\"version\":\"1.8.2\"}")])`)
	if err != nil {
		r.Fail("structuredAttrs parse", err.Error(), "no error")
	} else {
		d3.Source = "/nix/store/hash-jq.drv"
		_, entry := d3.Show()
		r.CheckBool("structuredAttrs decoded", len(entry.StructuredAttrs) == 2, true)
		r.Check("show name from the store path", entry.Name, "jq")
	}
	return r
}

// quoteVisible renders control characters so a case reports what it saw.
func quoteVisible(s string) string {
	var b strings.Builder
	for _, c := range s {
		switch c {
		case '\n':
			b.WriteString(`\n`)
		case '\r':
			b.WriteString(`\r`)
		case '\t':
			b.WriteString(`\t`)
		default:
			b.WriteRune(c)
		}
	}
	return b.String()
}
