# Autonomous Agent Task: Portable Static GLIBC Binary Toolchain / Environment

## Mission

Your mission is to **research, prototype, implement, patch, vendor, and validate a practical toolchain for producing portable statically built Linux binaries using GLIBC rather than MUSL**.

The ultimate goal is to determine whether we can provide a single practical tool that allows a user to take an arbitrary existing project and build a binary against GLIBC in such a way that the resulting binary can run across a very broad range of Linux environments, including both:

* GLIBC-based distributions;
* MUSL-based distributions.

The desired result is a binary with **little to no runtime/container/application overhead**, without inventing a new application packaging format unless absolutely necessary.

The emphasis is on **practicality, compatibility, reproducibility, and evidence**.

This should not become an academic exploration of libc internals for its own sake.

At the same time, do not hand-wave around the extremely difficult parts of GLIBC portability.

The project must establish, experimentally and with evidence:

> What is actually required to make a GLIBC-built binary run reliably on both GLIBC and MUSL systems, and can those requirements be automated into one reusable tool?

---

# Primary Goals

The project has three major goals.

## 1. Research

Research the **best existing tools, scripts, wrappers, libraries, techniques, build environments, compatibility layers, and implementation strategies** for compiling static Linux binaries against **GLIBC rather than MUSL**, while avoiding the usual drawbacks and portability problems.

The research must investigate both:

* existing approaches that already solve portions of this problem;
* and combinations/extensions of existing approaches that could solve the complete problem.

Do not assume that traditional "static linking with glibc" is sufficient.

Investigate the actual runtime behavior of GLIBC and the mechanisms that can cause an apparently static binary to still depend on:

* host configuration;
* host NSS modules;
* dynamic loading;
* gconv modules;
* locale data;
* resolver behavior;
* system databases;
* kernel interfaces;
* architecture-specific behavior;
* ABI/version requirements;
* other runtime resources.

The research should identify which dependencies are:

1. compile-time;
2. link-time;
3. load-time;
4. runtime;
5. environment-dependent;
6. host-dependent;
7. optional;
8. application-triggered;
9. difficult/impossible to discover statically.

Do not merely list known problems.

Determine how they can be **solved, contained, detected, or automatically handled**.

---

# 2. Implementation

Implement/patch/vendor whatever is required to move toward the following concrete goal:

## One tool

Create a **single practical tool** — implemented in shell, Python, Rust, or another appropriate language — that can create an environment in which a user can clone essentially any compatible Linux project and build a portable static GLIBC binary.

The tool may create or manage one or more of:

* a local directory;
* a local build environment;
* a local shell;
* a local chroot;
* a Docker image;
* a Podman image;
* another lightweight isolated environment if technically justified.

The exact mechanism is an implementation detail.

The important part is the resulting workflow.

A user should be able to conceptually do something like:

```text
run the tool
→ enter/use the generated environment
→ clone an arbitrary project
→ build it
→ obtain a binary
→ copy that binary to another Linux distribution
→ execute it
```

The desired binary should work on both:

* GLIBC systems;
* MUSL systems.

The goal is **little to no runtime overhead**.

The project should aim to make the resulting binary behave like a normal Linux executable rather than requiring a large runtime framework.

---

# "Pure" Binary Requirement

The solution should be as **pure** as technically possible.

Prefer a solution where:

* the application needs little or no modification;
* the application does not need to know that the portability machinery exists;
* the application can be built normally;
* the resulting binary can be copied and executed directly.

If application-specific patching/wrapping is required, investigate whether that can be automated or generalized.

The ideal hierarchy is:

1. **No application changes**
2. Automatic build/linker/toolchain changes
3. Generic wrapper/runtime technique
4. Automatic application patching
5. Application-specific patches
6. New packaging/runtime format

Prefer the earlier options.

Only move toward a more invasive solution when the evidence demonstrates that a simpler approach cannot work.

---

# Do Not Invent a New Binary / Packaging Format Unless Necessary

Do **not** invent a new application format merely to solve the problem.

In particular, avoid creating something analogous to:

* AppImage;
* onelf;
* a custom executable container;
* a proprietary launcher format;

unless research demonstrates that there is no sufficiently practical way to achieve the goal using a normal Linux executable.

The desired outcome is a **normal ELF/Linux executable whenever possible**.

The user explicitly does not care whether tools such as:

```text
file
ldd
readelf
```

claim that the resulting binary is technically static.

Those tools are **not the ultimate definition of success**.

The actual requirement is:

> **The resulting binary must work correctly on both GLIBC and MUSL Linux environments.**

