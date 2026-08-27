# 20 — Mirrored-pair inventory: bumbledb-log (Rust) ↔ ts-log (TS)

Scope: every hand-mirrored implementation pair between
`crates/bumbledb-log/src/` and `ts-log/src/`, graded for purity
(shareable behind one Rust core + FFI) and drift risk (how hard the
conformance/parity lane pins the two copies together).

Method: full read of all 16 TS source files (6,482 lines) against the
20 Rust source files (12,914 lines), plus the v3 conformance corpus
(`crates/bumbledb-log/conformance/v3/`), both parity suites
(`crates/bumbledb-log/tests/f7_parity.rs`, `ts-log/test/parity.test.ts`,
`conformance_v3.rs` / `conformance-v3.test.ts`) and the live interop
lane (`crates/bumbledb-log/tests/lane_b_interop.rs` ↔
`ts-log/test/interop-child.ts`).

## Headline totals

- TS driver source: **6,482 lines** across 16 files.
- Rust driver source: **12,914 lines** across 20 files (incl. in-file tests).
- Rust lines with a TS mirror (whole or partial): **~10,685**.
- Rust-only, no TS counterpart: **2,229 lines** — `gc.rs` (588, retention + PITR),
  `checkpointer.rs` (345), `inspect.rs` (308), `bin/duty.rs` (973), `lib.rs` (15).
- TS-only, no bumbledb-log counterpart: `descriptor.ts` `fromSealed`
  (~190 lines, `ts-log/src/descriptor.ts:196-385`) — parses the engine's
  *sealed* descriptor, which Rust gets natively as
  `bumbledb::schema::SchemaDescriptor`; and `errors.ts` (280 lines) —
  the sentinel/idiom layer whose refusal *names* mirror enums scattered
  across the Rust modules.
- One pair reaches outside the two named trees: `value.ts`
  `writeCanonicalLiteral` (`ts-log/src/value.ts:278-323`) mirrors the
  **engine crate's** `encode_literal`
  (`crates/bumbledb/src/encoding/encode.rs:58`), not anything in
  bumbledb-log.

## Pair table

Grades: drift risk **A** = byte-exact corpus goldens both suites replay
(lowest risk while the lane runs); **B** = strong semantic pin
(shared live test or pinned constant) but no byte goldens; **C** =
mirrored only by *disjoint* per-language test suites; **D** = no
meaningful cross-pin at all (drift lands silently).

