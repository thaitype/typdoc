# Ticket 6 Report

## Ticket
Body parsing on `pulldown-cmark` with the frontmatter cut first, a line and column map counted by the design's line rule, a GitHub-style slugger, and `typdoc toc <path> [--depth n] --json`.

## Outcome
done

## Decision
Nothing the design or the contract asks for turned out to be impossible. The design does not say the following, so each is a reading that a later ticket may change, with the doubt that remains.

- **The line map walks the text one whole line at a time.** A line holds at least its ending, so the length of a line is never zero and the walk always reaches the end of the text; the length is `None` only for an empty remainder, which begins no line. An earlier shape of this loop tested the remainder for emptiness with the wrong bound and stepped by zero at the end of every text, so it never ended and grew until the machine had no memory left. The shape now makes the step's own non-zero length the reason the walk ends, rather than the comparison in the condition. `an_empty_text_has_no_line` is the sharpest case: under the old shape every input diverged, that one included.
- **A final line with no line ending is a line; a line ending at the end of the text begins no other.** `"a\nb\n"` is two lines and `"a\nb\n\n"` is three.
- **`--depth` accepts any level from 1 upward**, while its help text names 1 to 6. `--depth 7` lists every heading and exits 0; `--depth 0`, a negative number and text exit 1. Doubt: the help text promises a range the parser does not enforce, and the design names no upper bound, so refusing 7 would be a choice made here rather than one the design asks for.
- **`end` is a property of the document.** `--depth 2` on a file whose deepest heading closes at line 7 still gives `end` 7 for the headings it lists, which is what the design asks for and is checked by running.
- **The character classes of a slug are pinned by the fixtures, not by a rule written in code.** `fixtures/valid/body/slugs.md` and its golden keep the renderer's answers for the cases that a hand-written rule gets wrong: a tab is deleted rather than turned into a hyphen, a line break inside a setext heading is deleted (`line-oneline-two`), a no-break space is deleted, hyphens at either end are kept, and the variation selector U+FE0F outlives the emoji it follows (`️-emoji--x`).
- **Collision suffixes skip results already taken.** `Dup`, `Dup`, `Dup 1`, `Dup` give `dup`, `dup-1`, `dup-1-1`, `dup-2`, as the design's own example asks.
- **A heading with nothing under it has `end` equal to `line`,** and the last section runs to the last line of the file.
- **A block that is never closed exits 2 for `toc` as it does for `get`,** and a block that is closed is cut without being read as YAML, so `toc` answers for a document whose frontmatter would not parse.

## Notes
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (303 passed and 1 ignored, from 254) and `scripts/check-public-text.sh` with the names list, all green.
- Every cargo command for this ticket ran under a memory ceiling (a 6 GB scope with swap refused), after the ceiling was shown to work by letting a program that allocates without stopping be killed inside it. This is the practice from now on: the machine is shared, and the line-map loop above had already exhausted it once.
- Re-run by hand, each planted, red, removed: `std::env::var` in `body.rs` (library code) and `std::fs::write` in `tests/headings.rs` (test code), both refused by the bans in `crates/typdoc-core/clippy.toml`. The two were first planted together, which proved nothing about the test-code ban, because the library failed to compile before the test was reached; the test-code ban was then planted alone and shown red at `tests/headings.rs:225`.
- `unicode-general-category` is added to the stack list: the slugger deletes characters by general category, which the standard library does not expose.
- `fixtures/.gitattributes` turns off line-ending conversion for every fixture, so the `\r\n`, lone `\r` and no-final-newline files keep the bytes they were written with. A test reads those bytes and fails if they change.
- `toc` is removed from `UNIMPLEMENTED_COMMANDS`.
- Not shown: no fixture has a heading deeper than level 4, so levels 5 and 6 are exercised only by `--depth`; and the text form of `toc` (without `--json`) still exits 1, as for `get`.
