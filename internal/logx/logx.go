// Package logx is pgb's diagnostic surface: levels, per-subsystem selection,
// and shell-quoted rendering of a command line so the reader can paste what
// pgb ran back into a terminal.
//
// Output discipline: Say* writes the tool's answer to stdout; everything else
// is narration and goes to stderr.
//
// SPDX-License-Identifier: MIT
package logx

import (
	"fmt"
	"os"
	"sort"
	"strconv"
	"strings"
	"sync"
)

// Level orders the five severities. Anything at or below the configured level
// is emitted.
type Level int

const (
	LevelError Level = iota
	LevelWarn
	LevelInfo
	LevelDebug
	LevelTrace
)

var levelNames = map[Level]string{
	LevelError: "error", LevelWarn: "warn", LevelInfo: "info",
	LevelDebug: "debug", LevelTrace: "trace",
}

func (l Level) String() string { return levelNames[l] }

// ParseLevel accepts a name or a number. An unrecognised value is an error, so
// a mistyped PGB_LOG cannot silently produce no output.
func ParseLevel(s string) (Level, error) {
	switch strings.ToLower(strings.TrimSpace(s)) {
	case "error", "err", "0":
		return LevelError, nil
	case "warn", "warning", "1":
		return LevelWarn, nil
	case "info", "2":
		return LevelInfo, nil
	case "debug", "3":
		return LevelDebug, nil
	case "trace", "4":
		return LevelTrace, nil
	}
	return LevelError, fmt.Errorf("unknown log level %q (error warn info debug trace)", s)
}

var state struct {
	mu sync.Mutex

	level Level
	// Subsystem selection narrows debug and trace only. subsysAll is `all`;
	// an entry in subsysOff came from a `-name` term and wins over both.
	subsysAll bool
	subsysOn  map[string]bool
	subsysOff map[string]bool

	color bool
	// known is every subsystem that has asked to log in this process, so the
	// list a user can select from is derived rather than maintained.
	known map[string]bool
}

func init() {
	state.subsysOn = map[string]bool{}
	state.subsysOff = map[string]bool{}
	state.known = map[string]bool{}
	state.level = LevelWarn
	state.color = isTTY(os.Stderr) && os.Getenv("NO_COLOR") == ""
}

// Configure applies the environment and then the command line, so a flag beats
// an exported variable.
//
//	PGB_LOG=debug           the level
//	PGB_DEBUG=nix,wrapper   subsystems at debug/trace; `all`; `-nix` excludes
//	PGB_LOG_COLOR=0|1       override tty detection
//
// verbose is the historical -v and raises the level to info.
func Configure(verbose bool, levelFlag, debugFlag string) error {
	state.mu.Lock()
	defer state.mu.Unlock()

	if v := os.Getenv("PGB_LOG"); v != "" {
		l, err := ParseLevel(v)
		if err != nil {
			return fmt.Errorf("PGB_LOG: %w", err)
		}
		state.level = l
	}
	if v := os.Getenv("PGB_DEBUG"); v != "" {
		applySubsys(v)
		if state.level < LevelDebug {
			state.level = LevelDebug
		}
	}
	if v := os.Getenv("PGB_LOG_COLOR"); v != "" {
		state.color = v != "0" && v != "no" && v != "false"
	}
	if verbose && state.level < LevelInfo {
		state.level = LevelInfo
	}
	if levelFlag != "" {
		l, err := ParseLevel(levelFlag)
		if err != nil {
			return fmt.Errorf("--log: %w", err)
		}
		state.level = l
	}
	if debugFlag != "" {
		applySubsys(debugFlag)
		if state.level < LevelDebug {
			state.level = LevelDebug
		}
	}
	return nil
}

func applySubsys(spec string) {
	for _, raw := range strings.Split(spec, ",") {
		s := strings.TrimSpace(raw)
		switch {
		case s == "":
		case strings.HasPrefix(s, "-"):
			state.subsysOff[strings.TrimPrefix(s, "-")] = true
		case s == "all" || s == "*":
			state.subsysAll = true
		default:
			state.subsysOn[s] = true
		}
	}
}

// CurrentLevel is the configured level.
func CurrentLevel() Level {
	state.mu.Lock()
	defer state.mu.Unlock()
	return state.level
}

// SubsysSpec re-renders the selection in the form Configure accepts, so it can
// be exported to a pgb re-entered inside a build environment.
func SubsysSpec() string {
	state.mu.Lock()
	defer state.mu.Unlock()
	var parts []string
	if state.subsysAll {
		parts = append(parts, "all")
	}
	for k := range state.subsysOn {
		parts = append(parts, k)
	}
	for k := range state.subsysOff {
		parts = append(parts, "-"+k)
	}
	sort.Strings(parts)
	return strings.Join(parts, ",")
}

