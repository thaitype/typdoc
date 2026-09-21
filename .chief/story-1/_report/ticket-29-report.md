# Ticket 29 Report

## Ticket
Turn on `clippy::expect_used` and `clippy::unwrap_used` for the non-test code of `typdoc-core`, so
that a new `expect` or `unwrap` there fails the build until a reason for it is written down. Each
remaining `expect`, `unreachable!` and `panic!` site becomes an `#[expect(clippy::..., reason =
"...")]` whose reason is the fact that makes the site hold and where it is checked.

## Outcome
done. `crates/typdoc-core/src/lib.rs` carries

```rust
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used, clippy::panic,
    clippy::unreachable, clippy::todo, clippy::unimplemented))]
```

and `cargo clippy --workspace --all-targets -- -D warnings` passes. Test modules and `tests/` are
outside `cfg(not(test))` and the integration tests are their own crates, so a test-code `expect`
needs no attribute and no `clippy.toml` setting was added.

- `expect_used` and `unwrap_used` do not cover `unreachable!` or `panic!`, so those two are listed
  as well, with `todo` and `unimplemented`. The lints were run before any annotation was written:
  27 sites in non-test code: 19 `expect`, 7 `unreachable!` and 1 `panic!`. The table below gives each.
- The sites were annotated first, with the lints still off, in five steps (argument and links; the
  five `links::scan` expects in `project.rs`; the other sites in `project.rs`; `query.rs`;
  `refs.rs`), each green on its own. An `#[expect(clippy::expect_used)]` is fulfilled while the
  lint is off: the attribute sets the lint to `expect` at that site, so the lint is evaluated
  there. The last step only adds the crate attribute.
- Behaviour did not change and no test was edited. One line moved: in `links::blank` the
  `String::from_utf8(..).expect(..)` is bound to a `let` and then assigned, because an attribute on
  an assignment expression does not compile (`attributes on expressions are experimental`).
- Every site could be given a reason and none is left unannotated.

The sites (27) and the fact each reason names:

| Site | Lint | What holds, and where |
|---|---|---|
| `argument.rs` `discover_for`, `strip_prefix` | expect | `root` is one of `dir.ancestors()` and `dir` is the parent of `absolute`, so `root` is a leading run of its components |
| `links.rs` `blank`, `from_utf8` | expect | the span is a `pulldown_cmark` offset range into this text or into `body` (equal byte for byte outside blanked spans), and only ASCII spaces replace whole characters |
| `project.rs` `list`, `links::scan` | expect | `parsed_fields` returned `Some` for the same `text`, which needs `frontmatter::block` to be `Ok`, and `scan` fails only when `frontmatter::split` does |
| `project.rs` `refs`, `links::scan` | expect | `frontmatter::block(&text).map_err(bad)?` above already returned that error for this `text` |
| `project.rs` `check_body`, `links::scan` | expect | `check_entry` is the only caller and runs it only when `validate::check_document` gave no `frontmatter.parse` finding, which it returns at fixed `Severity::Error` whenever `block(text)` fails |
| `project.rs` `check_body`, `links::mentions` | expect | `links::scan(text)` earlier in the same function ran `split` on this `text` and returned `Ok` |
| `project.rs` `parsed_fields_and_body`, `links::scan` | expect | `frontmatter::block(text).ok()?` above returns before it on an `Err` |
| `project.rs` `list_all`, `let ... else` | unreachable | `scope.imports` is filled only by `scope::select` through `Project::scope`, which refuses an alias outside `self.imports` and one that is `Absent`; a `Scope` built by hand with another alias would reach it |
| `project.rs` `schema_for` | panic | its only caller is the sort in `list_all`, on documents `Project::list` built with the name of a collection of the project it ran on (`self`, or the import `doc.project` names) |
| `project.rs` `ref_condition_inner_scope`, `let ... else` | unreachable | `RefField` has two variants and the first `if` returns for `Body` |
| `project.rs` `evaluate_ref_condition`, `incoming` | expect | its one caller, `Project::list`, builds the one `RefEvalCtx` and sets `incoming` whenever a `Dir::RefBy` condition is among `filter.wheres`; the condition passed is one of them |
| `project.rs` `resolve_import_outcome`, `let ... else` | unreachable | both callers get `alias` from a `BodyDestination::Import`, which `refs::classify_body` builds only for a `Loaded` entry of `ctx.imports`, and each builds `ctx` with `&self.imports` |
| `project.rs` `resolve`, `index.get(&path)` | expect | the `Path` arm returns `NotFound` unless the entry exists; a key group holds only paths with an entry, because `Index::build` binds and removes them together and `Index` has no other mutator |
| `project.rs` `resolve_key`, `found.into_iter().next()` | expect | the arm of `match found.len()` runs only for length 1 |
| `query.rs` `parse_value`, two `last_mut()` | expect | `segments` starts as one element and changes only by `push` and by `mem::replace` with a one-element vec |
| `query.rs` `finish_item`, `next()` | expect | the `if` above runs it only when `segments.len()` is 1 |
| `query.rs` `evaluate`, `items.first()` | expect | in this crate `PlainCondition` is built only in `parse_plain_from`, and `parse_value` ends with an unconditional push |
| `query.rs` `evaluate`, `_ =>` | unreachable | `Op::is_ordering` is true for the four ordering operators and the `if` above returns for them, so only `Eq` and `Ne` reach it |
| `query.rs` `ordering_matches`, `let Value::Number` | unreachable | `coerce` with `Number` and a `Value::Text` takes the arm that returns `Value::Number` |
| `query.rs` `ordering_matches`, `parse_date(have)` | expect | `have` is the text of a `Value::Date`, which no code in this crate builds but `coerce`, after `coerce::date` accepted it; `parse_date` runs the same check and parse |
| `query.rs` `ordering_matches`, `parse_datetime(have)` | expect | the same, through `coerce::datetime` and `DateTime::parse_from_rfc3339` |
| `query.rs` `ordering_matches`, `parse_date(&have[..10])` | expect | `coerce::datetime` checked that byte 10 is `T` and that `date(&text[..10])` holds |
| `query.rs` `ordering_matches`, `_ =>` | unreachable | the `matches!` at the top returns `OrderingNotAllowed` for every kind but the three arms above |
| `query.rs` `compare`, `Eq \| Ne` | unreachable | `compare` is called only at the end of `ordering_matches`, which `evaluate` calls only when `is_ordering()` |
| `refs.rs` `code_of` | expect | two callers test `looks_like_key(..)` on the same string in the enclosing `if`; `Project::mention_missing` passes the part after the last `:` of a `Mention.written`, which `links::mention_shape` accepts only when `looks_like_key_shape` holds for it |
| `refs.rs` `resolve_key`, `index.get(path)` | expect | `path` comes from `index.key(..)` on the same index, and a key group holds only paths with an entry (see `resolve` above) |

