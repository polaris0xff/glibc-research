// stamp.go — the live build log: a timestamp on every line and a heartbeat
// when there are none.
//
// The column surface follows `ts`; the heartbeat exists so a long silent step
// is distinguishable from a hang. Relative time counts total hours, so a run
// past 24 hours does not wrap back to zero.
//
// SPDX-License-Identifier: MIT
package logx

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"strings"
	"sync"
	"time"
)

// A Column is one timestamp field.
type Column string

const (
	ColRel   Column = "rel"   // hh:mm:ss.mmm since the stream opened
	ColDelta Column = "delta" // since the previous line
	ColWall  Column = "wall"  // local wall clock, HH:MM:SS.mmm
	ColISO   Column = "iso"   // RFC3339 with milliseconds
	ColEpoch Column = "epoch" // seconds since the epoch, with milliseconds
)

var knownColumns = []Column{ColRel, ColDelta, ColWall, ColISO, ColEpoch}

// StampConfig configures a Stamper.
type StampConfig struct {
	Columns   []Column      // printed in this order
	Separator string        // between columns, and before the line
	Heartbeat time.Duration // 0 disables
	Prefix    string        // e.g. "  "
	Out       io.Writer     // nil means os.Stderr
}

// ParseColumns turns "rel,delta" into the ordered list. An unknown or repeated
// name is an error naming the valid set.
func ParseColumns(spec string) ([]Column, error) {
	var out []Column
	seen := map[Column]bool{}
	for _, raw := range strings.Split(spec, ",") {
		s := Column(strings.ToLower(strings.TrimSpace(raw)))
		if s == "" {
			continue
		}
		ok := false
		for _, k := range knownColumns {
			if s == k {
				ok = true
				break
			}
		}
		if !ok {
			names := make([]string, len(knownColumns))
			for i, k := range knownColumns {
				names[i] = string(k)
			}
			return nil, fmt.Errorf("unknown timestamp column %q; the set is: %s",
				s, strings.Join(names, ", "))
		}
		if seen[s] {
			return nil, fmt.Errorf("timestamp columns name %q twice", s)
		}
		seen[s] = true
		out = append(out, s)
	}
	if len(out) == 0 {
		return nil, fmt.Errorf("no timestamp columns given")
	}
	return out, nil
}

// StampFromEnv overlays the environment onto def and reports whether stamping
// is enabled.
//
//	PGB_TS=0|1
//	PGB_TS_COLUMNS=rel,delta
//	PGB_TS_SEPARATOR='  '
//	PGB_TS_HEARTBEAT=30s   ('0' or 'off' disables)
func StampFromEnv(def StampConfig) (StampConfig, bool, error) {
	c := def
	if c.Separator == "" {
		c.Separator = "  "
	}
	if c.Columns == nil {
		c.Columns = []Column{ColRel, ColDelta}
	}
	on := EnvBool("PGB_TS", true)
	if v := os.Getenv("PGB_TS_COLUMNS"); v != "" {
		cols, err := ParseColumns(v)
		if err != nil {
			return c, on, fmt.Errorf("PGB_TS_COLUMNS: %w", err)
		}
		c.Columns = cols
	}
	if v, ok := os.LookupEnv("PGB_TS_SEPARATOR"); ok {
		c.Separator = v
	}
	if v := os.Getenv("PGB_TS_HEARTBEAT"); v != "" {
		if v == "0" || strings.EqualFold(v, "off") {
			c.Heartbeat = 0
		} else {
			d, err := time.ParseDuration(v)
			if err != nil {
				return c, on, fmt.Errorf("PGB_TS_HEARTBEAT: %w", err)
			}
			c.Heartbeat = d
		}
	}
	return c, on, nil
}

// A Stamper timestamps lines as they arrive and emits a heartbeat when they
// stop. It is safe for concurrent writers, so one Stamper serves a child's
// stdout and stderr together.
type Stamper struct {
	cfg   StampConfig
	out   io.Writer
	start time.Time

	mu    sync.Mutex
	last  time.Time
	lines int64
	bytes int64

	stop chan struct{}
	done chan struct{}
	once sync.Once
}

// NewStamper starts the heartbeat if one is configured. Close it when the
// stream ends.
func NewStamper(cfg StampConfig) *Stamper {
	out := cfg.Out
	if out == nil {
		out = os.Stderr
	}
	if len(cfg.Columns) == 0 {
		cfg.Columns = []Column{ColRel, ColDelta}
	}
	if cfg.Separator == "" {
		cfg.Separator = "  "
	}
	now := time.Now()
	s := &Stamper{cfg: cfg, out: out, start: now, last: now,
		stop: make(chan struct{}), done: make(chan struct{})}
	if cfg.Heartbeat > 0 {
		go s.beat()
	} else {
		close(s.done)
	}
	return s
}

