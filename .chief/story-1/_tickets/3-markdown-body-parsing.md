# 3: Can a Markdown parser deliver what body-link checking needs, with line and column?

Type: wayfinder:research
Status: resolved
Blocked by: None (can start immediately)

## Question

`validate` reports `path:line:col`, treats links inside fenced code and inline code as not-links, checks `[text](path#heading)` anchors, and `toc` needs heading line ranges counted from the top of the file, frontmatter included.

Find, from primary sources, whether `pulldown-cmark` (already a dependency of the sibling `ship` crate) gives byte offsets sufficient to derive line and column for links and headings, how it reports fenced and inline code, whether reference-style links and autolinks need special handling, and how it copes with a file whose frontmatter must be skipped. Also establish whether it offers heading-slug generation or whether slugs must be written by hand. Recommend build-on-a-parser or hand-rolled scanning, with evidence.

## Answer

Full findings, prototypes and sources: [research 3](../_research/3-markdown-body-parsing.md).

**Recommendation: build on `pulldown-cmark` 0.13.4 with `default-features = false`**, and hand-write only three small things: the frontmatter cut, the byte-offset to line/column mapping, and a GitHub-style heading slugger with a test that mirrors GitHub's own output.

- **Offsets suffice.** `into_offset_iter()` yields byte ranges that map to exact line and column (checked on Thai text, CRLF, and fenced, tilde, nested-fence, indented and inline code). Code exclusion comes free from the event stream.
- **Cut the frontmatter first, then parse the body slice and add the offset back.** Ranges are byte-for-byte identical to parsing the whole file. Keep `Options::ENABLE_YAML_STYLE_METADATA_BLOCKS` off: the researcher found it recognises a `---...---` pair anywhere in the file, and a mid-file pair silently swallowed a link. (Independently confirmed only that the option exists in 0.13.4, `src/lib.rs:691`; the swallowing behaviour is the researcher's observation.)
- **No slug generation in pulldown-cmark.** A hand-written slugger matched GitHub's `/markdown` API on 29 headings, Thai and duplicates included. Two first-draft bugs (image alt text, soft breaks becoming spaces) were found only by that comparison, so the mirror test is part of the recommendation.
- **`toc` section ends** must be computed from the heading list (next heading of the same or higher level, or EOF); the parser gives only heading offsets.
- **Rejected: hand-rolled scanning.** It means reimplementing CommonMark's fence, code-span, HTML-block, container and reference-definition rules; the main risk is silent wrong answers on `~~~` fences and indented code. **Runner-up: `comrak` 0.55.0** beats hand-rolling and has an `Anchorizer` that matched GitHub on 20 of 20 headings, but costs 16 crates against 4, and its columns are byte-based.

**Decisions this surfaced, now tickets:** which link forms `body.links` checks, including `[t](my file.md)`, which CommonMark does not treat as a link ([10](10-link-forms-checked.md)); and what unit `col` is in ([11](11-column-unit.md)). Ticket 4 (slug rules) is unblocked.

## Not verified

Which column convention editors use; `markdown-rs` positions and frontmatter handling; empty-slug headings on GitHub (odd, unexplained); lone-`\r` line endings; performance.

