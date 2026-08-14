# SDK representation audit — leftover wave (schema, closed, statements, answers, write, napi, raii)

Scope: the SDK layers wave 1 skipped. `ts/crate/src/` (marshal beyond the `has_over` cite already in sdk-008), `cpp/foreign/raii.cc`, `cpp/src/schema/`, `cpp/src/closed/`, `cpp/src/db/` (except `prepare` of `query_value`, sdk-001), `cpp/src/answers/`, `cpp/src/relation/`, host `ts/src/schema.ts` / `closed.ts` / `statements.ts` / `db.ts` / `marshal.ts` / answers, `crates/bumbledb-query/` beyond ParsedRule / param-style (sdk-014 / sdk-015), fingerprint files.

Doctrine applied: representation first; illegal states unrepresentable; a bool where a sum belongs is the defect's shape. Validation that discards its proof is a defect. Wave-1 query findings sdk-001–022 are **not** re-filed. Sites already named there (`has_over` marshal, `has_rec` on `query_value`, ABI optionals) are noted as already owned, not new ids.

What this cut already got right (not counted): TS `StatementData` / `WeightSpec` / `CapacityWindowSpec` / `LiteralSpec` are tagged sums; `closed()` makes `.where` absent on the bare tier by type; `WriteResult` / `WriteDecision` / C++ `Answers` `Value` are variants; NAPI `schema_spec` parses closedness as `Option` and statements by kind; raii `owned_statement` is already `std::variant<fd, containment, capacity>` and `owned_relation.closed` is `std::optional`; the cross-host fingerprint lock is one pin, two pipelines; `crates/bumbledb-query/src/lib.rs` is a re-export (the leftover lives in the macros crate). The leftover is the schema-lane twin of Program: a flattened table of tag-plus-all-payloads, a closed flag beside a full `closed_info`, `==` as a bool, and `query!` HeadTerm still encoding Count-nullary as `over: None`.

---

## 1. C++ `relation_data` is a Program flag-machine — closedness is a bool beside a full `closed_info`

**Where:** `cpp/src/schema/spec.cc:66-72`; `cpp/src/schema/schema.cc:233-234`; `cpp/src/schema/classes.cc:37`; `cpp/src/db/wire.cc:140-165`

**Wrong:** An admitted relation is a sum: ordinary (declared fields, writable) or closed (sealed roster, axioms, no insert). C++ stores both arms at once:

```
struct relation_data {
    name_text name;
    std::size_t field_count;
    std::array<field_data, max_relation_fields> fields;
    bool closed;
    closed_info closed_data;   // always present
};
```

`closed == false` still carries `closed_info` (8 handles + 32 axiom slots). `closed == true` with a default-empty `closed_data` is well-typed. `schema()` backfills `closed_data` in a second walk (`schema.cc:233-234`) after `relation_entry` already set the flag (`classes.cc:37`). `wire.cc:141` then *re-derives* the raii sum: `std::optional<owned_closed>` from the flag, which is the representation the dialect threw away. That is `has_rec` + leftover `rec_ir` (sdk-001), on the schema table.

**Collapsing representation:** `relation_data` is a sum, or `closed_data` is a member only when `closed` is a type-level `true` (the phase-machine move). raii's `owned_relation.closed: std::optional<owned_closed>` is the host spelling they already used one layer down. Empty `closed_info` on an ordinary relation is unrepresentable.

**Essential vs accidental:** Accidental. NTTP wants trivial types; it does not require a bool plus a max-sized axiom array on every ordinary relation. TS already spells `closed: ClosedSpec | undefined` as the kind (R7).

**Severity:** high

---

## 2. Discriminator-plus-all-payloads across the C++ schema lane (spec, raii, where, manifest)

**Where:** `cpp/src/schema/spec.cc:51-57` (`field_data`), `:79-87` (`selection_literal`), `:142-167` (`bound_data` / `window_data` / `statement_data`), `:175-179` (`class_entry.classed`); `cpp/src/closed/axioms.cc:24-30` (`axiom_literal`); `cpp/src/closed/where.cc:27-32` (`where_slot.bound`); `cpp/src/relation/classify.cc:36-39` (`field_class.width == 0` = general interval); `cpp/src/db/manifest.cc:24-28` (`StatementRow.is_key`); `cpp/foreign/raii.cc:860-868` (`owned_literal`), `:935-958` (`owned_weight` / `owned_bound` / `owned_capacity_window`)

