# 10 — Data Model

## Relations are sets of facts

- Every relation is a set of full, typed facts. Canonical membership is implicit for
  every relation — it is not modeled as a primary key or covering unique constraint.
- `insert(fact)` is an idempotent no-op if the fact exists; `delete(fact)` is an
  idempotent no-op if it doesn't. Both report whether they changed state.
- **There is no update operation.** Mutation is delete + insert.
- There are no hidden row identities in the logical model. (Storage assigns internal row
  ids; they are never visible — see `40-storage.md`.)

**Decision: no primary-key concept.** **Alternative:** day-1 (v1) design had entity
relations with PKs and a whole-row `replace`. **Why it lost:** the set-native model has
one identity concept (the fact itself) and one mutation algebra (insert/delete) — fewer
concepts, no PK-vs-unique duality, no "which fields can change" special cases. The cost —
mutating a fact that other facts reference through an FK — is a real corner, and how to
handle it is `OPEN` (commit-time FK checking vs a `replace` convenience; see README).

## No nulls

Null does not exist anywhere: not in storage, not in the IR, not in results, not in
aggregation. Optional data is an absent fact in a separate relation. This is load-bearing
for the whole design — negation (when it comes) is plain set anti-join, empty aggregate
groups simply don't appear, and no operator anywhere needs a null branch.

## Value types

```
Bool                          1 byte
U64, I64                      8 bytes, order-preserving encodings
Enum { name }                 1 byte, closed domain declared in schema
String, Bytes                 interned; 8-byte dictionary id in facts
Serial { type_name, owning_relation }   nominal 8-byte id
```

- **Serial** values are database-generated monotonic `u64` sequences per declared serial
  field, and they are *nominal*: `AccountId` and `InstrumentId` never unify even though
  both are 8 bytes. Aborted transactions don't advance the committed sequence; values are
  never reused; ETL may supply explicit values, which advance the high-water mark.
- **Timestamps** are an application convention: `I64` UTC microseconds.
- **Money** is an application convention: scaled `I64` (first-class `Decimal`/`Money`/
  `Currency` types were deliberately dropped in the v5 line; that paring stands).
- **UUID was deleted** (v3) and stays deleted. Floats are forbidden in persistent data.
- `OPEN`: nominal scalar domains (`I64 as "UsdCents"`, `I64 as "TimestampMicros"`) —
  Serial's nominal-typing rule applied to plain scalars, zero new encodings, closes the
  hole where two unrelated scaled-i64 conventions unify silently in a query.

**Decision: tiny closed type roster, conventions over first-class domain types.**
**Alternative:** v1's roster (Decimal<scale> as i128, Timestamp, Date, Uuid, Money).
**Why it lost:** every first-class type is encoding + comparison + aggregation +
validation branches through the whole engine. Conventions cost nothing in the engine;
the type-safety gap they open is the nominal-domain proposal above, which buys the
safety back at the typechecker layer only.

## Interning

Strings and bytes are dictionary-interned: forward map keyed by hash of the raw bytes
(with equality verification on lookup), reverse map id → bytes, ids monotonic and never
reused, entries append-only. **Known accepted limitation:** no dictionary GC — deleted
facts leak their interned values. At the design scale this is noise; documented so
nobody rediscovers it as a bug.

Consequence: string *equality* is cheap (id compare); string ordering, prefix search,
and text search are not supported. If ever needed, they get explicit ordered text
indexes — they will not fall out of the dictionary by accident.

## Constraints

- **Unique**: named logical constraints over field subsets — `Unique { name, fields }`.
- **Foreign key**: `ForeignKey { name, fields, target_relation, target_constraint }`,
  targeting a *named unique constraint* (never bare fields), positional exact-type
  compatibility, `Restrict` semantics only. Enforcement timing is `OPEN` (see README).
- Required-ness is implicit: no nulls means every field of a fact is always present.
- Check constraints, cascades, deferred constraints: not in the model. Cross-fact
  invariants are application logic.

## Schema

Schemas are declared in Rust and compiled into the binary. The declaration produces
relation/field/constraint descriptors and a canonical byte serialization hashed (blake3)
into a **schema fingerprint** stored at database creation. Open compares fingerprints;
mismatch is a hard failure. There is no migration, no ALTER, no compatibility reader:
schema change = ETL into a new database with the new binary.

**Decision: schema lives in Rust, not in a data file or query text.** **Alternative:** a
schema declaration syntax in some external format. **Why it lost:** the schema must
generate typed Rust API surface anyway, and there is exactly one consumer. A second
declaration language is a parser plus a synchronization problem, for nobody.

## The modeling discipline (BCNF, no way out)

Natural n-ary relations for domain facts; natural edge relations (`OrgParent(child,
parent)`) welcome; forbidden: nullable columns (unrepresentable anyway), JSON blobs,
generic entity-attribute-value relations. Temporal/status/history needs are modeled as
immutable event facts, not mutable columns.
