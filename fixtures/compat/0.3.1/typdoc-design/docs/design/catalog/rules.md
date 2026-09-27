---
title: Validation rules catalog
content_type: json
explained_by: [SPC-1]
---

{
  "rules": [
    { "id": "schema.valid", "configurable": false },
    { "id": "frontmatter.parse", "configurable": false },
    { "id": "frontmatter.types", "configurable": false },
    { "id": "frontmatter.transitions", "configurable": false },
    { "id": "refs.resolve", "configurable": false },
    { "id": "refs.target", "configurable": false },
    { "id": "refs.acyclic", "configurable": false },
    { "id": "keys.unique", "configurable": false },
    { "id": "collections.overlap", "configurable": false },
    { "id": "collections.empty", "configurable": false },
    { "id": "state.missing", "configurable": false },
    { "id": "state.malformed", "configurable": false },
    { "id": "state.behind", "configurable": false },
    { "id": "state.retired", "configurable": false },
    { "id": "files.unreadable", "configurable": false },
    { "id": "body.links", "configurable": true },
    { "id": "body.anchors", "configurable": true },
    { "id": "body.mentions", "configurable": true },
    { "id": "refs.codedByPath", "configurable": true },
    { "id": "refs.moved", "configurable": true },
    { "id": "names.shadowed", "configurable": true },
    { "id": "frontmatter.unknown", "configurable": true },
    { "id": "filename.pattern", "configurable": true },
    { "id": "imports.absent", "configurable": true }
  ]
}
