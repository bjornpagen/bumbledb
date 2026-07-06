# 10 — Data Model

## The type layer: a type is an encoding, and nothing else

Bumbledb is **hard structurally typed** (owner ruling, 2026-07-02). The engine's type
layer answers exactly one question — what do the bytes mean and which operations exist —
and carries no names, no identity metaphysics:

```
Bool                     1 byte, strictly 0x00 or 0x01
Enum(variants)           1 byte, ordinal into an ordered variant-name list
U64                      8 bytes, big-endian (order-preserving)
I64                      8 bytes, sign-flipped big-endian (order-preserving)
String                   8 bytes in facts: interned dictionary id
Bytes                    8 bytes in facts: interned dictionary id
```

Six types, two physical widths (1 and 8 bytes). Type equality is **structural equality
of the description**; unification, comparison legality, and FK compatibility are all
just that equality. There is no `name` field anywhere in the type layer.

**Enum identity is its ordered variant-name list.** Two fields declaring
`[Active, Closed, Frozen]` are the same type and unify, whatever the schema calls them.
Rationale (the owner's, verbatim in spirit): *an enum is a schema-level relation whose
extension is closed and known at compile time* — three states don't deserve a table; the
byte is the table, inlined. The ordinal encoding is declaration order; >256 variants is
a schema-declaration error; an out-of-range ordinal anywhere is corruption.
**Decision:** structural enums. **Alternative:** nominal `Enum{name}` (v5's design).
**Why it lost:** hard-structural ruling — the variant list is what determines the
encoding, so it *is* the identity; the wrapper name types nothing. **Reverses if:**
never — philosophy.

**Orderability, complete:** U64 and I64 support ordering (`Lt/Le/Gt/Ge`, `Min`, `Max`,
range predicates). **Everything else is equality-only.** Enum ordinal order is a
declaration-order accident, not semantics; String/Bytes intern ids are meaningless to
order; Bool ordering is noise. One sentence, no matrix, no exceptions.

**Names live in the host.** The schema macro generates Rust newtypes
(`struct AccountId(pub u64);`, `struct Cents(pub i64);`) mapped to structural engine
types at the boundary. rustc polices typos and cross-domain confusion at the
application's compile time — stronger than engine nominal typing, since it also protects
non-database code. The engine will not stop a query joining `Posting.account` to
`Instrument.id`; both are U64, that query means what it says. FK declarations document
intent; the host prevents the mistake. This is the design, not a hole.
**Decision:** nominal typing rejected everywhere (including the previously-OPEN
"nominal scalar domains" proposal — closed). **Alternative:** nominal Serial/domain
wrappers in the engine. **Why it lost:** owner is a hard structural-typing fan; nominal
safety is free in the host. **Reverses if:** never — philosophy.

**Conventions** (documentation, invisible to the engine — the host newtypes them):
Timestamp = I64 UTC microseconds; Date = I64 days since epoch; Duration = I64
microseconds; Money = scaled I64, scale chosen per application. First-class
Decimal/Timestamp/Uuid/i128/narrow-int types are all considered-and-rejected: scaled i64
covers personal-ledger magnitudes (±92 quadrillion cents), and a second scalar width
would double the codec and NEON kernel shapes for bytes that don't matter at this scale.

## Fields: type + generation

A field is `(name, type, generation)` where `generation ∈ {None, Serial}`. Generation is
a **storage behavior, not a type**:

- A `Serial` field must be `U64`. The database mints its values: monotonic per
  (relation, field), never re-issuing any value observable in a committed state; aborted
  transactions don't advance the committed sequence.
- **The usage pattern this exists for** — insert a new row without ever reading a max:

  ```rust
  db.write(|tx| {
      let id: AccountId = tx.alloc()?;             // mints the next AccountId value
      tx.insert(&Account { id, holder, status })?; // insert always takes complete facts
      Ok(id)
  })?
  ```

  (The typed surface infers the field from the `Serial` newtype; the untyped form is
  `tx.alloc_dyn(relation, field)`.)

  `alloc` is the only generator; `insert` is always full-fact and stays idempotent —
  one insert semantics, no generative variant.
- Explicit values are legal on the normal write path (not just ETL): inserting with a
  chosen value ≤ or > the high-water mark succeeds and advances the mark past it. This
  is load-bearing: correcting a serial-keyed fact is `delete(old); insert(new with the
  same id)`. "Never reused" constrains the *generator* only — it never re-issues a value
  that was ever committed; explicit re-supply of a deleted value is legal. Mixed
  explicit/generated allocation within one transaction tracks the running maximum.
  A *successful* commit persists every serial value it issued, even when no facts
  changed — the closure may have returned those ids to the host, and an observed id is
  never re-issued (the counters-only commit writes exactly the dirty `Q` marks: no
  generation bump, no cache eviction). Aborted transactions (`Err`/panic) still drop
  their allocations; nothing they minted was observably returned.
- **A Serial field auto-materializes an ordinary named unique constraint** on itself,
  visible in the relation's descriptor and FK-targetable like any declared constraint.
  Two Accounts sharing an AccountId is unrepresentable by construction, with zero
  special enforcement machinery — the generator implies the invariant; the invariant is
  expressed in the one mechanism that owns invariants.
- Referencing fields are plain `U64` fields; referential intent is carried entirely by
  an explicit FK constraint. There is no defining-vs-reference occurrence question —
  generation is a per-field-declaration attribute and nothing else.

**Decision:** serial demoted from type to generation attribute + auto-unique.
**Alternatives:** (a) nominal `Serial{type_name, owning_relation}` (v5) — lost to the
structural ruling; (b) generative insert (insert mints and returns the id) — lost
because it splits insert into two semantics and breaks idempotence and the fact-hash
membership check. **Reverses if:** never — this is strictly simpler and preserves the
ergonomic contract.

## Relations are sets of facts

- Every relation is a set of full, typed facts. Canonical membership is implicit for
  every relation; there is no primary-key concept and no hidden row identity in the
  logical model (storage's row ids never surface).
- `insert(fact)` is an idempotent no-op if the fact exists; `delete(fact)` is an
  idempotent no-op if it doesn't. Both report whether they changed state. **There is no
  update operation**; mutation is delete + insert (within one write transaction), with
  explicit serial re-supply preserving identity across the swap.
- Nullary (zero-field) relations are legal: a set that is empty or contains the single
  empty fact — a database-level boolean. Every layer (encoding, hashing, IR) defines
  behavior for it because it falls out of the representation.

**Decision: no primary keys.** **Alternative:** v1's entity relations with PKs and
whole-row `replace`. **Why it lost:** one identity concept (the fact), one mutation
algebra (insert/delete), no PK-vs-unique duality. Consequences now explicit: mutating a
referenced fact needs constraint-timing rules (decided below: commit-time, against the
final state) and serial re-supply (specified above). **Reverses if:** the delete+insert idiom proves unlivable in real
app code.

## No nulls

Null does not exist anywhere: not in storage, the IR, results, or aggregation. Optional
data is an absent fact in a separate relation. Load-bearing: negation (when it comes) is
plain anti-join; empty aggregate groups don't appear; no operator anywhere has a null
branch.

## Fact identity: the canonical encoding

This document owns fact equality. **Value equality ≡ `fact_bytes` equality**, where
`fact_bytes` is the concatenation of each field's canonical encoding **in declaration
order**, with **no padding between fields** — facts are dense (1-byte enums/bools sit
flush against 8-byte fields; Apple Silicon's near-free unaligned loads make intra-row
alignment a pure waste, `00-product.md`). Canonical means injective and unique: Bool is strictly 0/1 (any other byte is
corruption, never a distinct "true"); Enum is the declaration-order ordinal; integers
are their order-preserving encodings; String/Bytes are their intern ids (one byte
sequence ⇒ exactly one id, ever). Storage (`40-storage.md`) implements membership as
blake3-256 of `fact_bytes`; **hash equality is treated as fact equality — collisions
are an accepted axiom** (2⁻¹²⁸-scale event), not verified against, and the same axiom
applies to the dictionary's content hash. Recorded once, here.

## Interning

One global dictionary for String and Bytes, segregated by a type-tag byte inside the
hashed key (a String and a Bytes with identical bytes get distinct ids). Forward map:
blake3(tag ‖ bytes) → id (collision axiom above); reverse map: id → bytes. Ids are
monotonic, never reused, append-only; interning happens only inside write transactions.
Strings are validated UTF-8 at intern time (parse, don't validate). On the read path,
query literals resolve by read-only lookup — a dictionary miss means the literal cannot
match any fact: **empty result, never an insert, never an error** (resolved
per-execution; see `20-query-ir.md`). Known accepted limitation: no GC — deleted facts
leak their interned values; at design scale this is noise.

Consequence: string equality is cheap (id compare); string ordering, prefix search, and
text search are unsupported and would require explicit ordered text indexes if ever
wanted.

**Decision: interning.** **Alternative:** inline variable-length or fixed-prefix string
encodings preserving order. **Why it lost:** interning keeps facts fixed-width (O(1)
column slicing — the load-bearing property of the whole storage design) and makes
equality one word-compare; the ledger workload orders by time and amount, not by string.
**Reverses if:** string range/prefix queries become a real workload need (then: ordered
text indexes, not de-interning).

## Constraints

- `Unique { name, fields }` — `fields` is ordered (the order defines the guard key and
  the FK target shape), non-empty, duplicate-free by construction. Names are scoped per
  relation.
- `ForeignKey { name, fields, target_relation, target_constraint }` — targets a named
  unique constraint, positional structural-type equality, `Restrict` only.
- Serial fields contribute their auto-materialized unique constraint (above).
- **Constraints are invariants on committed states, enforced once at commit** against
  the transaction's final state; a violation aborts the whole transaction with a typed
  error carrying the relation and constraint ids (names resolvable through the schema)
  and the offending fact's bytes — the Restrict arm names the surviving *referrer* by
  its fact, since storage row ids never surface. There is no per-operation
  enforcement and no deferral opt-in — commit-time is the only semantics. This is the
  strict option: since queries inside write transactions are forbidden, intermediate
  states are unobservable by construction, so "every state anyone can ever see
  satisfies every constraint, with no way out" is the total guarantee — stricter than
  SQL's opt-in deferrable constraints. `Restrict` means precisely: *no committed state
  contains a dangling reference* (deleting a target and all its referrers in one
  transaction passes, as it should). Consequences: user operation order is
  semantically irrelevant (the delete-before-insert ordering trap is unrepresentable —
  see the delta write path, `40-storage.md`), and cyclic references insert without any
  staging concept.
  **Decision.** **Alternative:** per-operation checking with staged same-transaction
  visibility (the day-1 design). **Why it lost:** it enforces invariants on states
  nobody can observe, pushes ordering obligations onto the caller, makes cyclic inserts
  need a preallocation dance, and fights the accumulate-then-commit write path.
  Offering both would be a semantics-splitting mode. **Reverses if:** never —
  semantics. (This also closes the `replace` question: with ordering irrelevant,
  delete+insert is the blessed idiom and `replace` is at most host-side sugar.)
- Check constraints, cascades, deferred-as-a-choice: not in the model. Cross-fact
  invariants are application logic.

## Schema

Schemas are declared in Rust and compiled into the binary. The declaration produces
descriptors, the host-side newtypes, and a canonical byte serialization hashed (blake3)
into the **schema fingerprint**, stored at database creation; open compares fingerprints
and mismatches are hard failures. No migration, no ALTER: schema change = ETL into a new
database (export surface: `60-api.md`).

**Fingerprint inputs, exhaustively:** an encoding-format version label; relations in
declaration order — for each: name, fields in declaration order (name, structural type
description — including the full ordered variant list for enums — and generation flag),
and constraints in *materialized* order (name, ordered field list, and for FKs the
target relation + constraint names). Materialized order = the serial auto-uniques
first (one per serial field, in field order), then the declared constraints in
declaration order — a deterministic function of the declaration, so constraint ids
remain pinned by the fingerprint without being hashed separately. Relation and field
ids are plain declaration order.
Stated consequence, accepted: **adding an enum variant changes the fingerprint — a full
ETL rebuild.** Closed domains are closed.

**Decision: schema lives in Rust.** **Alternative:** external schema declaration
format. **Why it lost:** the schema must generate typed Rust API anyway and there is one
consumer; a second language is a parser plus a sync problem for nobody. **Reverses if:**
never, absent a second consumer.

## The modeling discipline (BCNF by discipline)

Natural n-ary relations for domain facts; natural edge relations (`OrgParent(child,
parent)`) welcome; enums for closed domains instead of two-row lookup tables; forbidden
by construction: nullable columns, JSON blobs; forbidden by discipline: EAV,
denormalized redundancy. Temporal/status/history needs are modeled as immutable event
facts, not mutable columns.
