## Compile-fail roster covers 9 of the query! macro's spanned refusals, leaving eight semantic-law diagnostics unpinned

category: missing-free-feature | severity: low | verdict: CONFIRMED | finder: query:crates
outcome: fixed a270d093

### Summary

The hand-rolled compile-fail suite at `crates/bumbledb-query/tests/compile_fail.rs` asserts exactly nine fixtures (`assert_eq!(seen, 9, "the compile-fail roster has nine fixtures")`, line 165) and its runner machinery makes each new case one small fixture file plus a count bump. But the `query!` macro (`crates/bumbledb-query-macros/src/lib.rs`) carries roughly fifty `fail()` sites, and at least eight of them are deliberate, message-bearing *semantic* refusals — not mere token-shape errors — with zero fixtures, zero unit tests (the macro crate has no `#[test]` at all), and no coverage anywhere else in the repo. Two of the uncovered refusals guard rules the macro's own module doc elevates to laws.

### Evidence

All file:line citations verified directly.

The nine fixtures and their pinned diagnostics (`crates/bumbledb-query/tests/compile-fail/*.rs`, every `//@ error:` directive read): `typo_relation` (BUZY), `typo_field` (BUSY_PERSN), `ambiguous_punning`, `param_in_head`, `datalog`, `program_without_bare_output`, `explicit_dense_positions`, `mixed_predicate_bindings`, `uppercase_predicate_name`.

Uncovered refusals in `crates/bumbledb-query-macros/src/lib.rs`:

- **:1070 and :1090** — named/positional `?param` mixing, both directions: `"query!: named and positional ?params cannot mix — pick one spelling per query"`. The module doc states the law at :99 ("**Params** are one style per query"). Note the :1090 direction spans at `Span::call_site()`, so a fixture would also pin (or expose) that span choice.
- **:1361** — bare handle at a predicate position: `"query!: a bare handle resolves through the field-named host enum, and a predicate position has no field name — qualify it"`. The handle-resolution rule is a documented law at :92-98.
- **:567** — the measure fold under a non-Sum/Min/Max op: `"query!: the measure folds under Sum/Min/Max only (docs/architecture/20-query-ir.md § the measure)"`. `AggOp::Pack/Count/CountDistinct/ArgMax/ArgMin` (:268-277) all reach this arm; the message even cites the architecture doc it enforces.
- **:1139** — unbound head variable: `` "query!: head variable `{}` is not bound in the rule body" ``.
- **:415** — `` "query!: a negative literal cannot carry `u64`" ``.
- **:421** — `` "query!: unsupported integer suffix on `{text}` — the value types are u64 and i64" `` (the literal-spelling typing rule is a documented law at :88-91).
- **:721** — a binding's `in` without a `?param`: `` "query!: a binding's `in` takes a ?param bound to a set — interval membership is the `==` typing rule or a body item" ``.
- **:1421** — numeric labels on a relation's named fields: `` "query!: `{}` — numeric labels address a predicate atom's head positions; a relation's fields are named" ``.

Absence of other coverage: a repo-wide grep for fragments of all eight diagnostic strings hits only the macro source itself plus incidental doc-comment phrases in unrelated files (`ir/validate/validate.rs:45`, `api/prepared/bind.rs:760`, `cookbook.rs:16` — all matching "cannot carry" in prose about other things). `crates/bumbledb-query-macros/src/lib.rs` contains zero `#[test]` functions.

The "coverage is free" claim: the runner (`check_fixture`, compile_fail.rs:91-137) discovers fixtures by directory scan and needs no per-case code — a new case is one fixture file with `//@ error:`/`//@ line:` directives plus incrementing the count at :165.

### Failure scenario

A refactor of the macro's parse path that weakens or drops one of these refusals passes the entire test suite. Concretely: accidentally accepting `Pack(Duration(x))` (the :567 guard), whose lowering the IR has no shape for, or silently coercing a `-5u64` literal (:415), would go unnoticed — the diagnostics' text and spans are unpinned, so even benign message/span drift on documented laws is invisible to CI.

### Suggested fix

Add one fixture per uncovered refusal (eight files) to `crates/bumbledb-query/tests/compile-fail/`, each pinning the diagnostic substring and — where the span is meaningful, as in the param-mixing and unbound-head cases — the `//@ line:` span; bump the roster assertion to 17. The param-mixing fixture for the positional-after-named direction will additionally surface that its refusal spans at `Span::call_site()` (:1090) rather than at the offending token, which is worth a deliberate ruling while writing the fixture.
