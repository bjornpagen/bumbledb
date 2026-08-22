# 70 — The TypeScript package

`@bjornpagen/bumbledb-log`. A thin peer of `@bjornpagen/bumbledb` (peer
dependency, 0.17.x lockstep): the codec and the footprint/braid functions
mirrored byte-exactly, the five-verb store over `fetch` + `aws4fetch`,
replica and writer composed from the engine SDK's existing verbs. No
engine surface duplicated — the replica hands out the SDK's own `Db`.

## Surface

```ts
import {
	openReplica, openWriter, openTenants,
	braidsOf, footprintOf, encodeBatch, decodeBatch,   // the pure protocol trio (+1)
} from "@bjornpagen/bumbledb-log"

// ── Replica ───────────────────────────────────────────────────────────────
const replica = await openReplica({ store: s3(env), prefix: "prod/main", dir: "/tmp/store", theory: Ledger })
replica.db                                  // the SDK's Db<Rels>; reads are engine verbs
replica.vector                              // ReadonlyMap<Braid, bigint>
await replica.refresh()                     // all braids, round-robin → vector
await replica.refresh(braid)                // one braid
await replica.waitFor(vector)               // session vector: pointwise dominance —
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
// inferred. Contention is absorbed by the loser algebra; bounded retries
// surface as ErrContention carrying the raw determinant values of the hot
// key (an operational signal, not an outcome arm).

// ── Tenants ───────────────────────────────────────────────────────────────
const tenants = openTenants({ store, root, budgetBytes: 400_000_000, maxOpen: 32 })  // 50's 400 MB gate
const t = await tenants.get(tenantId)       // a Replica; "_shared" pinned

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
`batch.*` recorders and `braidsOf`/`footprintOf` are pure and sync. The
gate test asserts every exported async function awaits a store verb on
some path.

## Mirrored pure functions (the parity-critical set)

Three functions must be byte-equal with Rust, pinned by the goldens (80):
`encodeBatch`/`decodeBatch` (20), `footprintOf(descriptor, ops)` (15), and
`braidsOf(descriptor)` (10). All three are pure, take the descriptor the
SDK already lowers, and touch no I/O — they are the protocol; the rest of
the package is plumbing around them.

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
error names the hot determinant; the remedies are a reservation relation
on the hot capacity, or resident mode). No Next.js wrapper is shipped; a
wrapper would be a second way to write three lines.

## Dependency ruling

`aws4fetch` only (~4 KB SigV4 over platform `fetch`); R2/OCI ride the same
signer; `fs` store on Node `fs` for dev and tests. No AWS SDK. Blake3 via
the engine package's existing native binding (the napi module already
links blake3 — expose a doc-hidden hash entry rather than adding a JS
blake3 dependency; that exposure rides the SDK, not the engine crate).

## Error identity

Exported values on the SDK idiom: `ErrRefused` (version, fingerprint,
manifest shape, checkpoint braid-set drift — typed per cause),
`ErrSpanningCommit` (naming the braids; the `commit`-vs-`commitSplit`
boundary), `ErrGapDetected`, `ErrReplayDiverged`, `ErrFootprintMismatch`,
`ErrChainMismatch` (cause: `"prev" | "slot" | "timestamp"` — one
identity, three proved causes, mirroring 20), `ErrContention` (cause sum
mirroring 60: `{ kind: "hot-key", statement, determinants }` when
conflicts exhausted the bound, `{ kind: "slot-race", tip }` when
fully-disjoint racers did), `ErrStore` (the vendor channel). There is deliberately no `ErrAlreadyApplied` — the state it
would name is absorbed by idempotent replay (20) and never surfaces. No
message-string matching anywhere.
