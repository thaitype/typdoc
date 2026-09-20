//! The rules and config errors the binary has, by id, and the ones it does not have yet.

/// The ids the binary reports. An id that is here needs a fixture in `fixtures/broken/`.
pub const RULES: &[&str] = &[];

/// Ids the design names and the binary does not report yet. Each is a difference between
/// the design and the binary, with the story expected to deliver it as a comment. The list only shrinks: an id leaves it
/// when it enters `RULES`, and it is empty when v1 is finished.
pub const UNIMPLEMENTED_RULES: &[&str] = &[
    "schema.valid",               // story 1
    "frontmatter.parse",          // story 1
    "frontmatter.types",          // story 1
    "frontmatter.transitions",    // story 2
    "refs.resolve",               // story 1
    "refs.target",                // story 1
    "refs.acyclic",               // story 1
    "keys.unique",                // story 1
    "collections.overlap",        // story 1
    "state.missing",              // story 1
    "body.links",                 // story 1
    "body.anchors",               // story 1
    "body.mentions",              // story 1
    "refs.codedByPath",           // story 1
    "refs.moved",                 // story 1
    "names.shadowed",             // story 1
    "frontmatter.unknown",        // story 1
    "filename.pattern",           // story 1
    "imports.absent",             // story 1
    "config.parse",               // story 1
    "config.version",             // story 1
    "config.unknown-key",         // story 1
    "config.legacy-file",         // story 1
    "config.collection-parse",    // story 1
    "config.collection-name",     // story 1
    "config.collection-schema",   // story 1
    "config.rule-unknown",        // story 1
    "config.rule-always-on",      // story 1
    "config.match-template",      // story 1
    "config.coded-schema-shared", // story 1
    "config.state-uncoded",       // story 1
    "config.state-orphan",        // story 1
    "config.namespaces-entry",    // story 1
    "config.namespace-name",      // story 1
    "config.namespace-nested",    // story 1
    "config.schema-url",          // story 1
    "config.schema-unpinned",     // story 1
    "config.vendor-missing",      // story 1
    "config.vendor-edited",       // story 1
    "config.config-dir",          // story 1
];
