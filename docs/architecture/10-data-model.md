# 10 — Data Model

## The type layer: a type is an encoding, and nothing else

Bumbledb is **hard structurally typed** (owner ruling). The engine's type
layer answers exactly one question — what do the bytes mean and which operations exist —
and carries no names, no identity metaphysics:

```
Bool                     1 byte, strictly 0x00 or 0x01
Enum(variants)           1 byte, ordinal into an ordered variant-name list
U64                      8 bytes, big-endian (order-preserving)
I64                      8 bytes, sign-flipped big-endian (order-preserving)
String                   8 bytes in facts: interned dictionary id
Bytes                    8 bytes in facts: interned dictionary id
Interval(element)        16 bytes: start ‖ end, each the element encoding;
                         element ∈ {U64, I64}; strictly start < end
```

Seven types, three physical widths (1, 8, and 16 bytes). Type equality is **structural
equality of the description**; unification, comparison legality, and dependency
compatibility are all just that equality. There is no `name` field anywhere in the type
layer.

**Enum identity is its ordered variant-name list.** Two fields declaring
`[Active, Closed, Frozen]` are the same type and unify, whatever the schema calls them.
Rationale (the owner's, verbatim in spirit): *an enum is a schema-level relation whose
extension is closed and known at compile time* — three states don't deserve a table; the
byte is the table, inlined. The ordinal encoding is declaration order; >256 variants is
a schema-declaration error; an out-of-range ordinal anywhere is corruption.
**Decision:** structural enums. **Alternative:** nominal `Enum{name}`.
**Why it lost:** hard-structural ruling — the variant list is what determines the
encoding, so it *is* the identity; the wrapper name types nothing. **Reverses if:**
never — philosophy.

**Orderability, complete:** U64 and I64 support ordering (`Lt/Le/Gt/Ge`, `Min`, `Max`,
range predicates). Interval supports **equality, `Overlaps`, `Contains`, and point
membership** (below) — never `Lt`-family order or `Min`/`Max`: the value order that
exists (lexicographic by start) is an encoding accident, and offering it would invite
queries that mean overlap and say "less than". Everything else is equality-only. Enum
ordinal order is a declaration-order accident, not semantics; String/Bytes intern ids
are meaningless to order; Bool ordering is noise.

**Names live in the host.** The schema macro generates Rust newtypes
(`struct AccountId(pub u64);`, `struct Cents(pub i64);`, `struct ValidDuring(pub
Interval<i64>);`) mapped to structural engine types at the boundary. rustc polices
typos and cross-domain confusion at the application's compile time — stronger than
engine nominal typing, since it also protects non-database code. The engine will not
stop a query joining `Posting.account` to `Instrument.id`; both are U64, that query
means what it says. Inclusion statements document intent; the host prevents the
mistake. This is the design, not a hole.
**Decision:** nominal typing rejected everywhere. **Alternative:** nominal
Fresh/domain wrappers in the engine. **Why it lost:** owner is a hard
structural-typing fan; nominal safety is free in the host. **Reverses if:** never —
philosophy.

**Conventions** (documentation, invisible to the engine — the host newtypes them):
Timestamp = I64 UTC microseconds; Date = I64 days since epoch; Duration = I64
microseconds; Money = scaled I64, scale chosen per application; rates = scaled I64
(basis points or ppm). First-class Decimal/Timestamp/Uuid/i128/narrow-int types are
all considered-and-rejected: scaled i64 covers personal-ledger magnitudes (±92
quadrillion cents), and a second scalar width would multiply the codec and NEON kernel
shapes for bytes that don't matter at this scale.
**Uuid, rejected explicitly:** uuidv7 in the surveyed Postgres workloads
does three jobs — identity, clash-avoidance, and clock — because Postgres cannot trust
its clients to mint ids. Here the single-writer engine mints fresh ids (identity and
clash-rejection by construction), and wall-clock time is an explicit I64 column.
An id is an id; a clock is a clock; a type that is secretly both is a lie with a
timestamp in it. **Reverses if:** never — the jobs are covered separately and better.

## Interval: the denotation

An `Interval` value `[s, e)` is **a finite set of points, written as its bounds** —
half-open over the element domain, `s < e` enforced at the encoding boundary exactly
as Bool's strict 0/1 is (a stored `s ≥ e` is corruption, and the empty interval is
unrepresentable: a fact never denotes nothing). Encoding: `start ‖ end`, each in the
element type's order-preserving encoding, so the 16 bytes sort lexicographically by
start — the property the storage layer's neighbor probes stand on (`50-storage.md`).

