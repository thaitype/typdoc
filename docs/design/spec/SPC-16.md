---
title: Pinned remote schemas explained
status: active
migrated_from: docs/archived-design/design.md#config-typdocconfigjson
---

`.typdoc/lock.json` holds a pin for every remote schema a project uses: the SHA-256 of the copy
that was fetched, and when it was fetched. It has no version of its own, since the `version` in
`config.json` covers every file typdoc owns (`SPC-7`). It is split into sections so that pins
other than schemas can join it without a rename, and so a reader takes the `schemas` section and
leaves any other top-level key alone. A project with no `lock.json` has no pins, and a
`lock.json` that cannot be read as this shape stops the command.

```json
{
  "schemas": {
    "https://schemas.example.dev/chief/wayfinder/v1.json": {
      "sha256": "9f2c…", "fetchedAt": "2026-09-19T14:30:00+07:00"
    }
  }
}
```

The pinned copy of a schema is read from `vendor/schemas/<sha256>`, and its contents are hashed to
check them against that name. A URL with no pin is `config.schema-unpinned`, a pinned copy that is
not there is `config.vendor-missing`, and one whose contents hash to another name is
`config.vendor-edited` (`SPC-6`).
