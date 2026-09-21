# 16: What does `mv` do when the destination belongs to a different collection, or to none?

Type: wayfinder:grilling
Status: open
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

<filled in on resolve>
