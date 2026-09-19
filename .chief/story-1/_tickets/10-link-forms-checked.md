# 10: Which link forms does `body.links` check?

Type: wayfinder:grilling
Status: open
Blocked by: None (can start immediately)

## Question

The design counts only `[text](path)` and `[text](path#heading)`. Research 3 found that a CommonMark parser sees more forms, and that one common form is silently invisible:

- `[t](my file.md)` (an unescaped space) is **not a link** to CommonMark, so a checker never sees it, yet a human reads it as one. Needs `<my file.md>` or `%20`.
- Reference-style links (`[t][ref]` with `[ref]: path`) and images (`![alt](x.png)`).
- Autolinks (`<https://...>`), which the design already skips as URL-scheme links.

Decide which forms are checked, which are ignored, and whether a link that looks intended but is not a link (the unescaped space) is reported by some rule rather than passing unseen. Given the team's experience that a check which cannot see its input looks exactly like a pass, prefer the loud option unless there is a reason not to. Amend `docs/design.md`.

## Answer

