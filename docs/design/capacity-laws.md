# Capacity laws: the cardinality window dies into the aggregate containment

STATUS: LANDED (2026-07-25) — the campaign executed whole and ships as 0.8.0:
design + rulings `4edd773a..355a2ed2`, spec flush `dc272208..caefe16c`, code +
proofs + gate `b2584bcb..df7c25e2` (2026-07-24/25). §8 rulings 1–6 and §8b
C1–C19 landed as ruled, with ONE open tail: C17's slot-vs-fetch measured
choice rides the bench phase, which the owner deferred at close (2026-07-24)
— both `measure_children` arms stand behind the one `CAPACITY_WEIGHT_SLOT`
constant (`crates/bumbledb/src/storage/commit/judgment.rs`), fetch-per-child
ships as the baseline, and the winner-lands/loser-deletes measurement is owed
on bench resume (`TODO.md`; the §9 step-4 bench re-pins and the two weighted
lanes' numbers are owed with it — no capacity perf number is claimed
anywhere). The §8b zero-trace gate ran GREEN at close over the full scope.
Ground-truth deltas from execution are stamped on the companion dossier
(`capacity-cutover.md`). Drafted 2026-07-24 from the weighted-capacity
discussion. This document specifies a deletion and a generalization: the `<={lo..hi}`
cardinality-window mechanism is removed root and branch, and schema capacity laws are
restated as **aggregate containments** — the query aggregate vocabulary appearing in law
position, folded per target group, bounded by a window. Counting becomes the unit-weight
corollary it always was: `length = sum ∘ map(const 1)`.

Owner's framing, recorded: the window operator is "the arbitrary cardinality hack in our
macro language" — the notation is precisely the thing being replaced. No desugaring, no
grandfathering, no preserved twin. Per the standing maximal-churn/maximal-elegance
policy, every spelling migrates and the old mechanism leaves no residue.

## 1. Why this is a theorem, not a refactor

The repo has already half-proved the unification ladder, one rung at a time:

- keyed `==` is exactly the `{1}` window — `keyed_eq_unit_window`, `unit_window_containsEq`
  (30-dependencies.md § cardinality window).
- a floored window implies the reverse containment — `window_floor_containment`, which is
  why `{1..*}` is banned today as a respelling of `<=`.
- the count window bounds `|group|` — and `|group| = Σ 1` over the group.

So the ladder reads: keyed-eq ⊂ containment ⊂ count window ⊂ **weighted window**. Each
rung was discovered separately and got its own mechanism; the top rung subsumes the count
mechanism entirely. **Where the unification stops, and why:** existence obligations remain
containment's alone. The current doctrine "windows never manufacture parents" survives
verbatim — a capacity law is satisfaction-only over selected targets, never an existence
claim. Containment and functionality statements are untouched by this design.

