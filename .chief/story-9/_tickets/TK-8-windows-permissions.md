---
blocked_by:
- TK-6
- TK-12
status: resolved
title: 'Permissions on Windows: read-only flag only, or ACLs too'
type: wayfinder:grilling
---

# TK-8: 'Permissions on Windows: read-only flag only, or ACLs too'

## Question

When typdoc replaces a document on Windows, what counts as the mode it keeps: only the read-only attribute, or the ACL as well? What a user would see when a replaced file loses an ACL it had, and whether that must be said in the documentation. Owner's decision, through the director.

## Answer

Decided by the owner: the DACL is carried. Before the POSIX-semantics rename (TK-12), the replaced file's DACL
is copied to the temp file, and its read-only flag with it. If the DACL cannot be read from the old file or set
on the temp file, the write stops with a clear error and nothing is replaced: typdoc never writes a document
without the permissions it had. The copy is measured before it is built.
