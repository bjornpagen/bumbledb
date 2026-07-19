# Disjunctive containment (sum-domain references) — feature investigation

Date: 2026-07-19. Read-only investigation; no code changed. Repo snapshot:
`/Users/bjorn/Documents/bumbledb/.claude/worktrees/host-idiom-040` (main @ 8e30387e);
workload: `/Users/bjorn/Documents/primer/src/tools/graph-builder`.

Proposed feature: `contained(on(task,"subject"), anyOf(on(A,"id"), on(B,"id"), …))` —
a containment whose target is a union of N faces; a source fact commits if its
projected tuple appears in AT LEAST ONE target face.

---

## 0. The finding that reframes the question

The motivating workload does not want this law — in two independent ways.

**(a) task.subject is a TAGGED sum, and the calculus already states tagged sums.**
`primer/src/tools/graph-builder/store/schema.ts:879-895` declares
`task = relation("task", { id: u64.fresh, kind: TaskKind.id, sheet: u64, subject: u64 })`
with the comment: "`subject` is the task's operand row id in the kind's home relation —
a u64 pointer whose meaning is kind-scoped … Kind-scoped means no single containment can
hold it." The discriminant (`task.kind`, a closed vocabulary, containment-held at
schema.ts:1306) selects which of 8 relations the subject targets (Enrich/Cartograph→sheet,
Author→grp, Realize→strandEdge, ReviewEdge→programEdge, ReviewDissolve→dissolve,
ReviewPrecourse→assumption, JudgeCandidate→candidateEdge, Supervise→task itself).
bumbledb's dependency calculus already states exactly this shape as N conditional
containments — the discriminated-union derivation, `docs/architecture/30-dependencies.md`
§ "The derivations" and cookbook recipe 22: `Task(subject | kind == Author) <= Grp(id)`.
σ on the source is legal today in both hosts (30-dependencies.md: "Selections may appear
on either side"; TS S2 PRD: `contained(on(A.where({f: Kind.V}), "x"), on(B, "y"))`), and
primer already uses target-side `where` (schema.ts:1282-1283). The tagged spelling is
strictly stronger than `anyOf` (unambiguous referent, exclusivity as a theorem via the
parent key) and strictly cheaper (one φ-literal compare per non-matching statement + ONE
probe on the matching arm, vs N probes).

**(b) task.subject dangles BY DESIGN, so NO containment — tagged or anyOf — may hold it.**
The task ledger is append-only while operands are deleted as settle receipts (judged
dissolves delete their row; both JudgeCandidate settle branches delete the candidate;
refuted claims delete their rows; repartition deletes grp rows — view.ts:358-368). Dead
work is REPRESENTED as a dangling subject (`operandLiveOf` → `operandLive: false`,
view.ts:370-405). An always-on final-state containment would convict those legal deletes —
the exact failure mode primer already documents for `steerScope.grp` (schema.ts:952-977:
"a contained `grp` field would make the honor-write unwritable or force ledger deletion —
both illegal"). task.subject is a weak reference; bumbledb judges committed states, always
(30-dependencies.md § judged on final states — the modes died), so the honest declaration
is bare. rebirth.ts (lines 12-19, 150-167, 537-548) preserves the pointer by verbatim
interior-id copy precisely BECAUSE it is invisible to statements — no remap table exists
or could be written.

Everything below evaluates the feature on its own terms anyway.

---

## 1. FEASIBILITY

### 1.1 Descriptor shape
Today: `StatementDescriptor::Containment { source: Side, target: Side }`
(`crates/bumbledb-theory/src/schema.rs:296`); Lean `Statement.containment (source target : Atom)`
(`lean/Bumbledb/Schema.lean:428-455`, Atom = relation + projection + selection). The union
form is `source: Side, targets: Vec<Side>` (nonempty; N=1 degenerates to today's form —
by the canonical-utterance law the N=1 spelling should be BANNED as the plain containment
respelled, mirroring `{1..*}`/`CardinalityContainmentWindow`). Sealed representation:
the typed containment arena (`Schema::containment` witnesses) would carry the target list;
`Schema::dependents` (containment witnesses indexed by target-key witness) becomes
one-to-many: each of the N faces' keys indexes the same statement.

Engine detail (agent-verified): the theory-crate descriptor is already variable-arity —
`StatementDescriptor::Containment { source: Side, target: Side }`
(`crates/bumbledb-theory/src/schema.rs:288-313`) with `Side` heap-boxed
(`Box<[FieldId]>` projections, `Box<[(FieldId, LiteralSet)]>` selections), so
`targets: Box<[Side]>` fits with no layout assumption. The SEALED engine struct is
strictly two-sided: `ContainmentStatement { id, source, target, enforcement, checks,
mirror }` (`crates/bumbledb/src/schema.rs:397-418`), with `CompiledSides` hard-coded
`{source, target}` pairs (`storage/commit/judgment.rs:124-127`) and ONE
`Enforcement { target_key, key_permutation }` — a union needs per-face
`(Side, Enforcement, check)` lists. The `mirror` (`==`) pairing assumes exactly two
sides — `==` over a union must be refused (consistent with the Lean finding). In-house
disjunctive precedent to copy: `LiteralSet::Many` (theory schema.rs:226-237) — sorted,
duplicate-free, ≥2 members, degenerate spellings rejected.

### 1.2 Fingerprint encoding
The fingerprint is Rust-side only (no Lean encoding proofs exist — confirmed; "fingerprint"
appears in Lean only as bridge prose). Canonical bytes for a union: length-prefixed list of
target Side encodings. Ordering: DECLARED order must be canonical-or-rejected, not silently
sorted — the codebase's precedent is the σ literal-set rule (30-dependencies.md: "canonical
form sorted and duplicate-free — validation sorts and rejects the degenerate spellings")
and FieldSet (sorted, dup-free, with Projection retaining statement order beside it). So:
targets sorted by (relation id, projection, selection) at validate, duplicate faces
rejected (`DuplicateUnionTarget`), and the sorted form is the fingerprint input. Precedent
for variable-length statement payloads exists (selection literal sets; closed member sets).

Engine detail: containment fingerprints as form tag `1` + `put_side(source)` +
`put_side(target)` (`crates/bumbledb/src/schema/fingerprint.rs:106-110`); `put_side`
already length-prefixes every list (`put_len`, u32 LE, line 158; contract at lines 5-6:
"no two schemas can alias to one byte stream"). A union takes a NEW form tag (tombstoned
tags never reused — fingerprint.rs:212-215, descriptor_codec.rs:205), `put_len(N)` +
per-face `put_side` in canonical order, and a `FORMAT_VERSION_LABEL` bump
(fingerprint.rs:35, currently `b"bumbledb-schema-v4"`). Decoder `side()`
(descriptor_codec.rs:282-318) already loops counted reads — mechanical.

### 1.3 Validation
Per face, verbatim reuse of today's containment roster (30-dependencies.md § validation
roster): each target projection resolves a declared key of its relation
(`resolve_target_key` / `NoMatchingTargetKey`), arity match, positional structural type
equality source↔each face. New rules: N ≥ 2 (N=1 is the plain containment respelled —
banned spelling); duplicate faces rejected; v0 refusals should extend — no interval
positions in any face (pointwise coverage against a union of interval sets is a genuinely
new sweep), no closed faces in v0 (or compile the member-set arm per face — possible but
more machinery), and no `==`/window interaction (the backward direction of a union
containment is not a containment; keyed-equality uniqueness fails across faces).

Engine detail: `validate_containment` (`crates/bumbledb/src/schema/validate.rs:642-747`)
runs side shapes → arity (`ContainmentArityMismatch`) → positional types
(`ContainmentTypeMismatch`) → selection semantics → closed×interval refusal →
`resolve_target_key` (validate.rs:1164-1279; `NoMatchingTargetKey` /
`NoPointwiseTargetKey` with `TargetKeyCandidate` evidence) → closed-source decidability.
A union replays this pipeline per face, mints one `Enforcement` per face, extends
`DuplicateStatement`'s normalized comparison, and needs a mixed-enforcement ruling
(closed + ordinary faces in one union — probably refused v0, house style: refusal with
trigger).

### 1.4 Commit judgment — cost model
Lean side (agent-verified, `lean/Bumbledb/Oracle.lean`, `Txn/DeltaRestriction.lean`):
- Source side: per genuinely-inserted φ-fact, N keyed point probes worst case,
  short-circuit on first hit (violation costs exactly N). Today: 1
  (`ind_source_plan_consultations = 1`, Oracle.lean:516). Still O(N log n) per
  delta-touched fact with N a schema constant — the acceptance gate's cost law survives;
  the plan is N independent exact-key descents, NOT a join, so the E1 join fence
  (`joined_window_form_uninhabitable`) is not implicated; `AdmissibleForm` already
  supports multiple consulted surfaces and "never merges answer sets" is untouched.
- Target side (the expensive novelty): a key tuple disestablished in face k convicts only
  if a surviving source still demands it AND no holder for the tuple stands in ANY other
  face — so each disestablishment costs 1 reverse-edge probe + up to N−1 cross-face
  probes. Re-establishment becomes cross-face: the byte-exact engine/model coincidence
  recorded at DeltaRestriction.lean:584-605 ("coincidence (2)", keyed-bucket subsingleton)
  breaks; the engine needs explicit N-face re-establishment probes. Reverse-edge (`R`)
  namespaces: today one per statement keyed by the target; a union statement needs its
  reverse edges consultable from N different target relations' delete paths — N delete-path
  hooks for one statement.
- Semantics of the delete: a delete from A is LEGAL if the value coincidentally exists in
  B. Under per-(relation,field) fresh mints (`10-data-model.md:323`: "comparing fresh ids
  across relations compares two unrelated mint[s]"; `Txn/Fresh.lean:65-68`: "sequences
  never interact") this is not an edge case — dense independent counters make cross-face
  numeric collision the NORM. See §3.

Engine sites (agent-verified, `storage/commit/judgment.rs`, entry `judge` line 80, called
from `storage/commit/write.rs:138-141`):
- Insert side `check_source` (judgment.rs:285-362): plan derivation emits one `EdgeOp`
  per outgoing containment whose σ the fact satisfies (`storage/commit/plan.rs:316-353`);
  `check_scalar` (807-822) is "one `U` get on the target key's determinant. A miss is the
  violation." Probes are key-sorted for B-tree locality (a measured 1.20-1.33x license,
  judgment.rs:293-302). Union = short-circuited N-probe loop; the probe order over faces
  must be CANONICAL for the key-least-violator witness-determinism law
  (`tests/witness_stability.rs`).
- Delete side `check_target` (judgment.rs:396-603): per-statement reverse (`R`) edges
  keyed `reverse_prefix(statement_id, determinant)`; disestablished tuples
  (plan.rs:191-237) drive one `R`-prefix seek per (dead tuple × dependent statement);
  any survivor is `Violation::Containment { direction: TargetRequired }`. Dependents are
  registered per-KeyId (validate.rs:115,149) — a union statement must register under all
  N faces' keys, and a survivor found after deleting from face k is NOT yet a violation:
  conviction requires N−1 forward `U` re-probes (the value may stand in another face).
  Re-establishment (judgment.rs:424-443) must consider re-landing in ANY face — this is
  the Rust twin of the Lean "coincidence (2)" break (§2).

### 1.5 Violations rendering
Mechanically easy: the citation carries statement id + source fact bytes (the cited
relation is the containment's SOURCE — 30-dependencies.md § rendering — unchanged);
`render_declared`'s bijection extends to the anyOf spelling; the message becomes "tuple
absent from every face: [A(id), B(id), …]". Direction citations: today a containment cites
per direction (source/target); a union target-side conviction should cite the statement
once with the disestablished face named in the payload. Sealed-violations ordering law
(`Violations::seal`) unaffected.

Engine detail: `Violation::Containment { statement, direction, fact }`
(`crates/bumbledb/src/error.rs:952-959`; `Direction::{SourceUnsatisfied, TargetRequired}`
919-927); text via the ONE bijective renderer (`schema/render.rs:405-428`); plain-data
`RenderedViolation` (render.rs:26-40). Union needs a canonical spelling in the renderer
(the bijection law, render.rs:22-24), "no target in any member" / "still required and no
other member holds it" message forms; the payload stays the source fact (doctrine at
error.rs:949-951) and the dedup key (error.rs:993-1004) already collapses N misses to one
citation.

### 1.6 The TS class-map story (agent-verified, `ts/src/law.ts`, `scope.ts`, `lower.ts`)
This is where the feature stops being an extension and becomes a rebuild:
- Classes are computed FROM the laws by a union-find — type-level (`AddPair`,
  law.ts:192-205, disjoint components) and runtime (`makeUnionFind`, law.ts:362-402).
  Feeding anyOf through the pair mechanism transitively merges A.id with B.id into one
  component with two generators — the ClassWall fires VERBATIM ("the statements unify two
  generators into one class — two mints cannot share a carrier", law.ts:455-461; pinned
  ts/test/law-typing.test.ts:130-161). The wall calling this a contradiction is BY DESIGN
  and CORRECT for pair-edges; a lawful union needs a NEW node kind with one-directional
  subset edges into member classes — union-find cannot represent it; the class map
  becomes a DAG and the wall check becomes reachability ("no class reachable-equal to two
  generators"), a different algorithm at BOTH tiers, including the tail-recursive
  type-level one already at instantiation-budget scale (law.ts:46-50).
- A column's class is a single `string | undefined` at every layer (`RelationClasses`
  law.ts:65, `ClassedField` scope.ts:198, wire `FieldSpec.newtype` lower.ts:69-76). No
  union-of-classes representation exists anywhere.
- Joins are class EQUALITY, bare-pairs-only-with-bare (`JoinOk` scope.ts:250-266 mutual
  extension; `fieldJoins` scope.ts:284-290 `===`; five enforcement sites: EnvJoinOk,
  SiblingJoinOk, EqOk, `advanceMatch`, `validateCond`). A join `task.subject = A.id`
  under a union class is a NARROWING — it needs a subset partial order between classes,
  breaking the deliberate symmetry of JoinOk. And semantically the narrowing join is a
  filter that keeps accidental cross-mint collisions (§3) — the answer set contains
  subjects whose numeric value happens to exist in A regardless of which store they
  "mean." So a join against a union class means something only if the union's referent is
  unambiguous, which untagged anyOf cannot promise.
- Wire + engine coherence: `StatementSpec` has exactly one target SideSpec
  (spec.ts:143-156); the engine's newtype-coherence law (`ErrNewtypeMismatch`,
  db.ts:1469-1471: faces "agree on their newtype, or neither carries one" — the checked
  taxonomy of 30-dependencies.md, owner ruling 2026-07-18 "option 1") REJECTS a source
  paired with N differently-labeled generator faces by construction. The wall carve must
  be cut in the engine's lowering too, in both hosts, identically.
