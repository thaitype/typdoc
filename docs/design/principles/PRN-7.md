---
title: An upgrade never fails a passing project at the default levels
status: active
---

**A project that passes `validate` before an upgrade still passes after it, at the default
levels.** Where that cannot hold, the changelog says exactly which projects are affected.

## Why

A version bump that breaks projects nobody changed teaches users not to upgrade. `version: 1` in
`config.json` is a promise that the meaning of a project's files does not move under it.

## What follows

- A new check is a configurable rule, and its default level is lenient enough for projects that
  already exist: an out-of-date slug is `warn`, not `error`.
- A new setting defaults to the reading that changes least: a collection without a `slug`
  setting accepts file names with and without one, and `new` without `--slug` names files as
  before.
- A change that cannot keep the promise in every case lists the cases in the changelog as an
  upgrade note.

## Where it stops

A project that already fails is not protected: making a failing file valid is allowed.
