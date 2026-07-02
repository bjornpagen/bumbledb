# Audit: 10-data-model.md

Auditor note: the post-mortem dump referenced as `todo/` at `1b65ae8^` does not exist in
any commit of this clone (verified by scanning every tree in history); lessons were taken
from the architecture docs' own history notes and the project memory summaries instead.

## 1. **Serial generation contradicts the idempotent full-fact insert; the write API for serial-bearing relations is undefined** [blocker]

The doc's entire mutation algebra is "`insert(fact)` is an idempotent no-op if the fact
exists," which presumes the caller presents a *complete* fact. But Serial values are
"database-generated monotonic `u64` sequences per declared serial field" — so either (a)
insert takes a fact *without* the serial value and generates one, in which case inserting
"the same" fact twice generates two values and produces two facts, destroying idempotence
and the "one mutation algebra" claim; or (b) insert takes the full fact including the
serial, in which case generation must be a separate, unspecified operation (a
`next(AccountId)` call? a returning insert variant?) that the doc never mentions. An
implementer cannot write the insert signature for a relation with a serial field from
this document. This is not covered by the OPEN FK-mutation corner — it exists for a
serial-keyed relation with no FKs at all.

Question: What exactly does `insert` take for a relation with a generated serial field —
a complete fact (and if so, what operation mints the serial value and what are its
transactional semantics), or a partial fact (and if so, how is idempotence defined)?

## 2. **Fact equality and the canonical fact encoding are never defined, yet everything is built on them** [blocker]

"Every relation is a set of full, typed facts" and insert/delete are defined by whether
"the fact exists" — but the doc never says what makes two facts equal, and storage
(`40-storage.md`) implements membership as `blake3` of `fact_bytes`, so equality is
*byte* equality of an encoding this doc does not specify. For byte equality to coincide
with value equality, every encoding must be canonical and injective: Bool must be exactly
0/1 (any other byte is an unequal "true"), enum variants need a defined variant→byte
mapping, field concatenation order must be fixed (declaration order?), and interned
strings must map one byte sequence to exactly one id. None of this is stated in the
document that owns identity; `40-storage.md` describes the layout but neither doc claims
ownership of "the canonical fact encoding is fact identity." The task's question "are two
facts with the same interned string always byte-identical?" is answerable only by
guessing that the dictionary is global and write-time — see finding 10.

Question: Will 10-data-model.md own a normative definition of fact equality — canonical
encoding per type (Bool 0/1 only, enum variant numbering, field order), with the
statement that value equality ≡ `fact_bytes` equality — so storage merely implements it?

## 3. **Explicit serial supply is silently load-bearing for the no-update model, and duplicate serial values are representable** [blocker]

The doc frames explicit serial values as an ETL affordance: "ETL may supply explicit
values, which advance the high-water mark." But under "mutation is delete + insert," the
*only* way to correct a non-key field of a serial-keyed fact is to reinsert with the old
serial supplied explicitly — so explicit supply must be a normal-path API feature, not an
ETL side door, and the doc's framing contradicts its own mutation story. Worse, "values
are never reused" is ambiguous against this: is reinserting a deleted fact's serial
"reuse" (forbidden, making delete+reinsert mutation impossible) or allowed (making "never
reused" only a statement about the generator)? And nothing says a serial field carries an
implicit unique constraint — without one, two Account facts sharing one `AccountId` are a
representable illegal state, a direct philosophy violation the no-PK decision paragraph
does not confront.

Question: Is explicit serial supply legal on the normal write path; does supplying a
value ≤ the high-water mark (including a previously deleted value) succeed; and is a
declared serial field implicitly unique in its owning relation, or must the schema
declare that separately?

## 4. **Unique-constraint enforcement timing is undecided but not marked OPEN, and it is entangled with delete+insert mutation** [design-gap]

Only the FK gets "Enforcement timing is `OPEN` (see README)"; the Unique bullet says
nothing about timing, and the README's OPEN list mentions only "FK enforcement timing."
But the same question bites Unique harder: with "mutation is delete + insert," a
per-operation unique check forces the application to order delete-before-insert inside
the transaction (insert-first transiently violates the constraint), while commit-time
checking makes ordering irrelevant. Since delete+insert is the *only* mutation story, the
model has silently acquired either an operation-ordering obligation or a deferred-check
semantics — exactly the kind of consequence the no-PK decision paragraph should have
recorded.

Question: Is unique enforcement per-operation or commit-time, and if per-operation, is
"delete before insert within a transaction" a documented obligation of the mutation
idiom?

## 5. **The schema fingerprint's inputs are unspecified, as is the origin of relation/field ids** [design-gap]

"The declaration produces relation/field/constraint descriptors and a canonical byte
serialization hashed (blake3) into a **schema fingerprint**" — but what is serialized is
never enumerated: relation names? field names and order? enum variant lists and their
order? serial `type_name`/`owning_relation`? constraint names, field order, and targets?
the encoding format version? Every inclusion decision has a real consequence under "no
migration": if enum variant lists are included (they must be, since they define the
1-byte encoding), adding one variant means a full ETL rebuild — a heavy consequence the
doc never acknowledges. Separately, the IR and storage speak `RelationId`/`FieldId` and
`relation_id | field_id` keys, but no doc says how those dense ids are assigned
(declaration order?) or whether they are fingerprint inputs; the v1–v5 post-mortem lesson
about carrying "names AND ids everywhere" demands a single stated rule for where names
end and ids begin.

