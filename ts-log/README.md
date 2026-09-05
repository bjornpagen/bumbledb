# @bjornpagen/bumbledb-log

Durable named application commands over
[Bumbledb](https://github.com/bjornpagen/bumbledb): a thin peer of
`@bjornpagen/bumbledb` (exact peer `0.20.3`). The package adds a durable
envelope around the exact core change/read machinery — it never duplicates
the engine surface. Core types (`ChangeSet`, `QueryReader`,
`ExecutionPolicy`, `DbError`, …) are the peer's own exports.

The API is **Effect-native**: every operation constructs a lazy
[`Effect`](https://effect.website) and every native resource is scoped.
There is no Promise, synchronous, or disposal twin. The package requires
Effect `4.0.0-rc.112` exactly, as a peer dependency, and Node >= 24.

The surface is small:

1. **`LocalHistory` / `HostedHistory`** — one durable history per database.
   Local authority is one LMDB directory; hosted authority is S3-compatible
   object storage over immutable decisions, with a disposable local
   materialization directory beside it. `open` of a missing or unreadable
   configured database never creates a replacement; `create` is the explicit
   constructor: it refuses existing authority and validates a stable
   creation identity plus a **checked initialization artifact** (the
   canonical schema snapshot the migration tooling renders) instead of
   fabricating genesis.
2. **`Command`** — `Command.seal` turns `{ scope, id, changes, precondition,
   result }` into one owned sealed command with a copyable pre-dispatch
   `ref`; `Command.encode`/`Command.decode` are the one bounded versioned
   command codec. The `changes` are the core's own `ChangeSet`.
3. **`PublishedSnapshot`** — the read side. It extends the core
   `QueryReader` exactly (same `get`/`execute`/`session`, policies, errors
   and result owners) and adds durable provenance: `identity`,
   `decisionStamp`, `stateStamp`, `freshness`. `ReadOptions.consistency`
   selects `cached`, `latest`, or `at-least` a known stamp.
4. **`TenantCache`** — one bounded native registry of histories for
   multi-tenant hosts: `make`/`acquire` (a `HistoryBorrow` whose `release`
   frees only the borrow)/`inspect`/`evict`/`close`. Pressure is byte and
   count budgets; evicting a borrowed slot refuses instead of revoking.
5. **Maintenance and migrations** — explicit admin operations
   (`checkpoint`, `pinRestorePoint`/`releaseRestorePoint`,
   `rotateReceiptEpoch`, `retireReceipts`, `collectGarbage`, `backup`/
   `verifyBackup`/`restore`, `erase`) and the generated-migration workflow
   under the `./migrations` subpath (below).

## Install

```sh
pnpm add @bjornpagen/bumbledb-log @bjornpagen/bumbledb effect
```

## Quick start: one durable round trip

Create a local history with its checked initialization artifact, seal one
insert command, submit it to a decided receipt, then reopen the directory
and resolve the retained ref to the exact recorded outcome.

```ts
import * as fs from "node:fs"
import { ChangeSet, key, NativeRuntime, relation, Schema, schema, str, u64 } from "@bjornpagen/bumbledb"
import type { ExecutionPolicy, NativeRuntimeOptions } from "@bjornpagen/bumbledb"
import { Command, DatabaseId, IncarnationId, LocalHistory, OperationId, ReceiptEpoch, RequestId } from "@bjornpagen/bumbledb-log"
import type { DatabaseIdentity, LocalBinding, ReadOptions, SubmitOptions } from "@bjornpagen/bumbledb-log"
import { Effect, ManagedRuntime, Result } from "effect"

const Entry = relation("Entry", { id: u64, body: str })
const Ledger = schema("Ledger", { Entry }, [key(Entry, ["id"])])

const runtimeOptions: NativeRuntimeOptions = {
	workers: 2,
	queueCapacity: 16,
	cleanupCapacity: 16,
	ownerCapacity: 16,
	nativeHandleCapacity: 64,
	inputBytes: 8_000_000n,
	workingBytes: 8_000_000n,
	scratchBytes: 8_000_000n,
	resultBytes: 1_000_000n,
	chunkBytes: 1_000_000n,
	cleanupTimeout: "2 seconds"
}
const work: ExecutionPolicy = {
	inputBytes: 1_000_000n,
	workingBytes: 1_000_000n,
	scratchBytes: 1_000_000n,
	resultBytes: 100_000n,
	rows: 100_000n,
	workUnits: 10_000_000n,
	timeout: "10 seconds"
}
const submitOptions: SubmitOptions = { ...work, attempts: 4, backoff: { baseMillis: 5, capMillis: 100 } }
const readOptions: ReadOptions = { ...work, consistency: { kind: "cached" } }

// Genuinely fallible small parsing is Result, not Effect.
const unwrap = <A, E>(result: Result.Result<A, E>): A => Result.getOrThrow(result)

const program = Effect.gen(function* () {
	const compiled = yield* Schema.compile(Ledger, work)
	// The checked initialization artifact: the canonical schema snapshot the
	// migration generator wrote to your checked-in repository. Its native
	// fingerprint IS the identity's schemaId — creation re-judges both.
	const artifact: Uint8Array = fs.readFileSync("bumbledb/migrations/meta/0000.schema.json")
	const identity: DatabaseIdentity = {
		databaseId: unwrap(DatabaseId.fromHex("ab".repeat(16))),
		incarnationId: unwrap(IncarnationId.fromHex("cd".repeat(16))),
		schemaId: compiled.schemaId
	}
	const binding: LocalBinding = { kind: "local", directory: "/tmp/ledger", identity }

	// Scope 1: create, seal, submit; retain the ref and receipt.
	const retained = yield* Effect.scoped(
		Effect.gen(function* () {
			const history = yield* LocalHistory.create(binding, Ledger, {
				...work,
				creation: {
					operationId: unwrap(OperationId.fromHex("e1".repeat(16))),
					artifact
				}
			})
			const draft = yield* ChangeSet.builder(Ledger, work)
			yield* draft.insert(Entry, [{ id: 42n, body: "hello" }])
			const changes = yield* draft.finish()
			const command = yield* Command.seal(
				{
					scope: history.identity,
					// Generate ids ONCE for an original intent and persist them;
					// a retry resubmits the identical sealed command.
					id: {
						receiptEpoch: unwrap(ReceiptEpoch.from(1n)),
						requestId: unwrap(RequestId.fromHex("0b".repeat(16)))
					},
					changes,
					precondition: { kind: "blind" },
					result: {}
				},
				work
			)
			// submit NEVER throws or fails: certainty is data.
			const outcome = yield* history.submit(command, submitOptions)
			if (outcome.kind !== "decided") {
				return yield* Effect.die("expected a decided submit in this example")
			}
			return { ref: command.ref, receipt: outcome.receipt }
		})
	)

	// Scope 2: reopen; the retained ref resolves to the recorded receipt,
	// and the committed fact reads back through the core QueryReader.
	yield* Effect.scoped(
		Effect.gen(function* () {
			const history = yield* LocalHistory.open(binding, Ledger, work)
			const resolved = yield* history.resolve(retained.ref, work)
			if (resolved.kind === "found" && resolved.receipt.outcome.kind === "committed") {
				const snapshot = yield* history.snapshot(readOptions)
				const fact = yield* snapshot.get(Entry, { id: 42n }, work)
				yield* Effect.log(fact._tag === "Some" ? fact.value.body : "missing")
			}
		})
	)
})

const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
await runtime.runPromise(program)
await Effect.runPromise(runtime.disposeEffect)
```

## Certainty, receipts, and errors

`submit` returns `Effect<SubmitOutcome>` with `E = never` — the outcome is a
three-armed certainty sum, never an exception channel:

- `decided` — the authority recorded a decision; the arm carries the
  `TerminalReceipt` (`committed`, `no-change`, or `rejected` with canonical
  violation evidence).
- `not-submitted` — proven never registered (a typed cause rides along);
  safe to fix and submit a NEW command.
- `outcome-unknown` — dispatch crossed the authority boundary but the
  decision could not be read back. Never retried blindly: `resolve(ref)`
  later returns the recorded receipt (`found`), a proven `not-submitted`,
  or `unknown` again.

Receipts and refs are plain owned data: they outlive their scope, survive
process restarts (render/parse with `renderCommandRef`/`parseCommandRef`),
and `resolve` after reopen returns the exact recorded outcome.

Failures that ARE errors use exactly two classes, checked by `_tag`, never
message strings: the core's own `DbError`, unchanged, for core failures
crossing the log; and `ProtocolError` for the log-specific reason roster
(`protocolErrorCodes` spells it natively — contention, consistency
not-yet-available, artifact/authority/identity refusals, maintenance
backpressure, migration reasons). `LogError = DbError | ProtocolError` and
nothing else. Interruption and finalizer defects stay in `Cause`.

The same core `QueryReader` helper that lists a local `Db` snapshot lists a
published history snapshot. There is no log-specific query wrapper. The
packed consumer spells this as `readAttempts(snapshot, student, work)` on
both `Db.snapshot` and `history.snapshot`.

Retain the command or admin `operationId` **before** `submit` / `initialize` /
`backup` / `restore`. `outcome-unknown` is resolved under that identity.
A later missing receipt is not proved loss.

## Backup and restore

```ts
import { NativeRuntime } from "@bjornpagen/bumbledb"
import type { ExecutionPolicy, NativeRuntimeOptions } from "@bjornpagen/bumbledb"
import {
	backup,
	OperationId,
	restore,
	verifyBackup,
	type LocalBinding
} from "@bjornpagen/bumbledb-log"
import { Effect, Result } from "effect"

declare const runtimeOptions: NativeRuntimeOptions
declare const work: ExecutionPolicy
declare const binding: LocalBinding

const unwrap = <A, E>(result: Result.Result<A, E>): A => Result.getOrThrow(result)

const cycle = Effect.gen(function* () {
	const operationId = unwrap(OperationId.fromHex("a1".repeat(16)))
	const destination = { kind: "filesystem" as const, directory: "/tmp/ledger-backup" }
	const backed = yield* backup(binding, { ...work, operationId, destination })
	if (backed.kind !== "completed") {
		return backed
	}
	yield* verifyBackup(destination, work)
	return yield* restore(destination, binding, { ...work, operationId })
})
void NativeRuntime.layer(runtimeOptions)
void cycle
```

## Migrations

Schema evolution is generated, checked-in, inert data — never inferred at
runtime:

- `@bjornpagen/bumbledb-log/schema` — pure intent constructors
  (`migrationIntent`, `renameField`, `renameRelation`, `convert`,
  `backfill`, `seed`, `dropField`, `dropRelation`). No native work at
  import or call.
- `@bjornpagen/bumbledb-log/migrations` — `generateMigrations` diffs the
  declared schema against the recorded chain and appends one validated,
  canonically rendered plan to the repository (`manifest.json`,
  `NNNN-<label>.plan.json`, `meta/NNNN.schema.json`, `snapshots.json`,
  `index.ts` exporting `{ manifest, plans, snapshots }`, and the
  `runtime-contract.json` your deploy pins). The admin runner decodes that
  triple — snapshots are the empty-base schema plus one target per entry.
  `checkMigrations` is the same judgment without writes. Ordinary onboarding
  is generated `initialize`,
  then `LocalHistory.open` / `HostedHistory.open`. `LocalHistory.create`
  remains the explicit constructor when a checked artifact is already in
  hand — it is not the Notes/Alchemy path. The runner verbs —
  `migrationStatus`, `initialize`, `migrate`, `activateMigration`,
  `abortMigration` — execute generated plans through the one native
  executor, with `AdminOutcome` certainty (`completed` / `not-started` /
  `outcome-unknown`).
- Field arithmetic such as `Scalar.add(Scalar.field("units"), Scalar.u64(1n))`
  is valid intent metadata. Native chain compilation binds it before any
  new manifest write or source freeze, including zero input rows.
- The `bumbledb-log` bin is the same generator/checker as a CLI.

## Platform and packaging

The native engine arrives through the peer `@bjornpagen/bumbledb`
(darwin-arm64, linux-arm64, linux-x64); this package ships TypeScript only
and declares both peers exactly (`@bjornpagen/bumbledb 0.20.3`, `effect
4.0.0-rc.112`). Version lockstep across the package family is enforced in
CI.
