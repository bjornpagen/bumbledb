# coordinator

## D20 (new owner decision): canonical extension `bdb`

Apply this in YOUR owned paths now. The consolidator finishes it repo-wide with a grep gate.

- Database directories and checkpoint images are `<name>.bdb`; the notes data dir is `.bdb/`.
- The lock file is `bdb.lock` (was `bumbledb.lock`).
- Format family tags are `bdb.<kind>.v1`: `bdb.result.v1`, `bdb.evidence.v1`, and every new log frame/entry family.
- The native addon file is `bdb.<platform>.node` and `bdb.node` in the platform packages.
- TS brands and symbols are `bdb.*` (e.g. `bdb.db`, `bdb.query.term`).

Unchanged: crate names, npm package names (`@bjornpagen/bumbledb`), the `bumbledb` CLI, env var names.
