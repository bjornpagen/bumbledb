# 60 — Containment target-key parity: one contract, every boundary, names in every refusal

The TypeScript boundary accepts — and `lower()` emits — a `contained`
statement whose target projection resolves no declared key; the native
engine rejects the same schema at `Db.create()`:

```text
target relation 5 projection {2} matches no declared key;
available keys: key 0 {0}; key 40 {1, 2}
```

Primer hit it stating `Coverage[sourceAddress] ⊆
AnalysisTargetEntry[sourceAddress]` against a relation whose keys are
`{entry}` and `{policyPackage, sourceAddress}`. Two admission boundaries,
one law, two answers (V8) — and the answer that finally lands speaks ids.

## The contract ruling (the report demands one; this is it)

**A containment's target projection must resolve a declared key of the
target relation. Everywhere.** The engine's rule is not an implementation
accident to be relaxed; it is architected and priced:

- The IND rule: "the target projection Y
  must be a permutation of some declared key of B", with the Lean theorem
  (`lean/Bumbledb/Oracle.lean: accepted_target_key_prices_the_probe`).
- The enforcement machinery is keyed by the resolved key: source-side `R`
  reverse edges and target-side `U` determinant probes
  (`resolve_target_key` → `Enforcement::ScalarProbe`/`IntervalCoverage`,
  `crates/bumbledb/src/schema/validate.rs`), and the delete-side
  re-establishment judgment relies on the key's uniqueness. General
  inclusion dependencies into non-key projections would require a new
  index class, a per-tuple survivor count, and new theory — that is
  *essential* complexity of a different feature, not accidental complexity
  of this one ([00-doctrine.md](00-doctrine.md), the limit). Refused.
- The exact match rule, mirrored precisely by every boundary below:
  **set equality** of the target projection's field set against some
  declared functionality's field set (`matching_functionality` compares
  `FieldSet`s — permutations resolve, subsets and supersets do not), with
  the closed-relation special form: a **closed** target's projection must
  be exactly the synthetic `id` (`ClosedTargetNotHandle` — its own rule,
  closedness, not key absence). `mirrors` materializes as two
  containments, so **both** faces must resolve keys of their own
  relations. `capacity` reuses the target rule verbatim
  (`resolve_capacity_target`).

Explicitly refused, in the report's own words: **do not silently
synthesize a target key** (a key is a semantic FD the user must state —
inventing one changes the theory behind the user's back) and **do not
require a false or globally overstrong FD** (the refusal names the missing
key; whether to declare it is the caller's judgment — Primer chose a
stage-local law instead, correctly).

## One law at every boundary

Two *tiers* of one wall is the house pattern (the class laws already work
this way: `ClassWall` at the type tier, `computeClasses` at the value
tier, the engine as final authority). This document extends exactly that
pattern to the target-key law.

### 1. Value tier — `schema()` (authoritative, always on)

After the existing per-statement walls, for every `containment`,
`mirrors`, and `capacity` statement:

- Build the target relation's key roster from what `schema()` already
  holds: every declared `key()` statement in this list, plus the
  fresh-implied keys, plus the closed auto-key — the same population
  `collectImplied` walks today (extended to return projections, not just
  rendered strings).
- Judge: closed target ⇒ projection is exactly `["id"]`; ordinary target
  ⇒ the projection's field-name set equals some roster member's set.
  `mirrors` judges both orientations; `capacity` judges its target face.
- Refuse with the canonical utterance, **names throughout**, the engine's
  shape with the engine's hint:

  ```text
  schema Analysis: Coverage(sourceAddress) <= AnalysisTargetEntry(sourceAddress):
  target projection (sourceAddress) matches no declared key of AnalysisTargetEntry —
  available keys: (entry); (policyPackage, sourceAddress)
  ```

  plus, when the projection carries an interval position, the engine's
  pointwise hint ("declare the exact pointwise key …") in the same words.

Soundness bar, pinned by the parity suite: **the value tier never rejects
what the engine accepts.** For this law the set-match + closed-id rule is
the complete engine rule, so the tiers agree exactly; every *other* schema
judgment (key-internal legality, fresh-on-u64, selection rules, …) stays
engine-first, untouched — this document adds one wall, not a second
validator.

### 2. Type tier — `TargetKeyWall` (best effort, statically known tuples)

A named, self-locating compile error in `LawfulStatements`
(`ts/src/law.ts`), the `ClassWall` pattern: from the statements tuple
type, per containment/mirrors/capacity statement, compare the target
face's projection tuple (as a set — mutual-subset over string-literal
unions, duplicate-safe) against the key roster readable from the tuple
(declared `key()` data) and the relation record (fresh-marked fields,
closed ids). Same degradation law as the class machinery: a widened
`Statement[]` degrades the type tier to silent; the value tier stays
authoritative. Gated by **G3** ([10-measurement.md](10-measurement.md)):
if the law-scale suite's check time regresses more than 15%, the type
tier ships disabled and this section is amended with the numbers that
refused it.

