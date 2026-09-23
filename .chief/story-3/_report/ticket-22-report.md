# Ticket 22 Report

## Ticket

Record every shape story 3 changed in `docs/design/spec/` and the user docs (contract decision
5, M-2b's doc line).

## Outcome

done

## Notes

Added `docs/design/spec/SPC-5.md` ("Text output explained") authored through the real
`.typdoc` project via the built binary (bumped `spec.last` 4→5, an expected side effect, not an
accident). Covers every changed shape with real-run examples, including a rich `mv` example with
non-empty `unrewritten`/`findings` from the schema-mismatch fixture, and `mv --json`'s field
order confirmed by running it. Deliberately left `migrated_from` off (correct — this is new
content, not migrated from `design.md`, unlike SPC-1..4) and deliberately left `toc --depth`
filtered-to-zero-headings undescribed (M-14, still open) — verified directly: the `--depth 1`
example shown still has results, no empty-case description anywhere in the file.

Fixed several already-stale user-doc spots, not just added new content: `docs/commands.md`'s
intro paragraph and `mv --json` examples (missing the new `rewritten` field), `new`'s bare-key
example, `docs/getting-started.md`'s bare-key line, README's `list` examples and a "not built
yet" bullet that was no longer true. Every example verified run against the real built binary,
including a README concept example that surfaced a real subtlety (the identity column duplicates
the literal `key` header when `key` is also requested via `--fields`) rather than being assumed.

Confirmed `docs/design/design.md` untouched, unmoved, and never referenced as if already
archived — ticket 13 hasn't run in this worktree, and the build correctly treated it as still at
its original path throughout.

All three gates re-verified green after merging (1022 tests, 56 suites, unaffected as expected
for a docs-only ticket). Fast-forward merged into `story-3-catalog-and-release`.

**Remaining open work:** tickets 12 and 13, both blocked on M-13 (ticket 24's HOW), still open
with Mild. The CI red-before-green proof (both OSes) is separately blocked on a GitHub token
permission issue, reported to Aria/Mild.
