# 3: One order for taking namespace locks

Type: wayfinder:grilling
Status: open
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

<filled in on resolve>
