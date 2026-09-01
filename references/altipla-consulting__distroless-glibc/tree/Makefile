
FILES = $(shell find . -type f -name '*.go')

lint:
	go install ./...
	linter ./...
	go vet ./...

gofmt:
	@gofmt -s -w $(FILES)
	@gofmt -r '&α{} -> new(α)' -w $(FILES)
	@impsort . -p github.com/altipla-consulting/distroless-glibc
