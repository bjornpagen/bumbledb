# 50 — Native-core feasibility: one Rust core for bumbledb-log

Feasibility study for making `@bjornpagen/bumbledb-log` work like the
engine's query builder: one shared Rust core (`crates/bumbledb-log`)
with the TS package reduced to typed payload construction + async IO
glue over napi, replacing today's hand-mirrored pure-TS implementation
(ts-log/src, 6,482 lines across 16 files).

**Verdict up front: feasible, and cheaper than it looks.** The bridge
pattern already exists and is proven three releases deep
(`internalBlake3` in 0.17.1, `internalDescriptor` in 0.18.0 — the
latter deleted a 741-line TS re-derivation; ts/PUBLISHING.md:60-93).
ts-log is *already* unconditionally native: every store verb mints
etags through `internalBlake3` (ts-log/src/store.ts:21, 117-119) and
`replica.db` *is* the native engine handle
(ts-log/src/replica.ts:14, 651, 672). Moving the codec and document
grammars behind napi changes the amount of native surface, not its
presence — so the deployment envelope does not move at all. The one
structural precondition is a feature split in
`crates/bumbledb-log/Cargo.toml`, which today pulls `object_store` +
`tokio` unconditionally (lines 10-11) and would otherwise bloat the
2.8 MB `.node` (ts/npm/darwin-arm64/bumbledb.node; ts/crate/Cargo.lock
has 99 packages vs 245 in the root lock, which carries
reqwest/hyper/rustls/quick-xml for the S3 store).

---

## 1. The beachhead as it stands

Three internal exports cross the FFI today, all `#internal`-doc'd in
ts/src/native.ts and consumed only by ts-log:

| Export | Bridge site | ts-log consumers | Call shape |
|---|---|---|---|
| `internalBlake3(Uint8Array): Uint8Array` | ts/src/native.ts:452-456 → ts/crate/src/lib.rs:44-48 (`blake3_hash`) | store.ts:118 (every etag), replica.ts:134-139 (chain hashes), writer.ts:520,753 (publish digests) | Buffer in/out, per read+write — hot, cheap |
| `internalDescriptor(spec): SealedDescriptor` | ts/src/native.ts:465-469 → ts/crate/src/lib.rs:56-79 (`descriptor`) | descriptor.ts:265 (`fromSealed`) | full SchemaSpec object → wire, **once per theory, WeakMap-cached** (descriptor.ts:196-211) |
| `lower(schema): SchemaSpec` | ts/src/index.ts:123 (pure TS, not FFI) | descriptor.ts:206 | per theory, cached |

The pattern to extend is exactly the engine query builder's: TS builds
a typed payload (query IR / SchemaSpec), Rust owns the semantics, a
sealed per-theory handle amortizes setup (the `Sealed` rosters at
ts/crate/src/lib.rs:81-98 are the precedent for a per-theory
`Codec`/`Vocabulary` handle — `bumbledb_log::codec::Codec::new`
already takes exactly `(descriptor, fingerprint)` at
crates/bumbledb-log/src/codec.rs:501).

---

## 2. Function-family verdicts

### MOVE NOW

**F1. Batch codec — `encodeBatch`/`decodeBatch` + the tagged value
grammar.**
- TS today: ts-log/src/codec.ts:72-157 (encode), 188-297 (decode);
  value.ts:146-257 (`writeTagged`/`readTagged`); bytes.ts
  ByteReader/ByteWriter (most of 304 lines).
- Rust core already has it, byte-pinned: codec.rs `Codec::encode`
  (:527) / `Codec::decode` (:607) over `Vocabulary` (:457).
- Napi shape: `logCodecNew(spec: SchemaSpecWire) -> External<LogCodec>`
  (sealed once per theory, mirroring `instance_builder_new`
  lib.rs:1346), then
  `logEncodeBatch(codec, header: HeaderWire, ops: OpsWire) -> Buffer`
  and `logDecodeBatch(codec, bytes: Buffer) -> DecodedWire | RefusalWire`.