A binary that is technically "dynamic" according to `file`, `ldd`, or `readelf`, but is fully self-contained and reliably works across GLIBC and MUSL, may be more useful than a binary that is technically static but fails at runtime.

Conversely, a binary that is technically static but still:

* reads host configuration;
* loads incompatible host modules;
* requires host GLIBC components;
* fails on MUSL;
* crashes;
* fails DNS;
* fails locale operations;
* fails NSS;
* fails gconv;

does **not** satisfy the actual goal.

Focus on **behavioral portability**, not labels emitted by inspection tools.

---

# Core Technical Question

The central question of the project is:

> Can we build a normal Linux ELF binary using GLIBC such that it can run reliably across both GLIBC and MUSL systems without requiring a separate application packaging format or significant runtime overhead?

Do not assume the answer is yes.

Do not assume the answer is no.

**Prove what is possible.**

If the strongest possible solution still has unavoidable constraints, clearly document those constraints.

If the problem is only solvable for a particular class of applications, precisely define that class.

If some applications fundamentally require host-specific functionality, demonstrate that with evidence.

---

# GLIBC Runtime Problems Must Be Treated as First-Class Problems

A major part of the research must focus on GLIBC's behavior beyond ordinary ELF linking.

In particular, investigate how a nominally static GLIBC-linked application can still interact with host resources.

Important areas include, but are not limited to:

* NSS;
* `/etc/nsswitch.conf`;
* `libnss_*`;
* dynamic module loading;
* `dlopen`;
* resolver functionality;
* DNS;
* `/etc/hosts`;
* `/etc/resolv.conf`;
* passwd/group lookups;
* user/group databases;
* gconv;
* iconv;
* locale;
* timezone data;
* configuration files;
* system databases;
* kernel interfaces;
* syscall ABI;
* CPU/architecture requirements;
* GLIBC symbol/version requirements;
* TLS;
* thread-local storage;
* IFUNC;
* hwcaps;
* tunables;
* environment variables;
* loader/runtime behavior;
* audit modules;
* preload mechanisms;
* plugins;
* libraries loaded indirectly by libc;
* other filesystem/runtime dependencies.

Do not assume this list is exhaustive.

Research the actual GLIBC implementation and relevant upstream documentation/source code.

---

# NSS

One explicitly identified problem is that GLIBC may read the host's:

```text
/etc/nsswitch.conf
```

and dynamically load the NSS modules specified there.

This is extremely important.

Investigate the mechanisms involved and how a portable binary can prevent host NSS configuration/modules from causing:

* symbol collisions;
* ABI incompatibility;
* unexpected behavior;
* dynamic dependencies;
* crashes;
* host-specific behavior.

The project should investigate whether the mechanisms in:

https://github.com/pkgforge-dev/cross-libc-dlopen

already solve this problem.

Do not merely cite the repository.

Inspect the implementation, especially:

```text
./src
```

and determine:

* exactly what problem it solves;
* how it solves it;
* what assumptions it makes;
* what cases it covers;
* what cases it does not cover;
* whether it can be reused directly;
* whether it needs patching;
* whether it should be vendored;
* whether it should be generalized;
* whether its approach can be integrated into the proposed tool.

The goal is to ensure that host NSS modules cannot unexpectedly collide with the application's own symbols or otherwise break portability.

---

# Gconv

Another explicitly identified major GLIBC portability problem is **gconv**.

GLIBC's character conversion system can dynamically depend on gconv modules.

A major practical problem is:

> There may be no straightforward way to know ahead of time exactly which gconv modules an application will need.

If a required module is missing at runtime, the application may fail in difficult-to-diagnose ways.

Investigate:

* how GLIBC discovers gconv modules;
* how applications trigger gconv;
* how gconv modules are selected;
* how dependencies can be detected;
* whether dependencies can be traced dynamically;
* whether they can be traced statically;
* whether the required modules can be automatically collected;
* whether gconv can be redirected to an application-controlled location;
* whether a minimal self-contained gconv environment can be generated;
* whether modules can be embedded;
* whether module loading can be wrapped/intercepted;
* whether unused modules can be safely omitted;
* what happens when the host has different gconv configuration;
* how this behaves on MUSL systems.

The research should specifically investigate whether the project can **solve or substantially automate the gconv problem**.

Do not simply document that gconv is difficult.

Attempt to engineer a practical solution.

---

# Existing Relevant Project: cross-libc-dlopen

Study:

https://github.com/pkgforge-dev/cross-libc-dlopen

This is likely one of the most relevant pieces of prior art.

Pay particular attention to:

```text
./src
```

Investigate whether it already provides mechanisms that can be incorporated into a generic portable-GLIBC build environment.

Document:

* architecture;
* implementation;
* assumptions;
* supported use cases;
* limitations;
* integration method;
* build process;
* runtime behavior;
* compatibility implications.

If the implementation is useful, prefer **reusing or appropriately vendoring/patching it** over independently reinventing the same mechanism.

---

# Existing Relevant Project: Anylinux-AppImages

Study:

https://github.com/pkgforge-dev/Anylinux-AppImages

Its tooling contains scripts/tools that are often forked or patched versions of other projects.

Do not assume every component is relevant.

Investigate the repository and determine which tools are useful for this project.

In particular, investigate:

https://github.com/pkgforge-dev/Anylinux-AppImages/raw/refs/heads/main/useful-tools/lib/anylinux.c

This is believed to be especially relevant.

Inspect its implementation carefully.

Determine:

* what problem it solves;
* how it works;
* whether it can be reused;
* whether it needs modifications;
* whether its techniques can be generalized into the new tool;
* whether it handles runtime dependencies that ordinary static linking does not;
* whether it provides useful portability mechanisms.

---

# Existing Relevant Project: Anylinux-sharun

Study:

https://github.com/pkgforge-dev/Anylinux-sharun

Investigate its techniques for:

* dependency discovery;
* runtime packaging;
* library handling;
* ELF analysis;
* dynamic dependency resolution;
* portability;
* execution across Linux environments.

Determine which parts of its architecture or implementation could be adapted without necessarily adopting a new application packaging format.

The objective is to extract useful techniques while preserving the goal of producing a normal executable where possible.

---

# Existing Relevant Project: userland-execve-rust

Study:

https://github.com/pkgforge-dev/userland-execve-rust

Investigate whether its mechanisms can help with:

* executing binaries;
* controlling runtime environment;
* ELF execution;
* interpreter behavior;
* compatibility;
* namespace/environment isolation;
* userland emulation of execution mechanisms.

Determine whether any portion should be:

* reused;
* vendored;
* patched;
* generalized;
* or merely used as research/reference material.

---

# Existing Relevant Project: ppkg & related tooling

Study:

https://github.com/leleliu008/ppkg/tree/master/core/wrappers
https://github.com/leleliu008/elftool
https://github.com/leleliu008/patches


Investigate whether their mechanisms can help with:

* executing binaries;
* controlling runtime environment;
* ELF execution;
* interpreter behavior;
* compatibility;
* namespace/environment isolation;
* userland emulation of execution mechanisms.
* anything else useful

Determine whether any portion should be:

* reused;
* vendored;
* patched;
* generalized;
* or merely used as research/reference material.

---

# Existing Relevant Project: onelf

Study:

https://github.com/QaidVoid/onelf

Investigate its architecture and approach to producing portable/self-contained executable behavior.

However, do not automatically adopt the onelf model.

The project explicitly prefers **not inventing a new executable format**.

Use onelf primarily as evidence/prior art for understanding:

* what problems arise;
* which problems require a launcher/runtime;
* which mechanisms can be achieved using standard ELF;
* what tradeoffs are introduced.

If onelf provides a technique that can be implemented without adopting its entire format, investigate that possibility.

---

# Prior-Art Research Must Be Broader Than the Listed Projects

The listed repositories are mandatory starting points, not the complete research scope.

Research other relevant projects and techniques involving:

* static GLIBC;
* portable GLIBC;
* static-pie GLIBC;
* glibc-bundling;
* libc compatibility;
* cross-libc execution;
* ELF wrapping;
* ELF loaders;
* custom dynamic loaders;
* NSS isolation;
* gconv isolation;
* locale bundling;
* runtime dependency discovery;
* ELF dependency analysis;
* `dlopen` interception;
* syscall compatibility;
* containerized builds;
* chroot-based build environments;
* reproducible Linux toolchains;
* cross compilation;
* portable binaries;
* static linking;
* hermetic builds;
* hermetic runtime environments.

Research upstream implementations, not merely blog posts.

Where possible, inspect source code.

The objective is to discover existing mechanisms that can be combined into a practical solution.

