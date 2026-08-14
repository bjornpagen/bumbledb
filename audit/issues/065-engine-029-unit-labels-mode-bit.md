# engine-029: `IntrospectionReport` encodes its two modes as "are the unit labels empty?"

- **Severity:** medium
- **Tree:** engine
- **Status:** FIXED(9c77c002)
- **Source:** audit/engine.md F29
- **Depends on:** engine-012 (one stats/report reshape, one version bump), engine-001

## The bug

`crates/bumbledb/src/exec/introspection.rs:83-97` — one report type, two modes, distinguished by emptiness of a parallel array:

```rust
pub struct IntrospectionReport<'p> {
    pub rules: Vec<RulePlan<'p>>,
    /// Fixpoint unit labels, parallel to `rules`
    /// (`predicate p0 rule 1 delta variant 0`); empty for query-shaped
    /// programs, whose label is the rule index.
    pub unit_labels: Vec<String>,
    pub stats: crate::api::stats::ExecutionStats,
}
```

Display (`display.rs:26-31`) then mode-switches on `unit_labels.get(rule_idx)` (`Some(label)` / `None if multi` / `None`), and per-unit stats presence is ANOTHER parallel probe (`stats.rules.get(rule_idx)` miss = "fixpoint plan unit", `display.rs:184-188`). The doc-comment's label example (`predicate p0 rule 1 delta variant 0`) matches no code — actual labels are `reach base {i}` / `reach rec {i} (delta occ {d})` / `main {i}` (`introspect.rs:67,83,94`).

## Why it's wrong

Two parallel arrays whose relative lengths encode the mode is the "null in every slot" pattern spread across a struct (Insight 4): a labels/rules length mismatch is representable and silently misrenders; the mode is re-decided per row at display time rather than once at construction. The stale doc example is drift already delivered (Insight 1).

## The fix

Per `audit/CONTRACT.md §C3`: the report body is a sum matching the pipeline:

```rust
enum ReportBody<'p> {
    Cq    { rules: Vec<(RulePlan<'p>, RuleStats)> /* aligned by construction */ },
    Reach { units: Vec<(String, RulePlan<'p>)>, /* + interiors/reach stats via engine-012 */ },
}
```

- No `unit_labels` parallel array; no `stats.rules.get` miss as a mode bit. Display matches the body once.
- Label VOCABULARY stays as shipped (`reach base {i}`, `reach rec {i} (delta occ {d})`, `main {i}`) unless engine-033 changes strings — coordinate so `INTROSPECTION_VERSION` bumps once for the whole reshape (with engine-012).
- Doc comments describe the actual labels.

## Acceptance criteria

- [ ] Gone: `rg -n 'unit_labels' crates/bumbledb/src` → no matches; `rg -n 'predicate p0 rule 1 delta variant 0' crates/bumbledb/src` → no matches.
- [ ] Unchanged: rendered output byte-identical for both modes IF strings don't change (snapshot tests are the arbiter); otherwise one coordinated version bump with engine-012/033.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- One `INTROSPECTION_VERSION` increment for the whole introspection campaign. Lands with engine-012.