The judgment-theory grounding (owner's preservation-under-extension analysis, 2026-07-24):
a capacity law is not preserved under extension — it is the class of law that *cannot* be
checked once. The industry answers are an incrementally-maintained ledger (drifts: only
the operations you wrote maintenance rules for are sound) or a full re-derivation (can't
drift, can't scale). bumbledb's commit judgment already sits at the third point:
**delta-scoped re-derivation** — no stored ledger (no second copy of the truth, deletes
need no special rule), no full rescan (only touched groups re-derive). The weighted
generalization inherits this judgment shape unchanged, because non-negative weights keep
the polarity argument intact: an upper bound is newly violable only by inserts into the
counted side, a floor only by deletes. The preservation analysis is the scheduler.

## 2. The statement

One schema statement — the **capacity statement** — replacing `CardinalityStatement`:

```
capacity := TARGET '<=' WEIGHT? WINDOW SOURCE          -- target-left, the B-family order

WEIGHT   := '[' field ']'              -- u64-encoded field of SOURCE — the measure
          | '[' Duration(field) ']'    -- interval-measure weight (R5)
          |                            -- absent: unit weight — the count instance
WINDOW   := { BOUND } | { BOUND .. BOUND } | { BOUND .. * }
BOUND    := int-literal                -- the fixed capacity
          | field                      -- DEPENDENT BOUND: a u64 field of TARGET's row
          | Duration(field)           -- dependent interval-measure bound of TARGET
TARGET   := Relation(proj... | ψ)      -- the grouping side; proj resolves a key of TARGET
SOURCE   := Relation(proj... | φ)      -- the weighed side, partitioned by TARGET's keys
```

The operator algebra, completed (owner correction 2026-07-24: the dependency-theoretic
operator style is the notation — no `of`/`in`/`per` keyword prose; the windowed
containment is a numerical dependency and the weighted form completes the family):

```
B(Y | ψ) ==            A(X | φ)     -- the {1} window            keyed_eq_unit_window
B(Y | ψ) <=            A(X | φ)     -- existence                 window_floor_containment
B(Y | ψ) <={lo..hi}    A(X | φ)     -- unit weight (count) — utterance UNCHANGED
B(Y | ψ) <=[w]{lo..hi} A(X | φ)     -- weighted capacity
```

Every rung is the same partition law — the target's keys partition the source, and the
operator bounds each class's measure. No aggregate name appears in the notation: with the
law-position roster fixed at Count/Sum, the fold is always Σweight and Count is the
unweighted operator. The `{lo..hi}` count spelling survives character-for-character — not
as a grandfathered form, but because it was always the unit-weight utterance of the
general operator; everything count-only *beneath* it (the five TS constructors, the
count-only IR/judge/Lean) still dies per § 7.
```

Rust text notation — the ruled spellings:

```
Holder(id)       <={0..3}                                Account(holder)   -- count, unchanged
Cabinet(id)      <={0..4}                                Drive(cabinet)    -- drives in a container
Pool(id, supply) <=[watts]{0..supply}                    Device(pool)      -- power budget, dependent bound
Room(id, span)   <=[Duration(booked)]{0..Duration(span)} Booking(room)     -- calendar capacity
```

TS surface mirrors the operator positionally (target, weight?, window, source):

```ts
capacity(on(Holder, "id"),            within(0n, 3n),          on(Account, "holder"))
capacity(on(Pool, "id", "supply"),    weigh(f(Device, "watts")), within(0n, ref("supply")), on(Device, "pool"))
```

The grammar's net delta is small and *negative in count-specific surface*: `parse_window`
generalizes (an optional `[weight]` between `<=` and the brace; bounds admit target-
projection idents), while the count-only estate — the `window(target, count, source)` TS
constructor and all five `count.ts` constructors with their two-tier ban machinery
(bans move into the one capacity builder) — is deleted. The `in lo..hi per` tombstone
(`window_in_per_is_deleted.rs`) stays exactly as it is: keyword prose stays dead; the
operator was always the notation.

## 3. Semantics

Per ψ-selected target fact `g`, over the group of φ-selected source facts whose projected
tuple equals `g`'s key tuple (exactly today's `ChildGroup`):

```
measure(group) ∈ window        where measure = Σ weight(row) over the group
                                     weight  = 1 (Count) | row.field (Sum)
```

- **Set semantics**: the group is the deduplicated fact set (today's `dedupFacts`
  enumeration, `Decide.childGroup_enum`); weights are summed over distinct facts.
- **Both window ends inclusive**; `*` remains the only spelling of "no upper bound".
- **The `{0}` exclusion keeps its footnote** — denial-flavored but satisfaction-only, same
  touched-parent plan. For `Sum`, `{0}` means "the group's total is zero", which admits
  zero-weight rows: a *different, weaker* law than `Count in {0}`. Stated loudly (§ 6).
- **Acceptance gate unchanged**: TARGET's projection resolves a declared key (ScalarProbe)
  or the closed member-set (Closed) — `resolve_target_key` survives as-is, as does the
  both-sides-closed decidable refutation (validate.rs:822, generalized from count to sum).
- **Windows never manufacture parents** — no holder / ψ-miss ⇒ satisfied, verbatim today's
  check_window behavior (judgment.rs:1000–1060).

### Weight typing (representation does the enforcement)

- `Sum(field)`: the field must be a **u64-encoded** position of SOURCE. Signed encodings
  are a typed refusal — a negative weight would break the polarity scheduler (an insert
  could lower a sum), so the illegal weight is unrepresentable, not checked.
- `Sum(Duration(field))`: the interval measure as weight, u64 by construction (R5
  machinery). This is the free feature: **calendar capacity** — "total booked time per
  room within the window" — as one schema statement.
- The v0 interval-position refusal (`CardinalityIntervalPosition`) survives *for
  projections*: intervals enter through the measure argument, never the group key.

### Where the weight ladder tops out: composition, not paths (ruled 2026-07-24)

Unit and column weights are two instances of the general object — a functional term over
the source row — and the next instance is a term that walks a reference (the weight lives
on a catalog relation: `Device.model → Model.watts`). That use case is real and, in
normalized stores, the default. It is supported **by composition** in the existing
algebra, not by admitting paths into the bracket:

```
Device(model, watts) <=                    Model(id, watts)   -- watts pinned to the catalog
Pool(id, supply)     <=[watts]{0..supply}  Device(pool)       -- capacity reads the local column
```

The two-column containment IS the join, stated as a law: a device's watts provably equals
its model's, at every commit, through the judge that already exists. Path weights
(`[model.watts]`) are a typed refusal whose diagnostic names this idiom, because the deep
form is wrong three ways: the index-slot weight would become a cached copy of another
relation's field (the maintained-ledger drift class, resurrected); a `Model.watts` update
would need a new reverse-adjacency walk re-judging every transitively affected group (a
query evaluator growing inside the judge); and the live semantics is worse — catalog
edits silently re-weigh deployed fleets, where the pinned form refuses the inconsistent
commit at the right site and makes the migration explicit. The weight vocabulary is
closed: `[field]`, `[Duration(field)]`, absent. Everything further factors through
statements composing — the algebra is the join language for laws.

### Overflow

The engine accumulates in **u128**: `2^64` max weight × any realistic group cardinality
cannot approach `2^128`, so overflow is unrepresentable in practice and no refusal
variant is needed. The Lean denotation states the sum in unbounded ℕ (`natSum`), with
`natSum_le_length_mul` as the bound lemma tying the u128 claim down. (The query-side
`checkedSum`/`Overflow(Aggregate)` machinery is for query folds with u64 answer cells;
the judge's verdict is a comparison, not an answer cell, so the wide accumulator is the
simpler sound choice.)

## 4. Judgment: what generalizes, what it costs

**Plan phase — unchanged.** `touched_parents` superset narrowing (plan.rs:401 `mark_ops`,
φ-blind source half, ψ-gated target half) is already weight-agnostic: it marks *groups*,
not counts. `WindowCheck { window, parent }` renames to the capacity check; the flattening
and the delta-restriction theorem shape survive.

**Verdict phase — the fold generalizes, and the clipped walk survives.** Today's
`count_children` (judgment.rs:1082) has two arms:

- **Closed source**: the honest ≤256-row extension scan — becomes `Σ weight(row)` over
  φ-survivors. Trivial.
- **Keyed source**: the ordered R-bucket prefix walk, clipped at `decided_at` — today it
  counts *keys without reading values*. A weighted walk must see each child's weight.
  Two options (§ 8.2 ruling): **(a)** fetch each child fact (one descent per child — turns
  the cheap walk into k probes); **(b)** the reverse-index entry for capacity-source
  relations carries the weight in its value slot — pay one u64 at write time, judge reads
  the walk it already does. (b) is the representation-first move and the recommendation;
  it implies an index-format arm for capacity-weighted relations and a format-version bump.
  The clip generalizes cleanly under non-negative weights: the running sum is monotone, so
  an upper-bound walk exits the moment `sum > hi`, and a floor walk exits at `sum ≥ lo` —
  same early-exit soundness the count walk has today, same worst case (the whole bucket)
  only when the verdict is genuinely close.

**Polarity refinement (optional, recorded not required):** today both delta halves mark
touched parents regardless of window shape. Upper-bound-only windows are insert-violable
and floor-only windows delete-violable, so `mark_ops` could skip half the marks by window
polarity. The superset is sound; the refinement is a measured-choice follow-up, not part
of this design's correctness story.

**Violation shape:** `Violation::Cardinality { statement, fact, count }` becomes
`Violation::Capacity { statement, fact, measure }` — the witnessed group total, which for
Count *is* the count. One violation, one display arm.

## 5. The Lean restatement

The current model is deliberately count-shaped at the denotation layer:
`Cardinality.lean` states windows over list-witnessed set bounds (`Set.AtLeast/AtMost`) —
no number is ever materialized; `Decide.childCountB`/`Oracle.window_admits_iff_enum` are
where sets meet lengths. **A weighted law is irreducibly numeric, so the layering
inverts:**

- The denotation becomes a fold: `groupMeasure (w : Weight) (s) : ℕ` defined over any
  Nodup enumeration of the group, with the permutation-invariance lemma (sums commute)
  playing the role `window_admits_iff_enum` plays today — "one walk decides a window"
  survives with `sum` in place of `length`.
- `Statement.cardinality (source, window, target)` → `Statement.capacity (agg, source,
  window, target)`; `CapacityLaw` replaces `CardinalityWindow` with
  `window.admitsMeasure (groupMeasure w (ChildGroup A φ X (g.project Y)))`.
- **The count theorems restate as unit-weight corollaries, no preserved twin**:
  `cardinality_zero_star` → `capacity_zero_star` (vacuity is weight-independent);
  `cardinality_of_empty_parent`, `cardinality_window_mono` (widening) generalize verbatim;
  `window_point_admits_iff`'s exact-count reading becomes the `weight = 1` instance.
  The witness-style `Set.AtLeast/AtMost` primitives survive only as the lemmas backing
  the unit case.
- `Decide.cardinalityB` → `capacityB` (the fold over `dedupFacts`, `natSum` of weights),
  with `capacityB_iff` re-proved; `Oracle.cardinality_plan_decides` → the key new proof
  obligation, `capacity_plan_decides`: the touched-parent probe + weighted group walk
  decides the delta restriction. Existing machinery reused: `checkedSum`/`natSum` lemmas,
  the `dedupFacts` Nodup enumeration, `childGroup_enum`.
- Ladder theorems recast: `keyed_eq_unit_window` and `window_floor_containment` restate
  against `Count`-instance capacity laws — they are the rungs that justify the bans
  (§ 6) and they keep their names in the doc trail.

Corpus: the 9 dedicated `judgment-window-*` cases + 4 mixed cases re-baseline under the
new statement encoding, and the corpus gains the weighted rows: sum-pass, sum-exceed,
zero-weight-under-floor, duration-weight, dependent-bound (if § 8.3 rules it in),
closed-extension-sum-refuted.

## 6. The canonical-utterance law, generalized (and where it becomes weight-sensitive)

The window vocabulary and its ban table (70-api.md:141–169) survive — but three rows of
the table were secretly *count* facts, not *window* facts, and the general law must split
them by aggregate:

| spelling | Count | Sum |
|---|---|---|
| `{1..*}` | **banned** — containment respelled (`window_floor_containment`) | **legal** — "the group's total is positive" is not an existence claim over rows |
| `{0..*}` | banned — vacuous | banned — vacuous (sums are ≥ 0; weight-independent) |
| `{0}` | the exclusion: no φ-child exists | legal, *weaker*: total is zero — zero-weight rows may exist. The doc states this beside the exclusion footnote. |
| `{n..n}`, `{0..0}`, inverted, open shorthands | banned → canonical form | banned → canonical form (weight-independent) |

The general statement of the law: **a ban is canonical-utterance policing when it is
weight-independent, and semantic deduplication when it is not; the second kind applies
per-aggregate.** `SpecIssue::WindowContainmentRespelled` therefore fires only on the
Count instance; everything else in the ban table survives unchanged.

Lower-bound footgun, stated loudly (the doc owes this the way 20-query-ir owes the
join-multiplicity warning): `Count in {1..*}`-shaped intent ("at least one child") is
containment; `Sum(w) in {1..*}` ("positive total") admits any number of zero-weight rows
and is satisfied by one row of weight 1. They are different laws. Choose by what you mean.

## 7. The deletion inventory (exact, from the 2026-07-24 estate maps)

**Grammar/surface:**
- bumbledb-macros lib.rs: the `<`/`=`/brace-group dispatch (:868), `parse_window` + arms
  (:968–994), `parse_window_bound` (:954), `Statement::Cardinality` (:297), lowering
  (:1529), descriptor codegen (:1987–1999), ban Display (:1764–1786), the in-per tombstone.
- ts: `count.ts` whole file (five constructors, `BannedWindow` type tier, runtime tier,
  `admitted` brand); `statements.ts` `window()` (:278–293) + renderer arm (:314); the
  `WindowSpec` union in `spec.ts` (:93–96).
- Renderer: `${target} <=${window} ${source}` form (schema/render.rs) → the capacity form.

**IR/engine:**
- `CardinalityStatement` (schema.rs:471), `WindowId`, `StatementRef/View::Cardinality`,
  `Schema.windows`, `Relation.window_sources/targets` → capacity equivalents.
- theory: `WindowSpec` 3-kind enum (spec.rs:151), `StatementSpec::Cardinality` (:183),
  `StatementDescriptor::Cardinality` (schema.rs:327), the five window `SpecIssue`s (:335).
- validate: `validate_cardinality` (validate.rs:766) → `validate_capacity` (weight typing
  + generalized ban table + same acceptance/closed-refutation arms); sealing loop
  (:197–237).
- judge: `check_window`/`count_children` (judgment.rs:993/:1082) → capacity verdict +
  weighted fold; `window_child_image` survives renamed (group keying is weight-blind).
- errors: 4 window `StatementErrorKind`s + `Violation::Cardinality` → capacity set.
- FFI: tags `EXACT/RANGE/FLOOR` + `CARDINALITY` arm (tags.rs:179–201, marshal.rs:504–547)
  → capacity statement shape (window kinds survive; the statement gains the agg field).
- fingerprint: statement-form tag `cardinality window = 2` (10-data-model.md:588) — the
  encoding changes, so **schema fingerprints move**; this rides a format-version bump with
  the § 4 index arm. (The 0.6.0 "fingerprints unmoved" note was that release's fact, not
  a standing law.)

**Docs (census-tracked):** 30-dependencies.md:196–222 + :323/:387–402/:645 rewritten;
70-api.md:119–169 + :298 (the law's home moves to the aggregate form); 50-storage.md:367;
10-data-model.md:588; architecture README table row. ~14 Lean citation strings re-pin to
the new symbols.

**Migrations:** ~40 Rust spelling sites (tests + bench: schema_macro 6, schema_spec 5,
render 6, translate/builder 6, windowed 4, judgment tests 6, lawful 3, misc); 10
compile-fail files re-derived against the new grammar's refusals; ~84 TS test usage lines
across 8 files; 13 conformance corpus cases re-baselined; `lawful` bench lane re-pins
(its law mix includes windows).

**Free wins riding the deletion:** `atMost` no longer folds into `range{lo:0}` and `none`
into `exact 0` — the wire vocabulary stops encoding one thing two ways; and finding 109's
already-unified side-pair gate serves both remaining statement families with no
copy-paste left.

## 8. Rulings — RESOLVED (owner, 2026-07-24: "aggressively churning, zero backwards
compat, hard deletion of all of the cardinality logic")

1. **The spelling IS the operator, completed** (owner re-ruled 2026-07-24: the first
   resolution's `of/in/per` keyword form was a dependency-theoretic style regression and
   is dead). `Target <=[weight]{window} Source`, target-left B-family preserved; absent
   weight = unit = count, so the existing `<={lo..hi}` utterance survives unchanged as
   the unit instance. No aggregate names in law position — the fold is always Σweight.
   TS: one positional `capacity(...)` builder mirroring the operator.
2. **The weight lives in the reverse-index value slot.** One u64 paid at write time; the
   judge's walk stays exactly one range scan. The index gains the capacity-weighted arm;
   the format version bumps. Per-child fact fetches are not a fallback — they don't exist.
3. **Dependent bounds are in.** `{0..supply}` reads the bound from the target row —
   per-group capacity is the point of the feature, not an extension to it. Literal bounds
   are the degenerate constant case of the same BOUND production.
4. **Law-position roster: `Count` and `Sum` (field / Duration).** The polarity-clean folds.
   `Min/Max` windows: typed refusal with the recorded trigger in the feature register.
5. **The name is capacity.** `Statement.capacity`, `Violation::Capacity`, `Capacity.lean`.
   The word "cardinality" survives nowhere in the mechanism — it was the unit-weight
   instance naming the whole, and the audit's own doctrine applies: the special case does
   not get to keep the family name.
6. **The weight vocabulary is closed at the row: `[field]` / `[Duration(field)]` / absent.**
   Joined weights are supported by composition (§ 3, the pinned-column idiom) and the path
   spelling `[a.b]` is a typed refusal whose diagnostic names the idiom. No recorded
   trigger — this is a boundary, not a deferral: admitting terms into the bracket grows a
   query evaluator inside the judge and reopens the cached-truth drift class.

## 8b. Cutover rulings C1–C19 — RESOLVED by doctrine (owner approved the whole hard
cutover 2026-07-24; each blocker below resolves from rulings already made)

- **C1 (bound spelling):** bound idents resolve by NAME against the target's full roster;
  the written projection tuple stays the pure grouping key: `Pool(id) <=[watts]{0..supply}
  Device(pool)`. The design §2 examples are errata'd to this form.
- **C2 (field order):** the statement reads as the operator does — target, weight, window,
  source — in the Lean constructor, corpus JSON, FFI marshal, descriptor codec, and
  fingerprint encoding alike.
- **C3 (measure width):** the accumulator is u128 and the witnessed measure crosses whole:
  `Violation::Capacity { measure: u128 }`, BigInt on the TS wire, ℕ in Lean. Truncation is
  unrepresentable.
- **C4 (weight shape):** `Weight` is a total sum — `Unit | Field(FieldId) |
  DurationOf(FieldId)`. No Option; Unit is a case, not an absence.
- **C5 (fingerprint tag):** capacity mints tag **4**. Tag 2 retires with the mechanism;
  tag 3 stays retired (order marks) — the never-reissue law governs.
- **C6 (dependent-bound slot):** dependent bounds are **hi-slot only** — inversion with
  idents becomes unrepresentable at parse. A dependent floor has no use case; refusal
  names the ruling.
- **C7 (Lean totality):** Capacity.lean states the witness-style pair
  (`MeasureAtLeast`/`MeasureAtMost`) — the no-finiteness-token law stands; the numeric
  fold lives at the Decide/Oracle enumeration boundary, as today.
- **C8 (corpus names):** zero traces — `judgment-window-*` re-keys to
  `judgment-capacity-*`; Bridge instrument tokens, corpus README, and 60-validation move
  in the same commit.
- **C9 (validation roster):** the exhaustive roster gains named rows: signed/non-u64
  weight field; path weight (names the composition idiom); bound ident not on target;
  bound field not u64/Duration-capable; Duration weight/bound over a non-interval field;
  dimension mixing (count vs Duration bound — C18). Closed target × dependent bound:
  bounds resolve per ground-axiom row at seal time; the closed-refutation arm judges each
  axiom row against its own resolved bound.
- **C10 (rays):** a ray-valued Duration weight or bound at judge time is a typed commit
  refusal naming the row — the R6 precedent (a ray has no finite measure), enforced at
  the law site.
- **C11 (Admission):** the capacity verdict quantifies over the witnessed false-surface
  parents (bounded quantification); `AdmissibleForm` generalizes only if the Lean lane
  finds the bounded form unprovable — report, don't improvise.
- **C12 (clip soundness):** the clipped walk gets its named lemma (prefix monotonicity of
  non-negative sums) and a Bridge row; the §4 claim will be cited, not asserted.
- **C13 (generator coverage):** capacity enters the querygen/theorygen ledger day one;
  every corpus digest moves — a deliberate act under the hard cutover. Finding-025's law:
  nothing ships unspellable by the generator.
- **C14 (measure parity):** both differential twins widen to carry the witnessed measure;
  on conviction the judge completes the full walk so the reported measure is
  walk-order-independent (the clip serves the verdict, the full sum serves the witness).
- **C15 (calendar lane):** fresh twin world; the existing calendar corpus digests stand.
- **C16 (names):** `capacity_plan_decides`, `capacity_plan_consultations`. The word
  "window" survives exactly where it names the `{lo..hi}` object (which survives);
  Countermodels/Subsumption symbols keep their names.
- **C17 (slot scope + the measured choice):** the R value-slot is **statement-scoped**,
  and per the global-maximum review, slot-vs-fetch-per-child is a **measured choice**:
  the campaign implements fetch-per-child as the baseline, benches both on the
  power-budget lane, and lands the winner with the number recorded in the code.
- **C18 (dimensions):** Duration weights pair with Duration-capable bounds; a count
  window with a Duration bound is a typed validation refusal.
- **C19 (the next rung, recorded):** the balance-law trigger enters the feature register:
  aggregate-vs-aggregate windows (`Sum(debits) == Sum(credits) per Transaction`) are the
  known next generalization, deliberately unbuilt; the trigger is a real host asking for
  a balance constraint.

**The zero-trace gate:** at campaign close, `rg -i cardinal` over crates/, ts/src, ts/crate,
ts/test, lean/, docs/architecture/, docs/research/, scripts/ returns zero hits. Historical
records (docs/design/, audit-2026-07/, bench-out/, git history) are exempt as records.

## 9. Sequencing

Same shape as the audit campaign, deliberately smaller: **(1)** rulings § 8 → **(2)** spec
flush (30-dependencies + 70-api rewritten around the aggregate form, 10-data-model tag,
Capacity.lean stated with obligations named) → **(3)** one code+proofs campaign (grammar,
TS, engine, FFI, Lean discharge, corpus re-baseline, spelling migrations, format bump) →
**(4)** bench: `windowed.rs`/`lawful` re-pins plus a new weighted lane (the calendar-
capacity shape), then the ledger entry and the 0.8.0 release ride the normal process.
