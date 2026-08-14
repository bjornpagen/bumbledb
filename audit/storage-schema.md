# Storage / schema / image / snapshot-API representation audit

Brooks: the tables make the flowcharts obvious. Pike: data dominates; algorithms follow. Applied to the trees the Query-IR campaign never touched: `crates/bumbledb/src/schema/`, `storage/` (env, commit, read, delta, snapshot seams), `image/`, `api/db/` (snapshot, write, prepare glue — not prepared-query internals), plus `error.rs` payload shapes and `obs.rs`.

`audit/CONTRACT.md` C1 does **not** freeze these trees. C7 already named `ForeignPreparedQuery` as essential-with-horizon (docs-018 is the docs sentence). This dump does not re-litigate that as a must-fix engine change.

The schema *witness* already knows how to parse: `Enforcement` is a sum, `Weight` is a sum (`Unit` is a case, not an absence), `FunctionalityEvidence` is a sum, `CompiledCheck` is a sum, `StatementRef` is a spine selecting typed arenas, `Disposition` / `DeterminantOverlay` / `StoreKind` / `View` / `ColumnView` / `DistinctState` are sums. Then several of those proofs are flattened back into `Option` and `bool` on the sealed values, and every consumer reconstitutes them.

Program / Idb / stratum vocabulary has **not** leaked into these trees as live data. The leftovers are `error.rs` / `obs.rs` comments still saying "program."

---

## The shape that is wrong

The lever is one missing pair of sums on the *sealed* schema. After `SchemaDescriptor::validate`, a relation is not an ordinary row-store with an optional bag of axioms, and a key is not a projection with two independent flags. Today that fact is restated as Options:

```
Relation { extension: Option<Box<[SealedRow]>>, fresh_row_field: Option<FieldId>, .. }
KeyStatement { tail: Option<IntervalTail>, fresh_row: bool, .. }
CapacityStatement { weight: Weight, weight_tail: Option<IntervalTail>,
                    hi: Option<Bound>, bound_tail: Option<IntervalTail>,
                    enforcement: Enforcement /* includes IntervalCoverage */ }
```

Every `is_closed()`, `extension.is_some()`, `tail.is_some()` / `pointwise()`, `fresh_row`, `weight_tail.expect("validate seals a tail")`, and `unreachable!("capacity statements refuse interval positions")` is a branch guarding a state those products still admit. Closedness sits *beside* the ordinary fields that were supposed to be the whole relation. The image cache then pads `closed_slots: Box<[Option<u32>]>` — engine F6's Option-hole array, one tree over. Point reads re-derive the access path as `is_closed × fresh_row × U-tree`.

The collapsing coordinate is homogeneous sealed kinds plus a parsed body that cannot spell the illegal combinations. Until that exists, the flowchart cannot get simpler than the table.

What is already right, and is not a finding: `Enforcement` for containments, `Weight::Unit` as a case, `Witness<S>` branding for `write_from`, `View::{Unbound, All, Survivors}`, `Disposition`, `StoreKind` on disk, the `StatementId` spine over typed arenas. Those are the coordinate. The Option flatten is the leftover.

---

## Findings

### F1. Sealed `Relation.extension: Option` — closed vs ordinary as a flag; every consumer re-tests

