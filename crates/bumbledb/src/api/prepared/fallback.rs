//! The complete disk-native cursor fallback (chapter 12 §3): a depth-first
//! index-nested-loop evaluation of one rule over the source's committed
//! row cursors — no relation image, no COLT, bounded state (the current
//! binding stack plus one decoded row per depth). Its purpose is complete
//! bounded operation outside the resident regime and a simple
//! implementation to compare against (`Q-FALLBACK`/`Q-DISK`); it is never
//! the performance reference the warm Free Join path may regress to.
//!
//! Semantics are the rule's set denotation, unchanged: positive atoms bind
//! in plan order with filters and unifications applied as soon as their
//! operands exist; negative atoms and residual comparisons run once their
//! variables are bound; surviving bindings stream to the SAME sinks the
//! resident path uses, so dedup, aggregation, computed outputs and stage
//! error boundaries are shared, not re-implemented. Text words come from
//! the one interner (intern-on-read: the fallback's text working set is
//! charged and bounded by the operation ledger; the hashed-word text path
//! for text-heavy beyond-RAM relations is a recorded C04 follow-up).

use std::sync::Arc;

use super::derived::SealedStage;
use super::reach::SealedStageRef;
use super::source::QuerySource;
use crate::error::{Error, Result};
use crate::exec::run::{Bindings, Flow, Sink};
use crate::image::RelationImage;
use crate::image::canon::RowWords;
use crate::image::intern::InternerHandle;
use crate::image::view::{Const, FilterPredicate, ImageRow, Loaded, OperandAddr, Operands};
use crate::ir::VarId;
use crate::ir::normalize::{AntiProbe, OccBind, Occurrence, Role};
use crate::schema::{DistinctnessWitness, Schema, VisitControl, VisitOutcome};
use bumbledb_theory::schema::{FieldId, RelationId, ValueType};

/// How a bound variable's slots load for residual comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoadKind {
    Word,
    Pair,
    Block(u8),
}

/// One rule's sealed fallback program: the normalized occurrences (filter
/// templates included), residual rosters and the PLAN's slot layout, so
/// emitted bindings land exactly where the shared sinks' find specs read.
pub(super) struct FallbackRule {
    occurrences: Vec<Occurrence>,
    residuals: Vec<FilterPredicate>,
    word_residuals: Vec<FilterPredicate>,
    allen_residuals: Vec<FilterPredicate>,
    anti_probes: Vec<AntiProbe>,
    /// `(var, slot, width, kind)` in the plan's layout.
    slots: Vec<(VarId, usize, usize, LoadKind)>,
    /// True at a string variable's first slot. Numeric words can equal
    /// `SCRATCH_TOKEN_TAG`; only text slots dispatch on the token tag.
    text_slots: Vec<bool>,
    slot_count: usize,
    /// Per occurrence: this execution's resolved filters (params/literals
    /// substituted); refilled by `resolve` before every run.
    resolved: Vec<Vec<FilterPredicate>>,
    /// Per occurrence, per selection level: resolved key words for
    /// key-aware indexed probes and early stopping.
    resolved_selections: Vec<Vec<Vec<u64>>>,
    /// Plan-side selection templates (not carried on normalized occurrences).
    selection_templates: Vec<Vec<crate::plan::fj::Selection>>,
    /// Plan-side trie schemas for keyed fallback probes.
    trie_schemas: Vec<Vec<Vec<VarId>>>,
}

impl FallbackRule {
    pub(super) fn seal(
        normalized: &crate::ir::normalize::NormalizedQuery,
        plan: &crate::plan::fj::ValidatedPlan,
        var_type: impl Fn(VarId) -> ValueType,
    ) -> Self {
        let mut text_slots = vec![false; plan.slot_count()];
        let slots = plan
            .slot_spans()
            .into_iter()
            .map(|(var, slot, width)| {
                let ty = var_type(var);
                if matches!(ty, ValueType::String) {
                    text_slots[slot] = true;
                }
                let kind = match ty {
                    ValueType::Interval { .. } | ValueType::FixedInterval { .. } => LoadKind::Pair,
                    ValueType::Id128 => LoadKind::Block(2),
                    ValueType::FixedBytes { len } => {
                        match crate::encoding::fixed_bytes_words(len) {
                            1 => LoadKind::Word,
                            count => LoadKind::Block(u8::try_from(count).expect("≤ 8 words")),
                        }
                    }
                    _ => LoadKind::Word,
                };
                (var, slot, width, kind)
            })
            .collect::<Vec<_>>();
        let occurrences = normalized.occurrences.clone();
        let resolved = vec![Vec::new(); occurrences.len()];
        let resolved_selections = vec![Vec::new(); occurrences.len()];
        let selection_templates = plan
            .occurrences()
            .iter()
            .map(|occurrence| occurrence.selections.clone())
            .collect();
        let trie_schemas = plan
            .occurrences()
            .iter()
            .map(|occurrence| occurrence.trie_schema.clone())
            .collect();
        Self {
            occurrences,
            residuals: normalized.residuals.clone(),
            word_residuals: normalized.word_residuals.clone(),
            allen_residuals: normalized.allen_residuals.clone(),
            anti_probes: normalized.anti_probes.clone(),
            slots,
            text_slots,
            slot_count: plan.slot_count(),
            resolved,
            resolved_selections,
            selection_templates,
            trie_schemas,
        }
    }

    fn locate(&self, var: VarId) -> (usize, usize, LoadKind) {
        self.slots
            .iter()
            .find(|(bound, ..)| *bound == var)
            .map(|(_, slot, width, kind)| (*slot, *width, *kind))
            .expect("plan slots cover every rule variable")
    }
}