- Row typing is the cheap part: rows are structural (no value brands); task.subject stays
  `bigint` either way.
- The SDK's own doctrine already rules on this exact shape: cookbook law 3
  (ts/COOKBOOK.md:75-77): "A field in no law is bare, and bare pairs only with bare in
  queries — a deliberate sum-domain pointer stays legal because you simply write no law
  over it." Same ruling in scope.ts:8-10 and law.ts:24-25.

### 1.7 Rust notation spelling
Statements are the operator algebra; there is no keyword sugar (the `union` keyword was
explicitly refused — 30-dependencies.md § statements: "no `union` block"; blessed sugar
`key`/`in`/`union` lost by owner ruling, "reverses if: never"). A faithful spelling would
be a right-hand disjunction of atoms: `Task(subject) <= Grp(id) | Sheet(id) | …` (with
`|` already meaning selection — a collision; `\/` or `,`-list would be new grammar). Any
spelling adds a fourth statement form to a chapter that opens "exactly three statement
forms … and nothing else." TS spelling would be `anyOf(on(A,"id"), on(B,"id"))` as a new
face-set combinator in statements.ts.

Engine detail: today's macro spelling `Account(holder) <= Holder(id)` (with σ:
`Submission(kind | status == Frozen) <= Kind(id | mastered == true)` —
`crates/bumbledb-macros/src/lib.rs:20,47`; `tests/schema_macro.rs:47-48,692`); runtime
spec `StatementSpec::Containment { source, target, bidirectional }`
(`bumbledb-theory/src/schema/spec.rs:163-169`). The most house-idiomatic union spelling
is the brace-set already used for literal sets: `Task(subject) <= { A(id), B(id) };`
(macro lib.rs:60-63 reads `{…}` disjunctively) — avoiding the `|`-collision noted above.

