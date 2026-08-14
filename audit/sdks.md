# SDK representation audit — leftover Program, flags, dual builders, ABI optionals

Scope: `crates/bumbledb-query/` (`query!`), `ts/`, `cpp/` (dialect + C ABI / bridge). Engine IR is the coordinate the sugar claims to occupy: `Query { interiors, rec: Option<Rec>, head, rules }` — interiors a DAG, at most one linear rec, anonymous nonempty main. Named head without `interior`/`recursive` is a compile error. No `max_interiors` cap.

Doctrine applied: representation first; illegal states unrepresentable; special cases vanish by changing coordinates. Validation that discards its proof is a defect. A bool where a sum belongs is the defect's shape.

What the cutover did get right (not counted): no `bdb_program`, no `program!`, no `MAX_INTERIORS` / `TooManyCtes` in sugar, `query!` named-head-without-keyword is a spanned compile error (`crates/bumbledb-query/tests/compile-fail/named_head_without_keyword.rs`), TS `AtomSourceIr` is already a sum, C ABI struct is named `bdb_query`. The leftover is not the *word* Program. It is Program-shaped state space: a bag of named rule-lists, a recursive flag, an optional output, and a second constructor that will take whatever you stuff in.

---

## 1. C++ `query_value` is a Program flag-machine

**Where:** `cpp/src/query/query.cc:107-112`, `:199-200`, `:205-209`, `:258`; `cpp/src/db/db.cc:279-281`; `cpp/foreign/query_view.cc:529-537`

**Wrong:** The post-cutover Query is a sum of phases (interiors → optional rec → main). C++ stores it as independent knobs on a public aggregate:

```
std::array<interior_ir, NI> interiors{};
bool has_rec = false;
rec_ir rec{};
query_ir ir{};
```

`has_rec == false` still carries a full `rec_ir`. `has_rec == true` with a default `rec` (empty counts, empty name) is well-typed. `.recursive` returns the *same* `query_value<S, NI>` so a second `.recursive` / `.interior` after rec remain in the overload set; refusal is `if (has_rec) detail::a_second_recursive_is_refused()`. Main-rule count lives in `ir.rule_count`, a runtime counter. `Db::prepare<Query>()` takes any NTTP `query_value` — including `query(S)` never `.rule()`'d, interiors-only, rec-only. That is the old Program: named derived tables, optional rec flag, output slot optional, engine roster as the real parser.

`query_view` then projects the flag into the ABI optional:

```
.rec = Query.has_rec ? &query_rec<Query> : nullptr,
.rule_count = Query.ir.rule_count,   // 0 is representable
```

**Collapsing representation:** `query_value<S, NI, HasRec, NR>` (or a phase type: `Interiors<S,NI>` / `Rec<S,NI>` / `Query<S,NI,NR>` with `NR >= 1`). `.recursive` exists only on `HasRec=false`. `.interior` exists only before rec and before main. `.prepare` is a method of the main-bearing type only. `rec_ir` is a member only when `HasRec`. Empty main is unrepresentable, not `EmptyRuleSet` at the engine.

**Essential vs accidental:** Accidental. `NI` is already a type-level count — they knew the move and declined to spend it on rec/main. NTTP does not require a bool.

**Severity:** high

---

## 2. Dual Query IR in C++ (`query_ir` ⊕ `query_value`)

**Where:** `cpp/src/query/ir.cc:331-345`; `cpp/src/query/query.cc:107-112`; `cpp/src/query/lower.cc:371-387`

**Wrong:** Engine `Query` is one product. C++ splits it: interiors and rec ride `query_value`; main rules, main head, and the param registry ride `query_ir`, whose comment admits the split ("Named interiors and the optional rec ride `query_value`, not this struct"). Every walk (`append_rule`, `derived_tables`, `query_view::for_each_wire_rule`) re-assembles the Query by threading `interiors` + `has_rec` + `rec` + `ir` as four arguments. That is a flowchart compensating for a table that was cut in half.