- Marshal cost shape: encode is once per commit (writer.ts:570-581)
  plus once per readdress-after-loss (replica.ts:989-1000) — one row
  marshal of the same class the immediately following `tx_insert`
  already pays; µs against a commit path that fsyncs a sidecar and
  does a conditional PUT. Decode is once per fetched slot on
  catch-up/refresh (replica.ts:390) — Buffer in, values out.
- Deletes: codec.ts encode/decode (~230 lines), value.ts tagged half
  (~150), most of bytes.ts, and the *entire drift surface* of the
  wire grammar (tag table, truncation math, refusal identities).
- Caveat carried to §7: decoded ops are re-marshaled back into the
  engine by `applyOps` → `tx_insert` (replica.ts:354-374), so
  decode-alone is a double crossing per slot; F5 removes it.

**F2. Braid derivation — `braidsOf`, `serialAtStatementsOf`,
`deriveBraids`.**
- TS today: braids.ts:14-47; the union-find in
  descriptor.ts:708-771 (`deriveBraids`) and :142-170 (`serialAtOf`).
- Rust core: braids.rs `braids(descriptor)` (:121),
  `components()` (:86).
- Napi shape: no new call — **extend `DescriptorWire`**
  (ts/crate/src/marshal.rs:1514) to carry
  `braids: [{braid: u32, members: u32[]}]` and
  `serialAt: [{statement, braid}]`. Crossed once per theory, cached.
- Deletes: ~100 lines of union-find + serial-at logic and the risk
  that two union-finds disagree on component heads.

**F3. Manifest + checkpoint documents —
`parseManifest`/`renderManifest`/`parseCheckpoint`/`renderCheckpoint`/`auditCatalog`.**
- TS today: manifest.ts:89-206 (whole file, 209 lines).
- Rust core: manifest.rs `Manifest::render/parse` (:197/:206),
  `Checkpoint::render/parse/digest/vector` (:284/:308/:365/:379).
- Napi shape: bytes-in/values-out on the codec handle —
  `logParseManifest(codec, Buffer) -> {fingerprint, checkpoint}`,
  `logParseCheckpoint(codec, Buffer) -> CheckpointWire`, renders the
  inverse.
- Marshal cost shape: cold. Manifest parse runs on manifest *change*
  and on the heartbeat (every 16th pass — replica.ts:46, 537);
  checkpoint parse on pointer movement (replica.ts:598-612); renders
  on checkpoint publish only (writer.ts:752-753).
- Deletes: manifest.ts whole.

**F4. Chain sidecar grammar — `parseSidecar`/`renderSidecar`.**
- TS today: chain.ts:86-180 (~95 lines of grammar). The IO half
  (`readSidecar`/`writeSidecar`, chain.ts:182-222) **stays TS** — it
  is fs temp+rename+fsync.
- Rust core: sidecar.rs `Chain::render/parse` (:204/:238); its IO
  siblings `write_atomic`/`read` (:310/:336) stay unlinked.
- Marshal cost shape: parse once at open; render per `persistSidecar`
  — i.e. every applied slot and every commit (replica.ts:260-262,
  writer.ts:584). Warm, but the payload is 52 B per braid plus
  pending bytes; one Buffer copy per persist is noise next to the
  fsync it precedes.

### MOVE LATER

**F5. `applySlot` as one crossing — decode + chain discipline +
db.write + publish-law instrument in Rust.**
- The observation: `core.db` is already `External<DbHandle>` wrapping
  the Rust `Db<SchemaDescriptor>` (lib.rs:249-257), and
  `bumbledb_log::apply::apply(db, chain, codec, braid, slot, bytes)`
  (apply.rs:136) is exactly today's TS sequence
  decodeBatch → verifyChain → applyOps → no-op instrument
  (replica.ts:383-410) with the sidecar persist deliberately left to
  the caller ("persisting it is the caller's step two",
  apply.rs:125-130 — which matches replica.ts:408).
