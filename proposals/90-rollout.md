# 90 — Rollout: the build

Self-contained dispatch plan. The numbered docs (`00`–`80`) are the
living law — including the cutover spellings (v:3 refuse v:2,
`Settled | Pending`, `generation(chain)`, the fenced CAS lease,
write-once checkpoints with `prev` inside the hash, the upward
`[0, marker)` sweep, publish-clock retention, one stepper). Where this
file's lane specs and a numbered doc disagree, the numbered doc wins
and this one gets fixed. The receipts below are historical: they record
the one-path deletion campaign and are not a second spelling of the
protocol. The pass's own dispatch directory deleted itself as its
final act and its record is git history.

## Ground rules (binding on every lane)

1. The docs are the spec; a gap is a report, not an improvisation.
2. House laws: representation over control flow; one way per question;
   zero `dyn` in our own Rust; sums for outcomes, `Err` for
   infrastructure; parse-all-first at every boundary; attribution-first
   for perf claims; unbounded repair with a legible scream.
3. Workspace: `crates/bumbledb-log` follows the `bumbledb-c` precedent —
   outside the heed+blake3-pure engine workspace, path dep on the engine,
   own lockfile. The engine gains only Lane 0's seam.
4. Lanes own disjoint files; one commit per deliverable, house-style
   message; tests land with the deliverable.
5. **The Lean gate is structural**: replay-forward recovery must not
   merge before L10 exists and builds, and the braid map's cross-component
   claims cite L9 (Lane L). The crash matrices and parity goldens gate
   final integration.

## Lane 0 — the engine seam (first; tiny)

**Owns:** `crates/bumbledb/src/` (digest + three law pins).
**Spec:** [30](30-engine-seams.md). `catalog_digest` on both arms; the
intern-mint, fresh-in-command, and host-order law comments with pinned
tests. Engine suites stay green; nothing else changes.

## Lane L — the Lean theorems (parallel from minute one)

