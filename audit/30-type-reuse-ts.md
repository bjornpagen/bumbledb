# Type-reuse audit: ts-log vs the engine SDK

Mission: every place `@bjornpagen/bumbledb-log` (ts-log/) re-declares, near-duplicates,
or gratuitously diverges from a type the engine SDK (`@bjornpagen/bumbledb`, ts/)
already exports or could export. Three categories:

- **(a) IMPORT INSTEAD** — the engine already exports it verbatim.
- **(b) ALIGN THEN IMPORT** — the engine has it but signatures drifted; the exact delta is listed.
- **(c) GENUINELY LOG-SPECIFIC** — no engine counterpart; justified.

All paths absolute from the repo root `/Users/bjorn/Documents/bumbledb/`. Line numbers
are against the current tree (HEAD `c5947e3c`).

---

## Summary table

| # | ts-log declaration | engine counterpart | category | deletable TS |
|---|---|---|---|---|
| 1 | `Interval` (ts-log/src/value.ts:15-18) | `IntervalValue` (ts/src/index.ts:115, from ts/src/fields.ts:16-19) | (a) | 4 |
| 2 | `Value` (ts-log/src/value.ts:20) | `FactValue` (ts/src/index.ts:126, from ts/src/native.ts:34) | (a) | 1 |
| 3 | `isInterval` (ts-log/src/value.ts:68-70) | `isIntervalValue` (ts/src/fields.ts:188-197, **not in index.ts**) | (b) | 3 |
| 4 | `SideInfo` (ts-log/src/descriptor.ts:44-49) | `SealedSide` (ts/src/index.ts:131, ts/src/native.ts:164-168) | (a) | 6 |
| 5 | `WeightInfo` (ts-log/src/descriptor.ts:51-54) | `SealedWeight` (ts/src/index.ts:133, ts/src/native.ts:170-173) | (a) | 4 |
| 6 | `HiInfo` (ts-log/src/descriptor.ts:56-60) | `SealedHi` (ts/src/index.ts:130, ts/src/native.ts:175-179) | (a) | 5 |
| 7 | `StatementInfo` (ts-log/src/descriptor.ts:62-78) | `SealedStatement` (ts/src/index.ts:132, ts/src/native.ts:181-202) | (a) | 17 |
| 8 | `asValue`/`sideOf`/`statementOf` (ts-log/src/descriptor.ts:217-262) | identity conversions once #2 lands | (a) | ~46 |
| 9 | `FieldInfo.type` indexed-access spelling (ts-log/src/descriptor.ts:28) | `ValueTypeSpec` (ts/src/index.ts:211) | (a) | 1 (clarity) |
| 10 | `Batch` (ts-log/src/writer.ts:117-125) | `WriteTx` (ts/src/index.ts:69, ts/src/db.ts:296-311) | (b) | 8 |
| 11 | spec re-join in `fromSealed` (ts-log/src/descriptor.ts:83-105, 265-307) | `ManifestField` lacks `fresh`/`newtype` (ts/src/native.ts:134-138) | (b) | ~50 |
| 12 | `factOf` (ts-log/src/replica.ts:325-351) | `factOf` (ts/src/marshal.ts:175-197, **not in index.ts**) | (b) | 27 |
| 13 | `lowerFact` (ts-log/src/writer.ts:286-319) | `rowOf`/`cellOf` (ts/src/marshal.ts:87-142, **not in index.ts**) | (b) | 34 |
| 14 | test corpus loader duplicated (ts-log/test/parity.test.ts:26-249 vs ts-log/test/conformance-v3-support.ts:17-227) | intra-package (both already import engine spec types) | (b) | ~225 |
| 15 | codec machinery, braids, keys, errors, store, vector, chain, manifest, tenants | none | (c) | 0 |
| 16 | `assembleFromSpec` family (ts-log/src/descriptor.ts:392-706) | shadow of the engine seal, test-only consumers | (c)* | ~315 movable out of src |

Total deletable from ts-log/src: **~205 lines** ((a) ~85 + (b) ~120), plus ~225 lines of
test dedupe and ~315 lines movable from shipped src to test support.

---

## 1. `Value` / `Interval` (ts-log/src/value.ts) — category (a), the headline

