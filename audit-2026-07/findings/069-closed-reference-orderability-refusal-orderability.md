## Closed-reference order refusal exists only in the TS SDK — engine validation orders vocabularies silently, against the doctrine's "refused" ruling

category: incoherence | severity: medium | verdict: CONFIRMED | finder: cross:free-features

### Summary

`docs/architecture/10-data-model.md` § "Orderability, complete" rules that a closed reference's declaration-id order "is refused exactly as the enum's ordinal order was", and README's closed-relation row records "**order refused**". The engine implements this refusal roster for every other equality-only type as dedicated typed validation errors — but has zero closed-awareness in query validation. A closed-referencing field is a plain `u64` column, so its var resolves `ValueType::U64` and every order position (`Lt/Le/Gt/Ge`), fold position (`Sum`/`Min`/`Max`), and Arg key (`ArgMax`/`ArgMin`) validates and executes over vocabulary handles, ranking by the declaration-order accident the doctrine forbids. Only the TypeScript SDK enforces the ban (type tier + construction-tier twin); Rust `query!` hosts and raw-IR hosts get no wall at all. `70-api.md`'s claim that "the engine cannot backstop this wall" is false for the refusal itself — the engine already builds the exact closed-reference position table the refusal needs, for the renderer.

### Evidence (all verified against the working tree)

Doctrine — the refusal is stated as a system property, in the same paragraph as refusals the engine does implement:
- `docs/architecture/10-data-model.md:103-112` — "**Orderability, complete:** ... a closed reference's declaration-id order is a declaration-order accident, not semantics (order on it is refused exactly as the enum's ordinal order was)"; the same paragraph makes bytes<N> order "typed validation errors".
- `README.md:410` — closed-relation row: "`==` `!=`, ∈-sets; **order refused**".

The engine's refusal roster covers every equality-only type except closed references:
- `crates/bumbledb/src/ir/validate/context.rs:301-313` — `screen_order_operand` refuses Interval/FixedBytes/String/Bool with dedicated errors (`OrderComparisonOnInterval/OnFixedBytes/OnString/OnBool`) and passes everything else — including U64 — through. ValueType-only; no field or containment resolution.
- `crates/bumbledb/src/ir/validate/finds.rs:172-177` — `Sum|Min|Max` over a var checks only `ValueType::U64 | ValueType::I64`.
- `crates/bumbledb/src/ir/validate/finds.rs:229-233` — `ArgMax`/`ArgMin` key: same ValueType-only check (`NonOrderableArgKey` otherwise).
- `grep -rn closed crates/bumbledb/src/ir/validate/` — zero hits (source and tests both); no validation test covers closed order.
- A closed-referencing var really is U64 on this path: `crates/bumbledb-theory/src/schema.rs` `sealed_fields()` mints the synthetic id as `ValueType::U64`, and a referencing field is a declared `u64` column plus a containment (`10-data-model.md` § closed relations).

The wall exists only in TypeScript:
- `ts/src/query/atom.ts:479-490` — `OrderVarOk` excludes fields carrying a `ClosedRoster` even though their kind is `u64`, citing the same doctrine section; it gates every order-comparison and fold position at the type tier.
- `ts/src/query/lower.ts:828-865` — `closedOrderError` + `assertNotClosed`, "the orderable ban's runtime twin", applied to the order roster and point membership.

No wall on the Rust surface:
- `crates/bumbledb-query-macros/src/lib.rs:94,210` — the macro knows closed handles only as literal spellings (`Kind::Focus`); grep for "closed" across `bumbledb-query` and `bumbledb-query-macros` finds no refusal logic. Lowered IR goes straight to the validation shown above.

The "cannot backstop" sentence is wrong for the refusal:
- `docs/architecture/70-api.md:1000-1001` — "The engine cannot backstop this wall: the wire carries plain u64s, no rosters." True for the name↔id marshal (names never cross the wire), false for the refusal: at prepare the engine holds the full `SchemaDescriptor`, including which relations are closed (`extension: Some(rows)` IS the closed marker, `bumbledb-theory/src/schema.rs:29`) and which statements are containments into closed ids. The engine already computes exactly this: `crates/bumbledb/src/ir/render.rs:60-90` builds a `(relation, field) → closed relation` table (field whose declared containment targets a closed relation's id and projects that field, plus each closed relation's own id field) — the renderer and `plan/ground/evaluate.rs::containment_into_id` both own this resolution today. The refusal is computable centrally from representation the engine already owns.

### Failure scenario

A Rust host writes `query!(Tickets { (t, worst: Max(p)) | Ticket(id: t, priority: p); })` where `priority` is a closed reference into `Priority = { Low, Med, High }`. It prepares and executes without diagnostic, ranking by Priority's declaration order. Reordering the vocabulary in the schema — a new theory, entirely legitimate per the migration policy — silently changes query ANSWERS. This is precisely the enum-ordinal bug the doctrine says the closed-relation design refused; the TS SDK catches it, the Rust surface and any raw-IR host do not, so the same query IR is refused or admitted depending on which host spelled it — a one-semantics-per-wire violation.

### Suggested fix

Enforce the ban where the representation lives, in engine query validation: build (or share) the closed-reference position table `render.rs` already constructs, resolve each var's binding fields against it, and refuse `Lt`-family comparisons, `Sum`/`Min`/`Max` folds, and Arg keys over closed-bound vars with a dedicated typed error (`OrderComparisonOnClosedReference` / an aggregate twin) — the same one-judgment-for-all-hosts shape `OrderComparisonOnFixedBytes` et al. already have. Keep the TS wall as the ergonomic (type-tier) layer. Then correct the `70-api.md:1000` sentence: the engine cannot backstop the *name marshal*, but it can and should own the *order refusal*. Add validate tests (currently zero "closed" coverage in `ir/validate/tests`).
