---
tags:
  - cli
  - explanation
---

# CLI Design

This page documents every design decision for the FlatRoot CLI, and the reasoning behind it — what each command's shape is, why positionals vs flags, why a particular name or convention, and how each choice ties back to the design rules below. For the full flag-by-flag reference and source-format grammar, see [CLI reference](../reference/cli.md).

## Design principles

The CLI design rules below synthesize established guidance from POSIX, GNU, clig.dev, 12-Factor CLI Apps, and the Heroku style guide. They stand independent of flatroot's specific choices; the per-subcommand sections that follow show where flatroot applies each rule and where it deviates.

??? note "Arguments and flags"

    1. **Prefer flags over positional arguments.**

        > "Prefer flags to args."
        > — [clig.dev, "Arguments and flags"](https://clig.dev)
        > — [Jeff Dickey, *12 Factor CLI Apps*, factor 2](https://medium.com/@jdxcode/12-factor-cli-apps-dd3c227a0e46)

    2. **Every short option has a long form; short letters reserved for the most-common options.**

        > "Have full-length versions of every short option." ... "Short letters only for most-common flags."
        > — [clig.dev, "Arguments and flags"](https://clig.dev)

    3. **Support bare `-` for stdin/stdout when the operand is a file.**

        > "A `-` operand used alone as an argument following the options shall be interpreted ... as standard input or standard output." (Guideline 12)
        > — [POSIX.1-2017 §12.2](https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap12.html)
        > "Support `-` for stdin/stdout when input/output is a file."
        > — [clig.dev, "Arguments and flags"](https://clig.dev)

    4. **Never accept secrets via flags.**

        > "Never read secrets from flags — `--password foo` appears in `ps` and shell history."
        > — [clig.dev, "Arguments and flags"](https://clig.dev)

    5. **Boolean flags use paired `--foo` / `--no-foo` for negation.**

        > "Paired `--foo` / `--no-foo` for negation ... avoids the 'flag day' problem where adding a flag that changes the default breaks existing invocations."
        > — [GNU Coding Standards §4.6](https://www.gnu.org/prep/standards/html_node/Command_002dLine-Interfaces.html)

    6. **Reject arbitrary prefix abbreviation of long options.**

        > "Don't accept arbitrary abbreviations (locks out future names)."
        > — [clig.dev, "Subcommands"](https://clig.dev)

??? note "Subcommands"

    7. **Noun-verb when a noun has a family of verbs; bare plural or flat verb when one verb covers the domain.**

        > "Noun-verb (`docker container create`, `aws s3 cp`, `gh pr list`) scales better to many resources and is preferred by clig.dev."
        > — [clig.dev, "Subcommands"](https://clig.dev)
        > "Never create `*:list` subcommands — bare `heroku config` should list."
        > — [Heroku CLI Style Guide](https://devcenter.heroku.com/articles/cli-style-guide)

    8. **No catch-all subcommands.**

        > "Don't have catch-all subcommands (breaks future extensibility)."
        > — [clig.dev, "Subcommands"](https://clig.dev)

    9. **Avoid ambiguous subcommand pairs.**

        > "Avoid ambiguous pairs (don't ship both `update` and `upgrade`)."
        > — [clig.dev, "Subcommands"](https://clig.dev)

    10. **Options not owned by any specific subcommand belong in the global scope.**

        A flag is *owned* by a subcommand if its meaning or value space changes depending on which subcommand runs it. Cross-cutting infrastructure (transport config, retry policy) and session context (which source, which cluster, which region) have identical meaning across every subcommand that uses them — those go at the tool root. Action-specific flags stay local to the subcommand they modify.

        > Ecosystem precedent (no formal rule in the research set; each quotation below is from the tool's official reference):
        >
        > - [`kubectl --namespace` / `-n`](https://kubernetes.io/docs/reference/kubectl/) — scopes an operation to a Kubernetes namespace. Global because every namespaced-resource verb (`get`, `apply`, `delete`, `logs`, `describe`) uses the same value with the same meaning; no single verb owns it.
        > - [`docker --host` / `-H`](https://docs.docker.com/reference/cli/docker/) — *"Daemon socket to connect to."* Global because every docker verb (`ps`, `run`, `build`, `image ls`, `network create`) talks to the same daemon; the address is transport infrastructure, not action state.
        > - [`git --git-dir`](https://git-scm.com/docs/git) — *"Set the path to the repository ('.git' directory)."* Listed in the SYNOPSIS as a top-level option that can appear before any subcommand. Global because every git command operates against a repository; `--git-dir` re-targets the entire toolchain at once.
        > - [`aws --region`](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-options.html) — *"Specifies which AWS Region to send this command's AWS request to."* Global because AWS APIs are region-scoped; `s3`, `ec2`, `iam`, `lambda` all dispatch to the same region.
        > - [`helm --kube-context`](https://helm.sh/docs/helm/helm/) — *"name of the kubeconfig context to use."* Global because every helm action (`install`, `upgrade`, `uninstall`, `list`, `status`) targets a cluster through the same context.

??? note "Configuration and precedence"

    11. **Precedence: CLI flags > environment variables > project config > user config > system config.**

        > "Precedence: flags > env > project > user > system."
        > — [clig.dev, "Configuration"](https://clig.dev)

    12. **Never accept secrets via environment variables.**

        > "Do not read secrets from environment variables — they leak through `ps`, `/proc/*/environ`, `docker inspect`, `systemctl show`, child processes, and crash dumps."
        > — [clig.dev, "Environment variables"](https://clig.dev)

??? note "Interaction and safety"

    13. **Confirm dangerous actions with severity-scaled friction.**

        > "Mild = maybe prompt; moderate = definitely prompt or dry-run; severe = require typing a non-trivial confirmation string like the resource name."
        > — [clig.dev, "Interactivity"](https://clig.dev)

    14. **Never require an interactive prompt — always allow flags / args / stdin to bypass.**

        > "Never require a prompt — always allow flags/args/stdin to bypass; if `--no-input` is passed, don't prompt."
        > — [clig.dev, "Interactivity"](https://clig.dev)

??? note "Extensibility"

    15. **Changes are additive; warn before breaking.**

        > "Keep changes additive; warn before breaking. Don't create 'time bombs'."
        > — [clig.dev, "Future-proofing"](https://clig.dev)

## Global options

| Flag | Required for | Default | Role |
|------|--------------|---------|------|
| `--from <SOURCE>` | every subcommand that touches a distribution index | *(no default — command errors if missing)* | Identifies the distribution source; the `<distro>:<release>[@<date>]` DSL |
| `--arch <ARCH>` | (optional everywhere) | Host architecture (`uname -m`) | Target architecture for the rootfs; a comma-separated multiarch list is accepted by `install` only — `search`, `query`, and `analyze trace` take a single architecture |
| `--http-retries <N>` | (optional everywhere) | `3` | Max HTTP retry attempts for index + package downloads |
| `-v`, `--verbose` | (optional everywhere) | `false` | Emit diagnostic warnings that are hidden by default |

### Selecting the Distribution

The core of FlatRoot mechanism requires the selection of a remote to read and pull the linux distribution packages from. The selected flag for this was `--from`, on the global namespace, it defines internal data that is shared by existing commands and could be used by novel ones as well. The flag is analogous to the `FROM` clause in a `Dockerfile` to enhance usage intuition.

```bash
flatroot --from debian:bookworm search firefox-esr
```

??? note "Precedent"

    ```bash
    # docker pull — image reference DSL with the same <name>:<variant>[@<qualifier>] shape
    docker pull debian:bookworm
    docker pull debian:bookworm@sha256:abc123…

    # apt-get --target-release — flag-based release selection
    apt-get --target-release=bookworm install firefox

    # pip install --index-url — flag-based source-index selection
    pip install --index-url https://pypi.org/simple/ requests

    # cargo install --registry — flag-based registry selection
    cargo install --registry my-registry my-crate
    ```

### Selecting the Architecture

Cross-architecture builds are a core FlatRoot capability: a user on `x86_64` must be able to build a rootfs for `aarch64`, `riscv64`, or a multiarch target, independent of what their host CPU is. The selected flag for this was `--arch`, on the global namespace; it defines internal data (the target architecture) shared by every subcommand that resolves or downloads packages (per the rule of global scope for cross-cutting options). The flag name mirrors the kernel naming convention printed by `uname -m`, so `--arch x86_64` is the same string a user would write in any kernel-facing context.

```bash
# Explicit flag
flatroot --arch aarch64 --from debian:bookworm install -o ./arm-root bash

# Multiarch — runs the pipeline once per architecture into the same directory
flatroot --arch x86_64,i686 --from debian:bookworm install -o ./multi bash

# Host arch is the default (equivalent to running without --arch)
flatroot --from debian:bookworm install -o ./root bash
```

??? note "Precedent"

    ```bash
    # debootstrap --arch — closest analog, same domain (rootfs package install)
    debootstrap --arch=amd64 bookworm ./root
    debootstrap --arch=arm64 bookworm ./arm-root

    # docker buildx --platform — multiarch builds, comma-separated like flatroot
    docker buildx build --platform linux/amd64,linux/arm64 .

    # cargo / rustc --target — cross-compile target triple
    cargo build --target aarch64-unknown-linux-gnu
    rustc --target x86_64-unknown-linux-gnu main.rs
    ```

### Network Attempts

Networks between the user and distribution mirrors are inherently unreliable: connection resets, stale mirrors, transient 5xx responses. FlatRoot retries failed HTTP requests before surfacing an error. The selected flag for this was `--http-retries <N>`, on the global namespace; it defines internal data (retry budget) shared by every subcommand that performs HTTP, per the rule of global scope for cross-cutting options. The `http-` prefix makes the scope explicit: this budget governs HTTP requests only, not extraction or post-install steps. (A checksum mismatch triggers a single automatic re-download of that package, independent of this retry budget.)

```bash
# Default — the first try plus 3 retries (up to 4 attempts per request)
flatroot --from debian:bookworm install -o ./root bash

# Increase for flaky networks or slow mirrors (first try plus 10 retries)
flatroot --http-retries 10 --from debian:bookworm install -o ./root bash

# Fail on first error (strict CI mode) — exactly one attempt, no retries
flatroot --http-retries 0 --from debian:bookworm install -o ./root bash
```

??? note "Precedent"

    ```bash
    # curl --retry — the canonical HTTP retry flag
    curl --retry 3 https://example.com/pkg.tar.gz

    # wget --tries — same concept, older tradition
    wget --tries=5 https://example.com/pkg.tar.gz

    # skopeo copy --retry-times — retry semantics for OCI registry pulls
    skopeo copy --retry-times 3 docker://registry/image oci:./image
    ```

### Diagnostic Output

Many subcommands emit per-item probe failures that do not affect the user-visible result — a release codename whose mirror serves a corrupt `Packages.gz`, an expected 404 during discovery, a single package whose optional metadata field is missing. Surfacing every such event by default floods stderr with noise in the common case; suppressing them entirely hides real anomalies from anyone trying to diagnose unexpected output. The selected flag for this was `--verbose` / `-v`, on the global namespace; it defines internal data (the verbose bit) consulted by any subcommand that distinguishes "trustworthy result, one thing skipped" from "result materially skewed, user must know". The short form mirrors the `-v` convention shared across `curl`, `ssh`, `rsync`, `wget`, and most GNU coreutils.

```bash
# Default — quiet, only the structured release list on stdout
flatroot --from debian release list

# Verbose — diagnostic warnings surface on stderr
flatroot -v --from debian release list
```

??? note "Precedent"

    ```bash
    # curl -v — the canonical verbose flag
    curl -v https://example.com/pkg.tar.gz

    # ssh -v / -vv / -vvv — verbose levels
    ssh -v user@host

    # rsync -v — quiet by default, verbose per-file on -v
    rsync -v ./src/ ./dest/

    # wget -v — same convention
    wget -v https://example.com/pkg.tar.gz
    ```

## Output

The output side of the CLI is parameterized by stream (stdout vs. stderr), format (plain vs. JSON), and command category (mutating vs. reading). The principles below derive the per-command shapes documented under [Per-subcommand design](#installing-packages).

??? note "Output language"

    **Abstract syntax**

    ```
    Tree   ::= Object | Array | Leaf
    Object ::= { (Key, Tree)* }       (* keys unique within an Object *)
    Array  ::= ( Tree )*
    Leaf   ::= UTF-8, no '\n'
    Key    ::= UTF-8, no '.', no '='
    ```

    Traversal: pre-order DFS. Object members in lexicographic key order; Array members in index order.

    **Plain encoding (default)**

    ```
    plain     ::= leaf*
    leaf      ::= path '=' value '\n'
    path      ::= component ( '.' component )*
    component ::= Key | Index             (* Key: see Abstract syntax *)
    Index     ::= '0' | [1-9] [0-9]*
    value     ::= UTF-8, no '\n'
    ```

    **JSON encoding (`--format=json`)**

    [RFC 8259](https://datatracker.ietf.org/doc/html/rfc8259), with:

    - 2-space indent (`serde_json::to_string_pretty`).
    - Object members in lexicographic key order; Array members in index order.
    - Every Leaf rendered as a JSON string.
    - Trailing `\n`.

    **Empty Tree**

    Each reading command sets a per-command scope equal to its full command path (e.g. `analyze.trace.`, `search.`, `remote.list.`) on the plain encoding so consumers can identify the origin of every line. The JSON form omits the scope — it carries the same data tree without the print-time prefix.

    | `--format` | Output       |
    |------------|--------------|
    | plain      | (zero bytes) |
    | json       | `[]\n` (root is an Array for every reading command) |

    **Example**

    `flatroot --from debian:bookworm analyze trace bash` emits one entry per package, carrying keys `depends_on`, `name`, `reason`, `sonames_consumed`, `sonames_provided`, `unresolved_sonames`, and `version` — in lex order at every Object level. The `sonames_consumed` and `unresolved_sonames` arrays carry `(binary, provider, soname)` and `(binary, soname)` records respectively. Two-entry sample:

    ```text
    analyze.trace.0.depends_on.0=gawk
    analyze.trace.0.name=base-files
    analyze.trace.0.reason=declared
    analyze.trace.0.version=12.4+deb12u13
    analyze.trace.1.depends_on.0=base-files
    analyze.trace.1.depends_on.1=libc6
    analyze.trace.1.name=bash
    analyze.trace.1.reason=target
    analyze.trace.1.sonames_consumed.0.binary=usr/bin/bash
    analyze.trace.1.sonames_consumed.0.provider=libc6
    analyze.trace.1.sonames_consumed.0.soname=libc.so.6
    analyze.trace.1.version=5.2.15-2+b10
    ```

    ```json
    [
      {
        "depends_on": ["gawk"],
        "name": "base-files",
        "reason": "declared",
        "sonames_consumed": [],
        "sonames_provided": [],
        "unresolved_sonames": [],
        "version": "12.4+deb12u13"
      },
      {
        "depends_on": ["base-files", "libc6"],
        "name": "bash",
        "reason": "target",
        "sonames_consumed": [
          { "binary": "usr/bin/bash", "provider": "libc6", "soname": "libc.so.6" }
        ],
        "sonames_provided": [],
        "unresolved_sonames": [],
        "version": "5.2.15-2+b10"
      }
    ]
    ```

    **Implementation**

    The plain encoder lives at [`src/internal/printer_plain.rs`]({{ FLATROOT_REPO_URL }}/blob/master/src/internal/printer_plain.rs) as `PrinterPlain`. Commands push leaves, arrays, and nested objects through its imperative API (`push_string`, `push_array`, `push_object`, `push_array_objects`, `array`); the printer enforces lex ordering at every Object level and validates spec invariants (no `.`/`=` in keys, no `\n` in leaves) at render time. No command writes dotted lines by hand — every plain line goes through the shared encoder so the spec is enforced in exactly one place.

??? note "Streams"

    1. **stdout for data, stderr for diagnostics.** Errors, warnings, prompts, progress indicators, and command framing all go to stderr; a piped consumer of stdout never sees diagnostic noise.

        > "Send output to stdout. Send messaging to stderr. Log messages, errors, and so on should all be sent to stderr."
        > — [clig.dev, "Output"](https://clig.dev)

    2. **A command's stdout shape is its data contract.** Once published, treat it as an API — additions are backward-compatible, reorderings and deletions are not.

        > "Generally this means additional information is OK, but modifying existing output is problematic."
        > — [Heroku CLI Style Guide](https://devcenter.heroku.com/articles/cli-style-guide)

??? note "Output format"

    1. **Human-readable plain text by default; JSON opt-in.**

        > "Human-readable output is paramount. Humans come first, machines second."
        > — [clig.dev, "Output"](https://clig.dev)

    2. **The plain format is dotted `KEY=VALUE`.** One record per line, hierarchical paths separated by `.`, a single `=` separating key from value. Bijective with JSON for the structured reports; round-trips losslessly. (The `query` command is the exception: its plain form drops SQL `NULL` cells and empty arrays, which JSON preserves as `null`/`[]`, so query's two encodings are not bijective.) Models the convention used by `sysctl -a`, `git config --list`, `/etc/os-release`, and Java `.properties` files.

    3. **Composes with bare POSIX tooling without external parsers.** `grep '^trace\.'` filters a section, `cut -d= -f2-` extracts a value, line orientation lets `sort -u` and `xargs` work on individual leaves.

    4. **No tables.** Borderless ASCII tables fail the line-orientation contract — column padding shifts when content widens, multi-row entries break `grep`/`cut` alignment, and nested data has no place to live. The plain encoding replaces every table flatroot would otherwise emit.

??? note "Command output contracts"

    Output behavior is fixed per command category, independent of which command applies it.

    1. **Mutating commands are silent on stdout.** `install` and `export` produce side effects (file extraction, archive writing) and emit no stdout; stderr carries progress indicators and status lines.

    2. **Reading commands emit data on stdout.** `search`, `query`, `remote list`, `release list`, and `analyze trace` produce dotted `KEY=VALUE` by default; respect `--format`.

    3. **Empty results emit no lines.** A `search` whose pattern matches nothing produces 0 bytes on stdout. Exit code conveys success; absence of output conveys emptiness.

??? note "Format flag"

    1. **`--format` is family-level — documented once, applied to every reading command.**

    2. **Values: `plain` (default), `json`.** Both are part of the structured-output contract and are bijective for the structured reports — the same data in two syntaxes. (`query` is the exception, where the plain form drops SQL `NULL` cells that JSON keeps.)

    3. **Mutating commands reject `--format` for the encoding role.** A silent command has no encoding to choose. `export --format=<archive-type>` uses `--format` for a different role — selecting the output archive container (`oci`, `tar`, `dwarfs`, `sqfs`) — and is not the structured-output flag.

## Installing Packages

```
flatroot install -o <PATH> [OPTIONS] <PATTERNS>...
```

The primary function of FlatRoot is to build rootfs filesystems by resolving and extracting packages from a selected distribution into a target directory. The selected subcommand for this was `install`. Its subjects are the packages to install — named directly, or through a library or installed-path pattern under `--type` — so the pattern list is variadic and positional, matching the universal package-manager convention. The target directory is where the installation lands, so it is named by `-o <PATH>` / `--output <PATH>`.

```bash
# Explicit target
flatroot --from debian:bookworm install -o ./rootfs bash

# Target from env (common in CI / container builds)
export FLATROOT_ARG_INSTALL_OUTPUT=/build/rootfs
flatroot --from debian:bookworm install bash python3
```

??? note "Precedent"

    ```bash
    # rpm --root — install packages into an alternate root via flag; variadic positional
    rpm --root=/target -i pkg1.rpm pkg2.rpm

    # dpkg --root — same pattern as rpm
    dpkg --root=/target -i pkg1.deb pkg2.deb

    # apt / dnf / pacman / apk — the variadic-positional package list is universal
    apt install bash python3 git
    dnf install bash python3 git
    pacman -S bash python python-pip
    apk add bash python3 git

    # cc -o — structural analog: flag output, variadic inputs
    cc -o prog a.c b.c c.c
    ```

**Choosing one package when a file matches many (`--match`)**

The flags `--type path` and `--type library` let you ask for a package by a file it installs or a library it provides, instead of by name. The catch is that one file usually belongs to many packages. On Debian, ten different mail servers all ship `usr/sbin/sendmail`, so asking for that path alone installs all ten:

```bash
$ flatroot --from debian:bookworm install -o ./root --type path usr/sbin/sendmail
Resolved 'usr/sbin/sendmail' -> courier-mta dma exim4-daemon-light … postfix ssmtp   # all 10 installed
```

`--match all` narrows that down. List a second file that only the package you actually want ships, and `--match all` keeps just the package that has **both**:

```bash
$ flatroot --from debian:bookworm install -o ./root --type path --match all usr/sbin/sendmail usr/sbin/postfix
Resolved 'usr/sbin/postfix' -> postfix     # only postfix ships both → installs postfix alone
```

The default, `--match any`, keeps every match; `--match all` is the opt-in for narrowing. The flag works the same way on all three commands that take `--type` — `install`, `search`, and `analyze trace` — so you can check which package you land on with `search` before installing:

```bash
$ flatroot --from debian:bookworm search --type path --match all usr/sbin/sendmail usr/sbin/postfix
search.0.name=postfix
```

`--match all` is rejected with `--type package`: a package name already names exactly one package, so there is nothing to narrow.

## Exporting a Rootfs

```
flatroot export [--format FMT] [--tag TAG] <SRC_DIR> <OUTPUT>
```

A second FlatRoot function is to package a built rootfs as a portable artifact — a tar archive, an OCI image, or a SquashFS / DwarFS filesystem — for use in other runtimes. The selected subcommand for this was `export`. It has exactly one source (the rootfs directory) and one destination (the output file), so both are positional, following the `cp src dest` shape. The output format is a qualifier of the destination rather than the destination itself, so it is named by `--format <FMT>`; `--format` is inferred when the file extension is unique (`.sqfs`, `.squashfs`, `.dwarfs`, `.tar.gz`) and required when ambiguous (`.tar`: the `tar` shape is always gzip-compressed and OCI wraps its layout in an outer uncompressed tar, so a bare `.tar` name cannot say which of the two is meant).

```bash
# Extension uniquely identifies the format
flatroot export ./rootfs out.tar.gz
flatroot export ./rootfs out.sqfs
flatroot export ./rootfs out.dwarfs

# Ambiguous .tar — disambiguate with --format
flatroot export --format tar ./rootfs out.tar
flatroot export --format oci --tag myapp:v1.0 ./rootfs image.tar
```

??? note "Precedent"

    ```bash
    # cp / mv / rsync — canonical positional src-then-dest shape
    cp -a ./rootfs ./copy
    rsync -a ./rootfs/ ./copy/
    mv ./rootfs ./archive

    # mksquashfs — rootfs-to-archive analog, same positional shape
    mksquashfs ./rootfs out.sqfs

    # skopeo copy — src-then-dest for OCI image transfers
    skopeo copy oci:./local/image docker://registry/image
    ```

## Searching Packages

```
flatroot search [--type TYPE] [--format FMT] <PATTERNS>...
```

Before installing, users often need to discover which packages the selected distribution actually exposes and under what name. The selected subcommand for this was `search`. Its subjects are the name patterns, so they are variadic positional arguments — one or more, unioned and deduplicated — following the universal package-manager convention. `--type` selects whether the patterns match package names, library files, or installed paths; `--format` picks the output encoding. Glob wildcards (`*`, `?`, `[abc]`) are accepted so one invocation can match a family of names.

```bash
# Exact name
flatroot --from debian:bookworm search firefox-esr

# Glob wildcards — quote to prevent shell expansion
flatroot --from debian:bookworm search 'firefox*'
flatroot --from debian:bookworm search '*gtk*'
flatroot --from debian:bookworm search 'python3.??'
```

??? note "Precedent"

    ```bash
    # apt / dnf / pacman / apk / zypper — positional pattern across every package manager
    apt search firefox
    dnf search firefox
    pacman -Ss firefox
    apk search firefox
    zypper search firefox

    # find / grep — shell-level positional pattern search, the wider Unix convention
    find . -name 'firefox*'
    grep -rl 'firefox' .
    ```

## Querying the Package Index

```
flatroot query [FILE]
```

FlatRoot's package index is a SQLite database, and users occasionally need programmatic access for custom analyses — finding every package with a given dependency, counting packages by section, comparing two distributions. The selected subcommand for this was `query`. It reads SQL from a positional file argument, or from stdin when the argument is omitted or given as `-` (per the bare-`-` stdin convention, matching `cat` / `wc`).

```bash
# SQL from a file
flatroot --from debian:bookworm query report.sql

# SQL from stdin (implicit — bare invocation)
echo "SELECT count(*) FROM packages" | flatroot --from debian:bookworm query

# SQL from stdin (explicit -)
flatroot --from debian:bookworm query - <<'SQL'
SELECT name FROM packages WHERE essential = 1;
SQL
```

??? note "Precedent"

    ```bash
    # cat — positional file, bare stdin, explicit -
    cat report.txt
    cat                      # implicit stdin
    cat -                    # explicit stdin

    # wc — same pattern
    wc access.log
    wc < access.log          # implicit stdin
    wc -                     # explicit stdin

    # od — optional positional file, stdin default
    od binary.bin
    od                       # implicit stdin
    od -                     # explicit stdin
    ```

## Listing Supported Distributions

```
flatroot remote list
```

Before selecting a distribution with `--from`, users need to know which remotes FlatRoot supports. The selected subcommand for this was `remote list`, under a `remote` noun group. The grouping follows the rule of noun-verb grouping for extensible domains, leaving room for additional verbs — e.g. inspecting or adding a specific remote — without renaming the existing verb. No `--from` needed; the command reports what can go *into* `--from`.

```bash
# List all supported distribution backends
flatroot remote list

# Extract just the remote names for downstream use
flatroot remote list | grep '\.name=' | cut -d= -f2
```

??? note "Precedent"

    ```bash
    # git remote — the archetypal noun with a verb family
    git remote                     # list (implicit)
    git remote add origin https://example.com/repo.git
    git remote show origin

    # docker image — noun group with a verb family
    docker image ls
    docker image pull ubuntu
    docker image rm myimage

    # gh repo — noun-verb for GitHub repositories
    gh repo list
    gh repo view
    gh repo clone
    ```

## Listing Available Releases

```
flatroot --from <DISTRO> release list
```

Once a user has picked a distribution, they often need to know which specific releases (versions) that distribution actually serves. The selected subcommand for this was `release list`, under a `release` noun group. The grouping follows the rule of noun-verb grouping for extensible domains, leaving room for additional verbs — e.g. showing details for a specific release — without renaming the existing one. Only the distro prefix is needed in `--from` (e.g. `debian`, not `debian:bookworm`), since the whole point is discovering what goes after the colon.

```bash
# List available Debian releases
flatroot --from debian release list

# Pipe through grep to find a specific one
flatroot --from debian release list | grep bookworm
```

??? note "Precedent"

    ```bash
    # gh release list — direct functional analog, same noun-verb shape
    gh release list --repo owner/repo

    # npm view <pkg> versions — enumerate available versions of a package
    npm view react versions

    # git tag — enumerate release tags in a repository
    git tag
    ```

--8<-- "_glossary.md"
