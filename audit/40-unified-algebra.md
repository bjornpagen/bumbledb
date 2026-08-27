# 40 — One unified algebra: divergence audit

Scope: the four write surfaces — engine Rust (`crates/bumbledb`), engine TS (`ts/`),
log Rust (`crates/bumbledb-log`), log TS (`ts-log/`) — audited for where the value
types, outcome sums, and operations diverge, and where one definition could serve all.

Verdict vocabulary:
- **UNIFY** — one definition should exist (per language), all surfaces import it.
- **COMPOSE** — the log should wrap/embed the engine type rather than restate it.
- **ESSENTIAL SPLIT** — the divergence encodes a real semantic difference; keep it, but name it.

Structural fact that frames everything: the dependency arrows already point the right
way. Log Rust imports `bumbledb::{Db, Value, Violations, Admission}`
(`crates/bumbledb-log/src/writer/mod.rs:44`, `crates/bumbledb-log/src/apply.rs:10`);
ts-log imports `Fact`, `Violation`, `ValueTypeSpec`, `MemberRelation` from
`@bjornpagen/bumbledb` (`ts-log/src/writer.ts:19-26`, `ts-log/src/value.ts:10`).
The log *executes through* the engine (`apply.rs:178` runs `db.write`;
`ts-log/src/replica.ts:355` runs `core.db.write`). Every divergence below is therefore
a restatement the log chose, not a boundary it was forced into. There is no shared
kernel across the two languages; parity is pinned only where the conformance corpus
compares identity strings (`crates/bumbledb-log/src/codec.rs:349-371` ↔
`ts-log/src/errors.ts:66-95`).

---

## 1. The write vocabulary — 4-way signature matrix

Cells marked ◆ diverge from the row's majority.

| Verb | Engine Rust `WriteTx` | Engine TS `WriteTx` | Log Rust `Batch` | Log TS `Batch` |
|---|---|---|---|---|
| **driver** | `Db::write(f: FnOnce(&mut WriteTx)->Result<R>) -> Result<Admission<Committed<R>>>` — `api/db/write.rs:99-102`; `write_from(&Witness, f) -> Result<ConditionalWrite<R>>` — `write.rs:115-119` | `write(fn) -> WriteOutcome<Rels, SyncResult<R>>` — `ts/src/db.ts:367`; `writeFrom(witness, fn) -> WriteFromOutcome` — `db.ts:369`. ◆ sync-only callback enforced in type (`SyncResult`, `db.ts:233`) | `Writer::commit(body: FnOnce(&mut Batch)->Result<R>) -> Result<Commit<R>>` — `writer/mod.rs:556-559`; `commit_split -> Result<(R, Vec<BraidOutcome>)>` — `mod.rs:588-591` ◆ tuple | `commit(body: (batch)=>R\|Promise<R>) -> Promise<Commit<Rels,R>>` — `ts-log/src/writer.ts:139`; `commitSplit -> Promise<CommitSplit<Rels,R>>` (named struct `{value, outcomes}`) — `writer.ts:107-110,140` ◆ async body allowed |
| **insert** | typed: `insert(facts: impl IntoIterator<Item=&F>) -> Result<MutationReport>` — `api/db/insert.rs:7-12`; raw: `insert_dyn(RelationId, rows: AsRef<[Value]>) -> Result<MutationReport>` — `api/db/insert_dyn.rs:11-16` | `insert(relation, facts: CollectionWrite<R>) -> MutationReport` — `db.ts:297` | `insert(RelationId, rows: IntoIterator<Item=Box<[Value]>>)` returns `()` ◆, infallible pure record — `writer/batch.rs:34-40` | `insert(relation, facts: Iterable<Fact<Rel>>): void` ◆ — `writer.ts:118`; validates eagerly at record time (`lowerFact` + `checkAgainst`, `writer.ts:286-319`) ◆ |
| **delete** | `delete(facts) -> Result<MutationReport>` — `api/db/delete.rs:6-11`; `delete_dyn` — `delete_dyn.rs:9` | `delete(relation, facts: Iterable<Fact<R>>) -> MutationReport` — `db.ts:299` (◆ engine TS insert takes `CollectionWrite`, delete takes `Iterable` — internal asymmetry) | `delete(...)` returns `()` ◆ — `batch.rs:42-48` | `delete(...): void` ◆ — `writer.ts:119` |
| **reserve** | `reserve<T: Fresh>(count: u64) -> Result<FreshRange<T>>` — `api/db/reserve.rs:22`; `reserve_at(FreshField<S>, count) -> Result<FreshRange<u64>>` — `reserve.rs:29` ◆ typed-newtype lane exists nowhere else | `reserve(relation, field, count: bigint) -> FreshRange` (sum: `empty` \| `{start, endExclusive, count}`) — `db.ts:301,123-137` | `reserve(RelationId, FieldId, count) -> Result<Range<u64>>` ◆ — `batch.rs:56-69` | `reserve(relation, field, count) -> readonly bigint[]` ◆ materialized array — `writer.ts:120-124,193-226` |
| **reserve_capacity** | — (ordinary insert; no sugar) | — | `reserve_capacity(StatementId, parent, units, expiry) -> Result<()>` — `batch.rs:79-105` ◆ | **missing** ◆ — no `reserveCapacity` anywhere in `ts-log/src` |
| **tx reads** | `contains`/`get`/`get_dyn`/`contains_dyn` — `api/db/get.rs:131-194` | `contains`/`get` — `db.ts:303-310` | none ◆ (recording is pure; the body is never re-invoked — `batch.rs:1-3`) | none ◆ |
| **mutation report** | `MutationReport { submitted, changed }` — `api/db/mutation.rs:14-17` | `{ submitted, changed }` frozen — `db.ts:117-121` | `()` — judgment deferred to apply | `void` |
| **empty transaction** | commits the no-op arm; generation unchanged (`CommitReport`, `write.rs:217-226`) | same, plus `abandon(payload)` sentinel to decline explicitly — `db.ts:255-271` ◆ | `Err(Error::EmptyCommit)` ◆ — `writer/mod.rs:231,561-563` | throws `"commit recorded no ops"` ◆ — `writer.ts:814-816` |
| **fact spelling** | both lanes: typed `Fact` refs and positional `[Value]` rows | named objects | positional `Box<[Value]>` rows only ◆ (the `insert_dyn` shape) | named `Fact` objects only ◆ (the typed shape) |