**The denotation rule, normative:** a fact whose interval field holds `[s, e)` *means*
the family of point-facts obtained by replacing the interval with each `t`, `s ≤ t < e`.
Everything temporal in this database is a corollary of this sentence and owns no
machinery of its own:

- A **functional dependency over an interval position holds pointwise** — no two facts
  in the key group may share any point, i.e. their intervals must not overlap
  (`30-dependencies.md`). SQL:2011 spells this `WITHOUT OVERLAPS` and ships it as a
  keyword; here it is not an option but what the judgment *means* on this type.
- An **inclusion dependency over an interval position holds pointwise** — every point
  of the source's interval is covered by the target's intervals (SQL:2011's `PERIOD`
  foreign keys). The target needs a pointwise key for this to be checkable in
  logarithmic time; validation demands it (`30-dependencies.md`).
- **Point membership is a typing rule, not syntax**: a query atom binding an
  interval-typed field with an element-typed term means `t ∈ interval`
  (`20-query-ir.md`).

**Unbounded ends are the element maximum, by convention:** `[s, ∞)` is written
`[s, MAX)` where MAX is the element type's greatest encodable value. Consequence,
stated: MAX itself is unusable as a point. The alternative — first-class ∞ — buys a
17th byte or a stolen sentinel anyway, and changes nothing the neighbor probe can
observe. Open-ended states ("currently active") are exactly this convention.

**Coalescing is an aggregate, never a write rule.** Two facts `(x, [1,5))` and
`(x, [3,8))` are distinct facts whose denotations overlap; the engine stores what it
was given (identity is bytes, below). The packed canonical form — maximal disjoint
intervals per group, the temporal literature's *coalesce*, Postgres's `range_agg` —
is a future aggregate (`Pack`, OPEN in `20-query-ir.md`). A relation that *wants*
its intervals disjoint declares the pointwise key; the dependency system expresses
the policy, the engine never imposes it.

**Why a type and not a pattern — the argument, recorded.** `(start, end)` as two I64
columns is not a normalization violation (with key `(x, start)` every FD has a
superkey left side — BCNF is satisfied and blind), and decomposing an interval into
two *relations* glued by dependencies is the reductio: anything that needs constraint
machinery to reassemble its halves was one value all along. The dishonesty was never
dependency-shaped; it was two columns impersonating one value, hiding the algebra
(overlap, containment, coverage) from the one layer that could enforce it.
**Decision:** Interval is a first-class structural type. **Alternative (strong):**
keep the two-I64 convention and add an exclusion-constraint kind over column pairs —
Postgres's pre-range-types design. **Why it lost:** it grows the constraint vocabulary
instead of the type vocabulary, leaves query semantics (membership, overlap joins)
unexpressed, and every consumer re-implements half-open discipline by hand. The
surveyed workloads (payroll periods, session validity, active-status lifetimes) are
interval-shaped in exactly the way money is scaled-integer-shaped. **Reverses if:**
never — pretending a first-class structure is two scalars is the lie this redesign
exists to stop telling.

## Fields: type + generation

A field is `(name, type, generation)` where `generation ∈ {None, Fresh}`. Generation
is a **storage behavior, not a type**, and the name is the mechanism's own: minting an
id is generating a **fresh existential witness** — exactly what the chase does when a
statement demands a value that does not exist. Postgres's word for this was the last
SQL survivor of the deleted vocabulary; it died in the algebra pass (PRD 01).

- A `Fresh` field must be `U64`. The database mints its values: monotonic per
  (relation, field), never re-issuing any value observable in a committed state;
  aborted transactions don't advance the committed sequence.
- **The usage pattern this exists for** — insert a new row without ever reading a max:

  ```rust
  db.write(|tx| {
      let id: AccountId = tx.alloc()?;             // mints the next AccountId value
      tx.insert(&Account { id, holder, status })?; // insert always takes complete facts
      Ok(id)
  })?
  ```

  (The typed surface infers the field from the `Fresh` newtype; the untyped form is
  `tx.alloc_at(witness)` with the witness resolved once through
  `schema.fresh_field(relation, field)` — `70-api.md` § ETL.)

  `alloc` is the only generator; `insert` is always full-fact and stays idempotent —
  one insert semantics, no generative variant.