**Owns:** `lean/Bumbledb/Txn/Braids.lean`, conformance case additions.
**Spec:** the braid-independence and replay-idempotence obligations of
[10](10-protocol.md) §Braids and [50](50-replica.md).
L9 (component independence: every relation of a braid's component holds
the same rows over a foreign history's moved base as over the raw one,
and every statement anchored in the braid judges both final states
alike), L10 (replay idempotence — the recovery design's theorem). Named
exactly L9–L10 in the census; the Bridge ledger asserts 104 rows. This
lane's completion unblocks Lane D's no-forced-cases recovery.

## Lane A — the protocol pair, Rust

**Owns:** `crates/bumbledb-log/src/{codec.rs, braids.rs}` + tests +
`conformance/corpus/*`. **Spec:** [20](20-command-codec.md),
[10](10-protocol.md) §Braids.
Batch encode/decode (v3: header — braid, braid_gen, prev chain hash,
writer id, clamped timestamp — plus ops, nothing carried behind them;
kind-3, `Version` on a well-formed v:2, and
`ChainMismatch{Prev|Slot|Timestamp}` refusals);
`braids(descriptor)` with serial-at-statements as typed data beside the
map; the golden corpora; the decoder fuzz target; alloc windows (output
buffers only).

## Lane B — the store capability

**Owns:** `crates/bumbledb-log/src/store.rs`, `store/{fs,s3,mem}.rs`, tests.
**Spec:** [40](40-object-store.md). Five verbs as sums including
`Ambiguous`; `FsStore` on the
one on-disk protocol both languages speak (O_EXCL temp + link(2)
create-only, computed blake3 etags never stored, fenced CAS lease
`{holder, token, expires}` acquired and broken only through the store
CAS, expiry-only breaking, `Liveness = Alive | Dead | Unknown`);
`S3Store` over
`object_store`; `MemStore` as the third store (in-process, disk-free
tests); the ambiguous-outcome GET-verify law; credential-gated
smoke.

## Lane C — the TypeScript package (parallel; doc-driven)

**Owns:** `ts-log/` entirely. **Spec:** [70](70-typescript.md) + the trio
docs. The mirrored pure pair (codec, braid derivation) validated against
Lane A's corpus files byte-exactly; official `@aws-sdk/client-s3` store
(s3/r2) beside the conforming `fsStore` and `memStore`; replica/writer/tenants
with the exact unions of 70; the temporal gate; the Vercel recipe; the
Lambda recipe pointer; packaging on peer `^0.17.2` / package `0.18.0`;
exported `Err*` values.

## Lane D — replica + chain sidecar (after A + B)

**Owns:** `crates/bumbledb-log/src/{replica.rs, sidecar.rs, tenants.rs,
gc.rs}`. **Spec:** [50](50-replica.md), [10](10-protocol.md).
The chain sidecar (`Settled | Pending`; floor cache; no intent field, no
forced recoveries — recovery is the catch-up loop plus
`generation(chain)`; parse held to the canonical fixpoint like its
sibling documents; `v:3`, refuse `v:2`);
tip-vs-hole decided from the manifest checkpoint vector before probing;
the gc-safety manifest heartbeat; one stepper shared by
`refresh`/`wait_for`/catch-up/open (one slot per braid per round);
chain recompute-and-refuse on replay; tenant LRU with the pinned
`_shared`; `gc` per the retention law (upward `[0, marker)` sweep,
publish-clock aging); checkpoint Merkle backlink walk for PITR.

## Lane E — writer (after A + B + D; recovery gated on Lane L)

**Owns:** `crates/bumbledb-log/src/writer.rs` + tests. **Spec:**
[60](60-writer.md).
One commit discipline for both modes over the `Settled | Pending` chain;
`commit` (single braid, `Err::SpanningCommit` on spanning) +
`commit_split` (the explicit verb, `BraidOutcome` vector); the publish
law (`COMMIT_NOOP` ⇒ nothing published); the one loss path (byte-equal
absorption of an ambiguous PUT, else discard the directory, re-open to
tip, re-judge the recorded ops in one db.write — never a body re-run —
**citing L10 at the recovery sites the way the engine cites
DeltaRestriction**); one-rule pending recovery; `AckMode` as the whole
ack representation, `durability` in the outcome; group commit with the
drain-is-one-transaction law and one-by-one fallback;
`Batch::reserve_capacity` sugar (the idiom is 60's); id-lease draw;
checkpoint duty entirely off the commit lock against a `Settled`
chain, the document written once (`put_create`), `prev` inside the
full-bytes hash; `LOSS_BOUND` iterations then `Err::Contention` with
causes sourced from the terminal re-judgment's own violation.

## Lane F — integration and conformance (last)

**Owns:** `crates/bumbledb-log/src/conformance/`, census wiring, bench
pins. **Spec:** [80](80-conformance.md), lanes 1–9 in order plus the two
born lanes (cross-language interop, multi-process TS), then the law
gates (dyn census extension, TS temporal gate, alloc windows, census
tokens incl. L9–L10 and the one-owner constants), then the performance
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

- [x] 0: `catalog_digest` both arms; three law pins — cb291894 (the seam, both arms), 6fd20914 (intern-mint, fresh-in-command, host-order law comments with pinned tests), 1b3802e2 (the digest rendered as the order-quotient its contract claims: row ids quotiented to fact identity, so disjoint apply orders land one digest), c59f3f21 (the dual-claim corruption made the loud MembershipDesync its missing-claim sibling already was)
- [x] L: L9–L10 named, proven, census-wired — the transaction model's bytes landed via c8336b52 (provenance recorded in 1586a91e) and the one-path pass trimmed it to the two theorems the protocol spends: c6a359dc and f2a65db1 (the conflict-algebra tier — its model, its interval arithmetic, its stability engine, its three theorems with their relaxed family, and both strictness countermodels, 52 named declarations in all — died whole with its subject when the wire stopped carrying the structure its hypotheses read; L9 restated directly over the statement graph and the judgment through three small congruences, a strict strengthening that quantifies the braid's own batch freely; L10 untouched, its qualified name following the file to `Bumbledb.Txn.Braids`; the Bridge ledger asserts 104 rows and the census re-derives the count by grep). lean.sh green whole at f2a65db1: zero sorry, zero axiom, 104 rows / 332 tokens, corpus 277 cases at 0 disagreements, three-way comparator green
- [x] A: codec v2 (header + ops) + braids + goldens + fuzz + alloc windows — 4c2b431e, a4e852ea, 445d1f49, 699c792d, a4c6aeda, 2f806975, 0e93dfc3, c21d3a20 landed the lane; the one-path pass deleted the wire's carried conflict section with its derivation and routing (16b3426f) and regenerated the corpus bless-style as header + ops (e92f889e) — the version stays 2 because no release ever shipped the sectioned layout. The three `ChainMismatch` causes are apply-time goldens in conformance/corpus/chain, per 20 they are chain checks, not codec refusals
- [x] B: five verbs + FsStore + S3Store + retry law + gated smoke — five verbs, FsStore, and the retry law landed (3af08b6e, 6f21ac9a, 11faf8eb wired GET-verify into publish); the one-path pass unified FsStore onto the one on-disk protocol both languages speak (38c0e68d: link-published create-only, computed blake3 etags, pid-lockfile CAS; the flock, the crate's one unsafe block, the libc dependency, and the etag sidecar all dead) and raced it cross-language in the interop lane (a065797c). The standing gap is closed: Rust `S3Store` over `object_store` landed `f6c338e0`; Refresh kept `bc7ef05b` with the three-line census pin; multi-thread runtime `44e69915`; `MemStore` migrated the disk-free semantic tests `6de97425`; both-language gated smokes `ff097be2` (loud-skip without credentials). FsStore publication uses link(2) not rename(2) — POSIX rename cannot arbitrate exclusivity; `get_if_changed` on a vanished poll target is `Err` (40's Poll sum has no 404 arm)
- [x] C: TS pair parity + store + replica/writer/tenants + temporal gate + recipe + packaging — e2a2ed32, c8336b52, 2b4e53ac, 7582cbdd, 1b2e85d4 landed the lane; the one-path pass collapsed the TS loss path to one (7a978453, with the violation-sourced contention payload), fixed the ErrStore identity by representation (5e8a473f), moved fsStore onto the unified protocol (38c0e68d), proved it across real child processes (26fd9025), and re-pinned codec parity over the regenerated corpus from both sides (f467eca4). The standing gap is closed: official `@aws-sdk/client-s3` `7ada883d` (the fetch-signer is dead); `memStore` `6de97425`; Node >=24 `06f767f2`; peer `^0.17.2` / package 0.18.0 `520403db`; gated smoke `ff097be2`. The Lambda recipe pointer is `examples/lambda/`. TS checkpoint duty is Rust-owned by design (the duty binary + Layer; TS-native duty reopens only if a pure-TS deployment refuses one); ack=local unreachable in the type; the fingerprint-mirror refusal died with the descriptor collapse (`205729f5`); the TS discard-and-re-pull loop still caps at 3 attempts — the unbounded-repair law's TS twin awaits the owner's ruling on a scream channel for a browser-adjacent library
- [x] D: chain sidecar (recovery = catch-up + wholeness check) + tip-vs-hole rule + gc heartbeat + recompute-refuse + tenants + gc + checkpoint backlink walk — f0ccdf54, 3a84d900, 0d29f2fa, 6f37e9f7, 2ea169c1, bf63dfeb, plus 36537074's replay-reaching catalog audit (50's serve-while-replaying is represented as typed provenance, not exercised across threads; gc checkpoint age is the max braid-head timestamp — neither 10 nor 50 names the clock; an undecodable below-floor object conservatively blocks its braid's sweep; restore_by_time stops at a gc hole with `SlotMissing` rather than pronouncing BeyondRetention at mapping time). The discard-and-re-pull cap this receipt once recorded is dead: the one-path pass made the loop unbounded with the legible Scream and held the sidecar parse to the canonical fixpoint, order-strict like its siblings (16b3426f)
- [x] E: commit/commit_split + publish law + the one loss path + one-rule pending recovery + group commit + reserve_capacity sugar + leases + `Err::Contention` — 93625dbb, df065de0, f3a6e49f, 54b6ead0, 930da8ae, 918a4ef0, 3a6542b4, e1631ffa, 7aac50a6, 11faf8eb landed the lane; the one-path pass collapsed the routed loser arms to the one path in the single writer rewrite 40 sanctions (16b3426f): byte-equal absorption else discard/re-open/re-judge, the carried pending re-persisted crash-idempotently before any re-judgment, three of the four loss counters dead (losses ≡ re-judgments by construction), the byte-and-batch ack caps dead with `AckMode` the honest representation, the wait knob dead with its sleep-under-lock, checkpoint duty entirely off the commit lock against a proven consistent view with `prev` proven by the installing CAS (the checkpoint document at ckpt/{digest}.json re-renders in place on a Moved race — its address is the store digest), and the repair loops unbounded under the Scream. Contention causes read 10 concretely: below the bound a rejecting re-judgment IS the serial Rejected; at the bound the terminal re-judgment sources the cause — HotKey (statement + the offending facts' raw values, no Option, because the violation always names one) with the pending cleared, SlotRace with the applied batch retained. Standing shapes recorded: ack-local pipeline depth is structurally 1 under 50's one-slot sidecar; reserve_capacity's reservation group is a sibling ceiling (the engine's Capacity admits one source relation); group commit is caller-drains and drains serialize on the one core mutex (one LMDB env serializes db.write regardless); `Batch::reserve` refuses draws wider than one 4096 block
- [x] F1: three-way determinism (≥100 worlds) — 55818276 (100 seeded worlds, zero trophies; the checkpoint hop also driven through the writer's own cadence duty); the hop-ran assertion closed the silent-skip window at e92f889e
- [x] F2: commutativity oracle (braid interleavings) — a68b657e landed the oracle with the byte-identity gate deliberately red; 1b3802e2 fixed the engine's digest at the named fix site and the gate is green whole; e92f889e reshaped the lane to multi-braid convergence alone — L9's executable shadow — after the one-braid pair oracle's filtering predicate died with the algebra it was built from (string-carrying interleavings stay scoped: intern ids are store-local per 20's recorded aliasing ruling, so the multi-braid corollary is string-free by design)
- [x] F3: the serial-verdict lane — 31c154e7 landed the exhaustive matrix walk that kept the deleted algebra honest (engine truths pinned there and carried forward: mandatory target keys collapse multi-row C/W groups so those cells co-fire the K coordinate, and a group's bound is judged only while its parent row stands); e92f889e rebuilt the lane as f3_serial.rs, one racing fixture per statement family asserting exactly the serial verdict — the double-booking FD rejection, the dangling reference per order, the ceiling and floor rejections, the reservation spend and reclaim races, the byte-equal absorption
- [x] F4: both crash-step enums, both writer ack modes, every prefix — 065c92f0 (`WriterStep` through the in-src seam; `ReplicaStep` reified as test-side data and every prefix executed mechanically against the raw directory — a recorded deviation from the literal 'seam' wording, not from the matrix); e92f889e kept every step, added the in-place marker proving recovery is the byte-equal absorption arm and never a discard, and added the re-persist window cell (a crash at the carried pending's second PendingWrite recovers through ordinary pending resolution, the commit landing exactly once)
- [x] F5: contention lane, one path — 11faf8eb, b75ab4cb, b26b6549 landed the lane; e92f889e rewrote it whole with every gate structural: gap-free chains with each slot created once, every prev verified, every acked commit exactly once, digest convergence asserted byte-for-byte on the re-opened loser, wholeness identity after every fixture, ambiguity-drop resolution, livelock → `Err::Contention` with the violation-sourced HotKey payload beside the SlotRace arm, the stale-pending writer resolving through re-open with ONE race at tip where the deleted history-counting arm spent forty. Feral uniqueness storm at width 64 with rounds 100→16 under the wall-clock license, measurement recorded in the fixture; the association storm runs the full 64×100 and now proves its own non-vacuity (at least one target delete must win)
- [x] F6: PITR by vector and by time; gc retention — 36537074 (found and fixed the unimplemented replay-reaching catalog comparison; both lying-checkpoint directions pinned; writer-side parity of that audit is a recorded residue — writer.rs's own open paths do not yet run the comparison)
- [x] F7: Rust⇄TS parity goldens (codec, braids) — 481f8eb3, c21d3a20, 1b2e85d4 (the chain corpus convicted the TS verifyChain of ignoring the fetched braid; fixed and probed); re-pinned over the regenerated header+ops corpus from both sides at f467eca4, plus the born interop lane a065797c racing the one store protocol cross-language: byte-for-byte both directions, mixed-fleet create-only exclusivity, CAS linearized to the exact sum, etags agreeing everywhere
- [x] F8: engine-guarantee pins — 37061cf9 (digest equality demanded exactly where the fresh-mint sequence is arrival-independent — 20 keeps intern ids store-local, the wire carries raw values; fresh replays agree with each other byte-for-byte always)
- [x] F9: fuzz (decoder, manifest, comparator) — cfb423ba (the canonical-fixpoint gate convicted Checkpoint::parse of accepting a re-ordered braid map; fixed in src/manifest.rs); e92f889e extended the same mutation gate to the chain sidecar parser — every canonical document parses to a value that re-renders byte-identically, and accepted mutants refuse — while the storms for the dead carried section died with their subject
- [x] F10: law gates + census tokens — final-integration commits: zero-dyn extends to crates/bumbledb-log/src with the Error::source precedent plus the three-line Refresh pin in store/s3.rs (caller-owned credential behavior at a foreign async-trait boundary; cold path) — zero other log-driver dyns, lane (i) comment hygiene extends to ts-log/src, the surviving theorems wired as Bridge ledger rows (104 asserted since f2a65db1) citing the braid derivation, the one loss path, and the replay-forward recovery sites, and the one-owner constants gate standing at eight Rust and four TS protocol constants — the dead ack caps and wait knob left the roster with their knobs, the two wait_for poll cadences entered it (08109428, a1ad65cc), and the lock-retry cadence entered at the close-out (d54309e0); alloc windows are 0e93dfc3; the TS temporal gate is ts-log/test/temporal-gate.test.ts. (The protocol-token roster — refusal identities, op kinds, value arms — is censused by f7_parity's coverage census against the shared corpus rather than by spec-census.sh; the full one-owner LAW census of 80 — second full prose statements of the publish law, the drain, the lease width — is not mechanically statable beyond the constants gate and stands as the doc set's editorial law)
- [x] F11: performance pins recorded; 00's envelope superseded — 53a08fe9 (all figures recorded with attribution in tests/f11_pins.rs PIN lines: commit floor ~30 ms fsync-bound, group commit ~6.5× packing, cold open 2395 ms vs seeded 14.3 ms, contention curve 65 %-of-uniform at skew 0.999 against Aria's 39 %, crossover ~40 losses/s behind LOSS_BOUND=16); the lane's own 67.2-vs-64.3 ms end-to-end measurement is the figure that licensed the one-path ruling — the routed arm measured slower than the general one — and e92f889e re-measured the pins for the one path, the loss cost one re-open-plus-re-judgment number owned by the fsync floor. The S3/Express-vs-Delta-Lake baseline is still unmeasured — `S3Store` landed (`f6c338e0`); no live bucket on the close-out machine — and the FsStore group-commit figure stands in
