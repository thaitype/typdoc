# 7: The write seam in `deps`, and `Clock`

Type: wayfinder:grilling
Status: open
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

<filled in on resolve>
