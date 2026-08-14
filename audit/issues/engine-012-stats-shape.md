# engine-012: `ExecutionStats` is three independent fields; the per-interior/per-reach rule tables are ghosts

- **Severity:** high
- **Tree:** engine
- **Status:** OPEN
- **Source:** audit/engine.md F12
- **Depends on:** engine-001 (stats mirror the pipeline sum)

## The bug

`crates/bumbledb/src/api/stats.rs:16-53`:

```rust
pub struct ExecutionStats {
    pub rules: Vec<RuleStats>,
    ...
    pub interiors: Vec<InteriorStats>,
    pub reach: Option<ReachStats>,
}
```

Representable nonsense: `reach: Some` on interiors-only, `reach: None` on a Reach run, `interiors: []` on a query that ran interiors — `profile` assembles each combination with the same flag forest as execute (`introspect.rs:251-252, 281-290, 373-375`). And the per-stage rule tables are ghosts kept from the strata design, never filled:

```rust
// introspect.rs:383-386 — interior_stats()
.map(|(i, interior)| InteriorStats {
    interior: ...,
    rules: Vec::new(),          // always empty
    emits: interior.sink.len() as u64,
})
// introspect.rs:289
reach: Some(counters.into_reach(Vec::new())),   // ReachStats.rules: always empty
```

while `stats.rs:45-46` claims "Structured stats **are** the interiors block — there is no parallel span-or-stats fork" — except these empty `rules` vecs are exactly that fork.

## Why it's wrong

The stats type can describe executions that cannot happen and cannot describe the ones that do (per-interior rule stats — the field exists, is documented as real, and is unconditionally empty). A ghost field is worse than absence: consumers write code against it and get vacuous data (Insight 3; Insight 4 for the Option-and-Vec flag pair restating engine-001's product).

## The fix

Per `audit/CONTRACT.md §C3` ("Stats/introspection"): stats mirror the pipeline sum.

```rust
pub struct ExecutionStats { pub introspection_version: u16, pub emits: u64,
    pub subsumed: ..., pub dead: ..., pub body: StatsBody }
pub enum StatsBody {
    Cq    { rules: Vec<RuleStats>, disjoint_rules: Option<DisjointRules>,
            interiors: Vec<InteriorStats> },
    Reach { interiors: Vec<InteriorStats>, reach: ReachStats /* not Option */ },
}
```

(Exact field placement may follow the code's needs; the LAW is: `reach` is not `Option` — it exists exactly on the Reach arm; interiors exist on both arms; the ghost `rules` fields on `InteriorStats`/`ReachStats` are DELETED, not populated — `InteriorStats { interior, emits }`, `ReachStats { rounds }`.)

- `empty_stats` (`introspect.rs:394-415`) is **only** reached today when `interiors.is_empty() && Empty` (`introspect.rs:214`), so the hardcoded `interiors: Vec::new()` is not a current observable hole — dead-main-with-live-interiors already falls through to `interior_stats()`. After `Empty` dies (engine-023), do **not** route dead-main-with-interiors through a zero-interior stats constructor; report real interior emits. Also drop the phantom one-element `RuleStats` empty_stats currently mints for a query with zero surviving main rules — dead main is `rules: []` plus `stats.dead`.
- Reach profile today sets `stats.rules: Vec::new()` (no main-rule node stats). Keep that observable: the Reach stats arm does not grow a main-rule table this issue.
- This is a PUBLIC type change: `INTROSPECTION_VERSION` (currently 4) increments once, covering this + engine-029 + any engine-033 string changes — coordinate so the version bumps exactly once for the whole campaign.

## Acceptance criteria

- [ ] Gone: `rg -n 'reach: Option<ReachStats>' crates/bumbledb/src/api/stats.rs` → no matches; `rg -n 'rules: Vec::new\(\)' crates/bumbledb/src/api/prepared/introspect.rs` → no matches; `InteriorStats`/`ReachStats` have no `rules` field.
- [ ] New locks: a test profiling (a) interiors-only, (b) reach, (c) dead-main-with-interiors, asserting the arm shape and that interior emits are reported in all three.
- [ ] Unchanged: answer values and error behavior identical; tests reading old stats fields are updated mechanically (field moves), never weakened (same numeric assertions).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- One `INTROSPECTION_VERSION` bump for the whole stats/labels campaign (this + engine-029 + engine-033).
- Lands after engine-001.
