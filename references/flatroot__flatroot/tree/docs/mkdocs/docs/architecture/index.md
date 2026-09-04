---
tags:
  - architecture
  - pipeline
---

# Architecture

FlatRoot is a single binary that resolves, downloads, extracts, and post-installs distro packages into an unprivileged rootfs directory. The pipeline runs as an ordered sequence of stages — backend selection, index fetch, seed assembly, dependency resolution, download, extraction, post-install scripts, and cache regeneration. Each stage returns a typed outcome consumed by the next stage or by the manifest writer.

## The pipeline

![Install pipeline](index/pipeline.drawio)

Blue boxes are core pipeline stages. Purple boxes are post-install passes. Green is the output. Grey boxes are external inputs — the distribution mirror, the seed list assembled from user packages plus base and essential packages, and the local download cache.

--8<-- "_glossary.md"