fn slot_words_equal(
    rule: &FallbackRule,
    interner: &InternerHandle<'_>,
    store: &mut Option<crate::image::NonresidentTextStore>,
    bindings: &Bindings,
    slot: usize,
    width: usize,
    span_words: &[u64],
) -> Result<bool> {
    if width == 1 && rule.text_slots.get(slot).copied().unwrap_or(false) {
        return super::text::words_equal(interner, store, bindings.get(slot), span_words[0]);
    }
    Ok((0..width).all(|w| bindings.get(slot + w) == span_words[w]))
}

/// Everything one fallback run reads (immutable operation context).
pub(super) struct FallbackCtx<'a> {
    pub(super) source: &'a QuerySource<'a>,
    pub(super) schema: &'a Schema,
    pub(super) interner: &'a InternerHandle<'a>,
    pub(super) params: &'a [Const],
    pub(super) missed: &'a [bool],
    /// Production beyond-memory text: opened only on `BeyondMemory`.
    pub(super) nonresident: &'a mut Option<crate::image::NonresidentTextStore>,
}

fn resolve_derived<'a>(
    published: &'a mut [SealedStage],
    rec_delta: Option<&'a mut SealedStage>,
    rec_acc: Option<&'a mut SealedStage>,
    bind: OccBind,
) -> Option<SealedStageRef<'a>> {
    match bind {
        OccBind::Finished(id) => published.get_mut(id.index()).map(SealedStageRef::Stage),
        OccBind::RecDelta(_) => rec_delta.map(SealedStageRef::Stage),
        OccBind::RecAcc(_) => rec_acc.map(SealedStageRef::Stage),
        OccBind::Edb(_) => None,
    }
}

/// Run one rule through the cursor fallback into `sink`.
/// # Errors
/// Storage/work failure, corrupt stored bytes, or a latch refusal. A
/// short-circuited resolution (empty set under positive `Eq`) is `Ok` —
/// the rule contributes nothing on this snapshot.
pub(super) fn run_fallback<S: Sink>(
    rule: &mut FallbackRule,
    ctx: &mut FallbackCtx<'_>,
    published: &mut [SealedStage],
    rec_delta: Option<&mut SealedStage>,
    rec_acc: Option<&mut SealedStage>,
    bindings: &mut Bindings,
    sink: &mut S,
    latched: &mut u32,
) -> Result<()> {
    // Resolve every occurrence's filter templates for this execution
    // (literals latch through the shared interner exactly like the
    // resident path).
    let FallbackRule {
        occurrences,
        resolved,
        resolved_selections,
        selection_templates,
        ..
    } = rule;
    for (occ_idx, occurrence) in occurrences.iter_mut().enumerate() {
        if occurrence.role.discharged() {
            continue;
        }
        let negated = occurrence.role == Role::Negated;
        let filters = &mut resolved[occ_idx];
        if filters.len() != occurrence.filters.len() {
            filters.clear();
            filters.extend(occurrence.filters.iter().cloned());
        }
        for (template, slot) in occurrence.filters.iter_mut().zip(filters.iter_mut()) {
            if !super::bind::resolve_filter_admitted(
                ctx.interner,
                ctx.nonresident,
                ctx.source.work(),
                template,
                ctx.params,
                ctx.missed,
                negated,
                slot,
                latched,
            )? {
                return Ok(());
            }
        }
        let selections = &mut resolved_selections[occ_idx];
        let templates = &mut selection_templates[occ_idx];
        if selections.len() != templates.len() {
            selections.clear();
            selections.resize_with(templates.len(), Vec::new);
        }
        for (selection, words) in templates.iter_mut().zip(selections.iter_mut()) {
            if !super::bind::resolve_selection_into(
                ctx.interner,
                ctx.nonresident,
                ctx.source.work(),
                selection,
                ctx.params,
                ctx.missed,
                words,
                latched,
            )? {
                return Ok(());
            }
        }
    }

    let rule = &*rule;
    bindings.resize(rule.slot_count);
    let positives: Vec<usize> = rule
        .occurrences
        .iter()
        .enumerate()
        .filter(|(_, occurrence)| occurrence.role == Role::Positive)
        .map(|(idx, _)| idx)
        .collect();
    let mut state = Search {
        rule,
        ctx,
        published,
        rec_delta,
        rec_acc,
        positives,
        bound: vec![false; rule_slots(rule)],
        deferred_points: Vec::new(),
    };
    state.descend(0, bindings, sink)
}

fn rule_slots(rule: &FallbackRule) -> usize {
    rule.slot_count
}

struct Search<'a, 'c> {
    rule: &'a FallbackRule,
    ctx: &'a mut FallbackCtx<'c>,
    published: &'a mut [SealedStage],
    rec_delta: Option<&'a mut SealedStage>,
    rec_acc: Option<&'a mut SealedStage>,
    positives: Vec<usize>,
    bound: Vec<bool>,
    /// `(start, end, var, dense)` membership checks whose point variable
    /// was unbound when the interval row bound — verified at the leaf.
    deferred_points: Vec<(u64, u64, VarId, bool)>,
}

impl Search<'_, '_> {
    fn descend<S: Sink>(
        &mut self,
        depth: usize,
        bindings: &mut Bindings,
        sink: &mut S,
    ) -> Result<()> {
        let Some(&occ_idx) = self.positives.get(depth) else {
            return self.leaf(bindings, sink);
        };
        let occurrence = &self.rule.occurrences[occ_idx];
        match occurrence.bind {
            OccBind::Edb(relation) => self.scan_stored(depth, occ_idx, relation, bindings, sink),
            OccBind::Finished(_) | OccBind::RecDelta(_) | OccBind::RecAcc(_) => {
                self.scan_derived(depth, occ_idx, bindings, sink)
            }
        }
    }