Runs, each planted alone in a non-test function of `typdoc-core`, seen red, removed, and the file
compared with its backup (`cmp` identical each time):
- a new `path.strip_prefix("x").expect("x")` in `refs::folder_of`: `error: used `expect()` on an
  `Option` value`, lint level defined at `lib.rs` `clippy::expect_used`;
- `.unwrap()`, `panic!`, `unreachable!`, `todo!` and `unimplemented!` the same way: each is an
  error from its own lint;
- the `.expect(..)` of `code_of` replaced by `.unwrap_or(key)` with its `#[expect]` left in
  place: `error: this lint expectation is unfulfilled`, implied by `-D warnings`, so a reason
  whose site was deleted fails the build;
- `std::fs::write` in the same function: `use of a disallowed method `std::fs::write``, so
  `clippy.toml` still works beside the new lints.

## Decision if any
- Decided: the lints go on in `lib.rs` as `cfg_attr(not(test), deny(...))`, not through `[lints]`
  in `Cargo.toml` and not through `clippy.toml`'s `allow-*-in-tests`. It is the smallest form that
  leaves `#[cfg(test)]` modules alone and it sits next to the code it governs. `tests/` files are
  separate crates that do not carry the attribute.
- Decided: `crates/typdoc` is left as it is. Its library and its binary are clean under the same
  six lints (run: `cargo clippy -p typdoc --lib --bin typdoc` with each lint denied), but the
  stand-in binary under `crates/typdoc/tests/support/` is a target of the same crate that uses
  `expect` and `panic!`, so adding the lints to the crate would need an attribute per target. The
  ticket is about `typdoc-core`, and a per-target attribute there is a change of its own.
- Decided: every site is annotated and none is rewritten. Two sites (`ref_condition_inner_scope`
  and the `1 =>` arm of `resolve_key`) could be written without the panic, at the price of a wider
  diff; each is annotated with the fact that makes it hold.

## Notes
- Some reasons state a fact about the crate as a whole (only `coerce` builds `Value::Date`,
  `PlainCondition` is built only in `parse_plain_from`). `Value`, `Scope` and `PlainCondition` are
  public with public fields, so a caller of the library who builds one by hand can reach the site.
  Those reasons say "in this crate" (the `list_all` one says it in words). `Index` keeps its
  fields private and has no method that takes `&mut self` besides `Index::build`, so the claim
  about it holds for a caller of the library too.
- Not verified: whether the five `links::scan(..).expect(..)` sites in `project.rs` could go by
  passing the split body in, so that `scan` cannot fail. That is a change to the signature of
  `links::scan`, `links::mentions` and their callers, and is not part of this ticket.
- A stale reason is caught two ways: a site that goes away leaves an unfulfilled `#[expect]`
  (planted above), and a new site has no attribute at all. A reason that stays true only in its
  words while the guard it names is edited away is not caught by clippy.
- The `expect` messages inside the calls were left as they were.