Divergence highlights:

1. **The two log drivers picked opposite engine lanes.** Log Rust `Batch` records
   positional raw rows (`insert_dyn` shape); ts-log `Batch` records named typed facts
   (`insert` shape) and lowers them itself (`writer.ts:286-319`), duplicating the
   engine SDK's own marshalling (`ts/src/marshal.ts:127-129`). The wire is positional
   either way (`codec.rs:205-212` `Op { kind, relation, rows: Vec<Box<[Value]>> }`,
   mirrored at `ts-log/src/codec.ts`). **UNIFY** the recorded-op shape: `Op` is the
   real algebra; both batches should present the same fact spelling as their engine
   SDK and lower through the *engine's* lowering, not a private copy.
2. **Judgment timing.** Engine verbs judge shape at call (`Result`/throw);
   log Rust records infallibly and judges at encode (`EncodeError::Value`,
   `codec.rs:263-269`); ts-log judges at record. Three timings for one law.
   **UNIFY** on record-time refusal (the ts-log choice) or encode-time (Rust choice) —
   but pick one; today the same malformed fact refuses at different phases per surface.
3. **`MutationReport` vs `void`.** The log discards the engine's submitted/changed
   report even though apply eventually produces it. ESSENTIAL SPLIT is defensible
   (recording is pre-judgment), but then engine TS/Rust and log TS/Rust should at
   least agree pairwise — they do.
4. **`reserve` returns four different types** for the same drawn range — see §5.
5. **`reserve_capacity` exists only in log Rust** (`batch.rs:79`). Either port to
   ts-log or delete; a verb in one driver of one language is not an algebra.

---

## 2. Generation arithmetic: `GenerationId` vs `Vector`

- Engine: `GenerationId(u64)` — opaque scalar, `value()`, private `next()` (+1 per
  state-changing commit) — `crates/bumbledb/src/storage/env.rs:62-88`.
- Log Rust: `Vector { counts: BTreeMap<BraidId, u64> }` with `sum` (overflow-refusing),
  `dominates` (pointwise), `order` (total order by sum → `CheckpointOrder`), `at`,
  `advance` (saturating +1), `set`, `encode`/`parse` — `crates/bumbledb-log/src/vector.rs:49-140`.
