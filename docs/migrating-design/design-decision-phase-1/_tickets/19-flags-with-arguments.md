# 19: What does `validate` do when `--schemas` or `--audit` is combined with key or path arguments?

Type: wayfinder:grilling
Status: resolved
Blocked by: 15

## Question

The design gives `validate` arguments and two flags that each describe the whole project, and does not say what a combination means.

## Answer

Decided: combining either flag with arguments is bad arguments, exit 1. A flag that widens the report to the whole project cannot be combined with arguments that narrow it, and an error is louder than choosing a meaning. The values of `scope` in the summary (`all`, `paths`, `schemas`) then never mix.

Rejected: letting the arguments filter the findings while the audit lists stay project-wide, which would give a report that is partly one thing and partly another. Rejected: reading the arguments as schema files when `--schemas` is given. It is a wish that has a real use, checking only the schemas that were edited, in a pre-commit hook. It is a different feature (checking the schema files named) and not a combination of these two flags, so it is recorded on the map as possible later and not built half way now.