    fn scan_stored<S: Sink>(
        &mut self,
        depth: usize,
        occ_idx: usize,
        relation: RelationId,
        bindings: &mut Bindings,
        sink: &mut S,
    ) -> Result<()> {
        let rel = self.ctx.schema.relation(relation);
        // A closed relation's rows come from the sealed extension image
        // (synthesized, RAM-resident by definition of the sealed cap).
        if rel.body().closed_rows().is_some() {
            let image = crate::image::synthesize_closed(relation, rel);
            return self.scan_image_rows(depth, occ_idx, &image, bindings, sink);
        }
        let selections = &self.rule.resolved_selections[occ_idx];
        if self.try_scan_stored_keyed(depth, occ_idx, relation, selections, bindings, sink)? {
            return Ok(());
        }
        let fields = rel.fields();
        let field_types: Vec<ValueType> = fields.iter().map(|f| f.value_type).collect();
        let mut row = RowWords::new(&field_types);
        let mut failure: Option<Error> = None;
        let source = self.ctx.source;
        let result = source.scan_early(relation, &mut |bytes| {
            if failure.is_some() {
                return Ok(false);
            }
            if let Err(error) = super::text::decode_row(
                &mut row,
                fields,
                bytes,
                self.ctx.interner,
                self.ctx.nonresident,
                self.ctx.source.work(),
                true,
            ) {
                failure = Some(error);
                return Ok(false);
            }
            match self.try_row(depth, occ_idx, &row, bindings, sink) {
                Ok(()) => Ok(true),
                Err(error) => {
                    failure = Some(error);
                    Ok(false)
                }
            }
        });
        result?;
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    /// Key-bound walk through the compiled witness. Missing compiled
    /// projections refuse this path (no full-scan key fallback).
    fn try_scan_stored_keyed<S: Sink>(
        &mut self,
        depth: usize,
        occ_idx: usize,
        relation: RelationId,
        selections: &[Vec<u64>],
        bindings: &mut Bindings,
        sink: &mut S,
    ) -> Result<bool> {
        if selections.is_empty() || !selections.iter().all(|level| level.len() == 1) {
            return Ok(false);
        }
        let occurrence = &self.rule.occurrences[occ_idx];
        let trie_schema = &self.rule.trie_schemas[occ_idx];
        if trie_schema.is_empty() {
            return Ok(false);
        }
        let key_words: Vec<u64> = selections.iter().map(|level| level[0]).collect();
        let key_fields: Vec<FieldId> = trie_schema[0]
            .iter()
            .filter_map(|var| {
                occurrence
                    .vars
                    .iter()
                    .find(|(_, v)| v == var)
                    .map(|(field, _)| *field)
            })
            .collect();
        if key_fields.len() != key_words.len() {
            return Ok(false);
        }
        let Ok(theory) = self.ctx.schema.compiled_theory() else {
            return Ok(false);
        };
        let Some(projection) = theory.key_projections_of(relation).iter().find_map(|id| {
            let compiled = theory.projection(*id)?;
            (compiled.projection.as_ref() == key_fields.as_slice()).then_some(compiled)
        }) else {
            return Ok(false);
        };
        let last_positive = self.positives.get(depth + 1).is_none();
        let all_bound = occurrence.vars.iter().all(|(_, var)| {
            let (slot, _, _) = self.rule.locate(*var);
            self.bound[slot]
        });
        let witness = if last_positive && all_bound {
            DistinctnessWitness::ExistenceOnly {
                projection: projection.id,
            }
        } else {
            match theory.distinctness_witness(projection.id) {
                Some(DistinctnessWitness::ScalarKeyUnique { projection }) => {
                    DistinctnessWitness::ScalarKeyUnique { projection }
                }
                Some(witness) => witness,
                None => return Ok(false),
            }
        };
        let rel = self.ctx.schema.relation(relation);
        let fields = rel.fields();
        let field_types: Vec<ValueType> = fields.iter().map(|f| f.value_type).collect();
        let mut row = RowWords::new(&field_types);
        let mut failure: Option<Error> = None;
        let unique = matches!(witness, DistinctnessWitness::ScalarKeyUnique { .. });
        let existence = matches!(witness, DistinctnessWitness::ExistenceOnly { .. });
        let visited = self.ctx.source.consume_compiled_visits(
            self.ctx.schema,
            relation,
            witness,
            &key_fields,
            &key_words,
            &mut |bytes| {
                if failure.is_some() {
                    return Ok(VisitControl::Stop);
                }
                if let Err(error) = super::text::decode_row(
                    &mut row,
                    fields,
                    bytes,
                    self.ctx.interner,
                    self.ctx.nonresident,
                    self.ctx.source.work(),
                    true,
                ) {
                    failure = Some(error);
                    return Ok(VisitControl::Stop);
                }
                match self.try_row(depth, occ_idx, &row, bindings, sink) {
                    Ok(()) if existence || unique => Ok(VisitControl::Sufficient),
                    Ok(()) => Ok(VisitControl::Continue),
                    Err(error) => {
                        failure = Some(error);
                        Ok(VisitControl::Stop)
                    }
                }
            },
        )?;
        match (visited, failure) {
            (_, Some(error)) => Err(error),
            (None, None) => Ok(false),
            (Some(VisitOutcome::Stopped { .. }), None) => Ok(true),
            (Some(_), None) => Ok(true),
        }
    }

    fn scan_derived<S: Sink>(
        &mut self,
        depth: usize,
        occ_idx: usize,
        bindings: &mut Bindings,
        sink: &mut S,
    ) -> Result<()> {
        let bind = self.rule.occurrences[occ_idx].bind;
        match bind {
            OccBind::Edb(_) => Err(Error::Corruption(
                crate::error::CorruptionError::MalformedValue("fallback derived source"),
            )),
            OccBind::RecDelta(_) | OccBind::RecAcc(_) => {
                self.scan_rec_stage(depth, occ_idx, bind, bindings, sink)
            }
            OccBind::Finished(id) => self.scan_scratch_stage_at(depth, occ_idx, id.index(), bindings, sink),
        }
    }

    fn scan_rec_stage<S: Sink>(
        &mut self,
        depth: usize,
        occ_idx: usize,
        bind: OccBind,
        bindings: &mut Bindings,
        sink: &mut S,
    ) -> Result<()> {
        let stage = match bind {
            OccBind::RecDelta(_) => self.rec_delta.as_mut(),
            OccBind::RecAcc(_) => self.rec_acc.as_mut(),
            _ => None,
        };
        let Some(stage) = stage else {
            return Err(Error::Corruption(
                crate::error::CorruptionError::MalformedValue("fallback derived source"),
            ));
        };
        match stage {
            SealedStage::Resident(image) => {
                let image = Arc::clone(image);
                self.scan_image_rows(depth, occ_idx, &image, bindings, sink)
            }
            SealedStage::Scratch(scratch) => {
                let count = scratch.count;
                let row_words = scratch.row_words;
                let field_types = scratch.field_types.clone();
                for index in 0..count {
                    self.ctx
                        .source
                        .work()
                        .step(1)
                        .map_err(super::source::work_error)?;
                    let mut words = vec![0u64; row_words];
                    super::derived::SealedStage::scratch_row_words(scratch, index, &mut words)?;
                    let row = ScratchRow {
                        field_types: &field_types,
                        words: &words,
                    };
                    self.try_row(depth, occ_idx, &row, bindings, sink)?;
                }
                Ok(())
            }
        }
    }

    fn scan_scratch_stage_at<S: Sink>(
        &mut self,
        depth: usize,
        occ_idx: usize,
        stage_idx: usize,
        bindings: &mut Bindings,
        sink: &mut S,
    ) -> Result<()> {
        match self.published.get(stage_idx) {
            Some(SealedStage::Resident(image)) => {
                let image = Arc::clone(image);
                self.scan_image_rows(depth, occ_idx, &image, bindings, sink)
            }
            Some(SealedStage::Scratch(stage)) => {
                let count = stage.count;
                let row_words = stage.row_words;
                let field_types = stage.field_types.clone();
                for index in 0..count {
                    self.ctx
                        .source
                        .work()
                        .step(1)
                        .map_err(super::source::work_error)?;
                    let mut words = vec![0u64; row_words];
                    {
                        let scratch = match &mut self.published[stage_idx] {
                            SealedStage::Scratch(scratch) => scratch,
                            _ => unreachable!("scratch stage"),
                        };
                        super::derived::SealedStage::scratch_row_words(scratch, index, &mut words)?;
                    }
                    let row = ScratchRow {
                        field_types: &field_types,
                        words: &words,
                    };
                    self.try_row(depth, occ_idx, &row, bindings, sink)?;
                }
                Ok(())
            }
            None => Err(Error::Corruption(
                crate::error::CorruptionError::MalformedValue("fallback derived source"),
            )),
        }
    }

    fn scan_image_rows<S: Sink>(
        &mut self,
        depth: usize,
        occ_idx: usize,
        image: &RelationImage,
        bindings: &mut Bindings,
        sink: &mut S,
    ) -> Result<()> {
        let work = self.ctx.source.work();
        for position in 0..image.row_count() {
            work.step(1).map_err(super::source::work_error)?;
            let row = ImageRow { image, position };
            self.try_row(depth, occ_idx, &row, bindings, sink)?;
        }
        Ok(())
    }

    /// One candidate row of the occurrence at `depth`: filters, var
    /// unification, membership probes, then recurse. Restores the binding
    /// mask on the way out.
    fn try_row<S: Sink, O: Operands>(
        &mut self,
        depth: usize,
        occ_idx: usize,
        row: &O,
        bindings: &mut Bindings,
        sink: &mut S,
    ) -> Result<()>
    where
        Error: From<O::Error>,
    {
        let occurrence = &self.rule.occurrences[occ_idx];
        for filter in &self.rule.resolved[occ_idx] {
            if !super::text::holds_with_text(
                filter,
                row,
                self.ctx.params,
                self.ctx.interner,
                self.ctx.nonresident,
            )?
            .unwrap_or(false)
            {
                return Ok(());
            }
        }

        // Unify the occurrence's variables against the binding stack.
        let mut newly_bound: Vec<usize> = Vec::new();
        let mut matched = true;
        let mut span_words = [0u64; 8];
        for (field, var) in &occurrence.vars {
            let (slot, width, _) = self.rule.locate(*var);
            let count = load_span(row, *field, width, &mut span_words)?;
            debug_assert_eq!(count, width, "slot width equals field span width");
            if self.bound[slot] {
                if !slot_words_equal(
                    self.rule,
                    self.ctx.interner,
                    self.ctx.nonresident,
                    bindings,
                    slot,
                    width,
                    &span_words,
                )? {
                    matched = false;
                    break;
                }
            } else {
                for (w, word) in span_words.iter().enumerate().take(width) {
                    bindings.set(slot + w, *word);
                    self.bound[slot + w] = true;
                }
                newly_bound.push(slot);
            }
        }

        let deferred_before = self.deferred_points.len();
        if matched {
            // Var-sourced point membership: check now when the point var
            // is bound, defer to the leaf otherwise.
            for (field, var, dense) in &occurrence.point_vars {
                let (start, end) = pair_span(row, *field)?;
                let (slot, _, _) = self.rule.locate(*var);
                if self.bound[slot] {
                    let point = crate::image::view::element_probe_word(*dense, bindings.get(slot));
                    if !(start <= point && point < end) {
                        matched = false;
                        break;
                    }
                } else {
                    self.deferred_points.push((start, end, *var, *dense));
                }
            }
        }

        if matched {
            self.descend(depth + 1, bindings, sink)?;
        }

        // Unwind this row's bindings and deferrals.
        self.deferred_points.truncate(deferred_before);
        for slot in newly_bound {
            let (_, width) = self
                .rule
                .slots
                .iter()
                .find(|(_, s, ..)| *s == slot)
                .map(|(_, s, w, _)| (*s, *w))
                .expect("newly bound slots come from the layout");
            for w in 0..width {
                self.bound[slot + w] = false;
            }
        }
        Ok(())
    }

    /// All positive atoms bound: deferred points, residuals, negation,
    /// then emit.
    fn leaf<S: Sink>(&mut self, bindings: &mut Bindings, sink: &mut S) -> Result<()> {
        for (start, end, var, dense) in &self.deferred_points {
            let (slot, _, _) = self.rule.locate(*var);
            debug_assert!(self.bound[slot], "leaf: every rule variable is bound");
            let point = crate::image::view::element_probe_word(*dense, bindings.get(slot));
            if !(*start <= point && point < *end) {
                return Ok(());
            }
        }
        let ops = BindingOps {
            rule: self.rule,
            bindings,
        };
        for residual in self
            .rule
            .residuals
            .iter()
            .chain(&self.rule.word_residuals)
            .chain(&self.rule.allen_residuals)
        {
            if !super::text::holds_with_text(
                residual,
                &ops,
                self.ctx.params,
                self.ctx.interner,
                self.ctx.nonresident,
            )?
            .unwrap_or(false)
            {
                return Ok(());
            }
        }
        // Negated atoms: any matching stored/derived row rejects.
        for probe in &self.rule.anti_probes {
            let occ_idx = usize::from(probe.occurrence.0);
            if self.negated_hit(occ_idx, &probe.probe_bindings, bindings)? {
                return Ok(());
            }
        }
        self.canonicalize_text_bindings(bindings)?;
        let emitted = sink.emit(bindings);
        let progress = Flow::from_sink_progress(sink.progress()).or_skip(emitted);
        if progress.is_terminal() {
            return Err(sink.take_error().unwrap_or_else(|| {
                Error::Corruption(crate::error::CorruptionError::MalformedValue(
                    "fallback sink stop",
                ))
            }));
        }
        Ok(())
    }

    fn canonicalize_text_bindings(&self, bindings: &mut Bindings) -> Result<()> {
        for (slot, is_text) in self.rule.text_slots.iter().enumerate() {
            if !*is_text {
                continue;
            }
            if let Some(canon) = super::text::canonical_token(
                self.ctx.interner,
                self.ctx.nonresident.as_ref(),
                bindings.get(slot),
            )? {
                bindings.set(slot, canon);
            }
        }
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the anti-probe's bound/deferred arms read as one walk"
    )]
    fn negated_hit(
        &mut self,
        occ_idx: usize,
        probe_bindings: &[(FieldId, VarId)],
        bindings: &Bindings,
    ) -> Result<bool> {
        let occurrence = &self.rule.occurrences[occ_idx];
        debug_assert_eq!(
            occurrence.role,
            Role::Negated,
            "anti-probes name negated atoms"
        );
        let mut hit = false;
        let check_row = |row: &dyn ErasedOperands,
                         store: &mut Option<crate::image::NonresidentTextStore>|
         -> Result<bool> {
            for filter in &self.rule.resolved[occ_idx] {
                if !row
                    .holds_filter(
                        filter,
                        self.ctx.params,
                        self.ctx.interner,
                        store,
                    )?
                    .unwrap_or(false)
                {
                    return Ok(false);
                }
            }
            let mut span_words = [0u64; 8];
            for (field, var) in probe_bindings {
                let (slot, width, _) = self.rule.locate(*var);
                let count = row.span(*field, &mut span_words)?;
                debug_assert_eq!(count, width, "probe width equals span width");
                if !slot_words_equal(
                    self.rule,
                    self.ctx.interner,
                    store,
                    bindings,
                    slot,
                    width,
                    &span_words,
                )? {
                    return Ok(false);
                }
            }
            for (field, var, dense) in &occurrence.point_vars {
                let mut pair = [0u64; 8];
                row.span(*field, &mut pair)?;
                let (slot, _, _) = self.rule.locate(*var);
                let point = crate::image::view::element_probe_word(*dense, bindings.get(slot));
                if !(pair[0] <= point && point < pair[1]) {
                    return Ok(false);
                }
            }
            Ok(true)
        };
        match occurrence.bind {
            OccBind::Edb(relation) => {
                let rel = self.ctx.schema.relation(relation);
                if rel.body().closed_rows().is_some() {
                    let image = crate::image::synthesize_closed(relation, rel);
                    for position in 0..image.row_count() {
                        self.ctx
                            .source
                            .work()
                            .step(1)
                            .map_err(super::source::work_error)?;
                        let row = ImageRow {
                            image: &image,
                            position,
                        };
                        if check_row(&row, self.ctx.nonresident)? {
                            hit = true;
                            break;
                        }
                    }
                    return Ok(hit);
                }
                let fields = rel.fields();
                let field_types: Vec<ValueType> = fields.iter().map(|f| f.value_type).collect();
                let mut row = RowWords::new(&field_types);
                let mut failure: Option<Error> = None;
                self.ctx.source.scan_early(relation, &mut |bytes| {
                    if hit || failure.is_some() {
                        return Ok(false);
                    }
                    if let Err(error) = super::text::decode_row(
                        &mut row,
                        fields,
                        bytes,
                        self.ctx.interner,
                        self.ctx.nonresident,
                        self.ctx.source.work(),
                        false,
                    ) {
                        failure = Some(error);
                        return Ok(false);
                    }
                    match check_row(&row, self.ctx.nonresident) {
                        Ok(matched) => hit |= matched,
                        Err(error) => {
                            failure = Some(error);
                            return Ok(false);
                        }
                    }
                    Ok(!hit)
                })?;
                if let Some(error) = failure {
                    return Err(error);
                }
                Ok(hit)
            }
            OccBind::Finished(id) => {
                let stage_idx = id.index();
                match self.published.get(stage_idx) {
                    Some(SealedStage::Resident(image)) => {
                        for position in 0..image.row_count() {
                            self.ctx
                                .source
                                .work()
                                .step(1)
                                .map_err(super::source::work_error)?;
                            let row = ImageRow { image, position };
                            if check_row(&row, self.ctx.nonresident)? {
                                hit = true;
                                break;
                            }
                        }
                    }
                    Some(SealedStage::Scratch(stage)) => {
                        let count = stage.count;
                        let row_words = stage.row_words;
                        let field_types = stage.field_types.clone();
                        for index in 0..count {
                            self.ctx
                                .source
                                .work()
                                .step(1)
                                .map_err(super::source::work_error)?;
                            let mut words = vec![0u64; row_words];
                            {
                                let scratch = match &mut self.published[stage_idx] {
                                    SealedStage::Scratch(scratch) => scratch,
                                    _ => unreachable!("scratch stage"),
                                };
                                super::derived::SealedStage::scratch_row_words(
                                    scratch,
                                    index,
                                    &mut words,
                                )?;
                            }
                            let row = ScratchRow {
                                field_types: &field_types,
                                words: &words,
                            };
                            if check_row(&row, self.ctx.nonresident)? {
                                hit = true;
                                break;
                            }
                        }
                    }
                    None => {
                        return Err(Error::Corruption(
                            crate::error::CorruptionError::MalformedValue("fallback derived source"),
                        ));
                    }
                }
                Ok(hit)
            }
            OccBind::RecDelta(_) | OccBind::RecAcc(_) => {
                let Some(resolved) = resolve_derived(
                    self.published,
                    self.rec_delta.as_deref_mut(),
                    self.rec_acc.as_deref_mut(),
                    occurrence.bind,
                ) else {
                    return Err(Error::Corruption(
                        crate::error::CorruptionError::MalformedValue("fallback derived source"),
                    ));
                };
                match resolved {
                    SealedStageRef::Resident(image) => {
                        for position in 0..image.row_count() {
                            self.ctx
                                .source
                                .work()
                                .step(1)
                                .map_err(super::source::work_error)?;
                            let row = ImageRow { image, position };
                            if check_row(&row, self.ctx.nonresident)? {
                                hit = true;
                                break;
                            }
                        }
                    }
                    SealedStageRef::Stage(SealedStage::Resident(image)) => {
                        for position in 0..image.row_count() {
                            self.ctx
                                .source
                                .work()
                                .step(1)
                                .map_err(super::source::work_error)?;
                            let row = ImageRow {
                                image,
                                position,
                            };
                            if check_row(&row, self.ctx.nonresident)? {
                                hit = true;
                                break;
                            }
                        }
                    }
                    SealedStageRef::Stage(SealedStage::Scratch(scratch)) => {
                        let count = scratch.count;
                        let row_words = scratch.row_words;
                        let field_types = scratch.field_types.clone();
                        for index in 0..count {
                            self.ctx
                                .source
                                .work()
                                .step(1)
                                .map_err(super::source::work_error)?;
                            let mut words = vec![0u64; row_words];
                            super::derived::SealedStage::scratch_row_words(
                                scratch,
                                index,
                                &mut words,
                            )?;
                            let row = ScratchRow {
                                field_types: &field_types,
                                words: &words,
                            };
                            if check_row(&row, self.ctx.nonresident)? {
                                hit = true;
                                break;
                            }
                        }
                    }
                }
                Ok(hit)
            }
        }
    }
}