| # | Pair | Rust (lines) | TS (lines) | Implements | Purity | Corpus pin | Drift |
|---|------|--------------|------------|------------|--------|-----------|-------|
| P1 | Byte primitives + hex | `codec.rs:35-46` (ByteSink), `codec.rs:373-435` (Cursor), `manifest.rs:64-166` (hex32 + doc Cursor) ≈ 170 | `bytes.ts` 304 | LE integer read/write, 32-byte digests, lowercase-hex grammar, u64 saturating/checked adds | PURE | indirect — every golden crosses it | A |
| P2 | Value vocabulary + tagged codec + canonical literal | `codec.rs:48-190, 760-940` ≈ 320; engine `encoding/encode.rs:58` | `value.ts` 336 | Tag table 0–6, tagged LE encode/decode per field layout, typed cell refusals, engine's BE order-preserving literal form | PURE | tagged form: batch goldens; canonical literal: fingerprint pin only | A / B |
| P3 | Batch codec + chain discipline | `codec.rs` 1,016 total (encode 527-606, decode 607-730); `apply.rs:61-78,149-175` | `codec.ts` 335 (verifyChain 313-332) | The v:3 BDBL batch wire — full parse before apply, 20+ typed refusal identities — and the 3-cause chain verify | PURE | **byte-exact**: `batch/` 7 ok + 24 refusals, `chain/` 5, `fuzz/` 33 + storm recipe | A |
| P4 | Braid derivation + serial-at | `braids.rs` 200 | `braids.ts` 50 + `descriptor.ts:708-771` (deriveBraids) + `:142-170` (serialAtOf) ≈ 145 | Connected components of the statement graph; braid id = smallest member as `c{08x}`; empty-determinant serial statements | PURE | **byte-exact**: `braids/` 4 golden maps; every batch golden's membership refusals | A |
| P5 | Schema/descriptor assembly | `schema_file.rs` 646 | `descriptor.ts:389-706` (assembleFromSpec) ≈ 320 | Parse the corpus/duty `{relations, statements}` schema spelling into a descriptor: closed rosters, handle literals, capacity windows, synthesized key statements | PURE | corpus `schemas.json` exercised by both parity suites; shape-refusal edges unpinned | B |
| P6 | Vector algebra | `vector.rs` 282 | `vector.ts` 173 | Per-braid generation map: sum (one overflow site), dominates, checkpoint order, advance, and the `u32le count + (u32,u64)` binary coordinate | PURE | sum/order pinned via checkpoint-doc goldens; **`encode`/`parse` wire form has no golden** (Rust uses it in gc/PITR; TS side near-dead) | A core / **D wire** |
| P7 | Key grammar + key layout | `store.rs:17-160` (StoreKey, segment_ok, parse_prefix) + `manifest.rs:42-64` (manifest/log/ckpt keys) + `lease.rs:40` (ids_key) ≈ 200 | `keys.ts` 231 | Slash-path grammar, `~tmp`/`~lease` reserved tilde-family, `.lock` suffix ban, format-char stripping; `manifest`, `log/{braid}/{hex16}`, `ckpt/{hex}[.mdb]`, `ids/{rel}/{field}`, `t/{tenant}` | PURE | **no goldens** — mirrored unit tests (`store.rs:554-580` vs `keys.test.ts`) + interop objects happening to land at the same paths | **B–/C** |
| P8 | Scratch-lease body | `replica.rs:976-1045` | `keys.ts:131-162` | The 33-byte `~lease/ckpt-scratch` doc: version byte 3 + digest; names a crash-stranded checkpoint candidate | PURE | **none** — per-language unit tests only (`keys.test.ts`, `replica.rs` in-file tests) | **D** |
| P9 | Chain sidecar document | `sidecar.rs` 519 | `chain.ts` 225 | `dir/chain` binary record: version, ascending braid roster (u32,u64,32,u64), Settled/Pending arm with raw batch bytes; generation = sum(+1 iff Pending); atomic temp+rename+fsync write | PURE parse/render; thin IO shell | **byte-exact**: `documents/sidecar/` 8 + fuzz sidecar prefixes | A (bytes) / C (write choreography) |
| P10 | Manifest + checkpoint documents | `manifest.rs:168-390` ≈ 220 | `manifest.ts` 209 | v:3 manifest (fingerprint + optional ckpt digest) and digest-keyed checkpoint (roster, catalog, writer, prev); catalog audit at seed | PURE | **byte-exact**: `documents/manifest/` 5, `documents/checkpoint/` 10, fuzz doc prefixes | A |
| P11 | Checkpoint publish + store birth | `manifest.rs:391-493` (create_manifest, publish_checkpoint, Published/PublishRefusal) + `replica.rs:1008-1076` (scratch record/clear/sweep) + `checkpointer.rs` (Rust-only cadence) | `writer.ts:638-785` (putCreateOnce, deleteOrphan, claimScratch, casPublish, publishCheckpoint, birthStore) | Content-addressed ckpt pair upload, manifest CAS under the checkpoint order, loser deletes its orphan, scratch lease over the publish window | IO-BOUND (pure kernel = order comparison, P6) | none — `checkpoint-orphan.test.ts` vs `lane_e_checkpoint.rs`/`lane_d_manifest.rs`, disjoint | **C** |
| P12 | Store contract — fs | `store.rs:256-553` (sums, trait, prove_*, retry_read) + `store/fs.rs` 500 + `store/fence.rs` 396 | `store.ts:1-653` (fsStore + fs-lease algebra) | Five verbs over a directory: link(2) create-only, blake3 etag, fenced CAS mutation lease, Unknown-never-breaks, fsync-before-resolve, sweep at open | IO-BOUND | live semantic pin: `lane_b_interop.rs` ↔ `interop-child.ts` (byte-equal etags, exactly-one-Created, CAS linearization) — **and real drift already present, see notes** | **B outcomes / D lock protocol** |
| P13 | Store contract — s3 | `store/s3.rs` 800 | `store-s3.ts` 377 | Five verbs over S3/R2: If-None-Match:* / If-Match, 409→Ambiguous, GET-verify proofs, bounded jittered read retry, per-request credentials | IO-BOUND | env-gated smoke only (`s3_smoke.rs`, `s3-smoke.test.ts`) — skipped without creds | **C–** |
| P14 | Store contract — mem | `store/mem.rs` 127 | `store.ts:655-706` (memStore) | Five verbs over one in-process map, blake3 etags, fresh buffers | IO-BOUND (process-local) | none | C (low stakes) |
| P15 | Fresh-id lease algebra | `lease.rs` 540 | `writer.ts:75-280` (LEASE_WIDTH, drawIds, acquireLease, pushRange, ensureFreshLeases) ≈ 200 | `ids/{rel}/{field}` decimal counter; birth claims [0,4096); CAS block increment; draw = OverWidth \| Exhausted \| Drawn | draw algebra PURE; block CAS IO-BOUND | none — width 4096 and decimal body pinned only as twin constants; **Rust counter writes carry fencing tokens (`lease.rs:24-25`), TS has no Fenced write at all** | **C** |
| P16 | Apply + pending fold | `apply.rs` 220 | `replica.ts:326-473` (factOf, applyOps, applySlot, foldPending) ≈ 150 | Decode→chain-verify→one db.write→publish-law instrument; the 7-arm pending fold (Ours/Theirs±/Absent±/BelowFloor/Phantom) | PURE decision core over engine writes | chain causes pinned by `chain/` goldens; the fold arms have **no golden** (crash-matrix vs f4_crash are disjoint suites) | B / C |
| P17 | Replica machine | `replica.rs` 1,268 | `replica.ts` 1,107 | Open/seed/catch-up stepper: manifest adopt, checkpoint seed, sidecar resume, one-slot-per-braid pass, wholeness identity, wedge vs discard-and-re-pull, reserved-namespace sweep | IO-BOUND | none — `lane_d_replica.rs`/`f4_crash.rs`/`f1-f3` vs `replica-open/recovery/crash-matrix/temporal-gate` tests, disjoint; **constants already drifted, see notes** | **C** |
| P18 | Writer machine | `writer/` 2,734 (mod 840, open 446, pending 384, discipline 350, drain 280, batch 176, duty 157, loss 101) | `writer.ts` 870 | Commit discipline (Pending→durable→judge→publish), the one loss path, deposition, contention scream, inherited-pending settle | IO-BOUND | none cross-language; LOSS_BOUND=16 and WRITER_AT offset are twin constants (`writer/loss.rs:20` ↔ `writer.ts:82`); **Rust half is 3× larger — group commit (`drain.rs`), ack modes, checkpoint cadence (`duty.rs`), StepHook fault injection are Rust-only** | **C** |
| P19 | Tenants LRU | `tenants.rs` 330 | `tenants.ts` 436 | Per-tenant replica pool under `t/{tenant}`: refcounted pins, `_shared` pinned, byte+count budget eviction, dir lease renewal | IO-BOUND | none — `lane_d_tenants.rs` vs `tenants.test.ts` | C |
| P20 | Error/refusal identities | `codec.rs:276-370` (DecodeError::identity), `sidecar.rs:70-121`, `manifest.rs:168-273`, `apply.rs:61-124`, `writer/mod.rs:196-357` | `errors.ts` 280 | The cross-implementation refusal names (`Truncated`, `Version`, `TagMismatch`, …) plus driver-boundary sums (Ambiguous, OverWidth, Exhausted, SlotRetired, chain causes, contention causes) | PURE (names as data) | decode identities compared string-for-string by every corpus refusal; driver-boundary tail unpinned | A head / C tail |

