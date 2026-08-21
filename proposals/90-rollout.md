# 90 — Rollout: the build

Self-contained dispatch plan. The normative truth is this directory; where
this file and a numbered doc disagree, the numbered doc wins and this one
gets fixed. This file is the only proposals/ file agents edit (receipts in
the checklist).

## Ground rules (binding on every lane)

1. The docs are the spec; a gap is a report, not an improvisation.
2. House laws: representation over control flow; one way per question;
   zero `dyn` in our own Rust; sums for outcomes, `Err` for
   infrastructure; parse-all-first at every boundary; attribution-first
   for perf claims.
3. Workspace: `crates/bumbledb-log` follows the `bumbledb-c` precedent —
   outside the heed+blake3-pure engine workspace, path dep on the engine,
   own lockfile. The engine gains only Lane 0's seam.
4. Lanes own disjoint files; one commit per deliverable, house-style
   message; tests land with the deliverable.
5. **The Lean gate is structural**: the republish-without-re-judgment
   path (Lane E) must not merge before L7 exists and builds (Lane L). The
   crash matrices and parity goldens gate final integration.

## Lane 0 — the engine seam (first; tiny)

**Owns:** `crates/bumbledb/src/` (digest + three law pins).
**Spec:** [30](30-engine-seams.md). `catalog_digest` on both arms; the
intern-mint, fresh-in-command, and host-order law comments with pinned
tests. Engine suites stay green; nothing else changes.

## Lane L — the Lean theorems (parallel from minute one)

**Owns:** `lean/Bumbledb/Txn/Footprint.lean` (new), conformance case
additions. **Spec:** [15](15-conflict-algebra.md) §Lean obligations.
L6 (footprint soundness over the raw-value keys), L7 (footprint
stability — the strengthening of `DeltaRestriction`), L8 (commutativity,
set-level; representation half delegated to Lane 0's pins + Lane 2's
executable oracle), L9 (component independence, corollary). Named
exactly L6–L9 in the census. This lane's completion unblocks Lane E's
optimism path.

## Lane A — the protocol trio, Rust

**Owns:** `crates/bumbledb-log/src/{codec.rs, footprint.rs, braids.rs}` +
tests + `conformance/corpus/*`. **Spec:** [20](20-command-codec.md),
[15](15-conflict-algebra.md), [10](10-protocol.md) §Braids.
Batch encode/decode (v2 header: braid, braid_gen, timestamp, sorted
footprint section; kind-3 refusal); `footprint(descriptor, ops)` — every
class, every mode, the W deltas, closed-statement emptiness; the
intersection function returning the CONFLICT verdict per the matrices;
`braids(descriptor)`; the golden corpora for all three; the decoder fuzz
target; alloc windows (output buffers only).

## Lane B — the store capability

**Owns:** `crates/bumbledb-log/src/store.rs`, `store/{fs,s3}.rs`, tests.
**Spec:** [40](40-object-store.md). Five verbs as sums; `FsStore`
(O_EXCL create-only, flock+etag CAS) strong enough for Lanes 4–6;
`S3Store` over `object_store`; the ambiguous-outcome GET-verify law;
credential-gated smoke.

## Lane C — the TypeScript package (parallel; doc-driven)

**Owns:** `ts-log/` entirely. **Spec:** [70](70-typescript.md) + the trio
docs. The three mirrored pure functions validated against Lane A's corpus
files byte-exactly; `aws4fetch` store (s3/r2/fs); replica/writer/tenants
with the exact unions of 70; the temporal gate; the Vercel recipe;
packaging in the 0.15.x lockstep; exported `Err*` values.

## Lane D — replica + vector sidecar (after A + B)

**Owns:** `crates/bumbledb-log/src/{replica.rs, sidecar.rs, tenants.rs,
gc.rs}`. **Spec:** [50](50-replica.md), [10](10-protocol.md).
The vector sidecar with its two forced recoveries; open/catch-up
round-robin across braids; footprint recompute-and-refuse on replay;
`refresh`/`wait_for`; tenant LRU with the pinned `_shared`; `gc` per the
retention law.

## Lane E — writer + loser algebra (after A + B + D; optimism gated on Lane L)

**Owns:** `crates/bumbledb-log/src/writer.rs` + tests. **Spec:**
[60](60-writer.md), [15](15-conflict-algebra.md).
Auto-split by braid with the `Split` outcome; serverless mode with the
loser algebra (disjoint → apply-winner + republish, **cites L7/L8 at the
site the way the engine cites DeltaRestriction**; conflict →
fork-discard + re-run); resident mode with the pending slot and its two
forced recoveries; `ack` modes; group commit with one-by-one rejection
fallback; id-lease draw; checkpoint duty; bounded-retry `ErrContention`.

## Lane F — integration and conformance (last)

**Owns:** `crates/bumbledb-log/src/conformance/`, census wiring, bench
pins. **Spec:** [80](80-conformance.md), lanes 1–9 in order, then the law
gates (dyn census extension, TS temporal gate, alloc windows, census
tokens incl. L6–L9), then the performance pins. Full verification before
the final commit: engine suites, `ts` + `ts-log` suites, `bumbledb-log`
suites, `lake build`, `spec-census.sh` — all green.

## Order

```
Lane 0 ──┐
Lane L ──┼────────────► Lane E ──► Lane F
Lane A ──┼──► Lane D ──┘              ▲
Lane B ──┘                            │
Lane C ───────────────────────────────┘  (parity goldens + census)
```

## Acceptance checklist (receipts land here)

- [ ] 0: `catalog_digest` both arms; three law pins
- [ ] L: L6–L9 named, proven, census-wired
- [ ] A: codec v2 + footprint + braids + intersection + goldens + fuzz + alloc windows
- [ ] B: five verbs + FsStore + S3Store + retry law + gated smoke
- [ ] C: TS trio parity + store + replica/writer/tenants + temporal gate + recipe + packaging
- [ ] D: vector sidecar (two forced recoveries) + catch-up + recompute-refuse + tenants + gc
- [ ] E: auto-split + loser algebra (L7/L8 cited) + resident pending + group commit + leases + `ErrContention`
- [ ] F1: three-way determinism (≥100 worlds)
- [ ] F2: commutativity oracle (disjoint pairs + braid interleavings)
- [ ] F3: conflict matrix, every cell, serial-verdict equality
- [ ] F4: all three crash matrices, every prefix
- [ ] F5: contention lane (disjoint-never-re-judges pinned; ambiguity resolution; livelock → `ErrContention`)
- [ ] F6: PITR by vector and by time; gc retention
- [ ] F7: Rust⇄TS parity goldens (codec, footprint, braids)
- [ ] F8: engine-guarantee pins
- [ ] F9: fuzz (decoder, manifest, comparator)
- [ ] F10: law gates + census tokens
- [ ] F11: performance pins recorded; 00's envelope superseded