// beat polls at a quarter of the interval so a heartbeat fires close to when
// it is due rather than up to a full interval late.
func (s *Stamper) beat() {
	defer close(s.done)
	tick := time.NewTicker(s.cfg.Heartbeat / 4)
	defer tick.Stop()
	for {
		select {
		case <-s.stop:
			return
		case <-tick.C:
			s.mu.Lock()
			idle := time.Since(s.last)
			if idle >= s.cfg.Heartbeat {
				// Silence is measured on this side of the pipe: a child that
				// buffers its own output is quiet here while busy there, so
				// the wording says "no output", not "stalled".
				s.writeLocked(fmt.Sprintf("… still running, no output for %s (%d lines, %s so far)",
					roundDur(idle), s.lines, humanBytes(s.bytes)))
				s.last = time.Now()
			}
			s.mu.Unlock()
		}
	}
}

// Write implements io.Writer over whole lines.
func (s *Stamper) Write(p []byte) (int, error) {
	s.mu.Lock()
	s.bytes += int64(len(p))
	s.mu.Unlock()
	return len(p), s.pump(strings.NewReader(string(p)))
}

// Pipe reads r to EOF, stamping each line.
func (s *Stamper) Pipe(r io.Reader) error { return s.pump(r) }

func (s *Stamper) pump(r io.Reader) error {
	br := bufio.NewReaderSize(r, 64*1024)
	for {
		line, err := br.ReadString('\n')
		if line != "" {
			s.Line(strings.TrimRight(line, "\r\n"))
		}
		if err != nil {
			if err == io.EOF {
				return nil
			}
			return err
		}
	}
}

// Line stamps and emits exactly one line.
func (s *Stamper) Line(text string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.lines++
	s.writeLocked(text)
	s.last = time.Now()
}

func (s *Stamper) writeLocked(text string) {
	now := time.Now()
	var cols []string
	for _, c := range s.cfg.Columns {
		switch c {
		case ColRel:
			cols = append(cols, relative(now.Sub(s.start)))
		case ColDelta:
			cols = append(cols, "+"+shortDur(now.Sub(s.last)))
		case ColWall:
			cols = append(cols, now.Format("15:04:05.000"))
		case ColISO:
			cols = append(cols, now.Format("2006-01-02T15:04:05.000Z07:00"))
		case ColEpoch:
			cols = append(cols, fmt.Sprintf("%d.%03d", now.Unix(), now.Nanosecond()/1e6))
		}
	}
	fmt.Fprintf(s.out, "%s%s%s%s\n", s.cfg.Prefix,
		strings.Join(cols, s.cfg.Separator), s.cfg.Separator, text)
}

// Close stops the heartbeat. It is idempotent.
func (s *Stamper) Close() {
	s.once.Do(func() { close(s.stop) })
	<-s.done
}

// Elapsed is how long this stream has been open.
func (s *Stamper) Elapsed() time.Duration { return time.Since(s.start) }

// Counts reports lines and bytes seen.
func (s *Stamper) Counts() (int64, int64) {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.lines, s.bytes
}

func relative(d time.Duration) string {
	if d < 0 {
		d = 0
	}
	ms := d.Milliseconds()
	return fmt.Sprintf("%02d:%02d:%02d.%03d", ms/3600000, (ms/60000)%60, (ms/1000)%60, ms%1000)
}

func shortDur(d time.Duration) string {
	if d < 0 {
		d = 0
	}
	switch {
	case d < time.Second:
		return fmt.Sprintf("%.3fs", d.Seconds())
	case d < time.Minute:
		return fmt.Sprintf("%.2fs", d.Seconds())
	default:
		return roundDur(d)
	}
}

func roundDur(d time.Duration) string {
	switch {
	case d < time.Minute:
		return fmt.Sprintf("%.0fs", d.Seconds())
	case d < time.Hour:
		return fmt.Sprintf("%dm%02ds", int(d.Minutes()), int(d.Seconds())%60)
	default:
		return fmt.Sprintf("%dh%02dm", int(d.Hours()), int(d.Minutes())%60)
	}
}

func humanBytes(n int64) string {
	switch {
	case n < 1024:
		return fmt.Sprintf("%d B", n)
	case n < 1024*1024:
		return fmt.Sprintf("%.1f KiB", float64(n)/1024)
	default:
		return fmt.Sprintf("%.1f MiB", float64(n)/(1024*1024))
	}
}
