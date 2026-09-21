# 21: Whether a write warns when it is about to drop something

Type: wayfinder:grilling
Status: open
Blocked by: 20

## Question

[Decision 20](20-the-frontmatter-writer.md) settled that a write rewrites the whole frontmatter
block and that comments, blank lines, flow style, quote style, spacing, anchors, aliases and tags
do not survive it. The design states that, and advises that a document depending on an alias or a
tag be edited by hand rather than by typdoc.

The advice is in the design. The user running `typdoc set` is not reading the design.

Two of the losses are not cosmetic. An alias is one value used in several places; after a write it
is several values that no longer follow each other, and no rule in `validate` reports it, because
the file is valid and every value is the one it was. A tag may be what another tool in the user's
chain reads. In both cases a command that exits 0 with nothing said has destroyed work that was
done deliberately, and the user finds out later or never.

Decide:

- **Whether the command says anything.** A warning naming what was dropped, at which lines, is the
  obvious answer; against it is that a document with a comment in its frontmatter is ordinary, so a
  warning on every write teaches the user to ignore warnings, and the one about the alias drowns.
- **Which losses are worth saying, if not all.** Comments and spacing are cosmetic and common.
  Anchors, aliases and tags change what the file means and are rare. A split along that line is
  available; so is warning about nothing.
- **Before or after.** A warning after the write tells the user what has already happened. Before
  it, the command would have to ask, and no other command in v1 asks anything, or refuse, which
  would put the document back in the class decision 20 was chosen to abolish — one typdoc cannot
  write.
- **Whether a project can choose.** An option that turns the meaning-changing losses into a refusal
  would let a repository that uses anchors keep them safe, at the cost of a config surface and a
  second behaviour to test. The default has to be decided either way.
- **Whether `validate` reports it instead, or as well.** A rule that flags frontmatter holding an
  anchor, an alias or a tag would tell the user before any write ever happens, and it costs nothing
  at write time. It also flags documents nobody intends to write to.

The question `--audit` already answers for skipped files is the nearest precedent: something not
handled gets a place in an accounting rather than silence.

## Answer

<filled in on resolve>
