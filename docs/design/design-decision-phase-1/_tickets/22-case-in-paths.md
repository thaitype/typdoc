# 22: How does typdoc treat case in paths and keys?

Type: wayfinder:grilling
Status: resolved
Blocked by: 15

## Question

The default file system on macOS does not distinguish upper and lower case and Linux does. Documents are identified by key or path, and refs resolve through them.

## Answer

Decided, and each part follows from what the design already says. Two keys cannot differ only in case: a code is `[A-Z][A-Z0-9]*` and a key has the form `CODE-number`, so `keys.unique` has nothing to do about case, and that is the reason it does nothing and not a choice not to. Paths are compared exactly as written, case included, on every platform, through the index of names as they are on disk, which is how the design says refs resolve, so a ref whose path differs from the file's in case does not resolve (`not-found`) even where the file system would open it. There is no rule in v1 against two paths that differ only in case: a repository that has them cannot be checked out on a case-insensitive file system at all, which is a matter for the repository and for git before typdoc runs. A `mv` that changes only the case is settled in story 2.

Not verified: any of this on macOS; it has not been run there, and macOS is not a supported platform yet (map).
