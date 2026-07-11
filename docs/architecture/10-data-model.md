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
Bytes(N), N ∈ 1..=64     ⌈N/8⌉ × 8 bytes in facts: the N raw bytes, zero-padded
                         to the word boundary — the pad is encoding, not data
                         (a nonzero pad byte is corruption); never interned
Interval(element)        16 bytes: start ‖ end, each the element encoding;
                         element ∈ {U64, I64}; strictly start < end
```

Seven types (`bytes<N>` replaced variable `bytes` — the roster stays at seven; the
width is part of the type, so `bytes<16>` and `bytes<32>` are different types and a
width change is a new theory, fed to the fingerprint). Type equality is **structural
equality of the description**; unification, comparison legality, and dependency
compatibility are all just that equality. There is no `name` field anywhere in the type
layer.

**The decision rule for byte-shaped data: intern what repeats; inline what
identifies.** `str` is the reuse-shaped population (names, labels: low cardinality,
high reuse) — content-addressing is compression and id-equality is the win. `bytes<N>`
is the identity-shaped population (content hashes, external opaque ids: maximal
cardinality, near-zero reuse) — the value lives *in the fact*, exactly as the engine's
own `M` namespace stores its 32 inline blake3 bytes, uninterned. One law, uniform
across engine and schema; the two byte-shaped types share no axis (variable/fixed,
interned/inline, text/raw, reuse/identity). Variable-width *binary* with genuine reuse
had zero sightings in either deep-port target — a type without a population is
symmetry, not design; that cut reverses if a real schema surfaces one (the dictionary
machinery it would need survives intact under `str`). N ≤ 64: 64 bytes = 8 words = two
cache lines of key material; digests in the wild are 16/20/32/64. Not `fresh`-eligible,
not an interval element; the host type is `[u8; N]` — owned, `Copy`, borrow-free.

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
range predicates). Interval supports **equality, the `Allen` mask (the whole
interval-pair algebra as one comparison — `20-query-ir.md` § the Allen
operator), and point membership** (below) — never `Lt`-family order or
`Min`/`Max`: the value order that
exists (lexicographic by start) is an encoding accident, and offering it would invite
queries that mean intersection and say "less than". Everything else is equality-only. Enum
ordinal order is a declaration-order accident, not semantics; String intern ids
are meaningless to order; Bool ordering is noise. **`bytes<N>` is identity-only by
refusal** (`Eq`/`Ne` and membership; order comparisons and `Min`/`Max` are typed
validation errors): a digest's lexicographic order is an encoding artifact, and
admitting it would make hash-function choice semantically visible. The guard B-tree
still sorts the padded encodings — sortedness is the index's need, not a query
semantics (the padded bytes memcmp in value-byte order, which is all the guard asks).

**The mask value shape:** the interval-pair relation itself is a value —
`AllenMask`, a 13-bit word, bit *i* = Allen basic *i* in the palindromic order
(before, meets, overlaps, starts, during, finishes, equals, finished-by,
contains, started-by, overlapped-by, met-by, after), so the algebra's converse
involution is the 13-bit reversal. It is **not a field type** — nothing stores
a mask; the roster stays at seven — it exists so the temporal relation can be
a bind-time argument (`Value::AllenMask` / `BindValue::AllenMask`,
`20-query-ir.md`).

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

An `Interval` value `[s, e)` is **a set of points, written as its bounds** —
half-open over the element domain, `s < e` enforced at the encoding boundary exactly
as Bool's strict 0/1 is (a stored `s ≥ e` is corruption, and the empty interval is
unrepresentable: a fact never denotes nothing). Half-open and nonempty are not house
conventions but **Allen's algebra's preconditions**: the 13 basic interval relations
are jointly exhaustive and pairwise disjoint (JEPD) only over nonempty intervals —
an empty interval satisfies none of them cleanly — and *meets* (`a.end == b.start`,
no shared point) is only well-defined half-open; closed intervals would make meeting
and overlapping collide at the boundary point. Encoding: `start ‖ end`, each in the
element type's order-preserving encoding, so the 16 bytes sort lexicographically by
start — the property the storage layer's neighbor probes stand on (`50-storage.md`).

**The denotation rule, normative:** a fact whose interval field holds `[s, e)` *means*
the family of point-facts obtained by replacing the interval with each `t`, `s ≤ t < e`.
Everything temporal in this database is a corollary of this sentence and owns no
machinery of its own:

- A **functional dependency over an interval position holds pointwise** — no two facts
  in the key group may share any point: every pair satisfies `DISJOINT`, the
  Allen composite (`30-dependencies.md`, `20-query-ir.md` § the Allen
  operator). SQL:2011 spells this `WITHOUT OVERLAPS` and ships it as a
  keyword; here it is not an option but what the judgment *means* on this type.
- An **inclusion dependency over an interval position holds pointwise** — every point
  of the source's interval is covered by the target's intervals (SQL:2011's `PERIOD`
  foreign keys). The target needs a pointwise key for this to be checkable in
  logarithmic time; validation demands it (`30-dependencies.md`).
- **Point membership is a typing rule, not syntax**: a query atom binding an
  interval-typed field with an element-typed term means `t ∈ interval`
  (`20-query-ir.md`).

**The point-domain law, normative:** the point domain of each element type is
`MIN ..= MAX−1`, and `end == MAX` **denotes the unbounded ray** `[s, ∞)`. ∞ is a
value of the representation, not a hack around it: ongoing employment, the top tax
bracket, and until-forever recurrence are honest values (`Interval::ray(start)`
names the constructor; `is_ray()` the predicate; `new` admits `end == MAX`
directly — the ray is a name, not a mode). The zero-cost claim is the encoding,
not hope: both element types store order-preserving unsigned words (I64 is
sign-flipped), so ∞ = MAX participates in every unsigned comparison kernel with no
special case — there is no branch to take, and no judgment or interval predicate
needs ray awareness. Consequences, typed rather than left to be discovered:

- An **element-typed literal or param equal to the domain ceiling is an error
  wherever it meets an interval position** (membership bindings, `Contains`
  operands): a typed validation error for literals, the matching typed bind error
  for point params — never a silently-unmatchable query. Parse, don't validate.
- A **ray has no finite measure**: `Duration` over a ray is the typed execution
  error `MeasureOfRay` — the one runtime type error in the engine, since
  boundedness is not provable at validation. The alternative — silently yield
  MAX — fabricates arithmetic.
- **Coverage judgments over rays**: a source ray requires target coverage to ∞,
  satisfiable only by a target chain reaching a ray; the coverage walk's ordinary
  gap check enforces it with no special case.

The alternative — first-class ∞ — buys a 17th byte or a stolen sentinel anyway,
and changes nothing the neighbor probe can observe.

**The denotation defines exactly one arithmetic, and `Duration` is it:** a
point set has a measure, `|[s, e)| = e − s`, u64-valued for both element
types — and everything else that looks like interval arithmetic is endpoint
math and stays refused (the README refusals; `Duration` is not the thin end
of a wedge, it is the entire wedge, provably: the denotation defines nothing
else). The measure's query positions and the ray error's placement are
`20-query-ir.md` § the measure.

**Coalescing is an aggregate, never a write rule.** Two facts `(x, [1,5))` and
`(x, [3,8))` are distinct facts whose denotations overlap; the engine stores what it
was given (identity is bytes, below). The packed canonical form — maximal disjoint
intervals per group, the temporal literature's *coalesce*, Postgres's `range_agg` —
is an aggregate (`Pack`, `20-query-ir.md` § aggregation). A relation that *wants*
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

## Closed relations: ground axioms

A **closed relation** declares its extension in the schema:
`RelationDescriptor { name, fields, extension: Option<Extension> }`, where
`Some(rows)` is the kind — there is no relation-kind enum; the option *is* it. Its
rows are **ground axioms** — atomic sentences of the theory. A schema was
*signature + axioms* where every axiom was a universally quantified statement; a
closed relation gives the theory constants, and vocabularies stop being a type to
become what they always were relationally: unary-plus-payload relations with a
fixed extension.

- **Identity is the handle.** Each row declares a handle (`Usd`, `Q1`); its row id
  is the declaration index — exactly the declaration-order rule relations, fields,
  and statements already obey. The handle is NOT a column: the sealed relation
  opens with a synthetic first field (`id`, U64), so guards, statements, and
  queries address the id uniformly at field 0; the macro never lets the user
  declare it (a hand-built descriptor that tries collides on the field name).
- **The auto-key.** Closedness materializes `R(id) -> R` exactly as `fresh` does
  (materialized order below) — ordinary in every way and targetable: a reference
  to a closed relation is a plain u64 column plus a declared containment, like any
  reference. Nested closed-to-closed references are the same shape — no narrow
  encoding arm, ever (`docs/prd-comptime/README.md`, the refusal).
- **Intrinsic columns are value types only**: U64, I64, Bool, `bytes<N>`,
  Interval. `str` is refused — the handle IS the label and the renderer prints
  handles from the theory; interned columns on a virtual relation would force
  dictionary writes at open, breaking "the store contains zero vocabulary bytes".
  `fresh` is refused — identity is the handle; axioms are never minted.
- **The extension is validated at declaration and frozen by the fingerprint**:
  distinct handles; per-column typing through the one shared value check; interval
  axioms obey `start < end` (the constructor law holds for axioms too — a
  malformed ground axiom is a schema error, not corruption); 1..=256 rows (an
  empty extension is a vocabulary of nothing — write no relation; a larger one is
  policy data wearing a vocabulary costume). Values are canonically encoded ONCE,
  at validate — the sealed rows carry fact bytes and are never re-encoded (the
  staging law applied to the feature itself).
- **Writes are refused.** Any delta operation naming a closed relation —
  insert/delete, typed or dynamic, `bulk_load`, `alloc` — is the typed
  `ClosedRelationWrite`, checked at the write-surface entry before any encoding
  runs. The store holds no rows for a closed relation, and the sweeper
  (`verify_store`) convicts any `F`/`M`/`U`/`R` entry naming one as corruption.

**The intrinsic-vs-policy law, normative.** Intrinsic properties of a vocabulary
entry — what makes it *what it is* (a currency's minor-unit count, a quarter's
span) — go on the closed relation: changing one is a new theory, and the
fingerprint says so. Policy *over* a vocabulary — what the application currently
decides about it (which currencies are enabled, which quarter is open) — lives in
ordinary relations referencing the handle's id and changes by witnessed write. A
vocabulary that must drift without a rebuild was never a vocabulary: declare an
ordinary relation (the open-extension refusal, `docs/prd-comptime/README.md`).

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
String is its intern id (one byte sequence ⇒ exactly one id, ever); `bytes<N>` is its
N raw bytes zero-padded to the word boundary (nonzero pad is corruption).
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

**The dictionary is the compression representation for repeated text.** It is
str-only: `bytes<N>` values are inline in facts and never touch it (*intern what
repeats; inline what identifies*, above), so the key hash carries no type tag —
with one interned type there is nothing to segregate. Forward map:
blake3(bytes) → id (collision axiom above); reverse map: id → bytes. Ids are
monotonic, never reused, append-only; interning happens only inside write transactions.
Strings are validated UTF-8 at intern time (parse, don't validate). On the read path,
query literals resolve by read-only lookup — a dictionary miss means the literal cannot
match any fact: **empty result, never an insert, never an error** (resolved
per-execution; see `20-query-ir.md`). Known accepted limitation: no GC — deleted
facts leak their interned values, and a value interned for an insert that turned
out to be a storage no-op leaks even though no committed fact ever referenced it
(pending interns flush with any state-changing commit; filtering them would be
commit-path machinery spent against an accepted leak). The leak is scoped to the
population interning compresses — repeated text; the digest population that would
have turned it into an unbounded tax left the dictionary with variable `bytes`.
At design scale both classes are noise (revisit trigger recorded in the README
OPEN list, counting both).

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
type for intervals — and generation flag), then the closedness tag (ordinary = 0;
closed = 1 followed by the ground axioms in declaration order — handle, then the
row's canonical fact bytes); then the **dependency statements in
materialized order** — for each: the judgment form (functionality or containment,
with direction count) and both sides' (relation id, projection field-id list in
statement order, selection list as (field id, literal value) pairs in statement
order). Materialized order = the fresh auto-keys first (one per fresh field, in
relation-then-field declaration order), then the closed auto-keys (one per closed
relation, in declaration order), then the declared statements in declaration
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

### Derived relations (the view story)

SQL's *view* is one word for two different things, and each is answered with
machinery this design already has. No engine surface exists or is coming; the
precomputed transitive closures the discipline blesses above are this section's
standing instance.

**Virtual views are host-level IR composition — a view is a function returning
atoms.** Queries are plain data (`20-query-ir.md`), so the composition layer is
the host language: a derived predicate is a Rust function returning IR fragments
(atoms, predicates, rule bodies) that callers splice into their queries. Worked,
from the calendar theory (`60-validation.md`):

```rust
/// A person's busy claims — the one place `arm == Busy` is spelled.
fn busy_claims(person: VarId, span: VarId) -> Atom {
    Atom { relation: CLAIM, bindings: vec![
        (CLAIM_PERSON, Term::Var(person)),
        (CLAIM_ARM,    Term::Literal(Value::Enum(ARM_BUSY))),
        (CLAIM_SPAN,   Term::Var(span)),
    ] }
}
```

One fragment, three of the family's queries, three positions: `busy_scan` takes
it as the positive atom under `Allen` against the param window; `conflict_free`
pushes the *same* fragment into `negated` with a point-membership binding
(negation is a position in the query, not a kind of atom — `20-query-ir.md`);
`free_busy` folds its span variable under `Pack`. Change what "busy" means —
an added arm, an added guard — and every consumer follows at the next compile.
**Refusal, permanent (`docs/prd-algebra/README.md`): no named-view registry in
the engine, ever.** A registry would be a second schema with none of the
theory's guarantees — names resolved at run time, fragments outside the
fingerprint, no typing fixpoint until use — while rustc already polices the
real one: functions have names, types, visibility, and dead-code warnings.

**Materialized views are a relation plus statements — strictly stronger than
SQL's.** Materialize derived data into an ordinary relation and *state* its
relationship to the sources; the commit judgment (`30-dependencies.md`) then
decides which lies can never be stored. Worked, `Pack`-fed (`20-query-ir.md`
§ aggregation):

```rust
relation BusySpan { person: u64, span: interval<i64> }
BusySpan(person, span) -> BusySpan;                           // packed ⇒ disjoint: statable
BusySpan(person, span) <= Claim(person, span | arm == Busy);  // soundness, pointwise
```

The `<=` reads through the denotation: every point of every stored span is
covered by that person's busy claims, so an **unsound** materialization —
claiming busy time that isn't, or surviving its sources' deletion — is
**uncommittable**, judged on every commit that touches either side. Incomplete-
until-refresh stays representable; that is what a refresh window *is*, and the
direction is the dial: where the derivation is exact and the host commits to
same-transaction maintenance, `==` (gate permitting — the reverse projection
must target a pointwise key, as here) makes *any* divergence uncommittable —
the discriminated union's totality theorem replayed for derived data. SQL
matviews invert every default: stale silently, in both directions, and
`REFRESH` is a prayer. Here the host maintains and the engine judges;
maintenance is the generation-witness idiom verbatim (`70-api.md`
§ conditional writes, the third idiom): query the sources on a snapshot →
recompute (`Pack` is the coalesce) → diff → `write_from` with that snapshot as
the witness — the derived relation cannot commit against sources it didn't
actually read (`GenerationMoved` otherwise).

**The honest limit: statements prove presence and topology, never arithmetic
agreement.** Containment proves every derived row justified and — reversed —
every source represented; keys prove shape; selections pin arms; pointwise
lifting proves coverage. What no statement can say is that a *value* equals a
*computation* over its sources: the calendar's `Attendance(id | rsvp ==
Accepted) == Claim(source | arm == Busy)` proves every accepted attendance has
its busy claim and cannot add "…and the claim's span equals the attended
event's span" — copied intervals and summed balances are computations, outside
the ∀∃ vocabulary by the acceptance gate (`30-dependencies.md`: statements are
projections and literal selections; expression agreement has no O(log n)
enforcement plan). **Refusal, recorded (`docs/prd-algebra/README.md`): no
arithmetic-agreement statements.** The answer is host discipline — one function
owns each derivation, which the composition idiom above makes natural — plus,
where wanted, an offline `verify_store`-grade re-derivation: re-run the
deriving query on a snapshot and compare against the stored relation, the same
posture the store's own integrity sweep takes (`60-validation.md`). *Trigger:*
a sighted agreement invariant that host discipline plus offline re-derivation
demonstrably fails to hold; the candidate form would be projected copy-equality
across a containment's sides — never expression evaluation.

**Deleted vocabulary** (rows in `00-product.md`): *view* → a function returning
atoms; *materialized view / refresh* → a relation under statements, maintained
by witnessed writes.