- Log TS: hand-written mirror, same operations — `ts-log/src/vector.ts:33-170` — plus a
  branded `Generation` bigint (`ts-log/src/keys.ts:52-53`).

**Is the engine's generation a one-braid Vector?** Not the coordinate — the *sum*. The
log's own invariant states it: the engine store generation equals the vector sum (plus
the pending 0|1), and apply enforces `committed.generation >= chain.sum() - position.g + slot`
(`apply.rs:196-205`; identically `ts-log/src/replica.ts:400-406`). A single-braid store
has `sum() == at(braid)`, so yes: `GenerationId` is exactly the image of `Vector::sum`,
and a one-braid `Vector` *is* the engine generation. The embedding is already load-bearing;
it is just undeclared.

Overloading hazard: "generation" currently names **three** coordinates — (a) the engine
store-wide count (`env.rs:59-64`), (b) the per-braid slot number in log outcomes
(`writer/mod.rs:86-88`: "the slot number … never the store-wide sum"), (c) the per-braid
applied count `Vector::at` (`vector.rs:89`). (b) and (c) coincide; (a) is the sum.
Committed<R>.generation (engine, `error.rs:866-869`) and Commit::Accepted.generation
(log) are therefore *different quantities with the same name* — the single most
confusing spelling collision in the codebase.

Cross-language drift inside the log itself:
- `VectorError::TrailingBytes { at }` (offset) vs TS `{ tag: "trailing", bytes }`
  (remaining count) — `vector.rs:29` vs `vector.ts:24,156`.
- Rust `Vector` is mutable in place (`advance(&mut self)`, `vector.rs:94-97`); TS
  `advance` returns a fresh Vector (`vector.ts:109-114`). Both saturate at u64::MAX.

**Verdict: COMPOSE, with one renaming.**
- Keep the engine scalar — the engine has no braids and should not carry a map.
- Declare the embedding: `Vector::sum` should land in `GenerationId` space (Rust:
  return `GenerationId`, importable since bumbledb-log already depends on bumbledb),
  making "store generation = vector sum" a type-level fact instead of a comment.
- Rename the log outcome field from `generation` to `slot` (it already has the right
  name in `Deposition { slot, … }`, `writer/mod.rs:211-216`, and in
  `pending.gen`/`braid_gen`). One quantity per name.
