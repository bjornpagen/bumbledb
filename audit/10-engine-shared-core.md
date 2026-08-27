# 10 — How the engine shares one Rust core across three surfaces (Rust, TypeScript, C)

Audit of the bumbledb engine's bridge architecture, written as the reference for the
planned bumbledb-log refactor (today `crates/bumbledb-log` is a pure-Rust crate and
`ts-log/` is a hand-written TypeScript re-implementation of the same protocol,
module for module — `writer.ts`/`writer.rs`, `codec.ts`/`codec.rs`,
`manifest.ts`/`manifest.rs`, `replica.ts`/`replica.rs`, `braids.ts`/`braids.rs` —
which is exactly the duplication this pattern eliminates for the engine).

All paths are relative to the repo root `/Users/bjorn/Documents/bumbledb`.

---

## 0. Orientation: one core, two dumb bridges, three hosts

```
                         crates/bumbledb            (THE core: storage, planner,
                         ├─ ir.rs                    executor, schema seal,
                         ├─ schema/ (spec, seal,     fingerprint, error taxonomy,
                         │   fingerprint)            prepared queries, answers)
                         └─ api/ (Db, WriteTx, ...)
                            ▲            ▲            ▲
             compile-time  │            │ napi-rs     │ extern "C" + cbindgen
             proc macro    │            │ (in-proc)   │ (in-proc)
                            │            │            │
   crates/bumbledb-query ───┘   ts/crate (bumbledb-node)   crates/bumbledb-c
   + bumbledb-query-macros      └─ lib.rs, marshal.rs,     └─ lib.rs, db.rs, query.rs,
   (Rust hosts: `query!`)          tags.rs                     schema.rs, value.rs,
                                    ▲                          answers.rs, error.rs
                                    │ .node cdylib             ▲
                          ts/src (TypeScript SDK)              │ include/bumbledb_c.h
                          └─ query/, native.ts, db.ts,        (generated, committed)
                             spec.ts, lower.ts, marshal.ts    C/C++ hosts
```

Both bridges state the same law in their first lines:

- `ts/crate/src/lib.rs:1-2` — "The dumb-bridge law: no logic beyond marshaling will
  EVER live in this crate. Anything smart belongs in the TypeScript SDK or the engine."
- `crates/bumbledb-c/src/lib.rs:1-4` — "The C ABI: `bdb_*` symbols, dumb marshal only.
  The dumb-bridge law (the ts/crate precedent)…"

