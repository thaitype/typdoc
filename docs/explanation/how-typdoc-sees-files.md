# How typdoc sees your files

typdoc is a way of looking at a folder of Markdown files. It doesn't import them into anything,
and it doesn't keep a copy. This page is about what it looks at, what it ignores, and why a few of
its rules are stricter than you might expect.

## The files are the only source of truth

Every command starts by reading the project from disk: the config, the collections, the schemas,
and then every file the collections match. From those it builds an index of every key and path,
answers the question, and throws the index away. There's no cache to go stale and no database to
get out of sync with the files.

That's what makes it safe to edit files by hand, check out an old branch, or resolve a merge
conflict in your editor. The next typdoc command sees exactly what's on disk. The cost is that
every run reads the whole project.

## Which files count

A file is a document when it's matched by a collection's `match` template and starts with a
frontmatter block. Everything else in the folder is invisible to typdoc: a README, images, a
`.gitignore`, whatever you keep alongside.

A few choices here are deliberate:

- Wildcards don't walk into folders whose names start with a dot, the same way namespaces don't.
  If you do keep documents under a dotted folder like `.agents/`, name that folder in the
  template, and typdoc reads it. Naming a folder says you want it; a wildcard only says "whatever
  is here".
- Symbolic links to folders aren't followed. A run can't wander out of the project, and can't
  read one file twice under two names.
- `.gitignore` isn't consulted. What your version control hides is a different question from what
  your project declares.
- A file with no frontmatter isn't a document, but a file with an empty `---` block is. The empty
  block says "I'm a document with no fields yet", so it's checked, and every required field it
  lacks is reported. Without that distinction, a document missing everything would look the same
  as a plain note that was never meant to be one.

## Frontmatter is typdoc's, the body is yours

typdoc reads the whole file but only ever writes the frontmatter. It reads the body for two
things: headings, for `toc` and for checking `#heading` links, and links, which are refs. It never
reformats the body. The one time it changes body text is `mv`, which updates link paths.

When typdoc writes frontmatter, with `set` or `new`, it writes the whole block, not just the line
that changed. Every value comes back as the exact text it was written with: `1e3` stays `1e3`,
`0755` stays `0755`, `no` stays the string `no`. What doesn't come back is presentation: comments,
blank lines, quote style, inline `[a, b]` lists, YAML anchors and tags. typdoc promises the
values, not the presentation. If a document relies on comments or anchors in its frontmatter,
it's one to edit by hand.

A field written with no value (`reviewer:`) and one written as an empty string (`reviewer: ''`)
are different YAML, and another tool reading the file may care. typdoc keeps whichever one it
found, and treats both the same way in queries and checks.

## Two kinds of name

A document with a code is named by its **key**, and the key is its file name: `tickets/WF-3.md`
is `WF-3`. The title can change as often as you like and the file name never does, so links to
it never break because someone reworded a ticket. typdoc never builds a file name from a title:
a title can be an emoji, or a Thai sentence longer than a file system allows.

A document without a code is named by its **path**, which you choose.

A key never ends in `.md` and a document always does, so the two can't be confused, and typdoc
never has to guess which you meant.

## Refs

A ref is anything that points from one document to another:

- a frontmatter field of type `ref` or `ref[]`
- a Markdown link in the body

Both go through the same index, so a ticket can point at a note and a note can link to a ticket.

A frontmatter ref is read in a fixed order. `WF-3` is a key in the document's own namespace.
`story-2:WF-5` is a key in the namespace `story-2`, and `memory::notes/x.md` is a path in the
imported project `memory`. Anything else is a path relative to the document. A single colon only
ever means a namespace and a double colon only ever means an import; if the name doesn't exist
on that side, it's an error rather than a fallback to a relative path. A path that really contains
a colon is written with `./` in front.

In frontmatter, a numbered document should be referred to by key, since the key is its name and
the path only says where the file sits. By path works, but `validate` warns (`refs.codedByPath`).
Body links always use paths, because that's what Markdown and GitHub understand.

Plain text that happens to look like a key ("see WF-3") is never a ref. You can ask typdoc to
check that mentioned keys exist, with the `body.mentions` rule, but `mv` never rewrites them and
`refs` never lists them.

## Paths are compared exactly

A ref to `target.md` doesn't resolve to a file named `Target.md`, on any operating system. macOS
and Windows would happily open the file with either spelling. Linux wouldn't. If typdoc followed
the file system, the same repository would validate on a Mac and fail on a Linux CI runner, so it
compares names exactly everywhere.

## Why no new syntax

Everything typdoc understands is already standard: YAML frontmatter, Markdown links, headings.
Someone who has never heard of typdoc can open any document on GitHub or in an editor, read it,
follow its links, and edit it without breaking anything they can see. That's the reason typdoc
can be added to an existing folder without rewriting it, and removed again by deleting `.typdoc/`.
