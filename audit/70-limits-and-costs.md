# 70 — Limits and costs: the honest ledger for a shared Rust/napi core

The question: what does it really cost to move ts-log's pure machinery
(codec, manifest, chain document, vector, braid derivation) behind the
engine's native binding instead of mirroring it in TypeScript.
Brooks's frame is the method: a representation change removes
accidental complexity, never essential — and it is locally expensive.
Every cost below is graded **blocker / real-but-payable / negligible**,
with evidence from this tree and its history. No hand-waving in either
direction: several costs conventionally attributed to this refactor
turn out to be **already paid**, and one cost conventionally dismissed
turns out to be the only real one.

---

## 1. FFI call-overhead shape — negligible, with the honest caveat named

### Call frequencies (the hot paths, from the loops themselves)

| Pure call | Site | Frequency | What the same iteration already pays |
| --- | --- | --- | --- |
| `encodeBatch` | `ts-log/src/writer.ts:570-581` (`disciplineCommit`), `ts-log/src/replica.ts:989-1000` (`readdressPending`, loss path only) | **once per commit** (+once per live-tip loss) | one sidecar fsync (`persistSidecar`, writer.ts:584 → `chain.ts:199-222`: temp write + `handle.sync()` + rename + parent-dir sync), one full native `db.write` (`applyOps`, writer.ts:587), one store `putCreate` — an S3 PUT at **p50 ≈ 20–60 ms** per the retired canon (`proposals/00-product.md:128` at `48d47a48^`: "Standard S3 PUT p50 ≈ 20–60 ms") |
| `decodeBatch` | `ts-log/src/replica.ts:390` (`applySlot`), reached once per slot from `stepBraid` (replica.ts:512); edge calls at open: replica.ts:807, replica.ts:846, writer.ts:623 | **once per applied slot** during catch-up | one store `get` per slot (network GET or fs read, replica.ts:498), one native `db.write` (replica.ts:393), one sidecar fsync (replica.ts:408) |
| `verifyChain` | `ts-log/src/replica.ts:392` | once per applied slot | three byte/int compares (`codec.ts:313-332`) — trivial in either language |
| `parseManifest` | `ts-log/src/replica.ts:583` via `refreshManifest` (replica.ts:620-634) | at open, then **every 16th pass** (`HEARTBEAT_PASSES = 16`, replica.ts:47, gated at replica.ts:537), and only after `getIfChanged` reports a change (replica.ts:622-626) | one conditional GET |
| `parseCheckpoint` | `ts-log/src/replica.ts:602` | only when the manifest's checkpoint digest moved (replica.ts:597) | one GET of the checkpoint doc, and (on seed) a full `.mdb` download |
| `descriptorOf` / `braidsOf` | `ts-log/src/descriptor.ts:198-211` | **once per theory value**, WeakMap-cached (descriptor.ts:196) | already crosses napi today via `internalDescriptor` (descriptor.ts:265) |

### Payload sizes (from the encoders themselves)

- **Batch**: fixed 104-byte header (magic 4 + version 2 + flags 2 +
  fingerprint 32 + braid 4 + gen 8 + prev 32 + writer 8 + ts 8 +
  op_count 4 — `codec.ts:124-133`; `WRITER_AT = 84` at writer.ts:82
  confirms the layout) plus 9 bytes per op plus tagged cells per row.
  Typical commit: hundreds of bytes to low KB; bounded by u32 counts.
- **Manifest**: 34 or 66 bytes total (`renderManifest`,
  `manifest.ts:89-95` — `ByteWriter(66)`).
- **Checkpoint doc**: 46 + 52·braids (+32 if prev) bytes
  (`renderCheckpoint`, manifest.ts:132-155). A 4-braid theory: ~290 B.
- **Sidecar**: 6 + 52·braids bytes Settled; + 16 + batch bytes Pending
  (`renderSidecar`, chain.ts:86-117).

### The honest shape of the crossing

