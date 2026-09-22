# 7: The write seam in `deps`, and `Clock`

Type: wayfinder:grilling
Status: resolved
Blocked by: 4

## Question

Story 1 built `run(args, deps)` as the only path into the program, with `Deps` holding one member, `env`. Two more belong to story 2: the seam through which the program writes, and a clock, because `auto: create` and `auto: update` fields are stamped with the current time and a golden file cannot contain a time the test did not choose.

Decide:

- **The operations of the write seam.** Is it the file operations (create a temp file, write bytes, rename, remove) or the document operations (write this document, update this state file)? The lower seam is easier to reason about and lets the atomic-write rule live in one place; the higher one makes tests read like the domain and hides the temp file from every caller. The answer interacts with ticket 4: whichever side the temp file lands on is the side that owns the promise about what an interrupt leaves.
- **What is banned outside it.** `typdoc-core` already bans `std::env::var` and the home directory, and `Command::new` outside the spawn helper, each with a lint that covers test code. Is `std::fs`'s writing half banned in the same way, and what is the lint?
- **What the tests use.** A fake in memory, or a real temporary directory? A fake cannot fail the way a real filesystem fails — out of space, a permission refused, a rename across devices — and those failures are exactly what tickets 1 and 4 are about. A real directory cannot easily produce them either. Say which failures are provoked and how, and which are not covered and are recorded as such.
- **`Clock`.** Its shape, its resolution, and what the injected clock returns in tests. `auto: datetime` values carry offsets, so decide whether the clock gives an instant and the formatting is typdoc's, or the clock gives the formatted value.
- Whether reads go through the seam too, or only writes. Story 1 reads the filesystem directly; making writes special is defensible, and making the two asymmetric is a thing to decide on purpose rather than by default.

## Answer

**Decided: the seam is the file operations, `Deps` gains `fs` and `clock`, and reads stay outside
it except the three a write itself needs.**

### The seam is the file operations, not the document operations

`Deps` gains one member beside `env`: the operations a write is built from, not `write this
document`.

The reason is [decision 4](4-what-an-interrupted-write-leaves-behind.md). What it settled is
policy, not mechanism: a temp file takes a reserved name shape carrying the pid and a random
part; the walker skips that shape by rule before `match` is consulted; a leftover is a finding at
`warn` and joins the audit's account of things not read; a command holding a lock may remove
leftovers in that lock's scope and never by age; the mode of an existing file is carried to the
temp file before the rename. Every one of those is a rule about *when* and *what*, and every one
of them is worth a test.

Put the seam at the document level and all of it sits underneath, in the implementation. The fake
that the tests run against would have to reimplement the rules, so each of those tests would be
checking the fake. Put the seam at the file operations and the rules sit above it, in
`typdoc-core`, in the code that ships; the fake only has to pretend to be a filesystem, which is a
thing with no rules of typdoc's in it.

The second reason is that there is more than one writer. A document, the state file, a lock file
and the several renames of an `mv` all want the same atomic-write rule. At the document level it
would be written once per kind of thing written. At the file level it is written once.

**What this costs, and what pays it back.** Every caller can see the temp file, which a
document-level seam would have hidden, and a test at the seam reads like a filesystem rather than
like the domain. The answer is not to raise the seam but to put one function above it —
`write_atomically`, holding decision 4's rules end to end — and have every command call that. The
temp file then appears in two places, the seam and that function, and in none of the commands.

**The operations.** Create a file that must not already exist; write bytes to an open handle;
rename; remove a file; create a directory and its parents; read a file's mode and set it; ask
whether a path exists and whether two paths are the same file; and flush a file to storage. Their exact signatures belong to the build.
Two of them are on the list for reasons worth saying: `create_dir_all` because
[decision 3](3-one-order-for-taking-locks.md) needs the lock's directory to exist before the
first lock is taken, and the flush because
[decision 4](4-what-an-interrupted-write-leaves-behind.md) left durability without `fsync`
undecided — having it in the seam means that decision can be made either way later without the
trait changing.

### The gap decision 15 left open, and why it is left open

[Decision 15](15-a-write-whose-destination-already-exists.md) refused a write whose destination
exists, made the check under the lock, and had `new` create with `O_EXCL` so the file system
enforces it rather than a check that can go stale. It recorded one gap: `O_EXCL` does not reach
`mv` or `--renumber`, which put a document in place with a rename, and a plain rename replaces
silently.

**Decided: `mv` moves a document with a plain rename, after the check, under the lock. The gap
stays open and is not worth closing.**

There is a way to close it. `link` is the one call in the standard library that takes a name or
fails rather than replacing — measured here, it returns `AlreadyExists` and leaves the file that
is there untouched, where `rename` replaces with no error — so `link` followed by removing the
source would move a document without ever replacing anything.

What that buys is the reason not to do it. Under the lock, no other typdoc run can be in this
namespace, and the check immediately precedes the rename. The only thing left in that window is a
program that is not typdoc creating a file at exactly the destination path in the microseconds
between the two calls. That event cannot be told: not who it happened to, not what they were
doing.

What it costs is a state on disk that does not otherwise exist. Between the link and the removal
the document has two names, and the tool has to have a rule for meeting that state on a re-run,
a test for the rule, and a sentence in the design telling a user why their document briefly had
two names. A protection against an event nobody can describe, paid for with a state that needs
its own rule, is the wrong trade.

