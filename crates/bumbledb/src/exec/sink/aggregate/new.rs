use crate::exec::sink::{
    AggregateSink, ArgSpec, DENSE_GROUPS_CAP, FindSpec, FoldOp, GroupTable, SinkSpec,
};
use crate::exec::wordmap::WordMap;

/// Parses prepare's find vocabulary into the measure-free execution
/// vocabulary. Every measure becomes a derived scratch word past the
/// rule's real slots; the companion table records (derived word,
/// interval start slot) for the one subtraction site. No execution
/// consumer can observe the symbolic measure variants.
pub(in crate::exec::sink) fn parse_finds(
    finds: &[FindSpec],
    slot_count: usize,
) -> (Vec<SinkSpec>, Vec<(usize, usize)>) {
    let mut parsed = Vec::with_capacity(finds.len());
    let mut measures = Vec::new();
    parse_finds_into(finds, slot_count, &mut parsed, &mut measures);
    (parsed, measures)
}

/// [`parse_finds`] into retained buffers for the rule-loop re-aim path.
pub(in crate::exec::sink) fn parse_finds_into(
    finds: &[FindSpec],
    slot_count: usize,
    parsed: &mut Vec<SinkSpec>,
    measures: &mut Vec<(usize, usize)>,
) {
    parsed.clear();
    measures.clear();
    for find in finds {
        let spec = match *find {
            FindSpec::Var { slot, width } => SinkSpec::Var { slot, width },
            FindSpec::Duration { slot } => {
                let derived = slot_count + measures.len();
                measures.push((derived, slot));
                SinkSpec::Var {
                    slot: derived,
                    width: 1,
                }
            }
            FindSpec::AggDuration { op, slot } => {
                let derived = slot_count + measures.len();
                measures.push((derived, slot));
                SinkSpec::Agg {
                    op,
                    over_slot: Some(derived),
                    over_width: 1,
                    // The measure is u64 — the unsigned wide accumulator
                    // with the single finalize range check, like every
                    // Sum(U64).
                    signed: false,
                }
            }
            FindSpec::Agg {
                op,
                over_slot,
                over_width,
                signed,
            } => SinkSpec::Agg {
                op,
                over_slot,
                over_width,
                signed,
            },
            // The Arg key's measure form (R5) parses exactly as the
            // measure finds: one derived scratch word past the real
            // slots, computed (and ray-checked) per row — the
            // restriction sweep reads a plain word either way.
            FindSpec::Arg {
                slot,
                width,
                key,
                max,
            } => {
                let key_slot = match key {
                    crate::exec::sink::ProjSource::Slot(slot) => slot,
                    crate::exec::sink::ProjSource::Measure { start } => {
                        let derived = slot_count + measures.len();
                        measures.push((derived, start));
                        derived
                    }
                };
                SinkSpec::Arg {
                    slot,
                    width,
                    key_slot,
                    max,
                }
            }
            FindSpec::Pack { slot } => SinkSpec::Pack { slot },
        };
        parsed.push(spec);
    }
}

/// The one Pack slot of a find list, if any (validation: at most one
/// Pack per head — shared by construction and per-rule re-aiming).
fn pack_slot(finds: &[SinkSpec]) -> Option<usize> {
    let mut packs = finds.iter().filter_map(|f| match f {
        SinkSpec::Pack { slot } => Some(*slot),
        _ => None,
    });
    let slot = packs.next();
    debug_assert!(packs.next().is_none(), "validated: at most one Pack");
    slot
}

impl AggregateSink {
    /// Builds the sink. `slot_count` is the plan's binding-slot count in
    /// **words** (an interval variable holds two — the `SlotWidth` layout);
    /// Unhinted, seen-set-retaining construction (tests).
    #[cfg(test)]
    #[must_use]
    pub fn new(finds: impl AsRef<[FindSpec]>, slot_count: usize) -> Self {
        Self::build(finds.as_ref(), slot_count, DedupRegime::Bindings, 0, &[])
    }

