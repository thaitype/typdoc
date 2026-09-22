/// A frontmatter value typed `number`. It holds the digits written in the document, because
/// nothing between the file and the caller decides what `1e3` is, and the value converted out
/// of them, because a comparison, a sort and a table cell have to have one. The two are kept
/// apart deliberately: the digits stay exact however long they are, and the converted value is
/// where a number no primitive holds loses its tail, so two documents whose digits differ by
/// one meet there and nowhere earlier.
#[derive(Debug, Clone, Eq)]
pub struct Number {
    written: String,
    converted: serde_json::Number,
}

/// Two numbers are equal when the values they convert to are, which is what `--where num=1000.0`
/// matched against a document holding `1e3` before the digits were kept and goes on matching
/// now. The digits are deliberately no part of it: telling apart two numbers that convert to
/// one value is the comparison question, not the printing one.
impl PartialEq for Number {
    fn eq(&self, other: &Number) -> bool {
        self.converted == other.converted
    }
}

impl Number {
    /// `written` as a number, or `None` when it is not one. What counts as a number is what
    /// JSON calls one, decided by the same parse as before the digits were kept, so `0755`,
    /// `+3`, `.5` and `1e400` are among the things that are not and a field holding one keeps
    /// the text it was written as.
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

    /// The value converted out of the digits, in the form JSON prints a converted value in:
    /// `1e3` is `1000.0`, and `99999999999999999999` is `1e+20`, which is what
    /// `99999999999999999998` converts to as well. A glob, an equality, a sort key and a table
    /// cell all read a number here, and this is the form each of them read before the digits
    /// were kept beside it.
    pub fn converted(&self) -> String {
        self.converted.to_string()
    }

    /// The converted value as an `f64`, for an ordering and a sort key.
    pub fn as_f64(&self) -> Option<f64> {
        self.converted.as_f64()
    }

    /// The converted value as a `u64`, when it is a whole number that one holds.
    pub fn as_u64(&self) -> Option<u64> {
        self.converted.as_u64()
    }
}

/// A frontmatter value. `Text` and `List` are a value as it is written, which is what a value
/// stays when it does not fit the type its schema gives it; the others are a value that does.
///
/// `Empty` is `Text` holding no text, told apart because of how it was written: a field with a
/// name and nothing after it (`reviewer:`, YAML's null), not a field written as the empty
/// string (`reviewer: ''`). Everywhere a value is checked, compared, sorted or matched, `Empty`
/// reads as text with nothing in it, exactly as `Text(String::new())` does — a required field
/// holding it is present, not missing, and a field of a type that is not text still does not
/// fit. The two forms differ only where the file itself differs: the block a write produces
/// puts back the one that was there, and `--json` prints `Empty` as `null` where it prints
/// `Text(String::new())` as `""` (`docs/design.md`, "Document files" and "JSON output").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Text(String),
    /// A field written with a name and no value at all (design, "Document files": "A field
    /// written with no value at all is not the same as one written as an empty string").
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
