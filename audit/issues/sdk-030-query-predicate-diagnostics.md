# sdk-030: `query!` user-facing diagnostics still call derived tables "predicates"

- **Severity:** medium
- **Tree:** sdk (rust macros)
- **Status:** OPEN
- **Source:** adversarial pass (final validation; not in audit/sdks.md / sdk-rest.md)
- **Depends on:** none (string-only; same `lib.rs` as sdk-014/015/027/029 — land with them to avoid merge noise). engine-041 owns the engine type `Predicate` → `Signature`; this issue does not rename that type.
- **Conflicts with:** sdk-014, sdk-015, sdk-027, sdk-029 (same file)

## The bug

`crates/bumbledb-query-macros/src/lib.rs` still teaches the deleted Program coordinate in spanned diagnostics — interiors and the rec are "predicates":

- `:1079`, `:1139` — "query!: predicate names begin lowercase … a predicate spelled like a relation"
- `:1113` — "a predicate cannot take either tree name"
- `:1385`, `:1394`, `:1408`, `:1553`, `:1576`, `:1657` — "bare predicate binding" / "a predicate atom's bindings address head positions" / "predicate position has no field name"
- `:1625` — "query!: unknown predicate `{}` — lowercase names are predicates"
- `:1978` — "recursive predicates are unwritable"
- Comments: `:737-740` "the predicate table exists"; `:1870` "names the rec predicate"

The parse already distinguishes `interior` / `recursive` / bare main (sdk-014). The diagnostics re-introduce the Datalog-predicate table the cut deleted.

NOT this issue: C++ `where()` "predicate value" (`cpp/src/query/rule.cc:197-201`) — that is a comparison/boolean predicate, English logic. sdk-022's stratum/SCC/program strings. engine-041's `ir/validate::Predicate` type (docs-002/017 depend on that rename).

## Why it's wrong

Insight 1: a compile error is the SDK's teaching surface. Every `query!` user who misspells a derived name learns that interiors are predicates. C7: Query, interiors, one linear rec, main signature — no predicate-as-query-head.

## The fix

Per `audit/CONTRACT.md §C7`: derived-table / `interior` / `recursive` in every user-facing string. Suggested shape (keep spans, keep the punning-law substance):

- "derived-table names begin lowercase (`{}`) — UpperCamel names are relations"
- "unknown derived table `{}` — lowercase names are interiors or the rec, resolved macro-locally"
- "an interior/rec atom's bindings address head positions"
- "a second recursive is unwritable"

Comments that say "predicate table" die with the same pass.

## Acceptance criteria

- [ ] Gone: `rg -inw 'predicate|predicates' crates/bumbledb-query-macros/src/lib.rs` → no Datalog-predicate hits (comparison-predicate English, if any, listed in the commit). Compile-fail fixtures that pin these strings update in the same change (`mixed_predicate_bindings.rs` filename may stay — that is a test path, not a user diagnostic — or rename if the fixer is already there).
- [ ] Unchanged tests: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-query` green; phase-order / named-head-without-keyword diagnostics keep their meaning; only the noun changes.
- [ ] Green: `cargo test -p bumbledb-query`.

## Constraints

- Spanned diagnostic *meaning* identical. Do not rename engine `Predicate` (engine-041). Do not add Program vocabulary. Coordinate with sdk-014/015/027/029 if they touch `lib.rs` in one wave.
