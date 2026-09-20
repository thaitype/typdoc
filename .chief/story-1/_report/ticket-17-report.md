# Ticket 17 Report

## Ticket
Imports: finding other projects on this machine, following refs and arguments into them, and saying so when one is not there.

## Outcome
done

## Decision
Nothing the design or the contract asks for turned out to be impossible. The design does not say the following, so each is a reading that a later ticket may change, with the doubt that remains.

- **Three refusals left by earlier tickets are now real behaviour, not extended.** A `name::` ref used to be refused as a bad prefix whatever the name was; it now resolves when the alias is configured and the project is there, stays a bad prefix when the alias is unknown, and is `import-absent` when the alias is configured and the project is not on this machine, which is a third answer and not a shade of either. A `project::` argument used to exit 1 as not read yet, and now names a document, with `project` in its name. `import-absent` is a reason a reference can carry, which was listed in the output's shape before anything produced it.
- **The machine file adds to the project's own imports and never overrides them.** An alias in both wins from the committed config, since that is the one everybody shares and the machine file exists to add what cannot be committed.
- **A variable that is unset or empty leaves the import absent** rather than being replaced by nothing, so `${HOME}/projects` never quietly becomes `/projects`, which could exist and be the wrong project.
- **An empty `TYPDOC_CONFIG_DIR` counts as unset,** as the design says for the variable beside it and as this project already treats its own variables.
- **A relative import path is resolved from the importing project's folder,** which the design does not say, and which matches how every other path in the config is anchored.
- **An import whose location is there but whose own config is broken is a plain error, not `imports.absent`.** Absent means not on this machine; a project that is here and cannot be read is a different problem with a different fix.
- **A malformed machine file is an error with no id,** since the design's table gives none and the file is never committed, so no fixture in a public repository could carry one.
- **An argument naming an unknown or an absent alias is bad arguments either way,** which is how an unknown namespace prefix already behaves.

## Open against the design, carried to the end of the story
- **Reverse lookup does not enter an imported project.** The design says a reverse lookup scans every namespace of this project *and the projects it imports*, and its own table says `refby` sees refs from the imported projects too. This story builds neither: `refs --reverse` refuses a `project::` argument outright rather than answering it partly, but a `refby` condition in a query quietly searches only this project, so a document that an imported project points at can be reported as pointed at by nothing. No ticket of this story owns this, which is why it is written here rather than left in a comment.
- **`refs.target` and `refs.codedByPath` are not checked for a ref that crosses into an import,** because the check needs the imported project's own qualified schema names, which the ticket that reads pinned schemas is the first to assemble.
- **`body.anchors` is not checked across an import:** the file is checked for existence, its headings are not read.
Each of the three is marked in the code where it would otherwise silently under-deliver.

## Notes
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (627 passed and 1 ignored, from 579) and `scripts/check-public-text.sh` with the names list, all green, every cargo command under a memory ceiling.
- Re-run by hand, planted inside a function body, red with the error naming the banned call, removed: `std::env::var_os` in the substitution of variables in an import path. That ban matters most here, since this is the ticket that reads the environment to find a file.
- Checked by running, not by reading: a ref into a present import resolving with its `project` and namespace, including one into a project with several namespaces; a ref into an absent import reported as `import-absent` from frontmatter and from a body link; `get` and `toc` on a `project::` argument; an absent alias as an argument; and `--namespace` with an alias pattern listing an imported project's documents.
- No test reads the real home directory: the five ways of finding the machine file are exercised with a stand-in environment, as the design requires.
- A test written for the document that belongs to no collection passed for the wrong reason at first, because the written path survived even under the wrong project; it was replaced with a fixture whose namespace is not `default`, so the assertion can tell the two apart. A test that cannot fail under the fault it names is worth nothing, and this one was caught by reverting the fix and watching it stay green.
- A second fault surfaced while checking by hand rather than from a test: a frontmatter ref that crossed an import printed no `project` and the namespace of the wrong project. It is fixed and pinned.
