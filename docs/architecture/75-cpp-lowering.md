# 75 — C++ lowering

This is the single normative reference for the C++ frontend's lowering. The
goal is byte-exact recipe parity: a C++ cookbook theory MUST lower through the
Rust `SchemaSpec` path to the identical `SchemaDescriptor` — and therefore the
identical fingerprint — that the TypeScript SDK and the `schema!` macro
produce (docs/handoffs/2026-08-08-cpp-sdk-design-record.md:881-889, 1986-2001). Every claim below is cited against
the sources it was read from (verified 2026-08-08, branch `cpp-sdk`).

Authority chain, fixed and non-negotiable:

1. The C++ frontend builds a `SchemaSpec` (named plain data) and hands it to
   the bridge. It NEVER re-implements the canonical lowering as a runtime
   source of truth (docs/handoffs/2026-08-08-cpp-sdk-design-record.md:881).
2. The engine's `SchemaSpec::descriptor()` does name→id resolution, handle
   resolution, `==` splitting, and the canonical-utterance ban table
   (crates/bumbledb-theory/src/schema/spec.rs:996-1027).
3. Everything semantic beyond names stays at `SchemaDescriptor::validate`
   inside `Db::create`/`Db::open` (spec.rs:22-24). The frontend lowers; it
   never re-judges.

---

## 1. SchemaSpec shapes

Source of truth: `crates/bumbledb-theory/src/schema/spec.rs`. The TS wire
mirror (the shapes the napi bridge marshals verbatim, and the shapes the C
bridge views must reproduce field-for-field) is `ts/src/spec.ts`.

### 1.1 `SchemaSpec` (spec.rs:41-45; ts/src/spec.ts:208-211)

| field        | type                 | rule |
|--------------|----------------------|------|
| `relations`  | `Vec<RelationSpec>`  | declaration order — the order mints every `RelationId` (spec.rs:38-40) |
| `statements` | `Vec<StatementSpec>` | declaration (written) order — DECLARED statements only (see §2.1) |

### 1.2 `RelationSpec` (spec.rs:54-61; spec.ts:168-172)

| field    | type                  | rule |
|----------|-----------------------|------|
| `name`   | `Box<str>`            | |
| `fields` | `Vec<FieldSpec>`      | declaration order = field ordinals. For a CLOSED relation these are the declared intrinsic columns ONLY — the synthetic `(id, u64)` handle field is materialized by engine validation, never spelled in the spec (spec.rs:47-53) |
| `closed` | `Option<ClosedSpec>`  | the option IS the kind: `Some` = closed, `None` = ordinary (ruled 2026-07-23, R7; spec.rs:58-60) |

### 1.3 `ClosedSpec` (spec.rs:69-79; spec.ts:157-160)

| field     | type           | rule |
|-----------|----------------|------|
| `newtype` | `Box<str>`     | the handle newtype name. In SDK lowering this is ALWAYS the id's law-computed generator class, i.e. the string `"<RelationName>.id"` (ts/src/lower.ts:136-148). Host-side vocabulary only: `LiteralSpec::Handle` resolves through a referencing field's `FieldSpec::newtype` to this label; DROPPED at descriptor lowering, never fingerprinted (spec.rs:71-76) |
| `rows`    | `Vec<RowSpec>` | ground axioms in declaration order; row id = index (spec.rs:77-78) |

Two closed relations claiming one handle newtype is
`SpecIssue::DuplicateHandleNewtype` (spec.rs:366-377).

### 1.4 `FieldSpec` (spec.rs:87-101; spec.ts:131-136)

| field        | type                | rule |
|--------------|---------------------|------|
| `name`       | `Box<str>`          | |
| `value_type` | `ValueType`         | see §1.8 |
| `newtype`    | `Option<Box<str>>`  | the field's DOMAIN label. SDK lowering feeds the law-computed class name here (`undefined`/`None` on a bare field) — ts/src/lower.ts:63-70. Carried for handle resolution and the coherence check ONLY; dropped at lowering: two specs differing only in newtypes lower to identical descriptors (spec.rs:91-97) |
| `fresh`      | `bool`              | the mint mark; legal on `u64` only, judged at engine validate (spec.rs:98-100). Lowers to `Generation::Fresh` vs `Generation::None` (spec.rs:1078-1083) |

### 1.5 `RowSpec` (spec.rs:107-111; spec.ts:142-145)

| field    | type               | rule |
|----------|--------------------|------|
| `handle` | `Box<str>`         | |
| `values` | `Vec<LiteralSpec>` | one literal per declared intrinsic column in field-declaration order. MORE values than declared columns = `SpecIssue::RowArityExcess` at descriptor lowering (spec.rs:355-365, 1096-1104); FEWER survives lowering and is the engine's `ExtensionArityMismatch` |

### 1.6 Literals and selections

- `LiteralSpec` = `Value(Value) | Handle(Box<str>)` (spec.rs:117-121). A
  handle literal crosses BY NAME; the engine resolves it through the selected
  field's newtype to the owning closed relation's declaration-order row id, a
  `u64` word (spec.rs:743-780). Wire mirror: `{kind:"value", value} |
  {kind:"handle", handle}` (spec.ts:59-61).
- `LiteralSetSpec` = `One(LiteralSpec) | Many(Vec<LiteralSpec>)`
  (spec.rs:127-131). A `Many` with < 2 literals is
  `SpecIssue::DegenerateLiteralSet` (spec.rs:412-418): `{L}` is the bare
  literal, `{}` is no binding.
