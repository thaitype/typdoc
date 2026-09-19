# 4: What are the slug rules for headings and for `{slug}` in filenames?

Type: wayfinder:grilling
Status: open
Blocked by: 3

## Question

`toc` returns a `slug` that "a `#heading` link must use", and `match` templates can contain `{slug}` derived from the title. The design fixes neither algorithm.

Decide: match GitHub's heading-anchor algorithm or define our own; how duplicate headings are disambiguated; and how non-ASCII text is handled, since titles and headings here are often Thai (transliterate, keep as-is, or reject). The `{slug}` for filenames may differ from the heading slug, but if it does, say why. Amend `docs/design.md`.

Input from [research 3](../_research/3-markdown-body-parsing.md): a hand-written GitHub-style slugger matched GitHub's `/markdown` API on 29 headings, including Thai and duplicates, and `comrak`'s `Anchorizer` matched on 20 of 20. Heading-text extraction (image alt text, soft breaks) is where a slugger goes wrong.

## Answer

