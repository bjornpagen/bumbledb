# @bjornpagen/bumbledb-log

Durable named application commands over
[Bumbledb](https://github.com/bjornpagen/bumbledb): a thin peer of
`@bjornpagen/bumbledb` (exact peer `1.3.1`). The package adds a durable
envelope around the exact core change/read machinery — it never duplicates
the engine surface. Core types (`ChangeSet`, `QueryReader`, `DbError`,
`StorageInspection`) are the peer's own exports.

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
   creation identity plus a native-verified canonical schema snapshot.
2. **`Command`** — `Command.seal` turns `{ scope, id, changes, precondition,
   result }` into one owned sealed command with a copyable pre-dispatch
   `ref`; `Command.encode`/`Command.decode` are the one bounded versioned
   command codec. The `changes` are the core's own `ChangeSet`.
3. **`PublishedSnapshot`** — the read side. It extends the core
   `QueryReader` exactly (same `get`/`execute`/`prepare`, parameters, errors
   and result owners) and adds durable provenance: `identity`,
   `decisionStamp`, `stateStamp`, `freshness`. `ReadOptions.consistency`
   selects `cached`, `latest`, or `at-least` a known stamp.
4. **`TenantCache`** — one bounded native registry of histories for
   multi-tenant hosts: `make`/`acquire` (a `HistoryBorrow` whose `release`
   frees only the borrow)/`inspect`/`evict`/`close`. `maxOpen` bounds the
   number of open tenants; evicting a borrowed slot refuses.
5. **Maintenance and transitions** — explicit checkpoint, receipt, garbage
   collection, backup/restore and erase operations; native `Transition` capture,
   population, inspection, activation and abort. Schema tooling emits bindings;
   applications write their own transformations.

## Install

This guide follows the 1.3.1 source API. Use the guide from the Git tag matching
your installed package. Install the matched package family after publication.

```sh
pnpm add @bjornpagen/bumbledb-log@1.3.1 @bjornpagen/bumbledb@1.3.1 effect@4.0.0-rc.112
```

## Quick start: one durable round trip

Create a local history with its checked initialization artifact, seal one
insert command, submit it to a decided receipt, then reopen the directory
and resolve the retained ref to the exact recorded outcome.

```ts
import { ChangeSet, key, NativeRuntime, relation, Schema, schema, str, u64 } from "@bjornpagen/bumbledb"
import { Command, DatabaseId, IncarnationId, LocalHistory, OperationId, ReceiptEpoch, RequestId } from "@bjornpagen/bumbledb-log"
import type { DatabaseIdentity, LocalBinding, ReadOptions, SubmitOptions } from "@bjornpagen/bumbledb-log"
import { schemaSnapshot } from "@bjornpagen/bumbledb-log/schema"
import { Effect, ManagedRuntime, Result } from "effect"

const Entry = relation("Entry", { id: u64, body: str })
const EntryById = key(Entry, ["id"])
const Ledger = schema("Ledger", { Entry }, [EntryById])

const submitOptions: SubmitOptions = { attempts: 4, backoff: { baseMillis: 5, capMillis: 100 } }
const readOptions: ReadOptions = { consistency: { kind: "cached" } }

// Genuinely fallible small parsing is Result, not Effect.
const unwrap = <A, E>(result: Result.Result<A, E>): A => Result.getOrThrow(result)

const program = Effect.gen(function* () {
	const compiled = yield* Schema.compile(Ledger)
	const artifact = new TextEncoder().encode(yield* schemaSnapshot(Ledger))
	const identity: DatabaseIdentity = {
		databaseId: unwrap(DatabaseId.parse("abababab-abab-abab-abab-abababababab")),
		incarnationId: unwrap(IncarnationId.parse("cdcdcdcd-cdcd-cdcd-cdcd-cdcdcdcdcdcd")),
		schemaId: compiled.schemaId
	}
	const binding: LocalBinding = { kind: "local", directory: "/tmp/ledger", identity }

	// Scope 1: create, seal, submit; retain the ref and receipt.
	const retained = yield* Effect.scoped(
		Effect.gen(function* () {
			const history = yield* LocalHistory.create(binding, Ledger, { creation: {
					operationId: unwrap(OperationId.parse("e1e1e1e1-e1e1-e1e1-e1e1-e1e1e1e1e1e1")),
					artifact
				}
			})
			const draft = yield* ChangeSet.builder(Ledger)
			yield* draft.insert(Entry, [{ id: 42n, body: "hello" }])
			const changes = yield* draft.finish()
			const command = yield* Command.seal(
				{
					scope: history.identity,
					// Generate ids ONCE for an original intent and persist them;
					// a retry resubmits the identical sealed command.
					id: {
						receiptEpoch: unwrap(ReceiptEpoch.from(1n)),
						requestId: unwrap(RequestId.parse("0b0b0b0b-0b0b-0b0b-0b0b-0b0b0b0b0b0b"))
					},
					changes,
					precondition: { kind: "blind" },
					result: {}
				}
			)
			// Submission certainty is data; interruption remains interruption.
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
			const history = yield* LocalHistory.open(binding, Ledger)
			const resolved = yield* history.resolve(retained.ref)
			if (resolved.kind === "found" && resolved.receipt.outcome.kind === "committed") {
				const snapshot = yield* history.snapshot(readOptions)
				const fact = yield* snapshot.get(EntryById, { id: 42n })
				yield* Effect.log(fact._tag === "Some" ? fact.value.body : "missing")
			}
		})
	)
})

const runtime = ManagedRuntime.make(NativeRuntime.layer())
await runtime.runPromise(program)
await Effect.runPromise(runtime.disposeEffect)
```

## Certainty, receipts, and errors

`submit` returns `Effect<SubmitOutcome>` with `E = never` — the outcome is a
three-armed certainty sum, never an exception channel:

- `decided` — the authority recorded a decision; the arm carries the
  `TerminalReceipt` (`committed`, `no-change`, `precondition-failed`, or
  `invariant-rejected` with canonical violation evidence).
- `not-submitted` — proven never registered (a typed cause rides along);
  safe to fix and submit a NEW command.
- `outcome-unknown` — dispatch crossed the authority boundary but the
  decision could not be read back. Never retried blindly: `resolve(ref)`
  later returns `found`, `not-recorded-at`, `command-epoch-closed`, or
  `receipt-expired-unknown`. Absence at one decision is not proof that an
  in-flight command will never publish.

Fiber interruption remains interruption in Effect's `Cause`; it joins native
cleanup, not an invented successful outcome. Retain the command ref or admin
operation identity before dispatch and resolve it after cancellation. A failed
cleanup adds a `CloseFailure` defect without erasing the interruption.

Receipts and refs are plain owned data: they outlive their scope, survive
process restarts (render/parse with `renderCommandRef`/`parseCommandRef`),
and `resolve` after reopen returns the exact recorded outcome.

Failures that ARE errors use exactly two classes, checked by `_tag`, never
message strings: the core's own `DbError`, unchanged, for core failures
crossing the log; and `ProtocolError` for the log-specific reason roster
(`protocolErrorCodes` spells it natively — contention, consistency
not-yet-available, artifact/authority/identity refusals, maintenance
backpressure, transition identity). `LogError = DbError | ProtocolError` and
nothing else. Interruption and finalizer defects stay in `Cause`.

The same core `QueryReader` helper that lists a local `Db` snapshot lists a
published history snapshot. There is no log-specific query wrapper. The
packed consumer spells this as `readAttempts(snapshot, student)` on
both `Db.snapshot` and `history.snapshot`.

Retain the command or admin `operationId` **before** `submit` / `Transition.begin` /
`backup` / `restore`. `outcome-unknown` is resolved under that identity.
A later missing receipt is not proved loss.

## Backup and restore

```ts
import { NativeRuntime, key, relation, schema, str, u64 } from "@bjornpagen/bumbledb"
import {
	backup,
	IncarnationId,
	OperationId,
	restore,
	verifyBackup,
	type LocalBinding
} from "@bjornpagen/bumbledb-log"
import { Effect, Result } from "effect"

declare const binding: LocalBinding
// Ledger is the same declared schema used to create this history.
const Entry = relation("Entry", { id: u64, body: str })
const Ledger = schema("Ledger", { Entry }, [key(Entry, ["id"])])

const unwrap = <A, E>(result: Result.Result<A, E>): A => Result.getOrThrow(result)

const cycle = Effect.gen(function* () {
	const operationId = unwrap(OperationId.parse("a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1"))
	const destination = { kind: "filesystem" as const, directory: "/tmp/ledger-backup" }
	const backed = yield* backup(binding, { operationId, destination, schema: Ledger })
	if (backed.kind !== "completed") {
		return backed
	}
	yield* verifyBackup(destination, { backup: operationId })
	const target: LocalBinding = {
		...binding,
		directory: "/tmp/restored-ledger",
		identity: {
			...binding.identity,
			incarnationId: unwrap(IncarnationId.parse("b2b2b2b2-b2b2-b2b2-b2b2-b2b2b2b2b2b2"))
		}
	}
	return yield* restore(destination, target, {
		operationId: unwrap(OperationId.parse("c3c3c3c3-c3c3-c3c3-c3c3-c3c3c3c3c3c3")),
		backup: operationId,
		schema: Ledger
	})
})
void NativeRuntime.layer()
void cycle
```

Local and hosted histories produce the same independently verified backup
format. A local capture streams one coherent native snapshot; it never copies
live database files. The completion manifest is published last. Retry with the
same backup operation ID to resolve the original capture, including after the
source advances. A corrupt completed artifact refuses instead of being replaced.

Persist both operation IDs and the fresh target incarnation ID before dispatch;
the fixed IDs above are examples. Restore uses a separate operation ID, the
backup's operation ID, and a new incarnation with the same database/schema
identity. Adopt the completed restore's returned binding before opening it.
Providing `schema` also supports cold administrative opens when the source is
not already open in the runtime. Backup contains database facts and receipts;
external documents referenced by those facts need their own retention.

A completed writable restore retains its canonical genesis beside the native
activation. Repeating the same operation, target, and backup returns that
original result even if later commands changed the restored database. A different
operation or backup refuses without overwriting the target.

## Generated bindings, handwritten migrations

Log provides two mechanical schema operations:

```sh
pnpm exec bumbledb-log snapshot --schema src/schema.ts --export App --out schema.json
pnpm exec bumbledb-log bindings --snapshot source.json --out source.ts
pnpm exec bumbledb-log bindings --snapshot target.json --out target.ts
pnpm exec tsc --noEmit
```

`schemaSnapshot(schema)` emits native-verified canonical JSON. `schemaBindings(snapshot)`
emits ordinary SDK declarations for that one snapshot. Historical bindings do
not load old application modules. The generated field widths, closed handles,
keys and expanded laws compile to the same native schema identity. Unsupported
representations refuse before an output file is replaced.

Write the transformation yourself in TypeScript. This example uses the generated
[submission/proof fixtures](https://github.com/bjornpagen/bumbledb/tree/v1.3.1/ts-log/test/fixtures/transition): the old schema has one
proof relation, while the new schema has one relation for each submission method.
Both use ordinary `Fact` values, queries and `ChangeSet` batches.

```ts
import { ChangeSet, query, v } from "@bjornpagen/bumbledb"
import { LocalHistory, Transition } from "@bjornpagen/bumbledb-log"
import type { History, Population, PublishedSnapshot, TransitionContract } from "@bjornpagen/bumbledb-log"
import { Effect, Stream } from "effect"
import * as old from "./source.ts"
import * as next from "./target.ts"

const joined = query(old.schema).rule((r) => {
    const { id, method, recordedAt } = v(old.r1)
    const { reference } = v(old.r2)
    return r.match(old.r1, { id, method, recordedAt })
        .match(old.r2, { submission: id, reference })
        .find({ id, method, recordedAt, reference })
})
const documents = query(old.schema).rule((r) => {
    const row = v(old.r3)
    return r.match(old.r3, row).find(row)
})

function transform(source: PublishedSnapshot<typeof old.schema>, target: Population<typeof next.schema>) {
    return Effect.gen(function* () {
        const rows = yield* source.execute(joined, {})
        yield* Stream.runForEach(rows.pages(), (page) => Effect.scoped(Effect.gen(function* () {
            const batch = yield* ChangeSet.builder(next.schema)
            yield* batch.insert(next.r1, page.map(({ id, method, recordedAt }) => ({ id, method, recordedAt })))
            for (const [method, relation] of [["Imported", next.r3], ["Electronic", next.r4], ["Postal", next.r5]] as const) {
                yield* batch.insert(relation, page.filter(row => row.method === method)
                    .map(row => ({ submission: row.id, reference: row.reference })))
            }
            yield* target.apply(yield* batch.finish())
        })))
        const unchanged = yield* source.execute(documents, {})
        yield* Stream.runForEach(unchanged.pages(), (page) => Effect.scoped(Effect.gen(function* () {
            const batch = yield* ChangeSet.builder(next.schema)
            yield* batch.insert(next.r2, page)
            yield* target.apply(yield* batch.finish())
        })))
    })
}

// Application policy: compare mapped facts, unchanged documents in both
// directions, and relevant reports. Counts alone do not prove preservation.
declare function compare(
    source: PublishedSnapshot<typeof old.schema>,
    target: PublishedSnapshot<typeof next.schema>
): Effect.Effect<boolean, Error, import("effect").Scope.Scope>

function migrate(history: History<typeof old.schema>, contract: TransitionContract) {
    return Effect.scoped(Effect.gen(function* () {
        const start = yield* Transition.begin(history, next.schema, contract)
        if (start.kind === "activated") return start.binding
        if (start.kind === "aborted") return yield* Effect.fail(new Error("Operation was aborted"))
        const ready = start.kind === "ready" ? start : yield* Effect.gen(function* () {
            yield* transform(start.source, start.population)
            return yield* start.population.finish()
        })
        const preserved = yield* Effect.scoped(Effect.gen(function* () {
            const target = yield* LocalHistory.open(ready.binding, next.schema)
            const inspected = yield* Transition.inspect(target, ready.installed)
            return yield* compare(start.source, inspected)
        }))
        if (!preserved) {
            yield* Transition.abort(history, next.schema, contract)
            return yield* Effect.fail(new Error("Application comparison failed"))
        }
        return (yield* Transition.activate(history, next.schema, ready.installed)).binding
    }))
}
void migrate
```

Retain `contract` before dispatch: `operation`, full `source` and `target`
identities, and a 64-digit hexadecimal `commitment` identifying the application's
chosen transformation. Compile the target bindings to obtain its schema ID;
choose a fresh target incarnation. Native admission recompiles the supplied
schema, freezes the exact source decision/state, and accepts only ordinary
changes into disk-backed unpublished storage. A commitment records intent; it
does not attest which JavaScript ran.

Population applies batches sequentially, including deletions. Identical facts
deduplicate. `ChangeSet.compose` uses add-wins normalization within one command;
it is not sequential replay. Parent and child facts can arrive in either order.
`finish` drains admitted calls, consumes every population alias, performs complete
law judgment and a canonical hash, then installs one frozen target. Final
judgment checks declared database laws; it cannot detect omission of unconstrained
application data. Close the inspection scope before activation and adopt the
returned binding explicitly. The source stays frozen after activation.

On interruption or uncertain installation, call `Transition.resolve` with the
retained contract first. Reuse a ready target; never rerun the transformation on
top of it. An abandoned unpublished attempt restarts from empty staging after
its owner drains. Completed retries return the original activation even after
new target commands. `Transition.abort` is terminal: it fences the target before
thawing the matching source, and refuses if activation won. Scope cleanup only
releases resources; it does not abort a durable operation.

Pages bound JavaScript delivery, but queries still seal their complete native
result before paging. Use existing indexed partitions when results exceed native
limits, and bound batches by rows and bytes. There is no per-batch whole-target
judgment or hash, row journal, or exactly-once guarantee for external effects.
The [public integration tests](https://github.com/bjornpagen/bumbledb/blob/v1.3.1/ts-log/test/transition.test.ts) exercise generated type
inference, both batch orders, preservation failures, final-law refusals and retry.

**1.3.1 deliberately removes the previous migration API and artifact formats.**
There is no transformation DSL, plan generator, repository manager, generated
migration chain, replay adapter or compatibility export. Old migration records
refuse explicitly. Ordinary histories without those retired artifacts retain
their command, receipt and schema identities. 1.3.0 cannot read the new transition
records.

The TypeScript transition API supports local histories. The native implementation retains the
existing hosted backend using conditional authority and verified checkpoints.
Hosted operation claims are permanent authority records; backups preserve
installed evidence and facts, while writable restore creates a new incarnation.
Real-S3/IAM deployment and Graviton performance remain unqualified.

## Platform and packaging

The native engine arrives through the peer `@bjornpagen/bumbledb`
(darwin-arm64, linux-arm64, linux-x64); this package ships TypeScript only
and declares both peers exactly (`@bjornpagen/bumbledb 1.3.1`, `effect
4.0.0-rc.112`). Version lockstep across the package family is enforced in
CI.
