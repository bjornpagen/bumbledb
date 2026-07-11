use crate::exec::sink::{AggregateSink, ArgSpec, FindSpec, FoldOp};
use crate::exec::wordmap::WordMap;

/// The derived-slot rewrite (the measure's representation move (20-query-ir § the measure)): every
/// measure spec becomes a plain word spec over a **derived scratch
/// word** past the rule's real slots, and the measure table records
/// (derived word, interval slot) for the one place the subtraction runs
/// (`fold_scratch_row`). Group keys, dedup keys, folds, and finalize
/// then consume plain words with zero measure awareness — the measure is
/// a word in the row, not a branch in the folds.
fn rewrite_measures(finds: &mut [FindSpec], slot_count: usize, measures: &mut Vec<(usize, usize)>) {
    measures.clear();
    for find in finds {
        let (op_slot, rewritten) = match *find {
            FindSpec::Duration { slot } => (
                slot,
                FindSpec::Var {
                    slot: slot_count + measures.len(),
                    width: 1,
                },
            ),
            FindSpec::AggDuration { op, slot } => (
                slot,
                FindSpec::Agg {
                    op,
                    over_slot: Some(slot_count + measures.len()),
                    over_width: 1,
                    // The measure is u64 — the unsigned wide accumulator
                    // with the single finalize range check, like every
                    // Sum(U64).
                    signed: false,
                },
            ),
            FindSpec::Var { .. }
            | FindSpec::Agg { .. }
            | FindSpec::Arg { .. }
            | FindSpec::Pack { .. } => continue,
        };
        measures.push((slot_count + measures.len(), op_slot));
        *find = rewritten;
    }
}

/// The one Pack slot of a find list, if any (validation: at most one
/// Pack per head — shared by construction and per-rule re-aiming).
fn pack_slot(finds: &[FindSpec]) -> Option<usize> {
    let mut packs = finds.iter().filter_map(|f| match f {
        FindSpec::Pack { slot } => Some(*slot),
        _ => None,
    });
    let slot = packs.next();
    debug_assert!(packs.next().is_none(), "validated: at most one Pack");
    slot
}

impl AggregateSink {
    /// Builds the sink. `slot_count` is the plan's binding-slot count in
    /// **words** (an interval variable holds two — the `SlotWidth` layout);
    /// `distinct_bindings` is the plan's elision flag (30-execution): when
    /// set, the seen-set is skipped entirely; `union` is the multi-rule
    /// regime (the seen-set keys head projections and spans rules).
    /// Unhinted construction (tests; production sinks are hint-sized).
    #[cfg(test)]
    #[must_use]
    pub fn new(finds: Vec<FindSpec>, slot_count: usize, distinct_bindings: bool) -> Self {
        Self::with_capacity_hint(finds, slot_count, distinct_bindings, false, 0)
    }