/// Object-safe row access for the negation walker (two operand providers
/// behind one loop).
trait ErasedOperands {
    fn holds_filter(
        &self,
        filter: &FilterPredicate,
        params: &[Const],
        interner: &InternerHandle<'_>,
        store: &mut Option<crate::image::NonresidentTextStore>,
    ) -> Result<Option<bool>>;
    fn span(&self, field: FieldId, out: &mut [u64; 8]) -> Result<usize>;
}

impl ErasedOperands for RowWords {
    fn holds_filter(
        &self,
        filter: &FilterPredicate,
        params: &[Const],
        interner: &InternerHandle<'_>,
        store: &mut Option<crate::image::NonresidentTextStore>,
    ) -> Result<Option<bool>> {
        super::text::holds_with_text(filter, self, params, interner, store)
    }

    fn span(&self, field: FieldId, out: &mut [u64; 8]) -> Result<usize> {
        let words = self.span_words(field);
        out[..words.len()].copy_from_slice(words);
        Ok(words.len())
    }
}

impl ErasedOperands for ImageRow<'_> {
    fn holds_filter(
        &self,
        filter: &FilterPredicate,
        params: &[Const],
        interner: &InternerHandle<'_>,
        store: &mut Option<crate::image::NonresidentTextStore>,
    ) -> Result<Option<bool>> {
        match super::text::holds_with_text(filter, self, params, interner, store) {
            Ok(verdict) => Ok(verdict),
            Err(error) => Err(error),
        }
    }

    fn span(&self, field: FieldId, out: &mut [u64; 8]) -> Result<usize> {
        Ok(match self.loaded(field.into()) {
            Ok(Loaded::Word(w)) => {
                out[0] = w;
                1
            }
            Ok(Loaded::Byte(b)) => {
                out[0] = u64::from(b);
                1
            }
            Ok(Loaded::Pair(s, e)) => {
                out[0] = s;
                out[1] = e;
                2
            }
            Ok(Loaded::Block { words, count }) => {
                let count = usize::from(count);
                out[..count].copy_from_slice(&words[..count]);
                count
            }
            Err(infallible) => match infallible {},
        })
    }
}

