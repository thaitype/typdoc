# Exit codes

typdoc 0.2.0. Every command ends with one of these. A new code exists only where the caller has to
act differently, so the code alone says what kind of next step makes sense.

## Where the output goes

- **Exit 0**: the result is on stdout.
- **Any other exit**: the error is on stderr. Without `--json` it is one line starting `typdoc: `.
  With `--json` it is one object:

  ```json
  {"error":"no document at WF-9","code":5,"details":[]}
  ```

  `details` holds findings (same shape as `validate`'s) when there is something to locate, and an
  ambiguous name adds `candidates`.
- **`validate` is different**: its report is the result, so it is on stdout whether the verdict
  is good or bad, and exit 2 says some findings are errors.
- A run interrupted by SIGINT/SIGTERM releases its locks and ends by the signal — no code from
  this table and no error object.

## The codes

### 0 — success

Includes a query that matches nothing (`{"documents":[],"total":0,"truncated":false}`), and a `mv`
that landed on a schema the document does not satisfy — read `mv`'s `findings` for that, not
the code.

### 1 — the call is wrong

Retrying the same call gives the same answer. Fix the call.

| Seen as | Cause | Next |
| --- | --- | --- |
| `expected an operator …: did you mean status=open?` | Spaces in an expression, or broken query syntax | Write the expression as one word; see [query.md](query.md) |
| `` `WF-1` is a key in more than one namespace: story-1:WF-1, story-2:WF-1 `` | A bare key with several namespaces in scope; `--json` adds `"candidates":["story-1:WF-1","story-2:WF-1"]` | Pick one: prefix it (`story-2:WF-1`) or use the path |
| `the scope holds more than one namespace: story-1, story-2` | A write with no namespace chosen in a multi-namespace project | Add `--namespace story-1`, or run from inside that namespace's folder |
| `` `WF` is a schema's code, and `new` needs a title `` | `typdoc new WF` without a title | `typdoc new WF "Title"` |
| `` `other/x.md` matches no collection `` | `new <path>` where no uncoded collection's `match` fits | Use a path the collection allows; read its `match` in `.typdoc/collections/` |
| — | `mv` of a coded document to a new path in its own namespace, or `--renumber` into its own namespace or another project | A coded document keeps its key; `--renumber <other-namespace>` is the way to move it |

### 2 — validation failed

From `validate`: at least one finding is at level `error` (warnings alone give 0; `--strict`
turns them into errors). From a write (`new`, `set`): the value would break the schema, and
nothing was written — e.g. `the field status is not one of the schema's values: bogus`. Also
when the config itself cannot be loaded (`config.*` ids).

Next: read the `rule` of each finding and fix the document or the call — see
[validation.md](validation.md).

### 3 — `set --if` was false

```console
$ typdoc set WF-1 status=claimed --if status=open
typdoc: `status=open` is false
```

Nothing was written. The check and the write happen under one lock, so this is the reliable
signal that someone else changed the document first. Next: re-read (`typdoc get WF-1 --json`)
and decide again.

### 4 — lock not acquired

A write waits for the namespace's lock up to `--lock-timeout` seconds (default 5), then exits 4.
The message gives the lock file's path, pid, host and age, and what typdoc can tell about the
owner:

- **still running on this machine** — another write is in progress: wait and retry, or raise
  `--lock-timeout` (a `mv` in a large repository holds the lock longer);
- **no longer running on this machine** — the lock is stale and the message names the file to
  delete;
- **on another host** — typdoc cannot check; delete it only when that process is known to have
  stopped.

typdoc never deletes or takes over a lock itself. Remove a stale lock only when you are sure
nobody else is about to do the same, since two parties deleting "the stale lock" can delete a
fresh one.

### 5 — not found

The document, key, path or file the command was asked to act on does not exist — including
`no project found: there is no .typdoc/ in … or above it`. When a project-relative
path names nothing but a file of that name exists relative to the current directory, the message
says `./name` exists (a suggestion only).

Next: check the name with `typdoc list --ids`, or create it with `typdoc new`.

### 6 — cannot read or write

A file or directory could not be read or written (permissions, disk, a vanished folder). A
problem of the environment; retrying may work once it is fixed.

### 7 — the destination already exists

```console
$ typdoc new notes/getting-started.md --set title=x
typdoc: `notes/getting-started.md` already exists: choose another path or open the file that is there
```

From `new <path>`, `mv`, and `mv --renumber`. Nothing was written and, for `new`, no number was
used up. `mv` also exits 7 when the two names are the same file. Next: choose another name, or
open the file that is there.
