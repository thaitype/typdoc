//! The rules and config errors the binary has, by id, and the ones it does not have yet.

/// The always-on rules.
pub const ALWAYS_ON: &[&str] = &[
    "schema.valid",
    "frontmatter.parse",
    "frontmatter.types",
    "frontmatter.transitions",
    "refs.resolve",
    "refs.target",
    "refs.acyclic",
    "keys.unique",
    "collections.overlap",
    "state.missing",
];

/// The configurable rules, each with the options it accepts besides `level`.
pub const CONFIGURABLE: &[(&str, &[&str])] = &[
    ("body.links", &["ignore"]),
    ("body.anchors", &[]),
    ("body.mentions", &["inlineCode", "fencedCode"]),
    ("refs.codedByPath", &[]),
    ("refs.moved", &[]),
    ("names.shadowed", &[]),
    ("frontmatter.unknown", &[]),
    ("filename.pattern", &[]),
    ("imports.absent", &[]),
];

/// The ids the binary reports. An id that is here needs a fixture in `fixtures/broken/`.
pub const RULES: &[&str] = &[
    "config.parse",
    "config.version",
    "config.unknown-key",
    "config.legacy-file",
    "config.collection-parse",
    "config.collection-name",
    "config.collection-schema",
    "config.rule-unknown",
    "config.rule-always-on",
    "config.match-template",
    "config.coded-schema-shared",
    "config.schema-url",
    "config.namespaces-entry",
    "config.namespace-name",
    "config.namespace-nested",
    "config.config-dir",
    "frontmatter.parse",
    "frontmatter.types",
    "frontmatter.unknown",
    "schema.valid",
    "keys.unique",
    "collections.overlap",
    "filename.pattern",
    "refs.resolve",
    "refs.target",
    "refs.acyclic",
    "refs.codedByPath",
    "refs.moved",
    "names.shadowed",
    "body.links",
    "body.anchors",
    "body.mentions",
    "imports.absent",
];

/// Ids the design names and the binary does not report yet. Each is a difference between
/// the design and the binary, with the story expected to deliver it as a comment. The list only shrinks: an id leaves it
/// when it enters `RULES`, and it is empty when v1 is finished.
pub const UNIMPLEMENTED_RULES: &[&str] = &[
    "frontmatter.transitions", // story 2
    "state.missing",           // story 1
    "config.state-uncoded",    // story 1
    "config.state-orphan",     // story 1
    "config.schema-unpinned",  // story 1
    "config.vendor-missing",   // story 1
    "config.vendor-edited",    // story 1
];
