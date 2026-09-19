# YAML frontmatter in Rust: parse, type, and edit-in-place

Researched 2026-09-19 (all "latest" versions and dates below are as returned by the crates.io and GitHub APIs on that day). Method: primary sources (crates.io API, crate source as published on crates.io, upstream READMEs, RustSec advisory-db, the YAML 1.1/1.2 specs), plus empirical probes in a scratch cargo project outside the repo (rustc 1.96.0, cargo 1.96.0). Nothing in the typdoc repo was touched apart from this file.

Notation: **[verified]** = I saw it in a primary source or ran it. **[observed]** = my own probe result (reproducible from the inputs quoted). **[unverified]** = see the last section.

---

## 0. Bottom line up front

- No mainstream Rust YAML crate that goes through a data model (serde_yaml and its forks, saphyr `Yaml`, yaml-rust2, serde-saphyr) preserves comments, quote style or flow style on write. Reserialising a parsed value also rewrites user text (`1e3` becomes `1000.0`). **[observed]**
- Two crates *do* edit in place: `yaml-edit` (rowan lossless syntax tree) and `yamlpatch`/`yamlpath` (tree-sitter, from zizmor). Both are usable but both misbehaved in my probes (section 3).
- Line-based surgical editing is sound only if the scanner is a real state machine (multi-line quoted scalars, multi-line flow collections and block scalars all make a naive "key at column 0" scan wrong, which I demonstrated). It is a bounded amount of work, not a trivial one.
- `serde_yaml` is archived and deprecated. Verified, not assumed.

---

## 1. Options for parsing YAML in Rust

### 1.1 The serde_yaml family (serde data model, libyaml under the hood)

| Crate | Latest (crates.io) | Last release | Status, per primary source |
|---|---|---|---|
| `serde_yaml` | 0.9.34+deprecated | 2024-03-25 | GitHub repo `dtolnay/serde-yaml` has `archived: true`, last push 2024-03-25 (GitHub API). README says "(This project is no longer maintained.)" (repo README, as fetched). crates.io lists the version string `0.9.34+deprecated` (crates.io API). |
| `yaml_serde` | 0.10.7 | 2026-08-18 | README of `yaml/yaml-serde` says: "This is the actively maintained fork of serde-yaml, published as `yaml_serde` by the official YAML organization ... The original `serde_yaml` crate is no longer maintained." Repo not archived, pushed 2026-08-18, Apache-2.0, created 2025-11-25. Depends on `libyaml-rs ^0.3`. Migration is a Cargo rename (`serde_yaml = { package = "yaml_serde", version = "0.10" }`). |
| `serde_yaml_ng` | 0.10.0 | 2024-05-26 | Fork by acatton. Repo not archived, last push 2025-09-14. Depends on `unsafe-libyaml`. No crates.io release since 2024-05. |
| `serde_norway` | 0.9.42 | 2024-12-21 | Fork (repo moved from cafkafk/serde-yaml; GitHub API returned "Moved Permanently", new location not chased). Uses `unsafe-libyaml-norway`. |
| `serde_yml` | 0.0.13 | 2026-05-27 | Dead end. crates.io description: "DEPRECATED, `serde_yml` is unmaintained ... thin compatibility shim". Repo archived. RUSTSEC-2025-0068: "serde_yml crate is unsound and unmaintained" (rustsec/advisory-db, `crates/serde_yml/RUSTSEC-2025-0068.md`). |

Note on RustSec: `crates/serde_yaml/` in advisory-db contains only `RUSTSEC-2018-0005.md` (an old recursion issue). There is no "unmaintained" advisory for serde_yaml, so `cargo audit` will not flag it even though it is archived. **[verified: directory listing]**

Also: RUSTSEC-2024-0320 marks the old `yaml-rust` as unmaintained and points to `yaml-rust2` (advisory-db `crates/yaml-rust/RUSTSEC-2024-0320.md`).