- UNIFY the two log Vector implementations' refusal payloads and mutation style;
  the conformance corpus should pin `encode`/`parse` byte-for-byte (it pins the
  codec's identities already; the vector wire deserves the same).

---

## 3. Outcome sums

### Inventory

Engine Rust (`crates/bumbledb/src/error.rs`):
- `Admission<T> = Accepted(T) | Rejected(Violations)` — `error.rs:819-822`
- `Committed<R> { value, generation }` — `error.rs:866-869`
- `ConditionalWrite<R> = Accepted(Committed<R>) | Rejected(Violations) | Moved { witnessed, current }` — `error.rs:876-883`
- `Check = Holds | Violated(Violation)` — `error.rs:858-861`

Engine TS (`ts/src/db.ts`):
- `Admission = accepted | rejected` — `db.ts:240-242`
- `WriteOutcome = accepted(Committed) | rejected | abandoned` — `db.ts:244-247` (abandoned arm `db.ts:231`)
- `WriteFromOutcome = WriteOutcome | moved { witnessed, current }` — `db.ts:249-251`

Log Rust (`crates/bumbledb-log`):
- `Commit<R> = Accepted { value, braid, generation, durability } | Rejected(Violations)` — `writer/mod.rs:90-98`
- `BraidOutcome = Accepted { braid, generation, durability } | Rejected { braid, violations }` — `mod.rs:102-112`
- `Durability = Published | LocalPending` — `mod.rs:81-84`
- `Applied = Advanced | Absorbed | Rejected(Violations) | Refused(ApplyRefusal)` — `apply.rs:112-124`
- `Published = Replaced | Kept { incumbent } | Refused(PublishRefusal)` — `manifest.rs:402-412`
- `Refreshed = Vector(Vector) | Refused(OpenRefusal)` — `replica.rs:204-209`
- `Waited = Reached(Vector) | Wedged { braid } | Refused(OpenRefusal)` — `replica.rs:213-221`
- `Leased = Drawn { range, token } | Refused(LeaseRefusal)` — `lease.rs:74-77`

Log TS (`ts-log/src`):
- `Commit`, `BraidOutcome`, `CommitSplit`, `Durability` — `writer.ts:86-110` (faithful mirrors)
- `Published`, `PublishRefusal` — `writer.ts:638-643` (arms match Rust; payloads flattened to strings)
- `RefreshOutcome = advanced | wedged | reseed | refused` — `replica.ts:57-61` ◆ (internal only)
- public `refresh() -> Map`, `waitFor() -> void` ◆ — `replica.ts:83-84` — the Rust `Refreshed`/`Waited` sums are **not surfaced**; refusals become thrown errors, `wedged` blocks forever inside `waitFor`'s poll loop rather than returning `Waited::Wedged`.

### Where arms overlap but spell differently

| Semantic | Engine spelling | Log spelling |
|---|---|---|
| theory said yes | `Accepted(T)` / `"accepted"` | `Accepted { … }` / `"accepted"` — same word, restated sum |
| theory said no | `Rejected(Violations)` | `Rejected(Violations)` — payload already the engine's type (`mod.rs:44`), sum restated |
| CAS miss as data | `Moved { witnessed, current }` (`error.rs:879-882`) | internal loss loop; surfaces only as `Error::Contention` after `LOSS_BOUND` (`mod.rs:235-238`, `writer.ts:79,489-557`) |
| decline-to-commit with payload | TS `abandoned` arm only (`db.ts:231,244-247`) — **no Rust arm exists** | nothing (empty commit is an error) |
| no-op absorption | `CommitReport::Changed`/unchanged internal (`write.rs:217`) | first-class: `Applied::Absorbed` (`apply.rs:119-121`) |

### Can the log outcomes EMBED the engine outcomes?

Yes, and the Rust log is one constructor away from it. `Commit<R>` is precisely
`Admission<Slotted<R>>` for `Slotted<R> { value, braid, slot, durability }` — the
`Rejected` payload is already `bumbledb::Violations`, only the two-arm sum shell is
restated. Same for `BraidOutcome` = `Admission<Slotted<()>>` tagged with braid on the
rejected arm. Embedding buys: the engine's `unwrap`/`expect`/`map` combinators
(`error.rs:824-851`) for free, and one place where "accepted/rejected" is defined.
Note what the log's accepted arm must NOT embed: engine `Committed<R>` — its
`generation` is the store-wide sum, while the log reports the braid slot (§2). The
composition is `Admission` around a log-owned payload, not `Committed` inside a log arm.

TS side: ts-log already imports `Violation` from the engine SDK but redeclares the
`{ tag: "accepted" } | { tag: "rejected", violations }` shell (`writer.ts:88-96` vs
`ts/src/db.ts:240-242`). The engine's `Admission<Rels, T>` type is exported
(`ts/src/db.ts:1669`); ts-log should extend it, not restate it.

**Verdicts:**
- `Commit`/`BraidOutcome`: **COMPOSE** — `Admission<{value, braid, slot, durability}>`
  in both languages, engine-owned `Admission` shell, log-owned payload.
- `Moved` vs internal loss loop: **ESSENTIAL SPLIT** — the engine deliberately ships
  the outcome and never loops (`write.rs:112`); the log deliberately loops and ships
  only the bound's exhaustion. Opposite retry philosophies, both documented. Keep,
  but the *carried data* should align: `ContentionCause::HotKey` carries
  `StatementId + Box<[Value]>` in Rust (`mod.rs:196-204`) but `canonical: string +
  named fact objects` in TS (`errors.ts:106-112`, filled from the engine violation at
  `writer.ts:443-472`) — pick one payload spelling.
- `abandoned`: **UNIFY** — either add the arm to Rust `Db::write` (today a Rust host
  cannot decline-with-payload without minting an `Err`) or remove the TS sugar. An
  arm that exists in one language is a portability trap for hosts.
- `Refreshed`/`Waited`: **UNIFY within the log** — ts-log should surface the same
  outcome sums Rust does (`Waited::Wedged` in particular; today `waitFor` on a wedged
  braid polls forever, `replica.ts:1027-1041`, where Rust returns `Waited::Wedged`,
  `replica.rs:410-424`).
- `Applied::Absorbed`: fine as log-only (crash-window semantics the engine cannot see).

---

## 4. Refusal / error identities

Engine Rust: one workspace `Error` enum + `ErrorFamily` table
(`crates/bumbledb/src/error.rs:1260-1374, 1382-1407`). Engine TS: sentinel identities
(`ErrSpentHandle`, `ErrForeignWitness`, `ErrForeignPrepared`, `ErrAsyncCallback`,
`ErrUseAfterScope` — `ts/src/db.ts:712-720`; `ErrSchemaError`, `ErrFingerprintMismatch`,
`ErrIrError`, `ErrNewtypeMismatch` — `db.ts:1316-1321`) plus bridged native families.

Log Rust: `writer::Error` (`Fault | Refused(OpenRefusal) | SpanningCommit | EmptyCommit |
Encode | Contention | Wedged | Lease(LeaseRefusal) | ReservationShape | InjectedCrash |
Drain` — `writer/mod.rs:221-256`); `DecodeError` with pinned cross-impl `identity()`
strings (`codec.rs:279-371`); `VectorError::identity` (`vector.rs:34-47`);
`ApplyRefusal`/`ChainCause` (`apply.rs:64-104`). Log TS: sentinels
`ErrRefused / ErrSpanningCommit / ErrGapDetected / ErrReplayDiverged / ErrChainMismatch /
ErrContention / ErrManifestMissing / ErrAmbiguous / ErrOverWidth / ErrExhausted /
ErrSlotRetired / ErrStore` with WeakMap payloads (`ts-log/src/errors.ts:22-63,145-151`),
whose `RefusalCause.kind` strings are deliberately the Rust `DecodeError::identity`
names (`errors.ts:66-95`).

**One namespace or two?** Two, justified — with a shared floor:
- The engine's refusals judge *facts against a theory* (schema, validation, capacity,
  fresh, LMDB). The log's refusals judge *protocol objects* (batch bytes, chain,
  manifest, lease, slot). These are different courts; merging the enums would couple
  the engine to S3 vocabulary. ESSENTIAL SPLIT on the namespaces.
