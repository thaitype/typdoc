# 15: What does `--json` print on success?

Type: wayfinder:grilling
Status: claimed
Blocked by: 14

## Question

The design gave a shape for some commands and not for others: `get` "returns frontmatter plus" six names, `list` returns an object per document without saying what holds them, `toc` gives the shape of one heading, `refs` gives nothing, and `validate` returns "the summary and every finding". The `--json` output is what a program reads, and it is pinned by golden files, so the general rule, the shape of each command's output and whether each array has an order have to be decided together and written in one place in the design.

## Answer

Decided so far (the shapes of `toc`, `refs` and `validate`, and the order of their arrays, follow in this ticket):

1. **Every command prints an object with a named field for its result, never a bare value.** The reason is the failure this project exists to prevent: a wrong answer that looks right. `typdoc list --limit 20 --json` printing an array of twenty documents cannot say whether there are twenty matches or more, and a bare array has no place to put that fact. Adding a field beside a bare array later would change a shape people already read. Rejected: bare values (shortest to use with `jq`, but silent about truncation). Rejected: a single envelope such as `{ "result": ... }` around everything, because the exit code already says whether to read the result on standard output or the error on standard error, so the wrapper adds nothing.
2. **`list` reports `total` as well as `truncated`.** `truncated` alone tells the caller that some of the result is not visible but not how much, and the only way left is to run the query again without a limit, which may bring back an unknown amount. For a tool whose main users are programs, that is the difference between deciding and guessing. `truncated` is true exactly when `total` is larger than the number listed, and a test asserts that they agree.
3. **What `total` costs is a decision, not a side effect.** A `list` that reports `total` has to filter every document, so it cannot stop once `--limit` documents are found. The index is built on every run in any case, but filtering by frontmatter means reading the frontmatter of every candidate, and that part is new. Not measured. If the cost becomes too high it is a matter for the scale question on the map, and it is not a reason to ship a less useful shape now.
4. **A document has one shape, wherever it appears.** `get` and each member of `list` use the same object, with the same keys and the same nesting; only the wrapper differs. Otherwise a caller writes two readers for one thing. The frontmatter is held in `fields`, apart from `path`, `key`, `code`, `collection`, `schema` and `namespace`, because a frontmatter field that is not in the schema is kept and can have any name, `path` included; merged at the top level it could overwrite one of those.
5. **Consumers must ignore fields they do not know**, stated once in the design under JSON output and covering the error object too. The value of the first decision is that fields can be added without breaking the shape, and that is true only if a consumer is told not to fail on a field it has not seen; the first strict reader would otherwise break at the first addition.

The design has a new section, JSON output, holding the rule, the document and the shapes decided so far. Each further shape and the order of each array is added to that section, not to the command that prints it, so that the shape and the order of an array stay in the same place (ticket 9 ties them together through the golden files).

Already decided elsewhere and not reopened: `list` sorts by `--sort` and then in key or path order, with `key` compared by code and then numerically (`WF-2` before `WF-10`) and `string` and `path` compared lexicographically (Sorting under `list`).