ts-log/src/value.ts:15-20:

```ts
interface Interval {
	readonly start: bigint
	readonly end: bigint
}
type Value = boolean | bigint | string | Uint8Array | Interval
```

The engine already exports both, byte-for-byte:

- `IntervalValue` — ts/src/fields.ts:16-19 (`{ readonly start: bigint; readonly end: bigint }`),
  exported at ts/src/index.ts:115. (ts/src/native.ts:29-32 re-declares the same shape
  internally; `FactValue` references that one — structurally identical, so the public
  union is closed over the public `IntervalValue`.)
- `FactValue` — ts/src/native.ts:34 (`boolean | bigint | string | Uint8Array | IntervalValue`),
  exported at ts/src/index.ts:126.

**Verdict: re-implementation, import instead.** ts-log/src/value.ts:10 already imports
`ValueTypeSpec` from the engine on the same line-of-sight; the two value types should come
from the same place:

```ts
import type { FactValue as Value, IntervalValue as Interval } from "@bjornpagen/bumbledb"
```

(or drop the aliases and rename throughout). ts-log's public re-export at
ts-log/src/index.ts:49 (`export type { Interval, Value } from "#value.ts"`) then becomes a
re-export of the engine types under the log's names — the "EXACT same type-heavy payloads"
goal literally: `Op.rows` (ts-log/src/codec.ts:26-30), `RelationInfo.rows`
(ts-log/src/descriptor.ts:42), `lowerFact`'s output (ts-log/src/writer.ts:286-319) and
`applyOps`'s input (ts-log/src/replica.ts:353-374) all become engine-`FactValue`-typed,
which is what the native bridge takes on the other side (ts/src/native.ts:358-366
`txInsert(..., cells: readonly FactValue[])`).

Ripple: 8 files import from `#value.ts` (src/codec.ts:18, src/replica.ts:41,
src/writer.ts:71, src/descriptor.ts:24, src/index.ts:49, test/parity.test.ts:15,
test/conformance-v3.test.ts:26) — all mechanical renames or untouched if the alias
spelling above is used. Deletable: 5-7 lines, but it unlocks items 4-8 and 12-13 below.

Also in value.ts:

- `isInterval` (ts-log/src/value.ts:68-70) duplicates the engine's `isIntervalValue`
  (ts/src/fields.ts:188-197) with a narrower input type (`Value` vs `unknown`). The engine
  one is module-exported but **not** re-exported from ts/src/index.ts. **(b): export
  `isIntervalValue` from the engine index, delete `isInterval`** (3 lines; the engine's
  `unknown`-guard subsumes the narrower one).
- `wellFormedUtf8`/`WellFormedUtf8`, the `TAG` table, `writeTagged`/`readTagged`,
  `TaggedRefusal`, `valuesEqual`, `writeCanonicalLiteral` (value.ts:22-66, 145-323,
  ~290 lines): the wire codec pinned byte-for-byte against the Rust driver's conformance
  goldens. The engine SDK has no TS encoder at all (values cross to native as JS values,
  encoding happens in Rust). **Genuinely log-specific — see section 6.**
- `checkAgainst` (value.ts:89-143) is the one borderline: it re-encodes the engine's typed
  write judgment (u64/i64 range, fixedBytes width, interval domain/width) driven by the
  engine's own `ValueTypeSpec`. The engine SDK deliberately does **not** perform these
  range checks in TS — `cellOf` (ts/src/marshal.ts:87-132) checks shape only and defers
  ranges to the native engine at commit. The codec cannot defer (it must refuse before
  encoding bytes). Keep in ts-log, but it is the natural candidate if the engine ever
  exports a `ValueTypeSpec`-driven validator. **(c) with a note.**

## 2. `Batch` vs `WriteTx` — category (b), the write vocabulary

ts-log/src/writer.ts:117-125:

```ts
interface Batch<Rels extends SchemaRelations> {
	insert<Rel extends MemberRelation<Rels>>(relation: Rel, facts: Iterable<Fact<Rel>>): void
	delete<Rel extends MemberRelation<Rels>>(relation: Rel, facts: Iterable<Fact<Rel>>): void
	reserve<Rel extends MemberRelation<Rels>>(relation: Rel, field: FreshKeys<Rel> & string, count: bigint): readonly bigint[]
}
```

