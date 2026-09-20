# 10: Which link forms does `body.links` check?

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

## Question

The design counts only `[text](path)` and `[text](path#heading)`. Research 3 found that a CommonMark parser sees more forms, and that one common form is silently invisible:

- `[t](my file.md)` (an unescaped space) is **not a link** to CommonMark, so a checker never sees it, yet a human reads it as one. Needs `<my file.md>` or `%20`.
- Reference-style links (`[t][ref]` with `[ref]: path`) and images (`![alt](x.png)`).
- Autolinks (`<https://...>`), which the design already skips as URL-scheme links.

Decide which forms are checked, which are ignored, and whether a link that looks intended but is not a link (the unescaped space) is reported by some rule rather than passing unseen. Given the team's experience that a check which cannot see its input looks exactly like a pass, prefer the loud option unless there is a reason not to. Amend `docs/design.md`.

## Answer


Decided 2026-09-19. `docs/design.md` is amended (Body links, new paragraphs Reference definitions and Text that looks like a link but is not, `body.links` row, `mv`).

**Forms.**

| Form | Decision |
| --- | --- |
| Inline `[t](path)`, `[t](path#heading)` | Checked |
| Image `![alt](path)` | Checked as a link (design already said to put images in `ignore` to skip them) |
| Reference-style `[t][ref]`, `[ref][]`, `[ref]` | Checked; counted in `$body` |
| Autolink `<https://…>` | Skipped (URL scheme) |
| `[t][ref]` with no definition | Not reported: plain text to CommonMark, and the shape is common (`a[0][1]`). Reporting it was rejected because of the false positives |
| Definition `[ref]: path` | Every line checked, used or not, reported once at the definition with the number of uses; never reported at each use. Unused ones are not in `$body` (default) |
| Label defined twice | Later one reported (`already defined at line N; this definition is ignored`) even when the target is the same; its target is not checked; `mv` rewrites only the active definition. Labels compared as CommonMark does (case fold, whitespace collapse). `RefDefs` keeps only the first definition, so this needs the same line scanner as the next row |
| Looks like a link but is not (`[t](my file.md)`, `![t](my pic.png)`, `[ref]: my file.md`) | Reported under `body.links` when outside code, no URL scheme, and after removing a trailing title (`"…"`, `'…'`, `(…)`) the destination ends with a file extension (any, not only `.md`), optionally with `#anchor`. Message says to use `<…>` or `%20`. The rule is not limited to `.md`, because `body.links` checks every relative link and a title would hide a `.md` ending |

**Resolving.** The checker percent-decodes destinations and reads `<…>`. `mv` writes a new path in the link's original form and uses `<…>` when the new path has a space and the link used neither `<…>` nor `%20`.

**Defaults:**
- Extension means `.` plus one to eight ASCII letters or digits with at least one letter (so `version 1.2` is not reported; `ask Mr.Smith` still is, a known false positive).
- Namespace names and import aliases (ticket 13) do not count as URL schemes for this check.
- Beyond a space, `mv` also uses `<…>` for a new path containing `<` or unbalanced parentheses.
- A rejected definition's finding does not state a use count.
- A `col` points at `[` (or `!`, or the definition's `[`), following ticket 11.

**Checked in research 3:** `Parser::reference_definitions()` returns `RefDefs` with `LinkDef.span`, so unused definitions are visible and reportable at their line; it keeps only the first definition of a label; definitions in code are not definitions.

**Known gap, stated in the design:** a rejected definition cannot say how many uses it breaks, since its uses read as plain text.