- **Where:** `schema.rs:553-563`; `schema/relation.rs:19-28`; `api/db.rs:493-499`; `api/db/insert.rs:15`; `api/db/snapshot.rs:153-158,191-196,261-278`; `api/db/get.rs:345-348,366-376`; `image/cache.rs:104-127`; `image/cache/new.rs:17-28`; `image/cache/get_or_build.rs:77-79,269-270`; `image/build.rs:203,315`; `storage/keys.rs:226-230`; plus `verify_store/{facts,membership,determinants,fresh,reverse}.rs`, `exec/dispatch/classify.rs`, `plan/selectivity.rs`
- **What's wrong:** The comment says "the option *is* the kind — there is no relation-kind enum." That is Minsky's nullable-field example with a better name. Ordinary and closed relations share one product type. `is_closed()` is `extension.is_some()`. `WriteTx::refuse_closed` is a runtime check on every insert/delete/alloc. Point reads branch `if let Some(extension)`. Image build `debug_assert!(!is_closed())`. Key codec `debug_assert_ordinary`. The cache allocates an Option slot per relation and a second dense `OnceLock` array for the `Some`s, then `get_or_build` probes the hole before the generation map and `get_or_synthesize` `.expect("caller probed closed_slot")`. Two encodings of "this id is closed": the relation's Option, and the cache's Option hole. Shotgun parsing of a kind the validator already knew.
- **Collapsing representation:** at the *witness*, a relation is a sum. Closed carries the extension; ordinary carries the fresh-row mint field. No `is_closed()`. The cache's closed table is only as long as the closed relations (F4). `ClosedRelationWrite` remains the dyn-surface refusal (ids are data); the typed `Fact` path can stop offering write methods for closed relations (a marker trait, or no `insert` monomorphization). Descriptor-layer `RelationDescriptor.extension: Option` stays the hostile spelling — F25.
- **Essential vs accidental:** virtual storage vs LMDB storage is essential. Encoding it as `Option` beside the ordinary fields, then re-testing at every write, read, image, and key site, is accidental.
- **Severity:** high

### F2. `KeyStatement` flattens `FunctionalityEvidence` into `tail: Option` + `fresh_row: bool`

- **Where:** `schema/validate.rs:405-411,148-158`; `schema.rs:397-428`; `storage/commit/plan.rs:366-383`; `storage/commit/judgment.rs:879,1312`; `api/db/get.rs:372`; `api/db/snapshot.rs:267`
- **What's wrong:** Validate parses a functionality into `FunctionalityEvidence::{Scalar, Pointwise(DisjointDeterminantProof, IntervalTail)}` — "what validation learned survives to sealing." Then sealing throws the proof away:

```
tail: match evidence {
    Pointwise(_, tail) => Some(tail),
    Scalar => None,
},
fresh_row: projection.len() == 1 && relation.fresh_row_field() == Some(projection[0]),
```

Two independent flags, four states, three valid (fresh-row is U64, never an interval). `pointwise()` is `tail.is_some()`. `DisjointDeterminantProof` lives on containment `Enforcement::IntervalCoverage` and is discarded for keys — King: `validateNonEmpty` returns `()` and every consumer re-derives. Plan copies `pointwise: statement.tail` onto `DeterminantOp` as another Option. Judgment and point reads re-test `fresh_row`.
- **Collapsing representation:** keep the sum.

```
enum KeyForm {
    FreshRow { id: StatementId, relation: RelationId, field: FieldId },
    Scalar { id, relation, projection },
    Pointwise { id, relation, projection, tail: IntervalTail, disjoint: DisjointDeterminantProof },
}
```

Fresh-row × pointwise is unrepresentable. Plan/judgment/point-read match the form. No `pointwise()` bool. No `expect("a fresh-row determinant is one u64 word")` — the form is one word.
- **Essential vs accidental:** three key *behaviors* (F-put, scalar U, ordered-neighbor) are essential. Two flags plus a discarded proof token is accidental.
- **Severity:** high

### F3. Capacity tails are sidecar `Option`s; the judge `.expect`s the proof back

- **Where:** `schema.rs:474-508`; `schema/validate.rs:759-766,852-919`; `storage/commit/judgment.rs:120-198,1425-1429`
- **What's wrong:** `Weight` is already the right sum (`Unit` is a case, not an absence — the type's own comment). `Bound` is already a sum. Sealing then stores `weight_tail: Option<IntervalTail>` and `bound_tail: Option<IntervalTail>` *beside* them. Eight combinations; the legal ones are exactly "DurationOf iff weight_tail is Some" and "TargetDuration iff bound_tail is Some." `measure_weight` / `resolve_bound` take the pair and `.expect("validate seals a tail for every Duration weight")`. The proof never made it into the type.
- **Collapsing representation:** put the tail inside the arm that needs it.

