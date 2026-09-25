Type: implementation
Status: resolved
Blocked by: 04

# Ticket 05 — live-URL check (post-deploy step runs for real; the rest is code)

Pages now deploys on push to the story branch too (ticket 04), and the custom domain
(`typdoc.thaitype.dev`) is already live and HTTPS-enforced — so the **post-deploy step in
`pages.yml` is not code-only anymore**. It runs for real, on this branch, as soon as this ticket's
commit deploys, and it is this ticket's actual acceptance check:

- A step **after deploy** in `pages.yml`: fetch `https://typdoc.thaitype.dev/install` and
  `/install.ps1`, confirm both serve and are byte-identical to `pages/install` and
  `pages/install.ps1` in the repo, and confirm the root URL redirects. Blocks that `pages.yml`
  run on failure — for real, in this ticket, not just in theory. This is the ticket's actual
  demonstration: don't consider it done until this step has gone green on a real deploy.

The other two checks stay code written now, executed later — there's no release yet to check
against, and won't be until the full-flow proof step (after the loop) and, later, the real
0.3.1 release:

- A step **after the release** in `publish.yml`: run the live install one-liner and the `.ps1`
  equivalent on ubuntu, macOS and Windows runners, then `typdoc --version`; blocks that
  `publish.yml` run on failure once a real (or pre-)release is dispatched.
- A scheduled workflow (e.g. weekly) repeating the same live check independently of any deploy or
  release, to catch drift between releases.

None of these three are PR gates (see contract — they test already-deployed state, not a PR's
code); the post-deploy one is a push-triggered gate on `pages.yml` itself, which is exactly what
lets it run for real on this branch.

Demoable on its own: the post-deploy step in `pages.yml` runs and passes against the real
`typdoc.thaitype.dev` domain on this branch's own deploy. The `publish.yml` and scheduled-workflow
steps are demoable only as valid, reviewable YAML at this point — their first real execution
comes later (full-flow proof, then the real release).
