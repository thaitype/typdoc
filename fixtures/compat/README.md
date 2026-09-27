# Projects from earlier releases

Each folder here is a typdoc project exactly as it was at an earlier release, copied from a
public source and never edited. `crates/typdoc/tests/compat.rs` runs `typdoc validate` on every
one of them and requires exit 0: a project that passed before an upgrade passes after it (PRN-7).

| Folder | Made for | Copied from |
| --- | --- | --- |
| `0.1.0/examples` | typdoc 0.1.0 | `examples/` of this repository at tag `v0.1.0` |
| `0.3.1/examples` | typdoc 0.2.0 to 0.3.1 | `examples/` of this repository at tag `v0.3.1` (unchanged since `v0.2.0`) |
| `0.3.1/chief-example` | typdoc 0.3.1 | `docs/example-chief/` of https://github.com/thaitype/chief at `abd1dda` |

`examples/` at the repository root is where a release shows what a project can hold, with
made-up content that stands for real use. At a release that changes `examples/`, copy it here
unchanged as `<version>/examples` and add a row. Not every release needs a folder: one whose
`examples/` did not change adds nothing. A project in real use with a shape the examples do not
cover can be added the same way, under the version it was made for.

A change that makes one of these fail is a break for projects in use. Either the change is
wrong, or the break is intended and the changelog names it as an upgrade note. Never edit a
project here to make it pass.
