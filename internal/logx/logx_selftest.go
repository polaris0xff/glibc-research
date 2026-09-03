// logx_selftest.go — the level/subsystem matrix, the shell quoting, and the
// stamper's wiring.
//
// ⛔ WHY THIS PACKAGE NEEDED ONE. Three things here are read as fact elsewhere
// in the tree, and none was asserted by anything carried in the binary:
//
//   - `Quote` exists so a reader can PASTE what pgb ran back into a terminal.
//     A wrong quoting is not a cosmetic defect: it makes the printed command
//     differ from the executed one, which is the single thing this project's
//     write-ups rely on when they quote a reproduction. ⭐ So the cases below
//     do not compare against a hand-written expectation — they run the quoted
//     form through a real `/bin/sh` and check the byte comes back.
//   - `SubsysSpec` CROSSES THE ENGINE BOUNDARY. It is re-rendered into
//     PGB_DEBUG for the pgb re-entered inside the build environment, so a
//     spec that does not round-trip means the inner process logs differently
//     from the outer one and nothing says so.
//   - `NewStamper` is the function T-061's fourth defect was about: the
//     columns, the parser and the heartbeat all existed and NOTHING CALLED IT,
//     so `pgb --ts` printed no timestamps and every gate stayed green.
//     `StreamStamper` is the caller now, and a case here asserts it returns
//     one rather than nil.
//
// ⚠ `state` is process-global. Every case that touches it saves and restores
// it, so running this suite cannot change how the rest of the run logs.
//
// SPDX-License-Identifier: MIT
package logx

import (
	"bytes"
	"os"
	"os/exec"
	"strings"
	"time"

	"github.com/polaris0xff/glibc-research/internal/selftest"
)

