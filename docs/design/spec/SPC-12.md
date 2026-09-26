---
title: JSON output explained
status: active
migrated_from: docs/archived-design/design.md#json-output
---

Every command accepts `--json`, and its result is one JSON object on standard output. Commands
are of two kinds. A command whose result is a verdict on the project (`validate`, and
`pull --check`) prints that verdict on standard output whether or not it is favourable: the
findings are the result, and exit 2 says that some of them are errors. A command that does
something (`get`, `list`, `toc`, `refs`, `new`, `set`, `mv`, `pull`) prints its result on
standard output when it succeeds and, when it cannot, the error object on standard error. A new
command falls on one side by asking whether its result is a judgement about the project or the
outcome of an action.

The result is held in a field named for it and is never printed bare, so that facts about the
result can sit beside it: a `list` that `--limit` cuts short has to say so, and a bare array has
no place to say it. Output may gain fields in later versions, so a consumer must ignore any field
it does not know; this holds for the error object as well.

A value in the output is a fact about the document, never about the command that asked: a flag
that chooses or limits what is listed (`--depth`, `--limit`, `--field`) changes which items
appear and never the values of an item. The output promises only what a caller cannot work out
from what it is already given, because every field it promises has to stay true for as long as
the version does: a count of findings per rule, for example, is not in it.

## Naming a document

There is one way to name a document, and every shape that has to mention one uses it: `path`,
`namespace` (absent for a file outside every namespace folder, which a ref can still reach),
`key` when the document has a code, and `project` when the document belongs to an imported
project. `project` is the alias under which this project imports it; it is absent for a document
of this project. A document object is that name plus `code`, `collection`, `schema` and
`fields`; a finding is that name, without `project` since a finding is always in this project,
plus `rule`, `level`, `message` and a position; a reference is that name plus `field`, `written`
and a position. A new shape that mentions a document adds to the name and never renames a part
of it.

## A finding

A finding is the same object in the report of `validate` and in the `details` of an error object.
It always has `rule`, `level` and `message`. It has `path`, the file it is about, relative to the
project folder, for a document and for a configuration file alike; `namespace`, `collection` and
`key` when the file is a document (`collection` when it is in one, `key` for a coded one only);
`field` when it is about one field; and `line` and `col`, 1-based, when a position is known. A
finding is always located in a file of the project being checked, never in one of an imported
project: a broken ref is a fault of the document that holds it, not of its target.

## A `number`

A `number` is printed with the digits written in the document, not with a value converted from
them. JSON puts no limit on the digits of a number; its readers do, each in its own way, and a
reader that cannot hold one rounds it knowingly from a true value rather than being handed a
different one. It is the reason the frontmatter reader keeps text: nothing between the file and
the caller decides what `1e3` is. Converting first would lose more than digits: two documents
whose numbers differ by one would print the same value and could not be told apart, and `1e3`
would print as `1000.0`, which is not what the file says. A field of any other type prints the
value it holds, and a `string` of digits is printed as the text it is.

## A field written with no value

A field written with no value is `null` in `--json`, and one written as an empty string is `""`.
The output says what the file says, for the same reason a `number` carries its digits: the caller
is told what is there, not what typdoc would have made of it. Every rule and every command treats
the two alike, so nothing else in the output moves.

## A document

A document is the same object wherever it appears, in `get` and in `list` alike, including a
`list` that reaches an imported project with `--namespace 'chief::*'`. It has the name above,
`code`, `collection`, `schema`, and `fields`, which holds all of the document's frontmatter. The
frontmatter stays apart in `fields` because a field that is not in the schema is kept and can
have any name, `path` and `key` included.

## `list`

`total` is the number of documents that match, counted before `--limit`, and `truncated` is true
when `total` is larger than the number listed. Because `total` is always reported, a `list`
filters every document even when `--limit` is small.

## The summary of `validate`

The summary says what the report covers, so that a list of findings cannot be read as more than
was checked. `scope` is `all` for the whole project, `paths` when arguments named the documents
to check, and `schemas` for `--schemas`. `strict` is true when `--strict` was in effect.
`checked` holds `namespaces`, the namespaces covered, sorted by name, and `documents`, the number
of documents checked (0 for `schemas`); for `paths` it also holds `paths`, the `path` of each
document checked, sorted and each once, so it can be matched with the `path` of a finding.
`findings` counts the findings per `level` (`error`, `warn`, `info`) after the rule levels have
been merged and after `--strict` has raised the warnings, so the numbers agree with the exit
code. `strict` is there so that a count of errors is not read against a configuration file that
calls the same findings warnings.

With `--audit`, `summary` also has `audit`, true, and `unreported`, an object with `uncollected`
and `no_frontmatter`, the number of files in each of those two lists under `audit` below. In an
audit, `findings` is not the whole list of work: files in no collection and files with no
frontmatter are outside it, and `unreported` puts their numbers where a reader of the summary
sees them. These two counts can be worked out from the lists; they are in the summary because a
reader of the summary and the findings alone would otherwise report less work than there is,
which is the same reason as for `scope` and `strict`, and they are the exception to the rule that
the output promises only what a caller cannot work out.