## Per-pair notes

### P3/P1/P2 — the codec stack (codec.ts + bytes.ts + value.ts ↔ codec.rs)

The one place the split is architectural rather than incidental: Rust
keeps cursor, value tags, and batch grammar in one 1,016-line
`codec.rs`; TS factors it into `bytes.ts` (reader/writer/hex),
`value.ts` (tag table `TAG` at `value.ts:58-66` ↔ `codec.rs:22-28`;
`readTagged`/`writeTagged` ↔ `decode_value`/`append_value`), and
`codec.ts` (header + op grammar, `encodeBatch:72-157` ↔
`Codec::encode` at `codec.rs:527`, `decodeBatch:188-297` ↔
`Codec::decode` at `codec.rs:607`). Both share the same pre-loop
unbacked-count refusal (`codec.ts:178-185` ↔ `codec.rs` `bytes_back`)
and the same `MIN_OP_BYTES = 9`. Refusal identities are literally the
same strings (`errors.ts:71-95` ↔ `DecodeError::identity`,
`codec.rs:351`). This is the most heavily pinned surface in the repo:
7 ok + 24 refusal batch goldens, 33 materialised fuzz cases, and the
`storm.json` XorShift64 recipe replayed by the TS suite
(`conformance-v3.test.ts:682-691`) against `f9_fuzz.rs`.

