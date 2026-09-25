# Ticket 05 Report

## Ticket
Live-URL check: real post-deploy step in `pages.yml`, code-only post-release step in
`publish.yml`, and a scheduled drift check.

## Outcome
done. The post-deploy step's real-run pass, unproven from an isolated worktree at build time, is
now confirmed: after pushing and opening the PR, `pages.yml`'s run on this branch shows both
`deploy to GitHub Pages` and `verify live install URLs` jobs completed with `conclusion: success`
(run `36136202000`) — the real proof this ticket needed.

## Decision
- **Issue:** the ticket's own acceptance criterion — the post-deploy step in `pages.yml` actually
  going green on a real deploy — cannot be produced from an isolated git worktree with nothing
  ever pushed to `origin`. Confirmed, not assumed: no `story-5-*` branch exists on `origin`, no
  workflow runs exist for this branch, and `https://typdoc.thaitype.dev/` currently 404s (GitHub's
  own "no Pages site here" page) — DNS resolves, but no `deploy-pages` run has ever published
  anything, consistent with nothing having been pushed yet.
- **Options considered:** treat as a blocker and escalate vs. recognize this as the expected state
  before the loop's actual finish line (push + open PR, which is exactly what triggers the real
  deploy this check needs).
- **Chosen:** not a blocker — it's the loop's own next step. The script's real HTTP path was
  verified against the live domain's current (not-yet-deployed) state (correctly fails loud, HTTP
  404, exactly as designed) and a full offline unit suite (18 tests) covers the comparison/
  redirect logic. Real green/red is deferred to the push-and-open-PR step, right after this.

## Notes
- Extracted a reusable `live-check.yml` (`workflow_call`) during code review, called from both
  `pages.yml`'s post-deploy step and the new scheduled workflow — matches this repo's existing
  `dist-build.yml` reuse pattern, avoids duplicating the check logic.
- Scheduled workflow scoped to the page-serving check only (not the release install one-liner) —
  the contract's own stated purpose (catch cert/DNS/Pages drift) is domain-specific, not
  release-specific; a defensible reading, flagged in case a broader scope was intended.
- Root-redirect check asserts on body content (the redirect target string), not an HTTP 3xx —
  GitHub Pages has no server-side redirect, so there's no 3xx to check.
- Commit `3c51a51` on `story-5-prebuilt-installer`. Public-text grep clean, checked before and
  after the review-driven refactor.

## For the PR / loop close-out
This was the last ticket in the frontier. Pushed `story-5-prebuilt-installer` to `origin`, opened
`thaitype/typdoc#7` as a draft. The push itself triggered `pages.yml` for real and confirmed the
open question above.
