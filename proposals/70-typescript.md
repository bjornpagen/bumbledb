# 70 — The TypeScript package

`@bjornpagen/bumbledb-log`. A thin peer of `@bjornpagen/bumbledb` (peer
dependency `^0.17.2`; this package is `0.18.0`): the codec and the braid
derivation mirrored byte-exactly, the five-verb store over
`@aws-sdk/client-s3`, replica and writer composed from the engine SDK's
existing verbs. No engine surface duplicated — the replica hands out the
SDK's own `Db`. Node **>=24** is the floor (`engines`, the `.ts` test
runner, Lambda `nodejs24.x`). Never 22.

## Surface

```ts
import {
	openReplica, openWriter, openTenants,
	braidsOf, serialAtStatementsOf, encodeBatch, decodeBatch,   // the mirrored pure pair (+ the braid map's typed sibling)
} from "@bjornpagen/bumbledb-log"

// ── Replica ───────────────────────────────────────────────────────────────
const replica = await openReplica({ store: s3(env), prefix: "prod/main", dir: "/tmp/store", theory: Ledger })
replica.db                                  // the SDK's Db<Rels>; reads are engine verbs
replica.vector                              // ReadonlyMap<Braid, bigint>
await replica.refresh()                     // one stepper, all braids, one slot each → vector
await replica.refresh(braid)                // the same stepper, one braid, one slot
await replica.waitFor(vector)               // the same stepper with a predicate:
                                            // read-your-writes across a split, monotone
                                            // reads across instances, one call; a
                                            // singleton map IS the single-braid form
                                            // (one verb, one question — no overload)
await replica[Symbol.asyncDispose]()

// ── Writer (serverless mode) ──────────────────────────────────────────────
const writer = openWriter(replica)
const out = await writer.commit((batch) => {
	batch.insert(Booking, rows)               // Iterable<Fact> or ColumnBatch
	batch.delete(Hold, stale)
	const ids = batch.reserve(Booking, "id", 5n)   // draws on the id lease; never logged
	return ids
})
// out: { tag: "accepted", value, braid, generation, durability }
//    | { tag: "rejected", violations: readonly Violation<Rels>[] }
// durability: "published" | "local-pending" — the ack mode is part of the
// value, not a constructor secret. A spanning batch is a typed refusal on
// commit; writer.commitSplit(body) is the explicit verb, returning the
// per-braid outcome vector — splitness is chosen at the call site, never
// inferred. Contention is absorbed by the one loss path; at the bound,
// ErrContention's hot-key arm carries the statement and the offending
// facts' raw values from the terminal re-judgment's own violation —
// engine-produced, an operational signal, not an outcome arm.

// ── Tenants ───────────────────────────────────────────────────────────────
const tenants = openTenants({ store, root, dir, theory, budgetBytes: 400_000_000, maxOpen: 32 })
const t = await tenants.get(tenantId)       // a Replica; "_shared" pinned
// budgetBytes is 50's 400 MB gate, advisory and measured once at each
// tenant's open — a replica that grows after admission is not re-weighed
// until it is evicted and re-opened; dir and theory ride to each openReplica.

// ── Introspection (pure; no I/O) ──────────────────────────────────────────
braidsOf(Ledger)                            // ReadonlyMap<RelationName, Braid> — the
                                            // schema's own shard map, as data
```

One discriminant (`tag`); the SDK's `Violation<Rels>` in the rejected
arm. `commitSplit`'s parts are semantically independent by L9 — the docs
say so where the type is declared, because every reader will ask — and
partial completion is visible by design: the host asked for it by
choosing the verb.

## Temporal law

Async ⟺ network: `open*`, `refresh`, `waitFor`, `commit`, disposal.
Everything on `replica.db` keeps the engine's sync data-plane law;
`batch.*` recorders and `braidsOf`/`serialAtStatementsOf` are pure and
sync. The gate test asserts every exported async function awaits a
store verb on some path.

## Mirrored pure functions (the parity-critical set)

The mirrored pair must be byte-equal with Rust, pinned by the goldens
(80): `encodeBatch`/`decodeBatch` (20) and `braidsOf(descriptor)` (10),
with `serialAtStatementsOf` riding the same derivation. Both are pure,
take the descriptor the SDK already lowers, and touch no I/O — they are
the protocol; the rest of the package is plumbing around them.

## The Vercel recipe (documented example, not framework code)

```ts
// lib/db.ts — module scope; Fluid shares this across the instance's requests
export const replica = await openReplica({ store: s3(env), dir: "/tmp/store", theory: Ledger })
export const writer = openWriter(replica)

// route handler
const out = await writer.commit(b => b.insert(Booking, [row]))
if (out.tag === "accepted") ctx.waitUntil(replica.refresh(out.braid))
```

