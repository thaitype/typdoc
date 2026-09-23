# 20: Text output for plain and `--schemas` `validate`

Type: implementation
Status: claimed
Blocked by: None (can start immediately)

`--audit`'s text form is already built; this ticket builds the other two, from the same finding
shape `--json`/`--audit` already expose. No design question is open here.

## The work

Plain `validate` and `validate --schemas` without `--json` print one line per finding, built
from the same finding data `--json` already carries (level, rule, message, path). Replace the
refusal at `cli.rs:394` for the non-`--audit`, non-`--json` case. `validate`'s error path already
passes its real `json` variable today (verified: `Err(e) => failure(json, exit_code(e.kind()),
&e)`) — confirm this stays true after the change; no fix expected here, but check.

## Tests

- A hand-written golden with at least one finding at each level (`info`/`warn`/`error`) that
  `--audit`'s existing golden doesn't already need, confirming a readable one-line-per-finding
  format.
- A clean project (no findings): **decided (Aria, 2026-09-23), following `list`'s own precedent —
  nothing printed, the exit code carries the result.** Not a free choice; confirm this falls out
  naturally from "one line per finding" with zero findings, rather than adding a special-cased
  "clean" line. If a future build session thinks this is wrong for `validate` specifically (e.g.
  a caller genuinely can't tell a clean run from a crash with both silent), it flags to Aria
  rather than deciding a "clean" line on its own.
- `--schemas` alone, with a schema-only problem, uses the same one-line-per-finding shape.

## Done

- Plain `validate` and `validate --schemas` produce one line per finding without `--json`; no
  "not built yet" refusal remains for either.
- `validate`'s error path is confirmed to already print plain text without `--json` (or fixed, if
  the confirmation finds otherwise).