- `SideSpec` (spec.rs:135-142; spec.ts:80-84): `relation: Box<str>`,
  `projection: Vec<Box<str>>` (π, the statement's written order — positional
  pairing), `selection: Vec<(Box<str>, LiteralSetSpec)>` (σ, read
  conjunctively).

`Value` roster for literals (crates/bumbledb-theory/src/value.rs:20-50):
`Bool(bool)`, `U64(u64)`, `I64(i64)`, `String(Box<[u8]>)` (raw UTF-8),
`FixedBytes(Box<[u8]>)`, `IntervalU64(Interval<u64>)`,
`IntervalI64(Interval<i64>)` — intervals half-open `[start, end)`, strictly
`start < end` — plus `AllenMask` (query bind-time only, NEVER a schema
literal). Wire mirror `ValueSpec` (spec.ts:44-51) tags:
`bool/u64/i64/string/fixedBytes/intervalU64/intervalI64`.

### 1.7 Capacity payloads

- `WeightSpec` = `Unit | Field(Box<str>) | Duration(Box<str>)`
  (spec.rs:152-160). The weight is ALWAYS present — unit is a case, not an
  absence (C4; spec.ts:99-108). Names resolve against the SOURCE row's sealed
  roster; a dotted path is `SpecIssue::WeightPathRefused` (spec.rs:856-884).
- `BoundSpec` = `Lit(u64) | Field(Box<str>) | Duration(Box<str>)`
  (spec.rs:169-177). Dependent bounds resolve by name against the TARGET's
  WHOLE field roster (C1), never the projection; dotted path is
  `BoundPathRefused` (spec.rs:895-918); dependent bounds are hi-slot only
  (C6) — a dependent floor/exact is `CapacityDependentFloor`.
- `CapacityWindowSpec` = `Exact(BoundSpec) | Range{lo, hi} | Floor(BoundSpec)`
  (spec.rs:190-200). The ban table at lowering (spec.rs:930-993):
  `{hi<lo}` inverted, `{n..n}` exact-respelled, `{0..0}` exclusion-respelled,
  `{0..*}` vacuous, unit `{1..*}` containment-respelled (fires on the unit
  weight SPELLING only — spec.rs:1195-1199). The SDK's `within()` mint makes
  banned spellings unwritable host-side (ts/src/capacity.ts:343-433); the C++
  mint SHALL do the same, but the engine remains the wall.
  Wire tags: `{kind:"exact", n} | {kind:"range", lo, hi} | {kind:"floor", lo}`
  with bounds `{kind:"lit", value} | {kind:"field", field} |
  {kind:"durationField", field}` (spec.ts:93-123).

### 1.8 `ValueType` roster (crates/bumbledb-theory/src/schema.rs:83-118)

| variant | payload | wire spelling (spec.ts:26-36; ts/src/lower.ts:36-51) |
|---|---|---|
| `Bool` | — | `{kind:"bool"}` |
| `U64` | — | `{kind:"u64"}` |
| `I64` | — | `{kind:"i64"}` |
| `String` | — | `{kind:"string"}` (surface `str`) |
| `FixedBytes` | `len: u16` (1..=64) | `{kind:"fixedBytes", len}` — the length IS the type and a fingerprint input |
| `Interval` | `element: IntervalElement` (`U64`/`I64`, schema.rs:74), `width: Option<u64>` | `{kind:"interval", element, width}` — `width: None` is the general 16-byte encoding (rays representable); `Some(w)` is `interval<E, w>`; the width is a fingerprint input |

`Generation` = `None | Fresh` (schema.rs:123).

### 1.9 `StatementSpec` (spec.rs:206-232; spec.ts:183-197)

```
Fd          { relation: Box<str>, projection: Vec<Box<str>> }        // R(X) -> R; NO selection field exists
Containment { source: SideSpec, target: SideSpec, bidirectional: bool }
Capacity    { target: SideSpec, weight: WeightSpec,
              window: CapacityWindowSpec, source: SideSpec }         // operator read order: target, weight, window, source (C2)
```

`==` is NOT a variant: a bidirectional containment is
`Containment{bidirectional: true}`, split by the ENGINE into two adjacent
containment descriptors, `source <= target` FIRST (spec.rs:202-205,
1160-1180). Wire kinds: `"fd" | "containment" | "capacity"`.

### 1.10 What is dropped at descriptor lowering (the newtype slots)

`SchemaSpec::descriptor()` drops EVERY newtype: `FieldSpec::newtype` and
`ClosedSpec::newtype` never reach `SchemaDescriptor` and are never
fingerprinted (spec.rs:71-76, 91-97; docs/handoffs/2026-08-08-cpp-sdk-design-record.md:574, 886). They exist for
exactly two engine-side jobs, both authoring-time:

1. Handle-literal resolution: `LiteralSpec::Handle` resolves through the
   selected field's newtype → the closed relation registered under that
   newtype → the handle's declaration-order row id (spec.rs:743-780).
2. The coherence check: over every paired-face statement, positionwise, the
   paired columns' newtype labels must agree — same label with same label,
   bare with bare; labeled↔bare is a mismatch
   (`SpecIssue::StatementNewtypeMismatch`, spec.rs:419-435, 700-733). A
   closed relation's synthetic `id` carries the handle newtype in this
   judgment (spec.rs:652-670).

Consequence for C++: the class names computed in §3 MUST be fed into the
`newtype` slots (or handle literals will not resolve and the coherence check
will fire), and they MUST match the TS naming discipline exactly so
diagnostics agree cross-host — but they can never move the fingerprint
(docs/handoffs/2026-08-08-cpp-sdk-design-record.md:884-889).

### 1.11 The sealed shape / synthetic-id law

