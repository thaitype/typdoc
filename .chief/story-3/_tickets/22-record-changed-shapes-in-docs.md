# 22: Record every changed shape in `docs/design/spec/` and the user docs

Type: implementation
Status: open
Blocked by: 10, 16, 17, 18, 19, 20, 21

Contract decision 5 and M-2b, built. `docs/design/design.md` is archived and frozen (ticket 13);
this ticket is where the current, correct description of what this story changed actually lives.

## The work

For every shape this story changed from what `design.md` said (or added where `design.md` said
nothing): `new` (both forms), `set`, `get`, `toc`, both forms of `mv` (including the new
`rewritten`/`unrewritten` reporting and `mv --json`'s `rewritten` field), and `list`'s header
row —

1. Add or update the relevant entry in `docs/design/spec/` (the prose collection, ticket 10),
   describing the current text-output shape the way `design.md` used to describe it, so a person
   reading `docs/design/spec/` learns what's actually true today. **Exception: `toc --depth`
   filtered to zero headings is not described either way (silent-empty vs. some other signal) —
   ticket 17 flagged a real ambiguity there, routed to Mild as M-14, open. Write the rest of
   `toc`'s shape normally; leave this one case unstated until M-14 lands rather than asserting a
   behavior that might change.**
2. Update every place the user docs (`docs/commands.md`, `docs/getting-started.md`, `README.md`)
   already show one of these commands' output, so no shown example is stale. Where a user doc
   doesn't yet show a command's output at all, this ticket doesn't invent new coverage beyond
   what's needed to not contradict the change.
3. Add the `number`-comparison doc line (M-2b): states, in the user docs, where exact `number`
   comparison ends, with the `62e6335` demonstration (`typdoc list --where
   'count>99999999999999999998'` returns nothing when a document holds
   `99999999999999999999`). No claim about `date`/`datetime` unless a separate ticket measured
   it — none exists in this story as chartered.
4. Run every shown example for real against a built `v0.2.0` binary before committing it — an
   example that doesn't match what the binary actually does is worse than no example.

## Done

- `docs/design/spec/` describes every changed shape correctly.
- Every worked example in `docs/commands.md`, `docs/getting-started.md`, and `README.md` that
  touches a changed command matches what the binary actually does, verified by running it.
- The `number`-comparison doc line exists, with the demonstrated example.
