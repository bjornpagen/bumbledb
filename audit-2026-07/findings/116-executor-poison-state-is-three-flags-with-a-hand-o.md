## Executor poison state is three flags with a hand-ordered precedence branch instead of one sum

category: inelegance | severity: low | verdict: CONFIRMED | finder: cross:branching
outcome: fixed 0f13feff

### Summary

The executor's execution-stopping "poison" state lives in three parallel fields on `Executor` — `all_cancelled: bool`, `origin_overflow: bool`, `measure_of_ray: Option<[u64; 2]>` — whose mutual consistency is enforced only by convention: every poison site must remember to set two fields in tandem, resets are scattered across two functions, and `execute()` drains the state through a comment-ordered pair of `if`s ("the measure's ray poison outranks the origin overflow") that adjudicates a two-poisons-at-once state the execution can never actually produce. A single set-once `poison: Option<Poison>` sum makes the tandem-write convention, the triple reset, and the precedence branch unwritable. This is a direct instance of the doctrine in `docs/design/representation-first.md` ("patch the trace of the computation with another branch, flag, or guard — and complexity piles up in the control flow... or change the data... so the case stops being expressible at all"); the field's own doc comment concedes the accretion pattern: `measure_of_ray` follows "the poison-flag shape `origin_overflow` established" (run.rs:595).

### Evidence (all verified against source)

- **The three fields**: `crates/bumbledb/src/exec/run.rs:586` (`all_cancelled`), `:590` (`origin_overflow`), `:597` (`measure_of_ray`), with the shape-copying admission at `:595-596`.
- **Tandem set-sites (the convention)**:
  - `crates/bumbledb/src/exec/run/run_node.rs:392-393` — `self.measure_of_ray = Some(...); self.all_cancelled = true;`
  - `crates/bumbledb/src/exec/run/probe_pass.rs:359-360` — same pair, with the comment `// stops the pump loops upstream`
  - `crates/bumbledb/src/exec/run/probe_pass.rs:485-486` — `self.origin_overflow = true; self.all_cancelled = true; // stops the pump loops upstream`
- **The precedence branch**: `crates/bumbledb/src/exec/run/execute.rs:399-409` — the "outranks" comment followed by sequential `if let Some([start, end]) = self.measure_of_ray { return Err(MeasureOfRay...) }` then `if self.origin_overflow { return Err(Overflow...) }`. Verified unreachable-in-tandem: any poison sets `all_cancelled`, which breaks the pump loop (`pump.rs:54`) and the routing loop (`probe_pass.rs:492`) before a second poison can fire — the branch order defends a state only the representation, not the execution, permits.
- **Scattered resets**: `execute.rs:383` clears `measure_of_ray` (in `execute()`); `execute.rs:443-444` clears `all_cancelled` and `origin_overflow` (in `run_pipeline()`, a different function on a different path).
- **Extra residue found during verification**: the only readers of `all_cancelled` are pipeline-only (`pump.rs:54`, `probe_pass.rs:492`), so the write at `run_node.rs:393` is dead on the single-node path (`pipe: None`) and is never reset there (`execute.rs:443` runs only inside `run_pipeline`). It is harmless solely because an `Executor` is bound to one plan shape (`execute.rs:381` debug_assert) — i.e., another invariant carried by convention rather than representation.
- **Doctrine check**: `docs/design/representation-first.md` (Purpose and "three spiky points of view" sections) names exactly this pattern — flag-and-guard accretion where a data change would erase the special case — as the codebase's governing rule.
- **Deliberate-decision check (refutation attempt)**: the poison-flag shape exists to keep `Result` off the per-tuple path (run.rs:595-596), and the u32-origin width was an explicit representation ruling (probe_pass.rs:470-478). Neither justifies three fields: the sum is the same one-word flag write on the cold poison path, and the finding leaves both decisions untouched. Also checked that `all_cancelled` cannot simply be derived from the poison: it has one non-poison setter (root skip crossing the virtual root, `probe_pass.rs:561`), so it must remain the stop condition — raised by the poison helper rather than by hand at each site.

### Failure scenario / maintenance impact

No bug today. The lane is maintenance: a fourth poison kind — or a site that sets its poison field without `all_cancelled` — compiles cleanly and silently loses either the early stop (execution grinds through dead work) or the error drain (the poison is swallowed and `execute()` returns `Ok`). The precedence comment must also be re-litigated by hand for every new kind. All three failure modes are unwritable under the sum.

### Suggested fix

```rust
enum Poison { MeasureOfRay([u64; 2]), OriginOverflow }
// on Executor:
poison: Option<Poison>,
```

with a set-once helper (`fn poison(&mut self, p: Poison)` that writes `self.poison.get_or_insert(p)` and sets `all_cancelled = true`), `all_cancelled` kept as the one stop condition (still set directly by the root-skip site), one reset (`self.poison = None`) at the top of `execute()`, and a single drain:

```rust
match self.poison.take() {
    Some(Poison::MeasureOfRay([start, end])) => Err(Error::MeasureOfRay { start, end }),
    Some(Poison::OriginOverflow) => Err(Error::Overflow(OverflowKind::OriginCapacity)),
    None => Ok(()),
}
```

The set-once helper preserves first-poison-wins, which is behaviorally identical to today's "outranks" ordering since two poisons never coexist. The per-tuple path still carries no `Result`.