- BUT three identities currently straddle the boundary and are the same fact spelled
  differently:
  1. **Id-space exhaustion**: engine `Error::FreshExhausted { relation, field }`
     (`error.rs:1299-1302`, minted `storage/delta/alloc.rs:22`) vs log
     `LeaseRefusal::Exhausted { relation, field }` (`lease.rs:60-63`) vs ts-log
     `ErrExhausted` + `{relation, field}` data (`errors.ts:57,130-133`). Same
     conviction, three names. **UNIFY** the identity (one name, one payload shape).
  2. **Foreign witness/handle**: `Error::ForeignWitness` (`error.rs:1324`) ↔
     `ErrForeignWitness` (`db.ts:720`) — already aligned; keep pinned.
  3. **Decode identities**: already one namespace by convention
     (`DecodeError::identity` ↔ `RefusalCause.kind`, compared "string for string" by
     the conformance corpus — `errors.ts:66-69`). This is the model: **the shared
     namespace should be a generated table, not a convention** — one source of
     identity strings emitted to both languages, so a new arm cannot land in one
     driver only. ts-log's tail kinds (`ManifestVersion`, `CheckpointBraids`,
     `NoOpSlot`, … `errors.ts:92-95`) already extend the table unilaterally — exactly
     the drift a generated table prevents (Rust spells these as separate enums:
     `ManifestError`, `CheckpointError`, `ApplyRefusal::PublishLawViolation`).
- `ErrAmbiguous` (unproved conditional write, `errors.ts:51`) and `Deposition` have
  Rust analogs in the store layer / `Deposition` struct (`mod.rs:211-216`) — parity
  exists but is unpinned.

---

## 5. The reserve / id-lease algebra

