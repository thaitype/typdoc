# Research 3: Markdown body parsing for typdoc (line/col, code exclusion, links, slugs)

Date: 2026-09-19. Feeds `docs/design-decision-phase-1/_tickets/3-markdown-body-parsing.md` and blocks ticket 4 (slug rules). Nothing in the repo other than this file was touched.

## Recommendation (short)

Build on `pulldown-cmark` 0.13.4 (`default-features = false`, no features needed; already used by the sibling `ship` crate). Three rules make it safe:

1. **Do not enable `ENABLE_YAML_STYLE_METADATA_BLOCKS`.** Cut the frontmatter off yourself (ticket 1's splitter) and parse `&src[body_start..]`, adding `body_start` back to every range. Reason: the option recognises `---`...`---` blocks anywhere in the file, not only at the top, and silently swallows the content (section 1.3).
2. **Compute line/col yourself** from the byte ranges of `into_offset_iter()` with a line-start table (section 1.1).
3. **Write the slugger by hand** (about 15 lines plus a Unicode-category check). pulldown-cmark has none. I validated such a function against GitHub's own renderer on 29 headings with 0 mismatches (section 4).

Runner-up cost is in the last section.

## Method and trust levels

- **Crate source**: pulldown-cmark 0.13.4 as unpacked in `~/.cargo/registry/src/*/pulldown-cmark-0.13.4/` (`src/lib.rs`, `src/parse.rs`, `src/firstpass.rs`, `README.md`). This is the exact text docs.rs renders for that version; I read it locally rather than through docs.rs.
- **Empirical**: throwaway crates in the scratchpad (`pd/`, `sl/`, `slug/`, `cm/`), pinned `pulldown-cmark =0.13.4` and `comrak =0.55.0`. Inputs and outputs quoted below are real runs.
- **GitHub ground truth**: `POST https://api.github.com/markdown` with `mode: markdown` (GitHub's own renderer, unauthenticated) and its `id="user-content-..."` anchors. The GitHub algorithm itself is closed source, so this API is the best primary evidence available, dated 2026-09-19.
- **Spec**: CommonMark 0.31.2, fetched from spec.commonmark.org.

## 1. Offsets, line/col, frontmatter

### 1.1 Do offsets suffice for exact line and column?

Yes.

- `Parser::into_offset_iter()` yields `(Event, Range<usize>)` where the range is the event's span in the source string. Source (`src/parse.rs`, doc comment on `into_offset_iter`): "produces an iterator that produces `(Event, Range)` pairs, where the `Range` value maps to the corresponding range in the markdown source." README: "source-maps are supported"; "you can call `into_offset_iter()` to create an iterator that yields `(Event, Range)` pairs".
- Ranges are **byte offsets into the `&str`** given to the parser. Every offset I sliced with (`&src[r]`) landed on a char boundary, including Thai text with combining marks (U+0E31, U+0E47-0E4C) and CRLF input.
- The parser gives no line/col. Derive it: `line = 1 + count('\n' before offset)`; `col = chars from the last '\n' to the offset, plus 1`. For many findings, build a `Vec<usize>` of line starts once and binary-search.
- CRLF works with `\n`-counting because a `\r` sits at end of line and never precedes a start offset. Lone-`\r` line endings (classic Mac) were **not tested**.
- What the range covers:
  - `Start(Link)` / `Start(Image)`: the whole construct, `[text](dest "title")`, from `[` (or `![`) to the closing `)`. So line:col of the link is the `[`. The parser does **not** give the offset of the destination alone; if a finding should point at the destination, scan the source slice after `](`.
  - `Start(Heading)`: from the `#` (or first setext text line) through the trailing line ending. Setext headings span two lines. ATX text is at `range.start + prefix`; the `Text` child range gives the exact text span.
  - `Start(CodeBlock)`: for a fenced block, from the opening fence; for an indented block, from the first content byte after the 4-space indent (12:5 in my test). Inside a blockquote or list item the start is after the container prefix (`> ` puts it at col 3).
  - Multi-line links (`[text\ncontinues](multi.md)`) start at the `[` on the first line. Correct.
- **Column unit is a typdoc decision.** I used Unicode scalar values (chars). Editors differ (UTF-16 units, graphemes, bytes). comrak's `sourcepos` columns, by contrast, are **byte-based**: on `ไทย [ลิงก์](ไฟล์.md)` comrak reported the link at column 11 where pulldown-derived char column is 5. Which convention editors and the design want is **not verified**; pick one and document it.
- `toc` needs `line` and `end`. The parser gives heading start and end offsets; the section `end` (next heading of same or higher level, or EOF) must be computed from the heading list, not from the parser. Heading ranges include the trailing newline, so use `end - 1` when mapping the end offset to a line.

### 1.2 Frontmatter offset strategy (verified equivalent)

Slicing the body and adding the offset back gives byte-for-byte the same ranges as parsing the whole file with the metadata option on:

| Input | Whole file + yaml option | Body slice + offset |
| --- | --- | --- |
| `fm.md` link | `9:5  84..123` | `9:5  84..123` |
| `fm.md` heading | `7:1  67..79` | `7:1  67..79` |
| Thai/CRLF file, heading | `5:1  36..135` | `5:1  36..135` |
| Thai/CRLF file, links | `5:35 106..118`, `7:5 147..181`, `7:34 208..217` | identical |

Line numbers stay "counted from the top of the file, frontmatter included" because line/col are computed on the full source using the shifted offsets.

### 1.3 Why not `ENABLE_YAML_STYLE_METADATA_BLOCKS`

Source: `src/lib.rs` (option docs: "Metadata blocks in YAML style, i.e.: starting with a `---` line, ending with a `---` or `...` line") and `src/firstpass.rs` around line 314 ("metadata blocks cannot be indented"; the check is `if indent == 0 { scan_metadata_block(...) }` with no "at start of document" condition).

Observed with the option on (`mid.md` = `# H`, `text`, `---`, `[hidden](a.md)`, `---`, `[l](b.md)`):

- The mid-document `---` pair became `MetadataBlock(YamlStyle)`; `[hidden](a.md)` became its `Text` and **no Link event was emitted**. A checker would silently miss that link. This is the "wrong answer, silently" failure the map warns about.
- A file that begins with a blank line then `---`...`---` is also treated as metadata (frontmatter that the design would not recognise).
- Without the option, the same input yields `Rule` plus a setext H2 whose text is `[hidden](a.md)`, and the link IS reported. Correct per CommonMark, but see below.

Without the option and without stripping, frontmatter pollutes the tree (`fm.md`): YAML comment line `# a yaml comment` became an H1 heading, `title: ...` became a setext H2, `---` a `Rule`. So frontmatter must be removed before parsing, by the design's own splitter. Because your splitter and the parser must agree on where the body starts, cutting by the splitter's byte offset is the only consistent option.

Other frontmatter edge cases I ran with the option on (relevant only if you did use it): unterminated `---` at top: parsed as a `Rule`, not metadata; `...` terminator accepted; a BOM before `---` prevents metadata recognition (setext H2 instead); `---  ` with trailing spaces prevents it. Ticket 1's splitter should define these, since the parser will not be consulted.

## 2. Fenced, indented and inline code, and other non-link contexts

All from `code.md`, `cont.md` and `links.md` runs.

| Construct | Events | Links/headings inside? |
| --- | --- | --- |
| Fenced ```` ``` ```` and `~~~` | `Start(CodeBlock(Fenced(info)))` ... `End(CodeBlock)`; body is `Text` | No `Link`, no `Heading` emitted. `# not heading` stays text. |
| Fence in blockquote / list item | Same, range starts after the container prefix | Excluded correctly |
| Longer outer fence containing a shorter one (```` ````md ```` wrapping ```` ``` ````) | One CodeBlock | Excluded |
| Unclosed fence | CodeBlock runs to EOF (CommonMark: "or the last line of the document") | Excluded |
| Indented code (4 spaces) | `Start(CodeBlock(Indented))` | Excluded; `## indented 4` is not a heading, `## indented 3` (3 spaces) is |
| Inline code, incl. double-backtick with a backtick inside | `Event::Code(text)` with the range of the whole span | `` `[inline](d.md)` `` is a `Code` event, not a `Link` |
| Raw HTML block | `Start(HtmlBlock)`, lines as `Html` | `[html block](f.md)` inside `<div>` is **not** a link (CommonMark: raw HTML) |
| Inline HTML | `InlineHtml("<a href=\"g.md\">")` | `<a href>` is not a Markdown link; a checker for "standard Markdown links" ignores it, which matches the design, but it is a silent gap |
| HTML comment `<!-- [c](i.md) -->` | `HtmlBlock` | Excluded |
| Link inside blockquote, list, nested list, table cell | `Link` events with correct ranges | Reported (table cells need `ENABLE_TABLES`) |
| Heading in blockquote or list item | `Heading` event | Present: decide whether `toc` lists these (GitHub does render them with anchors) |

The parser handles: nesting of containers, fence length rules, tilde vs backtick, multi-backtick code spans, and the CommonMark precedence "Code spans, HTML tags, and autolinks have the same precedence" (spec.commonmark.org 0.31.2, section 6.1; the spec's own example `[not a \`link](/foo\`)` is code, not a link). A hand-rolled scanner must reimplement each of these (see the runner-up cost).

## 3. What a relative-path checker must handle

Filter `Event::Start(Tag::Link { link_type, dest_url, .. })`. Findings from `links.md`:

| Case | pulldown-cmark result | Action needed by the checker |
| --- | --- | --- |
| `[t](a.md)`, `[t](../x.md#h)`, `[t](a.md?x=1#h)` | `Inline`, `dest_url` = raw dest | Split at first `#` for the fragment; decide about `?query` |
| `[t](#local)` | `Inline`, dest `#local` | Same-file anchor check; no path |
| `[t]()` | `Inline`, dest empty | Decide: skip or report |
| `[t](https://x)`, `[t](mailto:x@y.z)` | `Inline`, dest keeps scheme | Skip by scheme (design already says so). Also decide `//host/x`, and `/x.md` (GitHub treats leading `/` as repo-root-relative per docs; typdoc's meaning is undefined in the design) |
| `[t](my file.md)` | **Not a link at all**: emitted as `Text` | CommonMark: a bare destination "does not include ... space character". The link is invisible to the checker. Consider a cheap "suspected link" scan over `Text` events (`](`) if silent misses matter |
| `[t](<my file.md>)`, `[t](<my file.md> "t")` | `Inline`, `dest_url` = `my file.md` (angle brackets removed) | Nothing extra; works |
| `[t](<a<b.md>)`, `[t](<a\nb.md>)` | Not links (`<` and line endings are disallowed inside angle dest) | Silent, as above |
| `[t](my%20file.md)` | `dest_url` = `my%20file.md`, **not** percent-decoded | Percent-decode before filesystem lookup |
| `[t](foo(1).md)` (balanced parens), `[t](foo\(1\).md)` | `foo(1).md` both | Backslash escapes and entities (`a&amp;b.md` becomes `a&b.md`) are already decoded by the parser |
| `[t](a\b.md)` | dest kept as `a\b.md` (backslash before a non-punctuation char is literal) | Do not treat `\` as a path separator |
| `[t][ref]`, `[t][]`, `[t]` with `[ref]: path` | `Link` with `link_type` `Reference`/`Collapsed`/`Shortcut`; `dest_url` already resolved; `id` = label. Event range is the usage site | Design text says only `[text](path)` counts, but reference-style is standard Markdown. **Decision needed**: filter by `link_type == Inline` to match the design literally, or check all. To report on the definition line, use `it.reference_definitions()` (`RefDefs`, `LinkDef.span`) |
| `[x]` where no definition exists (e.g. `[WF-3]`) | Plain `Text` (with a broken-link callback it would become `*Unknown`; `Parser::new_ext` does not) | No special handling; this matches "mentions are not links" |
| Reference definitions themselves | **No event.** Only in `RefDefs` | An unused definition, or a second definition of the same label, is never seen through events. `RefDefs` keeps only the first definition of a label (I saw `x => first.md`, `second.md` dropped). `RefDefs::iter()` is over a `HashMap`; sort by `span.start` for stable output |
| Definition inside a code fence or indented code | Not a definition | Correct, as in CommonMark |
| Definition inside a blockquote | Applies to the whole document (CommonMark example 218) | Nothing extra |
| Autolinks `<https://ex.com>` | `link_type: Autolink`, absolute | Skip by scheme. Only absolute URIs qualify (spec: "a scheme followed by a colon"), so `<./rel.md>` and `<foo.md>` are not autolinks and not links (they came out as text) |
| `<a@b.c>` | `link_type: Email` | Skip |
| Bare `https://x` and `www.x` | No `Link` (pulldown-cmark does not implement GFM extended autolinks) | Nothing; they are absolute anyway |
| `![alt](img.png)` | `Tag::Image`, separate from `Tag::Link` | The design's `ignore` example ("images, generated files") implies images are checked by default. With a parser the choice is one match arm. `![r][img]` reference images work the same |
| `[![alt](img2.png)](target.md)` | `Link` containing `Image` (both reported, nested ranges) | Fine |
| `[nested [brackets]](nb.md)` | One `Link` | Fine |
| `\[esc](z.md)`, `` `[x]`(y.md) `` | Not links | Fine |
| Wikilinks `[[x]]` | Only with `ENABLE_WIKILINKS` (off) | Leave off |

The design's own example `See [the secret-handling precedent](../../typmem/memory/precedents/secret-handling.md)` parses as expected (range covers `[` to `)`).

## 4. Heading slugs

### 4.1 pulldown-cmark

No slug generation anywhere in 0.13.4: `grep -ri slug src/*.rs` returns nothing. `Tag::Heading { id, classes, attrs }` carries an `id` only when `Options::ENABLE_HEADING_ATTRIBUTES` is on and the author wrote `{#id}` (`src/lib.rs`: "`id`, `classes` and `attrs` are only parsed and populated with [`Options::ENABLE_HEADING_ATTRIBUTES`], `None` or empty otherwise"). GitHub does not implement that syntax (it produced `foo-custom-id` for `# Foo {#custom-id}`), so leave it off. Slugs must be written by hand.

### 4.2 GitHub's documented rules

Source: docs.github.com "Basic writing and formatting syntax", section "Section links" (fetched raw HTML and read; quote):

> Letters are converted to lower-case. Spaces are replaced by hyphens (-). Any other whitespace or punctuation characters are removed. Leading and trailing whitespace are removed. Markup formatting is removed, leaving only the contents (for example, _italics_ becomes italics). If the automatically generated anchor for a heading is identical to an earlier anchor in the same document, a unique identifier is generated by appending a hyphen and an auto-incrementing integer.

The same page's worked example (heading with UTF-8 `Θ`, double spaces, formatting) shows the link `#thisll-be-a-helpful-section-about-the-greek-letter-Θ` with an **uppercase** `Θ`, which contradicts "letters are converted to lower-case". Live rendering gives lowercase `θ` (below), so the docs example looks like a docs typo. Unresolved but low impact: the live renderer is the authority.

### 4.3 Live behaviour (GitHub's `/markdown` API, 2026-09-19)

Each row is heading source text and the `id` GitHub rendered (`user-content-` prefix stripped; that prefix is added by the sanitiser and the link `href` omits it).

| Heading | GitHub anchor |
| --- | --- |
| `Sample Section` | `sample-section` |
| `This'll be a _Helpful_ Section About the Greek Letter Θ!` | `thisll-be-a-helpful-section-about-the-greek-letter-θ` (lowercased, apostrophe and `!` dropped) |
| `Dup`, `Dup`, `Dup 1`, `Dup` | `dup`, `dup-1`, `dup-1-1`, `dup-2` (collision-aware: the literal `Dup 1` is disambiguated against the earlier generated `dup-1`) |
| `ทดสอบ ภาษาไทย ก็ได้` | `ทดสอบ-ภาษาไทย-ก็ได้` (Thai kept as-is, including vowel and tone marks) |
| `หัวข้อ (สอง): ผ่านไหม?` | `หัวข้อ-สอง-ผ่านไหม` (Thai kept; `()`, `:`, `?` removed) |
| `Café Ünïcode ÀB` | `café-ünïcode-àb` (full Unicode lowercasing, accents kept) |
| `` `code` and [link](x.md) *em* &amp; R&D `` | `code-and-link-em--rd` (markup stripped; `&` removed, leaving two hyphens) |
| `A_B__C  d---e a.b/c` | `a_b__c--d---e-abc` (underscore and hyphens kept; two spaces give two hyphens; `.` and `/` removed) |
| `日本語 見出し` | `日本語-見出し` |
| `Trailing hyphen -` / `- leading` | `trailing-hyphen--` / `--leading` (hyphens are not trimmed) |
| `1. Numbered` | `1-numbered` |
| `⚙️ emoji 🎉 x` | `️-emoji--x` (emoji removed; the variation selector U+FE0F survives) |
| `Foo {#custom-id}` | `foo-custom-id` |
| Setext `Setext Title` / `---` | `setext-title` |
| `a ![alt text](i.png) b` | `a--b` (image alt text contributes nothing) |
| `<b>bold</b> x` | `bold-x` |
| `a<br>b` | `ab` |
| Setext two lines `Line one` / `line two` | `line-oneline-two` (the line break is removed, not turned into a space) |
| `Tab<TAB>here` | `tabhere` |
| `&lt;tag&gt; &copy; 2026` | `tag--2026` |
| `[WF-3](WF-3.md): thing` | `wf-3-thing` |
| `~~strike~~ **b** \`c\`` | `strike-b-c` |
| `a<NBSP>b` | `ab` (no-break space is removed, not converted) |

Quirk, unresolved: for an empty heading (`## ` with no text) and headings whose slug is empty (`## !!!`) the API returned an empty `id` for the first, `-1` for the next, and no anchor at all for the last `<h2>`. I did not pin this down. If typdoc can meet an empty slug, define it explicitly (ticket 4).

### 4.4 Community implementation as an independent check

`github-slugger` (github.com/Flet/github-slugger, `index.js`, `regex.js`, `script/generate-regex.js`, README). README: "Generate a slug just like GitHub does for markdown headings ... The overall goal of this package is to emulate the way GitHub handles generating markdown heading anchors as close as possible" and "This project is not a markdown or HTML parser ... Instead pass the plain text value of the heading". Algorithm (`slug()`): lowercase, delete every character matched by a generated regex, replace `' '` with `-`. The regex removes Unicode categories `Other_Number`, all punctuation except Connector (`Pc`, so `_` stays), all of `Dash_Punctuation` except `-`, all `Symbol`, `Control`, `Private_Use`, `Format`, `Unassigned`, and `Separator` except space, **minus anything with the Alphabetic property**. Marks (`Mn`, `Mc`, `Me`) are not in the strip list, which is why Thai tone marks survive. Uniqueness loop: `while occurrences has result: occurrences[original]++; result = original + '-' + n`, which reproduces `dup-1-1`. This is an emulation, not GitHub's code.

`html-pipeline` `TableOfContentsFilter` (github.com/gjtorikian/html-pipeline v2.14.3, `lib/html/pipeline/toc_filter.rb`) is an older open GitHub-era implementation: `str.downcase(:ascii)`, remove `/[^\p{Word}\- ]/`, `tr(' ', '-')`, suffix `-N` from a per-slug counter with **no collision loop**. It differs from live GitHub on non-ASCII lowercasing and on `Dup 1`; do not use it as the spec.

### 4.5 Prototype: pulldown-cmark heading text plus a hand-written slugger

Scratchpad crate `slug/` (pulldown-cmark 0.13.4, `regex`):
- Heading text = concatenation of `Text` and `Code` events between `Start(Heading)` and `End(Heading)`, **skipping `Text` inside `Tag::Image`**; `SoftBreak`/`HardBreak` contribute `'\n'` (which the slugger then removes), not a space; `InlineHtml` and `Html` contribute nothing. Options: `ENABLE_STRIKETHROUGH | ENABLE_TABLES` (strikethrough changes nothing since `~` is a symbol, but parse it the way GitHub does).
- Slug = lowercase, then delete `[\p{No}\p{Pe}\p{Pf}\p{Pi}\p{Ps}\p{Po}\p{Pd}\p{S}\p{Cc}\p{Co}\p{Cf}\p{Cn}\p{Z}--[\p{Alphabetic} \-]]`, then `' '` to `-`, then the github-slugger dedupe loop.
- Result: the 20 headings of set 1 (the first table in 4.3, through `---`) and the 9 of set 2 all matched GitHub's ids exactly. The first draft had two bugs I fixed only after comparing with GitHub: image alt text included, and soft break mapped to a space. That is the argument for keeping a fixtures test that mirrors the table in 4.3.
- Rust `std::char` exposes no general-category query that I know of (not verified against docs); I used the `regex` crate's `\p{..}` classes. A small crate such as `unicode-general-category` is the alternative. `ship` already depends on `regex 1`, but that does not bind typdoc.

comrak's `Anchorizer` (docs.rs/comrak/latest/comrak/struct.Anchorizer.html): "Returns a String that has been converted into an anchor using the GFM algorithm, which involves changing spaces to dashes, removing problem characters and, if needed, adding a suffix to make the resultant anchor unique." Run on the same 20 headings, comrak 0.55.0 matched GitHub on all 20, including Thai, `dup-1-1` and `⚙️`. It takes plain heading text, so heading-text extraction (image alt, line breaks) is still on the caller.

## 5. Alternatives to pulldown-cmark

- **comrak 0.55.0** (crates.io, updated 2026-09-06). Has `Anchorizer` and per-node `sourcepos` (`start/end` line and column), and a `front_matter_delimiter` option. Observed: front matter recognised only at line 1 (`FRONT 1:1-5:3`); a mid-document `---`...`---` was NOT swallowed (`[hidden](a.md)` was reported as a link, plus a setext heading), which is safer than pulldown's metadata option. Code blocks, code spans and links classified as expected. Cost: columns are bytes (see 1.1); an arena AST; `cargo tree` shows 16 crates for comrak (comrak plus 15 dependencies such as `phf`, `unicode-normalization`, `caseless`) versus 4 for pulldown-cmark (itself plus `bitflags`, `memchr`, `unicase`); one more parser in the fleet's dependency set beside the one `ship` already has. Not tested: reference-style link positions, autolinks, or whether its extension options change link edge cases.
- **markdown (markdown-rs) 1.0.0** (crates.io, released 2025-04-23; docs.rs description "CommonMark compliant markdown parser in Rust with ASTs and extensions"). **Not verified or tested by me**: whether its mdast positions carry line/column/offset, and its frontmatter handling. A quick fetch summary I got claimed a 2026 release date, which contradicts crates.io, so I treat that summary as unreliable.

## 6. Hand-rolled scanning versus a parser

Evidence that hand-rolling is not small: every row in sections 2 and 3 is a CommonMark rule a scanner must reproduce to get "links inside code are not links" and "headings inside fenced code are ignored" right:

- fences: 3+ backticks or tildes, indent up to 3, closing fence at least as long as the opener and of the same character, nested inside blockquote and list prefixes, unclosed to EOF;
- indented code (4 spaces, but not inside a paragraph continuation, and container-relative);
- code spans of any backtick length, which can span lines and take precedence over link brackets;
- raw HTML blocks (seven kinds) and comments, where `[x](y.md)` is text;
- backslash escapes, entities, balanced parentheses, angle destinations, and destinations with spaces that are not links;
- reference definitions and their resolution (first wins, case and whitespace normalised labels), including definitions inside containers;
- setext vs ATX headings and the `---` ambiguity between rule, setext underline and frontmatter.

A simple hand scanner (skip fences and backtick spans, regex `\[..\]\(..\)`) handles the common cases but would be wrong in exactly the ways the map warns about (silent misses or false positives) for: `~~~` and 4-backtick fences, indented code, links in HTML blocks, double-backtick spans, and reference-style links.

## 7. Recommendation and runner-up

**Recommendation: build on pulldown-cmark 0.13.4** for block/inline classification, link extraction and heading text, and hand-write only three small things: the frontmatter cut (ticket 1), line/col from byte offsets, and the GitHub slugger with a fixtures test built from the table in 4.3. Reasons: correct code/HTML/container exclusion for free (verified on 40+ constructs), source ranges sufficient for exact line:col (verified with Thai and CRLF), already in the fleet at the same version, 3 small dependencies (`bitflags`, `memchr`, `unicase`).

Known traps to write into the design or contract: (a) never enable the YAML metadata option; (b) `[t](my file.md)` is not a link and is invisible; (c) percent-decode `dest_url`; (d) reference-style links: decide checked or ignored; (e) images: decide checked or ignored; (f) column unit; (g) empty-slug headings.

**Runner-up: hand-rolled scanning.** Cost: a state machine that must track container prefixes, fence and code-span rules, raw HTML blocks and reference definitions (section 6) to match CommonMark, plus a test suite to police it; residual risk of silent wrong answers on constructs like `~~~` fences, indented code and double-backtick spans. It buys zero dependencies and full control over the "suspected broken link" heuristic, which are not needed given that pulldown-cmark is already vetted and in the fleet. If a parser were rejected, comrak (built-in `Anchorizer` and `sourcepos`) would beat hand-rolling: it costs 16 crates, byte-based columns and a second parser in the fleet, in return for not writing the slugger.

## Unverified or untested (summary)

- Column convention editors use; whether typdoc counts chars, UTF-16 units or graphemes.
- Lone `\r` line endings; tabs inside a line (counted as one char in my column).
- Empty and all-punctuation heading slugs on GitHub (observed oddity, not explained).
- Why GitHub's docs example shows uppercase `Θ`.
- markdown-rs positions and frontmatter; comrak reference-link and autolink positions.
- Whether Rust std offers a Unicode general-category query.
- Performance and memory on large files (not measured).
- GitHub's private implementation; the `/markdown` API output is the evidence, and it may change without notice.

## Sources

- pulldown-cmark 0.13.4 source in the local cargo registry (`README.md`, `src/lib.rs`, `src/parse.rs`, `src/firstpass.rs`); API pages https://docs.rs/pulldown-cmark/0.13.4/pulldown_cmark/struct.OffsetIter.html, `.../struct.Options.html`, `.../enum.Tag.html`, `.../struct.RefDefs.html` (HTTP 200 confirmed; text read from the vendored source, not the rendered page).
- CommonMark 0.31.2: https://spec.commonmark.org/0.31.2/ (link destinations, code fences, indented code, HTML blocks, autolinks, link reference definitions, code span precedence, example 218).
- GitHub docs: https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax (Section links; Relative links: "Links starting with `/` will be relative to the repository root").
- GitHub Markdown API (ground truth for slugs): https://api.github.com/markdown
- github-slugger: https://github.com/Flet/github-slugger (README.md, index.js, regex.js, script/generate-regex.js on `master`).
- html-pipeline: https://github.com/gjtorikian/html-pipeline/blob/v2.14.3/lib/html/pipeline/toc_filter.rb
- comrak: https://docs.rs/comrak/latest/comrak/struct.Anchorizer.html; version from https://crates.io/api/v1/crates/comrak (0.55.0).
- markdown-rs: https://docs.rs/markdown/latest/markdown/ ; version from https://crates.io/api/v1/crates/markdown (1.0.0).
- Sibling dependency line: the `ship` crate's `Cargo.toml` (`pulldown-cmark = { version = "0.13.4", default-features = false, features = ["html"] }`).