### 3. `lower()` — unchanged, totality inherited

`lower()` stays validation-free ("lowering is TOTAL on well-typed
inputs"). The report's requirement that it never emit an engine-refused
containment is discharged one boundary earlier: `lower()` only accepts
`schema()` outputs, and `schema()` now refuses. Its header gains one line
stating the inheritance.

### 4. Engine — names in the diagnostic, authority unchanged

`NoMatchingTargetKey`, `NoPointwiseTargetKey`, and
`ClosedTargetNotHandle` displays gain relation and field **names**
(available in the descriptor at validation time) alongside the ids; the
pinned reject-test strings (`schema/tests/reject.rs`) update in the same
change. The engine remains the final authority for every caller that
never passes through the TS boundary — parity means the SDK catches it
first, not instead.

### 5. The stale comments die with the gap

`ts/src/statements.ts:27-34` and `ts/src/schema.ts:1-10` currently
document the deferral as design ("DELIBERATELY left to the engine…").
After this change those words describe a bug that no longer exists;
both headers are rewritten to state the two-tier wall and the engine's
final authority ([70-deletions.md](70-deletions.md) D13). A normative
comment that contradicts the code is worse than no comment.

## The parity suite (pinned)

One matrix, each case run through **both** boundaries, verdicts required
equal — `schema()` throw ⇔ `Db.create()` `SchemaError` — with the
report's shapes as the first rows:

| Case | Verdict |
| --- | --- |
| the report's minimal shape (`Target{scope,value}` keyed, containment on `value` alone) | refused |
| Primer's shape (`AnalysisTargetEntry`, projection `(sourceAddress)` vs keys `(entry)`, `(policyPackage, sourceAddress)`) | refused |
| projection = a declared key, same order | admitted |
| projection = a declared key, permuted order | admitted (set equality) |
| projection = strict subset / superset of a key | refused |
| target = fresh-implied key | admitted |
| closed target, projection `["id"]` | admitted |
| closed target, any payload projection (even one that equals a declared payload key) | refused (`ClosedTargetNotHandle`, its own message) |
| `mirrors` with both faces keyed | admitted |
| `mirrors` with exactly one face keyed | refused, naming the unkeyed orientation |
| `capacity` with non-key target | refused, same rule |
| interval-bearing projection with no pointwise key | refused with the pointwise hint |

Plus the diagnostic pins: the value-tier message renders names and the
available-keys list; the engine message now carries names; both spell the
statement in the canonical rendering (`renderStatement` / the engine's
`schema/render.rs` — the paste-back law).

## Primer disposition (unchanged, by design)

Primer keeps its stage-local law — the bound policy instance checked by
the deterministic Analysis validator — and keeps the containment omitted
from the cross-theory workspace. What changes for Primer: the refusal now
happens at `schema()` assembly, in names, at the moment the statement is
written, instead of at `Db.create()` in a later phase in id-speak.
