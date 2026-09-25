# How to add typdoc to a folder you already have

You have a folder of Markdown files with frontmatter, and you want typdoc to check them. typdoc
doesn't rewrite your files to adopt them: you add a `.typdoc/` folder with some schemas in it next to
them, and fix whatever the checks turn up.

This guide uses a folder of decision records (`decisions/ADR-1.md`, `ADR-2.md`, ...) and some
notes as the example.

## 1. Mark the folder as a project

```console
$ mkdir -p .typdoc/collections .typdoc/schemas
$ echo '{ "version": 1 }' > .typdoc/config.json
$ echo '.typdoc/locks/' >> .gitignore
```

## 2. Write a schema and a collection for each kind of document

Look at a few files of each kind and write down the fields they share. For the decisions:

```json
{
  "name": "decision",
  "code": "ADR",
  "fields": {
    "title":  { "type": "string", "required": true },
    "status": { "type": "enum", "values": ["proposed", "accepted", "superseded"], "default": "proposed" }
  }
}
```

Give the schema a `code` only if the files are already named by number (`ADR-1.md`) and you want
typdoc to hand out the next ones. Otherwise leave it out and the documents are named by path.

Then point a collection at the files:

```json
{ "match": "decisions/{key}.md", "schema": ".typdoc/schemas/decision.json" }
```

For files named freely, use a glob instead: `{ "match": "notes/*.md", "schema": ".typdoc/schemas/note.json" }`.

The [project files reference](../reference/project-files.md) lists every field type and option.

## 3. Run an audit

```console
$ typdoc validate --audit
typdoc audit: 2 collections, 6 files (1 in no collection)

decisions  3 files   state.missing 1 error · frontmatter.types 1 error · frontmatter.unknown 1 warn
notes      2 files   body.links 1 error

in no collection: decisions/README.md (1)

no frontmatter: notes/scratch.md (1)
```

An audit always exits 0 (unless the config itself is broken), so you can run it as often as you
like while you adjust things. It groups what it found by collection and rule, and lists two things
a normal run doesn't: files no collection matched, and files with no frontmatter at all.

Work through it in this order.

**Files in no collection.** Either a `match` template is too narrow, or the file really isn't a
document. Widen the template, or leave the file out.

**Files with no frontmatter.** If they should be documents, add a frontmatter block. If not,
narrow the `match` so it skips them.

**`state.missing`.** This one is specific to adopting numbered files, so it gets its own step
below.

**Everything else.** Each finding names a rule. `frontmatter.types` means a value doesn't fit the
schema (`Accepted` where the schema says `accepted`), and `body.links` means a link points at a
file that doesn't exist. The [validation rules reference](../reference/validation.md) explains
each one. Decide for each whether the file or the schema is wrong.

To see the individual findings rather than the summary, run `typdoc validate` without `--audit`.

## 4. Record the numbers already used

If your schema has a `code`, typdoc needs to know the highest number that has ever been used
before it can hand out the next one. Until it knows, it refuses:

```console
$ typdoc new ADR "Next decision"
typdoc: the collection `decisions` has documents in this namespace and no `last` recorded in its state file
```

Write the state file yourself, once. Use the highest number that was **ever** used, including
records that have since been deleted. If `ADR-3` existed once and was deleted, and `ADR-4` is the
highest now, `last` is 4. If you know an `ADR-7` was issued and later removed, it's 7.

```console
$ mkdir -p .typdoc/state
$ echo '{ "decisions": { "last": 4 } }' > .typdoc/state/default.json
$ typdoc new ADR "Next decision"
path: decisions/ADR-5.md
...
```

From here on `typdoc new` keeps the file up to date. Commit it along with your documents.

## 5. Turn off what you don't want checked

Rules that can be configured are set in `.typdoc/config.json`, for the whole project, or in a
collection file, for that collection's documents:

```json
{
  "version": 1,
  "validation": {
    "global": {
      "frontmatter.unknown": { "level": "off" },
      "body.links": { "level": "error", "ignore": ["assets/**"] }
    }
  }
}
```

`frontmatter.unknown: off` stops warnings about fields your schema doesn't list, which is common
while a schema is still growing. `ignore` skips links to files you don't keep in the project.

A `README.md` inside a numbered folder like `decisions/` fits no template there and is reported as
`filename.pattern`. Move it out, or set `filename.pattern` to `off` under `global`. Setting it in
the collection file doesn't help, because the README isn't one of the collection's documents.

## 6. Run the plain check

When the audit looks right, run the real check:

```console
$ typdoc validate
```

It exits 2 while any finding is an error. Once it's clean, [add it to CI](check-in-ci.md) so it
stays that way.
