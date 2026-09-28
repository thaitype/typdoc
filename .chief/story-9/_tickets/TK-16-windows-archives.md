---
blocked_by: []
status: open
title: Windows archives in the release build
type: implementation
---

# TK-16: Windows archives in the release build

## What this delivers

The release build makes `typdoc-x86_64-pc-windows-msvc.zip` and its `.sha256`, in the naming the other four
targets use, runs the binary on a Windows runner, and attests it. `aarch64-pc-windows-msvc` is added only if it
builds and runs on a Windows arm64 runner.

## Scope

- `dist-workspace.toml`: the Windows target(s), `.zip`.
- `dist-build.yml`: a `windows-latest` leg (and `windows-11-arm` for aarch64, on evidence) that runs
  `typdoc.exe --version`.
- `publish.yml`: attestation covers `typdoc-*.zip` as well as `typdoc-*.tar.gz`; job names that say "4 targets".

## Checks

The dist build on the pull request makes the Windows archive and runs the binary; nothing is released.