Some more:
 - https://github.com/a2flo/standalone_musl
 - https://github.com/altipla-consulting/distroless-glibc
 - ./tmp/static-glibc-nss-dynamic-loading.md (in this repo)
 - https://gamesbymason.com/blog/2025/statically-linking-pipewire/ (fetch using webarchive if your web fetch doesn't, implementation here: https://github.com/allyourcodebase/pipewire/raw/refs/heads/main/src/wrap/dlfcn.zig , also see https://github.com/allyourcodebase/pipewire/issues/12)
 - https://news.ycombinator.com/item?id=21580998 (quoted verbatim)
  > JoshTriplett on Nov 20, 2019 | parent | context | favorite | on: Clang Format Tanks Performance
        The primary reason is that a statically linked glibc can't use NSS (Name Service Switch) modules from a different glibc version, so if you statically link glibc you can't reliably resolve names with getaddrinfo. You can do DNS with other libraries, and many applications do anyway because they want asynchronous DNS and NSS uses static DNS, but you can't really do anything other than DNS or static hosts files.
        musl, on the other hand, never supports NSS, whether static or dynamic, and exclusively supports DNS. You can't resolve things like mDNS names, or even resolve localhost if on a system that doesn't hardcode it in /etc/hosts.
        So either way, static linking and name resolution don't mix well, and musl and non-DNS name resolution don't mix well.

---

# Languages / Applications for Proof of Concept

The project must provide **at least five major FOSS projects** as reproducible proof-of-concept examples.

These projects should be deliberately chosen from **challenging languages/ecosystems that do not normally make static compilation easy**.

Do **not** simply choose:

* Rust;
* Go;
* C programs that trivially support static linking;
* projects specifically designed around static builds.

Rust and Go are explicitly **out of scope as primary challenge cases**, unless a particular project genuinely requires GLIBC functionality that creates an interesting portability problem.

They may be used as controls where useful.

The five primary projects should instead stress different problematic areas.

Examples of potentially useful categories include projects involving:

* C++;
* C with significant libc interaction;
* Python/native embedding;
* Java/native components where relevant;
* Perl/native components;
* Ruby/native components;
* Lua/native components;
* complex autotools/CMake projects;
* projects using `dlopen`;
* projects using NSS;
* projects using iconv/gconv;
* projects using locale functionality;
* projects using plugins;
* projects with complicated dependency trees.

Do not blindly use these examples.

Research and select **major, real-world FOSS projects** that provide meaningful stress tests.

The final five should be justified based on the specific portability problems they exercise.

---

# Proof-of-Concept Requirements

For every selected project:

1. identify the upstream project/version/commit;
2. document its normal build process;
3. document why static GLIBC compilation is difficult;
4. build it using the proposed system;
5. produce the resulting binary;
6. inspect the resulting binary;
7. test it on GLIBC;
8. test it on MUSL;
9. exercise meaningful functionality;
10. record failures;
11. record runtime dependencies;
12. preserve build commands;
13. provide a reproducible script/example;
14. document any required patch;
15. document whether the patch is generic or project-specific.

The POCs must be **real reproducible examples**, not hypothetical instructions.

---

# Compatibility Testing

A successful proof-of-concept should be tested across multiple Linux environments.

At minimum, investigate testing on representative:

### GLIBC systems

Examples may include:

* Debian;
* Ubuntu;
* Fedora;
* Arch Linux;

or other appropriate GLIBC distributions.

### MUSL systems

At minimum:

* Alpine Linux.

Additional environments may be used where they provide meaningful evidence.

The exact matrix should be determined based on engineering practicality.

Do not imply that testing on one GLIBC and one MUSL distribution proves universal Linux compatibility.

Instead, explicitly define what was tested and what remains untested.

---

# Runtime Compatibility Is the Actual Success Criterion

The project must prioritize actual execution over static-analysis labels.

For every resulting binary, investigate:

* does it start?
* does it execute correctly?
* does it resolve DNS?
* does it access files correctly?
* does it perform user/group operations?
* does locale functionality work?
* does iconv/gconv work?
* does NSS work?
* does threading work?
* does dynamic loading work where required?
* does application-specific functionality work?
* does it crash?
* does it depend on host libraries?
* does it depend on host configuration?
* does it depend on host modules?

The binary is successful if it behaves correctly in the target environment.

---

# "Works Everywhere" Must Be Defined Precisely

The phrase "works everywhere" must not be treated as an excuse for an unverifiable claim.

Define compatibility in terms of concrete test environments.

For example:

* GLIBC distribution A;
* GLIBC distribution B;
* GLIBC distribution C;
* MUSL distribution A;
* x86_64;
* aarch64.

Then expand the matrix where practical.

The project should clearly distinguish:

* verified compatibility;
* likely compatibility;
* untested compatibility;
* known incompatibility.

Never claim universal Linux compatibility solely from theoretical reasoning.

---

# Architecture

At minimum, investigate:

* x86_64;
* aarch64.

Where the proposed approach has architecture-specific behavior, document it.

If a component only works on one architecture, do not silently generalize its behavior.

---

# Build Environment

The proposed tool should be able to create a controlled environment that minimizes host contamination.

Investigate:

* Docker;
* Podman;
* chroot;
* local sysroot;
* dedicated toolchain;
* hermetic build roots;
* containerized toolchains.

The exact implementation can support one mechanism initially and add others later.

The priority is a robust and reproducible implementation.

---

# Host Contamination

The build process must avoid accidentally incorporating host-specific components.

Investigate contamination from:

* `/usr/include`;
* `/usr/lib`;
* `/lib`;
* compiler search paths;
* linker search paths;
* environment variables;
* pkg-config;
* CMake;
* autotools;
* Python;
* system configuration;
* architecture-specific libraries;
* compiler builtins;
* startup files;
* GLIBC headers;
* GLIBC libraries;
* NSS modules;
* gconv modules.

The tool should make its build environment explicit and inspectable.

---

# Build Reproducibility

The project must record enough information to reproduce a build.

Record, where relevant:

* source repository;
* source commit/tag;
* compiler;
* compiler version;
* linker;
* linker version;
* GLIBC version;
* kernel/environment;
* architecture;
* build flags;
* linker flags;
* sysroot;
* toolchain;
* patches;
* vendored dependencies;
* container image;
* container digest;
* environment configuration.

Do not rely on an undocumented developer workstation.

---

# Tool Design

The resulting tool should be:

* portable;
* scriptable;
* inspectable;
* deterministic where practical;
* easy to debug;
* easy to invoke from CI;
* easy to invoke locally;
* capable of producing useful diagnostics.

Consider Rust if it gives significantly better:

* process management;
* filesystem control;
* subprocess handling;
* environment management;
* error handling;
* deterministic behavior;
* portability.

Shell or Python are also acceptable if they provide sufficient portability and maintainability.

Do not choose a language for ideological reasons.

Choose based on the engineering requirements.

---

# The Tool Should Not Become a Black Box

The user should be able to understand:

* what environment was created;
* what compiler is being used;
* what GLIBC is being used;
* what libraries are being linked;
* what patches were applied;
* what runtime components were included;
* how the final binary is produced.

Provide verbose/debug modes.

Expose commands/configuration where practical.

Avoid a magic command that does everything while hiding the mechanism.

---

# Automatic Dependency Discovery

One important research/implementation goal is determining whether runtime dependencies can be automatically discovered.

Investigate techniques including:

* static ELF analysis;
* dynamic tracing;
* `strace`;
* `LD_DEBUG` where applicable;
* `dlopen` interception;
* NSS instrumentation;
* gconv instrumentation;
* filesystem access tracing;
* syscall tracing;
* controlled runtime environments;
* application test execution.

Determine whether the tool can:

1. build the binary;
2. execute representative workloads;
3. observe runtime dependencies;
4. collect required runtime resources;
5. construct a minimal portable runtime;
6. validate the result in a clean MUSL environment.

If fully automatic discovery is impossible, identify the limitations and provide the best practical semi-automatic mechanism.

---

# gconv / NSS / Dynamic Loading Should Be Tested Explicitly

Do not merely test an application that does nothing interesting.

At least some POCs should intentionally exercise:

* NSS;
* DNS;
* user/group lookup;
* iconv;
* locale;
* gconv;
* `dlopen`;
* plugins;
* dynamically discovered dependencies.

The goal is to validate that the portability machinery works under the exact circumstances that traditionally cause failures.

---

# Patching Policy

When upstream projects require modification:

1. prefer minimal patches;
2. document why the patch is necessary;
3. determine whether the patch is generic;
4. determine whether it can be automatically applied;
5. determine whether upstream already has a pending/fixed solution;
6. preserve the patch in a clearly identifiable form;
7. record upstream version/commit;
8. document how to remove the patch when upstream integrates the fix.

Do not silently modify third-party source.

---

# Vendoring

Follow the supplied vendoring methodology.

When code from another project must be incorporated:

* identify its upstream source;
* record its exact revision;
* preserve licensing;
* document modifications;
* preserve attribution;
* make the provenance obvious;
* avoid unexplained forks;
* update vendored code deliberately.

Do not copy source files into the repository without recording where they came from and what was changed.

---

# Methodology & Discipline

The agent **must follow the methodology documents provided below** when performing research, experiments, reference gathering, and vendoring.

Use the following methodology references:

### Experiments