`verifyChain` lives in `codec.ts:313-332` on the TS side but in
`apply.rs:149-175` on the Rust side — same three proved causes
(slot / prev / timestamp), same refusal data — pinned by the 5 `chain/`
goldens.

One deliberate residual gap: `value.ts:272-277` documents that the
canonical-literal mirror does not cover string cells (the engine's
`put_literal` length-prefixes them, `crates/bumbledb/src/schema/fingerprint.rs:204`);
the only cross-pin for that mirror is the single fingerprint constant in
`ts-log/test/fingerprint.test.ts:98-101`.

### P4 — braids, and the 200-vs-50 size asymmetry

`braids.rs` (200 lines; the brief said 179 — current `wc -l` says 200)
is self-contained: the `BraidId` newtype with its `c{:08x}` Display
(`braids.rs:37-41`), the array-based union-find (`braids.rs:99-114`),
the derivation walk over materialized statements including the
empty-determinant serial-at rules (`braids.rs:121-200`), and
`parse`/`components` accessors. `braids.ts` is a 50-line façade because
the actual derivation moved into the descriptor: the map-based
union-find is `descriptor.ts:708-771` (`deriveBraids`), serial-at is
`descriptor.ts:142-170` (`serialAtOf`), and both run once inside
`fromSealed`/`assembleFromSpec` and are cached on the theory
(`descriptor.ts:196-211`). `braids.ts` merely re-exports lookups
(`braidsOf`, `parse`, `serialAtStatementsOf`) over the cached
`Descriptor`. So the real mirrored surface is ~195 Rust lines ↔ ~145 TS
lines; the "50" is bookkeeping, not a 4× compression. Note the
union-finds are not textually equivalent (array + path-halving vs Map +
plain walk, `braids.rs:99-105` vs `descriptor.ts:718-727`) — only the
goldens make them agree.

### P5 — descriptor assembly (schema_file.rs ↔ descriptor.ts)

`schema_file.rs` (646) parses the duty/corpus `{relations, statements}`
JSON into the engine's `SchemaDescriptor`. Its TS twin is
`assembleFromSpec` (`descriptor.ts:589-706`) plus the closed-roster and
literal-resolution helpers (`descriptor.ts:392-587`): same synthesized
statements (fresh-field keys, closed-id keys, bidirectional containment
split), same handle→id resolution, same "dependent floors refused"
rule (`descriptor.ts:519-524`). The *other* half of `descriptor.ts`
(`fromSealed`, 264-367) has no bumbledb-log mirror at all — it re-parses
the engine SDK's sealed output, work the Rust driver never does because
it links the engine. Cross-pin: the 4 corpus schemas are assembled by
both parity suites and their fingerprints/braid maps compared; edge
refusals (duplicate names, missing cells) are unpinned.

### P6 — Vector

Method-for-method mirror (`sum`/`dominates`/`order`/`at`/`advance`/
`encode`/`parse`; `vector.rs:53-160` ↔ `vector.ts:33-169`), including
the "overflow only at sum" law and the overflow-vs-overflow `Equal`
ordering (`vector.rs:77-88` ↔ `vector.ts:80-101`). The binary
coordinate (`encode`/`parse`) is consumed in Rust by gc/PITR restore
points; in TS it is exported (`index.ts:50-51`) but nothing in
`ts-log/src` consumes it — a mirror of a wire format with **no golden
and no TS caller**, i.e. the cheapest possible silent-drift surface.

### P7/P8 — key grammar, key layout, scratch body

TS concentrates all of it in `keys.ts`; Rust splits it three ways:
grammar in `store.rs:31-160` (tilde-family table, format-char strip,
`.lock` ban — compare `keys.ts:26-47` TILDE_FAMILY / regexes), layout in
`manifest.rs:42-64`, ids key in `lease.rs:40`, scratch in
`replica.rs:976-1045`. Grammar equivalence is asserted only by two
independently written unit-test sets; the corpus never encodes a key.
The scratch-lease body (33 bytes, version 3 + digest;
`replica.rs:984-1005` ↔ `keys.ts:138-162`) is a real cross-driver byte
format — a Rust checkpointer's crash must be sweepable by a TS
successor and vice versa — with **zero** cross-language goldens. Grade D;
smallest candidate, highest pin-per-line payoff.

### P9/P10 — sidecar, manifest, checkpoint documents

