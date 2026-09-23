# 2: Is a doc-table generated-vs-committed comparison test inside the no-markdown-as-spec rule?

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

**Decider: Mild.** Answered directly, 2026-09-23, relayed by Aria. Recorded as Mild's decision
(M-1 in `.chief/story-3/brief.md`'s "Mild's decisions" section), not Aria's and not mine.

## Question

Once the spec (rule ids, command names, exit codes, frontmatter losses) lives in a hand-edited
JSON file (ticket 3), the tables in `docs/design/design.md` are generated from it. Mild's rule is
absolute: no program reads markdown to extract spec as machine data, and comparing counts as
importing. Does a test that compares a freshly-generated table against the one committed in
`design.md` (to catch a stale doc) fall inside that rule, since one side of the comparison is
markdown — or is it outside the rule because the JSON file is the actual source of truth and the
markdown table is only being checked as generated output, never read as spec?

## Answer

**M-1, Mild's own words:** *"M1 ถือว่าผิดกฏ ผมจะยกเลิกใช้ไฟล์ design.md โดยให้กฏที่ code ใช้ ref ให้อ่านจาก
json ครับ · หมายความว่า design.md สามารถ outdate ได้คับ"*

Comparing a generated table against `design.md` **violates the rule** — out entirely, no
generator, no compare against any markdown, in either direction. Code stops referencing
`design.md` as spec altogether; instead it reads its spec from JSON. `design.md` itself is
allowed to go stale — nothing keeps it in sync any more.

This reshapes the core of the story past the original "generate a table into design.md" plan
(ticket 3's format-JSON answer stands, but see ticket 5 for where that JSON now lives and how
code reads it — not a bare file, but the JSON-only body of a typdoc-managed document via a
central helper, M-6).