**Where `O_EXCL` stays, and why the line is there.** `new` keeps it. It is one call that either
creates the file or does not, it adds no second name and no window, and it costs nothing. The
line is not "how much protection" but "what the protection costs": a guarantee that comes free
with a call already being made is taken, and one that has to be bought with a new state on disk
has to name the event it prevents first.

**What this leaves.** A document is moved with `rename`. A document whose content is being
replaced — every file an `mv` rewrites a ref in, and every `set` — is written to a temp file in
the same directory and renamed over the original, which is not a move but the way to replace a
file's contents so that an interrupted run cannot leave half of one. That event can be told
immediately, and [decision 4](4-what-an-interrupted-write-leaves-behind.md) is about it.

**A rename cannot cross a filesystem**, measured: `errno 18, Invalid cross-device link`. The shell
command of the same name falls back to copying and deleting; typdoc does not, and fails with the
error instead. The event to tell would be a user with another device mounted inside their own
project folder, moving a document across the mount point, and it cannot be told.

### What is banned outside the seam

`typdoc-core`'s `clippy.toml` already bans the writing half of `std::fs` — `write`, `rename`,
`copy`, `remove_file`, `remove_dir`, `remove_dir_all`, `create_dir`, `create_dir_all`,
`hard_link`, `set_permissions`, `File::create`, `File::create_new`, `File::set_len`,
`OpenOptions::open`, `symlink` — each with the reason "the read core changes no file". The list
is already the right list. What changes is the reason, which becomes that a write goes through
the seam, and it stops being true that the core changes no file.

The ban keeps covering test code, as the environment's does, because a test that reaches around
the seam is exactly the test that stops proving anything about the code that ships.

**What the lint does not catch, said plainly.** It names functions. It cannot see bytes written
through `io::Write` to a handle that is already open. It holds anyway, because the only way to
get a writable handle is through one of the banned constructors, so the handle cannot exist
without a banned call having been made first. If a later change gives the core a writable handle
by some other route, this lint will not be what catches it.

The real implementation lives in `typdoc`, beside `Env`'s, and the fake lives in
`typdoc-testkit`.

### What the tests use: both, against one table

A fake in memory and a real temporary directory, with the same scenarios run against both.

**The fake exists for the failures a real filesystem will not produce on demand:** no space left,
a permission refused, a rename across devices, a link refused because the name is taken, and a
run stopped between any two operations — which is the failure decisions 1 and 4 are about and the
one no real directory will stage. The fake can be told to fail the third call, or the call after
the temp file is written and before it is renamed, and a test can then look at what is on disk.

**The real directory exists because a fake proves nothing about the implementation.** The same
scenario table runs against a real temporary directory for every case a real filesystem can be
made to reach, so the fake is held to what the real one does rather than to what it was written
to do. A case the real filesystem cannot reach is marked in the table as fake-only, and the table
is the record of which those are.

**What neither covers, recorded as not covered:** a power cut, and a `SIGKILL` between the write
and the rename at the level of the system call. Decision 4 already says v1 does not promise the
absence of a leftover for exactly this reason. Signals that can be caught are tested as story 1's
decision 9 settled, by holding a lock in a shipped binary with no test code in it, and that is a
different mechanism from this seam and stays where it is.

### `Clock`

**The clock gives an instant together with the offset to write it in; the formatting is
typdoc's.** `auto: create` and `auto: update` are `datetime` fields, and the design's `datetime`
is ISO 8601 with an offset, `2026-09-19T14:30:00+07:00`. If the clock returned the finished
string, typdoc's formatting would have no test: the test would be checking a value it had handed
in. Returning the instant and the offset keeps the formatting in typdoc where a golden file can
hold it to its word, and keeps the offset injectable, so a test's output does not change with the
machine it runs on.

**Resolution is one second**, which is what the design's own `datetime` shows and what its format
can carry; a finer instant would be written away and would not read back as itself.

**In tests the clock is fixed**, so a golden file can contain a time. The build picks the value;
it should be one that is obviously not now.

**Which offset the shipped clock reports — the machine's or UTC — is not decided here.** The seam
does not force it either way, which is the property wanted at this stage. It is user-visible: the
same document edited by two people in different places would carry different offsets, which
changes no comparison, since `datetime` compares as instants, but does change what a diff looks
like. Recorded as open.

### Reads: the seam covers writes, and the three reads a write makes

Story 1 reads the filesystem directly, in `index`, `namespaces`, `config`, `schema`, `state` and
`project`. That stays. Routing it through a seam would be a large change to finished, tested code
that no decision in this story needs.

The asymmetry is deliberate and this is the reason: a read that goes wrong gives a wrong answer,
and the next run gives the right one; a write that goes wrong damages a file the user owns, and
there is no next run that undoes it. They deserve different amounts of ceremony.

**The exception is the reads a write makes to decide what to do**, and there are three: whether
the destination exists, what an existing file's mode is, and whether two paths are the same file.
All three are on the seam. They are there because the write path's own rules depend on them —
decision 15's refusal, decision 4's mode carry-across, and the re-run rule above — and a fake
that could not answer them could not be used to test any of those rules.

**The cost, stated:** a read failure in the project-wide reading still cannot be provoked in a
test. That is unchanged from story 1, and no decision in story 2 rests on it.

### Written into `docs/design.md`

Nothing. Every part of this decision is internal: which trait the program writes through, what the
lint forbids, and what the tests use. Nothing here changes what any command promises, and no
decision already made is reopened: [decision 15](15-a-write-whose-destination-already-exists.md)'s
refusal at exit 7 and [decision 1](1-a-mv-that-fails-partway.md)'s re-run that finishes the work
both stand exactly as they were written.
