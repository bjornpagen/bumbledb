# Public API consumers

These examples exercise Rust core, TypeScript core, TypeScript log, and a
ledger-shaped application using the current public APIs.

## Run the installed-package check

After building the TypeScript packages and this host's native addon, run
from the repository root:

```sh
scripts/packed-import.sh --host-only
```

The script stages actual tarballs, installs them into temporary projects,
typechecks downstream declarations without workspace aliases, and executes
the shared runtime journey. It also checks pure metadata authoring with the
addon absent, runs the Rust example, and generates and exercises the Notes
application in an isolated copy. The default invocation, without
`--host-only`, requires all three platform binaries.

This checks staged artifacts, not installation from the public registry.

## Examples

- `core-ts/consumer.ts`: schemas, changes, bounded reads, and reusable queries.
- `log-ts/consumer.ts`: sealed commands, retained identity, and history reads.
- `native-ledger/consumer.ts`: application composition over those surfaces.
- `rust/`: a standalone Cargo consumer. It uses a source dependency because
  the Rust workspace crates currently have `publish = false`.

The TypeScript examples export lazy programs; the application supplies one
`NativeRuntime.layer`. Database work is Effect, while schema/query/scalar
authoring remains pure. `readAttempts` accepts the shared `QueryReader`
implemented by both core and published snapshots.

The runtime driver is `scripts/packed-consumer.ts`. It owns operational
assertions such as durable retries, bounded result refusal, migration, and
backup/restore. Read that driver and its execution output for actual coverage;
the existence of an example alone is not qualification.
