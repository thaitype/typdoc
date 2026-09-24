# 33: Rewrite the README and user docs for people, and ship an agent skill

Type: implementation
Status: resolved
Blocked by: None

**Mild's decisions, 2026-09-24, built by Aria (not handed to Mina, at Mild's request).**

- *"ต่อไปผมอยากทำ SKILL สำหรับ typdoc"* — grilled: the skill serves an agent working in an existing
  project (A); it describes and does not mandate; it is self-contained with `references/`, and names
  the repository URL only as a fallback; it lives in `skills/typdoc/`, versioned with the binary, and
  its first step is `typdoc --version`; references are hand-written. *"ทำ story-3 ไปเลย ไม่ต้องขึ้น
  story ใหม่"* — it lands in story 3.
- *"ฝากเขียน README และ doc ใหม่ … แบบไม่แก้ไขของเดิมเลย"* — rewrite from blank, replacing the old
  files (option A). Outline agreed page by page before writing (*"ทำไมไม่ถามผมถึง Outline ก่อน"*):
  README (motivation, install with the skill, concepts, how it works, try it, commands, docs map,
  status); Diátaxis split into tutorial / how-to / reference / explanation; a rewritten
  `development.md`.

## Done

- `skills/typdoc/SKILL.md` + `references/{commands,query,validation,exit-codes,project-layout}.md`.
- `README.md`; `docs/getting-started.md`; `docs/how-to/{adopt-an-existing-folder,move-and-rename,use-namespaces,link-projects,check-in-ci}.md`;
  `docs/reference/{commands,queries,project-files,validation}.md` (replacing `docs/commands.md`,
  `docs/projects.md`); `docs/explanation/{how-typdoc-sees-files,keys-and-numbers}.md`; `docs/development.md`.
- Every example was run against a release build of this branch; outputs are real. Behavior fixed
  by tickets 29–32 was re-verified on `09c7b7b` and the docs/skill updated to match.
- Found while writing, and routed to Mild as numbered items: M-18..M-20 (fixed, tickets 30–32),
  M-21 (M-18's reach) and M-22 (`mv` does not list mentions in `unrewritten`), both to fix.
