# Lane-1 "mintable pins" reshape — feasibility research

Whisper: primer's `Pin`/`Outcome`/`SteerKind` closed vocabularies carry frozen-dead
handles legislated only in prose; reshape them bare-tier → payload-tier and legislate
the freeze as ψ-selected containment law.

Investigated 2026-07-17. Primer main checkout: `/Users/bjorn/Documents/primer`
(SDK `@bjornpagen/bumbledb` **0.4.0**, `package.json:121`). Bumbledb worktree:
`/Users/bjorn/Documents/bumbledb/.claude/worktrees/host-idiom-040` (one commit behind
main; the missing commit is lockfile-only, no source difference).

---

## 1. FEASIBILITY — the exact reshape

### 1.1 Current state (the prose-only legislation)

`/Users/bjorn/Documents/primer/src/tools/graph-builder/store/schema.ts`:

- **Pin** (:175-180), prose :166-174: of `FableXhigh`, `Gpt56Max`,
  `FableXhighViaIntegrity`, `Gpt56MaxViaIntegrity`, exactly ONE is mintable
  (`Gpt56Max`, owner ruling 2026-07-17 — single-lane gpt-5.6-sol); the other three are
  "frozen roster vocabulary … no code path mints them" — comment only.
- **Outcome** (:204-210), prose :181-203: `Refused` and `MismatchServed` are "frozen
  roster vocabulary with NO writer"; `Accepted`/`Rejected`/`Superseded` are written.
- **SteerKind** (:224), prose :212-223: `PinBump` is "frozen roster vocabulary with NO
  writer or reader … the driver treats any pre-ruling PinBump row as a no-op."

The three laws that today state only bare roster membership:

- `attemptPinVocab: contained(on(attempt, "pin"), on(Pin, "id"))` — schema.ts:1308
- `verdictOutcomeVocab: contained(on(verdict, "outcome"), on(Outcome, "id"))` — :1310
- `steerKindVocab: contained(on(steer, "kind"), on(SteerKind, "id"))` — :1314

Enforcement beyond prose today: **none.** `attempt.pin` is typed `Pin.id`
(schema.ts:902) — the full 4-handle union; no TS narrowing, no lint, no test asserts
frozen handles are unminted (zero test references to
FableXhigh/MismatchServed/PinBump). The prose is replicated at `seats.ts:14`,
`driver/steers.ts:16-19` and :1-38 (PinBump no-op), `driver/supervisor.ts:812-814`,
`prompts/supervisor/output-contract.ts:6-9`, and two diag-map hints
(`store/diag-map.ts:441`, :459).

### 1.2 The payload-tier declarations

SDK surface (verified in the bumbledb worktree): `closed(name, columns, axioms)` in one
call — `ts/src/closed.ts:299-303`; every handle must carry every column exactly once,
type-enforced (`Axioms`, closed.ts:97-99). Payload columns may be any field descriptor
except `fresh` (`PayloadField`, closed.ts:49); the engine refuses `str` on closed
relations (`SchemaError::StrOnClosedRelation`,
`crates/bumbledb/src/schema/validate.rs:1456-1462`) and a column named `id`
(closed.ts:399-403). So `mintable: bool` is legal; a prose `reason: str` would not be.

```ts
const Pin = closed("Pin", { mintable: bool }, {
	FableXhigh: { mintable: false },
	Gpt56Max: { mintable: true },
	FableXhighViaIntegrity: { mintable: false },
	Gpt56MaxViaIntegrity: { mintable: false }
})

const Outcome = closed("Outcome", { writable: bool }, {
	Accepted: { writable: true },
	Rejected: { writable: true },
	Refused: { writable: false },
	MismatchServed: { writable: false },
	Superseded: { writable: true }
})

const SteerKind = closed("SteerKind", { writable: bool }, {
	Reissue: { writable: true },
	Repartition: { writable: true },
	PinBump: { writable: false },
	Observe: { writable: true }
})
```