    /// Unhinted dense-group construction (tests): the radixes are the
    /// schema-proven per-word domains (finding 049).
    #[cfg(test)]
    #[must_use]
    pub fn new_dense(
        finds: impl AsRef<[FindSpec]>,
        slot_count: usize,
        dense_groups: &[u16],
    ) -> Self {
        Self::build(
            finds.as_ref(),
            slot_count,
            DedupRegime::Bindings,
            0,
            dense_groups,
        )
    }

    /// Unhinted elided construction (tests): the proof is mandatory.
    #[cfg(test)]
    #[must_use]
    pub fn new_distinct(
        finds: impl AsRef<[FindSpec]>,
        slot_count: usize,
        witness: crate::plan::fj::DistinctWitness,
    ) -> Self {
        Self::build(
            finds.as_ref(),
            slot_count,
            DedupRegime::Elided(witness),
            0,
            &[],
        )
    }

    /// Presized construction: the dedup seen-set
    /// takes the plan's output estimate; the group map takes a small
    /// clamp of it (groups are few — the estimate bounds bindings, not
    /// groups).
    ///
    /// Dedup is structural: this constructor keys a single rule's whole
    /// slot array; [`Self::for_union`] keys the head projection
    /// (hand-written provenance) and [`Self::for_dnf_union`] the shared
    /// slot arrays (DNF-derived provenance — R2);
    /// [`Self::without_seen_set`] alone accepts the proof and
    /// omits the map. Multi-rule sinks always retain the spanning union
    /// representation, even when the rules are provably disjoint.
    /// `dense_groups` is the single-rule dense-domain proof (finding
    /// 049): per group-key word, the schema-proven radix — empty keeps
    /// the open-domain map.
    #[must_use]
    pub fn with_capacity_hint(
        finds: &[FindSpec],
        slot_count: usize,
        hint: usize,
        dense_groups: &[u16],
    ) -> Self {
        Self::build(finds, slot_count, DedupRegime::Bindings, hint, dense_groups)
    }

    /// Presized multi-rule construction, hand-written provenance: the
    /// head-projection seen-set is structurally mandatory because it is
    /// the union representation.
    #[must_use]
    pub fn for_union(finds: &[FindSpec], slot_count: usize, hint: usize) -> Self {
        Self::build(finds, slot_count, DedupRegime::Union, hint, &[])
    }

    /// Presized multi-rule construction, DNF-derived provenance (ruled
    /// 2026-07-23, R2): the union seen-set re-keys on the **shared slot
    /// arrays** — `spans` is rule 0's full slot array in `VarId` order,
    /// re-supplied per rule at [`Self::aim`] — so disjunction widens
    /// membership without moving the fold domain (the or-transparency
    /// law, `lean/Bumbledb/Exec/Dedup.lean: dnf_rekey_transparent`).
    #[must_use]
    pub fn for_dnf_union(
        finds: &[FindSpec],
        slot_count: usize,
        spans: &[(usize, usize)],
        hint: usize,
    ) -> Self {
        Self::build(finds, slot_count, DedupRegime::DnfUnion(spans), hint, &[])
    }

