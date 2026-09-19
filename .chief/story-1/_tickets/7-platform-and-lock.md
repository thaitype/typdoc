# 7: Which platforms does v1 support, and how does the lock behave on each?

Type: wayfinder:grilling
Status: open
Blocked by: None (can start immediately)

## Question

The lock is created with `O_EXCL`, records pid, hostname and timestamp, and is taken over if its pid is dead on the same host or it is older than 30 seconds. Checking that a pid is dead is easy on Unix and different on Windows. Config also reads `~/.config/typdoc/imports.json` and expands `${ENV}` in import paths.

Decide the supported platforms for v1 (Linux and macOS only, or Windows too), what that means for pid-liveness, path separators in keys and refs, and the config location, and how an unsupported platform fails. Consider that the 30-second stale threshold can steal a lock from a live process on a slow write. Amend `docs/design.md`.

## Answer

