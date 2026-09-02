// stamp_selftest.go — the environment stamp, and the rule that a named
// environment only means something to the engine that reads it.
//
// ⛔ WHY THIS EXISTS. `PGB_ENV_NAME=pgb-env-debian-trixie sh poc/10-gawk/run.sh`
// was the recorded way to build a POC against a candidate glibc. On a machine
// with dockerd running it built against the INCUMBENT instead: engine
// detection prefers docker, the docker engine builds from an image and never
// reads the name, and the stamp comparison passed because the wanted image was
// still the default. The binary's own `.comment` read GCC 12.2.0 where the
// candidate environment carries 14.2.0, and nothing in the POC's output said
// so — it printed a clean 11-of-11 table for the wrong environment.
//
// ⭐ The cases below are built so a regression cannot pass by accident: each
// engine is asserted in BOTH directions, so a predicate stuck at true and one
// stuck at false are each caught by a case the other satisfies.
//
// SPDX-License-Identifier: MIT
package envx

import (
	"os"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/selftest"
)

// StampSelftest asserts the stamp round trip and the named-environment rule.
// Offline: it starts no daemon and reads no rootfs.
func StampSelftest() *selftest.Report {
	r := selftest.New("env-stamp")

	// Which engines read PGB_ENV_NAME. Both directions, so a predicate that
	// answered a constant would fail one of the two.
	r.CheckBool("chroot reads PGB_ENV_NAME", ServesNamedEnv(cfg.EngineChroot), true)
	r.CheckBool("docker does not", ServesNamedEnv(cfg.EngineDocker), false)
	r.CheckBool("podman does not", ServesNamedEnv(cfg.EnginePodman), false)

	// The refusal itself, through RequireCurrent. A non-default name under an
	// image-based engine must be refused BEFORE any stamp is read, because the
	// stamp is what failed to notice: it compares images, and a caller that set
	// only the name still wants the default image.
	named := cfg.Load("/nonexistent")
	named.EnvName = "pgb-env-some-candidate"
	err := RequireCurrent(named, cfg.EngineDocker)
	r.CheckBool("a named environment is refused under docker", err != nil, true)
	got := "none"
	if err != nil {
		got = err.Error()
	}
	// The message has to name the variable, or the reader cannot act on it.
	r.CheckBool("the refusal names the engine that does read it",
		err != nil && strings.Contains(got, "chroot"), true)

	// ⛔ THE CASE THAT CATCHES OVER-REFUSAL. The default name must NOT trip
	// this rule; if it did, every ordinary docker build would refuse and the
	// guard would be useless in a way three passing cases above would hide.
	// RequireCurrent still goes on to inspect the docker environment, which may
	// legitimately fail on a machine with no docker — so this asserts only that
	// whatever comes back is not THIS rule's refusal.
	def := cfg.Load("/nonexistent")
	err = RequireCurrent(def, cfg.EngineDocker)
	tripped := err != nil && strings.Contains(err.Error(), "an engine that reads it")
	r.CheckBool("the DEFAULT name does not trip the rule", tripped, false)

	// The chroot engine may be handed any name; the rule must not fire there
	// either. It will still fail on the missing rootfs, which is a different
	// sentence.
	err = RequireCurrent(named, cfg.EngineChroot)
	tripped = err != nil && strings.Contains(err.Error(), "an engine that reads it")
	r.CheckBool("chroot is never refused by the rule", tripped, false)

	// PGB_ENGINE, the environment's form of --engine. It is what lets a shell
	// harness whose only flag slot is already claimed still name an engine, and
	// it is asserted in three directions so a stub that ignored it, or one that
	// accepted anything, would each fail a case.
	os.Setenv("PGB_ENGINE", "chroot")
	r.Check("PGB_ENGINE=chroot is honoured", string(cfg.Load("/nonexistent").Engine()), "chroot")
	os.Setenv("PGB_ENGINE", "host")
	r.Check("PGB_ENGINE=host is honoured", string(cfg.Load("/nonexistent").Engine()), "host")
	// ⛔ A value that is not an engine must not become one. Load cannot report,
	// so it falls back to detection -- which is never "nonsense".
	os.Setenv("PGB_ENGINE", "nonsense")
	r.CheckBool("a bad PGB_ENGINE is not adopted",
		string(cfg.Load("/nonexistent").Engine()) != "nonsense", true)
	os.Unsetenv("PGB_ENGINE")

	// ⛔ And it must NOT ride across an engine boundary: a re-entered pgb that
	// inherited it would try to enter a second container.
	carried := false
	for _, v := range cfg.OptVars {
		if v == "PGB_ENGINE" {
			carried = true
		}
	}
	r.CheckBool("PGB_ENGINE is not carried in OptVars", carried, false)

	// The stamp round trip, unchanged in intent: a rendered stamp must parse
	// back to the same thing, including a package list whose order differs.
	s := Stamp{Image: "debian:12@sha256:abc", Iconv: true,
		Packages: []string{"gcc", "make"}, Pip: []string{"meson==1.9.1"}}
	back, ok := ParseStamp(s.String())
	r.CheckBool("a rendered stamp parses back", ok, true)
	r.Check("the round trip preserves it", back.String(), s.String())

	// Reordering PGB_ENV_PACKAGES is not a difference. ⚠ Asserted through Want,
	// which is where the sort happens -- String renders the order it is given,
	// so a hand-built Stamp would measure nothing. Getting this wrong is what
	// would make an environment rebuild every time somebody edits the list.
	a := cfg.Load("/nonexistent")
	a.EnvPackages = []string{"make", "gcc"}
	b := cfg.Load("/nonexistent")
	b.EnvPackages = []string{"gcc", "make"}
	r.Check("package order is not a difference", Want(a).String(), Want(b).String())

	// And a real difference still is one, or the case above would pass for a
	// comparison that ignored packages entirely.
	d := cfg.Load("/nonexistent")
	d.EnvPackages = []string{"gcc"}
	r.CheckBool("a missing package IS a difference",
		Want(d).String() != Want(b).String(), true)

	return r
}
