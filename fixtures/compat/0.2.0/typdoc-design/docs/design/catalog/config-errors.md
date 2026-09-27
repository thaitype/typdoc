---
title: Config errors catalog
content_type: json
explained_by: [SPC-6]
---

{
  "errors": [
    { "id": "config.parse", "reported_when": "`config.json` cannot be parsed" },
    { "id": "config.version", "reported_when": "`version` is missing or unknown" },
    {
      "id": "config.unknown-key",
      "reported_when": "`config.json` or a collection file has an unknown key (`name` in `config.json` and `last` in a collection file included)"
    },
    { "id": "config.collection-parse", "reported_when": "a collection file cannot be parsed" },
    {
      "id": "config.collection-name",
      "reported_when": "a collection file's name uses anything but ASCII letters, digits, `-` and `_`"
    },
    {
      "id": "config.collection-schema",
      "reported_when": "a collection names a schema that does not exist"
    },
    { "id": "config.rule-unknown", "reported_when": "a rule name or option is unknown" },
    { "id": "config.rule-always-on", "reported_when": "an always-on rule is configured" },
    {
      "id": "config.match-template",
      "reported_when": "a `match` template breaks the placeholder rules"
    },
    {
      "id": "config.coded-schema-shared",
      "reported_when": "two collections name the same coded schema"
    },
    {
      "id": "config.state-uncoded",
      "reported_when": "a state entry names a collection this project has whose schema has no code. An entry for a collection the project no longer has is not this error; it is the finding `state.retired`"
    },
    {
      "id": "config.state-orphan",
      "reported_when": "a file in `.typdoc/state/` matches no current namespace (the message names the file and says to delete or rename it)"
    },
    {
      "id": "config.namespaces-entry",
      "reported_when": "a `namespaces` entry contains `/` or `**`, names a folder that does not exist, or names a symbolic link"
    },
    {
      "id": "config.namespace-name",
      "reported_when": "a matched folder's name uses anything but ASCII letters, digits, `-` and `_`, or is `default`, `http`, `https`, `mailto` or `file`"
    },
    { "id": "config.namespace-nested", "reported_when": "a matched folder holds its own `.typdoc`" },
    {
      "id": "config.schema-url",
      "reported_when": "a schema URL uses a scheme other than `http://` or `https://`"
    },
    {
      "id": "config.schema-unpinned",
      "reported_when": "a remote schema has no pin and cannot be fetched"
    },
    {
      "id": "config.vendor-missing",
      "reported_when": "a pinned copy is missing (run `typdoc pull`)"
    },
    {
      "id": "config.vendor-edited",
      "reported_when": "a pinned copy's contents hash differently from its file name (it was edited by hand; run `typdoc pull`)"
    },
    {
      "id": "config.config-dir",
      "reported_when": "`TYPDOC_CONFIG_DIR` is set but is not an absolute path to an existing directory"
    }
  ]
}
