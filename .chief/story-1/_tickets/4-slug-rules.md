# 4: What are the slug rules for headings and for `{slug}` in filenames?

Type: wayfinder:grilling
Status: resolved
Blocked by: 3

## Question

`toc` returns a `slug` that "a `#heading` link must use", and `match` templates can contain `{slug}` derived from the title. The design fixes neither algorithm.

Decide: match GitHub's heading-anchor algorithm or define our own; how duplicate headings are disambiguated; and how non-ASCII text is handled, since titles and headings here are often Thai (transliterate, keep as-is, or reject). The `{slug}` for filenames may differ from the heading slug, but if it does, say why. Amend `docs/design.md`.

Input from [research 3](../_research/3-markdown-body-parsing.md): a hand-written GitHub-style slugger matched GitHub's `/markdown` API on 29 headings, including Thai and duplicates, and `comrak`'s `Anchorizer` matched on 20 of 20. Heading-text extraction (image alt text, soft breaks) is where a slugger goes wrong.

## Answer


Resolved with the human, 2026-09-19. `docs/design.md` is amended (Heading anchors paragraph under Refs, `{slug}` removed, `toc` and `body.anchors` reworded).

1. **Algorithm: GitHub's, not our own.** So a link that passes `validate` also works on GitHub. Text is lowercased, punctuation (except `-` and `_`), symbols and other non-letter, non-digit characters are deleted, spaces become `-`.
2. **Non-ASCII: kept as written.** Thai vowels and tone marks stay; nothing is transliterated.
3. **Duplicates: GitHub's collision-aware suffix.** `Dup`, `Dup`, `Dup 1` give `dup`, `dup-1`, `dup-1-1`. Every heading counts, including those in block quotes and list items (not fenced code), so numbering matches GitHub.
4. **Empty slug: not special.** Probing GitHub's `/markdown` API on 2026-09-19 showed it treats `""` like any slug: first gets an empty id, the next `-1`, `-2`, `-3`. This explains the oddity research 3 left open. typdoc reproduces it with no extra code; the first empty-slug heading cannot be linked to.
5. **`{slug}` in file names: removed from v1.** The human's steer: a coded document has its key and needs no slug; a document without a code is named by its creator through `typdoc new <path>`, which is already a user-chosen name. So typdoc never derives a file name from a title. This removes the edge cases a derived name would need rules for (empty slug, the 255-byte name limit that Thai reaches at about 85 characters, Windows reserved names, names left stale after a retitle, a `--slug` override flag). Adding a `{slug}` placeholder later would not break anything. Cost accepted: a repo whose files are already named `KEY-n-some-title.md` cannot be matched by a template; migrating existing registries is out of scope for this story.
6. **Links: fragment is percent-decoded, then compared case-insensitively.** Browsers copy Thai anchors percent-encoded, and GitHub compares anchors without regard to case (stated by the human; not verified from the terminal).

**Defaults chosen by the agent, not asked (veto welcome):** heading text is the text and code spans, with image alt text, line breaks and inline HTML contributing nothing (research 3, section 4.5); a `%` not followed by two hex digits is kept as written; `toc` lists headings inside block quotes and list items; the exact character classes are left to the contract's fixtures, generated from GitHub's renderer.

Corrected mid-grill: NFC/NFD normalisation is not a Thai problem (Thai has almost no canonical decompositions); it would only affect accented Latin letters, and only if a name were derived, which it no longer is.