### 1.2 Pure-Rust parsers (own scanner, own value type)

| Crate | Latest | Last release | Status |
|---|---|---|---|
| `yaml-rust2` | 0.13.0 | 2026-09-11 | README: "This crate will receive only basic maintenance and keep a stable API. `saphyr` will accept new features, at the cost of a less stable API." Not archived. Claims YAML 1.2 compliance and passes the YAML test suite. |
| `saphyr` / `saphyr-parser` | 0.1.0 | **2026-09-19** (published the day of this research) | Same lineage (fork of yaml-rust2). Repo not archived, pushed 2026-09-19. README: "fully YAML 1.2 compliant". Version history: 0.0.6 in 2025-06, then 0.0.7 to 0.0.12 in 2026-07/08, then 0.1.0. A year-long gap followed by a burst; treat as young. |
| `serde-saphyr` | 1.3.0 | 2026-09-16 | Serde layer over the saphyr-family parser. Not archived, very active. README documents a `Commented<T>` wrapper that captures and emits comments **for fields you model**, not a document-preserving editor. |
| `marked-yaml` | 0.8.0 | 2025-05-26 | Spans on values ("provenance"); built on `yaml-rust2 ^0.10`. No release in 16 months. Read-only. |

### 1.3 Frontmatter-splitting crates (convenience only)

- `gray_matter` 0.3.2 (2025-07-10): parses with `yaml-rust2 ^0.10`, returns a data model. No write-back. crates.io dependency list.
- `yaml-front-matter` 0.1.0 (2021-09-25): depends on `serde_yaml ^0.8`. Stale.
- Splitting the `---` fences off a Markdown file is a few lines of code and typdoc must never re-serialise the body anyway, so neither crate earns its place. Recommendation: hand-roll the fence split.

### 1.4 Crates that can edit in place

**`yaml-edit` 0.3.2** (jelmer/yaml-edit, released 2026-09-17)
- README: "A Rust library for parsing and editing YAML files while preserving formatting, comments, and whitespace. Built with rowan for lossless syntax trees." DESIGN.md: primary goal is "modifying YAML files while preserving formatting, comments, whitespace, quote styles, key ordering, and anchors/aliases."
- API (from source): `Mapping::set`, `set_with_field_order`, `insert_after`, `insert_before`, `remove`, `rename_key`; `Sequence::push/insert/set/remove`; `ScalarValue::string()` (never type-detects) vs `ScalarValue::parse()` (detects).
- Dependencies: `rowan`, `regex`, `base64`. Rust-version 1.70.
- Maturity: first release 0.1.0 on 2025-08-10, 0.3.x by Aug 2026 (fast API churn). 442 of the commits are by one author (jelmer); 27 by another. 10 GitHub stars. 14 reverse dependencies on crates.io. Issue/PR #109 on 2026-09-17 is titled "Fix a large number of bugs found with fuzzing" (two days before this research), so bugs are being found and fixed quickly, and more are likely.
- Empirical results in section 3.

**`yamlpatch` 1.30.1 + `yamlpath` 1.30.1** (zizmorcore/zizmor workspace crates)
- yamlpatch README: "Comment and format-preserving YAML patch operations", and an explicit caveat: "This is not a substitute for comprehensive YAML processing libraries. It's designed for targeted modifications." Ops: Replace, Add, Remove, MergeInto, Append, ReplaceComment, EmplaceComment, RewriteFragment. "Each operation preserves the document's formatting and structure (as best-effort)."
- yamlpath README: built on `tree-sitter` + `tree-sitter-yaml`.
- Costs: crate versions move in lock-step with the zizmor CLI (1.30.1). **Latest versions pull `tree-sitter-iter` which declares rustc 1.97; my toolchain is 1.96 and `cargo build` refused.** I had to pin `=1.29.0` and pass `--ignore-rust-version` to get it to compile (it did compile and run). Tree-sitter also brings C build dependencies. Patch values are `yaml_serde::Value`. 8 reverse dependencies.