impl ErasedOperands for ScratchRow<'_> {
    fn holds_filter(
        &self,
        filter: &FilterPredicate,
        params: &[Const],
        interner: &InternerHandle<'_>,
        store: &mut Option<crate::image::NonresidentTextStore>,
    ) -> Result<Option<bool>> {
        super::text::holds_with_text(filter, self, params, interner, store)
    }

    fn span(&self, field: FieldId, out: &mut [u64; 8]) -> Result<usize> {
        let words = self.span_words(field);
        out[..words.len()].copy_from_slice(words);
        Ok(words.len())
    }
}

fn load_span<O: Operands>(
    row: &O,
    field: FieldId,
    width: usize,
    out: &mut [u64; 8],
) -> Result<usize>
where
    Error: From<O::Error>,
{
    match row.loaded(field.into()).map_err(Error::from)? {
        Loaded::Word(w) => {
            out[0] = w;
            Ok(1)
        }
        Loaded::Byte(b) => {
            out[0] = u64::from(b);
            Ok(1)
        }
        Loaded::Pair(s, e) => {
            out[0] = s;
            out[1] = e;
            Ok(2)
        }
        Loaded::Block { words, count } => {
            let count = usize::from(count);
            out[..count].copy_from_slice(&words[..count]);
            Ok(count)
        }
    }
    .inspect(|count| {
        debug_assert_eq!(*count, width, "span width equals slot width");
    })
}