| | Engine (both langs) | Log Rust | Log TS |
|---|---|---|---|
| substrate | LMDB meta sequence inside the write txn | object-store CAS counter, `LEASE_WIDTH = 4096` blocks, fencing token (`lease.rs:74-77,122-160`) | same protocol, `LEASE_WIDTH = 4096n` (`writer.ts:75`) |
| outcome | `FreshRange<T> = Empty \| NonEmpty { start, count }` (`mutation.rs:75-79`); TS sum `empty \| {start, endExclusive, count}` (`db.ts:123-137`) | `Leased = Drawn { range: Range<u64>, token } \| Refused` (`lease.rs:74-77`); `Batch::reserve` flattens to `Range<u64>` (`batch.rs:56-69`) | `readonly bigint[]` (`writer.ts:120-124`) |
| refusals | `FreshExhausted{relation,field}` on u64 overflow (`alloc.rs:22`) | `OverWidth{requested}` (draw > width) \| `Exhausted{relation,field}` (u64) \| `Counter` (malformed body) (`lease.rs:53-67`) | `ErrOverWidth` \| `ErrExhausted` (`errors.ts:54-57`) |
| abort semantics | escaped ids burn on abort/panic (`EscapedIdBurn`, `write.rs:21-54`; Lean `never_reissue_observable`) | drawn ids burn by construction (counter is monotone; reservations never enter the log — `batch.rs:50-53`) | same (`writer.ts:113-115`) |

Same algebra? **Yes at the interface, no at the substrate.** Both are "draw a half-open
range from a per-(relation, field) monotone counter; never reissue; refuse at u64".
The token and the width are log-protocol facts (fencing against deposed writers,
CAS amortization) with no engine analog — ESSENTIAL SPLIT below the interface.

But the interface itself has three real divergences to fix:

1. **Outcome shape**: four spellings of "a drawn range". ts-log's `bigint[]`
   materializes O(count) ids and is the odd one out even within ts-log's own protocol
   (`LeaseRange {next, end}` internally, `writer.ts:143-146`). **UNIFY** on the
   engine's `FreshRange` value (per language) for all four `reserve` verbs.
2. **Semantic drift in `Exhausted`**: log Rust `draw` leases a fresh block
   mid-transaction on a cache miss (`lease.rs:220-262` falls through to
   `lease_block`); ts-log `drawIds` can only serve from the pre-fetched pool and
   spells a *cache miss* as `ErrExhausted` ("cannot draw … from the cached block",
   `writer.ts:205-209`) because the recorder is synchronous and cannot await a CAS.
   Same identity, two meanings — a Rust host sees `Exhausted` only at the true end of
   the id space; a TS host sees it on a hot draw burst. **UNIFY the meaning** (either
   a distinct `Starved`/retryable identity in TS, or pre-lease sizing that makes the
   miss unrepresentable).
3. **`reserve(0)`**: engine returns `Empty` without touching the sequence
   (`mutation.rs:70-73`); log Rust returns `Drawn { range: 0..0 }` (`lease.rs:233-238`)
   — a degenerate range the engine's algebra deliberately made unrepresentable;
   ts-log returns `[]`. Adopt the engine's `Empty`-is-absence ruling everywhere.

---

## 6. The interval / value algebra

**Rust is already unified.** One `Value` sum lives in the zero-dep theory crate
(`crates/bumbledb-theory/src/value.rs:8-24`), with the checked half-open
`Interval<T>` (`start < end` by construction, empty unrepresentable —
`crates/bumbledb-theory/src/interval.rs:40-98`), and one `ValueType`
(`crates/bumbledb-theory/src/schema.rs:40-60`). Engine re-exports it
(`crates/bumbledb/src/value.rs:13`); log Rust imports it directly
(`batch.rs:9`, `writer/mod.rs:44`). This is the model the owner wants — it already
exists in one language.

**TS has two of everything:**
- Two value sums, structurally identical: engine
  `FactValue = boolean | bigint | string | Uint8Array | IntervalValue`
  (`ts/src/native.ts:34`) vs ts-log
  `Value = boolean | bigint | string | Uint8Array | Interval`
  (`ts-log/src/value.ts:15-20`).
- Two interval types, both `{start, end}` bigints: `IntervalValue`
  (`ts/src/fields.ts:16-18`) vs `Interval` (`ts-log/src/value.ts:15-18`).
- Two enforcement sites for `start < end`: `span()` (`fields.ts:28-31`) vs
  `checkAgainst` (`value.ts:122-141`) + `readTagged`'s `emptyInterval`/
  `intervalOverflow` refusals (`value.ts:240-255`). In Rust the invariant has one
  owner (`Interval::new`); in TS it has two, plus a third re-statement in the decoder.