Statement field NAMES address the sealed shape: on a closed relation, `id`
resolves to `FieldId(0)` carrying the handle newtype, and declared columns
shift to `declared index + 1` (spec.rs:652-670; the descriptor-side twin is
`RelationDescriptor::sealed_fields`, crates/bumbledb-theory/src/schema.rs:457).
Ordinary relations: `FieldId` = declaration index. A sealed roster past
u16::MAX fields is `RelationTooManyFields` (spec.rs:343-350, 1033-1046).

---

## 2. Statement lowering rules (`ts/src/lower.ts` — the parity target)

`lower(theory)` (lower.ts:161-170) produces the wire `SchemaSpec`:

- **Relations** in RECORD DECLARATION ORDER (the schema's relation record,
  entry order — lower.ts:162). C++: the order relations are declared in the
  schema constructor.
- **Statements**: DECLARED statements ONLY, in written order (lower.ts:169).
  The engine materializes fresh-implied keys and closed auto-keys itself —
  `SchemaDescriptor::materialized_statements`
  (crates/bumbledb-theory/src/schema.rs:500-534): one auto-`Functionality`
  per `Fresh` field (relation declaration order, then field order; projection
  = the one fresh sealed ordinal), then one closed auto-key `R(id) -> R` per
  closed relation (declaration order; projection = `FieldId(0)`), then the
  declared statements. Re-stating an implied key doubles it and CHANGES THE
  FINGERPRINT (spec.ts:199-207); the TS `schema()` rejects such duplicates at
  construction (ts/src/schema.ts:298-323) and C++ SHALL too.
- **Fixed key order per shape**: every lowered object is built with ONE fixed
  key order so serialization is byte-stable (lower.ts:1-9; spec.ts:12-15).
  The orders, exactly as written in lower.ts:
  - `FieldSpec`: `name, valueType, newtype, fresh` (lower.ts:63-70)
  - `SideSpec`: `relation, projection, selection` (lower.ts:73-81)
  - `fd`: `kind, relation, projection`; `containment`: `kind, source, target,
    bidirectional`; `capacity`: `kind, target, weight, window, source`
    (lower.ts:89-110)
  - `RelationSpec`: `name, fields, closed` (lower.ts:119-124, 136-148)
  - `RowSpec`: `handle, values`; `ClosedSpec`: `newtype, rows`
  - `SchemaSpec`: `relations, statements` (lower.ts:169)
- **u64s as u64**: every u64/i64 crosses full-width (`bigint` in TS,
  `uint64_t`/`int64_t` in C++), never a double (spec.ts:12-13;
  ts/src/native.ts:16-18).
- **Field lowering** (lower.ts:63-70): `fresh` is the literal structural mark
  (`true` exactly on a fresh-marked u64); `newtype` = the law-computed class
  name from §3, absent on bare fields.
- **Ordinary relation** (lower.ts:119-124): fields in declaration order,
  `closed: undefined`.
- **Closed relation** (lower.ts:136-148): `fields` = declared intrinsic
  columns only; `closed.newtype` = the id's generator class `"<Name>.id"`
  (ALWAYS present — a closed id is a generator); `closed.rows` = axioms in
  declaration order, literals already lowered at `closed()` construction
  (handle-valued column literals lower to `{kind:"handle", handle}` — one
  literal machine with σ selections, spec.rs:103-106).
- **Face lowering** (lower.ts:73-81): `relation` = owner name, `projection` =
  written tuple order, `selection` = the face's σ bindings as
  `[field, LiteralSetSpec]` pairs in written order. A ψ-selected CLOSED face
  lowers its selection AS-IS — the ENGINE folds it against the sealed
  extension at validate, never the SDK (ts/src/face.ts:1-20).
- **`mirrors` stays ONE statement** with `bidirectional: true`; the engine
  performs the `==` split, source-first (lower.ts:83-88).

### 2.1 Statement kind → StatementSpec, with worked cookbook examples

**key** — `key(R, [f...])` → `{kind:"fd", relation, projection}`
(lower.ts:92-93). Only ordinary relations; a key on a closed relation is
rejected as a duplicate of the auto-key (ts/src/statements.ts:217-232).
Example (ts/COOKBOOK.md:124):

```ts
key(Outage, ["service", "window"])
// → { kind: "fd", relation: "Outage", projection: ["service", "window"] }
```

**contained** — `contained(on(A, x), on(B, y))` →
`{kind:"containment", source, target, bidirectional: false}`
(lower.ts:94-100; constructor statements.ts:243-257). Example
(COOKBOOK.md:257):

```ts
contained(on(Posting, "account"), on(Account, "id"))
// → { kind: "containment",
//     source: { relation: "Posting", projection: ["account"], selection: [] },
//     target: { relation: "Account", projection: ["id"], selection: [] },
//     bidirectional: false }
```

**Multi-column face** (COOKBOOK.md:1648): projections pair positionwise;
both faces must project equally many fields (statements.ts:166-172):

```ts
contained(on(Device, ["model", "watts"]), on(Model, ["id", "watts"]))
// → source projection ["model", "watts"], target projection ["id", "watts"]
```

**σ-selected face / mirrors** (COOKBOOK.md:178; constructor
statements.ts:268-282): `on(Task.where({ kind: "Deterministic" }), "id")`
carries the selection as resolved bindings — a closed-reference field's
literal is its handle NAME (ts/src/relation.ts:122-125; resolved at `where()`
construction through the same literal machine):

