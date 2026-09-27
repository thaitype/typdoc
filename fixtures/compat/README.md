# Projects from earlier releases

Each folder `<version>/` holds typdoc projects that stand for what a project made for that release
could hold. `crates/typdoc/tests/compat.rs` runs `typdoc validate` on every one of them and
requires exit 0: a project that passed before an upgrade passes after it (PRN-7).

| Project | Made for | Where it came from |
| --- | --- | --- |
| `0.1.0/examples` | 0.1.0 | `examples/` of this repository at tag `v0.1.0`, unchanged |
| `0.1.0/tickets-notes` | 0.1.0 | Written: coded and path-identified collections, `extends`, enum with `transitions`, `default`, `auto` (`create`, `update`, `moves`), `number` values (`0.5`, `3`, `1e3`), an `acyclic` ref list, a ref to a document without a code, body links with anchors, a key mentioned in text, a `validation` level in `config.json` |
| `0.1.0/stories` | 0.1.0 | Written: namespaces from a glob (`story-*`), a state file per namespace, a ref into a sibling namespace (`story-1:WF-2`) |
| `0.1.0/with-import/app` and `memory` | 0.1.0 | Written: an import (`memory`), a ref field whose target is a schema of the import, a ref `memory::LRN-1` |
| `0.3.0/stories-excluded` | 0.3.0 | Written: a `!` exclusion in `namespaces`; the excluded folder holds a file that is never checked |
| `0.3.1/examples` | 0.2.0 to 0.3.1 | `examples/` of this repository at tag `v0.3.1` (unchanged since `v0.2.0`) |
| `0.3.1/chief-example` | 0.3.1 | `docs/example-chief/` of https://github.com/thaitype/chief at `abd1dda`, unchanged |

0.2.0 has no folder of its own: it changed what typdoc prints, not what a project can hold.

## Adding one

When a release lets a project hold something new, add a project under that version that uses
it, with made-up content. Before it is merged, prove it belongs to that version: build the
binary from the release's tag and run `typdoc validate --json` in the project (0.1.0 has no text
output), expecting exit 0. A project that uses something newer fails on the older tags, which
is also worth checking. A project in real use with a shape these do not cover can be copied in
unchanged, under the version it was made for. Add a row here either way.

## Never edit these

A change that makes one of these fail breaks projects in use. Either the change is wrong, or the
break is intended and the changelog names it as an upgrade note. Never edit a project here to
make it pass.
