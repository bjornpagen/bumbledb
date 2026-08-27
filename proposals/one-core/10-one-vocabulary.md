# 10 — One vocabulary

> **Decision.** `ts-log` speaks the engine SDK's types **verbatim**.
> Every value, fact, descriptor, outcome, and range that crosses between
> the two packages is the engine's type, imported — never a structural
> twin, never a near-copy with a drifted return shape. Where the log
> genuinely extends the engine's algebra, it **composes** the engine
> type rather than restating it. One fact, one spelling, per language.

## The current representation

The audit (audit/30, audit/40) found the vocabulary duplicated at every
seam, and every duplication either already drifted or is one edit from
drifting:

- **Two identical value unions.** `ts-log`'s `Value`/`Interval`
  (`ts-log/src/value.ts:15-20`) are byte-for-byte the engine's
  `FactValue`/`IntervalValue`, both already exported
  (`ts/src/index.ts:126,115`). Two unions, one meaning, and the
  `start < end` invariant owned twice.
- **Four sealed types restated.** `descriptor.ts:44-78` re-declares
  `SideInfo`≡`SealedSide`, `WeightInfo`≡`SealedWeight`,
  `HiInfo`≡`SealedHi`, `StatementInfo`≡`SealedStatement` identically
  (engine: `ts/src/native.ts:164-202`), plus ~50 lines of converters
  that become identity functions the moment the types unify.
- **The write vocabulary is one return type away from subtyping.**
  `Batch.insert/delete` params already equal `WriteTx`'s
  (`CollectionWrite<R>` *is* `Iterable<Fact<R>>`); the single blocking
  delta is `reserve` returning `readonly bigint[]` where the engine
  returns `FreshRange` — gratuitous, since `drawIds`
  (`writer.ts:193-226`) only draws contiguous runs. Across all four
  surfaces, `reserve` returns **four different types** (audit/40).
- **`Commit` restates `Admission`.** The log's commit outcome carries
  the engine's `Rejected(Violations)` payload inside a hand-restated
  sum shell — in both languages (audit/40 §3).
- **The marshal twins.** `replica.ts:325-351` and `writer.ts:286-319`
  duplicate the engine's `factOf`/`rowOf` because `ts/src/marshal.ts`
  is not index-exported; the duplication also forces an
  `as unknown as` cast in `applyOps`.
- **One identity, three spellings, two meanings.**
  `FreshExhausted`/`LeaseRefusal::Exhausted`/`ErrExhausted` are one
  identity spelled three ways — and ts-log spells a *cache miss* with
  it (`writer.ts:205-209`) while Rust leases a new block mid-draw.
- **A missing arm, live.** ts-log never surfaces `Waited::Wedged`:
  `waitFor` on a wedged braid polls forever where Rust returns an
  outcome.
- **Three coordinates share one name.** "generation" names the store
  sum, the braid slot, and the per-braid count depending on the file;
  the log `Commit.generation` is a *slot* (audit/40 §2, audit/60's
  naming trap).
- Smaller residue: the napi `ManifestField` drops `fresh`/`newtype`, so
  descriptor.ts re-joins ~50 lines of spec the engine already knows;
  `assembleFromSpec` (~315 lines) ships in src but serves only tests;
  `replica.ts:206-218` duck-probes an undeclared `catalogDigest`.

## The target representation

### 1. One value vocabulary — the same type, never an alias

The log uses `FactValue` and `IntervalValue` **themselves**: the same
declarations, imported from `@bjornpagen/bumbledb` at every use site.
Not `type Value = FactValue`, not a re-export under a local name — the
identifiers `Value` and `Interval` *die* from `ts-log`, and call sites
say the engine's names. An alias is a second name for one fact, and a
second name is where the next structural twin grows; the compiler's
nominal graph is the identity check, so there is nothing to keep in
sync. This is already the Rust driver's law — `bumbledb-log` does
`use bumbledb::{Value, Violations, Admission}` and always has — the TS
driver simply joins it. `ts-log`'s index re-exports **none** of the
engine's types: a consumer who needs them imports the engine, which is
already a peer dependency; `ts-log`'s public surface names only what
the log itself owns. `ts-log/src/value.ts`'s twin unions die, and
`Op.rows`, `lowerFact`, and `applyOps` become engine-typed end to end.
The four sealed descriptor types are imported the same way; the
converters die as identities. The engine exports `factOf`/`rowOf` from
its index (the first of two engine-side changes), and both marshal
twins die with the cast.