```
enum SealedWeight { Unit, Field(FieldId), Duration { field: FieldId, tail: IntervalTail } }
enum SealedBound { Unbounded, Lit(u64), TargetField(FieldId), Duration { field: FieldId, tail: IntervalTail } }
```

`measure_weight` matches `SealedWeight`. The expect deletes. (Window `*` is F16 — same doctrine, separate field.)
- **Essential vs accidental:** Duration measure needing the trailing encoding is essential. A sidecar Option plus expect is accidental.
- **Severity:** high

### F4. `ImageCache.closed_slots: Box<[Option<u32>]>` — Option-padded array, engine F6 analog

- **Where:** `image/cache.rs:104-127`; `image/cache/new.rs:17-28`; `image/cache/get_or_build.rs:77-79,269-270`
- **What's wrong:** `closed_slots[rel] = None` means ordinary; `Some(slot)` indexes `closed: Box<[OnceLock<Arc<RelationImage>>]>`. `get_or_build` tests `closed_slot(rel).is_some()` then `get_or_synthesize` expects the same fact. Three encodings of "this id is closed": the schema Option (F1), the hole in `closed_slots`, the length of `closed`. A foreign id also answers `None` — ordinary and unknown share a hole, "the ordinary path types that error."
- **Collapsing representation:** F1's closed arm owns its image slot (or a dense closed-only array sized at cache construction, indexed by a `ClosedSlot` minted only for closed relations). No Option hole. `get_or_build` on an ordinary relation cannot spell the synthesize path.
- **Essential vs accidental:** synthesizing closed images once, outside the generation map, is essential (virtual storage). The Option-padded index is accidental, and it exists only because F1 didn't parse the kind.
- **Severity:** high
- **Depends on:** F1

### F5. Capacity reuses containment `Enforcement`, so `IntervalCoverage` is representable then `unreachable!`

- **Where:** `schema.rs:489-493`; `schema/validate.rs:922-925`; `storage/commit/judgment.rs:1395-1398`
- **What's wrong:** "capacity projections refuse interval positions, so `IntervalCoverage` is unreachable." The type still allows it. `check_capacity` matches `IntervalCoverage` and panics. The roster parsed the refusal; the witness kept the wider type. Fowler's type-code: a tag legal in every statement form, forbidden in one.
- **Collapsing representation:** `CapacityEnforcement::{ScalarProbe, Closed}` — no coverage arm. Containments keep the three-arm `Enforcement` (that sum is the right coordinate for *them*).
- **Essential vs accidental:** probe vs member-set for a capacity parent is essential. Sharing the containment enum so an illegal arm exists is accidental.
- **Severity:** medium

### F6. `IntervalTail.width: Option<u64>` and `ValueType::Interval { width: Option<u64> }` — general vs fixed as absence

- **Where:** `schema.rs:195-198`; `bumbledb-theory/src/schema.rs:114-117`; consumers of `IntervalTail::bytes` / `words`
- **What's wrong:** `None` = general (`start ‖ end`); `Some(w)` = fixed. Weight already refused this encoding: unit is a case, not an absence. Two interval *encodings* are a sum, not a missing width. Every `match self.width { None => 16, Some(_) => 8 }` reconstitutes the kind.
- **Collapsing representation:** `enum IntervalTail { General, Fixed { width: u64 } }` on the sealed witness (and, cheaply, `ValueType::Interval` as `General { element } | Fixed { element, width }` — the descriptor *can* grow a sum; this is not a C ABI). Hostile JSON/spec spellings stay whatever they are today; the Rust type parses.
- **Essential vs accidental:** two encodings are essential. Option-as-kind is accidental.
- **Severity:** medium

### F7. `FactOp` is one product for insert and delete; memberships are "dead weight" on delete