**Wrong:** Every schema-lane "sum" is a struct with a tag and every alternative's fields. `statement_data` holds source, target, bidirectional, weight, weight_field, and window at once — a key statement carries a dummy capacity window. `selection_literal` / `axiom_literal` / `owned_literal` hold handle + bool + u64 + i64 + text at once (`is_handle` is the flag). `bound_data` holds `lit` and `field` for every kind. `window_data` always has both bounds. `class_entry` uses `bool classed` as an existence flag beside `class_name`. `field_class.width == 0` is a sentinel for "general interval" (the engine's `Option<u64>`). `where_slot` is sdk-009's wildcard: `bool bound` plus a leftover `selection_literal`. `StatementRow` is `bool is_key` plus relation/projection that only keys read.

`cpp/AGENTS.md` §8 blesses `std::variant` and forbids "manual discriminator + payload structs". raii *did* use a variant for `owned_statement` and an optional for closedness — then rebuilt `owned_literal` as the same Minsky product the dialect already had. NTTP/consteval wants trivial types. That is a constraint, not a representation.

**Collapsing representation:** `std::variant` (or per-alternative arrays) for statements, literals, bounds, windows, field types. `where_slot` is `std::optional<selection_literal>` — unbound is absence, not a flag. `StatementRow` is `Key { relation, projection } | Other`. `width == 0` dies: general vs fixed-width are cases, matching engine `ValueType::Interval { width: Option<u64> }` as a sum of two interval kinds at the dialect (the Option is the engine boundary). raii `owned_literal` becomes the same variant the statement enum already is.

