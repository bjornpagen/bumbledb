# bench-004: param anchors, coverage, contradiction, and `EdbAtom` walk `query.rules` over EDB atoms only

- **Severity:** medium
- **Tree:** bench
- **Status:** OPEN
- **Source:** audit/bench.md F4
- **Depends on:** none (land **before** engine-020's randomized entry; today these paths never see Interior atoms and the panic is latent)

## The bug

Four CQ-shaped consumers of a `Query`:

1. `querygen/oracle.rs:64-79,102-134` — `params_for` discovers params by walking `query.rules` only. `place` / `atom.relation()` assume EDB (`:107-117`).
2. `querygen/coverage.rs:532` — coverage tallies walk `query.rules` only.
3. `querygen/contradict.rs:22` — plants contradictions on `query.rules` only. Planting only main of a rec query does **not** make the query denote ∅ (base/step arms still fire).
4. `edb.rs:9-20` — `EdbAtom::relation()` panics on `Interior`: "harness atoms are stored-relation by construction."

`conformance.rs` uses the same `atom.relation()` (`:659,715,729,749,1025`) and `lower_rule` rewrites every atom as `AtomSource::Edb(atom.relation())` (`:729`) — an Interior atom would be corrupted, not just panic.

A rec query whose only param lives on a base arm is invisible to `params_for`. Hidden today only because the mixed generator does not feed them such a Query.

## Why it's wrong

King: the walk validated "main rules, EDB sources" and threw the rest of the type away, so every caller downstream must not mention interiors/rec — or panics (Insight 6). `EdbAtom` on `Atom` is a dual atom type: the IR has `AtomSource::Interior`; the harness pretends it does not (Insight 2).

## The fix

- One walk over interiors, then rec (base + step), then main, for param anchors / coverage / contradiction.
- Atom source is a match (`Edb` → schema field; `Interior` → derived column), not a panicking `relation()`. `conformance::lower_rule` must preserve `AtomSource::Interior`, not coerce to Edb.
- `EdbAtom` may remain on the CQ `Builder` (it only constructs EDB atoms). It dies as a trait implemented on `Atom`.
- Seeded serializer atom emission rides bench-001 (match `AtomSource`; seeded spelling still writes `"relation"` for EDB).
- **CQ draws stay identical:** for `interiors == [] && rec == None`, `params_for` anchors and RNG consumption must match today (walk skips empty prefixes). Do not reorder main-rule placement.
- **Contradiction:** plant every rule-list (interiors, rec.base, rec.rec, main) or refuse to return a derived query until every arm is poisoned. Main-only planting on a rec query is a semantic change (the lfp would still be nonempty).

## Acceptance criteria

- [ ] Gone: `rg -n 'impl EdbAtom for Atom' crates/bumbledb-bench/src` → no match; `atom.relation()` on a value that might be Interior is gone.
- [ ] `params_for` on `querygen::random_reach_query`'s output binds every param that appears in interiors/rec/main — pin with a unit test on a rec query whose param is only on the base arm (the closure-from-param shape). CQ-only queries keep today's anchors.
- [ ] Coverage/contradict do not panic on an interiors-or-rec query; contradict on a rec query poisons base+step+main (or skips derived until it can).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-bench`; `./scripts/check.sh`.

## Constraints

- Builder-internal EDB-only construction may stay. Boundary `Query` unchanged. No corpus regeneration. Land before the mixed `random_query` entry so stamp/fuzz cannot panic.
