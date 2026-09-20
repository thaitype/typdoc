/// A frontmatter value as it is written: text, or a list of text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Text(String),
    List(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// Relative to the project folder.
    pub path: String,
    pub namespace: String,
    pub collection: String,
    pub schema: String,
    /// In the order of the file.
    pub fields: Vec<(String, Value)>,
}
