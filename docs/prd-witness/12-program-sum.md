# PRD 12 — The program sum: PreparedRule and Empty stop impersonating

**Depends on:** baseline (independent of the 09–11 spine; touches
disjoint files — coordinate only if run concurrently).
**Modules:** `crates/bumbledb/src/api/prepared.rs` (the `PreparedRule`
struct, `PreparedQuery.rules`), `api/prepared/build.rs`,
`api/prepared/execute.rs`, `api/prepared/bind.rs` (`set_batch_size`),
`api/prepared/introspect.rs`, `exec/dispatch.rs` +
`exec/dispatch/exec_plan.rs` (the `ExecPlan` enum's fate),
`api/prepared/tests/*`.
**Authority:** the audit's finding 3 (~11 sites): `PreparedRule` is a
parallel discriminant (`executor: Option` — Some iff FreeJoin;
`guard_finds: Option` — guard-only; `resolved_filters`/`resolved_
selections`/`memo` — FreeJoin-shaped; `pinned` — "empty for guard
probes"), and `ExecPlan::Empty` is a statically-dead PROGRAM impersonating
a RULE (asserted unreachable in `run_rule`, special-cased in `run_bound`,
`profile`, `introspect`, `exec_plan`).
**Representation move:** two sums make both agreements unrepresentable —
the rule kind carries its own scratch, and emptiness is a program-level
variant, not a sentinel rule.

## Context (decided shape)

```rust
/// One rule's prepared artifact — the plan kind CARRIES its scratch.
enum PreparedRule {
    FreeJoin(FreeJoinRule),
    Guard(GuardRule),
}
struct FreeJoinRule {
    plan: ValidatedPlan,
    executor: Executor,                 // no Option
    finds: Vec<FindSpec>,
    resolved_filters: Vec<Vec<FilterPredicate>>,
    resolved_selections: Vec<Vec<Vec<u64>>>,
    resolved_complete: bool,
    memo: ViewMemo,
    pinned: Box<[OccurrencePin]>,
}
struct GuardRule {
    plan: GuardPlan,                    // today's ExecPlan::GuardProbe payload
    finds: Vec<FindSpec>,
    guard_finds: Option<Vec<(FieldId, ValueType)>>, // stays: the DIRECT
    // point lane is a genuine sub-mode of guard rules (Some iff the
    // all-scalar direct-decode lane applies), not a plan-kind agreement.
}

/// The program: emptiness is not a rule.
enum Program {
    /// Every rule statically dead — bind still runs (errors surface),
    /// execution touches nothing. Carries what EXPLAIN prints.
    Empty,
    Rules(Vec<PreparedRule>),
}
```

`PreparedQuery.rules: Vec<PreparedRule>` → `program: Program`.
`ExecPlan` (GuardProbe | FreeJoin | Empty) dies as a stored type — it was
the build-time intermediate whose variants the rule struct re-guarded;
`build.rs` constructs `PreparedRule`/`Program` directly. The
`exec/dispatch/exec_plan.rs` module's plan-kind surface (slot_count,
distinct_bindings dispatch) moves onto `PreparedRule` methods or inlines
at the two call sites — whichever leaves zero `unreachable!`.
`dead: Vec<DeadRule>` (EXPLAIN's reasons) stays on `PreparedQuery`
unchanged — `Program::Empty` is the all-dead case; the reasons list
already exists.

Dying asserts/special-cases (audit's list): `execute.rs` `run_rule`'s
`ExecPlan::Empty => unreachable!`, the executor/`guard_finds` expects
(×4: "free join plans carry executor scratch" ×2, "checked by the
caller", build's), the `run_bound`/`profile` early-return Empty branches
(become one `match self.program`), `introspect.rs`'s two Empty arms and
plan-kind destructures, `exec_plan.rs`'s two.

## Technical direction

1. Land the sums in `api/prepared.rs`; delete `ExecPlan`.
2. `build.rs`: the per-rule pipeline returns `PreparedRule` directly
   (classify → `GuardRule`; else plan/validate/executor → `FreeJoinRule`
   with executor constructed unconditionally — no Option); the all-dead
   branch returns `Program::Empty`. `pending_literals` matches
   `PreparedRule::FreeJoin` (its Guard early-return dies into the match).
3. `execute.rs`: `run_bound`/`profile` match `Program` once at entry;
   `run_rule` takes `&mut PreparedRule` and matches the two real kinds —
   the fast-path/latch logic moves into the FreeJoin arm unchanged.
   `execute_guard_direct`'s "checked by the caller" expect becomes a
   typed precondition: the direct lane is entered only via a
   `GuardRule` whose `guard_finds` is Some (one match, no expect).
4. `bind.rs` `set_batch_size` iterates FreeJoin arms naturally.
5. `introspect.rs`: plan-kind reporting matches the sums; the Empty
   report path reads `Program::Empty` + `dead`.
6. Tests: `statically_empty.rs` re-anchors (`ExecPlan::Empty` assertions
   → `Program::Empty`); `latch.rs`'s `ExecPlan::FreeJoin(plan)`
   destructure → `PreparedRule::FreeJoin`; guard/explain tests likewise.
   No behavioral assertion changes.

## Passing criteria

- `[shape]` `grep -rn "ExecPlan" crates` → zero hits;
  `grep -rn "unreachable!" crates/bumbledb/src/api/prepared` → zero
  plan-kind asserts (surviving hits only in unrelated arms, each
  justified by its message).
- `[shape]` `FreeJoinRule.executor` is not `Option`; no field on either
  rule struct is documented as "empty/None for the other kind".
- `[test]` The prepared suites (guard, latch, statically_empty, explain,
  view_memo, snapshot, params, rules, folded) green in re-anchored form
  with unchanged assertions; the trace suite's rule-span counts
  unchanged.
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

`40-execution.md` § access paths: the three plan kinds sentence becomes
"two rule kinds and the empty program" (the Empty paragraph from CT 10
re-anchors, semantics unchanged).
