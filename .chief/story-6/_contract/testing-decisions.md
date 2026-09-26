# Testing Decisions

No behavior changes, so no new tests. What is tested is that nothing but comments changed and
that the existing suite still passes.

- **Comment-only proof** (see the contract): expanded-source comparison per touched compile
  target, shown able to fail on a planted code change before it is trusted.
- **Existing suite:** `scripts/test.sh` locally, and the ubuntu and macOS CI jobs. A red test
  after a comment change means the change was not comment-only (or moved a line a test
  depends on, such as a `trybuild` expectation) — investigate, do not update an expectation to
  match without saying why.
- **Design documents:** `typdoc validate` on the repository at every commit, which covers the
  frontmatter of new and changed SPCs and the `spec` counter.
- **Citations:** checked by reading at the end of each batch, every `SPC-N` against the text it
  names. No automated check in this story.
- **Nothing removed was a test:** the `///` comments contain no code fences, so removing one
  removes no doc-test. Checked again at each batch with `grep`, since a batch that adds a fence
  would change this.
