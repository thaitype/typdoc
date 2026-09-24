# Ticket 29 Report

## Ticket

Two defects Aria found running a real release build against a fresh project: (1) `typdoc refs
--reverse` never named which document was pointing at the one asked about; (2) `list`'s header
said `key` for a result that mixed coded and uncoded documents, misdescribing rows that held a
bare path.

## Outcome

done

## What changed

**`refs` forward (`Out` direction)** — before, `written` then `field`, no way to know the
resolved name; after, `document` first (resolved name or `(unresolved: reason)`), then `field`,
then `written` (kept, since it can genuinely differ from `document` — an alias or relative form):

```
# before
written         field
chief:WF-7      context
learnings/x.md  $body

# after
document             field    written
chief:WF-7           context  chief:WF-7
team/learnings/x.md  $body    learnings/x.md
```

**`refs --reverse` (`In` direction)** — before, `written`/`field` with no holder name at all;
after, `document`/`field` only (no `written` column — for `In` it would only repeat how the
holder wrote a ref back to the very document already named on the command line):

```
# before ($ typdoc refs tickets/WF-2.md --reverse)
written             field
../tickets/WF-2.md  $body
WF-2                blocked_by
WF-2                context

# after
document      field
notes/a.md    $body
default:WF-1  blocked_by
default:WF-3  context
```

**`list` mixed result** (spans a coded collection and an uncoded one) — before, header wrongly
said `key` (from `any()`); after, three-way split gives `document`:

```
# before ($ typdoc list, valid/refs fixture: tickets is coded, notes isn't)
key         title
WF-1        Ticket one
WF-2        Ticket two
WF-3        Ticket three
notes/a.md  A note

# after
document    title
WF-1        Ticket one
WF-2        Ticket two
WF-3        Ticket three
notes/a.md  A note
```

Homogeneous cases are untouched: an all-coded result still says `key`, an all-uncoded result
still says `path`.

Code: factored `ref_outcome_text` out of `unrewritten_text` (`crates/typdoc/src/cli.rs`) so
`refs_text` reuses the exact same resolved/unresolved rendering rather than a second copy of the
match arm. `list_table`'s identity-label `if`/`else` became `all()`/`any()`/`else` (three cases,
not two).

## Verification

- `cargo fmt --check` — clean.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh` — `1001 test(s) passed across 56
  suite(s)`, 0 failed.
- Ran `/chief-review-code` (two parallel throwaway sub-agents, Standards + Spec axes) against
  `git diff fc98eab`:
  - **Standards**: no documented `.chief/_rules/_standard/**` or `CONTRIBUTING.md` in this repo,
    so no hard violations possible; smell baseline found nothing beyond one judgement call (a
    small, acceptable duplication of the `if forward` guard in `refs_text`'s header vs. row
    construction) it explicitly said wasn't worth fixing. It also flagged that my first pass at
    `docs/commands.md` had inserted the new identity-column paragraph *inside* the `list` options
    markdown table, breaking the table — fixed before commit (moved after the table, matching
    how the `refs` section places its own paragraph).
  - **Spec**: confirmed all 5 required test behaviors present (reverse golden naming the holder,
    forward 3-column golden, new mixed-`list` golden, the two pre-existing homogeneous
    `list_table` goldens untouched, both empty-case tests untouched), column names exactly
    `document`/`field`/`written`, `written` truly absent (no trace, not just empty) for `In`, the
    mixed-`list` header a genuine three-way split, and the CHANGELOG bullets corrected in place
    rather than duplicated. Its only finding was the same `docs/commands.md` table-placement bug,
    independently caught.
  - Both docs issues from the reviews were the same single bug (paragraph placement in
    `commands.md`); fixed, gates re-run green after the fix.
- Confirmed empty case: `refs tickets/WF-2.md` (no outgoing refs) and `list --where
  title=NoSuchTitle` both print nothing at all (no header) — unchanged from before this ticket.
- Confirmed the homogeneous `list` cases (`--code WF --where context=*` → `key`; `--collection
  notes` → `path`) still pass unchanged.

## Notes

- No new fixture was built for the literal "`WF-2` holds `blocked_by: WF-1`, run `refs WF-1
  --reverse`" example from the ticket's own illustration. The existing `valid/refs` fixture
  already has the mirror-image shape (`WF-1` holds `blocked_by` pointing at `WF-2`; `refs
  tickets/WF-2.md --reverse` shows `WF-1` as one of three holders, mixed with a coded and an
  uncoded document) and was updated as the golden for this — it exercises the exact same code
  path (a coded holder's resolved name shown in the `document` column) with no gap in coverage,
  and the ticket itself introduces this fixture example with "e.g." Building a second,
  narrower fixture just to match the ticket's illustration literally seemed like duplicate
  coverage rather than a real gap.
- Docs and CHANGELOG changes were kept scoped to the sections ticket 28 originally touched;
  CHANGELOG's two bullets (list header row, refs/validate header rows) were edited in place, not
  duplicated, since `[0.2.0]` is still Unreleased.
