# 4: CI — which gates, which triggers, environment

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

## Question

Story 3 must ship CI (a release blocker for v0.2.0, carried from stories 1-2 — "held in place by
a test" today means "if someone runs the gate"). Which of the existing local gates run in CI, on
what triggers, and how does the machine-specific environment story 2 hit get handled without
leaking into the workflow?

Local gates today: `scripts/test.sh` (needs `TMPDIR` pointed off a full `/tmp` on this
particular machine), `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `scripts/check-public-text.sh` (needs `PUBLIC_TEXT_TERMS_FILE` pointed at a
crew-local file of names to catch, absent which it prints "names are NOT checked" and still runs
its other patterns).

## Answer

**Settled 2026-09-23.** `scripts/check-public-text.sh` and its project rule were removed
deliberately, by the repo owner's own commit `340db02` ("drop a local check script and the notes
that pointed at it", 2026-09-21) — not by accident, and not restored. There are **three** local
gates, not four: `scripts/test.sh`, `cargo fmt --check`, `cargo clippy --workspace --all-targets
-- -D warnings`.

All three run in CI, on both push and pull-request into `main`. No machine-specific path anywhere
in the workflow or the scripts it calls: the workflow sets `TMPDIR` from the runner's own temp
directory, not the `/home/thw-home/.cache/typdoc-tmp` path story 2 used on this specific machine.

Acceptance for the CI ticket itself: on a throwaway branch, push one commit that breaks each gate
in turn (a failing test, a fmt diff, a clippy warning) and see CI fail on each — then delete the
branch. A workflow that is green because a step silently did nothing looks identical to one that
works, so green only counts once each gate has been shown red first.

Whether the runner's own `/tmp` has the disk-pressure problem this machine has is not checked
ahead of time — the first real CI run answers it.