    /// Presized single-rule construction without a binding seen-set. The
    /// only entry requires the plan proof by value; `dense_groups` as
    /// [`Self::with_capacity_hint`].
    #[must_use]
    pub fn without_seen_set(
        finds: &[FindSpec],
        slot_count: usize,
        witness: crate::plan::fj::DistinctWitness,
        hint: usize,
        dense_groups: &[u16],
    ) -> Self {
        Self::build(
            finds,
            slot_count,
            DedupRegime::Elided(witness),
            hint,
            dense_groups,
        )
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one sink constructor, every regime's wiring in one place — clearer kept together"
    )]
    fn build(
        finds: &[FindSpec],
        slot_count: usize,
        regime: DedupRegime<'_>,
        hint: usize,
        dense_groups: &[u16],
    ) -> Self {
        let union = matches!(regime, DedupRegime::Union | DedupRegime::DnfUnion(_));
        let distinct_witness = match regime {
            DedupRegime::Elided(witness) => Some(witness),
            DedupRegime::Bindings | DedupRegime::Union | DedupRegime::DnfUnion(_) => None,
        };
        // Parse first: everything below sees the measure-free execution
        // vocabulary over the extended scratch row.
        let (finds, measures) = parse_finds(finds, slot_count);
        let scratch_words = slot_count + measures.len();
        let group_spans: Vec<(usize, usize)> = finds
            .iter()
            .filter_map(|f| match f {
                SinkSpec::Var { slot, width } => Some((*slot, *width)),
                SinkSpec::Agg { .. } | SinkSpec::Arg { .. } | SinkSpec::Pack { .. } => None,
            })
            .collect();
        let key_words: usize = group_spans.iter().map(|(_, width)| width).sum();
        let n_aggs = finds
            .iter()
            .filter(|f| matches!(f, SinkSpec::Agg { .. }))
            .count();
        let carry_words: usize = finds
            .iter()
            .filter_map(|f| match f {
                SinkSpec::Arg { width, .. } => Some(*width),
                SinkSpec::Var { .. } | SinkSpec::Agg { .. } | SinkSpec::Pack { .. } => None,
            })
            .sum();
        // Validation guarantees every Arg term names one key and one
        // direction (20-query-ir § aggregation), so the first spec is
        // THE spec.
        let arg = finds.iter().find_map(|f| match f {
            SinkSpec::Arg { key_slot, max, .. } => Some(ArgSpec {
                key_slot: *key_slot,
                max: *max,
            }),
            SinkSpec::Var { .. } | SinkSpec::Agg { .. } | SinkSpec::Pack { .. } => None,
        });
        debug_assert!(
            finds.iter().all(|f| match f {
                SinkSpec::Arg { key_slot, max, .. } =>
                    arg.is_some_and(|spec| spec.key_slot == *key_slot && spec.max == *max),
                SinkSpec::Var { .. } | SinkSpec::Agg { .. } | SinkSpec::Pack { .. } => true,
            }),
            "validated: all Arg terms share one key and one direction"
        );
        let pack = pack_slot(&finds);
        // Measures fold per row too: their derived words exist only in
        // the scratch row, so no gather kernel or scan pushdown can read
        // them. Pack is set-valued group state like Arg — per-row as
        // well.
        let row_fold_only = arg.is_some()
            || pack.is_some()
            || !measures.is_empty()
            || finds.iter().any(|f| {
                matches!(
                    f,
                    SinkSpec::Agg {
                        op: FoldOp::CountDistinct,
                        ..
                    }
                )
            });
        // The union key by provenance (R2): head projection for a
        // hand-written rule set, the shared slot arrays for a
        // DNF-derived one.
        let union_spans = match regime {
            DedupRegime::Union => Some(union_key_spans(&finds)),
            DedupRegime::DnfUnion(spans) => Some(spans.to_vec()),
            DedupRegime::Bindings | DedupRegime::Elided(_) => None,
        };
        let union_words: usize = union_spans
            .as_ref()
            .map_or(0, |spans| spans.iter().map(|(_, width)| width).sum());
        debug_assert!(
            !(union && arg.is_some()),
            "validated: Arg-restriction never crosses rules"
        );
        // The group representation (finding 049): dense when the caller
        // proved every key word a small domain — the product is capped
        // at construction, so the table is at most `DENSE_GROUPS_CAP`
        // words and the untouched-slot scan at finalize stays trivial.
        let groups = if dense_groups.is_empty() {
            GroupTable::Hashed(WordMap::with_capacity_hint(key_words, hint.min(4096)))
        } else {
            debug_assert_eq!(
                dense_groups.len(),
                key_words,
                "one radix per group-key word"
            );
            let product: u32 = dense_groups.iter().map(|radix| u32::from(*radix)).product();
            debug_assert!(
                0 < product && product <= DENSE_GROUPS_CAP,
                "the caller caps the dense product"
            );
            GroupTable::Dense {
                radixes: dense_groups.into(),
                table: vec![0; usize::try_from(product).expect("capped")].into_boxed_slice(),
                ordinals: Vec::new(),
            }
        };
        Self {
            distinct_witness,
            dnf_rekey: matches!(regime, DedupRegime::DnfUnion(_)),
            groups,
            key_scratch: vec![0; key_words],
            binding_scratch: vec![0; scratch_words],
            // Single-rule: whole-binding key, elided when its own plan
            // proves distinct bindings. Multi-rule: head-projection key,
            // always retained as the union representation.
            seen: distinct_witness.is_none().then(|| {
                WordMap::with_capacity_hint(if union { union_words } else { scratch_words }, hint)
            }),
            union_scratch: vec![0; union_words],
            union_spans,
            acc_scratch: Vec::with_capacity(n_aggs),
            dedup_survivors: Vec::new(),
            scan_sources: Vec::with_capacity(n_aggs),
            scan_count: 0,
            cached_outer_slots: Vec::new(),
            cached_constant_group: false,
            value_sets: Vec::new(),
            value_sets_live: 0,
            arg,
            arg_best: Vec::new(),
            arg_answers: Vec::new(),
            carry_words,
            carry_scratch: Vec::with_capacity(carry_words),
            pack,
            pack_claims: Vec::new(),
            row_fold_only,
            #[cfg(test)]
            group_probes: 0,
            group_spans,
            finds,
            measures,
            real_slots: slot_count,
            ray: None,
            accs: Vec::new(),
            n_aggs,
        }
    }

    /// Re-aims the slot tables at one rule's binding layout (the rule
    /// loop, docs/architecture/40-execution.md): the head positions are
    /// fixed — arity, ops, widths, types — but each rule supplies its own
    /// slots, so every slot-addressed table rebuilds in place (capacities
    /// retained; the shared maps — groups, seen, value sets — carry
    /// across rules untouched: the spanning is the union). Single-rule
    /// sinks are built aimed and never call this.
    pub fn aim(&mut self, finds: &[FindSpec], slot_count: usize, shared_slots: &[(usize, usize)]) {
        debug_assert_eq!(finds.len(), self.finds.len(), "one head, fixed arity");
        // The parse, per rule: derived words sit past this
        // rule's real slots (the head's measure positions are fixed, so
        // the measure count never changes across rules) — rebuilt into
        // retained capacity (the warm allocation contract).
        parse_finds_into(finds, slot_count, &mut self.finds, &mut self.measures);
        self.real_slots = slot_count;
        self.group_spans.clear();
        self.group_spans
            .extend(self.finds.iter().filter_map(|f| match f {
                SinkSpec::Var { slot, width } => Some((*slot, *width)),
                SinkSpec::Agg { .. } | SinkSpec::Arg { .. } | SinkSpec::Pack { .. } => None,
            }));
        // The Pack slot is the rule's (the head position is fixed;
        // validation aligned every rule's Pack term against it).
        self.pack = pack_slot(&self.finds);
        if let Some(spans) = &mut self.union_spans {
            spans.clear();
            if self.dnf_rekey {
                // DNF-derived provenance (R2): the caller supplies this
                // rule's full slot array in `VarId` order — the shared
                // vocabulary every disjunct reads identically.
                spans.extend_from_slice(shared_slots);
            } else {
                spans.extend(self.finds.iter().filter_map(union_span));
            }
        }
        debug_assert!(
            self.arg.is_none() && !self.finds.iter().any(|f| matches!(f, SinkSpec::Arg { .. })),
            "validated: Arg-restriction never crosses rules"
        );
        self.binding_scratch.clear();
        self.binding_scratch
            .resize(slot_count + self.measures.len(), 0);
    }

    /// Groups held (finalize's reservation).
    #[must_use]
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    /// Distinct bindings the seen-set holds — the union observable the
    /// rule loop reads per rule (absorbed = emitted − newly-seen).
    /// `None` when the seen-set is elided (the single-rule
    /// distinct-bindings proof: every emit is first-seen).
    #[must_use]
    pub fn distinct_seen(&self) -> Option<usize> {
        debug_assert_eq!(
            self.seen.is_none(),
            self.distinct_witness.is_some(),
            "only a retained distinctness proof can remove the seen-set"
        );
        self.seen.as_ref().map(WordMap::len)
    }

    /// Whether the binding seen-set is elided (the plan proved distinct
    /// bindings) — the elision observable. `CountDistinct`'s value sets and
    /// the Arg row-dedup are different sets and are NEVER elided.
    #[cfg(test)]
    #[must_use]
    pub fn seen_elided(&self) -> bool {
        self.distinct_witness.is_some()
    }

    /// Whether the group table took the dense representation (finding
    /// 049) — the construction observable.
    #[cfg(test)]
    #[must_use]
    pub fn dense_group_table(&self) -> bool {
        matches!(self.groups, GroupTable::Dense { .. })
    }

    /// Distinct values held across every live `CountDistinct` set — the
    /// value-dedup observable the elision fixture asserts alongside
    /// [`Self::seen_elided`].
    #[cfg(test)]
    #[must_use]
    pub fn distinct_values_held(&self) -> usize {
        self.value_sets[..self.value_sets_live]
            .iter()
            .map(WordMap::len)
            .sum()
    }

    /// Empties the sink for the next execution, retaining capacity —
    /// value sets and Arg row sets stay pooled (cleared on reuse at
    /// group creation, never dropped). Called once per execution, never
    /// per rule: the seen-set spanning rules IS the union
    /// (docs/architecture/40-execution.md § the rule loop).
    pub fn reset(&mut self) {
        self.groups.clear();
        self.accs.clear();
        self.value_sets_live = 0;
        self.arg_best.clear();
        self.ray = None;
        if let Some(seen) = &mut self.seen {
            seen.clear();
        }
    }

    /// The measure poison: the first ray a measure position reached —
    /// the execution's answer is the typed
    /// [`crate::Error::MeasureOfRay`], checked after the rule loop.
    #[must_use]
    pub fn measure_of_ray(&self) -> Option<[u64; 2]> {
        self.ray
    }
}