Parse/render mirrored tightly (`sidecar.rs:204-360` ↔
`chain.ts:86-180`; `manifest.rs:189-390` ↔ `manifest.ts:89-190`),
including ascending-roster refusals, optional-digest presence bytes,
and overflow-at-parse. All pinned byte-exactly by
`documents/{manifest,checkpoint,sidecar}` (23 cases) plus fuzz doc
prefixes. The IO shells (atomic temp+fsync+rename write:
`sidecar.rs:310-335` ↔ `chain.ts:199-222`) are per-language and only
semantically mirrored.

### P12 — fs store: the drift is already real

The interop lane pins outcomes hard (byte-equal blake3 etags, exactly
one Created per slot, mixed-fleet CAS counter exact —
`lane_b_interop.rs:157-336`). But the mutation-lock protocol underneath
has **already diverged** despite `store.ts:6-17` claiming "the one
on-disk protocol, shared with the Rust driver":

- **Lease body format**: Rust writes 4 lines with a magic header,
  `LEASE/1\n{holder}\n{token}\n{expires}\n` (`store.rs:215-221`); TS
  writes 3 lines, `{holder}\n{token}\n{expires}\n` (`store.ts:126`),
  and `parseLease` refuses anything but exactly 3 lines
  (`store.ts:132-145`). Each side parses the other's lease as
  unreadable → `Unknown` → never breakable.
- **Lease location**: Rust mints token files under
  `{root}/~lease/{key}/` (`store/fence.rs:291-294`); TS puts dotfiles
  `.{basename}.lease.{token}` beside the object (`store.ts:32-38,511`).
  The two drivers' mutation locks are mutually invisible.
- **TTL**: Rust `MUTATION_TTL_MS = 5_000` (`store/fence.rs:15`) vs TS
  `MUTATION_LEASE_MS = 30_000` (`store.ts:111`).

Cross-driver create-only stays safe because arbitration is link(2) on
the shared object path itself; cross-driver swap linearization is
pinned only by the interop race's outcome assertion
(`lane_b_interop.rs:248-336`), not by a shared lock. This pair is the
strongest evidence in the repo that semantic-only pins drift.

### P13 — s3 store

Same sums, different vendor stacks (`object_store` crate vs
`@aws-sdk/client-s3`), same 409→Ambiguous and GET-verify laws
(`store/s3.rs` header ↔ `store-s3.ts:99-112,249-269`). Rust carries
fencing-token metadata on `Fenced` writes (`store/s3.rs` header); TS has
no fenced-write concept anywhere in its `ObjectStore`
(`store.ts:79-90`). Pin is an env-gated smoke on each side that most CI
runs skip — effectively C–/D in practice.

### P15 — id leases

The pure draw algebra (OverWidth/Exhausted/Drawn, width 4096, partial
last block leases) is mirrored `lease.rs:35-260` ↔ `writer.ts:75-267`.
Asymmetry: Rust counter mutations carry the writer's fencing token
(`lease.rs:24-25`, `Fenced` on `put_create`/`put_swap`); TS CAS's
unfenced. Counter body is decimal ASCII on both, checked by regex on TS
(`writer.ts:76,247-249`). No cross-golden for the counter document.

### P16/P17 — apply + replica

`fold_pending`'s 7 arms are mirrored one-for-one (`apply.rs:20-59` ↔
`replica.ts:441-473`), as is the publish-law instrument
(`apply.rs:196-205` ↔ `replica.ts:400-406`, identical
`sum − position.g + slot` identity). The surrounding machines
(`replica.rs` 1,268 ↔ `replica.ts` 1,107) mirror open/seed/resume,
one-slot-per-braid stepping, heartbeat every 16 passes
(`replica.rs:37` ↔ `replica.ts:47`), wedge-vs-reseed, sweeps, and the
scream. Constants have already drifted: `WAIT_FOR_POLL_MS` is 10 in
Rust (`replica.rs:42`) and 20 in TS (`replica.ts:50`). All pinning is
via disjoint suites (lane_d/f1–f6 vs the TS crash/recovery tests).

### P18 — writer

TS mirrors the discipline core only: encode → hold Pending + fsync →
local judge → publish or the one loss path → contention scream
(`writer.ts:483-597` ↔ `writer/discipline.rs` + `writer/loss.rs` +
`writer/pending.rs`), with twin constants LOSS_BOUND=16 and the
fixed header offset WRITER_AT (`writer/mod.rs:56`/`writer/loss.rs:20`
↔ `writer.ts:79-82`). Rust-only surface (~1,300 lines): group-commit
drain (`writer/drain.rs`), `AckMode`, checkpoint cadence on the commit
path (`writer/duty.rs`, `CHECKPOINT_EVERY_SUM/BYTES`), `StepHook`
fault injection, `quiesce`, `losses`/`backlog` observability. TS
`publishCheckpoint` (`writer.ts:744-769`) exists but is caller-driven —
no cadence.

