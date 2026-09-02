// onelf.go — turn a built AppDir into an onelf recipe, so the same payload can
// be packed by another format and compared.
//
// onelf's TOML cannot repeat a key, which is what made the repeated .env
// entries impossible to miss; the recipe therefore uses the same folded view
// FoldEnv produces. ${SHARUN_DIR} becomes ${ONELF_DIR}, and any other ${VAR}
// is a live variable, which onelf spells $${VAR}.
//
// SPDX-License-Identifier: MIT
package bundle

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
)

// liveVar matches a ${NAME} that is not ONELF_DIR.
var liveVar = regexp.MustCompile(`\$\{([A-Za-z_][A-Za-z0-9_]*)\}`)

// WriteOnelfRecipe renders the recipe for an AppDir.
func WriteOnelfRecipe(w io.Writer, appDir, mainProg string, level int) error {
	fmt.Fprintf(w, "[package]\nname = %q\ncommand = %q\n\n", mainProg, "bin/"+mainProg)

	binDir := filepath.Join(appDir, "shared", "bin")
	if entries, err := os.ReadDir(binDir); err == nil {
		var names []string
		for _, e := range entries {
			if e.Name() != mainProg {
				names = append(names, e.Name())
			}
		}
		sort.Strings(names)
		for _, n := range names {
			fmt.Fprintf(w, "[[entrypoint]]\nname = %q\npath = %q\n\n", n, "bin/"+n)
		}
	}
	fmt.Fprintf(w, "[compression]\nlevel = %d\n\n[bundle]\nskip = true\n\n", level)

	order, acc, err := readEnv(filepath.Join(appDir, ".env"))
	if err != nil || len(order) == 0 {
		return nil
	}
	fmt.Fprintln(w, "[env]")
	for _, k := range order {
		v := acc[k]
		sep := ":"
		if strings.Contains(v, ";") && !strings.Contains(v, ":") {
			sep = ";"
		}
		v = toOnelf(dedupePath(v, sep))
		v = strings.ReplaceAll(v, liveMarker, "$${"+k+"}")
		v = strings.ReplaceAll(v, `\`, `\\`)
		v = strings.ReplaceAll(v, `"`, `\"`)
		fmt.Fprintf(w, "%s = \"%s\"\n", k, v)
	}
	return nil
}

func toOnelf(v string) string {
	v = strings.ReplaceAll(v, "${SHARUN_DIR}", "${ONELF_DIR}")
	v = liveVar.ReplaceAllStringFunc(v, func(m string) string {
		if m == "${ONELF_DIR}" {
			return m
		}
		return "$" + m
	})
	return v
}

// readEnv replays an environment file the way FoldEnv does, without writing it
// back, so the recipe and the folded file agree by construction.
func readEnv(path string) ([]string, map[string]string, error) {
	b, err := os.ReadFile(path)
	if err != nil {
		return nil, nil, err
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
	return order, acc, nil
}
