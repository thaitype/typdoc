Type: implementation
Status: open
Blocked by: 03

# Ticket 04 — GitHub Pages site + deploy workflow

Add a root `index.html` under `pages/` (alongside the two scripts from ticket 03) that redirects
to `https://github.com/thaitype/typdoc` — no server-side redirect available on Pages, so a
meta-refresh or small JS redirect. `pages/` holds exactly these three files, nothing else.

Add `pages.yml`: deploy via `actions/deploy-pages`, triggered on push to `main`, path-filtered to
`pages/` (mirrors `publish-check.yml`'s existing path-filter pattern). This is a production
deploy per the brief — it goes through a normal PR with CI like everything else, no live-URL
check inside this ticket yet (that's ticket 05, since it depends on the custom domain being
attached).

Demoable on its own: the workflow deploys successfully to the default Pages URL (before the
custom domain is attached), and the root page redirects correctly there.