- Explicit values are legal on the normal write path (not just ETL): inserting with a
  chosen value ≤ or > the high-water mark succeeds and advances the mark past it. This
  is load-bearing: correcting a fresh-keyed fact is `delete(old); insert(new with the
  same id)`. "Never reused" constrains the *generator* only — it never re-issues a
  value that was ever committed; explicit re-supply of a deleted value is legal. Mixed
  explicit/generated allocation within one transaction tracks the running maximum.
  A *successful* commit persists every fresh value it issued, even when no facts
  changed — the closure may have returned those ids to the host, and an observed id is
  never re-issued (the counters-only commit writes exactly the dirty `Q` marks: no
  generation bump, no cache eviction). Aborted transactions (`Err`/panic) still drop
  their allocations; nothing they minted was observably returned.
- **A Fresh field auto-materializes a functional dependency** — the statement
  `R(field) -> R`, first in the relation's materialized statement order, ordinary in
  every way and targetable by inclusions like any declared key. Two Accounts sharing
  an AccountId is unrepresentable by construction, with zero special enforcement
  machinery — the generator implies the invariant; the invariant is expressed in the
  one mechanism that owns invariants.
- Referencing fields are plain `U64` fields; referential intent is carried entirely by
  an explicit inclusion statement. There is no defining-vs-reference occurrence
  question — generation is a per-field-declaration attribute and nothing else.
- **Fresh ids order within their relation and nowhere else.** A fresh id is monotonic per
  (relation, field); comparing fresh ids across relations compares two unrelated mint
  sequences and means nothing. Where the application needs cross-relation "happened
  after", it stores an explicit I64 time or an Interval — the convention exists so
  nobody rediscovers uuid.

**Decision — three rulings close the generation question, permanently.**

1. **u64-only.** Enforced at declaration (`FreshOnNonU64`). A monotone counter over
   i64 has no sighting; the census law forbids the surface area. **Reverses if:** a
   sighted workload needs a signed mint sequence.
2. **Writable-by-default is load-bearing, not a leak.** Update is delete+insert, so
   re-inserting a fact writes its existing id back; ETL and `bulk_load` must preserve
   ids other facts reference. The SQL-standard `GENERATED ALWAYS` shape is
   incompatible with the engine's own update idiom. Explicit writes advance the
   high-water (`saturating_add`); exhaustion at `u64::MAX` is ~585,000 years at 10⁶
   allocs/sec — no guard beyond `FreshExhausted`. **Reverses if:** never —
   writability is a theorem of the update idiom, not a preference.
3. **Generation attribute, not a type.** A type is an encoding and the value's
   encoding *is* u64; a distinct engine type would smuggle nominal typing past the
   structural-typing law while duplicating what host newtypes already provide under
   rustc. **Reverses if:** never — philosophy.

**Alternatives, recorded:** (a) nominal `Fresh{type_name, owning_relation}` — lost to
ruling 3; (b) generative insert (insert mints and returns the id) — lost because it
splits insert into two semantics and breaks idempotence and the fact-hash membership
check.

## Relations are sets of facts; the fact is its own identity

- Every relation is a set of full, typed facts. Canonical membership is implicit for
  every relation; storage's row ids never surface into the logical model.
- **There is no primary key, and this is doctrine, not omission.** "Primary key" is a
  bag-semantics crutch: when duplicate rows are possible, some column set must be
  *appointed* to give rows identity. Here the fact is its own identity — identity is
  the canonical bytes (below). Keys (functional dependencies, `30-dependencies.md`)
  are invariants a relation *satisfies*, plural and unprivileged; an inclusion targets
  whichever key it names, and fresh is a value-minting convenience that happens to
  materialize one. Nothing anywhere appoints "the" key.
- `insert(fact)` is an idempotent no-op if the fact exists; `delete(fact)` is an
  idempotent no-op if it doesn't. Both report whether they changed state. **There is
  no update operation**; mutation is delete + insert (within one write transaction),
  with explicit fresh re-supply preserving identity across the swap.
- Nullary (zero-field) relations are legal: a set that is empty or contains the single
  empty fact — a database-level boolean. Every layer (encoding, hashing, IR) defines
  behavior for it because it falls out of the representation.

**Decision: no primary keys.** **Alternative:** entity relations with appointed PKs
and whole-row `replace`. **Why it lost:** one identity concept (the fact), one mutation
algebra (insert/delete), no PK-vs-key duality. Consequences now explicit: mutating a
referenced fact needs dependency-timing rules (commit-time, against the final state —
`30-dependencies.md`) and fresh re-supply (specified above). **Reverses if:** the
delete+insert idiom proves unlivable in real app code.