- ts-log already imports `ValueTypeSpec` from the engine (`value.ts:10`) — the type
  language is shared; only the value language was forked.

Type-spelling drift across languages (semantics agree, spelling doesn't):
- Rust `ValueType` splits `Interval { element }` / `FixedInterval { element, width }`
  into two variants (`schema.rs:50-59`); TS folds them into one
  `{ kind: "interval", element, width: bigint | undefined }` (`ts/src/spec.ts:9-13`).
  The wire agrees (two tags: `TAG.interval`=5 / `TAG.fixedInterval`=6,
  `ts-log/src/value.ts:58-66` ↔ `codec.rs:180-185`), so this is purely a type-level
  fork; pick one shape (the two-variant form matches the wire and Lean).
- `Value::IntervalU64`/`IntervalI64` are two Rust variants; TS has one structural
  interval — acceptable, but note the TS side cannot distinguish element domain
  without the field type, which is why `checkAgainst` needs the spec at every site.

**Verdict: UNIFY (TS).** Export `FactValue`, the interval value type, and one
`checkAgainst`-style judge from `@bjornpagen/bumbledb`; delete ts-log's parallel
definitions, keeping only the codecs (`writeTagged`/`readTagged`/
`writeCanonicalLiteral`) in ts-log — encodings are log-protocol, values are not.
The canonical big-endian literal encoder in ts-log (`value.ts:278-323`) mirrors the
engine's Rust `encode_literal`; that mirror is a conformance artifact and should be
pinned by the corpus, not by prose.

---

## Ranked shortlist — highest-leverage unifications

1. **One TS value/interval vocabulary** (§6). Mechanical, zero semantic risk, kills a
   whole duplicated file (`ts-log/src/value.ts` types + checks vs `ts/src/fields.ts`,
   `ts/src/native.ts:34`) and the double-owned `start < end` invariant. Home: the
   engine TS SDK; ts-log keeps only codecs.
2. **`Commit`/`BraidOutcome` as `Admission` composition** (§3). Rust:
   `Admission<Slotted<R>>`; TS: extend the exported engine `Admission`. Payloads are
   already engine types (`Violations`); only the sum shell is restated. Rename the log
   outcome's `generation` field to `slot` while touching it (§2's three-way name
   collision).
3. **One id-lease refusal identity and one drawn-range value** (§5). Merge
   `FreshExhausted` / `LeaseRefusal::Exhausted` / `ErrExhausted` into one identity with
   one `{relation, field}` payload; adopt engine `FreshRange` as the reserve result on
   all four surfaces; fix ts-log's `Exhausted`-on-cache-miss semantic drift (a
   correctness-adjacent bug, not just spelling).
4. **Surface `Waited`/`Refreshed` sums in ts-log** (§3). Today a wedged braid means
   Rust hosts get `Waited::Wedged` and TS hosts get an infinite poll loop
   (`replica.ts:1027-1041`). This is the one divergence with a liveness consequence.
5. **Generated cross-language identity table** (§4). The `DecodeError::identity` ↔
   `RefusalCause.kind` convention is the right idea enforced by discipline; generate
   both sides from one table (theory crate or a schema file the conformance corpus
   owns), and fold `VectorError`, `ChainCause`, lease refusals, and the ts-log-only
   tail kinds (`ManifestVersion`, `NoOpSlot`, …) into it.
6. **Write-verb parity sweep** (§1): port `reserve_capacity` to ts-log (or retire it);
   align `commit_split` return shape (tuple vs struct); one fact-spelling per log
   driver matched to its engine SDK, lowering through the engine; one judgment timing;
   engine TS `insert`/`delete` argument-type asymmetry (`CollectionWrite` vs
   `Iterable`).
7. **The `abandoned` arm** (§3): add to Rust `Db::write` or retire from TS. Smallest
   item, but it is the only *engine-internal* cross-language sum divergence, and the
   sums are the contract everything else composes over.

Essential splits to leave alone (and document as rulings): the log's internal loss
loop vs the engine's `Moved`-as-data (opposite retry philosophies, both deliberate);
the lease width/fencing token (log-protocol facts); the two error namespaces above the
shared identity floor; `Applied::Absorbed` (crash-window semantics invisible to the
engine); the engine keeping a scalar `GenerationId` rather than carrying the log's
`Vector` (compose via `sum`, never merge).
