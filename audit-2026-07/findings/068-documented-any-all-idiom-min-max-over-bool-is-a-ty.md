## Documented Any/All idiom (Min/Max over bool) is a typed rejection in the engine — and 10-data-model.md contradicts itself about it

category: incoherence | severity: medium | verdict: CONFIRMED | finder: cross:free-features
outcome: fixed 592a5ffc (R3)

### Summary

Two normative surfaces record the boolean quantifier idiom as the shipped replacement for SQL's `bool_or`/`bool_and`: the data-model chapter's "idioms, recorded once" list ("**Any/All** — `Max`/`Min` over a bool column: the 0/1 encoding makes the two quantifiers the two extremes; no dedicated operators") and the README's type-table row for `bool` ("Any/All are `Max`/`Min`"). The implementation refuses this spelling on every surface: the Rust validator admits `Sum|Min|Max` only over `ValueType::U64 | I64`, the IR doc-contract says "U64 and I64 only", and the TS type gate excludes bool from every fold position. Verification found the incoherence is deeper than doc-vs-code: **10-data-model.md contradicts itself** — its own orderability ruling (§ Orderability, complete) declares "Bool ordering is noise" and makes everything outside U64/I64 equality-only, directly refusing what its idioms list prescribes 500 lines later. The query-IR chapter sides with the code and cites 10-data-model.md as its authority for the refusal, so the spec chain is split down the middle.

### Evidence (all verified against the working tree)

The idiom, recorded as normative:
- `docs/architecture/10-data-model.md:623-624` — "**Any/All** — `Max`/`Min` over a bool column: the 0/1 encoding makes the two quantifiers the two extremes; no dedicated operators."
- `README.md:406` — bool row: "`==` `!=`; Any/All are `Max`/`Min`".

The refusal, everywhere else:
- `crates/bumbledb/src/ir/validate/finds.rs:162-177` — the `(AggOp::Sum | AggOp::Min | AggOp::Max, Some(var))` arm returns `ValidationError::AggregateInputType` unless `resolved_var_type(*var)` matches `ValueType::U64 | ValueType::I64`. `ValueType::Bool` is a distinct variant (`crates/bumbledb-theory/src/schema.rs:82`), so a bool column cannot pun as u64.
- `crates/bumbledb/src/ir.rs:196-198` — `Min`: "U64 and I64 only (the orderable types — intervals excluded)"; `Max`: "U64 and I64 only, as [`AggOp::Min`]".
- `ts/src/query/atom.ts:486-490` — `OrderVarOk` is true only when `field.kind extends "u64" | "i64"`; `ts/src/query/find.ts:120-147` routes every `sum|min|max` entry and every Arg key through it (`FoldOverOk`), so `r.min(flagVar)` over a bool field is a compile-time type error. No `any`/`all` helper exists anywhere in `ts/src/query`.

The internal contradiction and the spec chain:
- `docs/architecture/10-data-model.md:103-112` (§ Orderability, complete) — "U64 and I64 support ordering (`Lt/Le/Gt/Ge`, `Min`, `Max`, range conditions). ... Everything else is equality-only: ... **Bool ordering is noise.**" The same document refuses at line 112 what it prescribes at line 623.
- `docs/architecture/20-query-ir.md:303-305` (§ aggregation) — "`Min`/`Max` accept U64 and I64 only (the orderable types — `10-data-model.md`)" — the query-IR spec agrees with the code and cites the self-contradicting chapter as its authority.
- Git history: the "Bool ordering is noise" ruling entered 2026-07-02 ("Fold all audit rulings into the architecture"); the idioms passage was last touched 2026-07-10 — neither side was reconciled against the other.

### Failure scenario

A host follows the recorded idiom — `query!(T { (grp, all_ok: Min(flag)) | Row(grp, flag); })` in Rust, or a `min(flagVar)` fold entry in TS — and gets `AggregateInputType` at prepare (Rust) or a type error at the find record (TS), for the exact query the data-model chapter and README tell it to write. There is no engine spelling of Any/All at all; the only recourse is to scan the group and fold host-side, which the docs nowhere say.

### Suggested fix

Pick one representation and make all five surfaces say it. The internal evidence leans toward the refusal being the intended doctrine (the orderability ruling is the audit-folded text, and 20-query-ir.md and both implementations follow it), in which case: delete the Any/All entry from 10-data-model.md's idioms list, fix README.md:406 to "`==` `!=` only", and record the refusal with its replacement (host-side: `Count` over the group vs. `Count` filtered by `flag == true`, compared by the host). Alternatively, if the idiom is the intent, the machinery is already paid for — bool cells are strictly 0/1 by encoding law and the fold sink is generic over ordered words — so admit `ValueType::Bool` into the Min/Max arm at finds.rs:173 (Sum stays refused), update ir.rs:196-198 and 20-query-ir.md:303, widen the TS gate for min/max only (a separate judgment from `OrderVarOk`, which also feeds `Lt`-family comparisons and Arg keys that should keep refusing bool), and rewrite 10-data-model.md:112 so "Bool ordering is noise" carves out the two quantifier extremes. Either way, the two passages of 10-data-model.md must stop disagreeing with each other.
