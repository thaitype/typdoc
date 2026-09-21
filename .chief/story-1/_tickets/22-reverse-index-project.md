# 22: A reverse lookup counts a ref that left the project

Type: implementation
Status: open
Blocked by: None (can start immediately)

## What this delivers

- The reverse index keyed by project, namespace and path, rather than by namespace and path, so that a ref which resolved into an imported project is not counted as a ref into a document of this one.
- The fix reaching both ways into that index: `refs --reverse` and a `refby.*` condition in `list`.

## The fault

A ref that crosses into an imported project resolves correctly, and is then recorded in the reverse index under its path alone. A document of this project whose path happens to match is then reported as the thing that ref points at.

With a project `a` importing a project `b`:

```
a/notes/pointer.md      see: b::notes/target.md     (points into b)
a/notes/target.md                                   (nothing in a points at it)
b/notes/target.md
```

```console
$ typdoc refs notes/target.md --reverse --json
{"refs":[{"path":"notes/pointer.md","namespace":"default","field":"see","written":"b::notes/target.md"}]}

$ typdoc list --where 'refby.any(see)' --ids
notes/target.md
```

Both answers are wrong: nothing in `a` points at `a/notes/target.md`. The control, with `pointer.md` holding `see: target.md` so that it really does point inside `a`, gives the same two answers, which is what shows the project is not part of the comparison.

The evidence is already in the output: the reference the reverse direction prints carries `written: b::notes/target.md`, which names the project it went to. The `out` direction of the same command prints a `project` field for such a reference and the `in` direction prints none, so the side that resolves the ref knows where it landed and the side that indexes it does not keep that.

This is not one of the differences in `KNOWN_GAPS`. Those are places where the binary knowingly does less than the design says; this is a wrong answer.

## Done when

- A test fails before the change and passes after it: the shape above, asserting that nothing points at `a/notes/target.md`, with a control test beside it where the ref really is inside `a` and is found. A test that stays green under the fault it names is worth nothing.
- The same pair for a `refby.*` condition in `list`, since both directions read one index.
- The reference printed by the reverse direction names the project it came from, or it is stated in the report why it should not.