// Enabled reports whether a subsystem may emit at the given level. Selection
// narrows debug and trace; it never silences a warning or an error.
func Enabled(subsys string, l Level) bool {
	state.mu.Lock()
	defer state.mu.Unlock()
	state.known[subsys] = true
	if l > state.level {
		return false
	}
	if l < LevelDebug {
		return true
	}
	if state.subsysOff[subsys] {
		return false
	}
	if state.subsysAll || len(state.subsysOn) == 0 {
		return true
	}
	return state.subsysOn[subsys]
}

// KnownSubsystems lists every subsystem that has asked to log in this process.
func KnownSubsystems() []string {
	state.mu.Lock()
	defer state.mu.Unlock()
	out := make([]string, 0, len(state.known))
	for k := range state.known {
		out = append(out, k)
	}
	sort.Strings(out)
	return out
}

// A Logger is one subsystem's view. The name is what PGB_DEBUG selects on, so
// keep it short, lowercase and stable.
type Logger struct{ subsys string }

// New names a subsystem.
func New(subsys string) *Logger { return &Logger{subsys: subsys} }

func (lg *Logger) emit(l Level, format string, args ...any) {
	if !Enabled(lg.subsys, l) {
		return
	}
	msg := fmt.Sprintf(format, args...)
	state.mu.Lock()
	color := state.color
	state.mu.Unlock()
	prefix := "pgb"
	if l >= LevelDebug {
		prefix = "pgb[" + lg.subsys + "]"
	}
	if color {
		prefix = colorize(l, prefix)
	}
	for _, line := range strings.Split(strings.TrimRight(msg, "\n"), "\n") {
		fmt.Fprintf(os.Stderr, "%s: %s\n", prefix, line)
	}
}

func (lg *Logger) Errorf(f string, a ...any) { lg.emit(LevelError, f, a...) }
func (lg *Logger) Warnf(f string, a ...any)  { lg.emit(LevelWarn, f, a...) }
func (lg *Logger) Infof(f string, a ...any)  { lg.emit(LevelInfo, f, a...) }
func (lg *Logger) Debugf(f string, a ...any) { lg.emit(LevelDebug, f, a...) }
func (lg *Logger) Tracef(f string, a ...any) { lg.emit(LevelTrace, f, a...) }

// DebugEnabled lets a caller skip building an expensive message.
func (lg *Logger) DebugEnabled() bool { return Enabled(lg.subsys, LevelDebug) }

func colorize(l Level, s string) string {
	switch l {
	case LevelError:
		return "\x1b[31m" + s + "\x1b[0m"
	case LevelWarn:
		return "\x1b[33m" + s + "\x1b[0m"
	case LevelInfo:
		return "\x1b[36m" + s + "\x1b[0m"
	default:
		return "\x1b[90m" + s + "\x1b[0m"
	}
}

var root = New("pgb")

// Say writes one line of the tool's answer to stdout.
func Say(format string, a ...any) { fmt.Fprintf(os.Stdout, format+"\n", a...) }

// SayRaw writes to stdout with no trailing newline, for values a caller
// captures with $(...).
func SayRaw(s string) { fmt.Fprint(os.Stdout, s) }

// Warnf reports something that changed the operation without stopping it.
func Warnf(f string, a ...any) { root.Warnf(f, a...) }

// Infof is the narration -v turns on.
func Infof(f string, a ...any) { root.Infof(f, a...) }

// Debugf is the root subsystem's debug channel.
func Debugf(f string, a ...any) { root.Debugf(f, a...) }

// Quote renders one argument so pasting it into a POSIX shell reproduces it.
// It is for display only; pgb executes argv arrays.
func Quote(s string) string {
	if s == "" {
		return "''"
	}
	for _, r := range s {
		if !(r >= 'a' && r <= 'z' || r >= 'A' && r <= 'Z' || r >= '0' && r <= '9' ||
			strings.ContainsRune("@%_+=:,./-", r)) {
			return "'" + strings.ReplaceAll(s, "'", `'\''`) + "'"
		}
	}
	return s
}

// QuoteArgs renders a whole argv for display.
func QuoteArgs(argv []string) string {
	parts := make([]string, len(argv))
	for i, a := range argv {
		parts[i] = Quote(a)
	}
	return strings.Join(parts, " ")
}

func isTTY(f *os.File) bool {
	fi, err := f.Stat()
	if err != nil {
		return false
	}
	return fi.Mode()&os.ModeCharDevice != 0
}

// EnvBool reads a 0/1-style environment variable with an explicit default.
func EnvBool(name string, def bool) bool {
	v, ok := os.LookupEnv(name)
	if !ok || v == "" {
		return def
	}
	switch strings.ToLower(v) {
	case "0", "no", "false", "off":
		return false
	case "1", "yes", "true", "on":
		return true
	}
	if n, err := strconv.Atoi(v); err == nil {
		return n != 0
	}
	return def
}
