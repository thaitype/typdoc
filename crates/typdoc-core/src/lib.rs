//! Reads a typdoc project and answers questions about it. It changes nothing.

mod argument;
mod body;
mod coerce;
mod config;
mod document;
mod env;
mod error;
mod frontmatter;
mod imports;
mod index;
mod lines;
mod links;
mod lock;
mod namespaces;
mod project;
mod query;
mod refs;
pub mod rules;
mod schema;
mod scope;
mod slug;
mod state;
mod template;
mod validate;

pub use argument::{Argument, DocumentArg, discover_for, resolve_on_disk};
pub use body::{Heading, headings};
pub use coerce::{coerce, fits};
pub use config::{Collection, Config, Level, Namespace, RefBase, RuleSetting, Rules};
pub use document::{Document, Value};
pub use env::{Deps, Env};
pub use error::{ConfigError, Error, ErrorKind};
pub use lines::{LineMap, Position};
pub use links::{
    BodyLink, BodyLinks, Definition, DuplicateDefinition, Mention, Suspect, mentions, scan,
};
pub use project::{
    AuditCollection, AuditOverlap, AuditReport, ListFilter, ListResult, Project, RefName,
    RefOutcome, RefsDirection, RefsReference, RefsReport, SortKey, Toc, ValidateReport, discover,
};
pub use query::{
    Condition, Dir, FieldRef, Item, Op, PlainCondition, Quant, QueryError, RefCondition, RefField,
    evaluate, parse as parse_query, parse_field,
};
pub use schema::{Auto, Field, FieldType, OptBool, Resolved, Schema, Target};
pub use scope::{Scope, Source};
pub use validate::{Finding, Severity, ValidateScope};
