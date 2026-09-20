# 5: Schemas: field types, coercion, refs and targets

Type: implementation
Status: resolved
Blocked by: 4

## What this delivers

- The schema format: field types, `code`, `extends`, `target`, `acyclic`, defaults, `auto` and `transitions` as declared data, and enum order.
- Coercion of frontmatter values by type, so `get` returns typed `fields`; a value is never guessed from its look.
- The config errors that need a schema: `config.collection-schema`, `config.match-template`, `config.coded-schema-shared` and `config.schema-url`.

## Done when

- A fixture for each field type, with the expected typed value written by hand, including the strings `no`, `yes`, `on` and `off`.
- Each of the four config errors has a fixture in `broken/` and an exact expected id set.
- Schemas that are wrong are reported by `schema.valid` in ticket 9, not here.
