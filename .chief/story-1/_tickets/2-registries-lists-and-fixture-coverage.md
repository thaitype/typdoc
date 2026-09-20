# 2: Registries, the three lists of acknowledged differences, and fixture coverage

Type: implementation
Status: resolved
Blocked by: 1

## What this delivers

- The registry of commands and `unimplemented_commands`; the registry of rules, whose ids the test reads from the design's rule tables, and `unimplemented_rules`; `unproduced_exit_codes`.
- The two checks for each list (testing decisions): everything the design names is in the registry or in the list, and nothing in a list is in the registry or produced by a test.
- `fixtures/broken/<rule>/` with the two coverage checks (every folder names a rule that exists, every rule that exists has a fixture) and the exact-set assertion on the rules a fixture trips; a rule in `unimplemented_rules` is exempt from the fixture requirement while it is listed.
- The loader that fails loudly outside a checkout.

## Done when

- Each check is shown red by planting a fault: a command built and left in the list, a rule in the design that is in neither place, a folder that names no rule, a rule with no fixture, a fixture that trips a second rule.
- Every list is written as a difference that is acknowledged, with the story expected to deliver each entry as a comment that no test reads.
