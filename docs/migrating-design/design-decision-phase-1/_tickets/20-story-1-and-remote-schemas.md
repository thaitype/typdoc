# 20: Is story 1 released on its own, and what does a read command do with a remote schema that has no pin?

Type: wayfinder:grilling
Status: resolved
Blocked by: 15

## Question

The design says a read command fetches a remote schema that has no pin, stores it and records the pin. The fetch adapter, the project lock and `pull` belong to story 3, so story 1 cannot do it.

## Answer

Decided: story 1 is an internal milestone and is not released on its own; v1 is the three stories together. Until story 3 a remote schema with no pin is reported as `config.schema-unpinned`, an id that exists and means exactly this ("a remote schema has no pin and cannot be fetched"). Pinned copies that are already present are read as usual, so fixtures can carry hand-made ones.

The cost, stated where a decision about demonstrating story 1 will see it: it cannot be tried on a project whose schemas are remote until story 3 is done. That limits what can be shown of `validate --audit` on real documents. It is also in the map's notes.

The allocation of the test strategy for the rows about pinned copies (ticket 9) follows from this decision.

Rejected: releasing story 1 as a read-only tool that supports only local schemas and schemas pinned by hand. It would need a way to pin by hand, which is what `pull` is for, and a promise to users that is smaller than v1's.
