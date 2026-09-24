# 18: Which rule reports frontmatter that has a block but cannot be parsed?

Type: wayfinder:grilling
Status: resolved
Blocked by: 15

## Question

The design has a rule for types, required fields and transitions, and none for a block that cannot be parsed at all: a block that is never closed, or YAML that does not parse. Without one, such a document has no defined result.

## Answer

Decided: a new always-on rule, `frontmatter.parse`, at level `error`, and no other rule is evaluated for that file, since there is nothing to evaluate. A block that is present but cannot be parsed is not an absent block: treating it as one would turn a damaged document into an ordinary Markdown file with no signal. The registry needs a fixture for the new rule like every other.

Rejected: folding it into `frontmatter.types`, because parsing comes before types. Rejected: treating it as no frontmatter, as above.

What the YAML reader gives, run on 2026-09-20 with `yaml_serde` 0.10.7. It reports a line and a column for errors of syntax, and the column counts Unicode scalar values (checked after Thai letters and after an emoji, and it agrees with the unit fixed in ticket 11). The line is that of the YAML text in the block, so the line of the opening `---` has to be added for a position counted from the top of the file. It reports no position for a block that holds a second YAML document. For a duplicate key in one mapping it reports the start of the mapping (line 1, column 1) and not the second key. So the finding has a position when the reader gives one and none when it does not, and the design says only that. Not decided here: whether the finding carries the reader's position as it is for a duplicate key or drops it, since the position must not read as more exact than it is (map, for story 1's contract). An empty block parses as null and is not an error.
