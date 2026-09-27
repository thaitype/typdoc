# Testing Decisions

Tests check behaviour from outside: the CLI's exit code, `--json` and files on disk, and the
template's public functions. They do not check private helpers.

- **Where the key ends.** Unit tests in `template.rs`, next to the existing
  `a_key_is_the_code_a_dash_and_one_or_more_digits` and
  `render_names_the_file_a_key_belongs_at_and_is_the_inverse_of_key`. They cover:
  - `WF-10.md` is `WF-10`;
  - `WF-12-x.md` is `WF-12` + `x`;
  - `WF-1-2x.md` is `WF-1` + `2x`;
  - `WF-1-v1.2.md` gives the slug `v1.2`;
  - `WF-1-.md`, `WF-1-a b.md`, `WF-1-a#b.md` and `WF-1-a:b.md` are members with `InvalidSlug`;
  - a slug in Thai is a valid member;
  - `WF-1x.md` is not a member;
  - `{key}/README.md` with the folder `WF-1-x/`;
  - `render` is the inverse of `key` with a slug and without one;
  - a T1-kind template: config error under `optional` / `required`, and under `none` it behaves
    exactly as the existing `a_key_can_be_followed_by_text_that_starts_with_a_digit` test says.
- **Config.** `fixtures/broken/config.collection-slug/`. The coverage tests (`coverage.rs`)
  enforce one fixture per id and exactly the rules it trips. `crates/typdoc/tests/config.rs`:
  a bad value, a non-string, and `slug` on an uncoded schema. `templates.rs`: the T1 template
  message names `slug: none`.
- **Arguments and refs.** `arguments.rs`: `WF-5-x`, `story-2:WF-5-x` and `chief::story-3:WF-5-x`
  name `WF-5`, and a stale slug prints nothing extra. `refs.rs` / `validate.rs`:
  - a full-form ref resolves by key;
  - `refs.slug` fires on a stale slug and on a target with no slug;
  - it never fires on a key-only ref;
  - its level can be raised, lowered and turned off;
  - before a write, `new` and `set` refuse on a stale slug only when `refs.slug` is `error`, and
    write at the default `warn`;
  - `refs.codedByPath` does not fire on a full-form ref.
  `fixtures/broken/refs.slug/`.
- **`filename.pattern`.** `validate.rs`: `WF-1.md` under `required`, `WF-1-x.md` under `none`,
  and `WF-1-a b.md`. Each stays a document (listed by `list`, and a ref to it resolves), and its
  finding carries `collection` and `key`. The existing stray-file test keeps its no-collection,
  no-key shape.
- **Mentions.** `body.rs`: `WF-3-lock-order` in text is not reported, even when `WF-3` does not
  exist, which pins the decided behaviour.
- **`new`.** `new.rs`:
  - `--slug` names the file;
  - a refused slug, `--slug` under `none`, a missing `--slug` under `required`, and `--slug`
    with a path each exit 1;
  - none of those four raises `last` in the state file.
  A golden case, `fixtures/output/new/coded-slug/`, for `new --slug --json`.
- **`mv`.** `mv.rs`:
  - change a slug, add one, remove one;
  - refs are rewritten in their written form (a key-only ref untouched, a full-form ref gets the
    new slug), including a body link;
  - `auto: moves` holds the previous path;
  - another key in the same folder is refused, exit 1;
  - an invalid-slug destination is refused, exit 1;
  - an unexpected form is carried out with `filename.pattern` in the findings, exit 0.
  `mv_renumber.rs`: the slug is kept, a full-form ref gets the new key with the same slug, and a
  mismatching destination `slug` gives exit 0 plus the finding.
- **Output.** A golden or `--json` assertion shows `key` without the slug and `path` with it.
- **Platform.** A test that creates `WF-5-a.md` and `WF-5-A.md` side by side cannot run on the
  default macOS file system. It takes the existing
  `#[cfg_attr(not(target_os = "linux"), ignore = "...")]` pattern, and the reason is written in
  the `ignore` text.
- **Every new check shown able to fail.** Before a ticket is accepted, plant a fault in:
  - the key/slug split (e.g. drop the maximal-digit rule);
  - the `refs.slug` comparison;
  - the `new` pre-lock slug check (check that `last` is still unchanged);
  - `mv`'s same-key condition;
  and see the named test go red, then remove the plant. This is the verifier's job, not only
  the builder's.
- **Suite.** `scripts/test.sh` locally, and the ubuntu and macOS CI jobs on PR #18.
