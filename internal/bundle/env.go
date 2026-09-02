// env.go — fold a bundle's .env.
//
// The lifting step appends a record per program, so a bundle carrying four
// programs from one Qt application asks Qt to scan the same plugin
// directories four times over on every start. Folding replays the file in
// order, resolves each ${KEY} against what the file has said so far, and drops
// repeated path components — keeping the first occurrence's position, so what
// resolved first still resolves first.
//
// A ${KEY} in a key's FIRST mention means the live variable from the
// environment and is preserved as such.
//
// SPDX-License-Identifier: MIT
package bundle

import (
	"fmt"
	"os"
	"strings"
)

// liveMarker stands in for "the variable as the process already has it" while
// the file is being replayed, so a later mention cannot resolve it away.
const liveMarker = "\x00LIVE\x00"

// FoldEnv rewrites a bundle's environment file in place and returns the number
// of keys and the before/after sizes.
func FoldEnv(path string) (keys int, before, after int64, err error) {
	fi, err := os.Stat(path)
	if err != nil {
		return 0, 0, 0, err
	}
	before = fi.Size()

	b, err := os.ReadFile(path)
	if err != nil {
		return 0, before, 0, err
	}
	var order []string
	acc := map[string]string{}
	for _, raw := range strings.Split(string(b), "\n") {
		line := strings.TrimSpace(raw)
		if line == "" || strings.HasPrefix(line, "#") || !strings.Contains(line, "=") {
			continue
		}
		k, v, _ := strings.Cut(line, "=")
		k = strings.TrimSpace(k)
		if k == "" {
			continue
		}
		self := "${" + k + "}"
		if _, seen := acc[k]; !seen {
			order = append(order, k)
			acc[k] = strings.ReplaceAll(v, self, liveMarker)
			continue
		}
		acc[k] = strings.ReplaceAll(v, self, acc[k])
	}
	if len(order) == 0 {
		return 0, before, before, nil
	}

	var out strings.Builder
	for _, k := range order {
		v := acc[k]
		// A value using ';' and no ':' is separated by ';' — Lua's paths are
		// the case that shows it.
		sep := ":"
		if strings.Contains(v, ";") && !strings.Contains(v, ":") {
			sep = ";"
		}
		v = dedupePath(v, sep)
		v = strings.ReplaceAll(v, liveMarker, "${"+k+"}")
		fmt.Fprintf(&out, "%s=%s\n", k, v)
	}
	tmp := path + ".part"
	if err := os.WriteFile(tmp, []byte(out.String()), 0o644); err != nil {
		return len(order), before, 0, err
	}
	if err := os.Rename(tmp, path); err != nil {
		return len(order), before, 0, err
	}
	if fi, err := os.Stat(path); err == nil {
		after = fi.Size()
	}
	return len(order), before, after, nil
}

// dedupePath drops repeated components, keeping the first occurrence.
func dedupePath(v, sep string) string {
	if !strings.Contains(v, sep) {
		return v
	}
	seen := map[string]bool{}
	var out []string
	for _, part := range strings.Split(v, sep) {
		if seen[part] {
			continue
		}
		seen[part] = true
		out = append(out, part)
	}
	return strings.Join(out, sep)
}
