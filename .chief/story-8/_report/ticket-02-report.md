# Ticket 02 report: changelog and docs

Fixed point: 797f2de. Standard mode.

- `CHANGELOG.md`: `## [Unreleased]` added above `[0.4.0]`, with Fixed (the macOS stale report, the
  two new endings) and Changed (`acquire` and `Env` in the `typdoc-core` library).
- `templates/skills/typdoc/references/exit-codes.md`: the five endings, grouped by what to do;
  `skills/` rendered, and `render_skills.py --check` passes.
- `docs/explanation/keys-and-numbers.md`: when typdoc cannot tell, it says so rather than guess.
- `.chief/project.md`: `libc` is an ordinary dependency of the binary crate, and why.
- `docs/reference/` lists no endings, so it is unchanged.

Checks: the four gates; `test_extract_release_notes.py` and `test_render_skills.py` pass with the
new `[Unreleased]` heading.
