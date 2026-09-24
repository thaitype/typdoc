# 22: Comparing numbers no primitive holds

Type: wayfinder:grilling
Status: open
Blocked by: 10

## Not on this story's frontier

This is a question about the query path, and story 2's destination is every decision the write
path needs. It blocks neither the goal nor the contract, and it is written down here rather than
in a list of things not yet specified because it has a measurement behind it and a decision to
make, not because it has to be made now.

## Question

[Decision 10](10-a-number-past-u64-on-the-write-path.md) settled that a `number` is printed with
the digits written in the document, so that nothing between the file and the caller decides what
the value is. Printing is not comparing, and the comparison is built on the conversion that
printing no longer uses. Measured on the binary built from this branch, with two documents whose
`count` differs by one and both past the range an integer holds:

```
typdoc list --where 'count>99999999999999999998'   ->   0 documents
```

The document holding `99999999999999999999` is not returned. Both values convert to the same
double, so the comparison is between a value and itself.

The same conversion carries every ordering in the tool: `--where` comparisons on `number`,
`--sort` on a `number` field, and the `date` and `datetime` comparisons that the design says
compare as instants. A number that a double cannot hold exactly is not rare only at the top of
the range: `1e3` and `1000.0` compare equal today and should, but two values differing in their
eighteenth digit do not and should not.

Story 1's review left "f64 ordering above 2^53" on its unverified list. This is that item, with a
measurement attached and a reason to decide it.

Decide:

- **Whether v1 compares exactly.** Comparing the digits themselves means an ordering that does not
  go through a primitive: comparing two decimal strings, or a big-number type, with a dependency
  or without one.
- **What the range is, if it is not exact.** If v1 compares as it does today, say so in the
  design, with where the exactness ends, rather than letting a caller find it. A silent wrong
  answer to a query is the failure this story kept choosing against elsewhere.
- **Whether a document holding a number outside the range is reported.** Today `validate` says
  nothing about it: a project can hold a number typdoc cannot compare and be told it is entirely
  well. A finding at `warn` would tell the user which documents are affected before a query
  misleads them.
- **What `--sort` does with such values**, since a sort with an inconsistent comparison can order
  differently from run to run, which is worse than an order that is merely wrong.
- **Whether `date` and `datetime` are affected**, given that they compare as instants and are
  parsed from text by typdoc rather than by a reader.

## Answer

<filled in on resolve>