- Napi shape: `logApplySlot(db, codec, chainWire, braid, slot,
  bytes: Buffer) -> {tag: advanced|absorbed, generation, entry} |
  {tag: rejected, violations} | {tag: refused, refusal}`.
- Payoff: **zero value marshaling per slot** (today: decode values
  out of Rust in TS, lift handles in `factOf` replica.ts:326-351,
  push them back into Rust via `tx_insert`). Catch-up of N slots
  becomes N single crossings. Also deletes `verifyChain`
  (codec.ts:313-332), `foldPending` (replica.ts:450-473 — its Rust
  twin is apply.rs:41-59), and `factOf`.
- Why LATER: it must thread the napi single-writer guard
  (`writing: AtomicBool`, lib.rs:965-971) so a native apply cannot
  interleave with a JS `db.write`; and it changes the machine's
  granularity, so land it after F1-F4 prove the topology. It is the
  step where "pure transitions call the core" becomes literal.

**F6. Scratch-lease encode/parse + `verifyChain` standalone.**
- keys.ts:141-162 (33-byte fixed record) and codec.ts:313-332
  (three compares). Trivial; they ride along with F3/F5 for free but
  are not worth their own crossings. If F5 lands, `verifyChain` dies
  inside it; the scratch record can move with the manifest family or
  stay as 22 lines of TS with no drift risk worth the ceremony.

### KEEP TS

**F7. `Vector` — sum/dominates/order/at/advance.**
- vector.ts:33-170. Hot pure bigint arithmetic used inside poll
  predicates (`waitFor` dominance check, replica.ts:1064-1071) and in
  `chainGeneration` on every pass (chain.ts:65-73). A napi crossing
  per comparison costs more than the ~80 lines it would delete.
  `Vector.encode/parse` (vector.ts:117-169) have **zero src
  consumers** (only the public export, index.ts:51) — public session-
  token surface; leave them, or move with F1 if the export ever gains
  a hot caller. Overflow-as-refusal semantics are already pinned by
  conformance documents.

**F8. Key grammar — keys.ts.**
- keys.ts:75-231: string assembly on every store op (hottest path in
  the file), tilde-family + Cf-strip segment law identical to Rust's
  store.rs:82-145. Crossing napi to build a `log/{braid}/{hex16}`
  string is a pessimization. Drift risk is real but bounded (one
  divergence found: TS TILDE_FAMILY has 15 lookalikes
  keys.ts:27-43, Rust `is_tilde_lookalike` lists 10 plus an
  NFKC-normalize on the TS side keys.ts:67 — worth a test-only
  parity pin, not a runtime crossing).

**F9. Stores — fsStore/memStore/s3Store.**
- store.ts:490-706 and store-s3.ts: Node fs (link(2) create-only,
  fsync discipline, lease files) and `@aws-sdk/client-s3`
  (store-s3.ts:13). All async IO; the Rust `ObjectStore` trait is
  deliberately synchronous and its S3 impl refuses calls from an
  async context and drives a dedicated multi-thread tokio runtime
  (store.rs:1-5, 362-366; s3.rs:66-79, 130-139) — hostile to the
  Node event loop by construction. Keep TS; the store *contract* is
  pinned by the interop lane, not shared code.

**F10. The machine loops — replica.ts/writer.ts steppers.**
- Every transition awaits store verbs (`stepBraid` awaits `get`
  replica.ts:498; `publishPending` awaits `putCreate`
  writer.ts:500; the gate serializes async work replica.ts:120-131).
  The machine can and should stay TS while its pure transitions
  (F1-F5) call the core: `applySlot` already isolates the pure part
  behind an interface that takes bytes and returns a verdict, and
  the recorder (`recorderOf`/`lowerFact`, writer.ts:286-367) is the
  typed-payload-construction half that mirrors the engine SDK's
  role. Confirmed: nothing in the steppers needs to move for F1-F4;
  F5 keeps the stepper in TS and only fattens one call.

