# 16: What is a path given as an argument counted from, and how is a key told from a path?

Type: wayfinder:grilling
Status: resolved
Blocked by: 15

## Question

The design took a path given as an argument to be a path on disk: the project is found by walking up from it, and it wins over `TYPDOC_DIR`. The `path` in `--json` is counted from the project folder, so a program that passes on a `path` it was given, from a directory above the project as `TYPDOC_DIR` is meant for, has the argument read against the wrong directory: at best not found, at worst a different file with the same relative name. The design also said an argument may be a key or a path and never said how the two are told apart.

## Answer

Decided: an argument that names a document is a path or a key, told apart by its form and never guessed. After any `project::` prefix, one that ends in `.md` is a path and one that has the form of a key is a key; a key never ends in `.md` and a document is always a `.md` file, so they cannot be confused; anything else is exit 1. A path that begins with `/`, `./` or `../` is on disk, and is the only kind that can name the project and that takes precedence over `TYPDOC_DIR`. Any other path is relative to the project folder, read after the project is found. The path of a document of an imported project is `project::path`. `mv` reads both arguments this way. The design's paragraph on Discovery is replaced, not amended, and a paragraph and a table under it give the string that names a document for every combination (this project or an import, one namespace or several, path or key).

Reasons. The property wanted is that a name a command prints can be passed to the next without conversion, and where it cannot, that is a fault of the shape and not a burden on the caller. The rule is about the string and not about the disk, so the same string means the same on every machine. The precedence of a path over `TYPDOC_DIR` could work only while a path was read against the current directory, because the file has to be found before its project is known; a path relative to the project folder cannot say which project it is in, so the precedence now applies to the kinds of path that can be found, and this is a change to a rule the design had already written, not a gap that was filled. The path form is the one to pass on, since every document has a path and it needs to know nothing about how many namespaces a project has; the key form is in the table for completeness.

Rejected: trying both places and answering with the one that exists, with an ambiguity error when both do, because the meaning of one string would then depend on what is on the disk, against the design's own statement that typdoc never guesses. Rejected: keeping paths relative to the current directory and adding an absolute path to the JSON, which is a second word for one thing.

Cost, stated: a person in a subfolder who types a path relative to it must write `./`. The error says that `./name` exists when it does, as a suggestion and not as a substitution. Not verified: whether the design meant a bare path to be read against the current directory; no statement to that effect was found.

A test built in story 1 walks every document of every fixture project, builds each string from the identity a command printed, passes it to `get` and checks that it returns the same file (ticket 9).
