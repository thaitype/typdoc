# Out of Scope

- **Remote schemas, entirely.** No fetch adapter, no project lock, no `lock.json`/`vendor/`
  writes, no `pull` or `pull --check`. A remote schema with no pin keeps reporting
  `config.schema-unpinned`.
- **Moving every piece of content out of `docs/migrating-design/`.** This story removes the
  code's dependency on markdown; it does not require the working copy to end the story empty.
- **The invalid `body.links` `ignore` glob becoming a config error.** It already fails strict
  today (the link is still checked and reported), so nothing unsafe ships by leaving this for a
  later story.
- **Exact comparison for numbers past what `f64` holds**, a `validate` finding for them, or a
  `--sort` tie-break for them. This story only documents where exactness ends.
- **`typdoc validate`/`get` learning a catalog document's declared body type.** The helper this
  story adds stays internal, used only by the coverage tests; a body-type-aware CLI is designed
  later, together with support for a plain `.json` document (as opposed to markdown with a
  JSON-only body).
- **`[reverse-scope]` and `[import-anchor]`**, known gaps carried unresolved from earlier stories.
- **Body-link scan cost, prebuilt binaries, and a crates.io release.** Named as explicitly
  deferrable; none of them block a `v0.2.0` a user can install from a git tag.