Declaration order is the axioms record's key order; handle row id = declaration index
(`crates/bumbledb-theory/src/schema.rs:321-328`) — payload columns do not renumber
rows, so keeping the orders above preserves id stability exactly.

Column naming: `mintable` is pin-language; `writable` reads better for
Outcome/SteerKind (the ruling to take). One bool per vocabulary suffices — no further
payload is warranted today. (A tempting second SteerKind column, `obeyed: bool` to
distinguish driver-obeyed levers from the `Observe` diary, would move DRIVER dispatch
policy into vocabulary — exactly the conflation §3 warns against; the driver's
obedience pass reads handle names and stays host law. Rejected.)

### 1.3 The new ψ statements (which prose they replace)

Each bare containment is REPLACED in place by its ψ-selected tightening — one
statement doing both referential integrity and the freeze (a mintable subset ⊆ roster,
so the tightened containment subsumes vocabulary membership):

| Law slot (schema.ts) | New statement | Prose it legislates |
|---|---|---|
| `attemptPinVocab` :1308 | `contained(on(attempt, "pin"), on(Pin.where({ mintable: true }), "id"))` | :166-174 "no code path mints them" |
| `verdictOutcomeVocab` :1310 | `contained(on(verdict, "outcome"), on(Outcome.where({ writable: true }), "id"))` | :181-203 "frozen roster vocabulary with NO writer" |
| `steerKindVocab` :1314 | `contained(on(steer, "kind"), on(SteerKind.where({ writable: true }), "id"))` | :212-223 PinBump "no writer or reader" |