## No nulls

Null does not exist anywhere: not in storage, the IR, results, or aggregation.
**Optional data is an absent fact in a 0..1 child relation**, and the idiom is now
fully load-bearing: the child's key plus a one-way inclusion back to the parent *is*
"nullable column" done honestly (`30-dependencies.md` owns the pattern; the surveyed
workloads' nullable-FK state machines and all-or-nothing nullable column groups are
both this idiom wearing bag-world disguises). Load-bearing consequences: negation is
plain anti-join; empty aggregate groups don't appear; no operator anywhere has a null
branch.

## Fact identity: the canonical encoding

This document owns fact equality. **Value equality ≡ `fact_bytes` equality**, where
`fact_bytes` is the concatenation of each field's canonical encoding **in declaration
order**, with **no padding between fields** — facts are dense (1-byte enums/bools sit
flush against 8- and 16-byte fields; Apple Silicon's near-free unaligned loads make
intra-row alignment a pure waste, `00-product.md`). Canonical means injective and
unique: Bool is strictly 0/1 (any other byte is corruption, never a distinct "true");
Enum is the declaration-order ordinal; integers are their order-preserving encodings;
Interval is `start ‖ end` with `start < end` (violation is corruption);
String/Bytes are their intern ids (one byte sequence ⇒ exactly one id, ever).
Storage (`50-storage.md`) implements membership as blake3-256 of `fact_bytes`;
**hash equality is treated as fact equality — collisions are an accepted axiom**
(2⁻¹²⁸-scale event), not verified against, and the same axiom applies to the
dictionary's content hash. Recorded once, here.

**Decision: blake3, full 32 bytes, as the identity hash.** The distinction that
decides it: this hash is **content-addressed identity**, not a corruption
checksum — the `M` probe and the dictionary resolve by hash with no byte
verification, so a collision is not a detected error but two distinct facts or
strings silently unified, and external-world strings enter the dictionary, so
the contract requires cryptographic collision resistance against chosen inputs.
And the hash is on no measured hot path: it runs once per touched fact per write
op (deduplicated by design), once per novel interned string, once per open — the
write path is fsync-bound (~100 ns of hash against milliseconds of commit) and
the query path never hashes (queries compare intern words). **Alternative
(strong):** AEGIS-128L, per TigerBeetle's storage checksums — severalfold faster
on AES-accelerated hardware including Apple Silicon. **Why it lost:**
TigerBeetle's adversary is bitrot (integrity), ours is unification (identity);
an unkeyed 128-bit AES-round tag carries no collision-resistance claim, and
under the shipping law no swap can cite a number while every swap breaks the
set-in-stone format (`M` keys, dict keys, fingerprint). **Reverses if:** never
for AEGIS (contract, not cost); **hardware SHA-256** (ARMv8 crypto extensions,
collision resistance intact, faster than blake3 at small inputs on this
hardware) only on a measured write-path CPU bottleneck. Synergy noted: blake3 is
one of the two sanctioned engine deps and frozen upstream — the hash-stability
risk TigerBeetle solved by vendoring does not exist here.

Note the denotation asymmetry, stated so nobody trips on it: *identity* is bytes —
`(x, [1,5))` and `(x, [1,8))` are different facts — while *dependency judgments and
membership queries* read intervals through the denotation. Both layers are exact;
they answer different questions ("is this stored?" vs "what does it mean?").

## Interning