**Not viable as editors:** `serde_yaml`-family, `yaml-rust2`, `saphyr` `Yaml` + `YamlEmitter`, `serde-saphyr`. They emit from a value tree (evidence in 3.1). `saphyr` `MarkedYaml` exposes spans, which could drive a splice approach; see 3.4 for why I would not trust it yet.

---

## 2. How each option types the values you care about

### 2.1 What the specs say

**YAML 1.2.2 core schema, section 10.3.2 (yaml.org/spec/1.2.2/)** [verified, fetched]. Plain scalars are matched against:

- null: `null | Null | NULL | ~` and empty
- bool: `true | True | TRUE | false | False | FALSE` (only these six)
- int: `[-+]? [0-9]+`, `0o [0-7]+`, `0x [0-9a-fA-F]+`
- float: `[-+]? ( \. [0-9]+ | [0-9]+ ( \. [0-9]* )? ) ( [eE] [-+]? [0-9]+ )?`, plus `.inf`/`.nan` forms
- anything else is `str`.
- The core schema has **no timestamp type**. Section 10.4 only says other schemas may add regexes "such as timestamps".

Consequences under 1.2 core: `WF-3` is a string. `no` is a string. `2026-09-19` and `2026-09-19T14:30:00+07:00` are strings. `1e3` is a **float** (matches the float regex with no dot). `0755` is the decimal integer 755 (1.2 dropped the bare-octal form; the int regexes above have no leading-zero-octal rule).