```ts
mirrors(on(Task.where({ kind: "Deterministic" }), "id"), on(DeterministicGrading, "task"))
// → { kind: "containment", bidirectional: true,
//     source: { relation: "Task", projection: ["id"],
//               selection: [["kind", { kind: "one",
//                            literal: { kind: "handle", handle: "Deterministic" } }]] },
//     target: { relation: "DeterministicGrading", projection: ["task"], selection: [] } }
// The ENGINE lowers this to two adjacent containments, Task <= DeterministicGrading first.
```

**ψ-selected CLOSED face** (COOKBOOK.md:391): a closed owner's selection over
a payload column lowers pass-through as a plain value literal:

```ts
contained(on(Certificate, "kind"), on(Kind.where({ mastered: true }), "id"))
// → target: { relation: "Kind", projection: ["id"],
//             selection: [["mastered", { kind: "one",
//                          literal: { kind: "value", value: { kind: "bool", value: true } } }]] }
```

**capacity** — `capacity(target, weight?, window, source)` →
`{kind:"capacity", target, weight, window, source}` (lower.ts:101-108;
constructor statements.ts:367-429). The weight is always present — the unit
overload lowers `{kind:"unit"}`. Unit example (statements.ts:355-357):

```ts
capacity(on(Holder, "id"), within(0n, 3n), on(Account, "holder"))
// → { kind: "capacity",
//     target: { relation: "Holder", projection: ["id"], selection: [] },
//     weight: { kind: "unit" },
//     window: { kind: "range", lo: { kind: "lit", value: 0n }, hi: { kind: "lit", value: 3n } },
//     source: { relation: "Account", projection: ["holder"], selection: [] } }
```

**weigh + ref (dependent bound)** (COOKBOOK.md:1650; mints
ts/src/capacity.ts:434-445, 453):

```ts
capacity(on(Pool, "id"), weigh("watts"), within(0n, ref("supply")), on(Device, "pool"))
// → weight: { kind: "field", field: "watts" }           // SOURCE-row u64
//   window: { kind: "range", lo: { kind: "lit", value: 0n },
//             hi:  { kind: "field", field: "supply" } } // TARGET-row u64, hi slot only (C6)
```

**duration in both slots** (COOKBOOK.md:1691-1696; mint capacity.ts:463):

```ts
capacity(on(Room, "id"), weigh(duration("booked")), within(0n, duration("span")), on(Booking, "room"))
// → weight: { kind: "durationField", field: "booked" }  // SOURCE interval measure
//   window: { kind: "range", lo: { kind: "lit", value: 0n },
//             hi:  { kind: "durationField", field: "span" } } // TARGET interval measure
```

`within(n)` → `{kind:"exact", n:{kind:"lit", value:n}}`; `within(lo, "*")` →
`{kind:"floor", lo:...}` (capacity.ts:343-413).

### 2.2 Construction-time walls the C++ frontend must replicate

These fire before the wire (statement constructors, statements.ts):
arity agreement (statements.ts:166-172); roster agreement — paired positions
are closed-with-closed through ONE roster or bare-with-bare, judged on
descriptor identity (statements.ts:190-204; the engine cannot backstop this —
the wire carries plain u64s); weight-on-source u64/interval typing
(statements.ts:292-312); bounds-on-target typing (statements.ts:322-345);
the unit `{1..*}` and unit-vs-`duration()` C18 bans (statements.ts:402-414).
`schema()` additionally rejects duplicate/implied statements and verifies
membership (ts/src/schema.ts:298-325).

---

## 3. The law-class algorithm (`ts/src/law.ts`), restated imperatively

The laws type the columns: every field's domain (class) is COMPUTED from the
statement list; nothing is declared (law.ts:1-51). The runtime computation is
`computeClasses(name, relations, statements)` (law.ts:426-491). Reproduce it
exactly:

1. **Coordinates.** For each schema member in relation-record declaration
   order, enumerate its SEALED fields in declaration order
   (`sealedFieldsOf` — a closed member is `id` first, then its declared
   columns; ts/src/closed.ts:221-226). Coordinate = the string
   `"<Relation>.<field>"` (law.ts:339-352, 433).
2. **Generators.** A coordinate is a generator iff: ordinary member and the
   field is fresh-marked; or closed member and the field is `id`
   (law.ts:343-349). Seed a union-find with every coordinate; mark generators
   (law.ts:431-440).
3. **Pairing.** Walk the statements IN WRITTEN ORDER. `key()` pairs NOTHING
   (an FD identifies no carriers — law.ts:405-412, 36-38). Every containment
   (bidirectional included) and every capacity statement pairs its source and
   target faces POSITIONWISE: for each projection position i, union
   `"<source.owner>.<source.projection[i]>"` with
   `"<target.owner>.<target.projection[i]>"` (law.ts:441-463). σ/ψ
   selections change pairing NOT AT ALL — a ψ-selected face pairs by its
   projection exactly as a bare one (law.ts:34-36; spec.rs:692-694). Record
   every paired coordinate in a `paired` set.
4. **The one-generator wall.** After each union, if the merged class holds
   more than one generator, FAIL construction with an error naming BOTH
   generator coordinates and the statement that unified them (law.ts:456-462:
   `"schema <name>: the statements unify two generators into one class —
   <gen1> and <gen2> (two mints cannot share a carrier) — <rendered
   statement>"`). The engine's spec-path twin is the coherence check's
   `StatementNewtypeMismatch` (spec.rs:419-435).
5. **Naming.** Walk members/fields in relation-declaration ×
   field-declaration order; for each class root not yet named: the class name
   is its generator coordinate if it has one (GENERATOR-FIRST), else the
   FIRST coordinate encountered in this walk — i.e. the least member
   coordinate in relation-declaration × field-declaration order
   (law.ts:465-475). Deterministic, pinned forever; this VALUE-tier name is
   the only thing the wire reads (law.ts:14-23).
