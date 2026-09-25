# Ticket 07 Report

## Ticket
GitHub Release creation + asset upload in `publish.yml` — the gap ticket 01 found (no release on
this repo, including v0.3.0, has ever had assets attached; it was always manual).

## Outcome
done

## Decision
- **Issue:** the ticket says the release body should be "the CHANGELOG entry for that version,"
  but the existing `v0.3.0` release body on GitHub is a curated blurb (install instructions,
  "What's new" prose), not a raw changelog dump — the two don't actually match in shape.
- **Options considered:** reproduce the curated shape (needs either an LLM-authoring step, out of
  scope, or hand-authoring per release — exactly the manual step this ticket exists to remove)
  vs. use the raw CHANGELOG.md section verbatim as the body.
  - **Chosen:** raw CHANGELOG section verbatim — matches the ticket's operative instruction
    literally, keeps the job fully automated with no per-release hand-authoring. Release bodies
    will read differently (plainer) than `v0.3.0`'s from `0.3.1` onward — worth knowing if a more
    curated body is wanted later.
- No other ambiguity — the tag/release-creation idempotency (`gh release view` guard) and
  asset-upload mechanism (`gh release upload --clobber`, sourced from the same `dist-*` artifacts
  `attest` already downloads) followed directly from the ticket and ticket 01's existing job
  shapes.

## Notes
- New `release` job in `publish.yml`, `needs: attest`, gated `if: inputs.dry_run == false` — same
  shape as the existing `cargo publish` step, so a dry run builds+attests but creates nothing
  public. True by construction (the job-level `if` gate), not exercised by an actual dispatch
  (this loop never runs one for real).
- New `scripts/extract_release_notes.py` (+ its own test file, 10 tests) pulls one version's
  section out of `CHANGELOG.md` by exact `## [<version>]` heading match — the one part of this
  job that could be TDD'd in isolation, since `gh release create`/`upload` themselves need the
  real GitHub API and were checked by reading the code instead of by a test.
- Did not build the separate `v0.3.1-rc.1` pre-release workflow (explicitly out of scope for this
  ticket) but left the extraction script reusable as a plain CLI for whoever builds that.
- Commit `fac62b9` on `story-5-prebuilt-installer` (ticket-07's single commit, fast-forwarded —
  branch was still at the same tip as when it was created, no rebase needed). Public-text grep
  clean.
