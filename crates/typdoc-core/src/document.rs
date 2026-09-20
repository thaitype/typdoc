/// A frontmatter value. `Text` and `List` are a value as it is written, which is what a value
/// stays when it does not fit the type its schema gives it; the others are a value that does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Text(String),
    List(Vec<String>),
    Number(serde_json::Number),
    Bool(bool),
    /// A calendar date written `YYYY-MM-DD`, as written.
    Date(String),
    /// A date and time with an offset, as written.
    Datetime(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// Relative to the project the document belongs to (its own project folder when `project` is
    /// `None`, the imported project's when it is `Some`).
    pub path: String,
    pub namespace: String,
    /// Present only when the schema has a code.
    pub key: Option<String>,
    /// The schema's code, if it has one.
    pub code: Option<String>,
    pub collection: String,
    pub schema: String,
    /// The alias this document was reached through, when it belongs to an imported project;
    /// `None` for a document of this project (design: "`project`... is absent for a document of
    /// this project").
    pub project: Option<String>,
    /// In the order of the file.
    pub fields: Vec<(String, Value)>,
}