https://github.com/Azathothas/TEMPLATE/raw/refs/heads/main/docs/methodology/experiments.md

### References

https://github.com/Azathothas/TEMPLATE/raw/refs/heads/main/docs/methodology/references.md

### Vendoring

https://github.com/Azathothas/TEMPLATE/raw/refs/heads/main/docs/methodology/vendoring.md

These are not optional reading.

Read them before conducting substantial experiments or incorporating external source code.

Apply their requirements to:

* experiment design;
* evidence;
* reproducibility;
* citations;
* source provenance;
* vendoring;
* patches;
* benchmark/POC documentation;
* conclusions.

---

# Evidence

Every major technical claim should be supported by evidence.

Evidence may include:

* upstream source code;
* upstream documentation;
* reproducible commands;
* build logs;
* runtime logs;
* syscall traces;
* dependency traces;
* binary inspection;
* successful execution;
* failed execution;
* CI output;
* controlled experiments.

Do not claim:

> "This fixes NSS"

unless there is an experiment demonstrating the relevant NSS behavior.

Do not claim:

> "This makes GLIBC work on MUSL"

unless the resulting binary has actually been executed and verified in a MUSL environment.

---

# Negative Results Are Valuable

A failed experiment is not a wasted experiment.

Document:

* what was attempted;
* exact source/configuration;
* expected result;
* actual result;
* failure mode;
* logs/evidence;
* suspected cause;
* confirmed cause where possible;
* potential solution;
* whether it is a fundamental limitation.

A documented failure can be more valuable than an undocumented success.

---

# Compare Approaches

The research should explicitly compare candidate approaches.

For example:

| Approach                   | Portability | Runtime overhead | Application changes | Complexity | Normal ELF | NSS | gconv | MUSL |
| -------------------------- | ----------- | ---------------- | ------------------- | ---------- | ---------- | --- | ----- | ---- |
| Traditional static GLIBC   | ?           | ?                | ?                   | ?          | ?          | ?   | ?     | ?    |
| Bundled GLIBC              | ?           | ?                | ?                   | ?          | ?          | ?   | ?     | ?    |
| cross-libc-dlopen approach | ?           | ?                | ?                   | ?          | ?          | ?   | ?     | ?    |
| sharun-style approach      | ?           | ?                | ?                   | ?          | ?          | ?   | ?     | ?    |
| onelf-style approach       | ?           | ?                | ?                   | ?          | ?          | ?   | ?     | ?    |
| Proposed approach          | ?           | ?                | ?                   | ?          | ?          | ?   | ?     | ?    |

Do not fill this table with assumptions.

Populate it with evidence from research and experiments.

---

# Avoid False Equivalence

Do not assume:

* "static" means self-contained;
* "self-contained" means portable;
* "portable" means compatible with every kernel;
* "works on Alpine" means works on every MUSL environment;
* "works on Debian" means works on every GLIBC version;
* `ldd` output is authoritative;
* `file` output is authoritative;
* `readelf -d` is sufficient to determine runtime dependencies.

Use actual execution tests.

---

# GLIBC Version Strategy

Investigate how the selected GLIBC version affects compatibility.

Determine:

* whether older GLIBC versions improve compatibility;
* whether newer GLIBC versions introduce requirements;
* whether symbols/versioned symbols matter;
* whether static GLIBC still has runtime GLIBC-version assumptions;
* whether a sufficiently old GLIBC can provide broad compatibility;
* what tradeoffs this creates.

Do not assume "newest GLIBC" is best.

The objective is maximum practical compatibility.

---

# Kernel Compatibility

Investigate whether the final binary depends on kernel features that may not exist on older kernels.

Distinguish:

* libc compatibility;
* kernel compatibility;
* CPU compatibility.

If the final binary requires a newer kernel, document it.

If CPU feature dispatch/IFUNC/hwcaps affects compatibility, investigate it.

---

# CPU / Architecture Compatibility

Do not accidentally produce binaries requiring newer CPU instructions than the intended target.

Investigate compiler options such as:

* architecture baseline;
* CPU tuning;
* SIMD;
* IFUNC;
* hwcaps.

Document the chosen compatibility baseline.

---

# Performance / Overhead

The project explicitly seeks **little to no overhead**.

Measure relevant overhead where practical.

Compare:

1. ordinary native build;
2. ordinary static build;
3. proposed portable GLIBC build;
4. alternative existing approaches.

Measure things such as:

* startup time;
* steady-state runtime;
* memory usage;
* binary size;
* filesystem footprint;
* runtime dependencies;
* build time.

Do not optimize prematurely.

First establish correctness and portability.

