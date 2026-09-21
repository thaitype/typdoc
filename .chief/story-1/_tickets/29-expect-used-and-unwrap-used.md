# 29: Turn on `clippy::expect_used` and `clippy::unwrap_used` for non-test code of `typdoc-core`

Type: implementation
Status: open
Blocked by: 27, 28

## What this delivers

The lints are on for the non-test code of `typdoc-core`, so a new `expect` or `unwrap` there is a build failure until it is justified. Each remaining `expect` and `unreachable!`/`panic!` site becomes `#[expect(clippy::expect_used, reason = "...")]` (or the matching lint) whose reason is the evidence for that site: what makes it hold, and where (a function and a line's worth of fact), never the word "safe".

## Done when

- `cargo clippy --workspace --all-targets -- -D warnings` passes; a test-code `expect` needs no attribute.
- Each reason is checked against the code by reading it, and a site whose reason cannot be stated is reported rather than annotated.
- One `expect` added inside a function body turns clippy red with the lint's message (planted, seen, removed).
