# 17: What can `validate --schemas` check about schemas of an imported project?

Type: wayfinder:grilling
Status: resolved
Blocked by: 15

## Question

The design promised that `validate --schemas` checks that every field used across namespaces still exists in the imported schemas, and said that without it queries silently return nothing.

## Answer

Decided: `validate --schemas` checks that each qualified `target` (`"memory::learning"`) names a schema that exists in the imported project. This is a smaller promise than the design made, and it is written down as what happened, not as if it had been designed so. The promised check could not be made: nothing in a project's files records which fields of another schema it relies on. A query is typed on the command line, and a schema can name a schema of another project only by name in `target`; there is no place that names a field of it.

The reason the design gave for the check was also wrong. It said that a renamed field makes queries return nothing silently. The rule under Names and scope says that a field name unknown to every schema in scope is an error and not an empty result, and the scope after `ref.*(f)` is the schemas named by `f`'s `target`, imported ones included. So a renamed field makes the query fail loudly when it runs.

The finding is located in the schema file of this project and names the ref field whose `target` no longer resolves, which also settles which file such a finding names. It is reported under `schema.valid`, which is extended, and not under a new rule, so the registry that needs a fixture for every rule does not grow. An import that is absent on this machine is reported by `imports.absent`, as a warning, and not as an error here. The paragraphs on Schema drift and on `--schemas` in the design are rewritten to say only what can be checked.