A file matched by more than one collection is counted as `overlapping`, beside `unreported` and
not inside it: it is reported under `collections.overlap`, so it is not outside `findings`, which
is what `unreported` means, and it is checked against no schema, since there is no one schema to
check it with. `not_read` is counted beside `unreported` and `overlapping` for the same reason:
an entry that was not read is reported in `findings`.

The account a reader can take from the summary is therefore `checked.documents` plus every count
of what was not checked, `unreported.uncollected`, `unreported.no_frontmatter`, `overlapping` and
`not_read`, and that total is the number of directory entries the run met. A document whose block
cannot be parsed is checked, since it has a finding, and is counted in none of the others.
Without `not_read` the account would be short by every entry the run skipped, the one case where
a file is reported and counted nowhere.

## Audit

With `--audit` the report also has `audit`:

- `collections`, one `{ "name", "documents" }` for each collection of the project, sorted by
  name, one that holds no document included with `documents` 0. A file matched by more than one
  collection is counted in the number of none of them, since it is listed under `overlapping`
  and checked against no schema, so a collection whose every file is also matched by another
  shows 0.
- `uncollected`, the `path` of every file that belongs to no collection.
- `no_frontmatter`, the `path` of every file that belongs to a collection and has no frontmatter.
- `overlapping`, one `{ "path", "collections" }` for every file matched by more than one
  collection, with `collections` the names of the collections that match it, sorted by name.
- `not_read`, one `{ "path", "reason" }` for every directory entry the run met and did not read
  (a symbolic link, a name that is not valid UTF-8, or a leftover temp file), so that an entry
  which is reported in `findings` is also counted somewhere in the account.

`uncollected`, `no_frontmatter` and `overlapping` are sorted by `path`. The three lists do not
overlap: a file in no collection has no schema to say what its frontmatter should hold, so it is
only in `uncollected`, and a file matched by more than one collection belongs to collections
rather than to none and its frontmatter is never read, so it is only in `overlapping`. The
documents per collection cannot be worked out from `findings`, since a clean document produces
none. A finding carries its `collection`, so the counts per collection and rule, which the text
form prints as a table, can be worked out from `findings`; they are not in the JSON.

The two modes treat a file with no frontmatter differently, and this is a difference of
mechanism, not of presentation. `validate` evaluates it against its schema like any document, so
each required field it lacks is a finding. `--audit` does not evaluate it: it lists the file in
`no_frontmatter` and produces no findings for it, which is the point of grouping it, since a
directory of plain notes would otherwise bury every other defect under required-field findings.

## References

In `refs`, `document` is the name of the document asked about, and `direction` is `out` for the
refs it holds and `in` for `--reverse`. A reference is the name of the document at the other end
(the target for `out`, the document that holds the ref for `in`), plus `field`, the field that
holds the ref (`$body` for a body link), and `written`, the text as it is written in the file,
which cannot be worked out from the other end and which `mv` needs to keep the written form. A
body link also has `line` and `col`, 1-based. For `in`, the documents of this project that hold
the refs are in `path` order.

A reference that does not resolve has no `path` and has `unresolved` instead, one of three values:
`not-found`, the place the ref names is present and the file or key is not; `import-absent`, the
import it names is not on this machine, which `imports.absent` reports; and `bad-prefix`, the
prefix names no namespace and no import, or names a project with several namespaces without saying
which. A missing `path` alone would make a broken link and a machine that has not been set up look
the same, and they are different problems with different fixes. `path` and `unresolved` never
appear together, and `unresolved` occurs only for `out`, since a reference read from a document
that holds it has been found. Unresolved references are listed: a ref is counted from what is
written in the field. `--field` keeps only the refs in that field, `$body` for body links, and it
means the field that holds the ref in both directions, so with `--reverse` it is a field of the
document that holds it.

## The write commands

`new` and `set` print the document as it stands after the write, in the same object `get`
prints. For `new` that repeats every default and every `auto` field the command filled in, which
is exactly what the caller could not have worked out. Neither says which fields the write
changed. Whether a field changed is not a property of the document: the same document reached by
another route would carry a different answer, and that is the test of whether something is a
fact about the result or about the command that asked. A caller that must act only when a
document is in a given state has `--if`, which is the mechanism for it.

`mv`'s `document` is the document under its new name, so a `--renumber` reports the key it issued
in the field every other shape carries a key in. The old name is not repeated: the caller gave
it. `rewritten` lists every ref the move rewrote. `unrewritten` holds one entry for every ref
left pointing at the old name, in the reference object, with `reason` saying why it was left.
`findings` carries what the destination's schema rejects when a move lands a document in a
collection whose schema it does not satisfy: the move is carried out, the exit code is 0, and
this is the field to branch on rather than the code. They are the finding object `validate`
prints, so one consumer reads both.

## Order

The order of `findings` is guaranteed: by `path`, then `line`, `col`, `rule` and `message`, a
finding with no position before one with a position in the same file. `path` is compared
lexicographically, by the bytes of the path. This differs from `list` on purpose. `findings` are
ordered by position in a file, so the path decides; `list` orders documents, and a document's key
contains a number, which is compared as a number. They order different things, and making them
agree would leave the order of findings depending on the numbers in keys. In `audit`,
`collections` is sorted by name and the other lists by `path`. In `mv`, `findings` is ordered the
same way as `validate`'s.
