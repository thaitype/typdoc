# 14: How is exit 1, which covers three unrelated outcomes, split?

Type: wayfinder:grilling
Status: resolved
Blocked by: None (can start immediately)

## Question

Exit 1 covered not found, bad arguments and I/O, and the design also gave it to a key or write that is ambiguous across namespaces. A caller that has to act differently for each cannot tell them apart without reading the message, although the design says exit codes exist so that a caller can branch without parsing text. The design also called a query that breaks the grammar an error without giving it an exit code. The table is shared by every command, and the read commands produce these outcomes from the first ticket, so it has to be settled before the shape of `--json` output.

## Answer

Decided: the outcomes are split by code.

| Code | Meaning |
| --- | --- |
| 0 | Success, including an empty `list` result and a well-formed query that matches nothing |
| 1 | Bad arguments: a malformed option or expression (a query that breaks the grammar included), or a key or write that is ambiguous across namespaces |
| 2 | Validation failed |
| 3 | An `--if` condition was false; nothing written |
| 4 | Lock not acquired within the timeout |
| 5 | Not found: the key, path or file the command was asked to act on does not exist |
| 6 | I/O: a file or directory cannot be read or written |

The criterion, written into the design next to the table so that it outlives this ticket: a new code is added only when the caller has to act differently. Not found may lead to creating the document. Bad arguments are a defect in the call and are not retried. An I/O failure is a problem of the environment and may be retried. Finer detail belongs in an id: every error carries in `details[].rule` an id that names its specific cause, and the ids are fixed together with the shapes of the output. Which side of the line a particular error falls on (an unknown namespace name, an unknown collection, an unknown field in a query) is settled with those ids.

A query that breaks the grammar exits 1. A well-formed query that matches nothing exits 0 with an empty result and is not an error. The two are easily confused, and a test that asserted the wrong one would be wrong from the day it was written, so the design says both.

Project rule 5 now says that the commands use the exit codes the design defines, and that the table in the design is the only place they are listed. It used to copy the range (0 to 4), which meant that every new code needed a change to the rule as well, and two places that state one fact drift apart, the one nobody edited being the one people read. Changing it now costs nothing: no release exists, no caller checks for exit 1, and the script in the worked examples is this project's own.

Not decided here: whether an I/O failure on the network shares code 6 with one on a disk. It belongs to the story that adds the fetch adapter, and the criterion above is what answers it then: does the caller act differently?

Consistency with the decision on interrupts (ticket 9): that decision kept project rule 5 unchanged and this one changes it. The measure is the same in both: do not pay the cost of amending a rule for a worse result. For an interrupt, the alternative gave the caller less information and needed the rule changed, so it was not worth it. Here the change gives more information, as the design says it intends, and the rewording of the rule means it is not changed again for a code. The fallback recorded in ticket 9 (an interrupt exits with 128 plus the signal number, if the scene cannot be made) now needs only a row in the design's table.

Rejected: keeping one code and putting the cause in an id in `--json`, because the caller would have to read JSON to branch, which is what the design says exit codes are there to avoid, and a shell script could not branch at all. Rejected: leaving it to a later story, because tests assert exit codes from the first ticket and golden files cannot be written before the codes are settled.
