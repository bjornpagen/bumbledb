## Capacity.lean and Oracle.lean docstrings claim an engine ceiling early exit that C14 deleted — the shipped clip is floor-only

lean-rust-drift | medium | CONFIRMED | lean-capacity-drift
outcome: fixed 09e300a7

### Summary

The capacity proof estate's prose describes the shipped clip wrongly. Four Lean doc sites still speak the pre-C14 design §4 text — the engine "ceiling exits at `sum > hi`" — while the engine's `Checker::measure_children` clips floor-only walks and deliberately completes every ceiling walk (ruling C14: on conviction the full sum is the walk-order-independent witnessed measure). `Bridge.lean` was reconciled to C14; the theorem docstrings, module header, and build-lane ledger were not, leaving the estate internally split and asserting engine behavior the engine explicitly refuses to have.

### Evidence (verified)

**The engine has no ceiling exit** — `crates/bumbledb/src/storage/commit/judgment.rs`:

- :1219-1226 (docstring of `measure_children`): "a FLOOR-only walk exits the moment `sum ≥ lo` ... a CEILING walk always completes — deciding `sum ≤ hi` needs the whole group anyway, and on conviction the full sum IS the witness, so the reported measure is walk-order-independent (ruled 2026-07-24, C14)".
- :1257-1258: the only clip predicate — `let floor_only_decided = |measure: u128| hi.is_none() && measure >= u128::from(statement.lo);`
- :1286-1288: the only `break` in the keyed walk fires on `floor_only_decided(measure)`. No `sum > hi` comparison exists in the loop.
- The C14 ruling, `docs/design/capacity-laws.md:395-397`: "on conviction the judge completes the full walk so the reported measure is walk-order-independent (the clip serves the verdict, the full sum serves the witness)."

**The four stale Lean doc sites:**

1. `lean/Bumbledb/Capacity.lean:113-118` (build-lane ledger): calls the C12 lemma the "named soundness of the engine's early-exit walk: ceiling exits at `sum > hi`, floor at `sum ≥ lo`".
2. `lean/Bumbledb/Capacity.lean:167-174` (`natSum_prefix_le` docstring): "the engine's early exit is sound in both polarities — a ceiling walk may convict the moment the running sum passes `hi` ... The design's § 4 early-exit claim is cited here, not asserted" — a bare citation of `capacity-laws.md:208-211` ("an upper-bound walk exits the moment `sum > hi`"), the design text C14 superseded in the same file.
3. `lean/Bumbledb/Oracle.lean:413-417` (`capacity_ceiling_exit_sound` docstring): "the engine's `sum > hi` early exit loses nothing".
4. `lean/Bumbledb/Oracle.lean:109-112` (module header, Capacity bullet): "the engine's clipped walk is priced sound by the C12 exit theorems (`capacity_ceiling_exit_sound` / `capacity_floor_exit_sound`)" — lumping the ceiling theorem into the pricing of a clip the engine only performs on the floor side.

**The reconciled row proving the split** — `lean/Bumbledb/Bridge.lean:188-192`: the `capacity_ceiling_exit_sound` row already carries the C14 wording ("the clip serves the verdict, while on conviction the full walk serves the walk-order-independent witnessed measure (C14)") and pins to `Checker::measure_children` with test `capacity_sum_ceiling_convicts_with_the_full_measure` (exists at `crates/bumbledb/src/storage/commit/tests/marks.rs:528`).

**Downstream stakes are real** — the differential cross-checks the full-walk measure: `crates/bumbledb-bench/src/naive.rs:479` ("C14: the engine completes the full walk on conviction"), `differential.rs:185`, `capacity/tests.rs:135` ("witnessed measure 6 (the full walk, C14)").

Note: the theorems themselves are true and stay — `capacity_ceiling_exit_sound` is a correct statement about `measureVerdict` over a prefix (it prices the verdict's early decidedness). Only the prose attributing a `sum > hi` exit to the engine is wrong.

### Failure scenario / impact

Under the repo's verify-against-in-repo-papers doctrine, Capacity.lean/Oracle.lean are the normative anchors an auditor checks first. A reader auditing the clip against them concludes the engine ceiling-exits at `sum > hi` and either (a) flags the full ceiling walk in `measure_children` as a missed optimization, or (b) "restores" the exit — which breaks C14's walk-order-independent witnessed measure and the differential's measure parity (naive.rs folds whole groups on both sides citing C14, and `capacity_verdicts_agree_with_the_model` compares witnessed measures whole). The estate currently presents soundness for a mechanism deliberately unbuilt as pricing a shipped one.

### Suggested fix

Re-baseline the four doc sites to the Bridge row's C14 wording (Bridge.lean:190 is the canonical sentence):

- `Capacity.lean:113-118` — the ledger paragraph: the C12 lemma prices the verdict's decidedness in both polarities and licenses the engine's floor-only clip; the engine completes ceiling walks for the witness (C14).
- `Capacity.lean:167-174` — drop "the engine's early exit is sound in both polarities"; cite the C14 ruling alongside §4 (the §4 exit text was superseded for the ceiling side).
- `Oracle.lean:413-417` — replace "the engine's `sum > hi` early exit loses nothing" with: the walk's verdict is decided the moment the sum passes `hi`; the engine nonetheless completes the walk so the convicted measure is the walk-order-independent witness (C14).
- `Oracle.lean:109-112` — say the floor-only clip is priced by `capacity_floor_exit_sound` while `capacity_ceiling_exit_sound` prices the verdict's decidedness under the C14 full walk.

Doc-only change in the Lean estate; no theorem statements, engine code, or tests move.