**Essential vs accidental:** Accidental relative to dialect law. Flat C ABI structs stay (C1 / sdk-008's essential-C ruling). Reconstructing them as the *recorded* schema IR is not.

**Severity:** medium

---

## 3. Schema sugar caps: `max_closed_handles = 8` (the `max_query_rules` that survived onto the schema)

**Where:** `cpp/src/schema/spec.cc:18-33`; `cpp/src/closed/axioms.cc:13-18`; `cpp/src/schema/classes.cc:22,44-45`; `cpp/src/schema/key.cc:59`; `cpp/src/schema/face.cc:38,53`; `cpp/src/closed/where.cc:18,154-155`

**Wrong:** Engine: `MAX_EXTENSION_ROWS = 256`, `MAX_DETERMINANT_WIDTH = 496` bytes, no SDK field-count cap of 16, no face-selection cap of 4. C++ schema sugar invents `max_projection_width = 8`, `max_relation_fields = 16`, `max_face_selections = 4`, `max_selection_literals = 4`, `max_closed_handles = 8`, `max_closed_columns = 4`. Comments: "Phase-C capacity" / "Phase-F capacity; the engine's bound is far higher." That is a second theory of size — sdk-012's exact sentence, on a different table. A closed vocabulary of 9 handles is a consteval trap (`face_has_too_many_selection_bindings` / `relation_exceeds_max_relation_fields`) on a type that could have used the engine number, or a pack length, as the bound.

`max_relation_fields` is also imported into the *query* IR (`cpp/src/query/ir.cc:81,269` atom bindings) — one invented schema cap leaked into a second coordinate.

**Collapsing representation:** No SDK cap, or the engine's number as the one array bound (`MAX_EXTENSION_ROWS` for handles; determinant-width for projections, if a bound is needed at all). Pack length / `NI`-style template counts where the size is per-schema. Delete the invented 4/8/16.

**Essential vs accidental:** Accidental. Fixed arrays are a C++ consteval tactic; the number `8` is not a fact of the IR.

**Severity:** medium

---

## 4. `==` is two constructors flattened to `bidirectional: bool`

**Where:** `ts/src/statements.ts:69-74,243-277`; `ts/src/spec.ts:175-190`; `ts/src/lower.ts:85-99`; `cpp/src/schema/contained.cc:28-32`; `cpp/src/schema/mirrors.cc:16`; `cpp/src/schema/spec.cc:163`; `cpp/src/schema/classes.cc:128-132,193`; `cpp/foreign/raii.cc:926-932,1184`

**Wrong:** Hosts already have two constructors: `contained()` / `mirrors()`, C++ `containment_law<S,T,false>` / `<S,T,true>`. The *recorded* data then throws the distinction away:

```
interface ContainmentData { kind: "containment"; bidirectional: boolean }
```

`mirrors()` writes `bidirectional: true` into the same type `contained()` writes `false` into. Render and lower re-learn the constructor from the flag (`operator = data.bidirectional ? "==" : "<="`, `classes.cc:193` `mirrors(` vs `contained(`). A `contained` value with `bidirectional == true` leftover faces is representable. Same polarity, two encodings — sdk-010's interior-atom bool, on statements.

The engine `StatementSpec::Containment { bidirectional: bool }` and the C ABI `uint8_t bidirectional` are the hostile boundary (C1 / essential C). They do not license the *dialect recorded* state to be a bool: C6's recorded-state sum is the host's job, and the constructors already knew.

**Collapsing representation:** Recorded data is `kind: "containment" | "mirrors"` (TS) / `std::variant<contained_law, mirrors_law>` or two statement_form cases (C++). `containment_law<S,T,bool>` as a *call-site* template may stay; the flattened `statement_data.bidirectional` dies. `lower()` / `wire.cc` project to the engine's `bidirectional` byte at the boundary, once.

**Essential vs accidental:** Engine/C ABI bool: essential (boundary, C has no sums; engine spec comment pins `==` as not a variant). Host recorded bool after two constructors: accidental.

**Severity:** medium

---

## 5. `query!` `HeadTerm::Agg` is `over: Option` plus `measure: bool` — leftover `has_over`

**Where:** `crates/bumbledb-query-macros/src/lib.rs:330-338,594-629,1750-1767`

**Wrong:** Engine `FindTerm = Var | Aggregate | Measure | AggregateMeasure`. The macro parses four cases (`Var`, `Measure`, `Count` with no arg, folds with `over`, measure-folds) and then stores three of them as one product:

```
enum HeadTerm {
    Var(Name),
    Measure(Name),
    Agg { op: AggOp, over: Option<Name>, measure: bool },
}
```

`Count` is `Agg { op: Count, over: None, measure: false }`. `Sum(Duration(v))` is `Agg { over: Some(v), measure: true }`. `Count` with `over: Some(_)` and `Sum` with `over: None` are representable. Emission (`:1750-1767`) then re-discovers the engine sum with a three-arm `match over` that special-cases `measure`. The parse never produces the illegal pairs (`parse_agg` returns Count before reading an argument); every later match re-learns what the type threw away. This is sdk-014's ParsedRule shape, on FindTerm, and the leftover `has_over` wave 1 did not reach because it lives in the macros crate's head parser, not `query_value`.

**Collapsing representation:** Four HeadTerm cases matching FindTerm. Count carries no `over`. Folds require it. Measure-folds are `AggMeasure { op, over }`, not a bool beside an Option.

**Essential vs accidental:** Accidental. The parse already knows.

**Severity:** medium

---

## 6. Violation and statement-slot payloads are optionals on every kind

**Where:** `ts/src/db.ts:125-132,440-451,591-599,835`; `ts/src/native.ts:238-244`; `ts/crate/src/marshal.rs:1189-1194,1237-1242`; `cpp/src/error.cc:84-90,264`; `cpp/foreign/raii.cc:74-82,170-178`

**Wrong:** A rendered violation is a sum by form: FD (spelling + facts), containment (plus direction), capacity (plus measure), mirrors-slot (plus orientation). The hosts store a product of optionals:

```
interface Violation {
    kind: StatementKindTag
    direction?: "sourceUnsatisfied" | "targetRequired"
    orientation?: "written" | "mirrored"
    measure?: bigint
}
```

`StatementEntry` (`db.ts:440-451`) is the same product: `key?: { owner, projection }` "exactly for functionality", `reversed?: boolean` "exactly for mirrors", `statement?:` undefined for implied keys. `orientationOf` (`:591-599`) is a flowchart on `boolean | undefined` producing a three-valued sum the type already had. C++ dialect `Violation` always carries `ViolationDirection` (dummy on FD/capacity) plus `optional<Measure>`. raii `violation_copy` copies ABI `has_measure` + two u64 words — that *copy* is sdk-008's outbound optional; the defect here is the *dialect* types staying a product after the copy.

NAPI `ViolationWire` omits absent keys (`if let Some(direction)`), which is a wire spelling, then TS re-inflates them as optionals on one interface.

**Collapsing representation:** One sum per form, each arm carrying only its payload. `StatementEntry` is `ImpliedKey { owner, projection } | DeclaredKey { statement, owner, projection } | Containment { statement } | MirrorsSlot { statement, orientation } | Capacity { statement }`. C++ `Violation` matches on `kind`; direction lives in the containment arm. ABI `has_measure` stays (sdk-008 / essential C); the dialect does not echo it as a field.

**Essential vs accidental:** Flat C `bdb_violation.has_measure`: essential (sdk-008). Host `Violation` / `StatementEntry` as a bag of optionals: accidental. They already branch on `kind`.

**Severity:** medium

---

## 7. `query!` interior-atom style is `Option<bool>`

**Where:** `crates/bumbledb-query-macros/src/lib.rs:1362-1369,1376-1399`

**Wrong:** `interior_style` walks bindings into `Option<bool>`: `None` = empty, `Some(true)` = all bare (dense), `Some(false)` = all numeric labels (sparse). Mixing is a runtime parse error on a pair of flags, the same coordinate as sdk-015's `saw_named` / `saw_index`. Three valid states, encoded as the nullable bool's three inhabitants — which works until a fourth appears, and which forces every later reader to decode `Option<bool>` as a style enum.

**Collapsing representation:** `enum Style { Empty, Bare, Numeric }`. Mixing is unrepresentable.

**Essential vs accidental:** Accidental. Parse-local, not stored on `ParsedRule`; still a bool where a sum belongs.

**Severity:** low

---

## Counts by severity

| Severity | Count | IDs (this dump) | Issue files |
|---|---|---|---|
| high | 1 | 1 | sdk-023 |
| medium | 5 | 2, 3, 4, 5, 6 | sdk-024, 025, 026, 027, 028 |
| low | 1 | 7 | sdk-029 |
| **total** | **7** | | sdk-023–029 |

Not counted as defects: C ABI tagged structs as a *layout* (essential for C; `has_over` death remains sdk-008 — not re-filed); NAPI `query_in` / `find_term_in` optional `over` (sdk-008); raii `violation_copy.has_measure` as an ABI copy (sdk-008); TS `AtomSourceIr` / statement / weight / window sums; C++ `WriteDecision` / `Answers` `Value` variants; raii `owned_statement` variant and `owned_relation.closed` optional (the collapsing spelling #1 asks for); fingerprint lock (one `PIN`, two pipelines — working); `crates/bumbledb-query` re-export; cookbook `Program` relation (docs-023); "rec SCC" / "program" test prose (sdk-022); `cpp/src/db/db.cc:279` unconstrained `prepare<Query>` (sdk-001).

The repeating move: TS schema constructors and raii's `owned_statement` / `optional<owned_closed>` show they know how to spend sums on the schema cutover. The C++ *recorded* schema table, `==` as a bool, Count as `over: None`, and violation optionals were left as flags. That is Program-shaped state, on the layers wave 1 did not read.

### raii / napi cleanliness

- **raii:** not clean. `owned_literal` / `owned_bound` / `owned_weight` / `owned_containment.bidirectional` / dummy `has_width` on `scalar_type` are dump #2 and #4. `violation_copy.has_measure` is sdk-008, not re-filed. The handle RAII, `owned_statement` variant, and `owned_relation.closed` optional are the parts that already collapsed.
- **napi:** query marshal leftover is sdk-008 (not re-filed). Schema marshal parses closedness and statement/weight/window/literal kinds as sums — clean on that lane. Leftover is `ViolationWire`'s parallel `Option` payloads (dump #6).

---

## Final adversarial validation (2026-08-14)

Verified `cpp/src/schema/spec.cc:66-72` (`bool closed` + `closed_info`), `:18-33` / `axioms.cc:13-18` sugar caps, `:163` `bidirectional`, `ts/src/statements.ts:73`, `ts/src/db.ts:125-132,440-451`, `cpp/src/error.cc:84-90`, macros `HeadTerm::Agg { over: Option, measure: bool }` at `lib.rs:330-338`, `interior_style` → `Option<bool>` at `:1369`. schema-001 (engine sealed `Relation` sum) is a different tree from sdk-023 (C++ `relation_data` closed flag) — both real, not merged.

Wave-1 leftovers not re-filed. sdk-030 (`query!` "predicate" diagnostics) lives with the macros crate and is filed under the wave-1 prefix sequence, not here.
