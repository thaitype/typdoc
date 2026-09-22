//! Reads a typdoc project and answers questions about it. It changes nothing.

// A panic in the code that ships needs its reason written where it stands. Test code is left alone.
#![cfg_attr(
    not(test),
    deny(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::unreachable,
        clippy::todo,
        clippy::unimplemented
    )
)]

mod argument;
mod body;
mod clock;
mod coerce;
mod config;
mod document;
mod env;
mod error;
mod frontmatter;
mod fs;
mod imports;
mod index;
mod lines;
mod links;
mod lock;
mod namespace_lock;
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
pub use clock::{Clock, at_one_second};
pub use coerce::{coerce, fits};
pub use config::{Collection, Config, Level, LockMode, Namespace, RefBase, RuleSetting, Rules};
pub use document::{Document, Number, Value};
pub use env::{Deps, Env};
pub use error::{ConfigError, Error, ErrorKind};
pub use frontmatter::{FrontmatterWriter, YamlSerdeWriter};
pub use fs::{FileId, Fs, Mode, TEMP_PREFIX, WriteHandle, is_temp_name, write_atomically};
pub use lines::{LineMap, Position};
pub use links::{
    BodyLink, BodyLinks, Definition, DuplicateDefinition, Mention, Suspect, mentions, scan,
};
pub use namespace_lock::{
    NamespaceLock, Released, acquire, git_common_namespace_lock_path, git_common_project_lock_path,
    local_namespace_lock_path, local_project_lock_path, order_locks, project_hash, project_hash_of,
    release,
};
pub use project::{
    AuditCollection, AuditOverlap, AuditReport, ListFilter, ListResult, Project, RefName,
    RefOutcome, RefsDirection, RefsReference, RefsReport, SetOp, SortKey, Toc, ValidateReport,
    discover,
};
pub use query::{
    Condition, Dir, FieldRef, Item, Op, PlainCondition, Quant, QueryError, RefCondition, RefField,
    evaluate, parse as parse_query, parse_field,
};
pub use schema::{Auto, Field, FieldType, OptBool, Resolved, Schema, Target};
pub use scope::{Scope, Source};
pub use validate::{Finding, Severity, ValidateScope};
