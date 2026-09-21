# 16: What does `mv` do when the destination belongs to a different collection, or to none?

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

## Question

A collection is defined by a `match` template or glob over paths, and a document's schema comes from the collection that matches its path. `mv` changes a document's path. So `mv` can change a document's schema, or take it out of every collection, and the design says nothing about either.

The cases:

- **Into another collection.** The document's frontmatter was valid against the old schema. Against the new one it may lack required fields, hold unknown ones, or hold an enum value that does not exist there. The move itself succeeds; the document is simply invalid afterwards, and the user finds out at the next `validate` — or does not, if nothing runs it.
- **Into no collection.** The file becomes an ordinary Markdown file with frontmatter that nothing checks. It disappears from `list`, is counted under `uncollected` in an audit, and its refs stop being validated. Nothing about that is visibly an error, which is what makes it easy to do by accident.
- **Out of a coded collection's folder.** A coded document's file name is its key. Moving it elsewhere either breaks `filename.pattern` in the old folder's collection or gives it a name that no `match` reaches while the key still exists — and the key is what refs use.
- **Into a coded collection.** The destination `match` is a template with `{key}` in it, so the name is determined; a document without a code cannot satisfy it, and one with a code from a different collection has the wrong prefix.

Decide:

- Whether `mv` checks the destination against the collections at all, and if so whether a change of collection is refused, allowed with a warning, or allowed silently.
- Whether it validates the document against the destination's schema before moving, and what it does when that fails. Refusing is defensible; so is moving and reporting, since `pull` already sets the precedent of changing schemas and reporting what breaks without rolling back.
- Whether moving a document out of every collection is a distinct case with its own message, since it is the one where nothing downstream complains.
- Whether a coded document may move within its own namespace to a path its collection's template does not produce, which is the same question `filename.pattern` answers for a file created by hand.

## Answer

**1. Two moves are refused, and the reason is structure, not policy.**

- A coded document cannot move out of its own collection's folder.
- A document without a code cannot move into a coded collection.

A coded collection's `match` takes `{key}` exactly once and allows no globs, so the file's name is
fixed entirely by the template; a document with no code has no key to put in it, and a coded
document taken out of the folder has a key that nothing resolves any more. That last one is the
sharp end: every ref written as a key would point at nothing, so the move would break references
that are in use. `mv --renumber` is the path that exists for moving a coded document.

**2. A move into another collection whose schema the document does not satisfy is carried out and
reported, not refused.**

Refusing sounds safer and is not, because it leaves the user with no order of steps that works:

1. To move, the fields would first have to suit the destination's schema.
2. Changing them makes the document invalid under the schema it still has.
3. `set` validates before it writes, so it refuses that change.
4. The only way left is to edit the file outside typdoc.

A rule that makes someone leave the tool to do ordinary work is the wrong rule. The precedent is
already in the design: `pull` changes the schemas, reports what the change breaks, and does not roll
back.

**3. The exit code of that move is 0, not 2.** This is the part most worth keeping.

Exit 2 means validation failed, and everywhere the design states it, it comes with nothing having
been written — `set` validates before writing, `pull --check` writes nothing at all. If `mv`
returned 2 after moving the file, one code would carry two meanings inside one tool, and a caller
reading 2 could no longer tell whether the world had changed. That is the entire reason exit codes
exist, and spending it here would cost more than the case is worth.

Deciding whether a document satisfies its schema is `validate`'s work, not `mv`'s. The result of the
check travels in `mv`'s `--json` payload, so an agent can branch on it, and CI catches it by running
`validate`, which exists for exactly this. The field that carries it is part of
[decision 17](17-the-json-shapes-of-new-set-and-mv.md).

**4. Moving out of every collection is allowed, and the command says so in its own words.** The
document will not appear in `list`, and its refs are no longer checked.

It is allowed because it is a legitimate thing to want. It is announced because it is the only one
of these cases where nothing afterwards reports anything: a document in no collection produces no
findings, so silence here means the user never finds out. Note that this case reaches only documents
without a code, since a coded document moving out of its folder is refused under 1.

## Noted while checking this: an adjacent silence in `pull`

The argument in 3 rests on exit 2 never following a write. That holds for everything the design
states. It also turned up a gap next door: plain `pull` writes `vendor/` and `lock.json`, then
validates every document against the new schemas and reports what the change breaks without rolling
back — and the design never says what it exits with. `pull --check` has an exit code stated;
plain `pull` after a breaking change does not.

`pull` is story 3 and out of this map's scope, so nothing is decided about it here. It is recorded
because the decision above is the precedent it will meet: a command that wrote successfully and then
found something wrong reports in its payload and does not spend exit 2 on it.
