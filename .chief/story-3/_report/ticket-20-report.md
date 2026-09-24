# Ticket 20 Report

## Ticket

Text output for plain and `--schemas` `validate`: one line per finding, decided empty-case
behavior, error-path confirmation.

## Outcome

done

## Decision

- **Issue:** the build's own reviewer checked whether "nothing printed on a clean run" is
  genuinely distinguishable from a crash for `validate` specifically (the concern ticket 17
  raised for `toc`, extended here as a sanity check).
- **Chosen:** no change needed — the exit code (0 clean vs. 2 a real failure) already
  unambiguously separates the two cases; crash paths print their own error text on stderr with a
  non-`{0,2}` exit code regardless. Not the same shape of ambiguity `toc --depth` has (M-14),
  where two genuinely different situations both produce identical silent output *and* identical
  exit codes. Nothing routed to Aria/Mild for this one.

## Notes

`validate_text` builds one line per finding — `path:line:col  level  message  rule`, bare `path`
with no position — column-padded the same deterministic way `audit_text` already does. Reused
`Severity::Info` only appearing under `--audit` (verified via `rg`) to satisfy the golden's
"each level" requirement without inventing an impossible plain-`validate` info case.

Rebasing onto the current tip hit one import-list merge conflict in `crates/typdoc/src/cli.rs`
(ticket 20 added `Position`; tickets 17/21, merged in the meantime, had already added
`Heading`/`RefName`/`RewrittenRef` to the same block) — mechanical union, verified by clippy
(no unused/missing import) same as the earlier ticket-21 conflict of the same shape. All three
gates re-verified green after. Fast-forward merged into `story-3-catalog-and-release`.

**This closes the entire original build frontier** (9, 14, 15, 16, 17, 18, 19, 20, 21 all merged).
Remaining open work: 10 and 14's macOS follow-up (both building now); 11, 12, 13, 22 behind them
or behind M-13 (ticket 24).