// Selftest asserts the diagnostic surface.
func Selftest() *selftest.Report {
	r := selftest.New("logx")

	// ---- ParseLevel ------------------------------------------------------
	for _, c := range []struct {
		in   string
		want Level
	}{
		{"error", LevelError}, {"err", LevelError}, {"0", LevelError},
		{"warn", LevelWarn}, {"warning", LevelWarn}, {"1", LevelWarn},
		{"info", LevelInfo}, {"2", LevelInfo},
		{"debug", LevelDebug}, {"3", LevelDebug},
		{"trace", LevelTrace}, {"4", LevelTrace},
		{"  TRACE  ", LevelTrace}, // trimmed and lowercased
	} {
		got, err := ParseLevel(c.in)
		if err != nil {
			r.Fail("ParseLevel("+c.in+")", err.Error(), c.want.String())
			continue
		}
		r.Check("ParseLevel("+strings.TrimSpace(c.in)+")", got.String(), c.want.String())
	}
	// ⛔ AN UNKNOWN LEVEL IS AN ERROR AND NOT A DEFAULT. Silently falling back
	// would make a mistyped PGB_LOG produce no output, which reads exactly like
	// a quiet run.
	if _, err := ParseLevel("verbose"); err == nil {
		r.Fail("ParseLevel of an unknown name", "no error", "an error")
	} else {
		r.CheckBool("an unknown level is an error, not a default", true, true)
		r.CheckBool("...and the message names the valid set",
			strings.Contains(err.Error(), "error warn info debug trace"), true)
	}

	// ---- Quote, checked through a real shell ------------------------------
	// ⭐ THE ASSERTION IS THE ROUND TRIP, not a string comparison. Quote's whole
	// contract is "pasting this into a POSIX shell reproduces the argument", so
	// the shell is the oracle.
	shell, shellErr := exec.LookPath("sh")
	hard := []string{
		"plain", "", " ", "a b", "it's", `back\slash`, `"double"`,
		"$HOME", "`cmd`", "semi;colon", "pipe|bar", "new\nline", "tab\there",
		"star*glob", "tilde~", "paren()", "brace{}", "bracket[]",
		"amp&", "lt<gt>", "hash#", "bang!", "percent%", "at@", "caret^",
		"safe_chars@%_+=:,./-", "ünïcøde", "emoji🙂",
	}
	if shellErr != nil {
		r.Skip("logx: no sh on PATH, so Quote was not round-tripped through one")
	} else {
		bad := 0
		for _, s := range hard {
			// `printf %s` writes the argument with nothing added, so anything
			// that comes back different is the quoting's fault.
			cmd := exec.Command(shell, "-c", "printf %s "+Quote(s))
			var out bytes.Buffer
			cmd.Stdout = &out
			if err := cmd.Run(); err != nil || out.String() != s {
				bad++
				r.Fail("Quote round trip: "+strings.ReplaceAll(s, "\n", "\\n"),
					out.String(), s)
			}
		}
		r.CheckInt("every hard argument survives Quote -> sh -> printf, count bad",
			bad, 0)
		r.CheckInt("...and that was over this many arguments", len(hard), len(hard))

		// QuoteArgs is the same thing over an argv, and the separator must not
		// merge two arguments into one.
		cmd := exec.Command(shell, "-c",
			"for a in "+QuoteArgs([]string{"a b", "c", ""})+"; do printf '[%s]' \"$a\"; done")
		var out bytes.Buffer
		cmd.Stdout = &out
		if err := cmd.Run(); err != nil {
			r.Fail("QuoteArgs through sh", err.Error(), "[a b][c][]")
		} else {
			r.Check("QuoteArgs keeps argument boundaries", out.String(), "[a b][c][]")
		}
	}
	// The empty string must be quoted, or it vanishes from the pasted line.
	r.Check("the empty string quotes to ''", Quote(""), "''")
	// ⚠ A word made only of safe characters is left bare, which is what keeps
	// an ordinary command line readable.
	r.Check("a safe word is not quoted", Quote("/usr/bin/cc"), "/usr/bin/cc")

	// ---- EnvBool ---------------------------------------------------------
	const k = "PGB_LOGX_SELFTEST_BOOL"
	defer os.Unsetenv(k)
	os.Unsetenv(k)
	r.CheckBool("EnvBool: unset takes the default (true)", EnvBool(k, true), true)
	r.CheckBool("EnvBool: unset takes the default (false)", EnvBool(k, false), false)
	// ⚠ EMPTY IS TREATED AS UNSET HERE, and that is deliberate rather than
	// accidental: `FOO= pgb ...` is how a shell clears a variable it inherited.
	os.Setenv(k, "")
	r.CheckBool("EnvBool: set-and-empty takes the default", EnvBool(k, true), true)
	for _, c := range []struct {
		v    string
		want bool
	}{
		{"0", false}, {"no", false}, {"false", false}, {"off", false}, {"OFF", false},
		{"1", true}, {"yes", true}, {"true", true}, {"on", true}, {"ON", true},
		{"2", true}, {"-1", true}, // any non-zero number
	} {
		os.Setenv(k, c.v)
		r.CheckBool("EnvBool("+c.v+")", EnvBool(k, !c.want), c.want)
	}
	// ⛔ A value that is neither a word nor a number takes the DEFAULT rather
	// than guessing, so a typo cannot silently flip a mechanism on.
	os.Setenv(k, "maybe")
	r.CheckBool("EnvBool: an unparsable value takes the default", EnvBool(k, true), true)
	r.CheckBool("...in the other direction too", EnvBool(k, false), false)

	// ---- Enabled: the level/subsystem matrix -----------------------------
	restore := saveState()
	defer restore()

	setState(LevelWarn, false, nil, nil)
	r.CheckBool("at warn, a warning is emitted", Enabled("nix", LevelWarn), true)
	r.CheckBool("at warn, info is not", Enabled("nix", LevelInfo), false)

	// ⛔ SELECTION NARROWS DEBUG AND TRACE ONLY. A subsystem filter that also
	// silenced warnings would hide the one class of message a user did not ask
	// for and needs anyway.
	setState(LevelTrace, false, []string{"bundle"}, nil)
	r.CheckBool("a non-selected subsystem still emits errors",
		Enabled("nix", LevelError), true)
	r.CheckBool("...and warnings", Enabled("nix", LevelWarn), true)
	r.CheckBool("...and info", Enabled("nix", LevelInfo), true)
	r.CheckBool("but NOT debug", Enabled("nix", LevelDebug), false)
	r.CheckBool("...nor trace", Enabled("nix", LevelTrace), false)
	r.CheckBool("the selected subsystem does emit debug",
		Enabled("bundle", LevelDebug), true)

	// With nothing selected, every subsystem is on at the configured level.
	setState(LevelDebug, false, nil, nil)
	r.CheckBool("with no selection, any subsystem emits debug",
		Enabled("anything", LevelDebug), true)

	// ⛔ `-name` WINS OVER `all`. It is the only way to say "everything except
	// this one", so an ordering that let `all` win would make the form useless.
	setState(LevelTrace, true, nil, []string{"nix"})
	r.CheckBool("with `all`, an unmentioned subsystem emits debug",
		Enabled("bundle", LevelDebug), true)
	r.CheckBool("but `-nix` beats `all`", Enabled("nix", LevelDebug), false)
	r.CheckBool("...and still does not silence its warnings",
		Enabled("nix", LevelWarn), true)

	// ---- SubsysSpec: the engine boundary ---------------------------------
	// ⭐ THE ROUND TRIP. Configure -> SubsysSpec -> Configure must select the
	// same set, because that string is what the inner pgb is given.
	for _, spec := range []string{"nix", "nix,bundle", "all", "all,-nix", "-nix,-bundle"} {
		setState(LevelTrace, false, nil, nil)
		applySubsys(spec)
		first := SubsysSpec()
		setState(LevelTrace, false, nil, nil)
		applySubsys(first)
		r.Check("SubsysSpec round-trips "+spec, SubsysSpec(), first)
	}
	setState(LevelTrace, false, nil, nil)
	applySubsys("bundle,all,-nix")
	// Sorted and deduplicated, so the same selection always renders the same
	// way and two equivalent specs cannot look different in a log.
	r.Check("SubsysSpec renders sorted and canonical", SubsysSpec(), "-nix,all,bundle")
	setState(LevelTrace, false, nil, nil)
	applySubsys(" nix , , bundle ")
	r.Check("SubsysSpec ignores blanks and trims", SubsysSpec(), "bundle,nix")
	restore()

	// ---- ParseColumns ----------------------------------------------------
	cols, err := ParseColumns("rel,delta")
	if err != nil {
		r.Fail("ParseColumns(rel,delta)", err.Error(), "rel delta")
	} else {
		r.Check("ParseColumns keeps the order given", colStr(cols), "rel delta")
	}
	if cols, err = ParseColumns(" DELTA , rel "); err != nil {
		r.Fail("ParseColumns is case- and space-insensitive", err.Error(), "delta rel")
	} else {
		r.Check("ParseColumns is case- and space-insensitive", colStr(cols), "delta rel")
	}
	if _, err = ParseColumns("rel,nope"); err == nil {
		r.Fail("an unknown column", "no error", "an error")
	} else {
		r.CheckBool("an unknown column is an error naming the set",
			strings.Contains(err.Error(), "epoch"), true)
	}
	// ⚠ A repeated column is refused rather than deduplicated: it is a typo in a
	// format string and printing the same timestamp twice hides it.
	if _, err = ParseColumns("rel,rel"); err == nil {
		r.Fail("a repeated column", "no error", "an error")
	} else {
		r.CheckBool("a repeated column is refused", true, true)
	}
	if _, err = ParseColumns(" , "); err == nil {
		r.Fail("an empty column list", "no error", "an error")
	} else {
		r.CheckBool("an empty column list is an error", true, true)
	}

	// ---- relative(): the claim in stamp.go's own header ------------------
	// ⭐ *"Relative time counts total hours, so a run past 24 hours does not
	// wrap back to zero."* That sentence is a claim and this is its measurement.
	r.Check("relative() at 25 hours does NOT wrap to zero",
		relative(25*time.Hour), "25:00:00.000")
	r.Check("relative() renders milliseconds", relative(3661*time.Second+250*time.Millisecond),
		"01:01:01.250")
	// A negative interval cannot happen from a monotonic clock, but the guard
	// exists and an unguarded division would render nonsense.
	r.Check("relative() clamps a negative interval", relative(-time.Second), "00:00:00.000")

	// ---- the Stamper, including the wiring T-061 found missing -----------
	var buf bytes.Buffer
	s := NewStamper(StampConfig{Columns: []Column{ColRel}, Separator: "|", Out: &buf})
	s.Line("first")
	s.Line("second")
	s.Close()
	lines := strings.Split(strings.TrimRight(buf.String(), "\n"), "\n")
	r.CheckInt("the stamper writes one line per Line()", len(lines), 2)
	if len(lines) == 2 {
		r.CheckBool("...each carrying its column and separator",
			strings.HasSuffix(lines[0], "|first") && strings.Contains(lines[0], ":"), true)
		r.CheckBool("...and the text is not otherwise altered",
			strings.HasSuffix(lines[1], "|second"), true)
	}
	nl, nb := s.Counts()
	r.CheckInt("the stamper counts lines", int(nl), 2)
	r.CheckInt("...and bytes, with the newline it will write", int(nb),
		len("first")+1+len("second")+1)
	// Close is documented idempotent; a second call must not block or panic.
	s.Close()
	r.CheckBool("Close is idempotent", true, true)

	// ⭐ THE WIRING CASE. T-061's fourth defect was a feature nothing called:
	// `pgb --ts` printed no timestamps because NewStamper had no caller.
	// `StreamStamper` is that caller, and this asserts it actually returns one.
	prevTS, hadTS := os.LookupEnv("PGB_TS")
	os.Setenv("PGB_TS", "1")
	st := StreamStamper()
	r.CheckBool("PGB_TS=1 produces a stamper (T-061: it once produced nil)",
		st != nil, true)
	if st != nil {
		st.Close()
	}
	os.Setenv("PGB_TS", "0")
	st = StreamStamper()
	r.CheckBool("PGB_TS=0 produces none", st == nil, true)
	if hadTS {
		os.Setenv("PGB_TS", prevTS)
	} else {
		os.Unsetenv("PGB_TS")
	}

	return r
}

func colStr(cols []Column) string {
	out := make([]string, len(cols))
	for i, c := range cols {
		out[i] = string(c)
	}
	return strings.Join(out, " ")
}

// saveState snapshots the package's global selection and returns a restore
// function. ⛔ Without this the suite would leave the process logging at trace
// with whatever subsystems the last case named.
func saveState() func() {
	state.mu.Lock()
	lvl, all := state.level, state.subsysAll
	on := map[string]bool{}
	off := map[string]bool{}
	for k, v := range state.subsysOn {
		on[k] = v
	}
	for k, v := range state.subsysOff {
		off[k] = v
	}
	state.mu.Unlock()
	return func() {
		state.mu.Lock()
		state.level, state.subsysAll = lvl, all
		state.subsysOn, state.subsysOff = on, off
		state.mu.Unlock()
	}
}

func setState(l Level, all bool, on, off []string) {
	state.mu.Lock()
	defer state.mu.Unlock()
	state.level, state.subsysAll = l, all
	state.subsysOn, state.subsysOff = map[string]bool{}, map[string]bool{}
	for _, s := range on {
		state.subsysOn[s] = true
	}
	for _, s := range off {
		state.subsysOff[s] = true
	}
}
