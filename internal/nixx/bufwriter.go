// bufwriter.go — a buffered writer for the index, kept separate so the index
// reads as the format it describes.
//
// SPDX-License-Identifier: MIT
package nixx

import (
	"bufio"
	"io"
)

func newBufWriter(w io.Writer) *bufio.Writer { return bufio.NewWriterSize(w, 1<<20) }
