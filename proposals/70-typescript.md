# 70 — The TypeScript package

`@bjornpagen/bumbledb-log`. A thin peer of `@bjornpagen/bumbledb` (peer
dependency, same 0.15.x lockstep): the codec mirrored byte-exactly, the
store capability over `fetch` + `aws4fetch`, and the replica/writer
components composed from the engine SDK's existing verbs. No engine
surface is duplicated — the replica hands out the SDK's own `Db`.

## Surface

```ts
import { openReplica, openWriter, openTenants } from "@bjornpagen/bumbledb-log"

// ── Replica (case 1 read path) ────────────────────────────────────────────
const replica = await openReplica({
	store: s3({ bucket, prefix, region, credentials }),   // or r2(...), fs(...)
	dir: "/tmp/app-store",
	theory: Ledger,
})
replica.db                          // the SDK's Db<Rels> — reads are the engine's own verbs
replica.generation                  // bigint
await replica.refresh()             // one catch-up pass → new generation
await replica.waitFor(g)            // read-your-writes across instances
await replica[Symbol.asyncDispose]()

// ── Writer (serverless mode on the same replica) ──────────────────────────
const writer = openWriter(replica)
const outcome = await writer.commit((batch) => {
	batch.insert(Booking, rows)                 // Iterable<Fact> or ColumnBatch
	batch.delete(Hold, stale)
	const ids = batch.reserve(Booking, "id", 5n)
	return ids
})
// outcome: { tag: "accepted", value, generation }
//        | { tag: "rejected", violations: readonly Violation<Rels>[] }
//        | { tag: "contended", winner: bigint }    // state advanced; loop to retry

// ── Tenants (case 4) ──────────────────────────────────────────────────────
const tenants = openTenants({ store, root, budgetBytes: 400 * MiB, maxOpen: 32 })
const t = await tenants.get(tenantId)           // a Replica; "_shared" is pinned
```

One discriminant (`tag`), the SDK's own `Violation<Rels>` in the rejected
arm, `contended` as the `moved`-style expected answer — the union narrows
with the exact skill hosts already have from `WriteOutcome`.

## Temporal law

The engine SDK's law was "async ⟺ AsyncTask". This package's law is the
same idea at its layer: **async ⟺ network** — `refresh`, `waitFor`,
`commit`, `open*`, and disposal (which may flush) are async because they
genuinely await I/O; everything on `replica.db` keeps the engine's
data-plane sync law (local microsecond reads). `batch.*` recorders are
sync (they build bytes). No async-in-name-only methods; the census-style
gate for this is a test asserting every exported async function awaits a
store operation on some path.

## The Vercel recipe (shipped as a documented example, not framework code)

```ts
// lib/db.ts — module scope: Fluid shares this across the instance's requests
export const replica = await openReplica({ store: s3(env), dir: "/tmp/store", theory: Ledger })
export const writer = openWriter(replica)

// in a route handler
const out = await writer.commit(b => b.insert(Booking, [row]))
ctx.waitUntil(replica.refresh())    // background freshness, off the response path
```

The recipe documents the budget gate (checkpoint + working set ≤ 400 MB of
`/tmp`), the retry-on-`contended` loop, and `waitFor` for cross-instance
read-your-writes. No Next.js-specific wrapper is shipped — a wrapper would
be a second way to do what three lines already do.

## Codec parity

The TS codec is generated-shape-checked against the Rust one through the
existing `tags.json`-style golden discipline: a checked-in corpus of
batches (every op kind, every value tag, boundary values, a multi-op
group-commit batch) with byte-exact expected encodings; both
implementations must produce and parse them identically (80 wires the
cross-lane). Value tags, header layout, and refusal cases come verbatim
from 20 — the doc is the codec; both implementations are projections.

## Dependency ruling (TS)

`aws4fetch` only (SigV4 over platform `fetch`; ~4 KB), per 40. R2 and OCI
ride the same signer. No AWS SDK. The `fs` store is Node `fs` under the
same five-verb interface for local dev and tests.
