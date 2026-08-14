# engine-031: the key-probe direct path re-matches the gate's pattern and swallows mismatch as `Ok(())`

- **Severity:** medium
- **Tree:** engine
- **Status:** OPEN
- **Source:** audit/engine.md F31
- **Depends on:** engine-001 (the parsed lane), engine-008 (one protocol consumes it)

## The bug

`execute.rs:100-110` gates on the pattern; `execute_key_probe_direct` (`execute.rs:408-418`) slice-matches the SAME pattern and silently returns empty on failure:

```rust
let [
    PreparedRule::KeyProbe(KeyProbeRule {
        plan: key_probe,
        key_probe_finds: Some(key_probe_finds),
        ..
    }),
] = self.body.rules()
else {
    return Ok(());     // gate and body disagree ⇒ silently no answers
};
```

## Why it's wrong

Parse, don't re-validate (Insight 6): the gate already established the shape; the body re-derives it and — worse than a panic — maps disagreement to a *dropped-answers* success. If the two patterns ever drift (the exact drift `profile` already exhibits, engine-008), the failure mode is silent wrong results, the worst class.

## The fix

Per `audit/CONTRACT.md §C3`: the direct lane is parsed ONCE at build into pipeline data (engine-001) — either its own arm (`Cq`-with-direct-probe) or a build-computed property. `execute_key_probe_direct` takes `&KeyProbeRule` (and the typed finds) as parameters from the match that dispatched it; the `else { return Ok(()) }` is unwritable because there is no second match.

## Acceptance criteria

- [ ] Gone: `rg -n 'else \{\s*return Ok\(\(\)\)' crates/bumbledb/src/api/prepared/execute.rs` → no silent-empty arm in the direct path; the slice re-match deleted.
- [ ] Unchanged tests: key-probe fast-lane tests green UNCHANGED; answers identical.
- [ ] New lock: covered by engine-008's execute/profile parity test.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- The point lane's no-sink decode path is performance-essential — keep its body; only its dispatch parses once. Lands with engine-001/008.
