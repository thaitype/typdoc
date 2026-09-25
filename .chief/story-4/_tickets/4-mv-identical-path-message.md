# 4: `mv a.md a.md` gets its own message

Type: implementation
Status: resolved
Blocked by: None (can start immediately)

## What this delivers

`mv a.md a.md` (source and destination are the literal same path) still exits 7, but no longer
reuses the case-only-rename wording ("the file system does not tell the two names apart") for a
case where there's no case difference to explain.

## Scope (contract §3)

`project.rs:4077-4089`: the `same_file` check (device+inode identity) is true both for the
literal-same-path case and the case-only-rename case, but only one message exists today. Branch
on `from_path == to_path` (string equality) first, give that case its own message (exact wording
is this ticket's call), and keep today's message only when `same_file` is true but the paths
differ as strings (the genuine case-only scenario). `Error::AlreadyExists`, exit code 7, stays
the outcome either way.

## Testing

Existing `mv` test module in `project.rs` (search for the case-only-rename test) gets a sibling
case for the identical-path scenario, asserting the new message text and the unchanged exit code.
