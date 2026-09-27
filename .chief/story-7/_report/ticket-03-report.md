# Ticket 03 Report

## Ticket
`new --slug`: the slug names the file after the key; every refusal is exit 1 before the lock and
before a number is spent.

## Outcome
done (3733027)

## Notes
- Verified by running: fmt, clippy (workspace), `scripts/test.sh` 1110 passed, `typdoc validate`,
  public-text check (no hit in a changed file). A planted fault that disabled the `required`
  refusal turned `no_slug_under_required_is_refused_with_no_number_spent` and
  `a_refused_slug_is_reported_without_waiting_for_the_lock` red; restored from saved bytes.
- Refusal order: an empty slug, then the character rule, then `none`, then `required`; an invalid
  slug under `none` reports the character rule. Either way exit 1 with no number spent.
- "Before the lock" is pinned by a test that holds the namespace lock and runs with
  `--lock-timeout 0`: a refusal must be exit 1, not exit 4.
- `--slug` with a path target is refused in the CLI, since a path target has no slug to carry.
- `mv --renumber` still passes no slug to `allocate_key`; ticket 05 changes it.
- Golden case `fixtures/output/new/coded-slug/` written by hand: `key` is `WF-2`, `path` is
  `tickets/WF-2-lock-order.md`.