Then measure overhead.

---

# No Toy Demonstration

The five proof-of-concept projects must be meaningful.

Do not select trivial applications solely because they compile easily.

The POCs should deliberately expose difficult areas of GLIBC/static portability.

The final result should demonstrate that the proposed system works for real software rather than a custom hello-world example.

---

# GitHub / CI

The project should be capable of running its research and validation through GitHub Actions.

Where practical, CI should:

1. build the tool;
2. build selected POCs;
3. generate portable binaries;
4. test them in GLIBC environments;
5. test them in MUSL environments;
6. collect evidence;
7. validate results;
8. publish logs/artifacts;
9. prevent regressions.

Do not make CI merely compile the tool itself.

It should exercise the actual portability objective.

---

# Local Environment

The project should ideally work with:

* Docker;
* Podman;

where practical.

If local containers are unavailable in the agent's current environment, this is **not a blocker**.

Use GitHub Actions or another available CI environment to perform the experiments.

If necessary:

1. implement;
2. commit;
3. push to `main`;
4. run CI;
5. inspect CI;
6. diagnose failures;
7. patch;
8. rerun;
9. document the outcome.

Do not stop merely because the local environment cannot execute containers.

---

# Remote Access / Proxy

If direct access to GitHub or other websites/resources is unavailable, use the user-provided proxies.

## GitHub read-only API

Use:

`https://api.gh.pkgforge.dev/<GH_API_PATH>`

for read-only GitHub API access.

## General web fetch

Use:

`https://api.rv.pkgforge.dev/<any_url>`

for other web/resource fetching when direct access fails.

This includes:

* GitHub raw files;
* documentation;
* upstream source;
* external web pages;
* references.

Do not prematurely claim that a source is inaccessible.

---

# Repository Documentation

Documentation must be comprehensive enough that another engineer can reproduce the research without relying on conversation history.

Document:

* problem statement;
* technical goals;
* definition of portability;
* supported environments;
* supported architectures;
* architecture/design;
* research findings;
* existing prior art;
* selected approach;
* rejected approaches;
* build environment;
* tool usage;
* implementation details;
* patches;
* vendored code;
* NSS handling;
* gconv handling;
* dynamic loading;
* runtime dependency discovery;
* testing;
* POCs;
* compatibility matrix;
* known limitations;
* known failures;
* performance/overhead;
* reproducibility.

---

# Critical Requirement: docs/AGENTS.md

Create and continuously maintain:

`docs/AGENTS.md`

This must be a **standalone agent handoff document**.

A future agent may read **only `docs/AGENTS.md`**, with:

* no conversation history;
* no previous agent memory;
* no assumptions;
* no additional explanation;

and must be able to understand the entire current state of the project.

It must enable a future agent to both **confirm existing work and immediately resume further work**.

At minimum, `docs/AGENTS.md` must document:

1. what the project is;
2. the exact problem being solved;
3. why the problem exists;
4. the desired end state;
5. what "portable static GLIBC" means for this project;
6. what does and does not count as success;
7. current architecture;
8. repository structure;
9. all relevant upstream projects;
10. all relevant references;
11. methodology requirements;
12. current implementation;
13. completed work;
14. current work;
15. blocked work;
16. known bugs;
17. known limitations;
18. known failed approaches;
19. successful approaches;
20. current tool usage;
21. build commands;
22. test commands;
23. CI workflow;
24. POC projects;
25. POC status;
26. GLIBC test environments;
27. MUSL test environments;
28. architecture coverage;
29. NSS findings;
30. gconv findings;
31. dynamic-loading findings;
32. dependency-discovery findings;
33. patches;
34. vendored components;
35. source provenance;
36. exact versions/commits;
37. current evidence;
38. where evidence/logs/results are stored;
39. reproducibility instructions;
40. current TODO list;
41. recommended next steps.

Explicitly categorize work as:

* **COMPLETE**
* **IN PROGRESS**
* **BLOCKED**
* **FAILED / KNOWN LIMITATION**
* **UNTESTED**
* **PLANNED**

Do not describe something as complete unless it has actually been verified.

Do not let this document become stale.

Update it whenever an important implementation or research result changes.

---

# Research Log

Maintain an appropriate research/experiment record.

For major experiments, record:

* hypothesis;
* setup;
* command;
* source versions;
* environment;
* result;
* evidence;
* conclusion.

The goal is for another agent to understand not just **what** was done, but **why certain approaches were accepted or rejected**.

This prevents future agents from repeatedly attempting already-failed approaches without understanding the historical evidence.

---

# Implementation Strategy

