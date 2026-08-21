# 50 — The replica

A replica is a local store that is a materialized view of a log prefix,
plus the loop that keeps it current. Replicas are disposable by
construction: correctness never depends on a replica surviving.

## Lifecycle

```
open(store: S, prefix, dir, theory) -> Result<Replica>
```

1. GET `manifest.json`; refuse `v ≠ 1` or fingerprint ≠ `theory`'s.
2. If `dir` already holds a store: open it (format-8 verification runs),
   confirm fingerprint, take its generation as `k`. A stale or torn local
   dir is deleted and re-pulled — local state is cache, never truth.
3. Else if the manifest has a checkpoint: download, verify blake3 =
   `checkpoint.digest`, write `data.mdb`, open, assert generation =
   `checkpoint.g`; `k = checkpoint.g`.
4. Else bootstrap: `Db::create(dir, theory)` (the empty-candidate
   admission); `k = 0`.
5. Catch up: probe `log/{k+1}` … apply … until 404 (the codec's `apply`,
   idempotent and gap-refusing).

## Refresh

`refresh()` runs the catch-up loop once and returns the new generation.
Policies (host-chosen, one mechanism):

- **On-demand**: call before a request that needs freshness.
- **Interval**: a timer (or Vercel `waitUntil`) calling `refresh()` every
  N ms.
- **Wait-for**: `wait_for(g)` — refresh until `generation() ≥ g` (the
  read-your-writes tool: a host that just committed `g` elsewhere passes
  `g` through and waits here).

Staleness bound = refresh cadence. The committing instance itself is always
read-your-writes without waiting (its local apply lands before ack).

The probe is one 404 GET when idle. On Express that is ~$0.00003/1000 and
single-digit ms; per-request probing is affordable, but interval + a
manifest `get_if_changed` fallback is the default recipe.

## Reads

The replica exposes the engine's own surfaces — it *is* a `Db`:
`replica.db()` for prepared queries, point reads, scans. No wrapper query
API; one way to read. Writers in the same process use the same `Db` via the
writer component (60); replica-only deployments never open a write path.

## Vercel Fluid recipe (case 1)

- Module-level singleton: `const replica = await openReplica(...)` at
  module scope — Fluid shares it across the instance's concurrent
  invocations; instances scale out with their own copies.
- Store lives under `/tmp` (500 MB budget, per-instance, ephemeral —
  exactly what "cache, never truth" wants). Cold start = checkpoint pull +
  tail replay; warm instances amortize it to zero.
- Freshness: `waitUntil(replica.refresh())` after responding, plus
  `wait_for` on flows that read their own cross-instance writes.
- Budget math is a deployment gate, not a hope: checkpoint size +
  working set must fit 400 MB (leave 100 MB headroom). The leaf-blob
  pattern (digests in relations, bytes in the bucket) is what keeps
  metadata stores in the tens of MB.

## Per-tenant (case 4)

```
openTenants(store, root, opts { budget_bytes, max_open }) -> TenantCache
tenants.get(tenant_id) -> Result<Replica>   // opens or returns cached
```

An LRU keyed by tenant id over the same `Replica` type: eviction closes
the store and deletes its dir (disposable law). The control-plane tenant
(`t/_shared`) is pinned, never evicted. Per-tenant budgets are enforced by
the same math as above. Cross-tenant queries are **not** a replica
feature — they are the heap arm's job (scan the tenants you need into an
`InstanceBuilder`, `admit`, query the `OwnedInstance`); the replica layer
refuses to pretend otherwise.

## Failure behavior

| Event | Behavior |
| --- | --- |
| Digest mismatch on checkpoint | delete download, retry once, then `Err` |
| `GapDetected` during replay (lifecycle deleted a needed log object) | discard local store, re-open from the newest checkpoint |
| `ReplayDiverged` (a logged batch rejected locally) | corruption-class `Err`; never retried; surfaces with generation and batch key |
| Manifest fingerprint mismatch | typed refusal at open |
| Local dir torn (open fails verification) | delete, re-pull — the disposable law |
