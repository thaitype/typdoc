# 11: What unit is `col` in `path:line:col`?

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

## Question

`validate` prints `path:line:col` and `--json` returns the same fields. The parser gives byte offsets; a column can be counted in bytes, Unicode scalar values (chars), UTF-16 code units, or grapheme clusters. They differ on non-ASCII text: on a Thai line, research 3 measured the same link at column 5 by chars and column 11 by bytes.

Decide the unit, given that documents here are often Thai and that findings are meant to be clickable in editors and consumed by agents. Which convention common editors use for `path:line:col` was not verified in research 3; check it before deciding. Amend `docs/design.md` so the unit is stated once.

## Answer


Decided 2026-09-19. `docs/design.md` is amended (Output paragraph under Validation rules).

**Decision.** `line` and `col` are 1-based. `col` counts Unicode scalar values (chars): a Thai consonant, a Thai vowel or tone mark, an emoji and a tab each count as 1. `line` counts from the top of the file, frontmatter included (same as `toc`). No byte offset in `--json` in v1; adding one later is not breaking.

**Checked before deciding (primary sources, fetched 2026-09-19):**

| Tool | Unit of column | Source |
| --- | --- | --- |
| Language Server Protocol 3.17 | UTF-16 code units by default, mandatory for servers; utf-8 and utf-32 negotiable; zero-based | LSP specification |
| rustc JSON diagnostics | Unicode scalar values, 1-based, with separate byte offsets | doc.rust-lang.org/rustc/json.html |
| ripgrep `--column` | bytes, 1-based ("does not try to account for Unicode") | ripgrep `defs.rs` |
| GCC | display columns by default since 11.1, `byte` optional, origin 1 | GCC diagnostic options |
| VS Code `--goto file:line:char` | not stated in the docs | code.visualstudio.com |

Not verified: Vim, Emacs, and what unit VS Code really uses. Nothing here proves any editor lands on the right character after an emoji.

**Why chars.** Thai is inside the BMP, so chars and UTF-16 code units agree on every Thai line; only characters outside the BMP (emoji) differ, by one per such character. Bytes are wrong for the team's main content (research 3: the same link is column 5 by chars and 11 by bytes). Graphemes and display width need tables and no tool in the survey needs them. An agent can slice with the number directly.

**Contract requirements.** Count scalar values (in Rust, `chars()`, never byte length). Counting by a string's UTF-16 length would be wrong in the same way. Tests must include a line with Thai and a line with an emoji, with the expected `col` written in the fixture.

**Default:** `col` marks where the offending element starts (for a link, its `[`).

**Known limit, stated in the design:** after an emoji, an editor that counts UTF-16 code units will be one position off on that line.
