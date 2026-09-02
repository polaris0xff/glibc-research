# pgb is one static binary. This is the whole build.
#
#   make            build ./pgb for this machine
#   make release    the distributable shape: stripped, trimmed, no cgo
#   make check      the carried selftests, then both record gates
#   make clean      remove the binary

GO      ?= go
GOFLAGS ?= -trimpath
BIN     ?= pgb

export CGO_ENABLED = 0

.PHONY: all release check clean vet fmt

all: $(BIN)

$(BIN): $(shell find . -name '*.go' -not -path './HISTORY/*' 2>/dev/null) go.mod
	$(GO) build $(GOFLAGS) -o $(BIN) ./cmd/pgb

release:
	GOOS=linux GOARCH=amd64 GOAMD64=v1 \
	  $(GO) build $(GOFLAGS) -ldflags='-s -w' -o $(BIN) ./cmd/pgb
	@printf '%-14s %s bytes\n' "$(BIN)" "$$(wc -c < $(BIN))"
	@file $(BIN)

# The exit-code contract is 0 ok, 1 a case ran and failed, 2 a case could not
# run on this machine. Only 1 stops the gates; 2 is reported and they still run,
# because a machine without zstd must still be able to check the record.
check: $(BIN)
	@./$(BIN) selftest; rc=$$?; \
	  case $$rc in \
	    0) ;; \
	    2) echo "make check: some selftests could not run here; recorded, not asserted" ;; \
	    *) exit $$rc ;; \
	  esac
	sh TODO/check.sh
	sh scripts/common/check-docs.sh

vet:
	$(GO) vet ./...

fmt:
	$(GO) fmt ./...

clean:
	rm -f $(BIN)