6. **Bare.** A field is classed iff it is a generator or appears in `paired`;
   otherwise its class is absent (`undefined`) — a field in no law has NO
   class (law.ts:476-490, 24-25). Note: a lone generator IS classed (its own
   name) even if no statement touches it — a closed id's class `"Kind.id"`
   always exists.
7. **Fingerprint neutrality.** Class names flow ONLY into `newtype` slots and
   query-join judgments; the engine drops them at descriptor lowering, so
   class identity NEVER enters the fingerprint (law.ts docs; lower.ts:57-62;
   spec.rs:426-428; docs/handoffs/2026-08-08-cpp-sdk-design-record.md:680-682). Getting a class name wrong cannot
   move a fingerprint — it fails handle resolution, the coherence check, or
   cross-host diagnostics instead.

C++ binding: docs/handoffs/2026-08-08-cpp-sdk-design-record.md:886-889 makes this discipline normative
("generator-first, else least member coordinate in relation-declaration ×
field-declaration order").

---

## 4. Query/program IR

### 4.1 Engine shapes (`crates/bumbledb/src/ir.rs`)

- `PredId(u16)` — index into `Program.predicates` (ir.rs:65).
- `AtomSource` = `Edb(RelationId) | Idb(PredId)` (ir.rs:76-79). An `Idb`
  atom's `FieldId(i)` addresses the target predicate's head position `i`.
- `VarId(u16)` — dense, RULE-SCOPED (the same id in two rules is two
  variables; ir.rs:105). `ParamId(u16)` — dense, QUERY-GLOBAL (ir.rs:109).
- `Term` = `Var(VarId) | Param(ParamId) | ParamSet(ParamId) | Literal(Value)
  | Measure(VarId)` (ir.rs:114-138). `ParamSet` is legal in atom bindings and
  one side of `Eq` only; `Measure` only as one side of an order comparison.
- `Atom { source, bindings: Vec<(FieldId, Term)> }` — absence of a field IS
  the wildcard; zero bindings = nonemptiness gate (ir.rs:150-166).
- `AggOp` = `Sum | Min | Max | Count | CountDistinct | ArgMax{key: ArgKey} |
  ArgMin{key} | Pack` (ir.rs:178-216); `ArgKey` = `Var(VarId) |
  Measure(VarId)` (ir.rs:227-231).
- `FindTerm` = `Var(VarId) | Aggregate{op, over: Option<VarId>} |
  Measure(VarId) | AggregateMeasure{op, over: VarId}` (ir.rs:248-271).
  `over` is `None` for nullary `Count`.
- `HeadTerm` = `Var | Aggregate(HeadOp)` — var-free head shapes
  (ir.rs:294-332).
- `MaskTerm` = `Literal(AllenMask) | Param(ParamId)` (ir.rs:341-344).
- `CmpOp` = `Eq | Ne | Lt | Le | Gt | Ge | Allen{mask: MaskTerm} | PointIn`
  (ir.rs:359-368). `PointIn` is lowered interval-LEFT, point-RIGHT
  (ir.rs:355-357).
- `ConditionTree` = `Leaf(Comparison) | And(Vec) | Or(Vec)` (ir.rs:418-422);
  `Comparison { op, lhs, rhs }` (ir.rs:394-398). OR is distributed to DNF
  rules at validation, engine-side.
- `Rule { finds, atoms, negated, conditions }` (ir.rs:433-455). Negated
  atoms bind nothing (safety rule).
- `Query { head: Vec<HeadTerm>, rules: Vec<Rule> }` (ir.rs:483-491);
  `PredicateDef` is the same pair (ir.rs:510-517);
  `Program { predicates, output: PredId }` (ir.rs:532-538). A plain query is
  the degenerate one-predicate program, `output = PredId(0)`
  (ir.rs:540-556). Caps: `MAX_RULES = 16` (ir.rs:31), `MAX_PREDICATES = 16`
  (ir.rs:42), `MAX_CONDITION_DEPTH = 64` (ir.rs:55).

Wire mirror (what the bridge takes, 1:1 — `ts/src/native.ts:80-175`):
`ProgramIr{predicates, output}`, `PredicateDefIr{head, rules}`,
`HeadTermIr {kind:"var"} | {kind:"aggregate", op}` with `HeadOpIr` strings
`"sum"|"min"|"max"|"count"|"countDistinct"|"argMax"|"argMin"|"pack"`,
`RuleIr{finds, atoms, negated, conditions}`,
`FindTermIr {kind:"var",var} | {kind:"aggregate",op,over?} |
{kind:"measure",var} | {kind:"aggregateMeasure",op,over}`,
`AggOpIr` (`argMax`/`argMin` carry `key: number`),
`AtomIr{source, bindings: [fieldId, TermIr][]}`,
`AtomSourceIr {kind:"edb",relation} | {kind:"idb",pred}`,
`TermIr {kind:"var"|"param"|"paramSet"|"literal"|"measure", ...}`,
`CmpOpIr` (allen carries `mask: MaskTermIr`), `ComparisonIr{op,lhs,rhs}`,
`ConditionTreeIr {kind:"leaf",cmp} | {kind:"and"|"or", children}`.
Ids only — the bridge never sees names in queries (native.ts:75-79).

### 4.2 How the SDK lowers (`ts/src/query/lower.ts` — reproduce exactly)

`lowerQuery` (query/lower.ts:1953-1994) is pure and stable: the same query
value lowers to deeply-equal IR every time.

- **Relation ordinals**: relation name → its declaration index in the
  schema's relation record (query/lower.ts:1956-1958).
- **Predicates**: recs in declaration order, `PredId` = index; the OUTPUT
  predicate (the query's own rules) appended LAST, `output = recs.length`
  (query/lower.ts:1959-1993).
- **Variable numbering** (query/lower.ts:1678-1697, 1912-1946): per rule,
  a fresh numberer keyed on the variable OBJECT REFERENCE assigns dense ids
  by FIRST OCCURRENCE during the lowering walk. The walk order is: body
  items in WRITTEN order — positive atoms, negated atoms, idb atoms, and
  conditions interleaved exactly as `.match/.where/.idb` were called (each
  lowered into its own bucket: `atoms`, `negated`, `conditions`) — then the
  find terms LAST. Within an EDB atom, bindings lower in the bindings
  record's WRITTEN property order; each binding is
  `[sealedFieldOrdinal, TermIr]` where the ordinal is the field's index in
  the SEALED roster (closed `id` = 0; query/lower.ts:1712-1734). Within an
  IDB atom, bindings are placed and numbered in HEAD ORDER — `FieldId(i)` =
  head position `i`, every head column bound exactly once
  (query/lower.ts:1759-1783).
- **Atom ordering**: `atoms`/`negated`/`conditions` each keep written order;
  an idb item goes to `atoms` or `negated` by its polarity
  (query/lower.ts:1917-1937).
- **Param registry** (query/lower.ts:1337-1426; entry shape
  ts/src/query/scope.ts:392-398): fold every rule's param uses — recs in
  declaration order (each rec's rules in order) FIRST, output rules LAST,
  uses within a rule in written order. First use of a name mints the dense
  `ParamId` (registry order = positional execution order); the first
  FIELD-ANCHORED use types the wire (anchor = the binding position's field
  descriptor, or the comparison sibling's field, or `"measure"`;
  query/lower.ts:633-654). One name keeps ONE shape (`value`/`set`/`mask`)
  and ONE closedness — conflicts are construction errors. A param with no
  field-anchored use (and not a mask) fails lowering
  (query/lower.ts:1965-1971).
- **Membership arrays** (closed-reference literal sets;
  query/lower.ts:453-487, 1400-1422): a plain array of ≥ 2 DISTINCT handle
  names at a closed-reference binding position lowers as a `paramSet` term
  over a synthetic content-addressed registry entry (name =
  `"∈ <Roster> <sorted-members-JSON>"`); the entry's `membership` is
  pre-resolved at BUILD to a frozen `{kind:"set", values:[{kind:"u64",...}]}`
  program constant — the execute-time params object is never consulted for
  it (ts/src/query/run.ts:57-63). Empty and one-element arrays are refused.
- **Aggregate heads** (query/lower.ts:1860-1909): `sum/min/max(var)` →
  `{kind:"aggregate", op:{kind:"sum"|...}, over: varId}`;
  `sum/min/max(duration(v))` → `{kind:"aggregateMeasure", op, over: varId}`;
  `count()` → `{kind:"aggregate", op:{kind:"count"}}` (no `over`);
  `countDistinct(v)` → over varId; `argMax/argMin(over, key)` →
  `{kind:"aggregate", op:{kind:"argMax"|"argMin", key: keyVarId}, over}`;
  `pack(v)` → `{kind:"aggregate", op:{kind:"pack"}, over}`. Head terms:
  var and measure finds → `{kind:"var"}`; aggregates →
  `{kind:"aggregate", op: <HeadOpIr string>}` (query/lower.ts:1887-1909).
  Every rule of a query must derive the same head signature AND the same
  per-column closed roster and class (query/lower.ts:1441-1478).
- **Match-literal handling** (query/lower.ts:1590-1667): a bare literal at
  an atom binding tags by the FIELD's structural kind
  (`taggedLiteral`) — bool→`{kind:"bool"}`, u64/i64→bigint-tagged,
  str→well-formed string, bytes→`{kind:"fixedBytes"}`, interval field:
  a bigint tags as the ELEMENT (point membership), an interval-shaped value
  as the interval (`taggedAtElementDomain`, query/lower.ts:1570-1584).
  A comparison/param literal tags by its SIBLING's anchor
  (`taggedCmpLiteral`, query/lower.ts:1643-1667): measure sibling → u64;
  interval-field sibling → element domain; and — op-aware, the POINT-DOMAIN
  rule — under `pointIn` an interval-shaped literal beside a scalar u64/i64
  sibling tags as an interval of the sibling's kind.
- **Closed-handle resolution in queries** (query/lower.ts:1548-1563): a
  handle NAME at a closed-reference position is verified against the roster
  and translated to its declaration-order row id, tagged
  `{kind:"u64", value: BigInt(index)}` (`taggedHandleId` — THE single
  roster-verification point). Order comparisons and folds over closed-bound
  terms are refused (the orderable ban, query/lower.ts:869-902).
- **Allen masks**: 13-bit literal masks (bit order
  ts/src/query/atom.ts:449-471, identical to the engine's palindromic order)
  or `maskParam` → `MaskTermIr {kind:"param", param}` (query/lower.ts:
  1823-1844).
- **pointIn operand sealing**: the VALUE stores interval-left, point-right
  whatever the surface argument order (ts/src/query/atom.ts:432-435); the
  lowering emits lhs = interval side, rhs = point side.

---

## 5. Runtime marshalling

### 5.1 Engine bind shapes (`crates/bumbledb/src/api/prepared.rs`)

```rust
pub enum BindValue<'a> {              // prepared.rs:53-71
    Bool(bool), U64(u64), I64(i64), Str(&'a str),
    FixedBytes(&'a [u8]),             // exactly the anchored field's N bytes
    IntervalU64(u64, u64),            // half-open [start, end)
    IntervalI64(i64, i64),
    AllenMask(AllenMask),             // bind-time mask param; ∅/full rejected at bind
}
pub enum ParamArg<'a> {               // prepared.rs:82-85
    Scalar(BindValue<'a>),
    Set(&'a [ir::Value]),             // param sets; slices dedup into pooled storage
}
```

Params are supplied POSITIONALLY by `ParamId` (registry order). Bind checks
count, scalar-vs-set usage, and element types (prepared.rs:73-80). Bridge
wire twin: `QueryParam = TaggedValue | {kind:"set", values: TaggedValue[]}`
where `TaggedValue = ValueSpec | {kind:"allenMask", mask}` (native.ts:66-72);
marshalled by `wireParams` in registry order, each value tagged by its
param's anchoring use (ts/src/query/run.ts:57-82).

### 5.2 Answers (`prepared.rs:87-129`)

```rust
pub enum AnswerValue<'a> {            // prepared.rs:89-101, borrowed from Answers
    Bool(bool), U64(u64), I64(i64),
    String(&'a str),                  // borrowed from the Answers string heap
    FixedBytes(&'a [u8]),             // borrowed from the Answers byte heap
    IntervalU64(Interval<u64>), IntervalI64(Interval<i64>),
}
```

`Answers` (prepared.rs:118-129) is the caller-owned reusable buffer: flat
cells (fixed-width inline; String/FixedBytes as ranges into two byte heaps —
prepared.rs:103-116), arity = find-term count, `clear()` retains capacity,
`get` panics out-of-range (docs/handoffs/2026-08-08-cpp-sdk-design-record.md:186-190). Answers are SETS — no order
exists; hosts sort. Column order = the program's head order = the find
record's written order.

Host decode mapping (the TS precedent, run.ts:107-127; C++ mirrors with its
own value vocabulary): `Bool→bool`, `U64/I64→64-bit ints`, `String→string
view` (borrowed), `FixedBytes→byte view` (borrowed), intervals →
`{start, end}`. A CLOSED answer column lifts its u64 row id back to the
handle NAME through the roster (`handleOf`, ts/src/marshal.ts:127-138); an
out-of-roster id is a pointed error, never a fallback.

### 5.3 Fact rows (dyn lane)

- `WriteTx::insert_dyn(rel, &[Value])` / `delete_dyn`: one `Value` per field
  in DECLARATION order; closed relations refuse writes
  (crates/bumbledb/src/api/db/insert_dyn.rs:7-25). Closed-reference cells are
  plain `u64` handle row ids on the wire — the host translates names ↔ ids
  (ts/src/marshal.ts:109-138: name→id by roster index on write, id→name on
  read).
- `Snapshot::scan(rel)` yields `Vec<Value>` rows, one value per SEALED field
  in sealed order (ordinary: declaration order; closed: synthetic `id` first
  — a closed relation scans its virtual sealed extension;
  crates/bumbledb/src/api/db/snapshot.rs:114-137).
- `contains_dyn(rel, &[Value])`: full row in declaration order
  (snapshot.rs:159-170; WriteTx twin api/db/get.rs:341).
- `get_dyn(rel, key_statement, &[Value])`: key values in the KEY STATEMENT'S
  PROJECTION order; returns the full row, fields in declaration order
  (snapshot.rs:200-230; WriteTx twin get.rs:258-296). `get` reads through
  the PRIMARY key = the first statement in MATERIALIZED order, so a
  fresh-bearing relation's primary key is its fresh field
  (ts/src/marshal.ts:52-65).
- TS bridge value mapping for rows (native.ts:53-59, the C++ analogue):
  `bool⇄bool`, `u64/i64⇄64-bit int`, `str⇄string` (UTF-8, well-formed —
  lone surrogates refused host-side, marshal.ts:176-187), `bytes<N>⇄N raw
  bytes` (width judged by the engine), `interval⇄{start, end}`.

---

## 6. Fingerprint mechanics

- The fingerprint is **blake3 over the canonical descriptor bytes**:
  `fingerprint = blake3(canonical_bytes(schema))`
  (crates/bumbledb/src/schema/fingerprint.rs:184-204), a 32-byte value
  rendered as 64 lowercase hex chars (fingerprint.rs:46-62).
- The byte stream opens with the length-prefixed format label
  **`bumbledb-schema-v5`** (fingerprint.rs:40, 86-87); every string and list
  is u32-LE length-prefixed (fingerprint.rs:206-214).
- Inputs, in order (fingerprint.rs:86-172): relations in declaration order —
  name; fields in declaration order (name, value-type tag + payload
  [Bool=0, tag 1 retired, U64=2, I64=3, String=4, FixedBytes=5‖len(u16 LE),
  general Interval=6‖element, fixed-width Interval=7‖element‖width(u64 LE);
  fingerprint.rs:260-295], generation byte 0/1); closedness tag
  (0 ordinary; 1 ‖ rows: handle bytes + canonical fact bytes) — then the
  statements in **MATERIALIZED order** (§2: fresh keys, closed auto-keys,
  declared) with form tags Functionality=0, Containment=1, Capacity=4 (tags
  2 and 3 retired, never reissued — C5), sides as (relation id, projection
  field ids, selection bindings with literal counts and type-aware encoded
  literals), and capacity bodies in operator read order (target, weight
  descriptor, lo, hi presence+kind, source).
- **Newtypes/class names are never hashed** (they were dropped before this
  point); the sealed `==` mirror pairing and enforcement data are computed
  from hashed inputs, not hashed (fingerprint.rs:10-17).
- **Where computed**: engine-side only, at schema acceptance inside
  `Db::create`/`Db::open`; stored beside the store at creation, compared at
  open — a mismatch is a hard failure (fingerprint.rs:42-45). No host ever
  computes it.
- **How TS reads it**: `native.dbFingerprint(db)` returns the 64-hex readback
  after a real `dbCreate` (ts/src/native.ts:380-389). The cookbook suite
  creates each recipe's store, reads the fingerprint, and asserts it equals
  the pinned golden in `fixtures/cookbook-fingerprints.txt` at the
  repository root (one line per recipe, `rNN <64-hex>`;
  ts/test/cookbook.test.ts:79-206). The cross-host lock additionally pins
  one everything-theory constant in both hosts (`PIN`,
  ts/test/fingerprint.test.ts:53; Rust twin
  `ts/crate/src/fingerprint_lock.rs`). Per docs/handoffs/2026-08-08-cpp-sdk-design-record.md:1993-2001 the
  goldens live host-neutral at the repository root and the C++ suite reads
  the same file, taking its hex off the create outcome exactly as TS does.

---

## 7. Parity checklist — get these byte-identical, ordered by how likely they are to be gotten wrong

1. **Declared statements ONLY.** Never emit fresh-implied keys or closed
   auto-keys into `SchemaSpec.statements` — the engine materializes them
   (fresh keys first, closed auto-keys second, declared last); restating one
   doubles it and moves the fingerprint (spec.ts:199-207;
   bumbledb-theory/src/schema.rs:500-534). Reject them at construction like
   TS does (schema.ts:314-318).
2. **`mirrors` crosses as ONE statement** with `bidirectional: true`. Do NOT
   pre-split; the engine emits the two adjacent containments,
   `source <= target` first (lower.ts:83-88; spec.rs:1168-1179).
3. **Closed relations carry declared columns only** — never spell the
   synthetic `id` field in `RelationSpec.fields`; but DO address `id` in
   statement/selection field names (it is `FieldId(0)`; declared columns
   shift +1) (spec.rs:47-53, 652-670).
4. **Every order is declaration order.** Relations = record declaration
   order; fields = declaration order; closed rows = declaration order (row
   id = index); statements = written order; projections = written tuple
   order; selections = written binding order (lower.ts:1-9, 161-170).
5. **Capacity shape**: field order target, weight, window, source (C2);
   weight ALWAYS present (`unit` is a case); dependent bounds hi-slot only,
   resolved by name against the target's FULL roster; no dotted paths in
   weight or bound (spec.rs:206-232, 856-918; spec.ts:99-123).
6. **Class algorithm exactness**: `key()` pairs nothing; positionwise unions
   over every containment/capacity face pair (ψ-selections irrelevant);
   generator = fresh field or closed id; one-generator wall names both
   coordinates and the statement; naming generator-first else least member
   coordinate in relation-declaration × field-declaration order; unpaired
   non-generators are bare (law.ts:426-491).
7. **newtype slots**: every field's `newtype` = its law-computed class name
   (absent when bare); `ClosedSpec.newtype` = `"<Name>.id"`. Wrong labels
   fail handle resolution (`NotAHandleField`) or the coherence check
   (`StatementNewtypeMismatch`) at descriptor lowering — they never move the
   fingerprint (lower.ts:63-70, 136-148; spec.rs:321-327, 419-435).
8. **Handle literals cross by NAME** (`{kind:"handle", handle}`), in σ
   bindings and closed-row values alike; the ENGINE maps name → row id.
   Never pre-resolve a schema literal to a u64 (spec.rs:117-121, 743-780).
   (Queries are the opposite: the HOST resolves handle names to row ids —
   query/lower.ts:1548-1563.)
9. **Literal sets**: a `many` set carries ≥ 2 literals; a one-element set is
   the bare literal; an empty set is no binding (spec.rs:127-131, 412-418).
10. **u64 full width.** Every u64 crosses as a real 64-bit integer
    (`uint64_t`), including window bounds and interval endpoints; never
    through a double (spec.ts:12-13; native.ts:16-18).
11. **Value types exact**: `str` spells `string`; `bytes<N>` carries `len`
    (a fingerprint input); intervals carry `element` and optional `width`
    (`width` is a fingerprint input; `None` ≠ `Some(w)`)
    (lower.ts:36-51; bumbledb-theory/src/schema.rs:83-118).
12. **`fresh` is structural**: `true` exactly on a fresh-marked u64
    (lower.ts:63-70) → `Generation::Fresh` (spec.rs:1078-1083).
13. **Query IR discipline**: per-rule dense var ids by first occurrence over
    the written walk (body items in written order, EDB bindings in written
    property order at sealed ordinals, IDB bindings in head order, finds
    last); params by first-use registry order = positional bind order; plain
    query = one-predicate program with `output` = rec count
    (query/lower.ts:1678-1994).
14. **Point/interval tagging**: field-directed at bindings, sibling-directed
    (op-aware at `pointIn`) at comparisons and params; `pointIn` lowers
    interval-left (query/lower.ts:1570-1667; atom.ts:432-435).
15. **String well-formedness**: only valid UTF-8 crosses (TS refuses lone
    surrogates at the marshal — marshal.ts:176-187); C++ `std::string_view`
    payloads must be valid UTF-8 or the same fact-identity drift appears.
16. **Row order at the dyn lane**: writes/contains one `Value` per field in
    declaration order; keyed gets take key values in the key statement's
    projection order; scans return sealed order (insert_dyn.rs:7-25;
    snapshot.rs:114-230).
