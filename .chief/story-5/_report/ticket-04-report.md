# Ticket 04 Report

## Ticket
GitHub Pages site (`pages/index.html` redirect) + deploy workflow (`pages.yml`).

## Outcome
done

## Decision
- **Issue:** the contract's own Pages section says `main` only; the ticket (following the
  full-flow-proof decision relayed mid-loop) explicitly asks for both `main` and
  `story-5-prebuilt-installer`.
- **Chosen:** followed the ticket's explicit, more specific instruction over the contract's
  general line, with the override and when it gets reverted (not this ticket's job) documented
  in the workflow's own header comment. Not a judgement call — pre-resolved by the ticket brief.
- Minor, non-blocking calls: added a `concurrency` group to serialize deploys (not required by
  ticket/contract, standard practice for a production deploy workflow); used a three-layer
  redirect (meta-refresh + JS + fallback link) instead of picking just one of the contract's
  either/or options, for UA robustness.

## Notes
- `pages/` now holds exactly the three required files (`install`, `install.ps1` from ticket 03;
  `index.html` new here).
- `pages.yml` uses `actions/deploy-pages`, `environment: github-pages` (the pre-approved
  environment name from the ticket), path-filtered to `pages/**` plus the workflow file itself —
  does not touch repo Pages settings, which are already configured.
- No live-URL check in this ticket (deferred to ticket 05, as planned) — but since Pages now
  deploys on push to this branch too, ticket 05's post-deploy check will run for real as soon as
  it lands, not just after merge.
- Commit `514108c` on `story-5-prebuilt-installer`. Public-text grep clean, checked twice.