### Rust-only modules (nothing to mirror, but the boundary matters)

`gc.rs` (retention + PITR restore over the checkpoint backlink chain),
`checkpointer.rs` (the compact-and-publish role), `inspect.rs`
(document pretty-printer), `bin/duty.rs` (the operational binary). A
future TS gc/restore would have to re-mirror the retention law and the
`Vector` wire coding — one more argument for sharing P6/P10 first.

## What the conformance lane pins, and what it does not

Byte-exact (drift caught on next run of either suite):

- Batch wire + all decode refusal identities — `batch/` (31),
  `fuzz/batch/` (15), storm recipe.
- Braid maps per schema — `braids/` (4).
- Chain discipline verdicts — `chain/` (5).
- Manifest / checkpoint / sidecar documents — `documents/` (23),
  `fuzz/documents/` (18).
- Corpus schema assembly + fingerprints (via the above).

No byte-level pin (drift lands silently or only in a live race):

- Key grammar & key layout (P7) — unit tests only.
- Scratch-lease body (P8) — nothing.
- `Vector::encode`/`parse` wire form (P6) — nothing.
- Id-lease counter document (P15) — nothing.
- Pending fold arms, replica/writer machines, tenants, checkpoint
  publish choreography (P11, P16–P19) — disjoint per-language suites.
- fs lock protocol (P12) — interop outcomes only; body format, location
  and TTL have **already** diverged.
- s3 mapping (P13) — env-gated smoke.

## Ranked: shared-core candidates vs must-stay-mirrored

Strongest candidates for one shared Rust core behind FFI (pure,
bytes-in/values-out, mostly already byte-pinned — the goldens become
the migration safety net, then the duplicate dies):

1. **Codec stack** (P3+P2+P1): `codec.rs` ↔ `codec.ts`+`value.ts`+`bytes.ts`.
   ~975 TS lines deleted against ~1,000 Rust lines; highest-value, best
   pinned, zero IO.
2. **Documents** (P10+P9 parse/render+P6): manifest, checkpoint, sidecar,
   vector algebra — ~600 TS lines; byte-pinned today, and sharing also
   fixes the unpinned `Vector` wire form for free.
3. **Braids + serial-at** (P4): small, already golden-pinned, and the
   two union-finds are structurally different implementations that only
   the goldens reconcile.
4. **Key grammar + layout + scratch body** (P7+P8): pure string/bytes
   with *no* pin today — sharing removes a grade-D surface outright.
5. **Apply decision core** (P16): chain verify + publish-law identity +
   `fold_pending` as a pure function; the db.write itself stays
   per-language, but the verdict logic is shareable.
6. **Descriptor-from-spec** (P5): shareable in principle, but entangled
   with engine types on both sides; do after 1–5.
7. **Error identity names** (P20 head): ride along with 1 — the
   identity strings become one table.

Must stay mirrored (IO-bound, runtime-idiomatic):

- **fs / s3 / mem stores + fence leases** (P12–P14): syscalls, AWS SDKs,
  process liveness. But the *documents* they exchange (lease body,
  counter body) should be specified once — the P12 lease-format fork is
  the cautionary tale. Recommend at minimum adding corpus goldens for
  the lease body, counter body, and scratch body.
- **Replica and writer machines** (P17, P18): event loops over per-language
  concurrency (tokio-less sync Rust vs async Node), LMDB lifecycle
  quirks (`replica.ts:147-158`), timers. Their pure kernels (P16, plus
  the fold/identity constants) should move into the shared core so the
  machines shrink to orchestration.
- **Tenants LRU** (P19): timers, refcounts, directory IO.
- **Checkpoint publish choreography** (P11): store-verb sequencing;
  its decision kernel (checkpoint order, P6) is already on the shared
  list.
- **errors.ts tail / Rust error enums** (P20 tail): idiomatic per
  language by design.

Observed live drift to fix regardless of the refactor: fs lease body
`LEASE/1` vs 3-line (`store.rs:215` vs `store.ts:126-145`), lease
location `~lease/{key}` vs dotfiles (`store/fence.rs:291` vs
`store.ts:28-38`), mutation TTL 5s vs 30s (`store/fence.rs:15` vs
`store.ts:111`), waitFor poll 10ms vs 20ms (`replica.rs:42` vs
`replica.ts:50`), and the braids.rs line-count note above.
