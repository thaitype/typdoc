# 3: One order for taking namespace locks

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

## Question

The design gives three orderings for taking locks, and they are not the same one:

- `mv`: "Takes the lock of every namespace it writes, in path order." Repeated under Concurrency: "`mv` takes the lock of every namespace it writes, in path order, so two `mv`s cannot deadlock."
- `mv --renumber`: "It holds the locks of both namespaces (in name order)."
- Pins: "one that needs both takes the project lock first, then the namespace locks in name order, so two commands cannot deadlock."

Path order and name order are not the same order. The namespace `default` has no folder of its own, so its documents' paths carry no namespace name at all; a namespace's folder name and the path that sorts it are different strings, and once an imported project is in the set (ticket 2) the paths come from a different tree entirely while the names do not. Two commands that take the same two locks in two different orders deadlock, which is precisely what each of the sentences above claims cannot happen.

Decide:

- One order for every command that takes more than one namespace lock, stated once and referred to everywhere else. Name it exactly, including how an imported project's namespace sorts against one of this project's.
- Whether the tie is broken by something that cannot collide, since two namespaces in two projects can have the same name.
- Amend all three places in `docs/design.md` so the document states one rule rather than three. The deadlock-freedom claim is only true once they agree.
- Whether "every command takes its locks through one acquisition path" is a rule of the code, which story 1's test strategy already relies on for the signal tests: the tests hold a lock in a shipped binary, and that only proves something about every command if there is one path to prove it about.

## Answer

**Decided: order by the path of the lock file itself.** A command that takes more than one lock
takes the project lock first, then every namespace lock in the order of the lock files' own paths,
compared byte by byte as absolute paths. Not the namespace's name, and not the document's path.

Why that and not the other two:

- What is being taken is the lock file, so ordering by the lock file is a total order by
  construction. Two lock files that are not the same file have different paths, so two of them can
  never compare equal and no tie-break is needed.
- A namespace's name cannot order them: two projects can have namespaces with the same name, and
  once an imported project is in the set (decision 2) the names collide with nothing to separate
  them.
- A document's path cannot order them either: the namespace `default` has no folder of its own, so
  its documents' paths carry no namespace at all and there is nothing to sort.
- One rule covers both lock modes without a second rule for `git-common`, because in both modes
  every lock file has a unique absolute path.
- It covers a namespace that belongs to an imported project without anyone having to decide whose
  project sorts first. That question does not arise.

**The project lock is still taken before any namespace lock**, as the design already said; this
decision changes nothing there.

**Canonical form, and the one mechanical point this raised.** A path is canonicalized before it is
compared, so that two spellings of one file are one lock. `std::fs::canonicalize` on a path whose
file does not exist returns `NotFound`, checked by running it, and the lock file does not exist at
the moment the order is decided — that is the whole point of deciding the order. So what is
canonicalized is the directory that will hold the lock file, with the file's name joined to it, and
the directory is created before the first lock is taken. The rule and its reasons are unchanged;
this is only how it is applied.

**One acquisition path: yes.** Every command takes and releases its locks through one piece of
code, and something has to enforce it rather than the next person remembering. Which mechanism
enforces it is decision 7, which is not settled yet.

**Written into `docs/design.md`:** a new `Lock order` bullet under Concurrency holds the rule, and
the three places that each stated a different order — `mv`, `mv --renumber`, and the paragraph on
pins — now refer to it, as does the `What is locked` bullet. The claim that two commands cannot
deadlock is only true once they agree, which is what this change makes true.
