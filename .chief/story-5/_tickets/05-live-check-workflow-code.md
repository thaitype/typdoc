Type: implementation
Status: open
Blocked by: 04

# Ticket 05 — live-URL check workflow code (not execution)

Nothing is actually live while this work sits on a story branch — Pages deploys on push to
`main`, and the real release is an owner action after merge (see the story's "After Merge" list).
This ticket writes the **code** for the checks, not their execution against real URLs:

- A step **after deploy** in `pages.yml`: fetch both live script URLs and confirm they serve;
  blocks that `pages.yml` run on failure once it actually runs on `main`.
- A step **after the release** in `publish.yml`: run the live install one-liner and the `.ps1`
  equivalent on ubuntu, macOS and Windows runners, then `typdoc --version`; blocks that
  `publish.yml` run on failure once a real release is dispatched.
- A scheduled workflow (e.g. weekly) repeating the same live check independently of any deploy or
  release, to catch drift between releases.

None of these are PR gates (see contract — they test already-deployed state, not a PR's code).

Demoable on its own: the workflow YAML is valid and the steps are reviewable; actual green/red
against the live domain only happens after merge, once the Pages setting is flipped (requested
separately, per the story's "After Merge" list) — that verification is explicitly out of this
ticket's scope.
