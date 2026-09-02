// Package selftest is the shared shape of pgb's carried-in selftests: named
// cases, an observed value, an expected value, and a count that decides the
// exit status.
//
// Every case prints what it saw as well as whether it matched, so a failure is
// diagnosable from the output alone.
//
// SPDX-License-Identifier: MIT
package selftest

import (
	"fmt"
	"io"
	"os"
	"strings"
)

// A Case is one assertion.
type Case struct {
	Name string
	Got  string
	Want string
	OK   bool
}

// A Report accumulates the cases of one selftest.
type Report struct {
	Subject string
	Cases   []Case
	Skipped []string
}

// New starts a report for a named subject.
func New(subject string) *Report { return &Report{Subject: subject} }

// Check records a case comparing got against want.
func (r *Report) Check(name, got, want string) bool {
	ok := got == want
	r.Cases = append(r.Cases, Case{Name: name, Got: got, Want: want, OK: ok})
	return ok
}

// CheckBool records a boolean case.
func (r *Report) CheckBool(name string, got, want bool) bool {
	return r.Check(name, boolStr(got), boolStr(want))
}

// Fail records a case that could not even be evaluated.
func (r *Report) Fail(name, got, want string) {
	r.Cases = append(r.Cases, Case{Name: name, Got: got, Want: want, OK: false})
}

// Skip records something the environment could not run, which counts as a
// failure: a selftest that quietly runs nothing reports success.
func (r *Report) Skip(reason string) { r.Skipped = append(r.Skipped, reason) }

// Failures counts cases that did not match, plus anything skipped.
func (r *Report) Failures() int {
	n := len(r.Skipped)
	for _, c := range r.Cases {
		if !c.OK {
			n++
		}
	}
	return n
}

// Write prints the report and returns the exit status: 0 all passed, 1 not.
func (r *Report) Write(w io.Writer) int {
	for _, c := range r.Cases {
		if c.OK {
			fmt.Fprintf(w, "  ok    %s = %s\n", c.Name, c.Got)
			continue
		}
		fmt.Fprintf(w, "  FAIL  %s = %s, wanted %s\n", c.Name, c.Got, c.Want)
	}
	for _, s := range r.Skipped {
		fmt.Fprintf(w, "  SKIP  %s\n", s)
	}
	if n := r.Failures(); n > 0 {
		fmt.Fprintf(w, "%s --selftest: %d of %d case(s) FAILED or skipped.\n",
			r.Subject, n, len(r.Cases)+len(r.Skipped))
		return 1
	}
	fmt.Fprintf(w, "%s --selftest: %d cases, all pass.\n", r.Subject, len(r.Cases))
	return 0
}

// Print writes the report to stdout.
func (r *Report) Print() int { return r.Write(os.Stdout) }

// Merge folds another report in under a prefix, for a combined run.
func (r *Report) Merge(other *Report) {
	for _, c := range other.Cases {
		c.Name = other.Subject + ": " + c.Name
		r.Cases = append(r.Cases, c)
	}
	for _, s := range other.Skipped {
		r.Skipped = append(r.Skipped, other.Subject+": "+s)
	}
}

func boolStr(b bool) string {
	if b {
		return "yes"
	}
	return "no"
}

// Lines is a helper for cases whose observed value is a list.
func Lines(v []string) string { return strings.Join(v, " ") }