**YAML 1.1 types (yaml.org/type/*.html, Working Drafts 2005)** [verified, fetched]:

- bool regexp: `y|Y|yes|Yes|YES|n|N|no|No|NO|true|True|TRUE|false|False|FALSE|on|On|ON|off|Off|OFF`, so `no` is **false**.
- float base-10 regexp: `[-+]?([0-9][0-9_]*)?\.[0-9.]*([eE][-+][0-9]+)?`. It requires a `.` and a signed exponent, so by that regexp `1e3` is **not** a float in 1.1 (it would fall through to string in a strict 1.1 resolver). I derived this from the regexp; I did not run a 1.1 parser (PyYAML etc.).
- timestamp: `[0-9]{4}-[0-9]{2}-[0-9]{2}` (ymd) and the longer form with `[Tt]` or space, optional fraction and `[-+]hh(:mm)?` zone. So both `2026-09-19` and `2026-09-19T14:30:00+07:00` are **timestamps** in 1.1.
- int: leading `0` means octal; `_` separators; base 60 with `:`.

Net: the same document reads differently under 1.1 and 1.2 for `no`, dates/datetimes, `1e3`, `0755`. That is exactly the set of values typdoc's design puts in frontmatter, so the design should not rely on any parser's untyped coercion. Read known fields into `String` (or a typed enum built from a string), and validate by regex in typdoc itself.

### 2.2 What the crates actually do [observed]

Probe document (top-level keys): `id: WF-3`, `status: no`, `ratio: 1e3`, `due: 2026-09-19`, `at: 2026-09-19T14:30:00+07:00`, `ver: 1.10`, `zip: 0755`, `yes_key: yes`, `nul: ~`, `tags: [a, b]`.

| Crate | `WF-3` | `no` | `1e3` | `1.10` | `0755` | `2026-09-19` | `...T14:30:00+07:00` | `yes` |
|---|---|---|---|---|---|---|---|---|
| serde_yaml 0.9 `Value` | str | str | **Number(1000.0)** | **Number(1.1)** | str | str | str | str |
| yaml_serde 0.10 `Value` | str | str | Number(1000.0) | Number(1.1) | str | str | str | str |
| serde-saphyr into `serde_json::Value` | str | **false** | 1000.0 | 1.1 | **755.0** | str | str | **true** |
| saphyr 0.1 `Yaml` | str | str | FloatingPoint(1000.0) | FloatingPoint(1.1) | Integer(755) | str | str | str |
| yaml-rust2 0.13 `Yaml` | str | str | Real("1e3") | Real("1.10") | Integer(755) | str | str | str |
| **Any of the above, target field typed `String`** (serde_yaml, yaml_serde, serde-saphyr) | "WF-3" | "no" | "1e3" | "1.10" | "0755" | "2026-09-19" | "2026-09-19T14:30:00+07:00" | n/a |

Takeaways:
1. No crate returns a date type for `2026-09-19` or the offset datetime. All three keep them as strings. The design's "dates are ordinary values" is satisfied for free. typdoc parses date syntax itself.
2. serde_yaml/yaml_serde/saphyr/yaml-rust2 all follow 1.2-ish bool rules (`no` stays a string). **serde-saphyr into an untyped target coerces `no`/`yes` to bool and `0755` to 755** (its README line: "By default, if the target field is boolean, serde-saphyr will attempt to interpret standard YAML 1.1 values as boolean"; my probe shows it also happening for an untyped `serde_json::Value` target). Do not use it with untyped values.
3. `1e3` and `1.10` are the dangerous ones: read into an untyped value they lose their text (`1.10` becomes `1.1`, a version number silently changes). Reading into `String` fields preserves the text in all three serde crates. **This is the single strongest argument for typed reads with `String` fields, and against a `Value` round trip.**
4. yaml-rust2's README says "This library does not try to interpret any type specifiers"; that sentence is under "Security" and is about tags, not plain-scalar resolution. It does resolve plain scalars (see table). A summarising tool misread this for me at first; I checked against the source and the probe.
5. yaml-edit: `ScalarValue::classify_plain` follows the 1.2 core schema (its docs say so). `ScalarValue::auto_detect_type` (used by `parse`) is "deliberately more permissive": recognises `yes/no/on/off` as booleans, timestamps and base64. When *writing* with `&str` values, yaml-edit quotes risky strings: I observed `set("newkey","no")` gives `'no'`, `set("newratio","1e3")` gives `'1e3'`, `set("s","42")` gives `'42'`, `set("t","true")` gives `'true'`, but `set("due","2026-10-01")` and `set("id","WF-4")` are written plain. That is consistent with "strings that would resolve to a non-string under 1.2 core get quoted". Dates plain is fine for 1.2 readers but would be timestamps to a 1.1 reader.
6. serde_yaml emitting the *string* `"no"` writes `no` bare, and `"yes"` writes `yes` bare (only `1e3` and `0755` got quoted). Fine for 1.2 readers; a 1.1 reader (PyYAML, Ruby Psych) would read false/true. **[observed]**
7. yamlpatch `Replace` with the string `no` wrote `id: no` unquoted (observed), whereas `Add` of the string `1e3` wrote `'1e3'`. Inconsistent quoting between operations.

---

## 3. Editing in place: what I actually ran

### 3.1 Parse then reserialise (all serde-family crates)

Input had a top comment, a trailing comment, `status: "no"`, a flow list, a block list with an interleaved comment, a `|` block scalar, a flow mapping and a single-quoted string. After `from_str::<Value>` then `to_string` (identical output from `serde_yaml` and `yaml_serde`) **[observed]**:

- all comments dropped (top, trailing, "keep me")
- `id:   WF-3` respaced
- `status: "no"` became `status: no` (quote style lost)
- `ratio: 1e3` became `ratio: 1000.0` (**user text changed**)
- `tags: [a, b]` became a block list; `unknown_field: {x: 1}` became a block mapping
- `zzz: 'single'` became `zzz: single`
- key order preserved (indexmap-backed), block scalar preserved.

So key order and unknown-field survival hold, but comments, style and scalar text do not. This fails the design's "preserve existing YAML style where possible" for every write. Fine for reads.

### 3.2 yaml-edit 0.3.2 [observed]

Probe: full mixed document, edits via `Mapping::set`, `Sequence::push`, `remove`.

Worked (exact output checked):
- No-op parse then `to_string` returns byte-identical text.
- `set` on an existing scalar replaces only the value token; `# trailing` comment and the spacing `id:   ` kept; top comment kept.
- New keys appended at end; unknown fields (`unknown_field: {x: 1}`) untouched; key order preserved.
- `push` on a block list keeps `# keep me` and indent; `push` on a flow list `[a, b]  # flow` gives `[a, b, c]   # flow`.
- Doc without trailing newline: appended key gets a newline correctly.
- `Document::from_str` refuses input with a leading comment ("Input contains stream-level comments outside the document. Use YamlFile::from_str()"), so use `YamlFile` and take `documents().next()`. A frontmatter block that starts with a comment is plausible, so this matters.

**Failed (real defects, reproduced at the 0.3.2 tag, which is the HEAD of the repo I cloned):**
- **Replacing a block scalar (`|` or `>`) with any value fuses the next line onto it and corrupts the document.** `a: 1\nnote: |\n  x\n  y\nb: 2\n` with `set("note","new")` gives `note: newb: 2`; reparsing fails ("mapping values are not allowed in this context"). Same for `ScalarValue::literal("l1\nl2\n")`, which gives `  l2b: 2`. Replacing multi-line double-quoted, multi-line plain, and block-list values with a string worked.
- Workaround that is order-lossy: `remove("note")` then `set("note", ...)` works but appends at the end. An order-preserving workaround (insert_before/insert_after the neighbour, then remove) is plausible but I did not test it.
- CRLF input: appended line was written with bare `\n` (`"a: 1\r\nb: 2\r\nc: x\n"`), i.e. mixed line endings.
- Setting a key that appears twice edits the first only, leaving the document with duplicate keys (which serde_yaml then refuses to parse). Whether typdoc should even accept duplicate-key frontmatter is a separate question.
- When the value is replaced the old quote style is not kept: `status: "no"` set to `"done"` became plain `done` (fine and correct here, but it means "keep existing quote style" is not automatic).

Not tested: anchors, tags, merge keys, `? ` complex keys, Windows-1252/BOM input, very large files.

### 3.3 yamlpatch 1.29.0 (pinned; 1.30.x did not build on rustc 1.96) [observed]

Same document; one patch per run.
- `Replace` scalar (`status`), date string, and block scalar (`note` -> `one line`): correct, comments and neighbours intact. (So it handles the case yaml-edit corrupts.)
- `Append` to a block list: correct, kept the interleaved comment.
- `Add` new key: appended at end, string `1e3` written as `'1e3'`.
- `Replace` string `no` gave bare `no` (see 2.2 point 7).
- `Replace` of the flow list `tags: [a, b]` with a 3-element sequence returned `Err(Query(InvalidInput(1, 1)))`. I did not diagnose whether this is my `Route` construction or a genuine limitation; **[unverified cause]**.
- Toolchain and dependency weight (tree-sitter, rustc 1.97 for latest) makes it a heavier dependency for a small CLI than the value it adds.

### 3.4 saphyr `MarkedYaml` spans (for a splice-based hybrid) [observed]

Idea: parse with spans, then replace just the value's byte range in the original text. Findings on saphyr 0.1.0:
- `Marker::index()` doc says bytes, the struct field doc says chars; **it is chars**. With `title: สวัสดีชาวโลก` (Thai) the marker for `todo` on a later line was 28 while the byte offset was 52. Any splice must convert char index to byte offset.
- Span ends are not reliable. Single-line values with a trailing comment returned a span *including* the comment (`"no" # q`); a flow sequence span omitted the closing bracket (`[a, b`). Splicing on those spans would corrupt text. Could be fixed upstream soon (0.1.0 shipped today) but I would not build on it now.

### 3.5 Line-based surgical editing as a fallback

Sound only under conditions. What I verified about why a naive approach breaks (serde_yaml/libyaml as the oracle, **[observed]**):

- A line starting at column 0 inside a multi-line double-quoted or single-quoted scalar is a continuation, not a key: `a: "x\nb: y"\nc: 1` parses as `{a: "x b: y", c: 1}`. Same for a multi-line flow sequence: `a: [1,\nb: y]` parses `b: y` as a mapping *inside* the list.
- A block scalar's body must be indented deeper than the key; a column-0 line inside it is an error, so block-scalar extent = following lines that are indented (or blank), and trailing blank lines/comments after it are ambiguous with respect to chomping (YAML 1.2.2 section 8.1: "the content indentation level" is set by the first non-empty line; comments at a lower indent end the scalar).
- Comments: "Comments must be separated from other tokens by white space characters" (YAML 1.2.2 section 6.6). So `# ` inside a quoted string or after `#` without preceding space is not a comment; a `#`-detector must be quote-aware.

Edge cases and what handling each costs:

| Shape | Needed to edit safely | Cost |
|---|---|---|
| `key: plain  # c` | Split value from trailing comment; keep the spacing before `#`. Plain scalars cannot contain ` #`. | Small. |
| `key: "quoted"` / `'quoted'` | Find the closing quote (with `\"` escapes / `''` doubling); decide whether to keep the original quote style for the new value. Needs an own escaper. | Small to medium. |
| Multi-line quoted or plain scalar | Track quote state across lines; continuation lines may start at column 0. | Medium. Easiest to refuse to edit such a key (error) while preserving it verbatim. |
| Block scalar `|` `>` (with `-`, `+`, indent indicator) | Extent detection by indentation; chomping affects trailing blank lines. | Medium. Refuse-or-handle decision. |
| Block list (`- a`), incl. interleaved comments and indent style (indented vs. flush with key) | Detect indent of the existing items and match it on append; comments and blank lines between items belong to the list. | Medium. |
| Flow list `[a, b]` on one line | Token-level append/remove with quote and nesting awareness; keep `, ` spacing. | Small to medium. |
| Flow collection spanning lines | Same as multi-line quoted: bracket depth tracking across lines. | Medium. Refuse. |
| Anchors `&x`, aliases `*x`, tags `!!str`, merge `<<` | Preserve on unrelated keys (free with line editing); refuse edits to keys carrying them. | Free to preserve, refuse to edit. |
| Duplicate keys | Decide which to edit or reject the document. | Policy, not code. |
| CRLF | Detect the file's newline and reuse it. | Small. |
| Quoting new values | Reimplement "does this string need quotes": `no`, `1e3`, `WF-3` (no), leading `-`/`?`/`:`/`#`, `: `, ` #`, leading/trailing space, numbers, dates if you want them stable across 1.1 readers. | Small but easy to get subtly wrong; yaml-edit does this and a test-suite for it is what you would be rewriting. |
| Key not present | Append at end of frontmatter, or after a chosen neighbour. | Small. |

Mitigation that makes both this approach and yaml-edit safe: after any write, reparse the new frontmatter with the real parser and assert that every key other than the edited ones has an *equal* parsed value (compare against the pre-edit parse), and that the edited key has the intended value. That turns silent corruption (like the yaml-edit block-scalar bug) into a refused write. I verified that this check would have caught the yaml-edit defect (the corrupted output failed to reparse).

---

## 4. Maintenance summary (all as of 2026-09-19)

| Crate | Verdict | Evidence |
|---|---|---|
| `serde_yaml` 0.9.34 | **Archived, deprecated. Do not use.** | GitHub `archived: true` (pushed 2024-03-25); README "no longer maintained"; crates.io version suffix `+deprecated`. Still used at huge scale (407M downloads), and it works, which is why it tempts. |
| `yaml_serde` 0.10.7 | Maintained, recent releases (Jan, Mar, Aug 2026), owned by the `yaml` GitHub org. Young repo (created 2025-11-25); relies on `libyaml-rs`. | crates.io API; repo README. |
| `serde_yaml_ng` | Repo pushed 2025-09; no release since 2024-05. Lightly maintained. | crates.io, GitHub. |
| `serde_norway` | No release since 2024-12. | crates.io. |
| `serde_yml` | Dead, unsound advisory. | RUSTSEC-2025-0068; repo archived. |
| `yaml-rust2` | Maintenance-only by its own README; still releases (0.13.0 on 2026-09-11). | README, crates.io. |
| `saphyr` | Active; reached 0.1.0 today; spans and emitter quirks noted above. | crates.io, changelog. |
| `serde-saphyr` | Very active (1.3.0 on 2026-09-16, 7M downloads); pure Rust. | crates.io. |
| `yaml-edit` | Active (weekly releases, single dominant maintainer), pre-1.0, 10 stars, 14 dependents; recent fuzz-driven bug-fix wave. | GitHub API, crates.io. |
| `yamlpatch`/`yamlpath` | Active (zizmor, 6.5k stars), but internal-tool crates versioned with the CLI, rustc 1.97 MSRV in latest. | crates.io, GitHub. |

---

## 5. What I could not verify

- Behaviour of `serde_yaml_ng` and `serde_norway` was **not run**; they are `serde_yaml` forks on the same libyaml lineage, so I expect the same typing as serde_yaml, but that is an inference.
- No YAML 1.1 parser (PyYAML, Ruby Psych) was run. The 1.1 readings of `no`, `1e3`, dates and `0755` are derived from the 1.1 type regexes in the spec drafts, not observed.
- Why yamlpatch's `Replace` on a flow list failed (`InvalidInput(1, 1)`): my `Route` usage or a real limitation is undetermined.
- yaml-edit behaviour on anchors, tags, complex keys, BOM, huge input, and whether an order-preserving workaround for the block-scalar bug works.
- saphyr span bugs were observed on 0.1.0 (published today); whether they are known upstream or already fixed on master was not checked.
- The YAML 1.2.2 grammar was not walked to prove *why* a column-0 line inside a multi-line quoted scalar at top level is legal; I relied on libyaml (via serde_yaml) accepting it. Another parser might differ.
- docs.rs pages were not fetched; API facts come from the crate source published on crates.io (which is what docs.rs renders) and the upstream READMEs.
- `serde-saphyr` README states it depends on `granit-parser` for comments ("As granit-parser now supports comments"), not `saphyr-parser`; I did not trace the relationship between the two parsers.
- Whether the typdoc design tolerates a 1.1 reader seeing bare `no` or dates. That is a design decision, not a fact I can look up.

---

## 6. Recommendation

**Read with `yaml_serde` into typed `String` fields; write with `yaml-edit` (pinned to an exact 0.3.x) behind a mandatory reparse-and-compare guard, and restrict the write operations to what typdoc needs (set scalar, append/remove list item, add key).**

Why this and not the others, using the two facts it depends on most:
1. **Reads must be typed as `String`, not read through a `Value` tree.** Every crate turns `1e3` into a number and `1.10` into `1.1` when untyped, while all serde-family crates return the exact text `"1e3"`, `"1.10"`, `"no"`, `"0755"`, `"2026-09-19"` and `"2026-09-19T14:30:00+07:00"` for a `String` field. yaml_serde is the maintained (YAML-org) drop-in for the archived serde_yaml.
2. **No serialiser round trip can meet the design (comments, quotes, flow style and scalar text are lost or rewritten), and yaml-edit is the only crate whose stated model matches it**, and in my probes it preserved comments, spacing, key order, unknown fields, flow and block lists and quoting of risky strings, with a byte-identical no-op round trip. Its one reproduced corruption (block-scalar replacement) is caught by the reparse guard and can be avoided by refusing to edit block-scalar keys, which typdoc's schema (strings, numbers, lists) should not need.

Implementation notes that follow from the evidence: fence-split by hand; use `YamlFile` not `Document` (leading comments); always write string values via `ScalarValue::string` / `&str` (never `parse`) so `no`, `1e3` and `WF-3` come out quoted or plain as the tool decides; normalise line endings yourself if CRLF files matter; pin `=0.3.x` and re-run a fixture suite on every bump.

### Runner-up: own line-based editor (state-machine scanner, refuse unsupported shapes) plus the same reparse guard

Cost of choosing it instead:
- You own a scanner that must track quote state, flow-bracket depth and block-scalar indentation across lines. I showed the shortcut ("a key starts at column 0") is wrong for multi-line quoted and flow values. Realistically a few hundred lines plus a fixture suite, and it will keep growing edge cases (indent detection for lists, comment attachment, chomping).
- You own the "does this string need quotes" rule, which yaml-edit already implements and which is where YAML 1.1/1.2 differences bite.
- You would decide up front to refuse edits on multi-line quoted scalars, multi-line flow collections, block scalars, and anchored/tagged nodes (they are preserved untouched, just not editable).
- In exchange: zero new dependencies, no pre-1.0 API churn, no dependency on one maintainer, full control of newline and quote style, and by construction it can never rewrite text it did not touch. It is also the right fallback if yaml-edit is abandoned or a fuzz-found defect appears in a shape typdoc really uses.

If a third path is wanted, `yamlpatch` handled the block-scalar replace correctly where yaml-edit did not, but the tree-sitter dependency, the rustc 1.97 requirement on current releases, the zizmor-lockstep versioning and the unexplained flow-list failure make it a worse fit than either option above.

### Sources

- https://github.com/dtolnay/serde-yaml (README, archived flag via https://api.github.com/repos/dtolnay/serde-yaml)
- https://crates.io/api/v1/crates/{serde_yaml,serde_yml,serde_norway,serde_yaml_ng,yaml_serde,yaml-rust2,saphyr,saphyr-parser,serde-saphyr,yaml-edit,yamlpath,yamlpatch,marked-yaml,gray_matter,yaml-front-matter} (versions, dates, dependencies, reverse dependencies)
- https://github.com/yaml/yaml-serde (README: actively maintained fork by the YAML organization)
- https://github.com/saphyr-rs/saphyr (README, `saphyr/CHANGELOG.md`)
- https://github.com/Ethiraric/yaml-rust2 (README: maintenance-only notice; "type specifiers" sentence is under Security)
- https://github.com/jelmer/yaml-edit (README.md, DESIGN.md, TODO.md, issues/PRs list, contributors) and its published source `src/scalar.rs` (`classify_plain`, `auto_detect_type`, `parse`, `string`)
- https://github.com/zizmorcore/zizmor/tree/main/crates/yamlpatch and `.../yamlpath` (READMEs)
- https://github.com/bourumir-wyngs/serde-saphyr (README: boolean handling, Commented<T>, comments)
- https://github.com/rustsec/advisory-db (`crates/serde_yml/RUSTSEC-2025-0068.md`, `crates/yaml-rust/RUSTSEC-2024-0320.md`, `crates/serde_yaml/` listing)
- https://yaml.org/spec/1.2.2/ (sections 6.6, 7.3.3, 8.1.1.1, 10.3.2, 10.4)
- https://yaml.org/type/bool.html, https://yaml.org/type/float.html, https://yaml.org/type/int.html, https://yaml.org/type/timestamp.html (YAML 1.1 type drafts)

Probe code lives only in the session scratchpad (not in the repo): `probe/` (typing, round trip, yaml-edit, spans) and `probe2/` (yamlpatch).
