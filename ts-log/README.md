# @bjornpagen/bumbledb-log

Braided object-store replication for [bumbledb](https://github.com/bjornpagen/bumbledb):
a thin peer of `@bjornpagen/bumbledb` (peer dependency, 0.17.x lockstep).
The package is three things:

1. **The pure protocol trio**, mirrored byte-exactly against the Rust
   driver and pinned by cross-language goldens: `encodeBatch`/`decodeBatch`
   (the BDBL v2 command codec), `footprintOf(descriptor, ops)` (the
   conflict algebra's raw-value footprints), and `braidsOf(descriptor)`
   (the schema's own shard map, as data — with
   `serialAtStatementsOf` naming the degenerate-serial statements beside it).
2. **The five-verb object store** — `get`, `getIfChanged`, `putCreate`,
   `putSwap`, `delete` — with `fsStore` as the tier-1 local-directory
   implementation (deployment case 5's production backend, not a dev
   double). The S3/R2/OCI store rides `aws4fetch` and is not yet in this
   build (the dependency was unfetchable offline).
3. **Replica and writer** composed from the engine SDK's existing verbs:
   `openReplica` hands out the SDK's own `Db`; `openWriter` adds the
   right to create log objects; `openTenants` is an LRU of per-tenant
   replicas. No engine surface is duplicated.

Async ⟺ network: `openReplica`, `refresh`, `waitFor`, `commit`,
`commitSplit`, and disposal await store verbs; everything on
`replica.db`, the `batch.*` recorders, and the pure trio are synchronous.

## The Vercel recipe (documented example, not framework code)

```ts
// lib/db.ts — module scope; Fluid shares this across the instance's requests
import { fsStore, openReplica, openWriter } from "@bjornpagen/bumbledb-log"

export const replica = await openReplica({ store: s3(env), prefix: "prod/main", dir: "/tmp/store", theory: Ledger })
export const writer = openWriter(replica)

// route handler
const out = await writer.commit((b) => b.insert(Booking, [row]))
if (out.tag === "accepted") ctx.waitUntil(replica.refresh(out.braid))
```

- **The `/tmp` budget gate**: checkpoint plus working set must stay
  ≤ 400 MB (100 MB headroom under the 500 MB instance limit); the
  leaf-blob pattern keeps metadata stores in the tens of MB. Per-tenant
  fleets get the same gate through `openTenants({ budgetBytes, maxOpen })`.
- **Cross-instance read-your-writes**: a commit returns
  `(braid, generation)`; a session token is the pointwise max of every
  pair a flow has seen; `replica.waitFor(vector)` refreshes until the
  local vector dominates it. The committing instance always reads its
  own writes without waiting. A singleton map is the single-braid form.
- **The `ErrContention` runbook**: the error carries its cause —
  `{ kind: "hot-key", statement, determinants }` names the hot
  determinant's raw values; the remedies are a reservation relation on
  the hot capacity (an ordinary weighted child row, 15's schema idiom)
  or resident mode. `{ kind: "slot-race", tip }` means fully-disjoint
  writers out-raced the bound: a hot braid wanting group commit.

## The local-fleet recipe (deployment case 5)

```ts
// one process per scope loop; all processes share one FsStore prefix
import { fsStore, openReplica, openWriter } from "@bjornpagen/bumbledb-log"

const replica = await openReplica({
	store: fsStore("/data/primer/log"), // the five verbs over a directory
	prefix: "world/v1",
	dir: `/data/primer/replicas/${scopeName}`, // per-process local dir — never shared
	theory: Explanation
})
const writer = openWriter(replica)

// one pass = refresh, render, emit, lower, one commit
await replica.refresh()
const out = await writer.commit((batch) => {
	batch.insert(Explanation, growth.explanations) // ids from batch.reserve
	batch.insert(Case, growth.cases)
	return growth.summary
})
// rejected ⇒ the host re-renders against the moved world and re-lowers —
// a K-conflict double-mint resolves to the winner's row on the next pass.
```

What makes the case easy: an insert-only theory never reaches a delete
cell of the conflict matrices; content-keyed determinants make
concurrent scope loops footprint-disjoint in the common case (republish,
not re-judge); a one-braid theory serializes slot claims on a create,
which at document-per-minutes commit rates is free. Each process owns
its local replica directory outright.

**`fsStore`'s discipline, stated**: every verb on a key serializes under
an O_EXCL lockfile in `<root>/.locks` whose body is the owner's pid; a
lock whose pid is dead is broken and retaken. One machine is
load-bearing, not descriptive — pid liveness and O_EXCL are the
arbitration primitives, and network filesystems weaken both; an
`fsStore` prefix on a network mount is a misdeployment. `putCreate` and
`putSwap` resolve only after fsync of the object file and its parent
directory.

## Error identity

Exported sentinel values on the SDK idiom, checked with `errors.is`,
never by message strings: `ErrRefused` (typed per cause — batch shape,
version, fingerprint, manifest shape, checkpoint braid-set drift),
`ErrSpanningCommit`, `ErrGapDetected`, `ErrReplayDiverged`,
`ErrFootprintMismatch`, `ErrChainMismatch` (cause `"prev" | "slot" |
"timestamp"`), `ErrContention` (cause `hot-key` or `slot-race`),
`ErrStore` (the vendor channel). There is deliberately no
`ErrAlreadyApplied`: the state it would name is absorbed by idempotent
replay and never surfaces.
