Type: implementation
Status: resolved
Blocked by: 03

# Ticket 04 — GitHub Pages site + deploy workflow

Add a root `index.html` under `pages/` (alongside the two scripts from ticket 03) that redirects
to `https://github.com/thaitype/typdoc` — no server-side redirect available on Pages, so a
meta-refresh or small JS redirect. `pages/` holds exactly these three files, nothing else.

Add `pages.yml`: deploy via `actions/deploy-pages`, triggered on push to **both** `main` and
`story-5-prebuilt-installer` (the story branch — temporary, so the full install flow can be
proven before the PR merges; removing this branch from the trigger is an After Merge item, not
this ticket's job), path-filtered to `pages/` either way (mirrors `publish-check.yml`'s existing
path-filter pattern). This is a production deploy per the brief — it goes through a normal PR
with CI like everything else, no live-URL check inside this ticket yet (that's ticket 05).

Pages is already configured on this repo (source: GitHub Actions, custom domain
`typdoc.thaitype.dev`, HTTPS enforced, `github-pages` environment's allowed deploy branches
already includes the story branch) — this ticket only needs to add the workflow and site files,
not touch repo settings.

Demoable on its own: pushing this ticket's commit deploys successfully to
`https://typdoc.thaitype.dev/`, and the root page redirects correctly there — the real custom
domain, not a placeholder default URL, since Pages is already live for this branch.
