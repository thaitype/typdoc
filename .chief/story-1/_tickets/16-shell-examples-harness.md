# 16: The harness for the shell examples

Type: implementation
Status: resolved
Blocked by: 14

## What this delivers

- Examples taken from the design, each with a character outside the safe set given a value declared by hand; sh and bash; a stand-in `typdoc` that records its arguments; the unquoted variant run in a directory that holds files a star can match; every code span of the Quoting paragraph classified; a listed shell that is missing makes the suite red.

## Done when

- Each check is shown red: an example with its quotes removed differs from the declared value, a declaration with no example is red, an example with no declaration is red.
