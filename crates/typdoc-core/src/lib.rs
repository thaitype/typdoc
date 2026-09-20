//! Reads a typdoc project and answers questions about it. It changes nothing.

mod body;
mod coerce;
mod config;
mod document;
mod env;
mod error;
mod frontmatter;
mod index;
mod lines;
mod namespaces;
mod project;
pub mod rules;
mod schema;
mod scope;
mod slug;
mod template;

pub use body::{Heading, headings};
pub use coerce::coerce;
pub use config::{Collection, Config, Level, Namespace, RefBase, RuleSetting, Rules};
pub use document::{Document, Value};
pub use env::{Deps, Env};
pub use error::{ConfigError, Error, ErrorKind};
pub use lines::{LineMap, Position};
pub use project::{DocumentArg, Project, Toc, discover};
pub use schema::{Auto, Field, FieldType, Resolved, Schema, Target};
pub use scope::{Scope, Source};
