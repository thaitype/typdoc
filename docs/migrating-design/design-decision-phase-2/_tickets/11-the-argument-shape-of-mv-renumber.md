# 11: The argument shape of `mv --renumber`

Type: wayfinder:grilling
Status: resolved
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

**1. The shape is `typdoc mv FROM --renumber NAMESPACE`.** The destination is the value of the flag,
not a second positional argument.

It has to be that, because story 1 already wrote and tested the rule for an argument that names a
document: after any prefix, something ending in `.md` is a path, something of the form of a key is a
key, and anything else is bad arguments. A bare namespace name is none of the three. Making it work
as a positional would mean cutting an exception into a rule that holds everywhere else in the tool,
and that rule is load-bearing precisely because it has no exceptions.

The shape also tells the truth about the command. `mv` takes a destination; this does not. It takes a
namespace and works the destination out for itself, because the new key is one nobody may choose.

**Two side effects, stated rather than discovered later:**

- `mv` reads two positional arguments in its ordinary form and one with `--renumber`.
- `--renumber` stops being a bare flag and becomes a flag that must be given a value. `--renumber`
  with nothing after it is bad arguments, and the message says a namespace is required rather than
  leaving someone to guess what is missing.

**2. A destination that is the namespace the document is already in is refused.** Nothing is
written.

The reason is worth keeping in full, because "refuse" can look like pedantry until the alternative is
written out. Carrying it out would do this: `WF-2` becomes `WF-3`, while the document sits exactly
where it was. The key `WF-2` is then dead for good, because `last` has risen past it and never comes
back down. Everything outside this project that cites `WF-2` — a commit message, an issue, a chat
message, another repository — now cites nothing. And `validate` reports none of it and exits 0,
because there is no broken ref inside the project to find: the document is fine, it simply answers to
a different name than the world outside believes.

So refusing is not the tool being strict. It is the only behaviour that does not quietly retire a
name that is in use, in exchange for nothing at all: no move happened.

The call is the defect here, so it falls under the existing rule for bad arguments, exit 1. The id
the error carries goes with decision 9's family along with the rest of the ids.

**3. It prints the new key, bare, on standard output.** As `typdoc new` prints the key it allocated,
so a shell can put it straight into a variable. One command, one line, nothing to parse.

**4. Renumbering into another project is not possible, and the design says so out loud.** Not
silence.

The reason it has to be said: reading across projects is written `project::namespace:key`, so the
shape for naming another project's namespace already exists in the design, and a reader who knows it
will reasonably guess that `--renumber` accepts it. It does not, and this agrees with
[decision 2](2-locks-when-mv-writes-into-an-imported-project.md), which settled that no command
writes into another project at all.

**5. The source's `last` does not go down, even when the document that moved out held the highest
number.** The design now says what `last` is: the highest number ever issued in that namespace, not
the highest that exists.

The two differ whenever a document is deleted or renumbered away, and that difference is ordinary
and is left alone. Reconciling them looks like tidying and is the bug: lower `last` to match what is
on disk, and the next `typdoc new` issues that number again. A ref written `story-1:WF-9` before the
move then resolves to a different document — and nothing reports it, because the ref is well formed
and so is the document. It is the one failure in this area that produces no finding at all, which is
why the gap is the thing being protected rather than the thing being fixed.

## Written into `docs/design.md`

The synopsis now shows both forms and the key `--renumber` prints; the `--renumber` paragraph carries
the argument shape and its reason, the same-namespace refusal with the account above, the bare-flag
error, the cross-project refusal, and the printed key; and the State paragraph says what `last` is,
that the gap is ordinary, and what lowering it would cause.

The shell-examples harness in `crates/typdoc/tests/shell_examples.rs` declares every example in the
design that holds a placeholder. Changing the synopsis from one line to two turned it red until both
new forms were declared there, which is the harness doing its job.