- **Where:** `storage/commit/plan.rs:84-118,127-137`; `storage/commit/applier.rs` (consumes both lists)
- **What's wrong:** The comment on `memberships`: "Dead weight on a delete op (removing a reference cannot violate an inclusion); only the insert-side judgment consumes it." `MarkEdgeOp.weight: Option<u64>` is `None` on delete "by construction — never derived." One struct, two roles, illegal fields present and ignored. Tag-plus-all-payloads.
- **Collapsing representation:**

```
enum FactOp<'d> {
    Delete { relation, fact, fact_hash, determinants, edges, capacity_keys },
    Insert { relation, fact, fact_hash, fresh_row: Option<FreshRowOp>,
             determinants, edges, memberships, capacity_edges },
}
```

Delete cannot carry memberships. Insert cannot forget them. Weight lives only on insert capacity edges (or a `CapacityEdge::{Unit, Weighted(u64)}` sum — F3's sibling).
- **Essential vs accidental:** delete-then-insert apply order is essential. One product type for both ops is accidental.
- **Severity:** medium

### F8. Point-read access path is `is_closed × fresh_row × U-tree`

- **Where:** `api/db/snapshot.rs:261-278`; `api/db/get.rs:362-376`; `storage/commit/judgment.rs:1306-1318`
- **What's wrong:** Three independent tests reconstruct one fact — which probe this key is. Closed + fresh_row is representable (then the closed branch wins). Fresh-row + U-tree is representable (then `fresh_row` skips U). The same forest in snapshot get, write-tx get, and capacity parent probe.
- **Collapsing representation:** match F1's relation kind, then F2's `KeyForm`. One probe function. The bool product deletes.
- **Essential vs accidental:** three probe implementations are essential. Re-deriving which one from two flags on two types is accidental.
- **Severity:** medium
- **Depends on:** F1, F2

### F9. `RenderedViolation` is kind-tag plus `direction: Option` plus `measure: Option`

- **Where:** `schema/render.rs:26-41,84-92`; the C ABI sibling is `bdb_violation.has_measure` + two u64 words (`audit/sdks.md` / sdk-008) — **do not duplicate sdk-008**; this is the engine-side flattening that ABI then re-flattens
- **What's wrong:** `Violation` is already a sum: Functionality / Containment{direction} / Capacity{measure}. `render_rejection` matches that sum, then stuffs it into a record where direction and measure are independently optional. Functionality-with-measure and Capacity-with-direction are representable. This is the in-process `has_measure + payload` the user named: the tag does not own its fields. (`MeasureOfRay { start, end }` is the *right* shape — both words are the ray — and is not a finding.)
- **Collapsing representation:** `RenderedViolation` mirrors `Violation`'s sum (or is a `match` producing per-arm structs). Bindings that need a flat record flatten at *their* boundary (C ABI keeps `has_measure` per C7/C6 — sdk-008).
- **Essential vs accidental:** bindings wanting named fields is essential at the C/TS edge. Flattening the engine sum into Option soup before that edge is accidental.
- **Severity:** medium

### F10. `Violation::Functionality.incumbent: Option` — scalar vs pointwise as absence

- **Where:** `error.rs:926-933`
- **What's wrong:** "`None` for a scalar put-conflict, where the determinant bytes inside `fact` already identify the collision." Pointwise carries both parties. Two conviction shapes, one product. Downstream `cited_facts` treats incumbent as optional parallel data.
- **Collapsing representation:** `Functionality::{Scalar { statement, fact }, Pointwise { statement, fact, incumbent }}`. The Option deletes.
- **Essential vs accidental:** two probe shapes are essential. Option-as-kind is accidental.
- **Severity:** medium

### F11. `Violations.cited` is empty until `attach_cited` — a phase flag in the data

- **Where:** `error.rs:1038-1045,1084-1094,1105-1108`
- **What's wrong:** "`Empty until the commit boundary's decode pass attaches it.`" `seal` / `one` inhabit an undecorated set; `cited_facts` returns `[]` for that phase and for an out-of-range index. A rejection without decoded facts is representable, then accidentally empty at the bindings layer. Parse-don't-validate: the decode pass learned the facts and stuffed them into a parallel array the type always allowed to be missing.
- **Collapsing representation:** `Violations` as sealed citations is one type; `DecoratedViolations` (citations ∥ cited facts, lengths equal by construction) is what `CommitRejected` carries. `attach_cited`'s `assert_eq!` becomes a constructor. Sweeper re-play that has no decode stays the undecorated type and is not `Error::CommitRejected`.
- **Essential vs accidental:** decode-at-reject-time (pending interns) is essential. One type spanning both phases is accidental.
- **Severity:** medium

### F12. Live `Environment` encodes three modes as `_lock: Option` + `dirty_marker: Option`

- **Where:** `storage/env.rs:238-261,284-287,398-407`; `storage/env/exhume.rs:55`; `storage/env/ephemeral.rs:148`
- **What's wrong:** Disk `StoreKind` is a sum (Durable | Ephemeral) — parse, don't validate, at open. The live handle then has two independent Options: lock (None = exhume, read-only) and dirty marker (Some = ephemeral writer). Durable writer, ephemeral writer, and exhume reader are three modes; the product admits lockless-ephemeral-writer and locked-exhume. `Drop` tests `dirty_marker.take()` to decide whether to fsync. Exhume's armed-marker conviction is `CorruptionError::MalformedValue("ephemeral dirty marker armed…")` — a string, not a variant (F17).
- **Collapsing representation:**

```
enum EnvMode {
    Durable { lock: File },
    Ephemeral { lock: File, dirty_marker: PathBuf },
    Exhume,
}
```

`Drop` matches Ephemeral. Write constructors cannot spell Exhume. Exhume cannot spell a marker.
- **Essential vs accidental:** three open lanes are essential (R17/R18). Two Options are accidental.
- **Severity:** medium

### F13. `Const` is a universal value; `ResolvedWordSource::Var` is inhabited then `unreachable!`

- **Where:** `image/view.rs:35-77,91-95,112-209`; `image/view/apply.rs:15-19,63-80,214-241`
- **What's wrong:** `FilterPredicate` is a good kind-sum. Its payloads then carry `Const`, which admits Word/Byte/Words/Interval/Param/ParamSet/WordSet/PendingIntern at every site. `FieldAllen.other`, `DurationCompare.value`, `AnyPointIn.set` each legally hold the wrong arm; apply.rs is a forest of `unreachable!("validated: …")`. `ResolvedWordSource::Var` "never reaches the view evaluator" (plan routes it) — the type still has the arm, and `point_word` panics. Proof discarded at plan, re-asserted in the image layer.
- **Collapsing representation:** per-kind payloads: `DurationCompare { field, op, value: WordOrParam }`, `AnyPointIn { field, set: SetConst }`, `FieldAllen { field, other: IntervalConst, mask }`. Drop `Var` from the view-level source (it lives on `PlanNode::point_probes` only). The unreachable arms delete.
- **Essential vs accidental:** bind-time Param vs already-resolved Word is essential. One Const enum at every site is accidental.
- **Severity:** medium

### F14. Dual coordinate: `KeyStatement.fresh_row` and `Relation.fresh_row_field`

- **Where:** `schema.rs:417,576-581`; `schema/validate.rs:155-157,1664-1679`; `image/cache/get_or_build.rs:140`; `storage/delta/accessors.rs:75`
- **What's wrong:** "this relation's first fresh field is the F row id" is stored on the relation *and* restated as a bool on the matching auto-key. Sealing re-derives the bool from the Option. Consumers pick a side. Dijkstra: the special case lives in two numberings of one fact.
- **Collapsing representation:** F2's `KeyForm::FreshRow` *is* the relation's mint. `Relation` ordinary arm holds `Option<KeyId>` to that key, or the key form alone is enough. No second bool.
- **Essential vs accidental:** the one-id-allocator law (R16) is essential. Two fields is accidental.
- **Severity:** medium
- **Depends on:** F2

### F15. Dual coordinate: sealed `ContainmentStatement.mirror` vs render's `Vec<Option<StatementId>>`

- **Where:** `schema.rs:447-463`; `schema/validate.rs:191,295-324`; `schema/render.rs:79`
- **What's wrong:** Validate seals `mirror: Option<StatementId>` on the containment. `render_rejection` cannot read the sealed field (it is pure over a possibly-rejected `SchemaDescriptor`), so it rebuilds `mirror_links` — an Option-padded array, one hole per statement including every FD and one-way containment that cannot have a partner. Two implementations of one pairing; the sealed one is unused on the public render path.
- **Collapsing representation:** `mirror_links` returns containments-only (a `BTreeMap<StatementId, StatementId>`, or a slice parallel to the containment subsequence). The Option holes for keys die. On a sealed `Schema`, render reads `ContainmentStatement.mirror` and does not re-search.
- **Essential vs accidental:** pairing from a rejected descriptor (never sealed) is essential for diagnostics. Padding the table with None for every non-containment is accidental.
- **Severity:** medium

### F16. Sealed `hi: Option<Bound>` — `*` as absence, contradicting `Weight::Unit`

- **Where:** `schema.rs:484-487`; `bumbledb-theory/src/schema.rs:379-391`; `storage/commit/judgment.rs:179-180,1344-1354`
- **What's wrong:** Theory `Weight` comment: "`Unit` is a case, not an absence, so the wire, the descriptor encoding, and this type agree." Capacity `hi: None` is the `*` spelling — the same doctrine, not applied. `resolve_bound` returns `Option<u64>` and the judge does `hi.is_some_and(|hi| measure > u128::from(hi))`. Unbounded is a real window, not a missing bound.
- **Collapsing representation:** F3's `SealedBound::Unbounded` (descriptor may keep `Option` as hostile `*` spelling, like F25). Judge matches Unbounded vs a resolved ceiling.
- **Essential vs accidental:** unbounded ceilings are essential. Option is accidental.
- **Severity:** medium
- **Depends on:** F3 (same constructor)

### F17. `CorruptionError::MalformedValue(&'static str)` is a catch-all kind

- **Where:** `error.rs:100-103`; `storage/env/exhume.rs:76-79`; other `MalformedValue("…")` sites
- **What's wrong:** The type's own doc on `MetaMissing` vs `StoreKindInvalid`: "the two states point at opposite remedies, so one error value never encodes both." Then an armed ephemeral dirty marker — a distinct remedy (wipe vs investigate) — is `MalformedValue("ephemeral dirty marker armed — …")`. Stringly typed corruption. Future sites add more strings instead of variants.
- **Collapsing representation:** a named variant per distinct remedy (`EphemeralDirtyArmed`, plus any other string that is really a kind). `MalformedValue` stays only for "this counter/id failed to decode," with the static name of the *width*, not the diagnosis.
- **Essential vs accidental:** unknown-shaped bytes needing a diagnosis string is essential for true decode failures. Distinct lifecycle states hiding in that arm are accidental.
- **Severity:** medium

### F18. `SealedField.declared: Option` — synthetic id as absence of a descriptor

- **Where:** `bumbledb-theory/src/schema.rs:427-437,457-471`
- **What's wrong:** "`declared: None` exactly at the synthetic id." Closed relations prepend `(id, u64)` with no `FieldDescriptor`. Callers test `declared.is_some_and(|f| f.generation == Fresh)` (`materialized_statements`). Synthetic vs declared is a sum, not a missing descriptor.
- **Collapsing representation:** `enum SealedField<'a> { SyntheticId, Declared(&'a FieldDescriptor) }`. `materialized_statements` matches. No `is_some_and`.
- **Essential vs accidental:** the synthetic-id law is essential. Option is accidental.
- **Severity:** medium

### F19. `TraceEvent`: `dur_ns == 0` ⇒ point event; `a0`/`a1` always present

- **Where:** `obs.rs:50-64,70-71`
- **What's wrong:** "(`dur_ns == 0` ⇒ point event)." A duration pun, like rec-as-`interiors.len()`. Span vs point is a sum. `a0`/`a1` are tag-plus-all-payloads; names document unused as `-`. Chrome-trace *wire* is two args — that flattening is essential at export (F27). The Rust type matching the wire before export is the accidental half.
- **Collapsing representation:** `enum TraceEvent { Span { name, cat, start_ns, dur_ns, args }, Point { name, cat, start_ns, args } }`. Export writes `dur: 0` for Point. Args can stay `(u64, u64)` — the wire's two words (F27).
- **Essential vs accidental:** Chrome-trace two-arg payload is essential. `dur_ns == 0` as a mode bit is accidental.
- **Severity:** low

### F20. `View::image` / `position_at` panic on `Unbound`

- **Where:** `image/view.rs:212-277`
- **What's wrong:** `View` is already the right three-variant sum ("not a sentinel vector"). Then `image()` and `position_at` are total over a type that includes Unbound and `unreachable!`. Phase is in the data; methods pretend it isn't.
- **Collapsing representation:** bound views as a type the executor holds after the first bind (`enum BoundView { All, Survivors }`); Unbound stays on the prepared object until then. Or the methods take `BoundView`. Typestate across prepare→execute is the expensive version (Insight 15) — do not introduce a third lifetime; split the enum the executor already has in hand.
- **Essential vs accidental:** Unbound-until-first-execute is essential. Methods that panic on it are accidental.
- **Severity:** low

### F21. `TransientImage.image: Option<Arc<RelationImage>>` — empty pool as Option

- **Where:** `image/build.rs:435-549`
- **What's wrong:** A retained-capacity pool: empty at construction, filled after first refill. `fill` expects "filled above" after a branch that just filled. Genuine lazy slot; the expects are the empty-vs-full product talking.
- **Collapsing representation:** `enum TransientImage { Empty { capacity: usize }, Occupied { image: Arc<RelationImage>, capacity } }` or always allocate a zero-row sealed image at `new` (sentinel image — Insight 8). Cheap either way; Empty-as-Option is the smaller smell.
- **Essential vs accidental:** pooling is essential. Option plus expect is accidental.
- **Severity:** low

### F22. "program" vocabulary in `error.rs` / `obs.rs` / prepare glue

- **Where:** `error.rs:603-604` ("hand-written 2+-rule program"); `obs.rs:155-156` ("legal program"); `api/db/prepare.rs:14-15` ("A query whose `rec` is `None` never enters the reach driver")
- **What's wrong:** C7: no `program` on live data. These are comments/docs on error variants and the db prepare entry — not live fields, not Idb/stratum. The prepare sentence restates Query's `Option<Rec>` as the API's explanation of reach, the coordinate the engine campaign is deleting.
- **Collapsing representation:** "query" / "rule set." Prepare: "a Reach pipeline runs the rec; a Cq pipeline does not" — after engine-001, not `rec is None`.
- **Essential vs accidental:** accidental naming. No control-flow diversion (unlike engine F11's `stats.strata`).
- **Severity:** low

### F23. `CommitReport { changed: bool, new_generation }` — a mode bit beside the clock

- **Where:** `storage/commit.rs:74-77`
- **What's wrong:** Two fields, four states. A counters-only/no-op commit is `changed: false` with a generation that did not advance (cache keys on this). The bool restates whether `new_generation` moved.
- **Collapsing representation:** `enum CommitReport { Noop { generation }, Changed { new_generation } }` — or keep the struct if `changed` is not a function of the two generations the caller already has. If the cache subscriber only reads `changed`, the enum is the subscriber's match.
- **Essential vs accidental:** no-op vs state-changing is essential (fresh marks persist on empty deltas). A bool beside the id is likely accidental; cheap to confirm against the cache-advance caller.
- **Severity:** low

### F24. `ForeignPreparedQuery` / `ForeignSnapshot` — essential runtime identity (WONTFIX)

- **Where:** `error.rs:1313-1323`; `api/db/write.rs:88-105,221-223`; `api/prepared.rs:183-189` (cited, not in scope to change); `api/db/prepare.rs` glue
- **What's wrong:** Nothing, as a must-fix. Cross-schema is unrepresentable (`Db<S>`). Cross-environment is a process-distinct instance id, a runtime fact no static type can carry across two `&Db<S>` of the same lifetime (lifetime equality is not identity). `Witness<S>` already brands write_from with instance+generation; `PreparedQuery` still key-probes `env_instance: u64` at execute. That check is the documented horizon, not a missing sum.
- **Collapsing representation:** the horizon C7 names — brand `PreparedQuery` with an environment/generation witness so a foreign snapshot fails at the call type *where the host language can express it*. Not cheap (invariant-lifetime tokens still fail when both Dbs share `'a`; a unique brand type per `open` is an API). Do not implement under this id.
- **Essential vs accidental:** essential. Recorded so a later campaign does not "fix" it into a bool product or delete the error.
- **Severity:** (recorded) — WONTFIX, same class as engine-037
- **Not docs-018:** that issue is the docs sentence. This row is the engine occurrence of the same ruling.

### F25. Descriptor `RelationDescriptor.extension: Option` is the hostile spelling (non-violation)

- **Where:** `bumbledb-theory/src/schema.rs:416-425`; `schema/descriptor_codec.rs:111-120` (already a 0/1 tag byte)
- **What's wrong:** Nothing to "fix" into a third relation type at the declaration boundary. Analog of engine F37 / CONTRACT C1: the untrusted descriptor admits the Option so validate can refuse `EmptyExtension` / `StrOnClosedRelation` / … by name. The codec already stores a sum (tag 0/1). The sealed `Relation` is where Option must die (F1). Charge every `is_closed()` to F1, not to this constructor.
- **Collapsing representation:** do not. `SchemaDescriptor` stays the hostile product. `Schema` / `Relation` become the sum.
- **Essential vs accidental:** hostile Option is the boundary's job. Sealed Option is F1.
- **Severity:** (recorded) — WONTFIX non-violation

---

## Counts

| Severity | Count |
|---|---|
| high     |     4 |
| med      |    14 |
| low      |     5 |
| WONTFIX  |     2 |
| **total**|  **25** |

High: F1–F4. Med: F5–F18. Low: F19–F23. WONTFIX: F24–F25.

---

## The one table that would delete the flowchart

```rust
enum RelationBody {
    Ordinary { fields, layout, keys, outgoing, capacity_sources, capacity_targets,
               fresh: Option<KeyId> },  // KeyForm::FreshRow
    Closed   { fields, layout, keys, outgoing, capacity_sources, capacity_targets,
               extension: Box<[SealedRow]> },
}

enum KeyForm {
    FreshRow  { id: StatementId, relation: RelationId, field: FieldId },
    Scalar    { id, relation, projection: Box<[FieldId]> },
    Pointwise { id, relation, projection, tail: IntervalTail, disjoint: DisjointDeterminantProof },
}

enum SealedWeight {
    Unit,
    Field(FieldId),
    Duration { field: FieldId, tail: IntervalTail },
}
enum SealedBound {
    Unbounded,
    Lit(u64),
    TargetField(FieldId),
    Duration { field: FieldId, tail: IntervalTail },
}
enum CapacityEnforcement { ScalarProbe { .. }, Closed { members: MemberSet } }

enum EnvMode { Durable { lock: File }, Ephemeral { lock: File, dirty_marker: PathBuf }, Exhume }
```

Closed cannot be written. Fresh-row cannot be pointwise. Duration cannot miss its tail. Capacity cannot spell interval coverage. Exhume cannot hold a dirty marker. `is_closed()`, `pointwise()`, `weight_tail.expect`, and `closed_slots[rel]` have nothing to test.

Brooks: show the tables. This is the table. The `if rel.is_closed()` forest is the flowchart it makes obsolete.