The recipe documents the `/tmp` budget gate (≤ 400 MB), `waitFor` for
cross-instance read-your-writes, and the `ErrContention` runbook (the
hot-key arm names the statement and carries the offending facts' raw
values from the re-judgment's own violation; the remedies are a
reservation relation on the hot capacity, or resident mode). No Next.js
wrapper is shipped; a wrapper would be a second way to write three
lines.

## The Lambda recipe (documented example, not framework code)

Non-normative. The working program is [`examples/lambda/`](../examples/lambda/)
— an afternoon of reading, not a second source of protocol law, and not
on the conformance path. One `nodejs24.x` arm64 function, one S3 bucket,
one EventBridge schedule, one function URL, one duty Layer. Infra is
Alchemy v2 Effect (`alchemy@2.0.0-beta.74` + `effect@rc`). The duty
binary requires `--theory PATH` (the crate corpus `{relations,
statements}` object whose fingerprint matches the handler theory). It
ships as a LayerVersion (`layer/duty/bin/bumbledb-log-duty` extracts to
`/opt/bin/bumbledb-log-duty`) because Rolldown has no `extraFiles`. IAM
cannot be attached exactly as the deploy sketch wanted: Alchemy's
Function always mints its own role and has no `roleArn`; yielding S3
bindings always adds `s3:ListBucket` on the whole bucket. The example
writes the intended prefix-only role as a document and records the
three owner choices (accept the leak, wait for `roleArn`, or inject a
user key). Until one of those three, a deploy with the derived role and
no extra policy is `AccessDenied` on every store verb.

## The local-fleet recipe (deployment case 5; documented example)

```ts
// one process per scope loop; all processes share one FsStore prefix
const replica = await openReplica({
	store: fsStore("/data/primer/log"),      // the five verbs over a directory
	prefix: "world/v1",
	dir: `/data/primer/replicas/${scopeName}`, // per-process LMDB — never shared
	theory: Explanation,
})
const writer = openWriter(replica)

// one pass = refresh, render, emit, lower, one commit
await replica.refresh()
const out = await writer.commit((batch) => {
	batch.insert(Explanation, growth.explanations)   // ids from batch.reserve
	batch.insert(Case, growth.cases)
	// …the admitted document's growth, one batch, one slot
	return growth.summary
})
// rejected ⇒ the host re-renders against the moved world and re-lowers —
// a K-conflict double-mint resolves to the winner's row on the next pass.
```

The recipe records what makes the case easy: an insert-only theory has
no delete races to lose; content-keyed determinants keep concurrent
scope loops off each other's obligations, so a lost slot re-judges to
the same accepted verdict at the moved base; a one-braid theory
serializes slot claims on a link publication, which at
document-per-minutes commit rates is free. Each process owns
its LMDB directory outright — the one-env-per-path law is satisfied by
construction, not by handle registries.

## Dependency ruling

`@aws-sdk/client-s3` (AWS-owned) is the TS S3 signer; R2/OCI ride the
same client (R2 needs `region: "auto"` and an endpoint). Credentials
are static keys or a caller-owned refresh — not the SDK default
provider chain. `memStore` is the third store: an in-process map,
single-process honesty stated where declared, blake3 etags, the
destination of every five-verb test that never touches a disk. The `fs`
store on Node `fs` is **tier-1, not a dev double** — it
is deployment case 5's production backend (00) and speaks the one
on-disk protocol of 40 verbatim: `wx`-opened synced temp published with
`fs.link`, computed blake3 etags, the fenced CAS lease
`{holder, token, expires}` — one protocol, two conforming
implementations, raced against each other
in the interop conformance lane. There is no pid-lockfile and no
`kill(0)`. Document and sidecar `u64` fields parse to `bigint`, never
`number`; pending bytes are the codec's lowercase hex. It runs every lane the S3 store runs
(80); a lane that passes on one and not the other is a reported gap.
Blake3 rides
the engine package's existing native binding: the SDK's
`internalBlake3` export (the napi module already links blake3), whose
named consumers are the store's etags and the descriptor fingerprint —
no JS blake3 dependency exists.

## Error identity

Exported values on the SDK idiom: `ErrRefused` (version, fingerprint,
manifest shape, checkpoint braid-set drift — typed per cause),
`ErrSpanningCommit` (naming the braids; the `commit`-vs-`commitSplit`
boundary), `ErrGapDetected`, `ErrReplayDiverged`,
`ErrChainMismatch` (cause: `"prev" | "slot" | "timestamp"` — one
identity, three proved causes, mirroring 20), `ErrContention` (cause sum
mirroring 60: `{ kind: "hot-key", statement, determinants }` sourced
from the terminal re-judgment's violation, `{ kind: "slot-race", tip }`
when accepted-but-outraced losses exhausted the bound), `ErrStore` (the
vendor channel, present in every wrapped store failure's cause chain so
`errors.is` matches by identity). There is deliberately no `ErrAlreadyApplied` — the state it
would name is absorbed by idempotent replay (20) and never surfaces. No
message-string matching anywhere.