### 2. `Batch` is a structural subtype of `WriteTx`

The engine's `reserve` shape — `FreshRange` — becomes the one shape
(the second engine-side change is none: the log adopts the engine's).
With that, a write-only body typechecks against both surfaces: **code
written for embedded mode runs unchanged under the log writer**, as a
compiler-checked fact. `contains`/`get`'s absence from `Batch` stays —
the journaled dialect is a pure recorder by law (the replay/re-judgment
representation), and that is the *only* difference left.

### 3. `Commit` composes `Admission`

In both languages, the log's commit outcome is
`Admission<{value, braid, slot, durability, …}>` — the engine's sum
carrying log payloads in its arms, never a restatement of the arms. The
`Rejected` payload is already the engine's `Violations`; now the shell
is too. The field named `generation` on the outcome is renamed
**`slot`**, and the three-coordinates ambiguity ends: *sum* is the
store generation, *slot* is a braid position, *count* is per-braid —
one name per coordinate, everywhere, both languages.

### 4. One identity per fact of failure

The exhaustion identity is defined once and means one thing; the TS
cache-miss path gets its own honest arm (a refill is not an
exhaustion). `Waited` is surfaced in ts-log as the full sum —
`Reached | Wedged | Refused` — so a wedged braid is an outcome, not an
infinite poll. The identity-string table that conformance pins
(`DecodeError::identity` ↔ `RefusalCause.kind`) becomes **generated
data** consumed by both languages ([40](40-the-oracle.md) owns the
generator), so a tail-kind added unilaterally — which already happened
(audit/40 §4) — becomes a build error, not a drift.

### 5. The verb-parity sweep

The remaining asymmetries get one ruling each, applied both ways:
empty commits are a no-op in the engine and an error in the log —
ruled: the log keeps its refusal (law 6: the empty commit is not a
commit) and the divergence is *documented in the type* by the distinct
outcome, not discovered in behavior. `reserve_capacity` exists in log
Rust only — it lands in ts-log or dies in Rust; the sweep decides by
whether any consumer exists, and logs the ruling.

## What gets deleted

| Deleted | Because |
| --- | --- |
| `ts-log` `Value`/`Interval` twin unions and the doubly-owned interval invariant | `FactValue`/`IntervalValue`, imported |
| the four sealed-type restatements + ~50 converter lines | imported; converters are identities |
| both marshal twins + the `as unknown as` cast | engine exports `factOf`/`rowOf` |
| `reserve`'s `readonly bigint[]` shape | `FreshRange` is the one shape |
| the hand-restated `Commit` sum shells, both languages | `Admission` composition |
| the cache-miss `ErrExhausted` spelling | a refill arm; exhaustion means exhaustion |
| `assembleFromSpec` from the shipped package | moves to test support |
| the `catalogDigest` duck-probe | declared on the engine handle |

Roughly ~205 lines of shipped TS and ~315 of misplaced test shadow die
immediately (audit/30), before [20](20-one-reader.md) deletes the
grammar mirror wholesale.

## The invariant

> **A fact that crosses the engine/log seam has exactly one type per
> language, and it is the engine's — nominally, not structurally.** The
> same declaration, imported; no alias, no local rename, no twin that
> happens to match. The log extends by composition — its arms carry
> engine payloads, its ranges are engine ranges, its verdicts are
> engine verdicts with log context — so a divergence between the
> packages' vocabularies is not reviewed away; it is unrepresentable in
> a tree that compiles, because there is only one declaration to
> diverge from.

Dissolves: audit/30 categories (a) and (b) whole; audit/40 ranked items
1, 2, 3, 4, 6, and 7. Item 5 (the generated identity table) is built in
[40](40-the-oracle.md); the live `waitFor` and `ErrExhausted` bugs are
also counted in [30](30-pin-the-dark.md)'s bug ledger.