fn pair_span<O: Operands>(row: &O, field: FieldId) -> Result<(u64, u64)>
where
    Error: From<O::Error>,
{
    match row.loaded(field.into()).map_err(Error::from)? {
        Loaded::Pair(start, end) => Ok((start, end)),
        Loaded::Word(_) | Loaded::Byte(_) | Loaded::Block { .. } => {
            unreachable!("validated: point membership reads interval fields")
        }
    }
}

/// Residual operands over the bound slots: the normalize-produced residual
/// rosters address variables (`OperandAddr::var_word` / `From<VarId>`),
/// loaded through the plan's slot layout.
struct BindingOps<'a> {
    rule: &'a FallbackRule,
    bindings: &'a Bindings,
}

impl BindingOps<'_> {
    fn var_loaded(&self, at: OperandAddr) -> Loaded {
        if at.width() == 1 {
            // A word residual addresses one slot word (var + offset), the
            // `place_comparisons` lowering.
            let (slot, _, _) = self.rule.locate(at.var());
            return Loaded::Word(self.bindings.get(slot + at.offset()));
        }
        let (slot, width, kind) = self.rule.locate(at.var());
        match kind {
            LoadKind::Word => Loaded::Word(self.bindings.get(slot)),
            LoadKind::Pair => Loaded::Pair(self.bindings.get(slot), self.bindings.get(slot + 1)),
            LoadKind::Block(count) => {
                let mut words = [0u64; 8];
                for (w, word) in words.iter_mut().enumerate().take(width) {
                    *word = self.bindings.get(slot + w);
                }
                Loaded::Block { words, count }
            }
        }
    }
}

