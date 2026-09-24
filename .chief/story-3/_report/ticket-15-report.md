# Ticket 15 Report

## Ticket

Release mechanics: every crate to `0.2.0`, a changelog, README installing from the tag.

## Outcome

done, with one follow-up flagged

## Notes

All four crates (`typdoc-core`, `typdoc-fs`, `typdoc`, `typdoc-testkit`) at `0.2.0`, including
the cross-crate path-dependency version pins; `Cargo.lock` regenerated to match (was still
`0.1.0` everywhere, would have failed a strict check). `CHANGELOG.md` (new) and README's install
section (points at the `v0.2.0` tag, which doesn't exist yet — expected, matches the ticket).

**Follow-up needed before this story closes:** the changelog entry was written from the goal and
contract, not from code visible in this ticket's worktree — at build time only tickets 9 and 14
had landed there. Its command-output descriptions should get one more read once every other
text-output/design.rs ticket has actually merged, to confirm nothing changed differently than
what the changelog says. Also caught and fixed in review: a stale "Version 0.1.0" line in
README's Status section, sitting right next to the new tag-based install instructions.

Non-blocking suggestion from review, not acted on: version numbers are duplicated across 8 sites
(4 crate versions + 4 cross-crate pins) with no `workspace.package.version` centralizing them —
worth a follow-up ticket for a future release, not this one.

Gates (fmt, clippy, `scripts/test.sh`) verified clean twice: once by the build, and again by me
after rebasing onto the current story-branch tip (tickets 9, 14, 16 had landed in between — no
conflicts, disjoint files; 56 test-suite blocks, 0 failed). Fast-forward merged into
`story-3-catalog-and-release`.
