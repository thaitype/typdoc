//! Reads a typdoc project and answers questions about it. It changes nothing.

mod config;
mod document;
mod env;
mod error;
mod frontmatter;
mod glob;
mod index;
mod project;
mod schema;

pub use document::{Document, Value};
pub use env::{Deps, Env};
pub use error::{Error, ErrorKind};
pub use project::{DocumentArg, Project, discover};
