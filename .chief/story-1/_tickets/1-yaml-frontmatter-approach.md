# 1: Which YAML approach reads frontmatter and edits it in place without reformatting?

Type: wayfinder:research
Status: resolved
Blocked by: None (can start immediately)

## Question

The design requires that values are ordinary strings, numbers and lists; that writes preserve key order and existing YAML style where possible; that unknown fields survive a write; and that the body is never re-serialized.

Find, from primary sources (crate docs, repositories, the YAML spec):

1. Which Rust options exist for parsing YAML frontmatter, and which of them (if any) can edit a document in place while preserving order, style and comments. If none can, is line-based surgical editing of the frontmatter block a sound fallback, and what does it cost?
2. How each option types `2026-09-19` and `2026-09-19T14:30:00+07:00`, and whether bare scalars such as `WF-3`, `no` or `1e3` are coerced (YAML 1.1 versus 1.2 core schema).
3. Maintenance state of each option, including whether `serde_yaml` is archived (believed so; verify, do not assume).

Deliver a recommendation with evidence.

## Answer

Full findings, probes and sources: [research 1](../_research/1-yaml-frontmatter-approach.md).

**Approach: split reading from writing.**

- **Read** with `yaml_serde` (the YAML-org fork of `serde_yaml`; 0.10.7, 2026-08-18) into **typed `String` fields**, then interpret types from the schema rather than from YAML. Reading a value untyped changes its text (`1e3` becomes 1000.0, `1.10` becomes 1.1); typed `String` fields keep `1e3`, `1.10`, `no`, `0755`, `2026-09-19` and `2026-09-19T14:30:00+07:00` exactly. No surveyed crate types dates, so date and datetime parsing is typdoc's own job, driven by the schema's `date` and `datetime` types.
- **Write** with `yaml-edit`, pinned to an exact 0.3.x, restricted to three operations: set a scalar, append or remove a list item, add a key. It is the only crate that preserved comments, quote style and flow style in the probes, with a byte-identical no-op round trip. Any parse-then-reserialise write (every serde-family crate) drops comments, quotes and flow style.
- **Mandatory guard behind every write:** reparse the result and compare it with the intended change before renaming the temp file into place. Known corruption to guard against: replacing a block scalar fuses two lines (`note: newb: 2`); the guard catches it and the schema can avoid it.
- **`serde_yaml` is archived**, confirmed on crates.io (latest `0.9.34+deprecated`, last updated 2024-03-25).
- **Runner-up: a hand-written line-based editor** plus the same guard. Adds no dependencies, but a column-0 key scan is unsound (multi-line quoted and flow values can put `b: y` at column 0), so it needs a real state machine and its own quoting rules.

**Risk to weigh:** `yaml-edit` 0.3.2 was published on 2026-09-17, two days before this research (crates.io). It is young and single-maintainer; the exact-version pin and the reparse guard are what make that acceptable, and the guard is the reason the runner-up remains a credible fallback.

## Not verified

No YAML 1.1 parser was run; `serde_yaml_ng` and `serde_norway` were not run; the cause of `yamlpatch`'s flow-list replace failure is unknown; `saphyr` 0.1.0 span bugs were observed but not checked upstream; `yaml-edit` was not tested on anchors or tags.

**Decided 2026-09-19:** the approach above stands, with these additions:
- `typdoc-core` calls the editor through its own trait of three operations (set a scalar, append or remove a list item, add a key), so a hand-written editor can replace `yaml-edit` later.
- If the re-read does not match the intended change, the write is rejected with an error. No automatic fallback.
- Round-trip tests use real documents (tickets and memory-shaped files) and check that every line not touched is byte-identical; a test checks that `no`, `yes`, `on` and `off` stay strings, as the schema says.
- Defaults: the re-read uses the reader (`yaml_serde`), not the writer's own parser, so the two crates must agree; fixtures are committed to the public repo only after review (this repo's own tickets are already public; memory files only after review, otherwise synthetic files with the same shape), and a test never skips silently when a file is missing.
- Still untested from the research (anchors and tags in YAML, the `yamlpatch` flow-list failure): to be probed in the prototype for the contract.
