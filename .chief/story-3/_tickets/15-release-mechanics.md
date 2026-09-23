# 15: Release mechanics — 0.2.0, changelog, install from tag

Type: implementation
Status: claimed
Blocked by: 14

Blocked by CI rather than by anything computational: a release is claimed only once the gates
that would catch a broken one are actually running.

## The work

1. Every crate's `version` in its `Cargo.toml` (`typdoc-core`, `typdoc-fs`, `typdoc`,
   `typdoc-testkit`) raised from `0.1.0` to `0.2.0`.
2. A changelog (new, or an existing one extended if one is added earlier in this story) naming
   what `v0.2.0` changes relative to `v0.1.0`, at a level a user of the tool would read — the
   catalog/spec document split is an internal detail unless it changes something a user sees;
   what a user sees is: every command works without `--json`, `list` gains a header row, `mv`
   reports what it rewrote, a documented `number`-comparison limit, and the stack-overflow fix.
3. README's install instructions point at the `v0.2.0` tag (once it exists) rather than at
   cloning `main`.

## Done

- Every crate reports `0.2.0`.
- A changelog exists and names this release's user-visible changes.
- README installs from the tag, not from a clone of `main`.