One global dictionary for String and Bytes, segregated by a type-tag byte inside the
hashed key (a String and a Bytes with identical bytes get distinct ids). Forward map:
blake3(tag ‖ bytes) → id (collision axiom above); reverse map: id → bytes. Ids are
monotonic, never reused, append-only; interning happens only inside write transactions.
Strings are validated UTF-8 at intern time (parse, don't validate). On the read path,
query literals resolve by read-only lookup — a dictionary miss means the literal cannot
match any fact: **empty result, never an insert, never an error** (resolved
per-execution; see `20-query-ir.md`). Known accepted limitation: no GC — deleted
facts leak their interned values, and a value interned for an insert that turned
out to be a storage no-op leaks even though no committed fact ever referenced it
(pending interns flush with any state-changing commit; filtering them would be
commit-path machinery spent against an accepted leak). At design scale both
classes are noise (revisit trigger recorded in the README OPEN list, counting
both).

Consequence: string equality is cheap (id compare); string ordering, prefix search, and
text search are unsupported and would require explicit ordered text indexes if ever
wanted.

**Decision: interning.** **Alternative:** inline variable-length or fixed-prefix string
encodings preserving order. **Why it lost:** interning keeps facts fixed-width (O(1)
column slicing — the load-bearing property of the whole storage design) and makes
equality one word-compare; the ledger workload orders by time and amount, not by string.
**Reverses if:** string range/prefix queries become a real workload need (then: ordered
text indexes, not de-interning).

## Dependencies

Owned entirely by `30-dependencies.md`. The one sentence this chapter contributes:
**dependencies are judgments about queries, checked once at commit against the
transaction's final state** — there are no constraint modes, no per-operation
enforcement, no deferral opt-in, and the words *unique*, *foreign key*, *primary key*,
*cascade*, and *restrict* do not name anything in this system.

## Schema

**A schema names a theory; a store models it.** A schema is a presentation of a
theory — relations plus statements — and a database is a model of that theory; the
`Theory` trait (`descriptor(self)`) is the definition surface the macro's header
struct implements, with a runtime-built `SchemaDescriptor` as its own definition
(`70-api.md`).

Schemas are declared in Rust and compiled into the binary. The declaration produces
descriptors, the host-side newtypes, and a canonical byte encoding hashed (blake3)
into the **schema fingerprint**, stored at database creation; open compares fingerprints
and mismatches are hard failures. No migration, no ALTER: schema change = ETL into a new
database (export surface: `70-api.md`).

**Fingerprint inputs, exhaustively:** an encoding-format version label; relations in
declaration order — for each: name and fields in declaration order (name, structural
type description — including the full ordered variant list for enums and the element
type for intervals — and generation flag); then the **dependency statements in
materialized order** — for each: the judgment form (functionality or containment,
with direction count) and both sides' (relation id, projection field-id list in
statement order, selection list as (field id, literal value) pairs in statement
order). Materialized order = the fresh auto-keys first (one per fresh field, in
relation-then-field declaration order), then the declared statements in declaration
order — a deterministic function of the declaration, so statement ids remain pinned
by the fingerprint without being hashed separately. Relation and field ids are plain
declaration order; statement ids are materialized order, schema-global.
Stated consequence, accepted: **adding an enum variant changes the fingerprint — a
full ETL rebuild.** Closed domains are closed.

**Decision: schema lives in Rust.** **Alternative:** external schema declaration
format. **Why it lost:** the schema must generate typed Rust API anyway and there is one
consumer; a second language is a parser plus a sync problem for nobody. **Reverses if:**
never, absent a second consumer.

## The modeling discipline (BCNF by discipline, temporality by type)

Natural n-ary relations for domain facts; natural edge relations (`OrgParent(child,
parent)`) welcome; enums for closed domains instead of two-row lookup tables;
**intervals for validity, sessions, periods, and lifetimes** instead of
start/end column pairs or status-plus-nullable-timestamp machines; optional
attributes as 0..1 child relations (the no-nulls idiom above); sum-typed domain
entities as a discriminator enum plus per-variant child relations glued by
bidirectional conditional inclusions (`30-dependencies.md` derives the pattern and
its theorems). Forbidden by construction: nullable columns, JSON blobs; forbidden by
discipline: EAV, denormalized redundancy. History is immutable event facts or
interval-stamped facts — never mutable columns; transitive closures are precomputed
relations maintained by the host (recursion is not coming to the query layer to
save a modeling shortcut).

**The idioms, recorded once** (each a representation choice, not a library):

- **Money** — i64 minor units, host newtype per currency and scale; `Sum` runs
  i128-checked so ledger totals cannot wrap silently.
- **Time** — i64 UTC microseconds, signed on purpose: payroll birthdates predate
  1970. Dates are i64 days since epoch; both are conventions the host newtypes.
- **Order** — explicit position columns, never successor pointers: a linked list
  inside a relation is control flow smuggled into data, and every reorder becomes
  a dependent chain of writes.
- **Any/All** — `Max`/`Min` over a bool column: the 0/1 encoding makes the two
  quantifiers the two extremes; no dedicated operators.
- **Large content** — facts stay fixed-width; big payloads live in external
  storage referenced by identity, with content churn recorded on the
  dictionary-GC OPEN item as its trigger profile.