The naive framing "bytes in, bytes out" is wrong in one direction and
that direction must be priced. `encodeBatch` is structured-in/bytes-out
and `decodeBatch` is bytes-in/**structured-out**: the decoded ops are
JS objects — relation names, per-cell `bigint`/string/`Uint8Array`
values — because `applyOps` must lift them into the SDK's named facts
(`factOf`, replica.ts:326-351). A native decode does not avoid that
materialization; it moves it to the boundary, paying per-cell napi
value creation. The upper bound on that cost is known **from this
repo's own engineering**: the engine already crosses the *same rows*
JS→native on every apply, and it deliberately flattened that crossing
to one row-major array precisely because per-item crossings cost
(`ts/src/native.ts:346-357`: "`cells` is ONE flat row-major array …
the one collection crossing"). So a native decode roughly doubles a
structured crossing the slot already makes once — microseconds per
typical batch, low milliseconds for a pathological bulk batch.

### The comparison that settles it

Every one of these calls sits next to (a) an fsync pair the sidecar
discipline mandates per slot and per commit (chain.ts:199-222,
replica.ts:266-285), and (b) a network CAS the protocol exists to pay
(PUT p50 20–60 ms; the measured loss-path cost in the one-path ruling
was **67.2 ms vs 64.3 ms** — commit `94c8e122`, "the fsync floor
dominates" per `proposals/00-product.md:135-137` at `48d47a48^`). A
sync napi call with a 104-byte-header batch is 3–5 orders of magnitude
under the floor the loop already stands on. The calls are also
naturally synchronous — the engine already exposes sync napi entry
points of exactly this shape (`blake3Hash`, `descriptor`;
native.ts:306-308) — so the package's temporal law ("the pure pair are
synchronous", `ts-log/README.md:32-34`) survives untouched.

**Grade: negligible.** No benchmark needed; the structure decides it.
The only sub-caveat is the structured decode marshal, bounded above by
the `txInsert` crossing the same rows already make.

---

## 2. Build/packaging coupling — the headline cost is already paid; the real new cost is a crate split

### What is already true today (and commonly mis-stated)

ts-log is *not* runnable without the engine's platform binary, and has
not been since `internalDescriptor` landed (commit `3679f445`:
"internalDescriptor landed first so Lane B can collapse the descriptor
re-derivation onto one engine-produced truth"). The chain:

- `ts/src/native.ts:439` — `const binding: NativeBinding =
  loadNativeBinding(process.platform, process.arch)` executes at
  **module scope**. Importing anything of value from
  `@bjornpagen/bumbledb` dlopens the `.node` or throws
  (native.ts:421-437; `SHIPPED_PLATFORMS = ["darwin-arm64",
  "linux-arm64"]`, native.ts:413).
- `ts-log/src/descriptor.ts:20` value-imports `internalDescriptor,
  lower`. `codec.ts:14`, `braids.ts:9`, `manifest.ts:16`,
  `vector.ts:9`, `chain.ts:18` all value-import from `#descriptor.ts`.
  **The pure codec cannot even be imported without the native module.**
- The only native-free modules are `bytes.ts`, `keys.ts`
  (type-only descriptor import, keys.ts:17), `value.ts` (type-only
  engine import, value.ts:10), and `errors.ts` (type-only,
  errors.ts:14).
- Every test file transitively hits native: `fixtures.ts` builds
  schemas from the engine, `parity.test.ts:6` and
  `conformance-v3.test.ts` import the codec, `store.test.ts:6` imports
  `internalBlake3` directly. **There is no ts-log test lane that runs
  without a built `.node` today.**
- CI proves it: `.github/workflows/bumbledb-log.yml` runs "Build the
  napi .node" (`node scripts/build.ts` in ts/) *before* installing
  ts-log and running `scripts/battery.sh`, on both the AL2023-arm64
  job and the darwin job. `battery.sh:31-32` runs the ts-log suite
  after the ts/ suite (which builds dist + the binary).

So the cost "after the refactor ts-log hard-requires platform binaries
at runtime for its pure functions" is **zero marginal** — it
hard-requires them at *import time* already, for the pure functions
specifically (the descriptor every pure function takes is sealed by
native code). Grade of this line item: **already paid / negligible**.

### What is genuinely new

1. **The napi crate must learn the log crate.** `ts/crate/Cargo.toml`
   depends only on `crates/bumbledb` today. The shared codec cannot be
   the engine's `descriptor()` path: the conformance corpus pins the
   codec on schema shapes **the engine seal refuses**
   (`ts-log/src/descriptor.ts:387-391` — `assembleFromSpec` exists
   because "the conformance corpus pins the codec and the braid map on
   shapes the engine seal refuses"); the Rust side that accepts those
   shapes is `crates/bumbledb-log/src/schema_file.rs` (646 lines) plus
   `codec.rs`. So `bumbledb-node` must grow a path dependency on
   `crates/bumbledb-log` — which **unconditionally** pulls
   `object_store` (aws) and `tokio` rt-multi-thread
   (`crates/bumbledb-log/Cargo.toml:8-11`) into a cdylib that will
   never open a socket. The honest fixes are (a) a crate split
   (`bumbledb-log-codec` with no async deps) — new module boundary,
   new roster entry for the lockstep gate, or (b) feature-gating the
   log crate — a features matrix the workspace currently doesn't have.
   Either way `ts/crate/Cargo.lock` (its own workspace,
   `ts/crate/Cargo.toml:9`) starts tracking the log crate's tree, and
   the CI cache keys already hash it (bumbledb-log.yml cache key
   `hashFiles('rust-toolchain.toml', 'Cargo.lock',
   'ts/crate/Cargo.lock')`). **Real-but-payable**, and it is the
   largest single piece of new work outside the boundary shim.
2. **The codec joins the slow edit loop.** Today a codec change is
   edit-`.ts` → `node --test` (ts-log has no build step at all —
   `package.json` ships `"files": ["src"]` and exports
   `./src/index.ts`). After, a codec change is edit-`.rs` → `cargo
   build --release` of `bumbledb-node` (with `lto = "fat"`,
   `codegen-units = 1`, ts/crate/Cargo.toml:33-35 — the slowest
   possible profile) → rebuild dist → `node --test`. The lockstep
   already forces a remint per version bump (commit `e6c21b98`: "A
   lockstep bump remints the napi crate and the ts-log peer");
   this extends that to every codec iteration. **Real-but-payable** —
   a DX tax on the driver's most-specified surface, not a correctness
   cost.
3. **ts-log stays platformless in form.** The native code rides the
   engine peer exactly as `internalBlake3`/`internalDescriptor` do
   (the blake3 dependency ruling, retired `proposals/70-typescript.md`
   at `48d47a48^`: "Blake3 rides the engine package's existing native
   binding … no JS blake3 dependency exists"). No new npm platform
   packages, no change to `SHIPPED_PLATFORMS`. **Negligible.**

---

## 3. Debugging and DX — real, enumerable, none of it a blocker

### Error identity across the boundary, as the engine marshals it today

The engine's napi crate throws **plain JS Error objects carrying a
string `kind` and a message** — nothing else. `ts/crate/src/lib.rs:
102-125` (`WireError { kind: Option<&'static str>, message: String }`,
`thrown`), `ts/crate/src/marshal.rs:43-58` (`throw_kind_message` →
`env.create_error` + `error.set("kind", kind)`). The TS side
reconstructs: `ts/src/native.ts:471-498` — `isEngineThrow` matches
`{kind, message}`, `errorFromThrow` mints a **new** Error at the
boundary, `bridged` wraps with context. Identity is a string; the
stack begins at the catch site.

ts-log's error discipline is stronger than that channel can carry
as-is:

- **Sentinel identity**: `errors.is(e, ErrRefused)` matches by
  reference on exported sentinel values (`ts-log/src/errors.ts:22-63`,
  README.md:114-127 "checked with `errors.is`, never by message
  strings").
- **Structured cause data in WeakMaps keyed by the exact Error
  instance**: `errors.ts:145-151` (`refusalData`, `chainData`, …),
  read back by `refusalOf`/`chainMismatchOf` (errors.ts:203-229).
  A WeakMap association cannot cross a process-of-minting boundary;
  the shim must catch, map, and re-mint.

The good news is structural: the Rust codec already exposes the
cross-implementation identity as data — `DecodeError::identity()`
(`crates/bumbledb-log/src/codec.rs:349-370`) returns exactly the
strings ts-log's `RefusalCause.kind` uses (errors.ts:66-69: "the
cross-implementation identity names the Rust driver's
`DecodeError::identity` pins — the conformance corpus compares them
string for string"). So a boundary mint-table (identity string →
sentinel + cause) is mechanical. What is **not** mechanical:

- **The cause payloads differ in shape.** TS carries *names*:
  `Truncated { at: "op count" }`, `TagMismatch { relation: "note",
  row, field: "body" }` (errors.ts:72-89). Rust carries *ordinals and
  offsets*: `Truncated { offset: usize }`, `TagMismatch { relation:
  RelationId, row, field: u16, expected, got }` (codec.rs:281-345).
  Preserving the TS public API (`refusalOf` returning named fields)
  means either widening the Rust variants to carry resolved names
  (Rust has the descriptor in hand, so it can) or serializing a cause
  object across napi per refusal and mapping ordinals→names in the
  shim. Enumerable work; an observable-API break if skipped.
  **Real-but-payable.**
- **Stack traces.** Today a `Version` refusal thrown inside
  `decodeBatch` has a JS stack through `applySlot → stepBraid →
  runPass → refresh` — the whole causal spine. After, the stack starts
  at the shim (marshal.rs:54-58 creates the JS error inside the native
  call). The system's own idiom mitigates this — refusals are
  identity + cause data + a detail string, never stack-mined — but the
  interactive story degrades: a consumer can read, breakpoint, and
  `console.log` inside `codec.ts` today; after, the decode is opaque
  to anyone who doesn't build Rust. **Real-but-payable** (a genuine DX
  regression, honestly priced as such).
- **Refusal sites that must stay JS regardless**: `braidOf`
  (replica.ts:161-168, the `UnknownBraid` wire-boundary check),
  `foldPending`, the store wrappers (`wrapStore`, errors.ts:234-236),
  `headerWriter`'s fixed-offset raw reads (writer.ts:401-420 — the
  deposition law deliberately reads bytes without a decode). The
  boundary does not swallow the whole refusal roster; it bisects it,
  and the bisection line must be documented or the "one identity,
  spelled identically by both drivers" law (canon rep 6) quietly gains
  a third speller: the shim.

---

## 4. The child processes — no new constraint; the evidence

`ts-log/test/multiprocess-child.ts:17` imports `#writer.ts` →
`#replica.ts` → `@bjornpagen/bumbledb` → module-scope native load.
`ts-log/test/interop-child.ts:16` imports `internalBlake3` directly.
Both children **already dlopen the `.node` on every spawn**. They are
spawned as full processes — `spawn(process.execPath, [childScript,
...])` (`multiprocess.test.ts:44-45`) and `Command::new("node")`
(`crates/bumbledb-log/tests/lane_b_interop.rs:63`) — not
worker_threads (no `worker_threads` usage anywhere in ts-log/src or
test), so napi context-awareness is not in play; each process gets its
own binding instance and its own LMDB environment registration (the
per-process env law, replica.ts:147-154). Moving more pure functions
into the same already-loaded binding adds zero module-loading
constraint and zero marginal dlopen. CI already stages the binary for
exactly these children (bumbledb-log.yml: "lane_b_interop.rs spawns
ts-log/test/interop-child.ts, which resolves @bjornpagen/bumbledb from
this install"). **Grade: negligible.**

---

## 5. Version coupling — the lockstep already collapsed it; one residual worth naming

The facts: ts-log's peer is `^0.19.2` (ts-log/package.json:44-46), the
lockstep gate asserts the peer is exactly `^<workspace version>` at
every build (`ts/scripts/build.ts:213-225`), the roster gate proves
every versioned manifest equals the one workspace fact
(`scripts/version-roster.txt`, build.ts:195-207), and the platform
binaries are exact-pinned `optionalDependencies` injected at pack
(`ts/scripts/pin.ts:50-56`). The retired lockstep decision
(`proposals/lockstep/20-one-version.md` at `49d45b5c^`): "There is one
version, it has one writer … the ts-log peer range is derived, not
written." Engine 0.19.0 + log 0.19.0 mandatory pairing is thus
**already the published law**, refactor or no refactor — the install
matrix a consumer can legally construct is engine `>=0.19.x` under the
caret with log `0.19.x`.

The residual: with a shared core, the log's **wire bytes become a
function of the *resolved* engine binary**, not the installed log
package. A consumer holding log 0.19.2 with engine 0.19.9 (caret-legal
today) would encode with 0.19.9's codec. Two honest observations cut
this down:

1. **It is already true for the wire's identity field.** The
   fingerprint in every batch, manifest, and checkpoint is computed by
   the resolved engine's native seal (`descriptor.ts:265`
   `internalDescriptor` → `sealed.fingerprint`, descriptor.ts:356),
   and every chain hash and etag by its `internalBlake3`
   (replica.ts:133-139, store.ts:118). The mirrored pair never
   insulated the wire from engine-version drift; it insulated only the
   grammar around those fields.
2. **The grammar is version-pinned on the wire itself.** `VERSION = 3`
   with refusal of anything else (codec.ts:22, codec.ts:200-203;
   canon: "0.19.0 reads nothing 0.18.0 wrote", commit `9807d94f`), and
   the corpus pins the byte grammar per release. A within-caret codec
   change is already a protocol event, not a patch.

If the caret residue still offends, the mechanical fix is narrowing
the peer to an exact pin — which the lockstep gate makes a one-line
change to the derivation (build.ts:222). Consumers then lose patch
float on the engine; that is the install-matrix pain, and it is small
because the two packages already release as one number in one
ceremony. **Grade: real-but-small; mostly pre-paid by the lockstep.**

---

## 6. What the mirrored pure pair bought — the reconstructed rationale, reason by reason

### The original statement of intent (from history)

- Retired `proposals/20-command-codec.md` (at `48d47a48^`): "One
  grammar, one codec, **two thin drivers** (Rust in `bumbledb-log`, TS
  in `@bjornpagen/bumbledb-log`), **pinned equal by cross-goldens**."
- Retired `proposals/70-typescript.md`: "The mirrored pair must be
  byte-equal with Rust, pinned by the goldens (80) … Both are pure,
  take the descriptor the SDK already lowers, and touch no I/O — **they
  are the protocol; the rest of the package is plumbing around them**."
- Retired `proposals/80-conformance.md` Lane 7: "parity goldens
  (Rust ⇄ TS, the mirrored pair) … Byte-exact both directions;
  refusals carry the same typed identity."
- The canon (retired `proposals/settlement/00-canon.md`, rep 1): "A
  behavioral divergence between the drivers is a conformance failure,
  not a design choice — **neither driver defines arms**"; rep 6: "One
  codec cannot decode one byte string to two values."
- Commit `a99c1470` renamed trio→pair when `footprintOf` died — the
  pair was a maintained deliberate artifact, re-affirmed as recently
  as the shipping README (`ts-log/README.md:7-11`).
- The counter-precedent inside the same design: blake3 and the
  descriptor seal were **not** mirrored — "Blake3 rides the engine
  package's existing native binding … no JS blake3 dependency exists"
  (retired 70-typescript.md), and `internalDescriptor` was landed
  precisely "so Lane B can collapse the descriptor re-derivation onto
  one engine-produced truth" (commit `3679f445`). The design already
  distinguished *commodity pure functions* (one native truth, ride the
  FFI) from *the protocol grammar* (two implementations as proof).

So the reconstructed rationale has exactly four load-bearing reasons:

**R1 — Two independent implementations as a grammar-unambiguity
proof.** The parity lane and the 191-file conformance corpus
(`crates/bumbledb-log/conformance/v3/`, walked by both
`conformance_v3` in Rust and `ts-log/test/conformance-v3.test.ts` +
`parity.test.ts:17-22`) are differential testing: two decoders written
by different hands agreeing on every golden byte and every refusal
identity is evidence the grammar has one reading.

**R2 — The TS codec as the legible spec for TS consumers.** 335 lines
of documented grammar (`codec.ts`) a Node developer can read,
breakpoint, and fuzz without a Rust toolchain.

**R3 — Package thinness / no native surface of its own.** ts-log ships
source-only (`"files": ["src"]`), no build, no platform packages.

**R4 — Independence from the engine's seal path.** The corpus pins the
codec on shapes the engine refuses (`descriptor.ts:387-391`,
`assembleFromSpec`), so the TS codec deliberately does *not* route
through the engine's native descriptor for those fixtures.

### Current validity, after (a) the hard native peer, (b) the lockstep, (c) the v:3 byte-parity law

- **R3 is dead.** It died the day `internalDescriptor` went onto the
  import path of every pure module (§2): the pure pair cannot be
  imported without the `.node`. The package is source-only in *form*
  and native-bound in *fact*. The refactor forfeits nothing here.
- **R4 is a constraint, not a reason to mirror.** The Rust log crate
  itself accepts the corpus shapes (`schema_file.rs`); a shared core
  exposed from *that* crate (not from the engine's seal) preserves R4
  exactly. It only rules out the cheapest implementation (reusing
  `internalDescriptor`) — see §2 item 1.
- **R2 is weakened but real.** Post-lockstep, the TS consumer already
  cannot run the "legible spec" without the binary; but they can still
  *read* it, and reading is most of R2's value. This is the DX cost of
  §3, restated.
- **R1 is the one reason that genuinely survives — in weakened form.**
  The v:3 cutover made byte-parity *the law* (canon rep 4/6; "0.19.0
  reads nothing 0.18.0 wrote"), and the corpus is the law's checked-in
  witness. With one implementation, the corpus degenerates from a
  *cross-check between independent readers* into a *regression pin on
  the single reader*: it still prevents drift over time, but it no
  longer provides evidence that the grammar admits one reading, and
  the TS walk of the corpus becomes self-agreement. The Rust driver
  remains independently exercised (its own conformance walk, the duty
  binary, `lane_b_interop.rs`), so the loss is one witness out of two,
  on the pure grammar only — the IO/protocol-machine conformance lanes
  (crash matrices, multiprocess, interop) keep their full differential
  value because the two *drivers* still exist. This is the single
  honest epistemic price of the refactor, and it should be paid
  knowingly or explicitly re-bought (e.g., keep a non-shipping TS
  decoder as a test-only oracle — which re-imports half the
  maintenance cost, so it must be a choice, not a default).

---

## 7. Essential vs accidental — the precise line

**Accidental duplication** (Brooks's sense: the same mathematical
function represented twice; collapsible in principle):

| TS module | LOC | Rust twin | LOC |
| --- | --- | --- | --- |
| `codec.ts` (encode/decode/verifyChain) | 335 | `codec.rs` | 1016 |
| `manifest.ts` (manifest + checkpoint render/parse, `auditCatalog`) | 209 | `manifest.rs` | 493 |
| `chain.ts` render/parse half (`renderSidecar`/`parseSidecar`) | ~180 of 225 | `sidecar.rs` | 519 |
| `vector.ts` | 173 | `vector.rs` | 282 |
| `braids.ts` + `deriveBraids`/`assembleFromSpec` in `descriptor.ts:589-771` | ~450 | `braids.rs` + `schema_file.rs` | 846 |
| `bytes.ts` (ByteReader/Writer, digest brands) | 304 | (Cursor in codec.rs, inline) | — |
| `value.ts` (tagged read/write) | 336 | (tag arms in codec.rs) | — |

Roughly **~2,000 LOC of TS mirroring ~3,100 LOC of Rust**, plus the
mirror-pinning harness whose *raison d'être* is the mirror:
`parity.test.ts` (Lane 7), `conformance-v3.test.ts` +
`conformance-v3-support.ts` on the TS side, and the corpus-regeneration
discipline. That is the accidental-complexity ledger the refactor
collapses — the TS members delete; the Rust members were staying
regardless (the duty binary and the resident writer are their other
consumers, `crates/bumbledb-log/src/bin/duty.rs`, 973 lines).

**Essential duplication** (two hosts, two runtimes; not collapsible
without inverting the architecture):

- The **replica/writer state machines** as *hosted*: `replica.ts`
  (1107) / `replica.rs` (1268), `writer.ts` (870) / `writer/` (2634).
  The transition table is one (canon rep 1), but each side is woven
  through its host's IO: JS async store verbs and the event loop,
  `AsyncDisposable`, the engine's JS `Db` handle whose `write`
  callback crosses napi synchronously (native.ts:342), `Date.now`
  clamps, `process.pid` store names (replica.ts:151-154) — versus
  tokio, `object_store`, and the duty/Lambda entry on the Rust side.
  Sharing *this* behind FFI would mean native code awaiting JS store
  promises and re-entering JS per fact-lift: the boundary would cross
  the machine's every arrow instead of its leaves. Essential.
- The **five-verb stores**: `store.ts`/`store-s3.ts` (1088) vs
  `store/` (1823) — same on-disk/on-wire protocol, two IO stacks by
  design (raced against each other in the interop lane; the
  duplication *is* the conformance instrument here).
- The **error idioms**: `errors.ts` (280) — sentinel values + WeakMap
  causes is the JS-native representation of what Rust spells as enums;
  a representation, not a duplication.
- The **lift/lower glue**: `factOf` (replica.ts:326-351), `lowerFact`
  (writer.ts:286-319), `headerWriter`'s raw fixed-offset reads
  (writer.ts:401-420) — these translate between the wire's values and
  each host's fact shape; they exist *because* there are two hosts.

**The line, stated precisely:** a function is accidental duplication
iff it is (bytes → values) or (values → bytes) with no store verb, no
engine handle, no clock, no process identity, and no host fact-shape
in its signature — i.e., `encodeBatch`, `decodeBatch`, `verifyChain`,
`renderManifest`/`parseManifest`, `renderCheckpoint`/`parseCheckpoint`,
`renderSidecar`/`parseSidecar`, the `Vector` algebra, and braid
derivation. Everything that touches a Promise, a `Db`, a file
descriptor, `Date.now`, or `process.pid` is essential to its driver
and stays. The descriptor sits *on* the line and has already been cut
correctly once: its seal is native (`internalDescriptor`), its parse
into driver tables is per-host glue (descriptor.ts:264-367).

---

## 8. The ledger

| # | Cost | Grade | Evidence |
| --- | --- | --- | --- |
| 1 | FFI marshal per commit/slot/heartbeat | **Negligible** | frequencies §1; fsync + S3 PUT 20–60 ms floor (`00-product.md:128` retired; loss cost 67.2 ms measured, commit `94c8e122`); rows already cross at `txInsert` (native.ts:346-357) |
| 2 | "ts-log gains a runtime native requirement for pure functions" | **Negligible — already paid** | module-scope load native.ts:439; `descriptor.ts:20` on the codec's import path; no native-free test lane exists; CI builds the `.node` before ts-log (bumbledb-log.yml) |
| 3 | napi crate grows a dep on the log crate → tokio/object_store in the cdylib, or a crate split + roster/lockfile churn | **Real-but-payable** (the largest new work item) | ts/crate/Cargo.toml deps; crates/bumbledb-log/Cargo.toml:8-11 unconditional async deps; `assembleFromSpec` rules out reusing `internalDescriptor` (descriptor.ts:387-391) |
| 4 | Error identity + cause-data fidelity across the boundary | **Real-but-payable** | kind+message channel only (marshal.rs:43-58, native.ts:471-498); sentinel + WeakMap idiom (errors.ts:145-151); identity strings already pinned equal (`DecodeError::identity`, codec.rs:349-370); cause payload shapes differ (names vs offsets) |
| 5 | Stack traces / breakpointability of the codec; codec joins the cargo edit loop | **Real-but-payable** (pure DX) | boundary-minted errors (marshal.rs:54-58); ts-log currently buildless (`"files": ["src"]`); lto=fat napi profile (ts/crate/Cargo.toml:33-35) |
| 6 | Child-process module loading | **Negligible** | children already dlopen (multiprocess-child.ts:17, interop-child.ts:16); processes, not workers |
| 7 | Install-matrix / exact-version pain | **Real-but-small; mostly pre-paid** | lockstep gate derives the peer (build.ts:213-225); one number, one ceremony (`20-one-version.md` retired); wire identity already engine-resolved (fingerprint via `internalDescriptor`) |
| 8 | Loss of the two-implementation grammar witness (R1) | **The one real epistemic cost — payable only knowingly** | Lane 7 + corpus become single-reader regression pins; canon rep 6's strongest test dies; driver-level conformance lanes unaffected |
| 9 | Third speller at the refusal boundary (the shim) unless the bisection is documented | **Real-but-payable** | JS-side refusals that must remain (braidOf replica.ts:161-168, headerWriter writer.ts:411-420, wrapStore errors.ts:234) |

**Blockers found: none.** The refactor's costs are one substantial
build-engineering item (#3), a set of enumerable boundary-fidelity
items (#4, #5, #9), and one epistemic trade (#8) that the canon's own
"neither driver defines arms" law softens but does not erase. The
costs conventionally cited first — FFI overhead, new native runtime
requirement, child-process constraints, version lock-in — are, on this
tree's evidence, respectively negligible, already paid, nonexistent,
and pre-paid by the lockstep. The mirrored pair's original four
reasons are today: one dead (thinness), one a design constraint on
*how* to share rather than *whether* (seal independence), one a DX
preference (legibility), and one a live but weakened correctness
instrument (the differential witness) — which is exactly the decision
the owner has to make on purpose.
