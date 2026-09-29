---
title: Agents are the main users
status: active
---

**Agents are typdoc's main users.** Every command answers with an exit code that can be branched
on and a `--json` form that can be parsed, and a name one command prints is a name the others
accept.

## Why

An agent does not read prose well under pressure and does not ask what a message meant; it
branches on the exit code and feeds output into the next call. A name that a command prints but
no command accepts is a dead end for an agent, however clear it looks to a person.

## What follows

- Exit codes have one meaning each across every command.
- `--json` names a document the way arguments accept it: by `key` for a coded document, by
  `path` otherwise.
- The typdoc skill describes what each way of doing a thing does to the files, and leaves the
  choice to the agent.

## Where it stops

A person still reads the text output, which follows `PRN-9`.
