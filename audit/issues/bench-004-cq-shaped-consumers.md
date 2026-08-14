# bench-004: param anchors, coverage, contradiction, and `EdbAtom` walk `query.rules` over EDB atoms only

- **Severity:** medium
- **Tree:** bench
- **Status:** OPEN
- **Source:** audit/bench.md F4
- **Depends on:** engine-020 / bench-003 (today these paths never see Interior atoms; the panic is latent)

## The bug

Four CQ-shaped consumers of a `Query`:

1. `querygen/oracle.rs:64-79,102-134` — `params_for` discovers params by walking `query.rules` only.
2. `querygen/coverage.rs:532` — coverage tallies walk `query.rules` only.
3. `querygen/contradict.rs:22` — plants contradictions on `query.rules` only.
4. `edb.rs:9-20` — `EdbAtom::relation()` panics on `Interior`: "harness atoms are stored-relation by construction."

`conformance.rs` seeded serializer uses the same `atom.relation()` (`:1025`) — it cannot emit an interior atom.

A rec query whose only param lives on a base arm is invisible to `params_for`; an Interior atom panics coverage / contradict / `render_case`. Hidden today only because bench-003 never feeds them such a Query.

## Why it's wrong

King: the walk validated "main rules, EDB sources" and threw the rest of the type away, so every caller downstream must not mention interiors/rec — or panics (Insight 6). `EdbAtom` on `Atom` is a dual atom type: the IR has `AtomSource::Interior`; the harness pretends it does not (Insight 2).

## The fix

- One walk over interiors, then rec (base + step), then main, for param anchors / coverage / contradiction.
- Atom source is a match (`Edb` → schema field; `Interior` → derived column), not a panicking `relation()`.
- `EdbAtom` may remain on the CQ `Builder` (it only constructs EDB atoms). It dies as a trait implemented on `Atom`.
- Seeded serializer atom emission rides bench-001 (match `AtomSource`; seeded spelling still writes `"relation"` for EDB).

## Acceptance criteria

- [ ] Gone: `rg -n 'impl EdbAtom for Atom' crates/bumbledb-bench/src` → no match; `atom.relation()` on a value that might be Interior is gone.
- [ ] `params_for` on `querygen::random_reach_query`'s output (or the one entry's derived class) binds every param that appears in interiors/rec/main — pin with a unit test on a rec query whose param is only on the base arm (the closure-from-param shape).
- [ ] Coverage/contradict do not panic on an interiors-or-rec query.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-bench`; `./scripts/check.sh`.

## Constraints

- Builder-internal EDB-only construction may stay. Boundary `Query` unchanged. No corpus regeneration. Latent until engine-020/bench-003 feed derived queries through these walks — still land the walks so the panic is unrepresentable.
