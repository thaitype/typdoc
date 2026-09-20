# 3: Golden files: the generator, its guard, and ordered comparison

Type: implementation
Status: open
Blocked by: 1

## What this delivers

- Golden files compared as parsed JSON, with arrays compared in the order the design declares and no sorting before comparing.
- Hand-written assertion files that the generator cannot write; a generator that writes only inside a `golden/` directory and only the one golden it is told to regenerate, with no regenerate-all.
- The golden of `get`.

## Done when

- A test makes the generator try to write an assertion file and sees it refuse.
- A golden changed on purpose turns the comparison red; an array in the wrong order turns it red.
- The read commands stamp no time, so no clock is involved.
