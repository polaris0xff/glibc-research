// wrapper.go — read a nixpkgs wrapper: what it execs, and what environment it
// sets on the way.
//
// nixpkgs installs many programs as a wrapper in bin/ that sets variables and
// execs the real ELF out of a second store path. A bundle does not run the
// wrapper — sharun runs the real ELF and reads .env — so the job is to LIFT
// the assignments out, not to keep a script alive.
//
// Two shapes exist and both are read. makeBinaryWrapper compiles a C program
// and embeds the generator command as a comment; makeWrapper writes a shell
// script. A wrapper doing something neither form covers is reported as "not a
// wrapper" rather than half-understood.
//
// SPDX-License-Identifier: MIT
package bundle

import (
	"bytes"
	"os"
	"regexp"
	"strings"
)

// WrapOp is what one record asks for.
type WrapOp string

const (
	OpTarget WrapOp = "target" // the real program the wrapper execs
	OpSet    WrapOp = "set"
	OpPrefix WrapOp = "prefix"
	OpSuffix WrapOp = "suffix"
	OpArgv0  WrapOp = "argv0"
	OpFlags  WrapOp = "flags"
)

// WrapRecord is one thing a wrapper does.
type WrapRecord struct {
	Op    WrapOp
	Var   string
	Sep   string
	Value string
}

// findCWrapperCommand extracts makeCWrapper's embedded generator command from
// a compiled wrapper: the text after the marker, up to the blank line that
// ends the block. It is scanned rather than matched with a regular expression
// because the block can be tens of kilobytes and Go's regexp caps a repeat
// count at 1000.
func findCWrapperCommand(blob []byte) (string, bool) {
	const marker = "makeCWrapper"
	_, rest, ok := bytes.Cut(blob, []byte(marker))
	if !ok {
		return "", false
	}
	if len(rest) == 0 {
		return "", false
	}
	limit := min(len(rest), 1<<16)
	rest = rest[:limit]
	// The block ends at the first newline followed by a line with nothing on
	// it but whitespace.
	for j := 0; j < len(rest)-1; j++ {
		if rest[j] != '\n' {
			continue
		}
		k := j + 1
		for k < len(rest) && (rest[k] == ' ' || rest[k] == '\t' || rest[k] == '\r') {
			k++
		}
		if k < len(rest) && rest[k] == '\n' {
			return string(rest[:j]), true
		}
	}
	return string(rest), true
}

// ReadWrapper reads either wrapper shape. It returns nil when the file is not
// a wrapper.
func ReadWrapper(path string) []WrapRecord {
	f, err := os.Open(path)
	if err != nil {
		return nil
	}
	var magic [4]byte
	n, _ := f.Read(magic[:])
	f.Close()
	if n == 4 && magic == [4]byte{0x7f, 'E', 'L', 'F'} {
		return readBinaryWrapper(path)
	}
	return readShellWrapper(path)
}

func readBinaryWrapper(path string) []WrapRecord {
	blob, err := os.ReadFile(path)
	if err != nil {
		return nil
	}
	cmd, ok := findCWrapperCommand(blob)
	if !ok {
		return nil
	}
	argv := splitWords(joinContinuations(cmd))
	if len(argv) == 0 {
		return nil
	}
	recs := []WrapRecord{{Op: OpTarget, Value: argv[0]}}
	for i := 1; i < len(argv); {
		a := argv[i]
		switch {
		case a == "--inherit-argv0":
			recs = append(recs, WrapRecord{Op: OpArgv0})
			i++
		case (a == "--set" || a == "--set-default") && i+2 < len(argv):
			recs = append(recs, WrapRecord{Op: OpSet, Var: argv[i+1], Value: argv[i+2]})
			i += 3
		case (a == "--prefix" || a == "--suffix" || a == "--prefix-each" || a == "--suffix-each") && i+3 < len(argv):
			op := OpSuffix
			if strings.HasPrefix(a, "--prefix") {
				op = OpPrefix
			}
			recs = append(recs, WrapRecord{Op: op, Var: argv[i+1], Sep: argv[i+2], Value: argv[i+3]})
			i += 4
		case (a == "--add-flags" || a == "--append-flags") && i+1 < len(argv):
			recs = append(recs, WrapRecord{Op: OpFlags, Value: argv[i+1]})
			i += 2
		case a == "--argv0" && i+1 < len(argv):
			recs = append(recs, WrapRecord{Op: OpArgv0, Value: argv[i+1]})
			i += 2
		default:
			i++
		}
	}
	return recs
}

