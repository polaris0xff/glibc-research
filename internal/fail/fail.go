// Package fail carries pgb's exit-code contract: 0 ok, 1 the operation ran and
// failed, 2 it could not run.
//
// Commands return an error; main turns it into a status. Wrapping an error
// preserves the code of the innermost fail.Error, so a helper's "could not
// run" does not become a caller's "ran and failed".
//
// SPDX-License-Identifier: MIT
package fail

import (
	"errors"
	"fmt"
)

// Error is an error with an exit code.
type Error struct {
	Code int
	Err  error
}

func (e *Error) Error() string { return e.Err.Error() }
func (e *Error) Unwrap() error { return e.Err }

// Ran reports an operation that ran and failed (exit 1).
func Ran(format string, a ...any) error {
	return &Error{Code: 1, Err: fmt.Errorf(format, a...)}
}

// Cannot reports an operation that could not run (exit 2): a missing tool, a
// missing environment, a malformed argument.
func Cannot(format string, a ...any) error {
	return &Error{Code: 2, Err: fmt.Errorf(format, a...)}
}

// Code extracts the exit code an error asks for, defaulting to 1.
func Code(err error) int {
	if err == nil {
		return 0
	}
	var e *Error
	if errors.As(err, &e) {
		return e.Code
	}
	return 1
}

// Silent marks an error whose message has already been printed, so main exits
// with the code without repeating it.
type Silent struct{ Code int }

func (s *Silent) Error() string { return "" }

// Exit returns an error that only sets the status.
func Exit(code int) error { return &Silent{Code: code} }

// IsSilent reports whether main should suppress the message.
func IsSilent(err error) (int, bool) {
	var s *Silent
	if errors.As(err, &s) {
		return s.Code, true
	}
	return 0, false
}
