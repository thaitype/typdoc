# 11: The argument shape of `mv --renumber`

Type: wayfinder:grilling
Status: open
Blocked by: None (can start immediately)

## Question

`typdoc mv <from> <to> [--renumber]` moves a coded document to another namespace under a new key: it holds the locks of both namespaces, issues the next number from the destination's `last`, moves the file and rewrites every visible ref in the form that is correct from each referencing document's own namespace.

The destination is the problem. The new key does not exist yet — typdoc issues it — so `<to>` cannot be the key. The file's name is its key and nothing else, so `<to>` cannot be the path either without naming a number the user is not allowed to choose.

Decide the shape, and check it against the rules that already exist:

- Is `<to>` a namespace name (`typdoc mv WF-5 story-3 --renumber`)? Then it is an argument that is neither a key nor a path, and the design's rule for telling arguments apart — after any `project::` prefix, an argument ending in `.md` is a path and one of key form is a key, anything else is exit 1 — makes a bare namespace name bad arguments today.
- Is the destination given by `--namespace`, with `<to>` dropped? Then `mv` takes one argument with `--renumber` and two without, and `--namespace` means something different here from everywhere else, where it chooses the scope a command reads.
- Is it a prefixed form (`story-3:`) that cannot be mistaken for a key or a path?
- What names a namespace of a *different* project? Nothing: a coded document cannot cross a project, and `--renumber` is about namespaces of one project. Say so explicitly, because the `project::namespace:key` form exists for reading and its absence here should be a decision rather than an oversight.
- What happens when the destination namespace is the source namespace: an error, or a no-op that still consumes a number?
- What is printed. `new` prints the bare key on stdout; the natural parallel is that `mv --renumber` prints the new key, which makes it usable in a shell pipeline.
- The old key is never issued again because the source namespace's `last` never goes down. Confirm that this is true when the moved document held the highest number in the source collection, since `last` is then above every existing document and a reader might be tempted to lower it.

## Answer

<filled in on resolve>