The engine never depends back on a host. The Rust host sugar is likewise quarantined:
`crates/bumbledb-query/src/lib.rs:1-7` ("hosts may depend on this crate, the engine
never depends back") is a 7-line re-export of the proc macro.

There is exactly ONE semantic trust boundary: `Db::prepare` /
`instance.prepare` inside the core, where the IR validator runs
(`crates/bumbledb-c/src/query.rs:2-4`: "The engine's IR validator remains the trust
boundary at `bdb_db_prepare`"). Every bridge only checks *shape* (tags, arity,
counts, depth), never semantics (`ts/crate/src/marshal.rs:1-4`).

---

## 1. The data flow, boundary by boundary

### 1.1 What the shared core exports (the whole bridging vocabulary)

`crates/bumbledb/src/lib.rs:107-111` exports the pure-data query IR that every
surface targets:

- `ir::{Query, Rule, ProjectionRule, RecRule, RecStep, Rec, Interior, Atom,
  AtomSource, Term, FindTerm, HeadTerm, HeadOp, FoldOp, CmpOp, Comparison,
  ConditionTree, Value, VarId, ParamId, InteriorId, NonEmpty, MAX_CONDITION_DEPTH}`
  — definitions in `crates/bumbledb/src/ir.rs` (`Query` at ir.rs:580, `Rule` at
  ir.rs:348, `Term` at ir.rs:105, `Atom` at ir.rs:121, `ConditionTree` at ir.rs:334,
  `MAX_CONDITION_DEPTH = 64` at ir.rs:40).
- `schema::{SchemaSpec, SchemaDescriptor, RelationId, FieldId, StatementId,
  StatementKind, Manifest, RenderedViolation, render_rejection, Theory}` and
  `schema::fingerprint` (`crates/bumbledb/src/lib.rs:118-124`). `SchemaSpec` and its
  sub-specs (`RelationSpec`, `FieldSpec`, `ClosedSpec`, `StatementSpec`, `SideSpec`,
  `LiteralSpec`, `LiteralSetSpec`, `WeightSpec`, `BoundSpec`, `CapacityWindowSpec`)
  are the *parse-once* schema wire form; `spec.descriptor()` resolves names → ids and
  is called identically by both bridges (`ts/crate/src/lib.rs:307-325`,
  `crates/bumbledb-c/src/db.rs:271-275`).
- `api::{Db, WriteTx, ReadInstance, Witness, InstanceBuilder, OwnedInstance,
  MutationReport, FreshRange}` and `api::prepared::{PreparedQuery, Answers,
  AnswerValue, ParamArg, BindValue}` (`crates/bumbledb/src/lib.rs:82-89`).
- `error::{Error, ErrorFamily, Admission, ConditionalWrite, Violations, Violation,
  Direction}` (`crates/bumbledb/src/lib.rs:90-93`) — the ONE error taxonomy every
  surface re-spells.
- `AcceptedCollection`/`CollectionBuilder` (`crates/bumbledb/src/lib.rs:79-82`,
  doc-hidden) — "the bridge crates' parse-once write representation".
- `bumbledb_macros::schema` re-export (`crates/bumbledb/src/lib.rs:197`) — the
  Rust-host schema macro that also mints the id constants the `query!` macro splices
  (see 1.3).

So the core's boundary vocabulary is four families: (a) plain-data spec/IR structs,
(b) the `Value`/`AnswerValue` tagged value pair, (c) opaque stateful handles
(`Db`, `PreparedQuery`, `WriteTx`, …), (d) the `ErrorFamily` taxonomy + tagged
admission outcomes. Every bridge is a marshaling of exactly these four families and
nothing else.

### 1.2 The TypeScript surface

#### 1.2.1 Building the typed payload (no IR yet — a frozen object graph)

- `ts/src/query/scope.ts` mints variables and params. `v(owner)` (scope.ts:97-114)
  creates one frozen `Var` per sealed column (`sealedFieldsOf` in
  `ts/src/closed.ts:101-106` — a closed owner mints `id` first, then payload
  columns; declaration order is law). A `Var` (scope.ts:23-29) carries
  `{[term]: "var", owner, column, field, label}` — variable identity is object
  identity. `makeParam`/`makeSetParam` (scope.ts:116-123) mint
  `{[term]: "param"|"setParam", name}`. The type tier does class-equality of join
  slots (`JoinOk`/`AntiJoinOk`, scope.ts:156-202) with runtime twins `fieldJoins`
  / `fieldAntiJoins` (scope.ts:204-223).
- `ts/src/query/atom.ts` defines the *builder-side* data model: `BindingEntry` /
  `AtomData` (atom.ts:27-37), `CmpData`/`TreeData`/`CondData` (atom.ts:39-62),
  `AggData`/`FindColumn` (atom.ts:64-82), `RuleItem`/`RuleData` (atom.ts:84-111),
  `InteriorData`/`RecData` (atom.ts:113-139). It also exports the comparison
  constructors (`eq/ne/lt/le/gt/ge/pointIn/allen/and/or/not`, atom.ts:229-354) plus
  the ALLEN mask constants (atom.ts:283-307) and a large type-level judgment battery
  (`CondOkBool` atom.ts:479-499, `CheckBindings` atom.ts:158-166, param inference
  `CondParams`/`BindParams` atom.ts:503-544).
- `ts/src/query/find.ts` defines find entries (var or aggregate; `sum/min/max/count/
  pack` constructors find.ts:24-52; `CheckFind`/`CheckRecFind` find.ts:64-70; row
  typing `RowOfFind` find.ts:82).
- `ts/src/query/lower.ts` hosts the builder machinery: the chain interfaces
  (`QueryRuleScope`/`QueryRuleChain`/interior/rec variants, lower.ts:131-300), one
  untyped `RawScope`/`RawChain` runtime (lower.ts:955-1096) admitted at its typed
  face by the `isTypedScope` "trusted admission seam" (lower.ts:1098-1120), and
  construction-time validation: boundness (`assertBound` lower.ts:778-782, negated
  safety lower.ts:930-939), closed-order ban (`closedOrderError` lower.ts:764-772,
  `validateCond` lower.ts:837-865), head alignment across rules
  (`assertAlignedHeads` lower.ts:1310-1340), interior/rec declaration-order walls
  (`lookupDerived` lower.ts:996-1022, `afterMainError` lower.ts:1342-1346), and the
  param registry fold (`paramRegistryOf` lower.ts:1206-1300 — first use mints the
  dense ParamId, first field-anchored use types the wire, one name = one shape and
  one roster).

The product is `Query.data: QueryData` (lower.ts:302-326) — a frozen tree of
`InteriorData`/`RecData`/`RuleData`/`FindColumn`/`ParamEntry`. Still names and
object references, no ids.

#### 1.2.2 Lowering to the IR (`lowerQuery`)

`lowerQuery(q)` (`ts/src/query/lower.ts:1892-1947`) assigns every id:

- relation ordinals = `Object.keys(theory.relations)` declaration order
  (lower.ts:1894-1897) — the SAME ordinal law the schema lowering and the engine
  descriptor use;
- interior ids = declaration order, rec = `interiors.length` (lower.ts:1898-1904);
- param ids = registry order (lower.ts:1905-1915; a param with no field-anchored use
  is refused at lower.ts:1909-1912);
- var ids are minted per rule, first-use dense (`freshVarIds` lower.ts:1668-1681);
- atom bindings become `[fieldOrdinal, TermIr]` pairs against `sealedFieldsOf`
  order (`lowerAtom` lower.ts:1691-1713); interior atoms are placed by HEAD position
  (`lowerInteriorAtom` lower.ts:1737-1760, `FieldId(i)` = head position i);
- literals are tagged at their anchor's domain: `taggedLiteral`
  (lower.ts:1588-1630) maps host values → `TaggedValue` (`{kind:"u64",value:bigint}`
  etc.), with the closed bijection handle-name → `{kind:"u64", value: BigInt(rowId)}`
  (`taggedHandleId` lower.ts:1555-1570) and the sibling-anchored comparison variant
  `taggedCmpLiteral` (lower.ts:1641-1654);
- conditions/finds lower structurally (`lowerCondition` lower.ts:1807-1817,
  `lowerFind` lower.ts:1819-1832, `headTermOf` lower.ts:1845-1851).

#### 1.2.3 The IR wire type and the parse brand

The wire IR is declared ONCE on the TS side in `ts/src/native.ts:40-131`:
`QueryIr` (`{kind:"cq"|"reach", interiors, [rec], head, rules}`), `RuleIr`
(`{finds, atoms, negated, conditions}`), `AtomIr`
(`{source:{kind:"edb",relation}|{kind:"interior",interior}, bindings:[[ordinal,
TermIr]]}`), `TermIr` (`var|param|paramSet|literal`), `CmpOpIr`, `ConditionTreeIr`
(`leaf|and|or`), `FindTermIr` (`var|count|aggregate|pack`), `HeadTermIr`. Values
cross as `TaggedValue = ValueSpec` (native.ts:36, defined in `ts/src/spec.ts:15-22`
— `bool|u64|i64|string|fixedBytes|intervalU64|intervalI64`) and params as
`QueryParam = TaggedValue | {kind:"set", values}` (native.ts:38).

`parseQueryIr` (`ts/src/query/parse-ir.ts:4-23`) is a structural re-check (rules
nonempty, per-rule find width == head width, find family == head family,
count-carries-no-over) that stamps the compile-time-only brand
`ParsedQuery = QueryIr & {[parsedQueryBrand]: true}` (native.ts:85-87) via a cast
(parse-ir.ts:22). Every native prepare entry point takes `ParsedQuery`
(native.ts:339, 368, 409).

**Payload shape at this boundary: plain JavaScript objects.** Not bytes, not JSON
strings, not serialized buffers — frozen object trees with string `kind` tags,
`number` ids, `bigint` u64/i64 payloads, `Uint8Array` bytes, and
`{start,end}` bigint intervals. The napi layer walks them property by property.

#### 1.2.4 The napi crossing (ts/crate)

`ts/crate` (crate name `bumbledb-node`, `ts/crate/Cargo.toml`: cdylib, deps =
`bumbledb` + `napi 3` + `napi-derive 3`, `unsafe_code = "deny"` with scoped
`#[expect]`s) exposes `#[napi]` snake_case functions that napi-rs surfaces to JS as
camelCase — the TS `Native` interface (`ts/src/native.ts:303-411`) is the
hand-written twin of those exports.

Inbound query marshal: `marshal::query_in` (`ts/crate/src/marshal.rs:1162-1189`)
recursively rebuilds `bumbledb::Query` from the JS object:
`term_in` (marshal.rs:824-838), `atom_in` (marshal.rs:899-935), `comparison_in`
(marshal.rs:937-969, re-minting `AllenMask::new` so an invalid mask dies at the
bridge), `condition_in` with the engine's own `MAX_CONDITION_DEPTH` re-checked for
stack safety (marshal.rs:971-990), `rule_in` (marshal.rs:1002-1033),
`projection_rule_in`/`rec_rule_in`/`rec_step_in` (marshal.rs:1067-1119 — the
`vars_only` wall, the no-negation-in-rec wall, and the self-atom split live at the
bridge because the core types make them unrepresentable: `RecStep.self_bindings` is
a field, not an atom). Generic walkers `req`/`req_at` (marshal.rs:76-91) give every
missing key a pointed `bumbledb marshal: missing …` error; `ordinal`/`u16_id`
(marshal.rs:113-131) police the number-typed ids; `u64_in`/`i64_in`
(marshal.rs:93-111) police BigInt range.

Every string tag on the wire comes from ONE table per mirrored core enum:
`ts/crate/src/tags.rs` `wire_tags!` (tags.rs:15-81) generates, per enum, the tag
constants, an EXHAUSTIVE `tag()` match ("Deliberately no wildcard: a new core
variant fails compile HERE", tags.rs:26-34), a generated `parse()` for unit enums
(tags.rs:36-47, closing the old drift gap where a new variant could render but not
parse), and a declaration-order `TAGS` roster (tags.rs:49-52). Tables:
`value` (tags.rs:83-94), `value_type` (one table for BOTH directions, tags.rs:96-108),
`interval_element` (110-118), `literal`/`literal_set` (120-134),
`capacity_window`/`capacity_bound`/`weight`/`statement` (136-174),
`statement_kind` (176-184), `term` (186-194), `head_op` (196-209),
`head_term`/`find_term`/`atom_source`/`cmp_op`/`condition`/`query`/`direction`
(216-281), `param` (283-291), `error_family` (293-322), plus the hand-rolled
outcome-tag modules `admission_tag`/`write_tag`/`open_kind`/`prepare_kind`
(tags.rs:324-352).

The drift lock: a `#[cfg(test)]` golden (tags.rs:354-418) asserts every roster
equals `ts/test/fixtures/tags.json`; the TS side pins the SAME file both at runtime
(`ts/test/wire-tags.test.ts:121-126`) and at the type level — 27 `Expect<Equal<…>>`
pins tie each roster to the corresponding TS union (`wire-tags.test.ts:91-118`). So
one JSON file locks Rust-enum ↔ wire-tag ↔ TS-union three ways.

Outbound values: `ValueOut` (marshal.rs:1191-1260) is a move-out mirror of
`bumbledb::Value` with a hand-written `ToNapiValue` (u64 → JS BigInt, i64 via
`i64n`, bytes → fresh `Uint8Array`, intervals → `{start,end}` objects). Query
answers cross as `Vec<Vec<ValueOut>>` decoded straight off the engine's flat
`Answers` buffer (`answers_out` marshal.rs:1268-1290). Non-UTF-8 at-rest strings
are refused typed rather than lossily repaired (marshal.rs:1202-1208 doc).

Prepared-query lane end to end:

1. TS `db.prepare(q)` → `lowerQuery(q)` → `native.dbPrepare(handle, queryIr)`
   (`ts/src/db.ts:1291-1299`) or, inside a read lease,
   `native.instancePrepare` (db.ts:967-968).
2. Bridge `db_prepare` (`ts/crate/src/lib.rs:1259-1268`) / `instance_prepare`
   (lib.rs:1248-1257) → `marshal::query_in` → `inner.db.prepare(&query)`.
   Outcome is the tagged `PrepareOutcome` (`{ok:true, prepared: External}` |
   `{ok:false, kind:"irError", message}`, lib.rs:1206-1214) — engine
   `Error::Validation` becomes the domain arm, anything else throws
   (`prepare_outcome` lib.rs:1237-1246).
3. TS keeps the `External<PreparedHandle>` in a WeakMap-keyed plan
   `{handle, owner, params, finds}` behind an empty frozen token object, with a
   `FinalizationRegistry` reclaiming the native plan (`ts/src/db.ts:694-710,
   819-845`); cross-store use is refused by owner identity (db.ts:738-749).
4. Execute: `wireParams` (`ts/src/query/run.ts:17-39`) maps the user's named params
   record → positional `QueryParam[]` in registry order, re-tagging every host value
   at its registered anchor (`wireValue` run.ts:8-15 → `taggedCmpLiteral`; baked
   membership sets short-circuit at run.ts:19-21). Bridge `prepared_execute`
   (lib.rs:1270-1284) → `marshal::params_in` (marshal.rs:543-561, `OwnedParam::
   Scalar|Set` — note a scalar param IS its tagged value, only `set` is an extra
   spelling, tags.rs:283-291) → `param_args` → `BindValue` borrows
   (lib.rs:210-230) → `execute_collect`/`execute` on the instance
   (lib.rs:695-711, 734-745).
5. Rows come back as `FactValue[][]`; `decodeAnswers` (run.ts:50-70) re-keys by
   find-column name and lifts closed ids → handle names via `handleOf`
   (`ts/src/marshal.ts:74-85`).

#### 1.2.5 The schema lane (parallel to the query lane)

- TS `schema(...)` values lower to the plain-data `SchemaSpec` of
  `ts/src/spec.ts:102-105` via `lower(theory)` (`ts/src/lower.ts:123-132`) — total,
  deterministic key order, declaration order throughout (lower.ts:1-13).
  Closedness is ONE fused sum on the wire (`ClosedSpec` spec.ts:65-78; absent
  `closed` = ordinary relation), mirrored verbatim by the bridge
  (marshal.rs:763-791) and by the core's fused `RelationSpec`.
- `db.create`/`db.open` (`ts/src/db.ts:1351-1408`) pass `(path, spec)` to
  `native.dbCreate`/`dbOpen`. The bridge parses the spec (`marshal::schema_spec`
  marshal.rs:746-808), resolves it (`spec.descriptor()`, `descriptor_of`
  lib.rs:307-325 splitting `SpecIssue::StatementNewtypeMismatch` into its own arm),
  and runs `Db::create`/`Db::open` on a libuv worker via napi `AsyncTask`
  (`CreateTask` lib.rs:345-405, `OpenTask` lib.rs:425-485). Outcomes are tagged
  unions built by the `outcome_to_napi!` macro (lib.rs:160-180): `CreateOutcome`
  (`accepted|rejected|schemaError|newtypeMismatch`, lib.rs:266-271), `OpenOutcome`
  (`ok:true|{schemaError|newtypeMismatch|fingerprintMismatch}`, lib.rs:290-295 —
  engine `Error::SchemaMismatch` is mapped to the `fingerprintMismatch` arm at
  lib.rs:447-449).
- The accepted handle wraps `Db<SchemaDescriptor>` plus a resident `Sealed`
  (`{descriptor, materialized statements, sealed field rosters}` — computed once at
  `seal()`, lib.rs:81-98) so the fact lane re-derives nothing per call.

#### 1.2.6 The fact lane (writes/reads of rows)

- TS flattens facts to ONE row-major `FactValue[]` plus an explicit `rows: bigint`
  (`rowsOf` `ts/src/db.ts:94-110`; law comment db.ts:82-93) — the explicit count
  exists because a fieldless relation projects N rows to 0 cells.
- Bridge `tx_insert`/`tx_delete` (lib.rs:1100-1134) call
  `marshal::accepted_collection` (marshal.rs:312-363): verifies
  `cells.len() == rows × arity` in u128 (marshal.rs:330-338), takes the arity-0
  count as data (O(1) `seal_nullary`, marshal.rs:339-354 — a stated 2^63 never buys
  2^63 pushes), and pushes cells type-directed by the resident sealed roster
  (`push_cell` marshal.rs:365-433) into the core's `CollectionBuilder`, sealing an
  `AcceptedCollection` the engine applies without re-judging shape.
- Point reads (`instance_get`/`tx_get`) marshal a key row against the statement's
  projection (`key_row` marshal.rs:455-508); scans return `rows_out`
  (marshal.rs:1262-1266).
- The TS marshal layer (`ts/src/marshal.ts`) is the ONE place fact objects ⇄
  positional rows convert (marshal.ts:1-28), including the closed bijection
  handle-name ⇄ u64 row id in both directions (`closedCellOf` marshal.ts:64-72,
  `handleOf` marshal.ts:74-85).

#### 1.2.7 Errors and outcomes across napi

Two channels, strictly separated:

- **Domain outcomes** (admission, open refusals, write results, prepare refusals)
  cross as tagged plain objects, never throws — `WriteOutcome`
  (`accepted|rejected|abandoned|moved`, lib.rs:931-947), `AdmitOutcome`
  (lib.rs:1515-1533), each with `ViolationWire` payloads
  (marshal.rs:1551-1657: statementId, kind, canonical spelling, direction, u128
  measure as BigInt, offending facts as name/value pairs).
- **Failures** throw a JS Error carrying a `kind` property from the
  `error_family` table: `throw_kind_message` builds the error object and throws it
  through the env (marshal.rs:43-58); the family tag comes from the exhaustive
  `error_family::tag` (tags.rs:293-322). TS lifts it back with `isEngineThrow`/
  `errorFromThrow` (`ts/src/native.ts:471-489`) and wraps every native call in
  `bridged`/`bridgedAsync` (native.ts:491-507). The TS union `ErrorFamilyKind`
  (native.ts:272-297) is the hand-mirrored twin, pinned by wire-tags.test.ts.

#### 1.2.8 Handles and lifetimes across napi

- Owned handles (`DbHandle`, `WitnessHandle`, `PreparedHandle`, `BuilderHandle`,
  `OwnedHandle`) are `External<T>` wrapping `RefCell<Option<Inner>>`; the shared
  verbs `take_handle`/`live`/`live_mut` (lib.rs:155-198) turn double-close,
  re-entrancy, and use-after-close into typed errors, never panics (tested,
  lib.rs:1642-1681). On the TS side each is an opaque branded type
  (native.ts:5-18).
- Callback-scoped capabilities are raw pointers + an `alive: AtomicBool` cleared
  before the owning engine frame returns: `InstanceHandle` stores
  `*const ReadInstance` (lib.rs:609-640), `TxHandle` stores `*mut WriteTx` and
  `*const Engine` (lib.rs:888-929, deliberately NOT an `Arc` clone so a stashed JS
  reference cannot keep the exclusive lock alive). `db_read` (lib.rs:757-811) and
  `run_write` (lib.rs:955-1026) call the JS callback synchronously inside the
  engine lease; a JS abort/throw crosses back out via an `abort_sentinel()` engine
  error (lib.rs:128-130) that is unwrapped into `Aborted` / rethrow.
- The single-writer rule is enforced bridge-side too (`writing` flag,
  lib.rs:965-972).
- Owned heap instances account their retained bytes to the JS GC
  (`adjust_external_memory`, lib.rs:1308-1332).

#### 1.2.9 The fingerprint lock (cross-host schema compatibility gate)

Three interlocking mechanisms:

1. **At open, engine-side:** the store's sealed fingerprint must match the offered
   descriptor's — `Db::open` returns `Error::SchemaMismatch`, which the napi bridge
   maps to the `fingerprintMismatch` open-outcome arm (lib.rs:447-449; TS type
   `DbOpenResult` native.ts:243-256; thrown as `ErrFingerprintMismatch` in
   db.ts:1330-1336).
2. **Cross-host pin:** `ts/crate/src/fingerprint_lock.rs` declares a full-width
   twin theory with the Rust `schema!` macro (fingerprint_lock.rs:6-83, exercising
   closed rosters, fresh ids, intervals, containment, weighted capacity, dependent
   bounds) and pins `PIN = "588df888…1418"` (fingerprint_lock.rs:90). The SAME
   constant is baked into `ts/test/fingerprint.test.ts:24` where the TS SDK builds
   the same theory and asserts `native.dbFingerprint` equals it
   (fingerprint.test.ts:105-114). Drift in either encoding surface fails one side's
   suite.
3. **Typestate interop test:** the bridge's exact typestate
   (`Db<SchemaDescriptor>`, what every JS `dbCreate` produces) and the macro twin
   open each other's stores, and a twisted twin (one statement fewer) refuses as
   `SchemaMismatch` (fingerprint_lock.rs:127-149).

The C surface exposes the same digest as `bdb_db_fingerprint` (64 hex chars,
`crates/bumbledb-c/src/db.rs:860-870`), so a C host can implement the same gate.

#### 1.2.10 Native loading and the internal exports

- `ts/src/native.ts:413-440`: the binding is `require`d (via
  `createRequire(import.meta.url)`) from the platform package
  `@bjornpagen/bumbledb-${process.platform}-${process.arch}`;
  `SHIPPED_PLATFORMS = ["darwin-arm64", "linux-arm64"]` (native.ts:413). Resolution
  failure and load failure are separate wrapped errors (native.ts:424-436). Loading
  happens at module import, once (native.ts:439-440).
- The platform packages (`ts/npm/darwin-arm64`, `ts/npm/linux-arm64`) each ship
  exactly `bumbledb.node` (`"main": "bumbledb.node"`, `os`/`cpu` gates,
  `ts/npm/darwin-arm64/package.json`). `ts/scripts/build.ts` builds
  `ts/crate` with `cargo build --release` (after `cargo clean -p bumbledb-node` so
  `engineVersion()`'s baked CARGO_PKG_VERSION cannot go stale) and copies the
  cdylib to `npm/<platform>/bumbledb.node`; `assertVersionLockstep` requires
  "main == platform == napi crate == engine == C ABI" — root
  `[workspace.package] version = 0.19.2` == `ts/crate` 0.19.2 == platform packages
  0.19.2 == `crates/bumbledb-c` 0.19.2.
- Two `@doc(hidden)` internals are deliberately exported for the replication driver
  (`@bjornpagen/bumbledb-log`, the ts-log package) so the SDK ships exactly one
  hash and one seal authority:
  - `internalBlake3` (native.ts:446-458) → `#[napi] blake3_hash`
    (`ts/crate/src/lib.rs:37-48`, `bumbledb::digest::Digest`). Consumed at
    `ts-log/src/writer.ts:22,520,753`, `ts-log/src/store.ts:21,118`,
    `ts-log/src/replica.ts:14,134-138`.
  - `internalDescriptor` (native.ts:459-469) → `#[napi] descriptor`
    (lib.rs:50-79): runs the pure seal path (no store) and returns
    `DescriptorWire = {relations (manifest form), statements (materialized,
    id-ordered), fingerprint hex}` (marshal.rs:1514-1549). Consumed at
    `ts-log/src/descriptor.ts:20,265`; the TS type `SealedDescriptor` is
    native.ts:204-208.
  - Both are re-exported from the package root (`ts/src/index.ts:136`) while "the
    raw native bridge is not exported" (index.ts:17).

### 1.3 The Rust host surface (`query!` — the compile-time bridge)

`crates/bumbledb-query` is a 7-line facade (`src/lib.rs`) over
`crates/bumbledb-query-macros`, a dependency-free proc macro
(`bumbledb-query-macros/Cargo.toml` — no syn/quote, hand-rolled token parser).

- The notation grammar is the module doc (`bumbledb-query-macros/src/lib.rs:6-75`):
  `interior* rec* barerule+`, `(head) | body;` rules, punning bindings, selection
  (`field == value`), set params (`field in ?p`), condition trees `and()/or()`,
  Allen masks as `MASK | MASK` unions, and derived tables addressed positionally.
  Datalog's `:-` is refused with one message everywhere (lib.rs:97-104).
- Expansion emits SOURCE TEXT that constructs the core IR literally: `expand`
  (lib.rs:2073-2099) → `emit_cq` (lib.rs:2126-2143, producing
  `::bumbledb::Query::cq(vec![Interior…], Rule::head(&rules[0]), rules)`) or
  `emit_reach` (lib.rs:2145-2181, wrapping `NonEmpty::from_vec` at lib.rs:2059-2063).
  Every constructor is spelled absolute: `::bumbledb::Term::Var(::bumbledb::VarId(n))`
  (lib.rs:1349-1358), `::bumbledb::Atom { source: AtomSource::Edb(...) }`
  (lib.rs:1571-1573), `ConditionTree::Leaf(Comparison{...})` (lib.rs:1586-1591),
  `FindTerm::Aggregate { op: FoldOp::…, over: VarId(n) }` (lib.rs:1643-1648),
  `Rule{…}` / `ProjectionRule{…}` / `RecRule{…}` / `RecStep{…}` (lib.rs:1699-1786).
- **Ids are resolved through the theory's generated constants, so name resolution is
  the Rust compiler's:** an EDB atom becomes
  `AtomSource::Edb({Theory}::{RELATION_SCREAMING_SNAKE})` and a binding field
  `{Theory}::{RELATION}_{FIELD}` (lib.rs:1527-1535, 1569). Those constants are
  minted by the `schema!` macro in `crates/bumbledb-macros/src/lib.rs:2166-2190`
  (`pub const SAVINGS_TERMS: RelationId = RelationId(n)` etc.); closed handles
  resolve through generated handle enums (`Kind::Focus.id().0`, query macro
  lib.rs:1409-1417). A typo'd relation/field is a compile error; a schema change
  recompiles every query against it.
- Derived-table names are macro-LOCAL (lib.rs:62-67): interiors resolve to dense
  `InteriorId`s at expansion (lib.rs:1501-1508); lowercase atom names that match no
  derived table are refused (lib.rs:1510-1526). Var ids are interned first-use per
  rule by `Scope`; head vars must be bound (`projection_vars` lib.rs:1652-1676
  refuses aggregates in derived heads — the same wall the TS builder has at
  `ts/src/query/lower.ts:1047-1063` and the bridges re-state).
- Params: `?name` / `?0` both resolve to dense `ParamId`s (`Params::resolve`,
  positional-vs-named mixing refused — see compile-fail tests
  `bumbledb-query/tests/compile-fail/mixed_params_*`). The refusal corpus
  (`bumbledb-query/tests/compile-fail/*.rs`, ~30 cases) plus `notation.rs`/
  `notation_corpus.rs`/`cookbook.rs` are the macro's conformance battery.

So the Rust host does not *marshal* at all: the "bridge" is code generation into the
core's own constructors, checked by rustc, with zero runtime crossing cost. The
engine's IR validator at `Db::prepare` still runs behind it (macro doc
lib.rs:2183-2188: "everything semantic beyond names surfaces as the validation
roster's typed errors at `Db::prepare`").

### 1.4 The C surface (`crates/bumbledb-c`)

`bumbledb-c` (staticlib + cdylib, sole dependency `bumbledb`,
`crates/bumbledb-c/Cargo.toml:29-31`) exports `bdb_*` symbols; the committed header
`crates/bumbledb-c/include/bumbledb_c.h` (966 lines) is GENERATED by cbindgen.

**Boundary protocol** (header banner, bumbledb_c.h:1-33 — sourced from
cbindgen.toml:19-52; enforced by the shared combinators in `src/lib.rs`):

- Every fallible export returns `bdb_status` (`Ok|Error|Aborted|Misuse`,
  lib.rs:73-80) and takes a trailing `bdb_error**` out-param. Contract violations
  (null required pointer, stale ref, unknown tag, bool ≠ 0/1, misaligned/oversized
  slice) are `MISUSE` with no allocation; real failures write a caller-owned
  `bdb_error*`. `guard` catches panics into typed errors (lib.rs:105-127).
- One small vocabulary of pointer combinators does ALL unsafe:
  `ref_in`/`mut_in`/`slice_in`/`out`/`box_out_to`/`box_in`/`require_out`
  (lib.rs:181-289); everything else in the crate is `unsafe_code = "deny"` with
  per-site `#[expect]`s.
- `c_tag!` (lib.rs:51-69) generates `u32 ⇄ enum` conversions per tag enum; wire
  fields are `u32` so an out-of-range C enum is MISUSE, not UB (cbindgen.toml:55-57).

**Query IR crossing** (`src/query.rs`): the C view structs "mirror `bumbledb::ir`
1:1 — relations, fields, and interiors by numeric id" (query.rs:2-3):
`bdb_term` (query.rs:40-47, flat struct: kind + var + param + literal, only the
field the kind names is read), `bdb_binding` (52-56), `bdb_atom` (69-78),
`bdb_find_term`/`bdb_head_term` (125-150), `bdb_cmp_op` with the literal 13-bit
Allen mask (181-186), `bdb_condition` (nested `*const bdb_condition` children,
212-219, depth re-checked against `MAX_CONDITION_DEPTH` at query.rs:394-400),
`bdb_rule` (223-234), `bdb_interior`/`bdb_rec` (237-256), and the tagged union
`bdb_query{kind, payload: union{cq, reach}}` (query.rs:294-308; union arms read
under `#[expect(unsafe_code)]` fenced by the kind, query.rs:581-618). `query_in`
rebuilds `bumbledb::Query` exactly as the napi bridge does — the same
`vars_only`/no-negation-in-rec/self-atom-split walls (query.rs:452-507),
including the same interior-count → rec `InteriorId` law (query.rs:572-575 twin of
marshal.rs:1173-1177).

**Prepared queries:** `bdb_db_prepare` (query.rs:673-701) validates/plans once and
returns an opaque `bdb_prepared` that pins its owner (`OwnerToken`,
db.rs:33-39) and holds the engine `Arc` alive (query.rs:624-629);
`bdb_instance_execute` (answers.rs:115-143) checks owner identity bridge-side
(the engine's `ForeignPreparedQuery` refusal, answers.rs:133-135), claims an
in-execute exclusion flag (query.rs:631-666), and fills the caller's reusable
`bdb_answers` carrier (answers.rs:12-15) — cleared first, capacity retained.
Cells are read back through the bounds-checked `bdb_answers_get`
(answers.rs:75-95): string/bytes payloads BORROW the carrier (view-lifetime
contract, header bumbledb_c.h:29-33).

**Values:** `bdb_value` (value.rs:88-110) is one flat `#[repr(C)]` POD ("no union,
no packing") carrying every `Value` variant both directions; inbound copies
(`value_in` value.rs:136-162, empty intervals refused at the bridge), outbound
borrows (`value_out`/`answer_out` value.rs:190-272). Params mirror the engine's
public `ParamArg` (`bdb_param` value.rs:285-296; `params_in` → `OwnedParam` →
`param_args` value.rs:305-335 — the same three-step the napi bridge uses).

**Schema:** `bdb_schema_spec` and sub-views mirror `SchemaSpec` field for field
(schema.rs:1-4), copied IMMEDIATELY into Rust-owned specs (no caller memory
survives the call); closedness is the same fused sum (null `closed` = ordinary,
schema.rs:250-269); `bdb_db_create`/`bdb_db_open` (db.rs:741-804) run
`schema_spec_in → spec.descriptor() → Db::create/open` (db.rs:271-275) and fill
tagged admission unions (`bdb_db_admission` etc., db.rs:130-189 — `Empty = 0` is
documented as never returned under `BDB_STATUS_OK`).

**Callbacks and capabilities:** reads/writes take extern-"C" function pointers
invoked from Rust (db.rs:204-218); the callback mints a `bdb_instance_ref` /
`bdb_tx_ref` (AtomicPtr + alive flag, db.rs:72-97) that is invalidated on return —
a stashed pointer answers MISUSE, not UAF (header bumbledb_c.h:17-23); a returned
`Abort` tag surfaces as `BDB_STATUS_ABORTED` (the ts bridge's abort sentinel
"spelled as control flow", lib.rs:82-93). Witnesses can outlive the callback only
via `bdb_witness_retain` (db.rs:1252). Stale refs a callback leaked are parked on
the db handle's `retired` list and leaked at destroy rather than freed under a
possible C pointer (db.rs:46-57, 840-852).

**Errors:** `bdb_error` is opaque `{origin: Engine|Bridge, kind, message}`
(error.rs:119-125); `bdb_error_kind` (error.rs:26-54) is one constant per engine
family plus bridge-synthesized `Panic|BusyHandle|Marshal`, mapped exhaustively from
`ErrorFamily` (error.rs:127-150+). The file states the mirroring cost outright:
"The kind table is the FOURTH spelling of the engine taxonomy (Rust enum,
TypeScript union, tags.json, this C header)" (error.rs:4-5). Violations are a
separate owning carrier `bdb_violations` with per-kind payload unions
(error.rs:85-117), matching the napi `ViolationWire`.

**cbindgen + the GENERATION discipline:**

- `cbindgen.toml`: language C with `cpp_compat`, pinned tool "cbindgen 0.29.4
  (installed via `cargo install --locked cbindgen`)", regeneration command
  documented in the file header (cbindgen.toml:1-9); the whole boundary protocol
  is restated in the emitted header banner (cbindgen.toml:19-52); tag enums are
  force-included because cbindgen drops unused types (cbindgen.toml:54-92); enum
  variants render `SCREAMING_SNAKE` prefixed with the type name
  (cbindgen.toml:93-95); `parse_deps = false` — only this crate's types enter the
  header.
- `bdb_abi_version()` returns the hand-bumped ABI generation, currently 4
  (lib.rs:42-49): "`4` is the 0.17.0 purge: the measure/duration family left the
  query surface, so `bdb_error_kind` and `bdb_find_term_kind` renumbered — a host
  compiled against the generation-3 header misreads those tags and must
  recompile." The discipline: C enum VALUES are the ABI; any core-enum change that
  renumbers a `bdb_*` tag enum requires a generation bump and host recompile, and
  the doc comment must record what renumbered and why. `bdb_version()`
  (lib.rs:25-34) mirrors the Node bridge's `engine_version` as a printable string.
- ~2000 lines of in-crate tests (`src/tests.rs`) drive the ABI from the Rust side
  as a stand-in C host.

---

## 2. The recipe: putting a NEW subsystem behind this bridge

What it concretely takes, in the order the engine did it. "Subsystem" below means a
new lane like the query IR or the schema spec — a coherent payload family plus the
operations over it.

### Step 1 — Give the core a parse-once, pure-data boundary type

Files: the core crate (`crates/bumbledb-log/src/…` for the log).

- Define plain-data structs/enums with numeric ids and NO host names (the IR
  pattern: `crates/bumbledb/src/ir.rs`), or name-carrying spec structs resolved
  once by a `spec.descriptor()`-style function (the schema pattern:
  `bumbledb::schema::spec`). Sum types over "maybe" fields — make illegal states
  unspellable (the fused `ClosedSpec` ruling, ts/src/spec.ts:65-74;
  `Query{rec: Option<Rec>}` vs a boolean).
- Put ALL semantic validation in one core entry point that returns a typed error
  roster (the `Db::prepare` IR validator). Bridges will only check shape.
- Export the types from the core's root (`crates/bumbledb/src/lib.rs:107-111`
  style) plus any limits bridges must re-check for stack safety
  (`MAX_CONDITION_DEPTH`, ir.rs:40).
- If hosts submit bulk data, add an `AcceptedCollection`-style parse-once builder
  (doc-hidden, lib.rs:79-82) so the bridge proves shape once and the engine never
  re-judges.
- Give errors a closed `ErrorFamily`-style enum and domain outcomes
  (`Admission`-style sums) — these become wire tags on every surface.

### Step 2 — napi bridge (ts/crate)

Files touched: `ts/crate/src/lib.rs`, `ts/crate/src/marshal.rs`,
`ts/crate/src/tags.rs`, `ts/test/fixtures/tags.json`.

1. For every mirrored core enum, add ONE `wire_tags!` table in tags.rs (camelCase
   string tags; unit enums get generated `parse()` too). Extend the golden test's
   `tables()` list (tags.rs:361-394) and the committed `tags.json`.
2. In marshal.rs, write the inbound walker (`req`/`req_at` over
   `Object`/`Array`; numbers via `ordinal`/`u16_id`, BigInt via `u64_in`/`i64_in`;
   recursion depth re-checked against the core's limit) and the outbound form:
   either a `ValueOut`-style move-enum with hand `ToNapiValue`, or an
   `outcome_to_napi!` tagged object for outcome sums (lib.rs:160-180).
3. In lib.rs, add `#[napi]` functions. Rules of the house:
   - handles = `External<X>` over `RefCell<Option<Inner>>`, lifecycle via
     `take_handle`/`live`/`live_mut` (lib.rs:155-198);
   - callback-scoped capabilities = raw pointer + `alive: AtomicBool` cleared
     before the engine frame returns (lib.rs:609-640, 888-929);
   - blocking engine calls that JS awaits = napi `AsyncTask` with a
     `Prep → compute (off-thread, no napi types) → resolve (on-thread, mint
     Externals)` split (CreateTask lib.rs:345-390);
   - domain outcomes return tagged objects; failures throw `{kind, message}` via
     `throw_kind_message` (marshal.rs:43-58);
   - keep a resident `Sealed`-style cache on the handle for anything the per-call
     path would otherwise re-derive (lib.rs:81-98).

### Step 3 — TypeScript SDK (ts/src)

Files touched: `ts/src/native.ts` (or the subsystem's twin), a builder module, a
lowering module, a run/decode module, `ts/src/index.ts`, `ts/test/wire-tags.test.ts`.

1. Hand-declare the `Native` interface additions (camelCase of the `#[napi]`
   snake_case names) and the wire types: string-`kind` tagged unions mirroring the
   tags.rs tables, `bigint` for u64/i64, `number` for ids, branded opaque handle
   types (native.ts:5-18), a `ParsedX` brand if there's a structural pre-check.
2. Add the type-level `Expect<Equal<roster, union>>` pins and runtime golden
   assertions to wire-tags.test.ts so the new unions are locked to tags.json.
3. Build the typed builder → frozen data → `lowerX()` id-assignment pipeline
   (scope/atom/find/lower pattern). Two tiers, deliberately:
   type-level judgments in the interfaces, runtime twins for what types cannot see
   (object identity, boundness), joined by narrow "trusted admission seam" guards
   (`isTypedScope` lower.ts:1098-1120, `varsMinted` scope.ts:79-83).
4. Wrap every native call in `bridged`/`bridgedAsync`; convert domain-outcome tags
   to typed SDK results; decode positional rows back to named records in ONE
   marshal module (ts/src/marshal.ts pattern).
5. Keep native-plan lifetimes on WeakMap + FinalizationRegistry keyed by empty
   frozen token values, with owner-identity checks (db.ts:694-710, 738-749).
6. Export only the typed surface from index.ts; `@internal` exports (the
   `internalBlake3`/`internalDescriptor` pattern, native.ts:446-469) for sibling
   packages that must share one authority.

### Step 4 — Rust host sugar (optional but the engine's precedent)

Files: a `bumbledb-log-…-macros` proc-macro crate + a re-export facade crate.

- The macro parses a notation and EMITS SOURCE that calls the core's own
  constructors with `::absolute::paths` (query-macros lib.rs:1349-1786), resolving
  names through constants the core's declaration macro generated
  (bumbledb-macros lib.rs:2166-2190). No runtime, no marshaling, no drift: a new
  core field breaks the emitted code's compile.
- Ship a compile-fail refusal corpus (`tests/compile-fail/*`) and a notation
  conformance suite.

### Step 5 — C surface (crates/bumbledb-c or a sibling)

Files touched: `src/<subsystem>.rs`, `src/lib.rs` (module list),
`cbindgen.toml` (`[export] include` for every new tag enum),
`include/bumbledb_c.h` (regenerated), `src/tests.rs`.

1. Mirror each core type as a `#[repr(C)]` view: flat structs with `u32 kind` +
   per-arm fields (bdb_term/bdb_value style) for hot small types, pointer+count
   pairs for sequences, tagged unions only for large alternatives (bdb_query).
   `c_tag!` per enum. Strings/bytes as view structs with the null/len contract
   (value.rs:10-19).
2. One `X_in` copier per view (borrowed C memory never survives the call) and one
   `X_out` borrower per owned carrier, with the carrier lifetime documented at the
   accessor.
3. Exports follow the boundary protocol: `guard(out_error, || …)`,
   `bdb_status` returns, out-params via `require_out`/`box_out_to`, destroy verbs
   via `box_in`, busy flags before `&mut` (query.rs:639-666).
4. Regenerate the header with the PINNED cbindgen version and commit it; if any
   existing tag enum renumbered, bump `bdb_abi_version` and record why in its doc
   comment (lib.rs:36-49).

### Step 6 — Locks and packaging

- If the subsystem has a canonical digest/encoding, add a fingerprint-lock pair:
  one pinned constant in a bridge-crate Rust test, the SAME constant in a TS test
  (fingerprint_lock.rs:90 ↔ fingerprint.test.ts:24), plus a cross-open/cross-decode
  interop test.
- Version lockstep: the napi crate, platform packages, C crate, and workspace share
  one version; `ts/scripts/build.ts` asserts it and reminting is part of the bump.

---

## 3. What is shared-compiled vs hand-mirrored today (engine side)

### Shared-compiled (one definition; drift is a compile error)

| Artifact | Where defined | Who compiles against it |
|---|---|---|
| Query IR (`Query`, `Rule`, `Term`, `Atom`, …) | `crates/bumbledb/src/ir.rs` | ts/crate marshal.rs (imports at marshal.rs:13-19), bumbledb-c query.rs (imports at query.rs:7-11), query! macro OUTPUT (expanded in the host crate) |
| Schema spec + descriptor + seal + fingerprint | `bumbledb::schema` | both bridges call the same `spec.descriptor()` and `fingerprint()` (ts/crate lib.rs:307-325, 70-77; bumbledb-c db.rs:271-275) |
| `Value`/`AnswerValue`/`ParamArg`/`BindValue` | `crates/bumbledb/src/value.rs`, api::prepared | both bridges' value lanes are exhaustive matches over them (marshal.rs:1209-1226; value.rs:190-272) — a new variant breaks both compiles |
| `ErrorFamily`, `Violations`, admission sums | `bumbledb::error` | tags.rs error_family table (exhaustive, no wildcard) and bumbledb-c `kind_of` (error.rs:127+) both break compile on a new family |
| Engine handles + engine typestate | `bumbledb::api` | both bridges instantiate `Db<SchemaDescriptor>` (ts/crate lib.rs:100; bumbledb-c db.rs:24) — proven interoperable by fingerprint_lock.rs:127-141 |
| Rust-host ids | `schema!`-generated `RelationId`/`FieldId`/`StatementId` consts (bumbledb-macros lib.rs:2166-2190) | `query!` splices them by name (query-macros lib.rs:1527-1535) |
| blake3 + sealed descriptor for the log driver | `bumbledb::digest`, seal path | lent through the napi bridge (`blake3_hash`, `descriptor`) so ts-log never re-implements them (ts/crate lib.rs:37-79) |

### Generated-but-committed (drift possible only by skipping a documented step)

- `include/bumbledb_c.h` — cbindgen output, pinned tool 0.29.4, regeneration
  command in cbindgen.toml:1-9 and in the header banner itself. Manual step; the
  in-crate tests exercise the Rust side of the ABI but nothing diffs the committed
  header against fresh output automatically.
- The C ABI generation integer (`bdb_abi_version`, lib.rs:42-49) — hand-bumped,
  hand-documented.

### Hand-mirrored (twin declarations; locked by tests/goldens, not the compiler)

| Twin | Sides | Lock |
|---|---|---|
| Wire tag rosters | tags.rs `wire_tags!` tables ↔ TS unions in native.ts/spec.ts ↔ C tag enums | `ts/test/fixtures/tags.json` golden, asserted from Rust (tags.rs:396-418) and TS (wire-tags.test.ts:121-126) with 27 type-level pins (wire-tags.test.ts:91-118); C side has no golden — only the exhaustive `tag()`/`c_tag!` compile breaks |
| Error taxonomy | "the FOURTH spelling" — core enum, `ErrorFamilyKind` TS union (native.ts:272-297), tags.json, `bdb_error_kind` (bumbledb-c error.rs:26-54) | tags.json golden covers Rust↔TS; the C enum relies on the exhaustive `kind_of` match |
| Query IR wire shape | `QueryIr` et al. (native.ts:40-131) ↔ marshal.rs walkers ↔ bdb_* structs (query.rs) | integration tests + the engine validator; no schema-of-the-wire artifact exists |
| `SchemaSpec` wire shape | ts/src/spec.ts:3-105 ↔ marshal.rs:746-808 ↔ bumbledb-c schema.rs views | same |
| Native function surface | `#[napi]` exports ↔ TS `Native` interface (native.ts:303-411) | none mechanical — a signature drift surfaces at runtime in tests |
| Builder-side semantic walls | TS construction-time validations (closed-order ban lower.ts:764-772, boundness, rec cuts) are declared "runtime twins" of the engine's rosters (atom.ts:357-366) | the engine's own refusal always stands behind them; the TS copies exist for error locality |
| Fingerprint pin | fingerprint_lock.rs:90 ↔ fingerprint.test.ts:24 | the two suites |
| ABI misc | `SHIPPED_PLATFORMS` (native.ts:413) ↔ ts/npm/* dirs; version lockstep | build.ts assertions |
| **bumbledb-log itself** | `crates/bumbledb-log/src/*` ↔ `ts-log/src/*` (writer/codec/manifest/replica/braids/…) | nothing but tests — the whole protocol is spelled twice; the only shared pieces are the two internal napi exports. This is the mirror the refactor exists to retire. |

---

## 4. Warts worth NOT replicating

1. **The head is dead weight on the wire.** Interior and rec heads are lowered,
   marshaled, and then DISCARDED by both bridges (`let _ = head_in(...)`
   marshal.rs:1126 and 1150; `let _ = head_in(...)` bumbledb-c query.rs:514 and
   524) — the core `Interior`/`Rec` types don't carry a head (the engine
   recomputes it from finds). Three surfaces carry, validate, and align a field
   nobody consumes. For the log bridge: never put data on the wire the core type
   cannot hold.

2. **Two integer disciplines on one boundary.** On the JS wire, u64/i64 payloads
   are BigInt (checked by `u64_in`/`i64_in`) but every ID (relation, interior,
   var, param, field, mask) is a JS `number` squeezed through `ordinal()`'s f64
   integrality check (marshal.rs:113-127) and then often through `u16_id`.
   It works, but each id site re-states the guard, and the TS types can't
   distinguish id spaces (`var: number`, `param: number`). Pick one scalar
   discipline (or branded id types) for a new lane.

3. **~1200 lines of hand-written object walking per direction.** marshal.rs's
   `req`/`req_at` traversal and the emit side of `outcome_to_napi!` are
   hand-maintained against hand-maintained TS twins. The tags golden locks the
   *tag strings* but nothing locks *payload keys* (`"over"`, `"bindings"`,
   `"endExclusive"`, …) — a renamed key drifts silently until an integration test
   trips on a "missing `x` in y" error. If the log payloads are large or evolve
   fast, consider a single self-describing encoding (or generating both twins
   from one declaration) instead of two hand mirrors + a partial golden.

4. **The C generation integer is a tripwire, not a fence.** Enum ORDER is the C
   ABI; generation 4 exists precisely because removing core variants silently
   renumbered `bdb_error_kind` and `bdb_find_term_kind` (lib.rs:36-49). Nothing
   mechanical detects a renumbering — the discipline is a comment. A new surface
   should either pin explicit discriminant values on every `bdb_*` tag enum or
   add a golden of `(enum, variant, value)` triples.

5. **The committed generated header has no freshness check.** Regeneration is a
   documented manual command (cbindgen.toml:1-9). A CI diff of committed vs
   regenerated header is cheap and absent.

6. **Overloaded tag namespace for params.** `QueryParam = TaggedValue |
   {kind:"set"}` (native.ts:38) means `"kind"` carries value kinds AND the set
   marker in one field; the golden must special-case it ("`scalar` never appears
   on the wire", tags.rs:359-367) and `params_in` string-compares `kind == "set"`
   before falling through to value parsing (marshal.rs:546-559). One extra
   nesting level (`{kind:"scalar", value}` | `{kind:"set", values}`) would have
   cost nothing and kept every roster uniform. (The C surface got this right:
   `bdb_param_kind { Scalar, Set }`, value.rs:278-283.)

7. **The lexical-capability pattern is re-implemented by hand per bridge.** Raw
   pointer + alive-flag + "cleared before the frame returns" appears four times
   (InstanceHandle, TxHandle in ts/crate lib.rs:609-640/888-929; bdb_instance_ref,
   bdb_tx_ref in bumbledb-c db.rs:72-97, 640-687), each with its own unsafe
   justifications, plus bumbledb-c's retired-list leak strategy (db.rs:46-57,
   840-852). It is sound but expensive to review; a new subsystem with
   callback-scoped state should extract this into one shared abstraction (or
   avoid callback-scoped borrows entirely if the log's API shape allows handles
   with real ownership).

8. **The `ParsedQuery` brand is a cast, not a seal.** parse-ir.ts:22 stamps the
   brand with `ir as ParsedQuery`; any TS code can do the same cast and skip the
   structural check. Fine as a lint-level convention; do not mistake it for a
   guarantee when replicating.

9. **TS re-states engine validation for error locality.** The builder's boundness/
   closed-order/head-alignment walls are deliberate duplicates of engine rosters
   ("the construction-time validations in `#query/lower.ts` are that ban's runtime
   twin", atom.ts:363-366). The engine still refuses behind them, so drift is
   safe-but-confusing (two differently-worded errors for one law). Budget for
   this cost consciously: every semantic rule added to the core may want a TS twin
   plus matching prose.

10. **`Object.keys` order as the relation-ordinal law.** `lowerQuery` derives
    relation ids from `Object.keys(theory.relations)` insertion order
    (lower.ts:1894-1897) matching `lower(theory)`'s `Object.entries` order
    (ts/src/lower.ts:124). It is correct under JS string-key semantics and the
    schema builder's construction, but the law lives implicitly in iteration
    order at two sites rather than in one minted id table. For the log, mint ids
    once, in one place, and pass them.

11. **Async is bolted on per-call-site.** Only create/open/publish/admit are
    `AsyncTask`s; prepare/execute/scan are synchronous on the JS thread. That is a
    reasonable engine tradeoff (reads are lease-scoped callbacks), but the split
    is invisible in the TS types (`Promise` vs not) and each new async export
    needs the full Task boilerplate (three types + two impls, lib.rs:337-405).
    If the log lane is IO-heavy (object stores), decide the sync/async split up
    front and consider a shared Task helper.