Question: Exactly which descriptor fields feed the canonical serialization, how are
relation/field/constraint ids assigned and are they covered by the fingerprint, and is
"adding an enum variant = new database via ETL" the intended and accepted consequence?

## 6. **No per-type comparability/orderability matrix; Min/Max and range predicates are undefined for half the roster** [design-gap]

`20-query-ir.md` rejects "comparisons over non-orderable types" and defines "Min/Max over
the value types' total order," but this doc — which owns the types — never says which
types are ordered. Interning explicitly kills string ordering ("string ordering, prefix
search, and text search are not supported"), so `Lt` and `Min/Max` over String must be
rejected; is Bytes the same? Is Enum ordered (by variant index — an accident of
declaration order), or equality-only? Is Serial ordered (`Lt` over nominal ids is
semantically dubious, yet monotonicity makes Min/Max tempting)? A ledger's
"time-range scans" only work because I64-convention timestamps are ordered; the doc
should state the full matrix rather than leaving the IR validator to invent it.

Question: For each of Bool, U64, I64, Enum, String, Bytes, Serial — which support
ordering comparisons and Min/Max, and which are equality-only?

## 7. **Serial's defining occurrence vs reference occurrences is unspecified** [design-gap]

Serial is declared as `Serial { type_name, owning_relation }` with "sequences per
declared serial field," and the FK rule requires "positional exact-type compatibility" —
so `Posting.account` must itself be typed `AccountId` to reference `Account`. But then
`Posting.account` is also "a declared serial field": does it get its own sequence
(clearly wrong), or does the sequence exist only for the field in the `owning_relation`?
Nothing states that rule, nor whether a serial type may have multiple defining fields,
whether the owning relation may omit the field entirely, or what happens if a serial type
appears in a relation with no FK back to its owner. Storage's `Q | relation_id |
field_id -> next_u64` keying suggests per-(relation, field) sequences, which makes the
ambiguity live.

Question: Is the rule "exactly one defining field — the declared serial field in
`owning_relation` — owns the sequence; all other occurrences are references with no
generator," and is that structurally enforced at schema declaration?

## 8. **Enum identity and canonical form are underspecified** [design-gap]

`Enum { name }` — "1 byte, closed domain declared in schema" is the entire specification.
Unstated: whether enums are nominal like Serial (two enums with identical variant sets
never unify — presumably, but Serial got an explicit sentence and Enum did not); what
defines the variant→byte mapping (declaration order?); what the variant's *identity* is
for the fingerprint and for IR literals (name, or index?); and the hard 256-variant cap
implied by "1 byte" with no stated behavior at the limit. Since fact bytes, the
fingerprint, and query literals all depend on variant numbering, "closed domain declared
in schema" is not enough for an implementer.

Question: Are enums nominal; is the canonical variant encoding its declaration-order
index; is the variant name (not index) its identity in schema and IR; and is >256
variants a schema-declaration error?

## 9. **Membership-by-hash makes blake3 the de facto fact identity — an undocumented decision** [design-gap]

This doc says "there are no hidden row identities in the logical model," yet storage's
membership table is `M | relation_id | fact_hash -> row_id` keyed by "blake3 of
fact_bytes" with no stated verification against the stored fact — so two distinct facts
that collide would silently alias, and idempotent insert would drop a real fact. At
256 bits this is a perfectly defensible accepted risk, but it is an *identity* decision
and identity belongs to this doc; the dictionary got its "equality verification on
lookup" sentence while fact membership got nothing. Per the README rule, this needs
either a stated "hash is identity, collisions accepted, here's why" or a stated
verify-on-hit rule.

Question: Is fact membership hash-only (collisions accepted as a documented
non-event) or verified against `fact_bytes` on lookup, and which doc owns that decision?

## 10. **Interning semantics have unanswered structural questions** [design-gap]

The interning section specifies the maps and monotonic ids but not: whether the
dictionary is global or per-relation (storage's single `_dict` implies global — this doc
should say so, since fact byte-identity across relations depends on it); whether String
and Bytes share one dictionary (same bytes, same id?) or are segregated; whether String
is validated UTF-8 at intern time (parse-don't-validate says yes; nothing says it);
and the read-path rule — a query comparing a String field to a literal must do a
read-only dictionary lookup inside a read transaction, where "not present" means
unsatisfiable, never an insert. Concurrency is presumably "single writer owns all dict
inserts, readers see snapshot," inherited from LMDB, but the section is silent while
promising "equality is cheap (id compare)" — which is only true if all of the above hold.

Question: One global dictionary shared by String and Bytes or segregated per type;
is UTF-8 validity enforced at intern time; and is query-literal interning defined as
read-only lookup with lookup-miss ≡ empty result?

## 11. **Interning is a major decision with no recorded alternative** [clarification]

The README's first rule is "every decision records its strongest alternative and why it
lost," and the no-PK, type-roster, and schema-in-Rust decisions all comply — but
interning (a decision with permanent consequences: no string ordering, no prefix search,
dictionary leak, an extra indirection on every string read) has no
Decision/Alternative block. The strongest alternative (inline variable-length or
fixed-prefix string encoding, keeping order-preserving comparison) is exactly what the
"Consequence" paragraph is implicitly arguing against.

Question: Can the interning section get its Decision block — inline/order-preserving
string encodings as the named alternative and why they lost?

## 12. **Constraint field-order and name-scoping rules are unstated** [clarification]

`Unique { name, fields }` — is `fields` ordered? For uniqueness semantics order is
irrelevant, but the FK's "positional exact-type compatibility" against a "named unique
constraint" makes the target's field order load-bearing, so it must be declared
significant (and duplicate-field or empty `fields` rejected). Constraint `name` scope is
also unstated — per-relation or database-global? — and `target_constraint` resolution
depends on the answer.

Question: Are constraint field lists ordered and duplicate/empty-free by construction,
and are constraint names scoped per relation?

## 13. **The convention list is not normative enough for the workload it must carry** [clarification]

Timestamps and Money get one-line conventions, but the ledger workload (00-product:
"time-range scans," balance aggregates, accounting periods) also needs dates,
periods/intervals, and quantities — none enumerated, and until the OPEN nominal-domains
proposal lands, two unrelated I64 conventions "unify silently in a query," which the doc
itself identifies as the hole. If conventions are the answer, the data model should carry
the owner's canonical convention table (e.g. Date = I64 days-since-epoch?) so five
applications don't invent five encodings the typechecker cannot distinguish.

Question: Should the doc enumerate the full set of blessed conventions (date, interval,
quantity) now, and does the nominal-domains OPEN item block real ledger schemas until
decided?

## 14. **"Aborted transactions don't advance the committed sequence; values are never reused" reads as a contradiction until unpacked** [clarification]

If an aborted transaction drew values 5–7 and the committed sequence did not advance, the
next transaction hands out 5 again — which is fine (the aborted values never existed)
but is literally "reuse" of the numbers. The intended invariant is presumably "no value
observable in any committed state is ever generated again," and it interacts with
finding 3's explicit-supply question (is a *deleted* committed value re-insertable?). One
precise sentence would kill the ambiguity.

Question: Is the invariant exactly "the generator never re-issues a value that was ever
part of a committed state," with explicit re-supply of deleted values governed
separately?

## 15. **Nullary (zero-field) relations: legal or not?** [clarification]

"Optional data is an absent fact in a separate relation" naturally drives schemas toward
narrow relations, and the limiting case is a zero-field relation — a set that is either
empty or contains the single empty fact (a database-level boolean). The model as written
neither permits nor forbids it, and every layer (fact encoding, hashing, unique
constraints, the IR's "unbound = existential" atoms) would need a defined behavior for
it.

Question: Are zero-field relations valid schema (with the empty fact as their sole
possible member), or rejected at declaration?