    /// Presized construction: the dedup seen-set
    /// takes the plan's output estimate; the group map takes a small
    /// clamp of it (groups are few — the estimate bounds bindings, not
    /// groups).
    ///
    /// Dedup is **per-query-shape**: a single-rule sink (`union` false)
    /// keys the whole slot array; a multi-rule sink keys the **head
    /// projection** — rule-independent by construction. `distinct` is
    /// the caller's proof that the emitted dedup-key stream is
    /// duplicate-free, and it elides the seen-set entirely: single-rule,
    /// the plan's distinct-bindings flag; multi-rule, the rule-
    /// disjointness composition (docs/architecture/40-execution.md § set
    /// semantics — pairwise-disjoint rules, per-rule distinct bindings,
    /// and heads reading every slot). One flag, one mechanism — the
    /// elision is a representation (`seen: None`), never a hot-loop
    /// branch.
    #[must_use]
    #[allow(clippy::too_many_lines)] // one table per sink concern, in order
    pub fn with_capacity_hint(
        mut finds: Vec<FindSpec>,
        slot_count: usize,
        distinct: bool,
        union: bool,
        hint: usize,
    ) -> Self {
        // The measure rewrite first: everything below sees plain word
        // specs over the extended scratch row.
        let mut measures = Vec::new();
        rewrite_measures(&mut finds, slot_count, &mut measures);
        let scratch_words = slot_count + measures.len();
        let group_spans: Vec<(usize, usize)> = finds
            .iter()
            .filter_map(|f| match f {
                FindSpec::Var { slot, width } => Some((*slot, *width)),
                FindSpec::Agg { .. } | FindSpec::Arg { .. } | FindSpec::Pack { .. } => None,
                FindSpec::Duration { .. } | FindSpec::AggDuration { .. } => {
                    unreachable!("rewrite_measures ran")
                }
            })
            .collect();
        let key_words: usize = group_spans.iter().map(|(_, width)| width).sum();
        let n_aggs = finds
            .iter()
            .filter(|f| matches!(f, FindSpec::Agg { .. }))
            .count();
        let carry_words: usize = finds
            .iter()
            .filter_map(|f| match f {
                FindSpec::Arg { width, .. } => Some(*width),
                FindSpec::Var { .. } | FindSpec::Agg { .. } | FindSpec::Pack { .. } => None,
                FindSpec::Duration { .. } | FindSpec::AggDuration { .. } => {
                    unreachable!("rewrite_measures ran")
                }
            })
            .sum();
        // Validation guarantees every Arg term names one key and one
        // direction (20-query-ir § aggregation), so the first spec is
        // THE spec.
        let arg = finds.iter().find_map(|f| match f {
            FindSpec::Arg { key_slot, max, .. } => Some(ArgSpec {
                key_slot: *key_slot,
                max: *max,
            }),
            FindSpec::Var { .. } | FindSpec::Agg { .. } | FindSpec::Pack { .. } => None,
            FindSpec::Duration { .. } | FindSpec::AggDuration { .. } => {
                unreachable!("rewrite_measures ran")
            }
        });
        debug_assert!(
            finds.iter().all(|f| match f {
                FindSpec::Arg { key_slot, max, .. } =>
                    arg.is_some_and(|spec| spec.key_slot == *key_slot && spec.max == *max),
                FindSpec::Var { .. }
                | FindSpec::Agg { .. }
                | FindSpec::Pack { .. }
                | FindSpec::Duration { .. }
                | FindSpec::AggDuration { .. } => true,
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
                    FindSpec::Agg {
                        op: FoldOp::CountDistinct,
                        ..
                    }
                )
            });
        let union_spans = union.then(|| union_key_spans(&finds));
        let union_words: usize = union_spans
            .as_ref()
            .map_or(0, |spans| spans.iter().map(|(_, width)| width).sum());
        debug_assert!(
            !(union && arg.is_some()),
            "validated: Arg-restriction never crosses rules"
        );
        Self {
            groups: WordMap::with_capacity_hint(key_words, hint.min(4096)),
            key_scratch: vec![0; key_words],
            binding_scratch: vec![0; scratch_words],
            // Single-rule: whole-binding key. Multi-rule: head-projection
            // key. Either is elided exactly when the caller proved its
            // stream duplicate-free (`distinct`).
            seen: (!distinct).then(|| {
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
            arg_rows: Vec::new(),
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
    pub fn aim(&mut self, finds: &[FindSpec], slot_count: usize) {
        debug_assert_eq!(finds.len(), self.finds.len(), "one head, fixed arity");
        self.finds.clear();
        self.finds.extend_from_slice(finds);
        // The measure rewrite, per rule: derived words sit past this
        // rule's real slots (the head's measure positions are fixed, so
        // the measure count never changes across rules) — rebuilt into
        // retained capacity (the warm allocation contract).
        let mut measures = std::mem::take(&mut self.measures);
        rewrite_measures(&mut self.finds, slot_count, &mut measures);
        self.measures = measures;
        self.real_slots = slot_count;
        self.group_spans.clear();
        self.group_spans
            .extend(self.finds.iter().filter_map(|f| match f {
                FindSpec::Var { slot, width } => Some((*slot, *width)),
                FindSpec::Agg { .. } | FindSpec::Arg { .. } | FindSpec::Pack { .. } => None,
                FindSpec::Duration { .. } | FindSpec::AggDuration { .. } => {
                    unreachable!("rewrite_measures ran")
                }
            }));
        // The Pack slot is the rule's (the head position is fixed;
        // validation aligned every rule's Pack term against it).
        self.pack = pack_slot(&self.finds);
        if let Some(spans) = &mut self.union_spans {
            spans.clear();
            spans.extend(self.finds.iter().filter_map(union_span));
        }
        debug_assert!(
            self.arg.is_none() && !self.finds.iter().any(|f| matches!(f, FindSpec::Arg { .. })),
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
        self.seen.as_ref().map(WordMap::len)
    }

    /// Whether the binding seen-set is elided (the plan proved distinct
    /// bindings) — the elision observable. `CountDistinct`'s value sets and
    /// the Arg row-dedup are different sets and are NEVER elided.
    #[cfg(test)]
    #[must_use]
    pub fn seen_elided(&self) -> bool {
        self.seen.is_none()
    }

    /// The differential guard's override: reinstates an elided seen-set
    /// (whole-binding or head-projection keyed, matching the regime) so a
    /// covered query runs both ways — the elision is *never* semantic,
    /// and forced-off results must be byte-identical.
    #[cfg(test)]
    pub fn force_seen(&mut self) {
        if self.seen.is_none() {
            let arity = if self.union_spans.is_some() {
                self.union_scratch.len()
            } else {
                self.binding_scratch.len()
            };
            self.seen = Some(WordMap::with_capacity_hint(arity, 0));
        }
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

/// One head position's contribution to the union dedup key: the slot
/// span the position reads — a group variable's span, a fold input's
/// span, or a Pack input's two-word span (the fold domain projected to
/// the head carries the *raw claim*, so the spanning seen-set keys
/// (group, claim) pairs and the coalesce folds the union — 20-query-ir
/// § aggregation); the nullary `Count` reads nothing and contributes
/// nothing (the naive model's "constant filler", represented as
/// absence). Arg terms are unreachable here (validation refuses
/// Arg-restriction across rules).
fn union_span(find: &FindSpec) -> Option<(usize, usize)> {
    match find {
        FindSpec::Var { slot, width } => Some((*slot, *width)),
        FindSpec::Agg {
            over_slot: Some(slot),
            over_width,
            ..
        } => Some((*slot, *over_width)),
        FindSpec::Pack { slot } => Some((*slot, 2)),
        FindSpec::Agg {
            over_slot: None, ..
        } => None,
        FindSpec::Arg { .. } => unreachable!("validated: no Arg across rules"),
        FindSpec::Duration { .. } | FindSpec::AggDuration { .. } => {
            unreachable!("rewrite_measures ran")
        }
    }
}

/// The multi-rule dedup-key spans over one rule's binding layout — the
/// head projection, position by position.
fn union_key_spans(finds: &[FindSpec]) -> Vec<(usize, usize)> {
    finds.iter().filter_map(union_span).collect()
}