/// The dedup regime, structural at construction (ruled 2026-07-23, R2:
/// the multi-rule key splits by written-rule provenance).
#[derive(Debug, Clone, Copy)]
enum DedupRegime<'k> {
    /// Single rule: the whole slot array.
    Bindings,
    /// Hand-written multi-rule: the head projection — the rules' only
    /// shared vocabulary.
    Union,
    /// DNF-derived multi-rule: the shared slot arrays — rule 0's full
    /// slot spans in `VarId` order, one variable scope across disjuncts.
    DnfUnion(&'k [(usize, usize)]),
    /// Single rule under the plan's distinct-bindings proof: no set.
    Elided(crate::plan::fj::DistinctWitness),
}

/// One head position's contribution to the union dedup key: the slot
/// span the position reads — a group variable's span, a fold input's
/// span, or a Pack input's two-word span (the fold domain projected to
/// the head carries the *raw claim*, so the spanning seen-set keys
/// (group, claim) pairs and the coalesce folds the union — 20-query-ir
/// § aggregation); the nullary `Count` reads nothing and contributes
/// nothing (the naive model's "constant filler", represented as
/// absence). Arg terms are unreachable here (validation refuses
/// Arg-restriction across rules).
fn union_span(find: &SinkSpec) -> Option<(usize, usize)> {
    match find {
        SinkSpec::Var { slot, width } => Some((*slot, *width)),
        SinkSpec::Agg {
            over_slot: Some(slot),
            over_width,
            ..
        } => Some((*slot, *over_width)),
        SinkSpec::Pack { slot } => Some((*slot, 2)),
        SinkSpec::Agg {
            over_slot: None, ..
        } => None,
        SinkSpec::Arg { .. } => unreachable!("validated: no Arg across rules"),
    }
}

/// The multi-rule dedup-key spans over one rule's binding layout — the
/// head projection, position by position.
fn union_key_spans(finds: &[SinkSpec]) -> Vec<(usize, usize)> {
    finds.iter().filter_map(union_span).collect()
}