SDK spelling verified: `where()` exists exactly when payload columns exist — absent on
the bare tier at both type and value (`Closed` conditional type, closed.ts:181-185;
runtime seam :362-368); `Kind.where({...})` returns a `SelectedClosed`
(closed.ts:150-153) which `on()` accepts (`ts/src/face.ts:94`); law-typing treats the
ψ-selected containment identically to the bare one ("a selection changes pairing not
at all", `ts/src/law.ts:34-36`).

Prior art doing EXACTLY this pattern, already pinned:
- `ts/test/cookbook.test.ts:405` — `contained(on(Certificate, "kind"), on(Kind.where({ mastered: true }), "id"))`
- `ts/test/cookbook.test.ts:448` — `contained(on(Escalation, "severity"), on(Severity.where({ pages: true }), "id"))`
  (the doc's own sub-vocabulary example, `docs/architecture/30-dependencies.md:377-380`, executed)
- Primer itself already ships a ψ-selected containment in production:
  `steerScopeSteerRef: contained(on(steerScope, "steer"), on(steer.where({ kind: "Repartition" }), "id"))`
  (schema.ts:1281-1284) — on an ordinary relation; no `.where` on a CLOSED relation
  exists in primer yet, but the SDK/engine treat both through the one selection machine.

### 1.4 Store-fingerprint consequence

The fingerprint hashes every relation's field list AND, for closed relations, the
ground axioms' handle bytes + fact bytes (`crates/bumbledb/src/schema/fingerprint.rs:63-92`),
plus every statement side's selection literals (v3, fingerprint.rs:26-35). Adding a
payload column changes both → **new fingerprint → every old store refuses to open**
(hard failure at open, fingerprint.rs:38-41). This is doctrine, not accident
(`docs/architecture/10-data-model.md:590-591`: "Closed domains are closed").

Primer's stores absorb this by construction:
- Per-run durable store at `out/graph-builder/runs/<runId>/store`
  (`store/run-store.ts:32-34`); `Db.create`/`Db.open` at :151/:163 with the wrapped
  fingerprint-mismatch error (:158-160) — never a silent migration.
- The disposability ruling is already on the books twice: the schema's FUNERAL NOTE
  (schema.ts:316-328, capability-ledger packet 2026-07-18: "run stores are per-run and
  disposable … the engine's fingerprint check refuses to open a pre-packet store") and
  lattice-cutover `prd-02-run-store-theory.md:520-527` ("fresh store, reshape legal").
- `store/rebirth.ts` is the zero-LLM carry-across-fingerprints path (exhume → create →
  copy), but the active lattice-cutover packet DELETES it ("the run store is disposable
  — new fingerprint, fresh store, regenerate", lattice-cutover `00-README.md:88-89`,
  :188-189). Rebuildability holds either way; the ruled story is regenerate, not carry.
- Rebirth hazard IF it survives: `isOrdinary` discriminates closed vs ordinary by
  `"fields" in member.data` (rebirth.ts:145-147) and treats closed targets as
  never-copied (:305-311) — a payload-tier closed relation carrying `fields` in its
  data record would be misclassified as copyable. Check the discriminator if rebirth
  outlives lattice-cutover; moot otherwise.

Note an important correction to folklore: there is NO fingerprint-stable extension —
even a bare-tier roster APPEND breaks the fingerprint. "Closed relations extend, never
reshape" (schema.ts:124-125 etc.) is about declaration-order id stability for old-ledger
decode, not fingerprint stability — and payload additions preserve id stability. A
payload-tier roster extends later exactly as a bare one does (append handle + full
axiom row); payload makes extension no harder.

### 1.5 Every primer call site that changes

**Value-level: ZERO changes.** On the 0.4.0 string-handle surface a handle IS its name
(SDK commits `22d43bc2`, `a158c7c2`, `31b265fa`; handle constants deleted `8feb4b9b`);
inserts pass string literals, reads compare strings — none of that shifts under
bare→payload:

- Inserts of `attempt.pin`: `driver/dispatch.ts:3295-3298` (`pin: "Gpt56Max"`),
  `driver/supervisor.ts:654-657` (same). Already mint only the mintable handle.
- Inserts of `verdict.outcome`: dispatch.ts:3293 (`"Superseded"`), :3329-3333
  (`"Accepted"|"Rejected"`), :2092/:2349/:2536 (`"Rejected"`), supervisor.ts:693
  (`"Rejected"`), :787 (`"Accepted"`). Only writable handles ever written.
- Inserts of `steer.kind`: supervisor.ts:768-773 via `kindIdOf()` :757-765 —
  returns only `"Reissue"|"Repartition"|"Observe"`; the supervisor output contract
  (`prompts/supervisor/output-contract.ts:36-52`) has no pinBump arm.
- Reads (unchanged): `driver/steers.ts:155,160,163,540`; `driver/mint.ts:503,517,641,
  672-702`; `driver/view.ts:471`; `supervisor.ts:339,481,484,613-614,874`;
  `dispatch.ts:2090,2347,2534`; `store/gates.ts:1227,1356`; `store/observe.ts:1229`;
  `prompts/supervisor/task-prompt.tsx:160`. Tests (~48 handle literals) all use
  writable handles — unchanged.

**What actually changes:**
1. schema.ts — the three `closed()` declarations (§1.2) and the three law slots (§1.3);
   the statements TUPLE (:1546-1650) is unchanged in shape (in-place replacement, same
   slots, same names or renamed at taste). Prose comments :166-223 shrink to
   provenance notes ("mintable:false since owner ruling 2026-07-17") — the law moves
   into the table.
2. `store/diag-map.ts` — the map is keyed by statement OBJECT identity
   (diag-map.ts:35) with load-time forward exhaustiveness (any statement in
   `runStoreSchema.statements` without a `register()` throws, :502-508). The three
   registrations (:438-442, :444-448, :456-460) reference `laws.attemptPinVocab` etc.,
   so in-place law replacement re-points them automatically; only the repair-hint TEXT
   updates ("pin is not a mintable handle" instead of "pin is out of vocabulary").
   **No DiagKind roster change, no new handles, no diag-map row additions.**
3. `schema.test.ts` tuple-completeness pin — unchanged (identity against the `laws`
   record, which still has three entries in the same slots).
4. Optional parity: the SDK's own primer-derived fixture
   `bumbledb/ts/test/fixtures/run-store-schema.ts:48-52` still carries the bare-tier
   trio; mirror the reshape there if fixture parity is wanted (SDK-repo change, cosmetic).

---

## 2. DECIDABILITY / ENGINE — zero engine work: VERIFIED

- **ψ fold + member set exist and are exercised.** A closed target has no key search:
  "the enforcement plan is the answer set itself" — `resolve_target_key`'s
  compiled-subset branch, `crates/bumbledb/src/schema/validate.rs:1176-1187`;
  `compile_member_set` folds ψ over the sealed extension at validate into a 256-bit
  `MemberSet` (validate.rs:1337-1350). Commit-time enforcement is one
  `members.contains(axiom_index)` per inserted source fact
  (`crates/bumbledb/src/storage/commit/judgment.rs:344-357`) — O(1), zero oracle
  consultations (Lean theorem `member_test_decides`, `lean/Bumbledb/Oracle.lean`).
  Closed targets are never deleted (writes refused), so no target-side re-judgment
  path is needed (judgment.rs:715, plan.rs:441). The architecture doc names this exact
  pattern as the intended idiom (`30-dependencies.md:377-380`, sub-vocabularies).
- **Since 0.3.0: verified.** Commit `3002259a` "the payload-tier closed value mints
  where() — hole A of psi-selection closes" sits in the 0.3.0 release train (before
  the 0.3.0 publish commits `dcbad3c0`/`6dc9b211`); the payload-tier runtime carrier
  landed the same train (`4fc913d5`). Primer is on 0.4.0 — the surface is available.
- **256-row cap absorbs it.** `MAX_EXTENSION_ROWS = 256`
  (`crates/bumbledb-theory/src/schema.rs:315-319`) caps HANDLES (the 4×u64 bitset
  width), not columns; Pin has 4, Outcome 5, SteerKind 4. Column count is separately
  capped per-relation (validate.rs:1352+) — one bool is nothing.
- **Fingerprint story absorbs it.** Payload columns + axiom fact bytes + selection
  literals are all fingerprinted (§1.4); ψ-selected closed targets round-trip through
  create/reopen in `ts/test/fingerprint.test.ts:119`. Fuzz already draws closed axis
  relations with bool/tag selections in containment sides (`fuzz/src/theorygen.rs:28-32,
  132, 288-293`) and seeds hit the 255/256 boundary (`fuzz/src/seeds.rs:17`).
- **Acceptance gate satisfied trivially.** For a closed target the projection must be
  exactly the synthetic id (`FieldId(0)`, validate.rs:1176-1183); a ψ selection does
  not disturb that. Law-typing: compile error on a wrong-typed selection literal
  (`ts/test/law-typing.test.ts:103`).

---

## 3. GOAL ALIGNMENT

**For.** This is the project's core move executed on its own residue: three blocks of
"legislated only in prose" become three Lane-1 statements, and a rogue writer (a
future refactor reintroducing a Fable lane insert, a bad settle arm writing `Refused`,
a resurrected PinBump emitter) becomes UNWRITABLE at commit instead of
grep-and-hope. The engine docs' intrinsic-vs-policy law (`10-data-model.md`, the
passage ending :590-591) says intrinsic properties of a sealed vocabulary belong ON
the closed relation, with exactly this rebuild cost — mintability under a standing
owner ruling is such a property: which handles are LIVE in this era of the theory is a
fact about the vocabulary, recorded as data, flipped only by a new ruling + new
fingerprint + fresh store. That's auditable law, not policy smuggling.

**Counter-arguments examined:**

1. *"Mintability is policy, not meaning — a window/cardinality law instead?"* The
   dependency calculus offers exactly one rival spelling: the exclusion window
   `window(on(Pin.where({ mintable: false }), "id"), none, on(attempt, "pin"))`
   (`none` = the {0} exact window, `ts/src/count.ts:27-28`; closed-parent window path
   exists, judgment.rs:989-1006). It is strictly weaker: it bans frozen pins without
   REQUIRING roster membership (the bare containment must stay alongside — two
   statements for one meaning), and costs a per-parent child-group walk vs the
   containment's O(1) membership test. The vocabulary's own `{1..*}` ban ("the
   containment respelled", `30-dependencies.md:389-397`) is the calculus saying: when
   the meaning is a containment, spell the containment. ψ-containment is the right
   spelling.
2. *"Just delete the dead handles"* (the funeral-note rival — precedent at
   schema.ts:316-328 and lattice-cutover prd-02 §6, both legalizing roster reshape on
   disposable stores). This achieves unwritability with no payload tier at all:
   Pin → {Gpt56Max}, Outcome → {Accepted, Rejected, Superseded},
   SteerKind → {Reissue, Repartition, Observe}. It is leaner, and since ANY theory
   change already breaks the fingerprint, "kept so pre-ruling ledgers still decode" is
   operationally moot today. The case for the flag over the funeral: (a) retirement
   becomes a one-bool flip with full era history in the theory itself — the vocabulary
   remembers what existed and that it is dead, vs deletion erasing the fable-first era
   from the store's self-description; (b) a lane returning (or refusals settling
   attempts someday) is a flip, not a delete/re-append churn; (c) it installs the
   reusable sub-vocabulary idiom the docs already teach. This is a ruling for the
   owner, not a technical fork — both are legal, both cost one rebuild.
3. *"Enforce host-side with TS narrowing instead?"* A `MintablePin` type alias would be
   compile-time only — documentation-tier again; the store would not refuse a rogue
   writer. The reshape does not narrow `Infer<typeof Pin.id>` (the column type stays
   the 4-union; the LAW narrows the writable set), so an optional host alias remains
   available as belt-and-braces but is not the enforcement.

---

## 4. COST

- **Primer:** one file's three declarations + three law slots (schema.ts); diag-map
  hint text only (registrations follow statement object identity automatically —
  §1.5); comments shrink; zero value-level call-site changes; zero test-literal
  changes; no DiagKind change; store rebuild = start the next run (per-run disposable;
  no ETL exists for run stores). Estimated diff: well under 100 lines, one commit.
- **Sequencing (the real cost):** the locked `lattice-cutover-exec` worktree is
  rewriting this exact file heavily (TaskKind/DiagKind reshaped, capability → u64
  refs) and its PRD-02 explicitly defers this reshape (:441-445 "not this packet's
  business"). Land AFTER lattice-cutover merges — on top of the new schema, where
  rebirth is already deleted and the disposability ruling already cited. Landing
  before/concurrently buys a guaranteed conflict in schema.ts and diag-map.ts.
- **SDK/engine: zero.** Verified end to end (§2). Optional cosmetic: mirror the
  reshape in `ts/test/fixtures/run-store-schema.ts:48-52`.
- **Rebirth fallout: none expected** (deleted by lattice-cutover); if it survives,
  audit the `isOrdinary` discriminator (rebirth.ts:145-147) against payload-tier
  closed data records first.

---

## 5. VERDICT MATERIAL

**RECOMMEND**, sequenced after lattice-cutover lands. The mechanism is fully built,
tested, doctrine-blessed, and already half-adopted by primer (ψ-selected containment
in production at schema.ts:1281-1284; only the closed-relation `.where` is new to it).
The reshape converts three prose freezes into three one-line laws with zero engine
work, zero call-site churn, and one disposable-store rebuild. The single open RULING
is flag-vs-funeral (§3.2): mark the dead handles `mintable/writable: false` (recommended —
reusable idiom, era history preserved as data, retirement becomes a bool flip) or
delete them outright under the existing funeral precedent (leaner, equally legal).
Defer only the timing, not the substance.
