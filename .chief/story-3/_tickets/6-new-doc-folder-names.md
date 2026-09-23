# 6: Folder names for the two new document kinds (M-4)

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

**Decider: Mild.** Answered 2026-09-23, relayed by Aria (M-4 in `.chief/story-3/brief.md`).

## Question

Mild's decision (M-1, see ticket 2) replaces `design.md`-as-spec with two kinds of typdoc-managed
documents:
1. Prose rules code must not read → `docs/design/<xxxx>/*.md`.
2. Structured rules code reads (JSON-only body, via the central helper, M-6) → `docs/design/<yyyy>/*.md`.

What are `<xxxx>` and `<yyyy>`? Aria has proposed `spec`/`SPC` and `catalog`/`CAT` but this is not
decided.

## Answer

**M-4, Mild's own words:** *"ชื่อ folder เห็นด้วย ทั้ง spec, catalog ครับ ส่วน ใน catalog ให้มี field
สำหรับบอก body type เช่น json เพื่อให้ helper รู้ว่าต้อง validate JSON ไม่ใช่ ดูจาก path ว่าต้อง
validate หรือไม่"*

- Prose (code must not read) → `docs/design/spec/`, code `SPC`.
- Structured (JSON-only body, code reads via the central helper) → `docs/design/catalog/`, code
  `CAT`.
- **Added requirement, not in the original fog:** a catalog document carries a frontmatter field
  declaring its body type (e.g. `json`). The central helper decides whether and how to validate
  the body **from that field, never from the path** — ticket 5 must design the helper around this
  field, not around a folder-name convention.
