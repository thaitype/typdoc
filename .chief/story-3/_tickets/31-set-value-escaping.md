# 31: M-19 — `--set` / `field=value` escaping per design §Query

Type: implementation
Status: claimed
Blocked by: None (can start immediately)

**Mild's decision (M-19), 2026-09-24, relayed by Aria, found running a real release build.**
Predates story 3 — the design already states the right escaping grammar (§Query) — this is a bug
fix, not a new decision.

**Repro (today's wrong behavior):**
- `set X field=a\*b` stores the value **with the backslash still in it** (`a\*b`), instead of the
  literal `a*b` the escape asks for.
- `set X field=x*y` is **accepted** and stores `x*y` literally, instead of being refused: an
  unescaped `*` in a `set` value is an error (`set` has no wildcard matching — that concept
  belongs to `--where`/`--if` only).

**The rule (design §Query, as applied to `--set`/`field=value`):**
- `\*` → a literal `*`.
- a bare, unescaped `*` → an error.
- `\,` → a literal `,`.
- `\\` → a literal `\`.
- `\` followed by anything else → an error.
- an unescaped `,` still splits a **list**/`ref[]` field's value into items (unchanged, existing
  behavior) — it has no special meaning for a scalar field's value (nothing to split into), so an
  unescaped `,` in a scalar's value is just a literal comma, not an error and not a split point.

## Where this lives today (the bug)

`crates/typdoc-core/src/query.rs`'s `parse_value`/`finish_item` (search `fn parse_value`, around
line 418) already implement almost exactly this grammar correctly, for `--where`/`--if` values —
read it first, it is your reference for the escape-scanning mechanics (`\`, `,`, `*` handling,
error messages, the "cannot end with `\`" case). **It is not directly reusable as-is**: for
`--where`, a bare unescaped `*` is a **wildcard split point** (`Item::Glob`), which `--set` must
instead treat as an outright error — `--set` has no wildcard concept at all. Don't call into
`parse_value` for `--set`; write a new, smaller function with the same character-scanning shape
but the different bare-`*` policy.

`crates/typdoc/src/cli.rs`'s `parse_set_op` (search `fn parse_set_op`, around line 568) currently
stores the raw value completely unprocessed: `raw: value.to_owned()`. **Important constraint**:
at this point (CLI argument parsing), the field's schema — and therefore whether an unescaped `,`
should split into list items — is not yet known (that's resolved later, deep in
`crates/typdoc-core/src/project.rs`, once the target document and its collection are found).
`crates/typdoc-core/src/project.rs`'s `apply_ops` (search `fn apply_ops`, around line 4870) is
where the list-vs-scalar decision already happens (`schema.field(field)`'s `kind`); today it
splits a list's raw value on a literal `,` with zero escape awareness
(`raw.split(',').map(str::to_owned).collect()`), and stores a scalar's raw value completely as-is
(`writer.set_scalar(field, raw.clone())`) — neither path processes `\*`/`\,`/`\\`/other-`\`/bare-
`*` at all.

## The fix

Don't try to do the splitting decision at CLI-parse time (the schema isn't known there yet).
Two reasonable shapes — pick whichever is cleanest once you've read both files:
- Validate escape syntax eagerly in `parse_set_op` (catches a malformed escape or a bare `*`
  immediately, before any lock is taken, matching how `--where`/`--if` already fail fast), but
  defer the actual unescape-and-maybe-split to `apply_ops` once the field's list-vs-scalar-ness
  is known; **or**
- Do the whole thing (validate + unescape + conditional split) in one new function called from
  `apply_ops`, since that's the only place that already knows both the raw value and the field's
  type.

Either way, `apply_ops` needs to become fallible (`Result<(), Error>` or similar — check what
`Error` variant is the right fit; `Error::BadArgument` is what `parse_set_op`/`parse_ifs` already
use for a caller-supplied value that doesn't parse, which this is), and **every** call site needs
updating to propagate the error: search for `apply_ops(` across `crates/typdoc-core/src/*.rs` —
there are (at least) three: `set_collected`, `set_loose`, and the shared `new`-candidate path
(`validate_new_candidate` or wherever the third call site actually is — confirm by searching, the
line numbers above are from before this ticket's own changes and may have shifted).

The escape-scanning logic itself (walk the raw value char by char; `\,`/`\*`/`\\` become a
literal character; any other `\x` is an error; a bare `*` is an error; end-of-string right after
a lone `\` is an error) is the same shape `query.rs`'s existing scanner already has — write it as
its own small, well-named function (in `project.rs`, next to `apply_ops`, or wherever fits best)
rather than inlining it, since it needs to run once per list item (after splitting on unescaped
commas, for a list field) or once on the whole value (for a scalar field, no splitting at all).

## Tests

1. **Red first** (Mild's instruction): write the two repro cases above as tests against the
   CURRENT code, confirm both fail (backslash kept; bare `*` accepted) before fixing anything.
2. After the fix:
   - `set X field=a\*b` stores the literal value `a*b` (read the file back and check the actual
     stored text, not just the exit code).
   - `set X field=x*y` (unescaped `*`) is refused — check the actual exit code (2, matching every
     other bad-argument-shaped rejection this project uses — confirm which exit code
     `Error::BadArgument` maps to elsewhere before asserting a number) and that nothing was
     written.
   - `set X field=a\,b` (scalar field) stores the literal `a,b` (comma escaped, no splitting since
     scalar).
   - `set X listfield=a\,b,c` (list field) stores two items: `a,b` and `c` (the first comma is
     escaped so it's literal within the first item; the second, unescaped, splits).
   - `set X field=a\\b` stores the literal `a\b` (one backslash).
   - `set X field=a\qb` (an unrecognized escape) is refused, clear error message.
   - `set X field=a\` (a value ending in a lone backslash) is refused, clear error message.
   - The same cases via `new`'s `--set` (not just `set`), since `apply_ops` is shared.
3. Confirm every existing test that sets a field value (search broadly — this touches a
   fundamental write path used throughout the existing test suite) still passes: a value with no
   backslash or asterisk in it must behave exactly as before.
4. `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `TMPDIR=/home/thw-home/.cache/typdoc-tmp scripts/test.sh`.

## Done

- `\*`, `\,`, `\\` in a `--set`/`field=value` produce their literal characters; a bare `*` is
  refused; any other `\x` is refused.
- An unescaped `,` still splits a list/`ref[]` field's value into items, and has no special
  meaning for a scalar field.
- Every existing write-path test still passes.
- All three gates green.