impl Operands for BindingOps<'_> {
    type Error = Error;

    fn word(&self, at: OperandAddr) -> Result<u64> {
        match self.var_loaded(at) {
            Loaded::Word(w) => Ok(w),
            Loaded::Byte(b) => Ok(u64::from(b)),
            Loaded::Pair(..) | Loaded::Block { .. } => {
                unreachable!("validated: word residuals address scalar words")
            }
        }
    }

    fn pair(&self, at: OperandAddr) -> Result<(u64, u64)> {
        let (slot, _, _) = self.rule.locate(at.var());
        Ok((self.bindings.get(slot), self.bindings.get(slot + 1)))
    }

    fn block(&self, at: OperandAddr) -> Result<([u64; 8], u8)> {
        match self.var_loaded(at) {
            Loaded::Block { words, count } => Ok((words, count)),
            Loaded::Word(_) | Loaded::Byte(_) | Loaded::Pair(..) => {
                unreachable!("validated: block residuals address bytes<N> variables")
            }
        }
    }

    fn loaded(&self, at: OperandAddr) -> Result<Loaded> {
        Ok(self.var_loaded(at))
    }

    fn string_field(&self, at: OperandAddr) -> bool {
        let (slot, _, _) = self.rule.locate(at.var());
        self.rule.text_slots.get(slot).copied().unwrap_or(false)
    }
}

