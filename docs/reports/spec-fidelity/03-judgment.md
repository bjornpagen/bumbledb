# Spec-fidelity review 03 — the commit judgment (pairing #3)

Reviewer 3 of 10 (covenant PRD 15). Lean normative. Scope:
`lean/Bumbledb/Dependencies.lean` (judgment half), `lean/Bumbledb/Txn.lean`
(judge / violation set), `crates/bumbledb/src/storage/commit/judgment.rs`,
`commit/plan.rs`, `Violations` in `src/error.rs`. Read-only; zero code changes.

## Per-theorem fidelity table

| Theorem / definition (Lean) | Engine site (Rust) | Verdict |
| --- | --- | --- |
| `Txn.judge` / `Txn.commit` — accept iff `holds`, else reject; verdict a function of theory + one final instance (Txn.lean:175-200) | `judgment.rs::judge` (77-82) over `FinalStateView` (59-73); consumed at `write.rs:135-138`, rejection aborts whole txn | FAITHFUL (modulo D1) |
| `holds` — every declared statement's judgment of the final state (Dependencies.lean:243-244) | `check_source` (220-276) + `check_target` (310-504), delta-restricted; restriction recorded in the Lean bridge text itself ("sound because an untouched binding cannot change a judgment's truth", Dependencies.lean:240-242) and backed by `State.models` induction + `Db::verify_store` | FAITHFUL |
| `holds_iff_no_violation` — accept path carries no rejection (Txn.lean:151-154) | `Violations::seal` returns `None` on empty (error.rs:786-793); `judge` maps that to accept | FAITHFUL |
| `rejection_is_complete` — completeness half: V contains EVERY violated statement of the final state (Txn.lean:280-283) | Key-violation preemption: `apply.rs:54-59` seals violated key statements and returns before phase 3 ever runs | **DIVERGENCE D1** |
| `rejection_is_complete` — soundness half: only violated statements (Txn.lean:283-284) | Both sides convict only on a genuine final-state miss: scalar U-probe miss or ψ-fail (638-649, 764-776), sweep gap (868-870), pre-existing survivor of a disestablished tuple (442-447), stranded closed-source axiom (412-420), out-of-range membership (261-270) | FAITHFUL |
| `rejection_is_complete` — nonemptiness (Txn.lean:285) | `seal` refuses emptiness; the reject arm exists only under `Some` (write.rs:136-138) | FAITHFUL |
| Violation-set representation: `Set Statement` narrowing (Txn.lean:55-60) — sort/dedup/per-direction citations declared representation | Stable sort + dedup by `citation()` = (statement, direction), source before target (error.rs:698-708, 754-792); fuzz trophy `multi-violation-citation-order` pins order | FAITHFUL (recorded narrowing; see D2 on witness choice) |
| `Containment` denotation — ∃ selected target witness with equal projected tuple (Dependencies.lean:122-125); `TargetKeyAccepted` a premise, never a conjunct | `check_scalar` probes the target key's unique U holder, then re-checks ψ on the found fact via one F get (`check_segment`, 764-776); acceptance lives in `resolve_target_key`, not here | FAITHFUL — the unique-candidate probe is complete only because the target key holds of the final state, which is exactly what the D1 preemption guarantees (adversarial note: the preemption is load-bearing here) |
| `Coverage` denotation + `Exec.sweep_covered_sound_complete` premises — token attests Ordered ∧ Disjoint; Disjoint licences the predecessor-seek entry below the fold (Sweep.lean:14-18, 38-48) | `check_coverage` consumes `DisjointDeterminantProof` by signature and calls `authorize_coverage()` at entry (671-676) BEFORE any seek; predecessor accepted only while still running at `s` (721), decisive under disjoint+ordered exactly as the Lean seam note records; window ends at source end — overhang never convicted (`one_way_overhang` respected) | FAITHFUL |
| Target-skip partition — sides partition the final state's sources; one statement cited once per genuinely violated direction (judgment.rs module doc 17-20) | Inserted survivors skipped scalar (439-441) and interval (484-489); the skipped survivor's own source-side edge probes the same missing tuple/gap (`check_source` over `plan.inserts`), so statement-level membership (all Lean requires of a `Set Statement`) is preserved | FAITHFUL |
| Delete-then-insert edges / per-family phases vs final state — `Txn.apply` add-wins set algebra (Txn.lean:111-115) | Net-coalesced delta; phase 1 deletes then phase 2 inserts under LMDB read-your-writes = `(base \ removes) ∪ adds`; re-established tuples via `deleted − inserted` with ψ-qualified marking (plan.rs:285-329; judgment.rs:340-359, one shared F get) | FAITHFUL |
| `final_state_judgment_order_free` — op order unrepresentable in the judge's input (Txn.lean:231-234) | `FinalStateView` holds only txn/schema/plan; plan lists are deterministic functions of the net-delta SET (plan.rs:133-181) | FAITHFUL |

## Divergences

**D1 — class (a).** Key-violation preemption yields an incomplete violation
set on mixed rejections. `apply.rs:54-59` (and `commit.rs:13-17`): any
violated `Functionality` statement seals and rejects before `judge` runs, so
a final state violating both a key and a containment (trivially
constructible: one colliding insert + one dangling insert) rejects citing
the key statements only. `Txn.lean:279-297` (`rejection_is_complete`,
completeness ∀ st ∈ T.statements) demands both, and `Statement.judgment`
is total over both kinds. The prose spec records the preemption
deliberately (`30-dependencies.md:67-75`, "never a mix") and it is
semantically motivated (containment probes are defined over the keyed
final state; the coverage walk's disjointness premise would be unsound
otherwise) — but the normative Lean narrowings (Txn.lean:55-80) do not
record it, so under "Lean is normative" this is engine behavior the spec
forbids. Recommended disposition: record the two-stage judgment as a Lean
narrowing (or model it), owner's call — no code change implied.

**D2 — class (b).** Witness-fact selection is spec-undetermined.
`violationSet` is a `Set Statement` (Txn.lean:146-148) carrying no facts;
the public `Violation` payload carries one convicting fact per citation,
chosen as first-discovered in scan order (stable sort, error.rs:786-793;
first pre-existing survivor + break, judgment.rs:442-447). The narrowing
(Txn.lean:57-63) covers sort/dedup/order but not which witness survives —
an observable, host-visible choice the spec does not determine. Record it;
harmless.

## GRADE: B

One confirmed class-(a) divergence under adversarial reading: the
key-preemption partition breaks `rejection_is_complete`'s cross-kind
completeness on mixed rejections — deliberate, prose-documented, and
mechanistically load-bearing (the scalar probe and the sweep token both
presuppose the keyed final state), but unrecorded in the normative Lean
model, whose bridge text explicitly claims `crate::error::Violations` as
"the complete violation set… both scan-complete sides." Everything else
is exact: the judge is a function of one final state, both sides are
scan-complete collectors, the seal is nonempty/sorted/deduped with the
documented citation order, the survivor partition preserves set-level
completeness, and the coverage dispatch consumes the
`DisjointDeterminantProof` precisely where `sweep_covered_sound_complete`'s
premises (and its recorded predecessor-seek seam) demand. Not an A because
D1 is real; not lower because the divergence is single, bounded to the
mixed-rejection payload (the verdict itself — reject — is always correct),
and already half-recorded outside the model.
