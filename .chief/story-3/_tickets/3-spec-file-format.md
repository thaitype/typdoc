# 3: Format of the structured spec file

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

## Question

`design.rs` currently parses `docs/design/design.md` to extract five sets used by tests and
production code (`typdoc-core/src/frontmatter.rs`, `typdoc-core/tests/rules.rs`,
`typdoc/tests/coverage.rs`): always-on rule ids, configurable rule ids, command names, exit
codes, and frontmatter losses. Per Mild's no-markdown-as-spec rule, this moves into a structured
file. What format?

## Answer

JSON. The workspace already depends on `serde`/`serde_json` in every crate that would read this
file (`typdoc-core`, `typdoc`, `typdoc-testkit`); no `toml` crate is a dependency anywhere in the
workspace today. typdoc's own domain files (`config.json`, `lock.json`) are JSON. The file is
hand-edited — nothing generates it; people edit it whenever a rule, command, or exit code is
added — but that happens rarely (a few times per story), so zero-new-dependency and consistency
with the existing domain files outweigh TOML's editing comfort at that frequency. Where an entry
needs prose (e.g. the frontmatter-losses descriptions), keep it short in the JSON file; the doc
around the generated table carries the fuller explanation.

Decided by Aria (2026-09-23), not Mild — recorded as her call.
