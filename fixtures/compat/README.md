# Projects from earlier releases

Each folder here is a typdoc project exactly as it was at an earlier release, copied from a
public source and never edited. `crates/typdoc/tests/compat.rs` runs `typdoc validate` on every
one of them and requires exit 0: a project that passed before an upgrade passes after it (PRN-7).

| Folder | Made for | Copied from |
| --- | --- | --- |
| `0.1.0/examples` | typdoc 0.1.0 | `examples/` of this repository at tag `v0.1.0` |
| `0.2.0/typdoc-design` | typdoc 0.2.0 | `.typdoc/` and `docs/design/` of this repository at tag `v0.2.0` |
| `0.3.1/typdoc-design` | typdoc 0.3.1 | `.typdoc/` and `docs/design/` of this repository at tag `v0.3.1` |
| `0.3.1/chief-example` | typdoc 0.3.1 | `docs/example-chief/` of https://github.com/thaitype/chief at `abd1dda` |

Not every release needs a folder. Add one when a release changes what a project can hold, or when
a project in real use has a shape these do not cover: copy it unchanged under the version it was
made for, and add a row here.

A change that makes one of these fail is a break for projects in use. Either the change is
wrong, or the break is intended and the changelog names it as an upgrade note. Never edit a
project here to make it pass.