Do not immediately build a huge framework.

Use staged development.

A recommended progression is:

## Phase 1 — Research

Study:

* methodology;
* cross-libc-dlopen;
* Anylinux-AppImages;
* Anylinux-sharun;
* userland-execve-rust;
* onelf;
* broader prior art;
* GLIBC source/documentation.

Produce a concrete technical model of the problem.

## Phase 2 — Minimal Proof

Create the smallest possible prototype that demonstrates:

* GLIBC-linked binary;
* portability mechanism;
* execution on GLIBC;
* execution on MUSL.

## Phase 3 — NSS

Explicitly test and solve/contain:

* `/etc/nsswitch.conf`;
* NSS module loading;
* host NSS interference.

## Phase 4 — gconv

Explicitly test:

* iconv;
* gconv;
* missing gconv modules;
* host gconv configuration.

Develop an automatic or semi-automatic solution.

## Phase 5 — Runtime Discovery

Develop mechanisms to detect runtime dependencies that static analysis cannot reliably identify.

## Phase 6 — Generic Tool

Turn the working techniques into one reusable tool.

## Phase 7 — Real-World POCs

Build and test at least five major challenging FOSS projects.

## Phase 8 — Cross-Distro Validation

Test across multiple GLIBC and MUSL environments.

## Phase 9 — CI / Reproducibility

Automate the complete process in GitHub Actions.

## Phase 10 — Documentation / Hardening

Make the tool usable by someone who has never seen the development process.

This phased plan is guidance, not a reason to blindly follow a predetermined architecture.

If experiments prove that a different order is better, adapt.

---

# Do Not Hide Fundamental Limitations

If the research proves that some aspects of universal GLIBC→MUSL portability are fundamentally impossible without:

* a bundled runtime;
* a custom loader;
* an application patch;
* a wrapper;
* a new executable format;
* host cooperation;

say so clearly.

Then determine the **least invasive practical solution**.

Do not artificially claim the original goal has been achieved.

The project should produce an honest answer even if that answer is:

> "A completely normal static GLIBC ELF cannot satisfy this requirement in all cases, but X approach gets us extremely close for Y class of applications."

That would still be a valuable result.

---

# Success Criteria

The project should ultimately demonstrate, with evidence, whether it can achieve the following:

### Tooling

A single tool can create a controlled build environment.

### Build

A user can take an existing FOSS project and build it against GLIBC using that environment.

### Portability

The resulting binary can execute on both GLIBC and MUSL environments.

### Application Independence

The application requires little or no modification.

### Normal ELF

The resulting executable remains as close as possible to a conventional Linux ELF executable.

### Runtime

There is little to no additional runtime overhead.

### NSS

Host NSS configuration/modules do not unexpectedly break the binary.

### gconv

gconv requirements are handled automatically or through a practical reproducible mechanism.

### Dynamic Loading

Relevant `dlopen`/runtime module behavior is correctly handled.

### Reproducibility

Another user can reproduce the result using the documented environment.

### Real Software

At least five major, challenging FOSS projects are demonstrated.

### Evidence

Every major claim is supported by reproducible evidence.

---

# Agent Operating Principles

Throughout the project:

* Research before reinventing.
* Inspect source code when implementation details matter.
* Reuse existing solutions where practical.
* Patch rather than rewrite when a small patch solves the problem.
* Vendor only with clear provenance.
* Preserve upstream attribution/licensing.
* Never fabricate compatibility.
* Never fabricate benchmark results.
* Never claim universal portability without evidence.
* Treat runtime behavior as more important than ELF labels.
* Test both GLIBC and MUSL.
* Test real applications.
* Test the difficult runtime paths explicitly.
* Preserve failure evidence.
* Keep `docs/AGENTS.md` current.
* Keep the research reproducible.
* Prefer normal ELF over custom formats.
* Prefer no application patches over application-specific patches.
* Prefer minimal runtime overhead.
* Prefer existing upstream mechanisms over unnecessary reinvention.
* Do not let theoretical purity prevent practical engineering.
* Do not let practical convenience justify unsupported claims.
* Do not turn the project into an academic exercise.
* Do not turn the project into a toy demonstration.

Most importantly:

> **Do not optimize for producing a binary that merely looks static. Optimize for producing a normal, portable Linux executable that actually works.**

The ultimate deliverable is not merely a compiler command or a collection of patches.

It is a **reproducible, evidence-backed, practical tool and methodology for building GLIBC-based Linux binaries that can reliably execute across GLIBC and MUSL environments with minimal application changes and minimal runtime overhead.**