ts/src/db.ts:296-311 (`WriteTx`, exported ts/src/index.ts:69):

```ts
interface WriteTx<Rels extends SchemaRelations> {
	insert<R extends MemberRelation<Rels>>(relation: R, facts: CollectionWrite<R>): MutationReport
	delete<R extends MemberRelation<Rels>>(relation: R, facts: Iterable<Fact<R>>): MutationReport
	reserve<R extends MemberRelation<Rels>>(relation: R, field: FreshKeys<R> & string, count: bigint): FreshRange
	contains<R extends MemberRelation<Rels>>(relation: R, fact: Fact<R>): boolean
	get(...): Fact<R> | undefined   // two overloads, db.ts:305-310
}
```

Note ts-log already imports `Fact`, `FreshKeys`, `MemberRelation`, `Violation`,
`SchemaRelations` from the engine (ts-log/src/writer.ts:19-26) — the generics vocabulary
is already shared. The full signature delta:

| member | Batch | WriteTx | judgment |
|---|---|---|---|
| `insert` param 2 | `Iterable<Fact<Rel>>` | `CollectionWrite<R>` = `Iterable<Fact<R>>` (ts/src/db.ts:76; alias not exported) | **identical** after alias expansion — no delta |
| `insert` return | `void` | `MutationReport` (`{ submitted: bigint; changed: bigint }`, ts/src/db.ts:71-74, exported index.ts:59) | **essential-leaning**: the recorder (writer.ts:336-346) journals ops without applying; `submitted` is knowable (row count) but `changed` is a post-apply fact the journaled dialect cannot know before `applyOps` runs in `disciplineCommit` (writer.ts:562-597, apply happens after the body completes and the pending batch is fsynced). Returning `{submitted, changed: submitted}` would lie. See "goal shape" below — `void` is also exactly what TS's void-return assignability needs. |
| `delete` return | `void` | `MutationReport` | same as insert |
| `reserve` return | `readonly bigint[]` | `FreshRange` (ts/src/db.ts:123-137, exported index.ts:55) | **GRATUITOUS — align.** `drawIds` (writer.ts:193-226) only ever draws from the head lease block (`pool[0]`, refusing if it cannot cover the whole draw, writer.ts:204-209), so the drawn ids are always one contiguous run `[next, next+count)`. That is precisely `FreshRange` (`{ empty, start, endExclusive, count, at(), [Symbol.iterator] }`). Nothing about the journaled dialect requires an eager array. |
| `contains` / `get` | absent | present (db.ts:303-310) | **essential absence.** The recorder is pure ("Pure and synchronous", writer.ts:112-116): nothing is applied at record time, so a point read against the would-be state has no substrate. This matches the stated goal shape — Batch = WriteTx minus the read methods. |

### The goal shape, precisely

The blocker is exactly one member. Under TS structural rules, a method returning `T` is
assignable where `void` is expected, and method parameters are bivariant — so
`WriteTx<Rels>` is **already assignable to `Batch<Rels>` for `insert` and `delete`**. The
only member that fails is `reserve`: `FreshRange` is not assignable to
`readonly bigint[]` (no `length`, no index signature; it has `count`/`at()` instead).

Two ways to close it, in order of preference:

1. **Minimal (Batch as supertype)** — change `Batch.reserve` to return `FreshRange`
   (import the type from the engine; index.ts:55). Then `WriteTx<Rels>` is a structural
   subtype of `Batch<Rels>`, and any write-only body typed `(tx: Batch<Rels>) => R`
   typechecks against BOTH a log commit and an engine `db.write`. One line changed in the
   interface; the implementation needs a `FreshRange` constructor. The engine's
   `freshRangeOf` (ts/src/db.ts:139-172, 34 lines) is exactly that constructor but is
   **not exported** — export it (its wire input `WireFreshRange`, ts/src/native.ts:25-27,
   is a trivial `{ empty } | { empty, start, endExclusive }` the recorder can spell
   inline). Callers of the current array (grep shows commit bodies in tests,
   ts-log/test/writer.test.ts, replica-writer.test.ts) switch from indexing to
   `range.at(0n)` / spread.