/// Flat column words for one scratch-backed derived row.
struct ScratchRow<'a> {
    field_types: &'a [ValueType],
    words: &'a [u64],
}

impl Operands for ScratchRow<'_> {
    type Error = Error;

    fn word(&self, at: OperandAddr) -> Result<u64> {
        Ok(match self.operand(at.field()) {
            crate::exec::dispatch::FactOperand::Word(w) => w,
            crate::exec::dispatch::FactOperand::Pair(..)
            | crate::exec::dispatch::FactOperand::Block { .. } => {
                unreachable!("validated: word operands are scalar fields")
            }
        })
    }

    fn pair(&self, at: OperandAddr) -> Result<(u64, u64)> {
        Ok(match self.operand(at.field()) {
            crate::exec::dispatch::FactOperand::Pair(s, e) => (s, e),
            crate::exec::dispatch::FactOperand::Word(_)
            | crate::exec::dispatch::FactOperand::Block { .. } => {
                unreachable!("validated: interval predicates read interval fields")
            }
        })
    }

    fn block(&self, at: OperandAddr) -> Result<([u64; 8], u8)> {
        Ok(match self.operand(at.field()) {
            crate::exec::dispatch::FactOperand::Block { words, count } => (words, count),
            crate::exec::dispatch::FactOperand::Word(_)
            | crate::exec::dispatch::FactOperand::Pair(..) => {
                unreachable!("validated: block operands are bytes<N>")
            }
        })
    }

    fn loaded(&self, at: OperandAddr) -> Result<Loaded> {
        Ok(match self.operand(at.field()) {
            crate::exec::dispatch::FactOperand::Word(w) => Loaded::Word(w),
            crate::exec::dispatch::FactOperand::Pair(s, e) => Loaded::Pair(s, e),
            crate::exec::dispatch::FactOperand::Block { words, count } => Loaded::Block {
                words,
                count,
            },
        })
    }

    fn string_field(&self, at: OperandAddr) -> bool {
        self.field_types
            .get(usize::from(at.field().0))
            .is_some_and(|ty| matches!(ty, ValueType::String))
    }
}

impl ScratchRow<'_> {
    fn span_words(&self, field: FieldId) -> &[u64] {
        let spans = crate::image::column_spans(self.field_types);
        let span = spans[usize::from(field.0)];
        let first = usize::from(span.first_column);
        match span.width {
            crate::image::ColumnWidth::Byte | crate::image::ColumnWidth::Word => {
                &self.words[first..first + 1]
            }
            crate::image::ColumnWidth::WordPair => &self.words[first..first + 2],
            crate::image::ColumnWidth::Words { count } => {
                &self.words[first..first + usize::from(count)]
            }
        }
    }

    fn operand(&self, field: FieldId) -> crate::exec::dispatch::FactOperand {
        let spans = crate::image::column_spans(self.field_types);
        let span = spans[usize::from(field.0)];
        let first = usize::from(span.first_column);
        match span.width {
            crate::image::ColumnWidth::Byte | crate::image::ColumnWidth::Word => {
                crate::exec::dispatch::FactOperand::Word(self.words[first])
            }
            crate::image::ColumnWidth::WordPair => crate::exec::dispatch::FactOperand::Pair(
                self.words[first],
                self.words[first + 1],
            ),
            crate::image::ColumnWidth::Words { count } => {
                let count = usize::from(count);
                let mut words = [0u64; 8];
                words[..count].copy_from_slice(&self.words[first..first + count]);
                crate::exec::dispatch::FactOperand::Block {
                    words,
                    count: u8::try_from(count).expect("bytes width is at most 8 words"),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exec::run::Bindings;
    use crate::image::view::{OperandAddr, Operands};
    use crate::ir::VarId;
    use bumbledb_theory::schema::{FieldId, ValueType};

    /// String columns take TextEq; i64 high-bit words do not.
    /// Verification: NotRun.
    #[test]
    fn fallback_operands_mark_string_columns() {
        let field_types = [ValueType::U64, ValueType::String, ValueType::I64];
        let words = [7u64, 0, 1u64 << 63];
        let row = ScratchRow {
            field_types: &field_types,
            words: &words,
        };
        assert!(!row.string_field(OperandAddr::from(FieldId(0))));
        assert!(row.string_field(OperandAddr::from(FieldId(1))));
        assert!(!row.string_field(OperandAddr::from(FieldId(2))));

        let rule = FallbackRule {
            occurrences: Vec::new(),
            residuals: Vec::new(),
            word_residuals: Vec::new(),
            allen_residuals: Vec::new(),
            anti_probes: Vec::new(),
            slots: vec![
                (VarId(0), 0, 1, LoadKind::Word),
                (VarId(1), 1, 1, LoadKind::Word),
            ],
            text_slots: vec![false, true],
            slot_count: 2,
            resolved: Vec::new(),
            resolved_selections: Vec::new(),
            selection_templates: Vec::new(),
            trie_schemas: Vec::new(),
        };
        let bindings = Bindings::new(2);
        let ops = BindingOps {
            rule: &rule,
            bindings: &bindings,
        };
        assert!(!ops.string_field(OperandAddr::from(VarId(0))));
        assert!(ops.string_field(OperandAddr::from(VarId(1))));
    }
}