// joinContinuations folds the block's backslash-newlines and stops at the
// first comment line, because prose about nix-shell follows it.
func joinContinuations(text string) string {
	text = strings.ReplaceAll(text, "\\\n", " ")
	var out []string
	for line := range strings.SplitSeq(text, "\n") {
		s := strings.TrimSpace(line)
		if strings.HasPrefix(s, "#") {
			break
		}
		out = append(out, s)
	}
	return strings.TrimSpace(strings.Join(out, " "))
}

var (
	exportRe = regexp.MustCompile(`^\s*export\s+([A-Za-z_][A-Za-z0-9_]*)=(.*)$`)
	assignRe = regexp.MustCompile(`^\s*([A-Za-z_][A-Za-z0-9_]*)=(.*?)(?:\s*;?\s*export\s+[A-Za-z_][A-Za-z0-9_]*)?\s*$`)
	execRe   = regexp.MustCompile(`^\s*exec\s+(?:-a\s+(?:"\$0"|\S+)\s+)?(\S+)`)
)

func readShellWrapper(path string) []WrapRecord {
	b, err := os.ReadFile(path)
	if err != nil {
		return nil
	}
	if len(b) > 65536 {
		b = b[:65536]
	}
	if !bytes.HasPrefix(b, []byte("#!")) {
		return nil
	}
	var recs []WrapRecord
	target := ""
	for line := range strings.SplitSeq(string(b), "\n") {
		if m := execRe.FindStringSubmatch(line); m != nil && strings.HasPrefix(m[1], "/nix/store/") {
			target = m[1]
			continue
		}
		m := exportRe.FindStringSubmatch(line)
		if m == nil {
			m = assignRe.FindStringSubmatch(line)
		}
		if m == nil {
			continue
		}
		name, raw := m[1], strings.TrimSpace(m[2])
		if (name == "PATH" || name == "LD_LIBRARY_PATH") && raw == "" {
			continue
		}
		val := strings.Trim(raw, `"'`)
		ref := "$" + name
		body := strings.ReplaceAll(val, "${"+name+"}", ref)
		if strings.Contains(body, ref) {
			before, after, _ := strings.Cut(body, ref)
			switch {
			case before != "" && after == "":
				recs = append(recs, WrapRecord{Op: OpPrefix, Var: name,
					Sep: lastRune(before), Value: strings.TrimRight(before, ":;")})
			case after != "" && before == "":
				recs = append(recs, WrapRecord{Op: OpSuffix, Var: name,
					Sep: firstRune(after), Value: strings.TrimLeft(after, ":;")})
			default:
				recs = append(recs, WrapRecord{Op: OpSet, Var: name, Value: val})
			}
			continue
		}
		recs = append(recs, WrapRecord{Op: OpSet, Var: name, Value: val})
	}
	if len(recs) == 0 && target == "" {
		return nil
	}
	if target != "" {
		recs = append([]WrapRecord{{Op: OpTarget, Value: target}}, recs...)
	}
	return recs
}

func lastRune(s string) string {
	if s == "" {
		return ":"
	}
	return s[len(s)-1:]
}

func firstRune(s string) string {
	if s == "" {
		return ":"
	}
	return s[:1]
}

// splitWords is a POSIX-shell word split with quoting, enough for the argv a
// wrapper generator embeds. An unterminated quote yields what it has rather
// than failing, because the block is a comment and can be truncated.
func splitWords(s string) []string {
	var out []string
	var cur strings.Builder
	inWord := false
	var quote byte
	for i := 0; i < len(s); i++ {
		c := s[i]
		switch {
		case quote != 0:
			if c == quote {
				quote = 0
				continue
			}
			if quote == '"' && c == '\\' && i+1 < len(s) {
				i++
				cur.WriteByte(s[i])
				continue
			}
			cur.WriteByte(c)
		case c == '\'' || c == '"':
			quote = c
			inWord = true
		case c == '\\' && i+1 < len(s):
			i++
			cur.WriteByte(s[i])
			inWord = true
		case c == ' ' || c == '\t' || c == '\n':
			if inWord {
				out = append(out, cur.String())
				cur.Reset()
				inWord = false
			}
		default:
			cur.WriteByte(c)
			inWord = true
		}
	}
	if inWord || cur.Len() > 0 {
		out = append(out, cur.String())
	}
	return out
}

// WrapperTarget returns the program a wrapper execs, or "".
func WrapperTarget(recs []WrapRecord) string {
	for _, r := range recs {
		if r.Op == OpTarget {
			return r.Value
		}
	}
	return ""
}