**F11. Tenants LRU + Lambda glue.**
- tenants.ts:1-49 (LRU, refcount pins, fenced dir leases — all fs +
  policy) and examples/lambda (non-normative, README.md:3-5). Keep TS.

---

## 3. Recommended napi topology: extend `ts/crate`, do not mint a second crate

**Extend `ts/crate` (bumbledb-node), gated behind a pure-core feature
of `crates/bumbledb-log`.** Evidence:

- `ts/crate` is a standalone workspace (`[workspace]` at
  ts/crate/Cargo.toml:8, excluded from the root at Cargo.toml:13-16)
  with one path dep on `bumbledb` (:28). Adding
  `bumbledb-log = { path = "../../crates/bumbledb-log", default-features = false }`
  is one line — **but requires the feature split first** (§6,
  blocker B1), because bumbledb-log unconditionally pulls
  `object_store` (aws) + `tokio` rt-multi-thread
  (crates/bumbledb-log/Cargo.toml:10-11). The napi crate needs only
  codec/braids/vector/manifest/sidecar/apply, whose deps are
  `bumbledb` + `blake3`.
- Platform shipping is the decisive cost: the `.node` rides exactly
  two platform packages, `@bjornpagen/bumbledb-darwin-arm64` and
  `-linux-arm64` (ts/npm/*/package.json, both 0.19.2), loaded by name
  at import (ts/src/native.ts:413-440, `SHIPPED_PLATFORMS`). A new
  napi crate in ts-log would need two *more* platform packages, a
  second by-name loader, a second amazonlinux:2023 CI artifact
  (PUBLISHING.md:5-9), and a second OTP publish ceremony per release
  — pure duplication of ts/scripts/build.ts + platform.ts machinery.
- The lockstep makes the coupling free: every published package
  spells the same semver ("main == platform == napi crate == engine
  == C ABI", asserted at build — ts/scripts/build.ts:26-29;
  PUBLISHING.md:47-56), and ts-log already peers on the exact engine
  version (`peerDependencies: ^0.19.2`, ts-log/package.json). Adding
  internal exports to `@bjornpagen/bumbledb` for the log driver is
  the *established release shape*: 0.17.1 existed to add
  `internalBlake3`, 0.18.0 to add `internalDescriptor`
  (PUBLISHING.md:60-93).
- The dumb-bridge law survives: "no logic beyond marshaling will EVER
  live in this crate" (ts/crate/src/lib.rs:1-2). Every proposed
  function is a marshal shell over `bumbledb_log::{codec, braids,
  manifest, sidecar, apply}` — same discipline as `descriptor` and
  `blake3_hash` today. The smart code lives in the workspace crate
  where the Rust battery (36 test lanes,
  crates/bumbledb-log/tests/) already pins it.

Naming: keep the `internal*` convention (`internalLogCodec`,
`internalLogApply`, …) — undocumented SDK surface, single consumer,
consumer-roster law (PUBLISHING.md:84-88).

---

## 4. Deployment-constraint truth table

The five cases are canon — proposals/settlement/00-canon.md was
retired at commit 49d45b5c ("Settlement and lockstep live in the code
and the gates"); the text is recoverable via
`git show 49d45b5c^:proposals/settlement/00-canon.md` (five cases at
lines 64-80 of that file).

| Case | Native today? | Native after refactor | Evidence |
|---|---|---|---|
| 1. Next.js on Vercel Fluid | **Yes already** — replica singleton at module scope imports the SDK, which loads `.node` at import time | Unchanged | canon case 1; ts-log/README.md:36-49 (Fluid host opens replica/writer at module scope); ts/src/native.ts:439 (load at import). Fluid is the Node runtime, not an edge isolate. The zero-native-on-Vercel *alternative* also exists and is documented: the Lambda example's Vercel side is fetch-only (examples/lambda/README.md:38, 142-146) |
| 2. Embedded macOS (Apple Silicon) | Yes (napi or C ABI) | Unchanged | canon case 2; darwin-arm64 platform package ships the binary |
| 3. Long-lived server (resident) | Yes | Unchanged | canon case 3; linux-arm64 package, AL2023/glibc 2.34 build (ts/npm/linux-arm64/package.json:4) |
| 4. Distributed per-tenant | Yes (same runtime as 1/3) | Unchanged | canon case 4; tenants.ts is TS policy over the same native replica |
| 5. Local fleet (N processes, one FsStore prefix) | Yes, per process | Unchanged | canon case 5; the multiprocess and interop children are plain `node` processes that import the packages and load `.node` via `createRequire` per process (ts-log/test/multiprocess-child.ts:12-17, interop-child.ts:16; spawned by crates/bumbledb-log/tests/lane_b_interop.rs:51-69). Cross-process arbitration is link(2)/CAS, not shared memory — nothing about child loading changes |
| Lambda specifically | Yes — the bundle installs `@bjornpagen/bumbledb-linux-arm64@0.19.x` from the registry and the duty binary ships at `/opt/bin` via a layer | Unchanged | examples/lambda/README.md:7-8, 43-49, 77-92 |
| Edge runtimes (Cloudflare Workers, Vercel Edge) | **Broken today** (no napi, and no fs/LMDB either) | Still broken | Not one of the five cases; canon lists no edge case. ts-log cannot run there now — `Db` is native and `fsStore` needs a filesystem |
| Non-shipped platforms (x64, musl, Windows) | **Broken today** — loader refuses by name | Still broken | ts/src/native.ts:413, 424-429: "ships darwin-arm64, linux-arm64 only" |

**Conclusion: no supported case is native-hostile, and no case changes
state.** The honest sentence is: ts-log has no native-free code path
to lose. The refactor's deployment risk is zero; the platform-matrix
risk (x64 etc.) is pre-existing and owned by the engine package.

---

## 5. The parity dividend

Today two implementations are proven equal by three artifact classes:
the v3 conformance inventory (version 3: 7 batch_ok + 24
batch_refusal + 5 chain + 23 documents + 33 fuzz_materialised + a
storm recipe; crates/bumbledb-log/conformance/v3/inventory.json, 61
files in batch/ alone), walked by *both* drivers
(crates/bumbledb-log/tests/conformance_v3.rs and
ts-log/test/conformance-v3.test.ts); the parity lane asserting
identical named outcomes on identical bytes (f7_parity.rs +
ts-log/test/parity.test.ts); and mirrored crash step tables
(f4_crash.rs ↔ crash-matrix.test.ts:1-6).

**Dies when the grammars are one implementation (F1-F4):**

| Artifact | Lines | Why it dies |
|---|---|---|
| ts-log/test/parity.test.ts | 451 | its entire purpose is "two implementations agree byte-for-byte" |
| ts-log/test/conformance-v3.test.ts | 695 | the TS byte-grammar walker; the Rust walker becomes *the* walker |
| ts-log/test/conformance-v3-support.ts | 247 | corpus→SchemaSpec assembly for the dead walker |
| ts-log/test/codec.test.ts | 278 | unit tests of a deleted TS codec |
| ts-log/test/bytes.test.ts | 10 | primitives die with bytes.ts |
| ts-log/test/chain.test.ts (grammar half) | ~40 of 68 | parse/render goldens move behind the one implementation |
| src: codec.ts, manifest.ts, value.ts tagged half, chain.ts grammar half, bytes.ts most, deriveBraids/serialAtOf/assembleFromSpec | ~1,100-1,300 | the mirrored halves |

≈ **1,700 test lines + ~1,200 src lines deleted**, and with them the
drift-risk *class* the canon calls out ("a behavioral divergence
between the drivers is a conformance failure" — canon §representations
1; refusal identities "spelled identically by both drivers and pinned
by the conformance inventory" — canon §representations 6). After the
move, a grammar change is one Rust edit plus regenerated goldens; no
second implementation can lag.

**Remains (and should):**
- The store contract lane: store.test.ts (244), s3-smoke.test.ts
  (184), and the cross-driver interop children (interop-child.ts
  driven by lane_b_interop.rs) — the fs/S3 stores stay two
  implementations of one *contract*, so this lane is the enduring
  parity obligation.
- The machine lane: crash-matrix.test.ts (352 — still mirrors
  f4_crash.rs while there are two steppers), replica-writer (404),
  writer (248), recovery (139), multiprocess (153+57), tenants (170),
  replica-open (112), temporal-gate (62), checkpoint-orphan (169).
- fingerprint.test.ts (115 — the cross-host PIN replay), errors.test
  (25), keys.test (52, plus the suggested segment-grammar parity pin).
- A thin *identity-mapping* conformance run should replace the dead
  TS walker: same corpus, but asserting that the napi-thrown refusals
  surface in JS as the exact `RefusalCause` table
  (ts-log/src/errors.ts:72-95). It pins the FFI error mapping, not
  the byte grammar — an order of magnitude smaller than today's 695
  lines.

Residual drift risk after F1-F4: the machine itself (two steppers,
two crash matrices). That is the honest boundary of this refactor —
retiring it means moving the stepper, which the store-verb-await
structure (F10) makes a different, much larger campaign.

---

## 6. Rust-side impact: what a napi host would stress

Audited crates/bumbledb-log for single-consumer assumptions:

- **No interior-mutability landmines.** Zero `Rc`, `RefCell`,
  `thread_local`, `static mut`, or `Cell` in src (grep clean). The
  pure types (`Codec`, `Vocabulary`, `Braids`, `Vector`, `Manifest`,
  `Checkpoint`, `Chain`) are plain owned data — safe to hold in the
  napi `Sealed` Arc exactly like the rosters (lib.rs:93-98). The one
  process-global is the jitter PRNG `AtomicU64` (store.rs:534-551) —
  store-side only, and thread-safe anyway.
- **The tokio runtime is confined to `store/s3.rs`** and is
  per-`S3Store` (a dedicated multi-thread `Runtime`, s3.rs:73,
  guarded so no verb ever `block_on`s from a foreign async context —
  s3.rs:79, 130-139). Under the pure-core feature split it is not
  even linked into the `.node`. If it ever were: the JS main thread
  and napi AsyncTask workers are not tokio contexts, so the
  `Handle::try_current` refusal would not fire — but the per-store
  runtime threads and blocking verbs are the wrong shape for Node
  regardless; keep the TS s3Store (F9).
- **`Db` is already multi-thread-proven in the napi host**: the
  bridge holds `Arc<Engine>` and runs `Db::from_instance` on an
  AsyncTask worker thread (lib.rs:531-572), so `Send + Sync` on the
  engine handle is an existing fact, not a new bet.
- **The one real hazard is F5's write path**: `apply::apply` does
  `db.write` internally (apply.rs:136+), and the napi bridge
  enforces single-writer per handle with `writing: AtomicBool`
  (lib.rs:965-971). A native `logApplySlot` must acquire that same
  flag or a JS `db.write` could race it. Mechanical, but it is the
  reason F5 is LATER.
- **Sidecar/store IO in the crate stays unlinked** (`sidecar.rs
  write_atomic/read` :310/:336, `replica.rs`/`writer/` machine,
  `gc.rs`, `checkpointer.rs`, `lease.rs`, `tenants.rs`, the duty
  binary). The napi crate consumes `codec`, `braids`, `vector`,
  `manifest` (parse/render/digest only), `sidecar` (Chain
  parse/render only), `apply` — nothing that opens a file or a
  socket.
- **Corpus schemas that the engine seal refuses**: the TS walker
  today assembles descriptors the seal would reject via
  `assembleFromSpec` (descriptor.ts:388-391). The Rust side already
  has the same needs solved — `schema_file.rs` parses the corpus
  schema object into a raw `SchemaDescriptor` "so a second descriptor
  grammar cannot exist" (schema_file.rs:1-4). If any TS-visible
  surface still needs a non-sealed codec after F1 (it shouldn't —
  the walker dies), the entry exists.

---

## 7. Honest blockers

- **B1 — Feature split in bumbledb-log (required, small).** No
  `[features]` exist (Cargo.toml); `object_store`+`tokio` are
  unconditional. Without a `store` (default-on) feature isolating
  `store/s3.rs` (+ `fs.rs`/`mem.rs` if desired), depending from
  ts/crate drags ~146 extra crates (99 → ~245 by lockfile
  comparison) including reqwest/rustls into a 2.8 MB `.node`, plus
  their compile time into every `pnpm build`
  (ts/scripts/build.ts:35-53 does a clean cargo release build in the
  publish path).
- **B2 — Error-identity mapping is the new parity surface.** TS
  refusals are `@superbuilders/errors` sentinels carrying typed cause
  data (`refuse`/`refusalOf`, errors.ts:72-95); the bridge throws
  `{kind, message}` objects (native.ts:471-489). Every
  `DecodeError`/`ManifestError`/`SidecarError`/`ApplyRefusal` arm
  must map onto the existing `RefusalCause` union *exactly* (the
  canon names the identities: Malformed, Version, UnknownBraid,
  Overflow, … spelled identically by both drivers). Mechanical, but
  it is the one place the "one implementation" claim can silently
  leak — hence the thin identity-mapping conformance lane in §5.
- **B3 — The decode/apply double-marshal (F1 without F5).** Landing
  decodeBatch alone makes catch-up cross values twice per slot
  (Rust→JS in decode, JS→Rust in `tx_insert`). Acceptable
  interim — the slot fetch is a store GET and dominates — but the
  design should aim at F5, where the crossing count per slot is one
  and the payload is one Buffer.
- **B4 — Closed-handle lifting stays split.** The wire carries raw
  values (u64 roster ids), the SDK API speaks handles;
  `factOf`/`lowerFact` (replica.ts:326-351, writer.ts:286-319) do
  the lift/lower in TS against `closedRef` metadata. F1 keeps this;
  F5 deletes `factOf` (Rust applies raw values directly) but the
  recorder's `lowerFact` remains as the typed-payload layer — by
  design, not debt.
- **B5 — Release ceremony coupling.** Any core move is a lockstep
  release of the engine SDK + both platform packages + ts-log
  (PUBLISHING.md:1-16: owner-run, darwin host, interactive OTP per
  package; the linux `.node` arrives from the AL2023 CI cell). Cheap
  by policy, but it means the refactor cannot ship ts-log-only; the
  first slice must ride an engine version bump like 0.17.1/0.18.0
  did.
- **B6 — Two small pre-existing drifts to reconcile in passing.**
  (a) The tilde-lookalike sets differ: TS lists 15 code points +
  NFKC check (keys.ts:27-43, 67), Rust lists 10 (store.rs:99-112) —
  today's grammars can disagree on e.g. U+2E1E-prefixed segments;
  keys stay TS (F8) so this wants a test pin either way. (b) Dead
  residue from the 0.18.0 beachhead: `writeCanonicalLiteral`
  (value.ts:278-323) and the big-endian half of bytes.ts have zero
  consumers since the fingerprint mirror deleted — they should die
  with F1's sweep regardless.

## 8. Sequencing sketch

1. B1 feature split; ts/crate gains `bumbledb-log` (pure core) dep.
2. F2 (braids on DescriptorWire) — smallest, zero new call sites.
3. F1 (codec handle + encode/decode) + B2 error mapping + retire the
   TS walkers per §5.
4. F3/F4 (documents + sidecar grammar); delete manifest.ts, shrink
   chain.ts to IO.
5. F5 (`logApplySlot`) once the topology is proven, threading the
   write guard.
6. Standing: interop/store lanes and the machine crash matrices stay
   as the two remaining parity obligations.
