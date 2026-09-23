# 11: The central helper for reading a typdoc document's JSON body

Type: implementation
Status: open
Blocked by: 10

Ticket 7 (M-9) fixed the scope: internal to `typdoc-core`, used by tests, not wired into
`validate`/`get`.

## The work

A `pub` function (or small set of functions) in `typdoc-core` that, given a document, reads its
`content_type` frontmatter field, and:
- if `content_type` is `json`, parses the body as JSON and deserializes it into a caller-given type
  via `serde`;
- if `content_type` is missing, unrecognized, or the body doesn't parse as valid JSON when it says
  it should, returns a distinct, readable error for each case — never silently falls back to
  treating the document as plain prose.

Dispatch is entirely on the `content_type` field; nothing about the function's behavior depends on
the document's path (ticket 6's requirement). `pub`, not `pub(crate)`: ticket 5's answer already
established this is required regardless of ticket 7's outcome, since two of the three callers
ticket 12 will wire up are integration tests in other crates.

## Tests (see `testing-decisions.md`, "Design.rs's replacement")

- A catalog document missing `content_type` fails with its own distinct error.
- A catalog document with an unrecognized `content_type` value fails with its own distinct error.
- A catalog document whose body is not valid JSON, despite declaring `content_type: json`, fails
  with its own distinct error.
- Each of ticket 10's four real catalog documents round-trips through the helper into the type
  ticket 12's callers will use, matching the document's actual content.

## Done

- The helper exists, is `pub`, and passes the tests above.
- Nothing about its behavior reads a document's path.
