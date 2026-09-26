/// A frontmatter value typed `number`: the digits written in the document, since nothing between
/// the file and the caller decides what `1e3` is, and the value converted from them, which a
/// comparison, a sort and a table cell need. The digits stay exact however long they are; the
/// converted value is where a number no primitive holds loses its tail.
#[derive(Debug, Clone, Eq)]
pub struct Number {
    written: String,
    converted: serde_json::Number,
}

/// Equal when the converted values are, so `--where num=1000.0` matches a document holding `1e3`.
/// The digits take no part: they are for printing, not for comparing.
impl PartialEq for Number {
    fn eq(&self, other: &Number) -> bool {
        self.converted == other.converted
    }
}

impl Number {
    /// `written` as a number, or `None` when JSON does not read it as one: `0755`, `+3`, `.5` and
    /// `1e400` are not, and a field holding one keeps the text it was written as.
    pub fn read(written: &str) -> Option<Number> {
        let converted: serde_json::Number = written.parse().ok()?;
        Some(Number {
            written: written.to_owned(),
            converted,
        })
    }

    /// The digits written in the document, to the character: `1e3` is `1e3` and not `1000.0`,
    /// `1e+3` or anything else a reader would have made of it.
    pub fn written(&self) -> &str {
        &self.written
    }

    /// The value converted from the digits, as JSON prints it: `1e3` is `1000.0`, and
    /// `99999999999999999999` is `1e+20`, as `99999999999999999998` is. A glob, an equality, a
    /// sort key and a table cell read a number in this form.
    pub fn converted(&self) -> String {
        self.converted.to_string()
    }

    /// The converted value as an `f64`, for an ordering and a sort key.
    pub fn as_f64(&self) -> Option<f64> {
        self.converted.as_f64()
    }

    pub fn as_u64(&self) -> Option<u64> {
        self.converted.as_u64()
    }
}

/// A frontmatter value. `Text` and `List` are a value as it is written, which is what a value
/// stays when it does not fit the type its schema gives it; the others are a value that does.
///
/// `Empty` is a field written with a name and nothing after it (`reviewer:`), not one written as
/// the empty string (`reviewer: ''`). Everywhere a value is checked, compared, sorted or matched
/// it reads as `Text(String::new())` does. Only a write, which puts back the form it found, and
/// `--json`, which prints it as `null` rather than `""`, tell the two apart (SPC-4, SPC-12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Text(String),
    Empty,
    List(Vec<String>),
    Number(Number),
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
    /// `None` for a file outside every namespace folder, which a `ref.*` condition can reach.
    pub namespace: Option<String>,
    /// Present only when the schema has a code.
    pub key: Option<String>,
    pub code: Option<String>,
    pub collection: String,
    pub schema: String,
    /// The alias this document was reached through, when it belongs to an imported project.
    pub project: Option<String>,
    /// In the order of the file.
    pub fields: Vec<(String, Value)>,
}
