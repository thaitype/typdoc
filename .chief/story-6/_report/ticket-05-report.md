# Ticket 05 Report

## Ticket
Every comment in the rest of `typdoc-core` (refs, links, validation, queries, schemas, the index,
config, arguments, documents, imports, namespaces, errors and their tests) reviewed, and the
design those comments cite moved into `docs/design/`.

## Outcome
done

## Notes
- Comment lines in scope: 1,341 before, 658 after. Eight comments the code clearly contradicted
  are corrected; eight mismatches with no clear answer are listed in the PR.
- Design moved: new SPC-17 (collections); sections added to SPC-1, SPC-2, SPC-3, SPC-12, SPC-13,
  SPC-14 and SPC-15. 75 lines deleted from `docs/migrating-design/design.md`, three
  cross-references repointed.
- Two `QueryError` variants keep a one-line doc comment: without one, `cargo fmt` lays the
  variant out differently, which would be a code change.
- Also in this PR, as its own commit: three strings that pointed at history (an assertion message
  in `refs.rs`, one in `namespaces.rs`, and the fixed host name in `tests/common/mod.rs`) no
  longer do; each meaning is unchanged. The proof reports those three and nothing else.
- Two of the listed mismatches are fixed in this PR: SPC-4 no longer says a `frontmatter.parse`
  finding has a position (a docs commit), and a `refs.rs` test named as if the import form were
  not read is renamed for what it asserts (its own commit, in the proof). The other six are
  deferred by decision.
- The PR's citation table has one row per hunk the citation audit lists (75).
- Gates at head: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `scripts/test.sh` (1057 passed, 0 failed, 1 ignored), `typdoc validate` at every commit.
