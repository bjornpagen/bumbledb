# 90 — Rollout: the overnight build

Self-contained dispatch plan for an agent fleet. The normative truth is
this directory ([README](README.md) → 00–80); where this document and a
numbered doc disagree, the numbered doc wins and this one gets fixed.

## Ground rules (binding on every lane)

1. The docs are the spec. Do not invent surface; do not omit named
   surface. A gap in the docs is a report, not an improvisation.
2. House laws: representation over control flow; one way per question;
   zero `dyn` in our own Rust; sums for outcomes, `Err` for
   infrastructure; parse-all-first at every boundary; attribution-first
   for any perf claim.
3. Workspace shape: `crates/bumbledb-log` follows the `bumbledb-c`
   precedent — outside the heed+blake3-pure engine workspace, path
   dependency on the engine crate, its own lockfile. The engine workspace
   gains nothing but Lane 0's seam.
4. Lanes own disjoint files. A cross-lane need is a report. One commit per
   deliverable, house-style message (one dense imperative sentence).
5. Every deliverable lands with its tests; the checklist at the bottom of
   this file is updated with test names as receipts (this file is the only
   proposals/ file agents edit).

## Lane 0 — the engine seam (first; tiny; blocks only gates)

**Owns:** `crates/bumbledb/src/` (the digest method + two law tests).
**Spec:** [30-engine-seams.md](30-engine-seams.md).
Deliver `Db::catalog_digest()` (+ the `OwnedInstance` twin), doc-hidden,
blake3 over the raw ordered `_data` then `_dict` enumeration; the
intern-mint-determinism law comment + pinned test; the fresh-in-command
test. Engine suites stay green; no other engine change is permitted.

## Lane A — the Rust codec

**Owns:** `crates/bumbledb-log/src/codec.rs`, `codec/tests.rs`,
`conformance/corpus/*.batch` (the checked-in golden files).
**Spec:** [20-command-codec.md](20-command-codec.md).
Deliver: batch encode/decode with full parse-all-first refusal (typed
errors naming relation/row/field), `apply(db, batch)` with
`AlreadyApplied`/`GapDetected`/`ReplayDiverged` semantics, and the golden
corpus authored per 80 Lane 5 (every op kind, every tag, boundary values,
every refusal). Allocation law: encode/decode allocate output buffers only
(one `alloc_counter` window test). A fuzz target over `decode` (the parser
is sequential and offset-free by design — fuzz it to prove it): no panic,
no overflow, every rejection typed, on arbitrary bytes and on
golden-corpus mutations.

## Lane B — the store capability

**Owns:** `crates/bumbledb-log/src/store.rs`, `store/fs.rs`,
`store/s3.rs`, their tests.
**Spec:** [40-object-store.md](40-object-store.md).
Deliver: the five-verb trait exactly as written (sums, not booleans);
`FsStore` (O_EXCL+rename create-only; flock+etag-file CAS) strong enough
to host Lanes 2–4 of the conformance suite; `S3Store` over `object_store`
with conditional put modes mapped to `Create`/`Swap`; the
ambiguous-outcome GET-verify retry law with jittered backoff; a
credential-gated S3 smoke test (skipped-with-reason otherwise).

## Lane C — the TypeScript package (fully parallel; doc-driven)

**Owns:** `ts-log/` (new package dir beside `ts/`), all its files.
**Spec:** [70-typescript.md](70-typescript.md) + 20 + 40 + 50 + 60.
Deliver: the codec mirror (validated against Lane A's corpus files —
byte-exact both directions), the `fetch`+`aws4fetch` store (s3/r2/fs),
`openReplica`/`openWriter`/`openTenants` with the exact unions of 70, the
temporal-law gate test, the Vercel recipe as a documented example, npm
packaging in the 0.15.x lockstep with a `PUBLISHING.md` row. Peer-depends
on `@bjornpagen/bumbledb`; duplicates zero engine surface.

## Lane D — replica and writer (after A + B merge)

**Owns:** `crates/bumbledb-log/src/{replica.rs, writer.rs, tenants.rs,
gc.rs}`, their tests.
**Spec:** [50-replica.md](50-replica.md), [60-writer.md](60-writer.md),
[10-protocol.md](10-protocol.md).
Deliver: replica open/refresh/wait_for with the disposable-local law and
the failure table of 50; both writer modes — serverless
(publish-before-ack, fork-discard on `Exists`, `Commit::Contended`) and
resident (one-slot sidecar, `ack = local | published`, the two forced
recovery resolutions); group commit (512 ops / 4 MiB, no linger,
one-by-one fallback on batch rejection); checkpoint publication + manifest
CAS; the FloorBump adoption batch; the `gc` verb per 10's retention law.

## Lane E — integration and conformance (after 0 + A + B + D; C joins for Lane 5)

**Owns:** `crates/bumbledb-log/src/conformance/`, census wiring, the
bench pins.
**Spec:** [80-conformance.md](80-conformance.md).
Deliver, in order: Lane 1 (three-way replay determinism over ≥100
generated worlds — reuse `corpus_gen` via the bench crate's generator or a
vendored slice of it), Lane 2 (both crash matrices as reified step enums,
every prefix), Lane 3 (contention + ambiguity injection), Lane 4
(PITR + gc), Lane 5 (cross-goldens with Lane C), Lane 6 (engine-guarantee
pins), then the law gates (dyn census extension, TS temporal gate, alloc
window, `spec-census.sh` tokens) and the performance pin lane. Full
verification before the final commit: engine workspace suites, `ts` and
`ts-log` suites, `bumbledb-log` suites, census — all green.

## Order

```
Lane 0 ──┐
Lane A ──┼──► Lane D ──► Lane E (integration)
Lane B ──┘                  ▲
Lane C ─────────────────────┘ (cross-goldens + census)
```

## Acceptance checklist (receipts land here)

- [ ] 0: `catalog_digest` on both arms; intern-mint + fresh-in-command pins
- [ ] A: codec + refusals + golden corpus + alloc window
- [ ] B: trait + FsStore + S3Store + retry law + gated smoke
- [ ] C: TS codec parity + store + replica/writer/tenants + temporal gate + recipe + packaging
- [ ] D: replica lifecycle + both writer modes + group commit + checkpoints + adoption + gc
- [ ] E1: three-way determinism (≥100 worlds)
- [ ] E2: both crash matrices, every prefix
- [ ] E3: contention + ambiguity resolution
- [ ] E4: PITR + gc retention
- [ ] E5: Rust⇄TS cross-goldens
- [ ] E6: engine-guarantee pins
- [ ] E7: law gates (dyn, temporal, alloc, census tokens)
- [ ] E8: performance pins recorded; 00's envelope superseded by numbers
