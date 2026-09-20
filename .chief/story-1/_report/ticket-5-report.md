# Ticket 5 Report

## Ticket
The schema format read as declared data, coercion of frontmatter values by the type the schema gives, and the four config errors that need a schema: `config.collection-schema`, `config.match-template`, `config.coded-schema-shared` and `config.schema-url`.

## Outcome
done

## Decision
Nothing blocked the build. The design does not say the following, so each is a reading that a later ticket may change, with the doubt that remains.

- **`bool` is exactly `true` or `false`.** The design names the type and says values are ordinary YAML, and does not say which spellings are a bool. `yes`, `no`, `on`, `off`, `True`, `TRUE`, `1` and `0` stay text. Doubt: YAML 1.2 also allows `True` and `TRUE`; widening later changes what `get` prints for them.
- **A number is a JSON number.** `0755`, `+3`, `.5`, `0x1F` and `1_000` are not numbers. Doubt: a `.5` written by a user stays text and is flagged by `frontmatter.types` in ticket 8.
- **A date is `YYYY-MM-DD` and a real day; a datetime is RFC 3339** (upper-case `T`, seconds, `Z` or `±hh:mm`). `+0700`, no seconds, and a space in place of `T` stay text. Doubt: the design says "ISO 8601 with offset", which is wider and does not name a subset.
- **A value that does not fit its type is shown as written, and `get` still exits 0.** `coerce` returns `None` for a misfit, which is what ticket 8 needs to report it under `frontmatter.types`. Doubt: until `validate` exists, a consumer of `fields` can see a string in a `number` field.
- **The schema decides, not the quoting in the file.** `count: "3"` reads as 3 for a `number`, because the reader cannot tell a quoted scalar from a plain one.
- **Empty and null-looking values are text:** `x:` gives `""` and `~` gives `"~"`.
- **A mapping, or a list holding non-scalars, still exits 2 with no id** and names the field (ticket 1's behaviour). Doubt: a real document with a nested field makes `get` fail.
- **A schema with a wrongly typed option** (`required: "yes"`, a missing `name` or `fields`) is an id-less exit 2, while an unknown type, `auto` or `target` name is read tolerantly so that ticket 9 can report it under `schema.valid`. Doubt: the two are inconsistent, and ticket 9 will need to relax the first.
- **`extends` is read relative to the folder of the schema that names it,** and a collection's `schema` relative to the project folder. The design shows `./base-ticket.json` and does not say relative to what. A cycle in `extends` ends the chain silently; rejecting it is ticket 9's.
- **A missing parent schema is `config.collection-schema`** at the collection file, with the parent named. Doubt: it could belong to `schema.valid`.
- **`config.coded-schema-shared`** compares the normalized schema path (`./a.json` and `a.json` are one) and reports one error per extra collection, on the collection whose name sorts later. Two different schema files with the same code are left to `schema.valid`.
- **`config.schema-url`** applies to any `scheme://` other than `http` and `https`, in a collection's `schema` or in `extends`. An `http` or `https` URL stays an id-less exit 2 and no request is made. `config.schema-unpinned` needs `lock.json` and is ticket 18's. The scheme match is case-sensitive, so `HTTPS://x` reports `config.schema-url`. Doubt: schemes are case-insensitive in general.
- **An id-less stop does not hide ids:** the config errors with ids are returned first, and an id-less fault appears once they are gone.

## Notes
- Run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (254 passed and 1 ignored, from 208) and `scripts/check-public-text.sh` with the names list, all green.
- The four ids moved from `UNIMPLEMENTED_RULES` to `RULES`, each with a fixture that trips exactly its own id. New fixtures: `valid/field-types` (one field of each type, and `no`, `yes`, `on`, `off`, `1.10` and `0755` in string fields) and three hand-asserted goldens under `fixtures/output/get/`.
- Re-run by hand, each planted, red, removed: `no` read as false, and the date shape check removed. The ticket build also showed red: config errors hidden by an id-less stop, number guessing, a lenient date, reshaped text, an extra id tripped, a wrong id reported, a number printed as text, extends parents ignored, and the lint bans on the new files.
- Only red as a hang: with the cycle guard removed the run never ends (killed by a timeout), because there is no in-test timeout.
- Removed with this ticket: `read_json` in `config.rs` and one ticket-1 test, `a_number_is_refused_and_not_read_as_a_guess`, whose behaviour this ticket replaces.
- Not shown: the text form of a multi-error failure (`get` without `--json` exits 1 first), and a non-UTF-8 `TYPDOC_DIR`.
