# Ticket 19 Report

## Ticket

Text output for `refs`: the design's own worked shape (`target   field`, one per line), and the
error-path fix.

## Outcome

done

## Notes

Verified byte-for-byte against `design.md`'s own worked example (checked with `cat -A` for exact
spacing — 3 literal spaces, no header, no column padding, unlike `list`/`toc`'s tables). Built a
new fixture (`fixtures/valid/refs-worked-example/`) specifically to reproduce that example rather
than approximating it in an existing fixture. `--reverse`/`--field` needed no new logic — they
already flow into the same report the new text renderer formats.

One judgment call recorded: the ticket's error-case options were "a document that doesn't exist,
or `--field` naming a field the schema doesn't have" — the second isn't actually an error in this
codebase (`filtered()` just returns empty), so the not-found-document case was used, matching the
precedent already set for `get`/`toc`'s own error tests.

Rebased cleanly onto the current tip (tickets 9/14/15/16/17/18 in between); all three gates
re-verified green after the rebase (56 test-suite blocks, 0 failed). Fast-forward merged into
`story-3-catalog-and-release`.