2. **Literal (Batch as subtype, the brief's exact wording)** — define
   `type Batch<Rels> = Pick<WriteTx<Rels>, "insert" | "delete" | "reserve">`
   (1 line replacing 9). This additionally requires `insert`/`delete` to return
   `MutationReport`. The honest option is to have the recorder return
   `{ submitted: n, changed: n }` **documented as the pre-judgment echo** (the log's
   verdict arrives as the `Commit`/`BraidOutcome` value, writer.ts:88-110, which already
   reuses the engine's `Violation` — writer.ts:96) — or to accept option 1 and not claim
   subtype-hood. Option 1 achieves the stated user-facing property ("write-only bodies
   typecheck against both") without inventing a `changed` number.

Deletable either way: the 9-line interface collapses to a `Pick` one-liner (option 2) or
keeps 3 method heads with engine-imported return types (option 1). `MutationReport`,
`FreshRange`, `MemberRelation` all come from the engine's existing exports.

## 3. `descriptor.ts` vs `internalDescriptor` — categories (a) and (b)

ts-log/src/descriptor.ts (785 lines) has two paths. The theory path (`fromSealed`,
descriptor.ts:264-367) already consumes the engine's sealed truth via
`internalDescriptor(spec)` (descriptor.ts:265; engine export ts/src/index.ts:136,
ts/src/native.ts:465-469) — good. But it then re-declares the sealed statement vocabulary
and re-joins the sealed output against the input spec to recover facts the sealed output
dropped.

### 3a. Byte-for-byte re-declarations — (a) IMPORT INSTEAD

Once `Value := FactValue` (item 1), these four are the engine's exported types verbatim:

| ts-log | engine | delta |
|---|---|---|
| `SideInfo` (descriptor.ts:44-49) | `SealedSide` (native.ts:164-168, index.ts:131) | `values: readonly Value[]` vs `readonly FactValue[]` — none after item 1 |
| `WeightInfo` (descriptor.ts:51-54) | `SealedWeight` (native.ts:170-173, index.ts:133) | none, including the `"duration"` arm spelling |
| `HiInfo` (descriptor.ts:56-60) | `SealedHi` (native.ts:175-179, index.ts:130) | none |
| `StatementInfo` (descriptor.ts:62-78) | `SealedStatement` (native.ts:181-202, index.ts:132) | none — same three arms, same field names and order |

With those imported, the converters become identity functions and delete:

- `asValue` (descriptor.ts:217-228, 12 lines) — runtime re-validation of what the typed
  napi bridge already promises (`ManifestRow.values: FactValue`); the engine SDK itself
  trusts the bridge (ts/src/db.ts consumes `Manifest` untranslated). Drop.
- `sideOf` (descriptor.ts:230-238, 9 lines) — identity.
- `statementOf` (descriptor.ts:240-262, 23 lines) — identity (its `"functionality"` arm is
  already `return statement`, line 243). `Descriptor.statements` becomes
  `sealed.statements` verbatim.

~46 lines of conversion plus 32 lines of declarations ≈ **78 lines deletable**, and the
`Descriptor` type (descriptor.ts:172-183) starts carrying the engine's own
`readonly SealedStatement[]`.

Also: `FieldInfo.type` is spelled
`SchemaSpec["relations"][number]["fields"][number]["valueType"]` (descriptor.ts:28) —
that is just `ValueTypeSpec` (exported, index.ts:211; also `FieldSpec["valueType"]`,
index.ts:202). One-line clarity fix.

### 3b. The spec re-join — (b) ALIGN THEN IMPORT

`fromSealed` cannot build `FieldInfo` from the sealed output alone because the engine's
`ManifestField` (ts/src/native.ts:134-138) carries only `{ name, id, valueType }` — it
drops `fresh` and `newtype`, which the input `FieldSpec` (ts/src/spec.ts:53-58) had and
the Rust sealer certainly knows. So descriptor.ts re-joins:

- `specByName` index + duplicate check (descriptor.ts:266-272);
- `closedOwners` set (descriptor.ts:273-281);
- per-field spec lookup, `fresh` recovery, and `closedRef` derivation by re-parsing the
  newtype label with `ID_CLASS` = `^(.*)\.id$` (descriptor.ts:83-84, 97-105, 289-307).

**Alignment:** extend the napi descriptor's `ManifestField` with
`readonly fresh: boolean` and `readonly newtype: string | undefined` (or directly
`closedRef: string | undefined` — the `{name}.id` convention is already the engine's own,
see ts/src/spec.ts:65-79). Then `FieldInfo` is a pure projection of the sealed relation
and **~50 lines delete** (the join at 266-281 and 289-307, plus `ID_CLASS`/`idClassOwner`
at 83-105). `lower(theory)` is still called to produce the spec `internalDescriptor`
takes — only the join disappears.

What legitimately remains in the theory path:

- `handles` and positional `rows` from the sealed extension (descriptor.ts:308-341): a
  re-shaping (named `ManifestRow.values` → sealed-order `Value[]`), not a re-parse; the
  codec wants positional rows. Keep (or the engine could offer positional extension rows —
  marginal).
- Braid derivation, `serialAtOf`, `braidHex`, `withFingerprint`, the cache
  (descriptor.ts:107-170, 196-214, 374-385, 708-771): the replication protocol's own
  derivation over the sealed statements. **No engine counterpart — (c).**
- Exporting `ManifestRelation`/`ManifestRow`/`ManifestField`/`Manifest` by name from
  ts/src/index.ts (today reachable only as `SealedDescriptor["relations"][number]`) would
  let ts-log spell `RelationInfo` as `ManifestRelation & { derived... }`. Minor (b).

### 3c. `assembleFromSpec` — the shadow sealer, (c) with an asterisk

descriptor.ts:392-706 (~315 lines: `rawValue`, `SpecTables`, `zipClosedPayload`,
`fieldsOf`, `resolveLiteral`, `literalSetOf`, `specSideOf`, `boundValue`, `capacityOf`,
`assembleFromSpec`, plus `deriveBraids` shared with the theory path) is a full TS
re-implementation of the engine's seal: declaration-order relation ids, the synthetic
closed `id` field at ordinal 0, fresh-key and closed-key functionality statements,
fd/containment/capacity materialization, and literal/handle resolution.

Its only consumers are tests: ts-log/test/parity.test.ts:12,299 and
ts-log/test/conformance-v3-support.ts:13,238. Per its own comment (descriptor.ts:387-391)
it exists for conformance-corpus spec shapes **the engine seal refuses** — so it cannot
simply call `internalDescriptor`. Judgment: genuinely log-specific in purpose, but it is
the single largest mass of engine re-implementation in the package and it ships in the
published `src`. Recommendation: move it (and `rawValue`, which duplicates
test/parity.test.ts:96-110) into ts-log/test/ support; any corpus schema the engine CAN
seal should assemble through `internalDescriptor` so drift is impossible, leaving the
shadow sealer only for deliberately-illegal shapes. **~315 lines out of shipped src.**

## 4. The marshal twins — (b) ALIGN THEN IMPORT

Both halves of the engine's fact⇄row bijection are re-implemented in ts-log because the
engine keeps ts/src/marshal.ts module-private (ts/src/index.ts:124 exports only the
`KeyFact` type from it):

- **Read side:** `factOf` (ts-log/src/replica.ts:325-351, 27 lines) — positional row →
  named fact with closed-handle lifting — duplicates the engine's `factOf`
  (ts/src/marshal.ts:175-197), including the out-of-roster refusal (marshal.ts:74-85
  `handleOf`). `applyOps` (replica.ts:353-374) already holds the `AnyRelation` member
  (line 359) that the engine `factOf` needs; using it also deletes the double cast
  `as unknown as Iterable<Fact<MemberRelation<Rels>>>` at replica.ts:363-365, because the
  engine `factOf` returns `Fact<R>` typed.
- **Write side:** `lowerFact` (ts-log/src/writer.ts:286-319, 34 lines) — named fact →
  positional row with handle lowering — duplicates `rowOf`+`cellOf`
  (ts/src/marshal.ts:87-142), down to the missing-field message
  ("fact is missing field", writer.ts:294 vs marshal.ts:138). Field order is safe:
  `lowerFact` is only reachable for ordinary relations (`infoOf` refuses closed,
  writer.ts:331-334), where sealed order = declaration order = `member.data.fields`.

**Alignment:** export `factOf`, `rowOf` (and transitively `cellOf`/`handleOf` if wanted)
from ts/src/index.ts. Then delete both twins (**61 lines**), and the closed-handle
bijection has exactly ONE spelling in TS, as the marshal header itself demands
(ts/src/marshal.ts:2-4 "in ONE place only").

One behavioral note: `lowerFact` additionally runs `checkAgainst` per cell
(writer.ts:316), which adds u64/i64 range and interval width/domain checks that `cellOf`
defers to native. This is redundant on the writer path — `encodeBatch` re-gates every
cell with `checkAgainst` before any bytes exist (ts-log/src/codec.ts:105-114), inside the
same `commit` call — so switching to `rowOf` only moves the range-refusal throw site from
record time to encode time, with identical outcomes.

## 5. Everything else ts-log could import but re-declares — sweep results

- **`Violation`** — already imported (ts-log/src/writer.ts:25); `Commit`/`BraidOutcome`
  carry `readonly Violation<Rels>[]` (writer.ts:96,105). No action; this is the model.
- **`Fact`, `FreshKeys`, `MemberRelation`, `SchemaRelations`, `Schema`, `Db`,
  `WriteOutcome`** — already imported (writer.ts:19-26, replica.ts:13-14, tenants.ts:17).
  No re-declarations found.
- **`internalBlake3`** — already imported (writer.ts:22, replica.ts:14, store.ts:21, and
  4 test files). No second hash implementation exists.
- **`Generation`** (ts-log/src/keys.ts:52-53, branded u64 bigint): the engine has no
  branded generation type — its generations are bare `bigint` (`Committed.generation`
  ts/src/db.ts:237, `instance.generation` db.ts:320) and mean the store's total commit
  count, whereas the log's `Generation` is a per-braid slot number. Different concept,
  brand is load-bearing (slot arithmetic). **(c).**
- **Error identities** (ts-log/src/errors.ts): the `RefusalCause` sum (errors.ts:71-95)
  is pinned string-for-string to the Rust driver's `DecodeError::identity`
  (errors.ts:66-69) — a cross-implementation wire contract, not an engine shape. The
  engine's `ErrFingerprintMismatch` (ts/src/db.ts:1320) is a store-open refusal; the
  log's `FingerprintMismatch` cause (errors.ts:76) is a batch-decode refusal. Same word,
  different objects; merging them would conflate channels. The engine's
  `ErrorFamilyKind` (ts/src/native.ts:272-296) does not overlap. **(c).**
- **Schema fingerprint types**: the engine's `SealedDescriptor.fingerprint` is a hex
  `string` (native.ts:207); the log carries it as branded `Digest32` bytes
  (ts-log/src/manifest.ts:28, descriptor.ts:181-182 keeps both spellings). `Digest32`
  (ts-log/src/bytes.ts:23) is the log's wire brand with no engine counterpart. **(c).**
  Non-type adjacency: `catalogDigestOf` (ts-log/src/replica.ts:206-218) duck-probes a
  `catalogDigest` method the SDK does not declare on `Db`/`ReadInstance` — an engine API
  gap worth a typed export, currently papered over with casts.
- **bytes.ts, vector.ts, chain.ts, store.ts, store-s3.ts, tenants.ts, manifest.ts,
  keys.ts**: byte reader/writer, per-braid vectors, the sidecar chain, the five-verb
  object store, tenants, and the manifest/checkpoint documents. Grep of every type
  declaration in these files found no engine-shaped duplicate. **(c) throughout.**

## 6. Genuinely log-specific (justifications)

| item | where | why no engine counterpart |
|---|---|---|
| tagged wire codec (`TAG`, `writeTagged`, `readTagged`, `TaggedRefusal`, `valuesEqual`, `writeCanonicalLiteral`, `WellFormedUtf8`) | ts-log/src/value.ts:22-66, 145-323 | the engine SDK never encodes values in TS — bytes are minted in Rust; this TS encoder exists to be pinned byte-equal to the Rust driver by the conformance goldens (codec.ts:2-8) |
| `checkAgainst` | ts-log/src/value.ts:89-143 | re-encodes the engine's typed write judgment over `ValueTypeSpec`, which the engine SDK deliberately defers to native; the codec must refuse pre-encode. Candidate for a future engine export, not an import today |
| `Braid`, braid derivation, `serialAt` | ts-log/src/descriptor.ts:80-170, 708-771; braids.ts | replication sharding derived FROM the sealed statements; the engine has no shard notion |
| `Batch`'s missing `contains`/`get` | ts-log/src/writer.ts:117-125 | the recorder journals without applying; there is no state to read until `disciplineCommit` applies (writer.ts:559-597) |
| `Commit`/`BraidOutcome`/`Durability`/`Deposition` | ts-log/src/writer.ts:86-141 | log-protocol outcomes (slot, durability, usurper); already reuse engine `Violation` for the rejected arm |
| `Generation`/`StoreKey` brands, key layout | ts-log/src/keys.ts | object-store key grammar |
| error identities + cause data | ts-log/src/errors.ts | pinned to the Rust log driver, disjoint from engine error families |
| `assembleFromSpec` | ts-log/src/descriptor.ts:392-706 | conformance corpus includes engine-refused shapes — but move to test support (section 3c) |

## 7. Tests (ts-log/test/)

- **No engine-shape re-declarations found.** `Corpus*` interfaces
  (parity.test.ts:26-49, conformance-v3-support.ts:17-40) and the sidecar interfaces
  (conformance-v3.test.ts:28-79) describe the JSON fixture format, and both loaders
  already build the engine's exported `SchemaSpec`/`ValueSpec`/`ValueTypeSpec`/
  `LiteralSpec`/`StatementSpec` (imports at parity.test.ts:5,
  conformance-v3-support.ts:9). fixtures.ts and fingerprint.test.ts are pure engine-SDK
  usage (`relation`, `schema`, `closed`, `span`, ... — fixtures.ts:1-17).
- **Intra-package duplication:** parity.test.ts:26-249 (`CorpusValue`, `CorpusField`,
  `CorpusRelation`, `CorpusSide`, `CorpusSchema`, `typeOf`, `valueSpecOf`, `specOf`) is a
  near-verbatim copy of conformance-v3-support.ts:17-227. parity.test.ts should import
  the loader (**~225 lines**). Its `rawValue` (parity.test.ts:96-110) additionally
  duplicates src/descriptor.ts:392-407.
- conformance-v3.test.ts:26 imports `Interval, Value` from `#value.ts` — flips to the
  engine types automatically with item 1.

## 8. Ordered punch list

Engine-side (ts/):
1. Export from ts/src/index.ts: `factOf`, `rowOf` (ts/src/marshal.ts), `freshRangeOf`
   (ts/src/db.ts:139), `isIntervalValue` (ts/src/fields.ts) — and optionally
   `Manifest`/`ManifestRelation`/`ManifestRow`/`ManifestField` (ts/src/native.ts).
2. Widen `ManifestField` (ts/src/native.ts:134-138 + the Rust napi descriptor) with
   `fresh: boolean` and `newtype: string | undefined`.

ts-log-side, in dependency order:
3. `Value := FactValue`, `Interval := IntervalValue` (value.ts:15-20); re-export engine
   types from index.ts:49; delete `isInterval` after (1).
4. descriptor.ts: import `SealedSide`/`SealedWeight`/`SealedHi`/`SealedStatement`; delete
   `SideInfo`/`WeightInfo`/`HiInfo`/`StatementInfo`/`asValue`/`sideOf`/`statementOf`;
   after (2), delete the spec re-join + `idClassOwner`; move `assembleFromSpec` family to
   test support.
5. writer.ts: `Batch.reserve` returns `FreshRange` (or full
   `Pick<WriteTx, "insert"|"delete"|"reserve">` if the pre-judgment `MutationReport` echo
   is accepted); delete `lowerFact` in favor of engine `rowOf`.
6. replica.ts: delete `factOf` in favor of engine `factOf`; drop the cast at 363-365.
7. tests: dedupe parity.test.ts's corpus loader into conformance-v3-support.ts.
