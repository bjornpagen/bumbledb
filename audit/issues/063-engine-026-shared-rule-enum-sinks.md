# engine-026: one shared rule enum across two sink protocols forces `run_into_projection` to unwrap Recursive

- **Severity:** medium
- **Tree:** engine
- **Status:** FIXED(1af537e5)
- **Source:** audit/engine.md F26
- **Depends on:** engine-002 (the two-variant rule enum)

## The bug

Main's `run_rule` (`execute.rs:351-381`) matches `EitherSink::Projection | Aggregate` to monomorphize `run_join` — correct, main can aggregate. Interiors and rec go through `run_into_projection` (`reach.rs:568-649`), which re-handles KeyProbe and unwraps Recursive to FreeJoin:

```rust
let rule = match &mut rules[rule_idx] {
    PreparedRule::FreeJoin(rule) => rule,
    PreparedRule::Recursive(rule) => &mut rule.variant.rule,
    PreparedRule::KeyProbe(_) => unreachable!("handled above"),
};
```

The projection-only nature of derived tables is real (folds through cycles are refused; `PreparedInterior` has a `ProjectionSink`) — but the RULE type shared across both protocols doesn't say so, so the derived runner carries arms for states its sink story excludes.

## Why it's wrong

The essential split (derived tables are projection-shaped; main may aggregate) exists in the sink fields but not in the rule lists those sinks consume, so the derived-side runner is written against the union of all rule shapes and must re-derive "which can actually appear here" per match (Insight 3: the type should carry the constraint the module doc states).

## The fix

Per `audit/CONTRACT.md §C3`: with engine-002, interior/base lists are `Vec<PreparedRule>` (`FreeJoin | KeyProbe` — both legal into a `ProjectionSink`), rec arms are `Vec<RecArm>`. `run_into_projection` then has exactly two cases (its real protocol) and a separate thin entry for rec arms taking `&mut RecArm` — the `unreachable!("handled above")` and the Recursive unwrap delete. Main's `EitherSink` match is untouched (essential). `EitherSink` stops being imported by reach.rs (engine-036).

## Acceptance criteria

- [ ] Gone: `rg -n 'unreachable!\("handled above"\)' crates/bumbledb/src/api/prepared/reach.rs` → no matches; no Recursive arm anywhere in reach.rs (covered by engine-002's grep); `rg -n '_either_sink_marker|use .*EitherSink' crates/bumbledb/src/api/prepared/reach.rs` → no matches.
- [ ] Unchanged tests: interior/rec answers byte-identical across the suites; aggregate-in-main behavior untouched.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Projection-only derived sinks are semantics (folds through cycles refused) — unchanged. Rides engine-002. Absorbs engine.md F36 (`_either_sink_marker` and the unused `EitherSink` import).
