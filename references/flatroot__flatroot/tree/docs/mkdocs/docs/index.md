<p align="center">
  <img src="assets/logo.svg" width="128" height="128" alt="FlatRoot logo">
</p>

<h1 align="center">FlatRoot</h1>

<p align="center">
  Build Linux root filesystem directories from official distribution packages — without root privileges or a running package manager.
</p>

---

FlatRoot is a tool that builds Linux root filesystem directories from official distribution packages — without root privileges or a running package manager. It resolves dependencies, fetches archives from distribution mirrors, extracts them into a target directory, and runs post-install scripts inside a namespace sandbox.

The sections below organise the documentation by intent — the [Diátaxis](https://diataxis.fr/) modes (Tutorials, How-To Guides, Reference, Explanation), plus an Architecture section on FlatRoot's internal design. Each has a different purpose — pick the one that matches what you need right now.

## Tutorials

Hands-on lessons for newcomers.

- [Build your first rootfs](tutorials/first-rootfs.md) — assemble a miniature Alpine Linux in five minutes.
- [Reproducible Builds](tutorials/reproducible-builds.md) — compile a program twice against a pinned toolchain and see the bytes match.

## How-To Guides

Task-oriented recipes for practitioners.

- [Get FlatRoot](how-to/get-flatroot.md) — How to install FlatRoot on your machine.
- [Install packages](how-to/install-packages.md) — How to put packages from a distribution into a rootfs directory.
- [Query packages](how-to/query-packages.md) — How to discover what a distribution offers and look up details about any of its packages.
- [Pin a build to a snapshot date](how-to/pinning.md) — How to make a build reproducible so the same command produces the same rootfs months from now.
- [Build for multiple architectures](how-to/multiarch.md) — How to combine packages for different CPU architectures in a single rootfs.

## Reference

Tables, flags, and other reference material.

- [CLI](reference/cli.md) — Complete reference for every command and option, plus how distribution sources are named on the command line.
- [Environment](reference/environment.md) — Every environment variable FlatRoot reads on startup and the fixed environment it sets inside the install sandbox.
- [Cache](reference/cache.md) — What FlatRoot stores on disk between runs, where it lives, and how to clean it up.
- [Database](reference/database.md) — The package database FlatRoot builds for each distribution and how to query it directly.
- [Path Index](reference/path_index.md) — The lookup that answers "which package owns this file?"
- [Rootfs Metadata](reference/metadata.md) — The on-disk `.flatroot/` format: fields, checksum algorithms, and reinstallation behaviour.
- [deb](reference/package-formats/deb.md) — The index and archive layout for Debian and Ubuntu packages.
- [rpm](reference/package-formats/rpm.md) — The index and archive layout for CentOS, Fedora, AlmaLinux, Rocky, and openSUSE packages.
- [pacman](reference/package-formats/pacman.md) — The index and archive layout for Arch Linux and CachyOS packages.
- [apk](reference/package-formats/apk.md) — The index and archive layout for Alpine Linux packages.

## Explanation

Explanations of how and why FlatRoot works the way it does.

- [Overview](explanation/index.md) — A guided tour of how FlatRoot turns a list of package names into a working rootfs.
- [The distro abstraction](explanation/the-distro-abstraction.md) — How one pipeline serves ten distributions by hiding distro specifics behind a single trait.
- [The seed list](explanation/the-seed-list.md) — Why the install set starts from more than the names you typed.
- [Dependency resolution](explanation/dependency-resolution.md) — How the resolver turns a seed list into the complete, ordered install closure.
- [Virtual names and providers](explanation/virtual-names-and-providers.md) — How declared names that aren't real packages map to the package that provides them.
- [Conditional dependencies](explanation/conditional-dependencies.md) — How RPM rich deps and Alpine install-if triggers are evaluated against the install set.
- [The sandbox](explanation/the-sandbox.md) — How package post-install scripts run unprivileged, without real root or a live system.
- [Cache regeneration](explanation/cache-regeneration.md) — How FlatRoot rebuilds the runtime caches — fonts, icons, MIME types, and so on — that applications expect to find inside a rootfs.
- [The metadata layer](explanation/the-metadata-layer.md) — The bookkeeping FlatRoot writes alongside every installed rootfs and what each piece is used for.
- [Cross-distro boundaries](explanation/cross-distro-boundaries.md) — Why FlatRoot refuses to mix packages from different sources into one rootfs.
- [Multiarch](explanation/multiarch.md) — How packages for multiple CPU architectures share a single rootfs tree.

## Architecture

How FlatRoot is built internally.

- [Overview](architecture/index.md) — The ordered pipeline of stages that turns a request into a finished rootfs.
- [CLI Design](architecture/cli.md) — Why FlatRoot's commands and options look the way they do, and which CLI conventions the design follows.
- [Packages](architecture/packages.md) — How the codebase is organized into packages and what each is responsible for.
- [Spec](architecture/spec/index.md) — The behavioural specification: one page per use case, recording what the tool promises and how the promise travels through the packages.

--8<-- "_glossary.md"