**Collapsing representation:** One `query_ir<NI, HasRec, NR>` (or the phase machine in #1) holding interiors, optional rec, main. `query_view` reads one value.

**Essential vs accidental:** Accidental. "Variadic pack, not a fixed array" is the excuse for interiors; it does not justify parking rec and main in a different struct keyed by a bool.

**Severity:** high

---

## 3. C++ `wire_atom`: bool instead of `AtomSource`

**Where:** `cpp/src/query/ir.cc:259-270`; `cpp/src/query/lower.cc:161-163`; `cpp/foreign/query_view.cc:292-311`

**Wrong:** Engine `AtomSource = Edb(RelationId) | Interior(InteriorId)`. C++ stores both ids plus a flag:

```
struct wire_atom {
    std::uint32_t relation;
    bool interior;
    std::uint32_t interior_id;
    ...
};
```

Lowering writes `out.interior = true; out.interior_id = id` and leaves `relation` as 0. `query_view` then *re-derives* the C ABI tag from the flag while still stuffing both fields into `bdb_atom`. `interior == true` with a stale `relation`, or `interior == false` with a stale `interior_id`, are representable. The C ABI already has `bdb_atom_source_kind`. The dialect invented a worse encoding and translates at the boundary.

**Collapsing representation:** `enum class atom_source { edb, interior };` is still a flag. Use a variant, or two wire types, or the C ABI tag as the dialect tag — one sum, one payload.

**Essential vs accidental:** Accidental in dialect (`src/`). Essential at the C ABI (C has no sums) — see #8.

**Severity:** high

---

## 4. C++ `find_form` has no `Measure` — duration finds collapse to `Var`

**Where:** `cpp/src/query/ir.cc:157-161`, `:279-288`; `cpp/src/query/rule.cc:229-238`; `cpp/src/query/lower.cc:338-347`; `cpp/foreign/query_view.cc:160-169`

**Wrong:** Engine `FindTerm = Var | Aggregate | Measure | AggregateMeasure`. C++ `find_form = variable | aggregate | aggregate_measure`. Projected variables and projected measures share `find_form::variable`; `find_of` always emits `BDB_FIND_TERM_KIND_VAR`. `find_slot` only constructs from `qvar`, so `(Duration(w))` as a find column is unwritable. `has_over` plus `over.form` is the secret discriminator the switch does not read. AggregateMeasure exists; Measure does not. A legal IR sentence is a special case of Var until the engine computes the wrong column.

**Collapsing representation:** Four `find_form` cases matching `FindTerm`. `find_slot` accepts `measure_ref`. `find_of` maps measure → `BDB_FIND_TERM_KIND_MEASURE`. `has_over` dies; Count is the no-over aggregate case in the sum.

**Essential vs accidental:** Accidental. They already distinguished `aggregate_measure`. Measure is the missing fourth.

**Severity:** high

---

## 5. TS `QueryStart` does not carry phase — second rec / interior-after-rec are runtime

**Where:** `ts/src/query/lower.ts:367-386`, `:1608-1649`; `ts/test/query.test.ts:1161-1193`

**Wrong:** After `.rule()`, `.interior` / `.recursive` are typed `never` (`Query` at `:344-347`). After `.recursive()`, the return type is still `QueryStart` with both methods live. The test is `assert.throws(..., /second recursive/)` — a flowchart on a type that admits the call. Interior-after-recursive is the same throw (`:1614-1617`). Declaration order is data. They encoded it for main and left rec as a `RecData | null` checked by `if`.

**Collapsing representation:** `QueryStart<Rels, Classes, P, Rec extends RecData | null = null>`. `interior` / `recursive` exist only when `Rec extends null`. `.recursive(...)` returns `QueryStart<..., RecData>`. The second call is `never`, like after main.

**Essential vs accidental:** Accidental. They performed the move for the main-rule boundary and stopped.

**Severity:** high

---

## 6. Public `QueryIr` is Program-shaped; `lowerQuery` is a second builder

**Where:** `ts/src/native.ts:78-96`, `:112-116`; `ts/src/index.ts:113`, `:145`; `ts/src/query/lower.ts:2078-2128`; `ts/test/ffi.test.ts:458+`; `ts/test/notation-corpus.test.ts:241-247`

**Wrong:** `QueryIr` is a structural bag:

```
interiors: InteriorIr[]   // each { head, rules } — a named-predicate slot without a name
rec: RecIr | null
head: HeadTermIr[]
rules: RuleIr[]
```

Nothing in the type says: interiors form a DAG, rec is linear with nonempty base *and* rec, main `rules` is nonempty, heads are bound-var only on interiors/rec, Count has no `over`. You can stuff a Datalog program into `interiors` with `rules: []` and `rec: null` — the old output-last Program, minus the name. `FindTermIr` aggregate is `{ kind: "aggregate"; op: AggOpIr; over?: number }` — Count-with-over and Sum-without-over are both legal TypeScript.

`index.ts` exports `QueryIr` and `lowerQuery`. The module doc says the raw native bridge is not exported; the wire type is. Notation corpus `"builder": false` cases are hand-written `QueryIr` because the builder's join-position law cannot spell queries the IR (and `query!`) can. Two constructors, two languages, one engine. That is dual builders.

**Collapsing representation:** `QueryIr` is not a public constructor. Brand it, or parse it into a refined type (`ParsedQuery`) that `dbPrepare` accepts and that only `lowerQuery` / a real parser inhabits. Split aggregate finds: `{kind:"aggregate", op:{kind:"count"}} | {kind:"aggregate", op:FoldOp, over:number}`. Builder walls that refuse legal IR are the wrong coordinate — widen the builder, do not keep a back door.

**Essential vs accidental:** The engine IR being loose (validate at prepare) is an engine ruling. Re-exporting that looseness as a host constructor, *and* keeping a stricter builder that cannot spell the IR, is accidental dualism.

**Severity:** high

---

## 7. `collectRec` mutates an illegal `RecData` through casts

**Where:** `ts/src/query/lower.ts:1572-1595`

**Wrong:**

```
const recData: RecData = { name, finds: freeze([]), base: freeze([]), rec: freeze([]) }
...
;(recData as { finds: readonly FindColumn[] }).finds = first.finds
;(recData as { base: readonly RuleData[] }).base = freeze(base)
;(recData as { rec: readonly RuleData[] }).rec = freeze(rec)
```

The type says nonempty sealed rec. Construction inhabits the empty triple, then lies to the type system to backfill. Arms built against `env.rec = recData` see empty finds during the base walk. Parse-don't-validate inverted: the finished value is the proof, the path to it was an illegal state the type already forbade.

**Collapsing representation:** Build `base`/`rec` arrays first (placeholder env with name + a deferred head), then `const recData: RecData = Object.freeze({ name, finds, base, rec })` in one assignment. No `as`. Empty `RecData` unrepresentable.

**Essential vs accidental:** Accidental. Circular env (rec arms must resolve the rec's head) is essential; mutating readonly through a cast is not. A two-phase *type* (name-only env → sealed RecData) carries the proof.

**Severity:** high

---

## 8. ABI optionals reconstruct illegal Query / Rec / Interior / FindTerm

**Where:** `cpp/foreign/bumbledb_c.h:522-528`, `:546-553`, `:573-578`, `:611-621`; `cpp/bridge/src/query.rs:271-283`, `:333-345`, `:452-475`; `ts/crate/src/marshal.rs:721-738`, `:920-960`

**Wrong:** C cannot have sums. The ABI therefore uses NULL, counts, and `has_*` bytes. Marshal *parses tags* and then inhabits the engine's still-loose `Query`:

- `bdb_query.rec == NULL` → `None`; non-NULL with `base_count == 0` or `rec_count == 0` → `Some(Rec { base: [], rec: [] })`. Empty rec lists are a typed engine value, not a marshal refusal.
- `rule_count == 0` / `interior.rule_count == 0` / `head_count == 0` copy through. Program-shaped empty main is a well-formed `bdb_query`.
- `bdb_atom` always has `relation` and `interior`. Tag selects one; the other is garbage the next consumer can read if the tag is dropped.
- `bdb_find_term.has_over` is `uint8_t`. `find_term_in` does `bool_in(has_over)?.then_some(VarId(over))` — false discards `over`; true accepts any id. Count-with-over is reconstructed from `{kind: Aggregate, has_over: 1}`.
- `bdb_condition` always has `cmp` and `children`. Leaf with leftover children, And with leftover cmp.
- `bdb_violation.has_measure` + two u64 words — same pattern on the way *out*.
- NAPI `obj.get("over")` optional on aggregate finds: missing `over` on Sum and present `over` on Count both parse.

The comment on `query_in` says the engine validator is the trust boundary. That is "validate, then forget": marshal learned the tag and threw away a refined type, so prepare re-checks emptiness, linearity, polarity, Count-nullary, …

**Collapsing representation:** At the C ABI, tagged structs are essential. Immediately inside the bridge, parse into engine enums *and* refuse lists the engine type should not have had to admit — or, if the engine IR stays loose by ruling, the *host* dialect must not be able to mint the empty/flag cases (findings 1, 5, 6). `has_over` dies: Count is a kind, not a bool. NAPI aggregate finds are a sum, not `over?: number`.

**Essential vs accidental:** Flat C structs: essential. Reconstructing illegal engine states from them without a refined parse result: accidental. `has_over` as a parallel optional on a tagged find: accidental even in C (Count can be its own `bdb_find_term_kind`).

**Severity:** high (host-visible reconstruction); the C layout itself is essential and not a defect

---

## 9. Wildcard reified as `query_term_form::absent`

**Where:** `cpp/src/query/ir.cc:38-51`; `cpp/src/query/rule.cc:56-58`, `:225`; `cpp/src/query/lower.cc:59-60`

**Wrong:** Engine law: no wildcard variant; absence from `bindings` *is* the wildcard. C++ match/find products are complete reflected structs, so every field exists, default `term_data{}` with `form == absent`, then `record_match` `continue`s those slots. The illegal state "wildcard bound to something" is unwritable in the engine and writable in the dialect as `absent` plus a leftover `variable`/`literal` payload. `wire_term_of` has an `absent` arm that emits a default wire term if it is ever reached.

**Collapsing representation:** Builder pattern stays a product (ergonomic designated init). The *recorded* IR is a binding list, like the engine. `absent` is not a term form; it is "not pushed onto `bindings`". `term_data` is only constructed for mentioned slots.

**Essential vs accidental:** Complete reflected products are an accidental coordinate of C++ designated init. The engine already chose the right one.

**Severity:** medium

---

## 10. Polarity: EDB is a sum, interior is a bool

**Where:** `cpp/src/query/ir.cc:127-132`; `cpp/src/query/rule.cc:85-90`, `:107-112`, `:273-280`; `ts/src/query/atom.ts:148-157`

**Wrong:** EDB atoms: `body_form::atom | negated_atom` (C++), `kind: "atom" | "negated"` (TS). Interior atoms: one form plus `bool negated` / `negated: boolean`. Same polarity, two encodings. Recursive-rule walls then branch on the flag (`if (item.interior.negated)`). A positive interior with `negated == true` leftover binds is representable in the recorded item.

**Collapsing representation:** `body_form = atom | negated_atom | interior | negated_interior | condition` (C++), or TS `kind: "interior" | "negatedInterior"` with no bool. `with_interior<Name, bool Negated>` can stay a template parameter at the *call*; the *data* is a sum.

**Essential vs accidental:** Accidental. Template `bool Negated` on the call is fine; storing it as a field next to a different polarity encoding is not.

**Severity:** medium

---

## 11. Discriminator-plus-all-payloads in C++ builder IR

**Where:** `cpp/src/query/ir.cc:26-35` (`query_literal`), `:59-66` (`term_data`), `:146-151` (`body_item`), `:177-187` (`find_data.has_over` / `classed`), `:204-212` (`param_data`)

**Wrong:** Every "sum" is a struct with a tag and every alternative's fields. `query_literal` holds bool, u64, i64, and two intervals at once. `body_item` holds `atom`, `interior`, and `condition` at once. `find_data` uses `has_over` and `classed` as existence flags. `param_data` uses `point` / `membership` bools. Eight states from three bools; a few valid. This is the Minsky example, in the query IR.

NTTP/consteval wants trivial types. That is a constraint, not a representation. `std::variant` is the dialect's blessed closed sum (`cpp/AGENTS.md` §8) and is unused here.

**Collapsing representation:** `std::variant` for body items, terms, literals, finds (four FindTerm cases, Count without `has_over`). If variant is too heavy for NTTP, *separate arrays* per alternative (the engine's `atoms` / `negated` / `conditions` buckets) — they already bucket at lower time, so recording a mixed `body_item` is a detour.

**Essential vs accidental:** Accidental relative to dialect law. Trivial NTTP layout is the local cost; the global cost is every downstream `if (form)`.

**Severity:** medium

---

## 12. Sugar caps: `max_query_rules = 4` (the `max_interiors` that survived)

**Where:** `cpp/src/query/ir.cc:10-16`; `cpp/src/query/query.cc:134`, `:203-204`; `cpp/src/query/ir.cc:317-328` (`rec_ir` base *and* rec each `max_query_rules`)

**Wrong:** Engine: `MAX_RULES = 16` per list, rec pooled, **no interior-count cap**. C++ sugar invents `max_query_rules = 4`, `max_query_atoms = 8`, `max_query_finds = 8`, `max_query_params = 8`, `max_query_vars = 32`. Comment: "SDK bounds only — the engine's own caps are far higher." That is a second theory of size. `rec_ir` has *two* arrays of length 4, so the struct admits 8 rec-pool rules while `static_assert(base+rec <= 4)` is a flowchart on top. Interior *count* is uncapped (`NI + 1`) — they removed `max_interiors` and left its cousins.

**Collapsing representation:** No SDK cap, or the engine's `MAX_RULES` as the one number, encoded as the array bound. `rec_ir` is one pool array plus `base_count`, not two independent arrays.

**Essential vs accidental:** Accidental. Fixed arrays are a C++ consteval tactic; the number `4` is not a fact of the IR.

**Severity:** medium

---

## 13. C++ conditions are leaves; ABI trees are a back door

**Where:** `cpp/src/query/ir.cc:272-277`; `cpp/foreign/query_view.cc:142-157`; `cpp/foreign/bumbledb_c.h:232-237`, `:573-578`

**Wrong:** Engine `ConditionTree = Leaf | And | Or`. C++ `wire_condition` is one comparison. `condition_of` hardcodes `BDB_CONDITION_KIND_LEAF`. `and`/`or` trees are unwritable in the dialect and writable as raw `bdb_condition` graphs. Dual constructors again: sugar is a strict subset, ABI is the Program-era escape.

**Collapsing representation:** `wire_condition` *is* a tree (or `std::variant<leaf, and, or>`). Sugar `and`/`or` lower into it. No second path.

**Essential vs accidental:** Accidental. TS already has the trees. C++ flattened because the builder only has `.where(leaf)`.

**Severity:** medium

---

## 14. `query!` `ParsedRule` is a flag plus optional name

**Where:** `crates/bumbledb-query-macros/src/lib.rs:341-357`, `:1883`, `:1933`

**Wrong:**

```
enum RuleKind { Bare, Interior, Recursive }
struct ParsedRule {
    kind: RuleKind,
    name: Option<Name>,  // None for bare; Some for the others
    ...
}
```

`Bare + Some(name)` and `Interior + None` are representable. Emission does `rule.name.clone().expect("interior rules carry a name")` — a panic on an illegal state the type admitted. The parse never produces those pairs; every later match re-learns what the type threw away.

**Collapsing representation:**

```
enum ParsedRule {
    Bare { head, items },
    Interior { name: Name, head, items },
    Recursive { name: Name, head, items },
}
```

**Essential vs accidental:** Accidental. The parse already knows.

**Severity:** medium

---

## 15. `query!` param style is two bools

**Where:** `crates/bumbledb-query-macros/src/lib.rs:1265-1270`, `:1276-1304`

**Wrong:** `saw_named` and `saw_index` — four states, two valid after first use, one valid at start. Mixing is a runtime parse error on a pair of flags.

**Collapsing representation:** `enum Style { Empty, Named(Vec<String>), Index }` (or `Named` vs `Index` after first resolve). Mixing is unrepresentable.

**Essential vs accidental:** Accidental.

**Severity:** medium

---

## 16. TS `isQueryValue` validates then forgets

**Where:** `ts/src/query/lower.ts:1516-1520`, `:1524-1534`

**Wrong:** "Trusted admission seam" checks `value.schema === theory` and then type-predicates the value into `Query<Rels, Row, P, Classes>`. The check does not learn interiors/rec/main well-formedness, head alignment, or param anchors. Those were thrown at construction (`makeRawQuery`) and are not in the type. A `RawQuery` from anywhere that happens to share the schema object is a typed `Query`.

**Collapsing representation:** Do not type-predicate. `makeQuery` returns `Query` because it *constructed* one; the constructor's result type *is* the proof. If a seam is required, it parses `RawQuery` into a branded `Query` whose brand is unforgeable.

**Essential vs accidental:** Accidental. Schema identity is a real check; using it as a substitute for the Query invariant is King's validator.

**Severity:** medium

---

## 17. TS `CmpData.mask` is an optional on every leaf

**Where:** `ts/src/query/atom.ts:90-96`

**Wrong:** `mask: MaskData | undefined` with a comment "present exactly for `allen`". Eq-with-mask and Allen-without-mask are representable. Engine `CmpOp::Allen { mask }` carries the mask in the op, not beside it.

**Collapsing representation:** `op: { kind: "allen", mask: number } | { kind: Exclude<CmpKind, "allen"> }` (TS already does this on `CmpOpIr` in `native.ts:150-158`). Runtime `CmpData` should match.

**Essential vs accidental:** Accidental. The wire type got the sum; the builder data did not.

**Severity:** medium

---

## 18. Compile-fail holes

The Rust `query!` suite actually pins the cutover (`named_head_without_keyword`, phase order, one rec, nonempty base/rec, bare main required). C++ and TS do not.

**C++ missing fixtures (paths exist as consteval traps, not types):**

- Recursive after main — `cpp/src/query/query.cc:208` `interior_or_recursive_after_a_main_rule`; there is `query_interior_after_main.cc`, no recursive twin.
- Interiors-only / rec-only `prepare<>` — empty main is a typed `query_value` (`#1`).
- `FindTerm::Measure` as a column — unwritable (`#4`), so neither a compile-fail nor a compile-success exists.
- Condition trees — unwritable (`#13`).
- Negation in rec — runtime/consteval wall, no compile_fail fixture.

**TS: runtime throws where the type should have been `never`:**

- Second recursive, interior-after-recursive, duplicate interior name, empty name (`ts/src/query/lower.ts:1614-1642`; `ts/test/query.test.ts:1161-1215`). After-main *is* `never`. Inconsistent coordinates.

**Collapsing representation:** Phase types (`#1`, `#5`). Then the compile-fail suite is the type, not a list of fixtures chasing traps.

**Essential vs accidental:** Accidental. Rust proved the errors are static.

**Severity:** medium

---

## 19. C++ `derived_tables` repeats the rec flag

**Where:** `cpp/src/query/lower.cc:113-118`, `:127-129`

**Wrong:** `bool has_rec` + `rec_ir const& rec`. Name lookup is `if (has_rec && rec.name == name) return NI`. Default `rec` with `has_rec == true` answers as the rec for whatever empty name is.

**Collapsing representation:** `std::optional<rec_ir>` is not NTTP-friendly; `if constexpr (HasRec)` on the query_value template parameter is. No bool.

**Essential vs accidental:** Accidental (same defect as #1, second site).

**Severity:** medium (counted separate because it is the lowering coordinate, not just the builder value)

---

## 20. `query_ir.head` dummy `op` on variable finds

**Where:** `cpp/src/query/rule.cc:229-232`; `cpp/foreign/query_view.cc:184-193`

**Wrong:** Variable finds store `op = fold_form::sum` as filler. `head_term_of` then writes `BDB_HEAD_OP_SUM` on Var heads "because the field exists". The ABI ignores `op` for Var; a consumer that forgets the tag reads Sum. Same as `has_over` / unused payloads.

**Collapsing representation:** Finding #4's four-case find; Var carries no op.

**Essential vs accidental:** Accidental.

**Severity:** low

---

## 21. `query_view` empty-interiors dummy array of length 1

**Where:** `cpp/foreign/query_view.cc:443-444`, `:530`

**Wrong:** `std::array<bdb_interior, Query.interiors.size() == 0 ? 1 : Query.interiors.size()>` then `.interiors = size==0 ? nullptr : data()`. Empty is a special case of "one dummy slot we promise not to point at". Dijkstra: the empty range is a coordinate error.

**Collapsing representation:** `std::array<T, N>` with `N=0` is legal; `.data()` / nullptr for count 0. No `?: 1`.

**Essential vs accidental:** Accidental. `array<T,0>` exists.

**Severity:** low

---

## Counts by severity

| Severity | Count | IDs |
|---|---|---|
| high | 8 | 1, 2, 3, 4, 5, 6, 7, 8 |
| medium | 11 | 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19 |
| low | 2 | 20, 21 |
| **total** | **21** | |

Not counted as defects: C ABI tagged structs as a *layout* (essential for C); absence of `bdb_program` / `program!` / `max_interiors`; `query!` named-head-without-keyword (the one compile-fail that actually encodes the cutover); TS `AtomSourceIr` as a sum.

The repeating move: NI / `never`-after-main / `AtomSourceIr` show they know how to spend types on the Query cutover. Rec, main-emptiness, Measure, polarity, Count-nullary, and the public wire type were left as bools, optionals, and a second constructor. That is Program, still inhabitable, with the word scraped off.

---

## Final adversarial validation (2026-08-14)

Verified against `cpp/src/query/{query,ir,rule,lower}.cc`, `cpp/foreign/{bumbledb_c.h,query_view.cc}`, `cpp/bridge/src/query.rs`, `ts/src/{native,query/lower,query/atom}.ts`, `ts/crate/src/marshal.rs`, `crates/bumbledb-query-macros/src/lib.rs`. No product-code edits.

- sdk-001–018, 021 KEEP (large refactors stay). sdk-019 DUPLICATE(sdk-001), sdk-020 DUPLICATE(sdk-004).
- REWRITE: sdk-004 (ABI `has_over` is sdk-008's C6 delta; dialect dies here). sdk-005 (do not pin SCC substring — sdk-022). sdk-007 (`RecData` is not actually a nonempty type). sdk-008 (**do not marshal-refuse empty rec/main** — C1 engine roster; `has_over` discriminator dies per C6). sdk-018 (`query_second_recursive.cc` already exists; do not add `query_recursive_twice.cc`). sdk-022 (ABI `bdb_rec` "SCC" comments + `a_recursive_rule_negates_no_stratum` trap name).
- NEW: sdk-030 — `query!` diagnostics still say "predicate" (`lib.rs:1079+`).
- C ABI `BDB_FIND_TERM_KIND_MEASURE` already exists (`bumbledb_c.h:211`); C++ `find_form` still has no `measure` (sdk-004). TS `FindTermIr` already has `measure` / `aggregateMeasure`; builder `AggData` is already a Count-vs-fold sum; the wire `over?: number` remains (sdk-006).
- CONTRACT: C1 freezes `bdb_query` nullable `rec`. C6 blesses `has_over` death on `bdb_find_term` only. Count has no `over`; folds require it (sdk-004/008/027 aligned). No new caps. schema-001 ≠ sdk-023.
