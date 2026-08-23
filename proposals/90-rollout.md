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
executable oracle), L9 (component independence, corollary), L10 (replay
idempotence — the recovery design's theorem). Named exactly L6–L10 in
the census. This lane's completion unblocks Lane E's optimism path and
Lane D's no-forced-cases recovery.

## Lane A — the protocol trio, Rust

**Owns:** `crates/bumbledb-log/src/{codec.rs, footprint.rs, braids.rs}` +
tests + `conformance/corpus/*`. **Spec:** [20](20-command-codec.md),
[15](15-conflict-algebra.md), [10](10-protocol.md) §Braids.
Batch encode/decode (v2 header: braid, braid_gen, prev chain hash,
writer id, clamped timestamp; per-class footprint suffixes; kind-3 and
`ChainMismatch{Prev|Slot|Timestamp}` refusals); `footprint(descriptor,
ops)` — every class, every mode, the W deltas and per-key merging,
closed-statement emptiness; the intersection function returning
subsumed/disjoint/conflict per the matrices; `braids(descriptor)`; the
golden corpora for all three; the decoder fuzz target; alloc windows
(output buffers only).

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
packaging in the 0.17.x lockstep; exported `Err*` values.

## Lane D — replica + chain sidecar (after A + B)

**Owns:** `crates/bumbledb-log/src/{replica.rs, sidecar.rs, tenants.rs,
gc.rs}`. **Spec:** [50](50-replica.md), [10](10-protocol.md).
The chain sidecar (floor cache; no intent field, no forced recoveries —
recovery is the catch-up loop plus 50's wholeness identity);
tip-vs-hole decided from the manifest checkpoint vector before probing;
the gc-safety manifest heartbeat; open/catch-up round-robin across
braids; footprint + chain recompute-and-refuse on replay;
`refresh`/`wait_for` (session vectors); tenant LRU with the pinned
`_shared`; `gc` per the retention law; checkpoint backlink walk for PITR.

## Lane E — writer + loser algebra (after A + B + D; optimism gated on Lane L)

**Owns:** `crates/bumbledb-log/src/writer.rs` + tests. **Spec:**
[60](60-writer.md), [15](15-conflict-algebra.md).
One commit discipline for both modes over the shared pending slot;
`commit` (single braid, `Err::SpanningCommit` on spanning) +
`commit_split` (the explicit verb, `BraidOutcome` vector); the publish
law (`COMMIT_NOOP` ⇒ nothing published); the loser algebra (subsumed →
report winner, engine-decided survive-or-discard; disjoint →
apply-winner + republish, **cites L7/L8 at the site the way the engine
cites DeltaRestriction**; conflict → discard + re-judge recorded ops,
never a body re-run); one-rule pending recovery; `ack` modes with
`durability` in the outcome and `max_pending`; group commit with the
drain-is-one-transaction law and one-by-one fallback;
`Batch::reserve_capacity` sugar (the idiom is 15's); id-lease draw;
checkpoint duty off the loop; bounded-retry `Err::Contention` with raw
determinants.

## Lane F — integration and conformance (last)

**Owns:** `crates/bumbledb-log/src/conformance/`, census wiring, bench
pins. **Spec:** [80](80-conformance.md), lanes 1–9 in order, then the law
gates (dyn census extension, TS temporal gate, alloc windows, census
tokens incl. L6–L10 and the one-owner law census), then the performance
pins. Full verification before the final commit: engine suites, `ts` +
`ts-log` suites, `bumbledb-log` suites, `lake build`, `spec-census.sh` —
all green.

## Order

```
Lane 0 ──┐
Lane L ──┼────────────► Lane E ──► Lane F
Lane A ──┼──► Lane D ──┘              ▲
Lane B ──┘                            │
Lane C ───────────────────────────────┘  (parity goldens + census)
```

## Acceptance checklist (receipts land here)

- [x] 0: `catalog_digest` both arms; three law pins — cb291894 (the seam, both arms), 6fd20914 (intern-mint, fresh-in-command, host-order law comments with pinned tests), 1b3802e2 (the digest rendered as the order-quotient its contract claims: row ids quotiented to fact identity, so disjoint apply orders land one digest)
- [x] L: L6–L10 named, proven, census-wired — Footprint.lean's bytes are in-tree via c8336b52 (a parallel lane's commit swept the staged entries; provenance recorded in 1586a91e, which carries the two strictness countermodels); census wiring in the final-integration commit (L7 is proven in its ACCEPTANCE form — the rejected arm is refuted by the mechanized countermodel `footprint_rejection_not_stable` and the protocol never spends it; L6/L7/L8 take the strict `KeyDisjoint` hypothesis, with the relaxed shared-W-parent license fully proven beside them as the `TestedDisjoint`/`L7_tested`/`L8_tested` family)
- [x] A: codec v2 + footprint + braids + intersection + goldens + fuzz + alloc windows — 4c2b431e, a4e852ea, 445d1f49, 699c792d, a4c6aeda, 2f806975, 0e93dfc3, c21d3a20 (the three `ChainMismatch` causes are apply-time goldens in conformance/corpus/chain, per 20 they are chain checks, not codec refusals; the two W-matrix cells 15 leaves unstated price as Conflict deliberately — re-judgment is always sound there)
- [ ] B: five verbs + FsStore + S3Store + retry law + gated smoke — five verbs, FsStore, and the retry law landed (3af08b6e, 6f21ac9a, 11faf8eb wired GET-verify into publish); `S3Store` over `object_store` NOT built: crates.io unreachable in every build session (the standing gap; the gated smoke loud-skips in f11 naming the missing credentials). FsStore publication uses link(2) not rename(2) — POSIX rename cannot arbitrate exclusivity; `get_if_changed` on a vanished poll target is `Err` (40's Poll sum has no 404 arm)
- [ ] C: TS trio parity + store + replica/writer/tenants + temporal gate + recipe + packaging — e2a2ed32, c8336b52, 2b4e53ac, 7582cbdd, 1b2e85d4; the `aws4fetch` s3/r2 store NOT built: npm unreachable (the standing gap; fsStore is the shipped store and every test lane runs on it). peerDependency ^0.17.1 awaits the engine SDK's next publish for `internalBlake3`; TS checkpoint duty is Rust-owned by design (checkpoint-SEEDED open is implemented); the W arithmetic shortcut is unwired (no cheap capacity-measure read on the SDK surface); ack=local unreachable in the type; fingerprint mirror refuses closed relations carrying string ground axioms
- [x] D: chain sidecar (recovery = catch-up + wholeness check) + tip-vs-hole rule + gc heartbeat + recompute-refuse + tenants + gc + checkpoint backlink walk — f0ccdf54, 3a84d900, 0d29f2fa, 6f37e9f7, 2ea169c1, bf63dfeb, plus 36537074's replay-reaching catalog audit (50's serve-while-replaying is represented as typed provenance, not exercised across threads; gc checkpoint age is the max braid-head timestamp — neither 10 nor 50 names the clock; an undecodable below-floor object conservatively blocks its braid's sweep; restore_by_time stops at a gc hole with `SlotMissing` rather than pronouncing BeyondRetention at mapping time; the discard-and-re-pull loop caps at 8 attempts)
- [x] E: commit/commit_split + publish law + loser algebra (subsume/republish/re-judge; L7/L8/L10 cited) + one-rule pending recovery + group commit + reserve_capacity sugar + leases + `Err::Contention` — 93625dbb, df065de0, f3a6e49f, 54b6ead0, 930da8ae, 918a4ef0, 3a6542b4, e1631ffa, 7aac50a6, 11faf8eb (the disjoint fast path, lifted only after Footprint.lean built, per the structural gate; 15's W arithmetic against winner-moved measures is unwired — the loser's intersect runs with an empty BaseMeasure map, always-sound Conflict; ack-local pipeline depth is structurally 1 under 50's one-slot sidecar; reserve_capacity's reservation group is a sibling ceiling — the engine's Capacity admits one source relation; group commit is caller-drains, linger spent holding the commit core; `Batch::reserve` refuses draws wider than one 4096 block)
- [x] F1: three-way determinism (≥100 worlds) — 55818276 (100 seeded worlds, zero trophies; the checkpoint hop also driven through the writer's own cadence duty)
- [x] F2: commutativity oracle (disjoint pairs + braid interleavings) — a68b657e landed the oracle with the byte-identity gate deliberately red; 1b3802e2 fixed the engine's digest at the named fix site and the gate is green whole (string-carrying interleavings stay scoped: intern ids are store-local per 15's recorded aliasing ruling, so the multi-braid corollary is string-free by design)
- [x] F3: conflict matrix, every cell, serial-verdict equality — 31c154e7 (39 tests named by matrix coordinate; engine truths pinned: mandatory target keys collapse multi-row C/W groups so those cells co-fire the K coordinate, and a group's bound is judged only while its parent row stands)
- [x] F4: both crash-step enums, both writer ack modes, every prefix — 065c92f0 (`WriterStep` through the in-src seam; `ReplicaStep` reified as test-side data and every prefix executed mechanically against the raw directory — a recorded deviation from the literal 'seam' wording, not from the matrix)
- [x] F5: contention lane (subsumed both arms; disjoint-never-re-judges pinned; wholeness identity asserted; ambiguity resolution; livelock → `Err::Contention`) — 11faf8eb, b75ab4cb, b26b6549 (Feral uniqueness storm at width 64 with rounds 100→16 under the wall-clock license, measurement recorded in the fixture; the association storm runs the full 64×100)
- [x] F6: PITR by vector and by time; gc retention — 36537074 (found and fixed the unimplemented replay-reaching catalog comparison; both lying-checkpoint directions pinned; writer-side parity of that audit is a recorded residue — writer.rs's own open paths do not yet run the comparison)
- [x] F7: Rust⇄TS parity goldens (codec, footprint, braids) — 481f8eb3, c21d3a20, 1b2e85d4 (the chain corpus convicted the TS verifyChain of ignoring the fetched braid; fixed and probed)
- [x] F8: engine-guarantee pins — 37061cf9 (digest equality demanded exactly where the fresh-mint sequence is arrival-independent — 20 keeps intern ids store-local, the wire carries raw values; fresh replays agree with each other byte-for-byte always)
- [x] F9: fuzz (decoder, manifest, comparator) — cfb423ba (the canonical-fixpoint gate convicted Checkpoint::parse of accepting a re-ordered braid map; fixed in src/manifest.rs)
- [x] F10: law gates + census tokens — final-integration commits: zero-dyn extends to crates/bumbledb-log/src with ZERO exemptions (the one Error::source dyn removed from store.rs — Display already carries the source text), lane (i) comment hygiene extends to ts-log/src, L6–L10 wired as Bridge ledger rows (107 asserted) citing the writer's republish/lose and the recovery sites, one-owner constants gate for the six Rust and two TS protocol constants; alloc windows are 0e93dfc3; the TS temporal gate is ts-log/test/temporal-gate.test.ts. (The protocol-token roster — refusal identities, op kinds, value arms — is censused by f7_parity's coverage census against the shared corpus rather than by spec-census.sh; the full one-owner LAW census of 80 — second full prose statements of the publish law, the drain, the lease width — is not mechanically statable beyond the constants gate and stands as the doc set's editorial law)
- [x] F11: performance pins recorded; 00's envelope superseded — 53a08fe9 (all figures recorded with attribution in tests/f11_pins.rs PIN lines: commit floor ~30 ms fsync-bound, disjoint loss +35 ms fast-path, group commit ~6.5× packing, cold open 2395 ms vs seeded 14.3 ms, contention curve 65 %-of-uniform at skew 0.999 against Aria's 39 %, crossover ~40 losses/s behind LOSS_BOUND=16; the S3/Express-vs-Delta-Lake baseline is unmeasured — no S3Store, no credentials, network blocked — and the FsStore group-commit figure stands in)