### 1.8 One more engine collision: closed-vocabulary faces
The engine has NO class calculus (grep across `crates/bumbledb/src` finds none — the
wall is TS-side only); what it has is the closed-reference roster discipline:
`resolve_target_key` requires a closed target to project exactly the synthetic id
(validate.rs:1180-1187) and the TS SDK requires roster agreement per paired face
(`assertRosterAgreement`, ts/src/statements.ts:158-172: "faces pair closed-with-closed
through one roster or bare-with-bare, never across") plus one declared containment per
closed reference (ts/src/schema.ts:240). An `anyOf` over two closed vocabularies breaks
"one meaning, one spelling": a source column typed with roster-A's handle descriptor
cannot simultaneously agree with roster B — the source must go bare-u64 or unions need
their own roster notion. So even the closed-target O(1) member-set arm does not make
union targets cheap to admit.

---

## 2. DECIDABILITY

**Satisfaction stays trivially decidable.** The judgment remains a finite membership
test — Lean's denotation generalizes from `∃ g ∈ B …` to `∃ t ∈ targets, ∃ g ∈ B_t …`
(view form: `View A φ X ⊆ ⋃ View B_t ψ_t Y_t`); the executable checker `containB`
(Decide.lean:651-681) becomes `targets.any`. No negation enters, no denial-constraint
class opens. The decidability firewall (engine judges satisfaction, never implication)
is untouched BY the engine — though for the record, disjunctive INDs are exactly where
implication theory gets dramatically worse, which the firewall renders moot but the
"presumed undecidable" recital in 30-dependencies.md would want a sentence.

**The class calculus is the real coherence question, and the carve is representable but
not free.** The 0.3.0 wall's contradiction is precise: pair-edges into one union-find
component with two mints. The lawful exception cannot be an exception to the merge — it
must be a different edge kind entirely: a sum node with directed subset edges into member
classes (DAG), the wall reformulated as "no equivalence class reachable-collapses two
generators," joins reformulated from equality to a partial order. That carve is SOUND
(subset edges never merge generator components; the wall's theorem survives as a
reachability statement) but it does unravel the wall's ALGORITHM at both TS tiers and its
engine twin — equality-over-disjoint-sets is load-bearing in ~7 sites and in the
instantiation-budget design of the type-level fold. It also weakens the wall's teaching:
today "two mints cannot share a carrier" is one sentence with no exceptions; after the
carve it is one sentence plus a subtype lattice.

**Lean obligations (agent-verified):** option (a) — extend the existing arm, not a new
judgment class. `target : Atom` → `targets : List Atom` (nonempty), per-face
`TargetKeyAccepted` (the target-key rule is an acceptance HYPOTHESIS, never a conjunct of
the denotation — Dependencies.lean:16-26, 168-170 — so it per-face-izes cleanly). Reproof
burden concentrates in: `containment_delta_restriction` (DeltaRestriction.lean:336 —
`removedTargetKeys` becomes per-face with cross-face survivorship; genuine reproof),
`containment_plan_decides` + `containmentForm : AdmissibleForm` (Oracle.lean:647,
Admission.lean:343 — consulted-surface index `Bool` → `Option (Fin n)`; the gate's cost
law survives, N probes per touched fact, no join shape), `containB_iff` (mechanical),
`contains_iff_view_subset` (mechanical, union view). Must NOT extend: `==`/
`KeyBackedEquality` (uniqueness across faces fails — same tuple in two faces breaks the
bijection; countermodel would be trivial to write), windows over unions, interval faces
(coverage against a union of interval sets is a new sweep — refuse v0). Bridge ledger
rows (Bridge.lean, count-pinned) added for each generalized theorem.

**Never-reissue interaction (the poisoned well):** Fresh mints are per-(relation,field)
sequences that "never interact" (Txn/Fresh.lean:65-68; Rust: `Generation::Fresh` doc,
theory schema.rs:124-126, burn-on-abort at `storage/commit/write.rs:86-110,171-181` —
commit d08651b4). Mechanically there is no enforcement interaction (the judgment probes
final-state `U` keys, never `Q` high-waters); the interaction is purely semantic.
Never-reissue is per-sequence.
Consequences for exists-in-any over fresh-keyed faces: (i) dense independent counters
make equal u64s in different faces the NORM, so a dangling pointer is usually
accidentally witnessed by an unrelated face — the law is nearly vacuous as an integrity
statement; (ii) a delete from face A legally commits when an unrelated equal-valued key
stands in face B — soundness holds, meaning fails; (iii) legal re-supply lets the witness
silently migrate between faces over time. The model would happily prove the weak
exists-reading; nothing in it can promise "this id lives in exactly ONE of these stores"
without disjoint id spaces (host discipline the engine cannot state) or an exclusivity
companion law that does not exist. The tagged form has none of these problems — the
discriminant names the face, exclusivity is the parent key's theorem
(30-dependencies.md § discriminated union, theorem 3).

---

## 3. GOAL ALIGNMENT — should the calculus state sum-domain pointers?

**The case FOR (steelman):** 30-dependencies.md's own creed is "generality of
representation, discipline of acceptance," and its brag list includes "conditional
reference targets (the arm's relation is selected by a discriminator value)." A sum-shaped
pointer IS a real relational shape (dependency theory has disjunctive INDs; LAV data
integration lives on them), the enforcement plan exists and prices within the gate's
O(log n)-per-touched-fact law (N constant), the Lean extension is honest (an indexed
existential, no join, no negation), and every future workload with a genuinely untagged
sum pointer otherwise inherits primer's fate: a bare column, a class hole, and host gates.
The engine's refusal to state a law the host must then enforce by hand is exactly the debt
the census method exists to find.

**The case AGAINST (the design's answer, and it is the honest one):**
1. **The tagged form is already in the vocabulary and is the better law.** Discriminated
   union = N conditional containments (σ on source) + the discriminant's vocabulary
   containment: unambiguous referent, exclusivity as a theorem, 1-probe enforcement,
   zero new forms. anyOf is the tagged form with the tag erased — strictly less
   information stated at strictly more cost.
2. **Untagged exists-in-any over fresh-keyed faces is semantically near-vacuous** (§2):
   per-relation mints collide by construction, so the law mostly certifies numeric
   coincidence. A law that cannot distinguish a valid pointer from a collision is not a
   stated invariant; it is false confidence with a fingerprint. bumbledb's whole posture
   is that a stated law is a measured promise — this one cannot keep its promise.
3. **The censused workload would not use it.** Primer's subject must dangle (append-only
   ledger over deletable operands; dangling = the representation of dead work), so
   membership-at-final-state is the WRONG law tagged or not; and the two gates named in
   the prompt (dissolveUnjudgedGate gates.ts:1194-1233, preCourseUnjudgedGate
   gates.ts:1318-1371) are quiescence laws — argmax-over-attempts with verdict-ABSENCE as
   a state, over the kind-scoped join — which the gate docs themselves say is "outside
   the IR vocabulary" INDEPENDENT of subject typing. Disjunctive containment deletes zero
   gate lines. The trigger law of the freeze census (reached-for FIRES, never-reached is
   DECLINED) says: this row is DECLINED — no consumer reached for it.
4. **The wall is the feature.** "Two mints cannot share a carrier" is the SDK's one-line
   type discipline; the carve costs a subtype lattice across ~7 judgment sites in three
   implementations (TS type tier, TS runtime, engine coherence law) to admit a law whose
   flagship consumer is a weak reference.
The honest current answer — "deliberately bare, engine judges nothing, host owns the
discipline" (cookbook law 3; schema.ts's own comment cites PRD-14 minting discipline) —
is not a gap apology; it is the correct statement that this pointer's integrity is not a
final-state property.

---

## 4. COST BY LAYER vs DO-NOTHING

Feature cost:
- **Lean:** moderate. Arm generalization + per-face key premise + reproofs
  (delta-restriction cross-face survivorship is the hard one; plan calculus index
  generalization; new countermodels for the ==/window refusals; Bridge ledger rows).
- **Theory crate + engine:** new descriptor variant (or generalized arm) + codec +
  fingerprint list encoding with a `FORMAT_VERSION_LABEL` bump (sort/dedup/N≥2 canon) +
  sealed `ContainmentStatement`/`CompiledSides` rework from a two-sided pair to per-face
  lists + validation roster rows + N-probe source arm with canonical probe order (the
  witness-determinism pin) + dependents registered under all N faces' keys + cross-face
  re-establishment and N−1 forward re-probes on the delete path + violation spelling in
  the bijective renderer.
- **TS SDK:** the largest line item — class map union-find → DAG with sum nodes at type
  AND runtime tiers, wall check → reachability, JoinOk equality → partial order at five
  sites + diagnostics, new anyOf face combinator, new StatementSpec variant, engine
  newtype-coherence relaxation kept identical across both hosts (the two-hosts-judge-
  identically ruling).
- **Docs:** 30-dependencies.md's opening sentence ("exactly three statement forms")
  changes; the acceptance gate, validation roster, enforcement summary, cookbook (law 3
  is directly contradicted), and the S2 PRD surface all move.
- **Opportunity cost:** the structural-1.0.0 packet is mid-flight; this cuts across S1/S2
  (statement algebra) and the frozen fingerprint.

Do-nothing cost (honest accounting):
- The two named gates: ~150 lines in gates.ts — but they are NOT attributable to the
  missing law (they are join+argmax vocabulary gaps; see §3.3). Attributable to bare
  subject: the membership HALF of operandLiveOf/enabledTasks/resolveOperand (~100-200 of
  ~525 lines across view.ts/mint.ts/dispatch.ts), which would anyway stay host-side for
  their state checks — and which a containment could not replace because subjects must
  dangle.
- One real documented incident: dispatch.ts:244-247 (R6 autopsy — 26 attempts dispatched
  against deleted subjects because enablement was computed at wave start). A containment
  would NOT have caught it (the deletes were legal; the bug was stale reads at dispatch).
- The class hole: task.subject joins only against bare columns; kind-scoped joins have no
  typed spelling. The fix the census actually FIRED is keyed-get/typed lookup (70-api.md
  ledger: task-by-(kind,subject) re-implemented host-side) — not a new statement form.
- Future sum-shaped pointers: the survey found ONE other bare pointer (steerScope.grp),
  single-target and bare on purpose. No untagged sum pointer has ever been sighted.

---

## 5. VERDICT

**REJECT** (as proposed: untagged anyOf target union), with the reasoning bare:

1. The tagged sum is already a stated law in today's vocabulary — N source-selected
   conditional containments + the discriminant containment; strictly stronger, strictly
   cheaper, both hosts, zero new forms. A workload with a discriminant should write that.
2. Untagged exists-in-any over per-relation fresh mints certifies numeric coincidence,
   not reference integrity — the law cannot keep the promise its spelling makes. That is
   disqualifying for a calculus whose statements are measured promises.
3. The flagship consumer would not adopt it: primer's subject is a weak reference that
   must dangle, and its gates are join/argmax gaps, not membership gaps. Under the
   census's own trigger law this is DECLINED vocabulary.
4. The costs concentrate exactly where the design is proudest: a fourth statement form in
   a chapter built on "exactly three," and a subtype lattice through the one-generator
   wall in three implementations.

**Recorded reopen trigger (the defer half):** a censused workload exhibiting an untagged
sum pointer whose faces share one id space (supplied ids, not fresh mints) and whose
references must hold at every committed state. If that ever appears, the Lean path is
pre-surveyed (option (a): indexed existential, per-face key premise, reproofs named in
§2) and the fingerprint/codec canon is sketched in §1.2 — but it re-enters engine-first
with its own ruling, and the ==/window/interval interactions stay refused.

**What primer actually needs (name the replacement, per house rule):** (i) the
already-FIRED keyed-get/typed lookup for task-by-(kind,subject); (ii) a typed spelling
for the kind-scoped join (source-filtered containment already exists as law vocabulary —
the gap is QUERY-side: joining subject against a kind-selected target, which is a
narrowing the bare-pairs-only rule currently refuses); (iii) latest-attempt/argmax with
absence-as-state, which is aggregate vocabulary, a separate census row. The Supervise arm
(subject → task.id, never deleted) is the one containment primer could declare TODAY with
a source `where` — worth mentioning to the primer side.
