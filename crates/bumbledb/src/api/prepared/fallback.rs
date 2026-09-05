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

use super::source::QuerySource;
use crate::error::{Error, Result};
use crate::exec::run::{Bindings, Sink};
use crate::image::RelationImage;
use crate::image::canon::{RowWords, TextWords};
use crate::image::intern::InternerHandle;
use crate::image::view::{Const, FilterPredicate, ImageRow, Loaded, OperandAddr, Operands, holds};
use crate::ir::VarId;
use crate::ir::normalize::{AntiProbe, OccBind, Occurrence, Role};
use crate::schema::Schema;
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
    slot_count: usize,
    /// Per occurrence: this execution's resolved filters (params/literals
    /// substituted); refilled by `resolve` before every run.
    resolved: Vec<Vec<FilterPredicate>>,
}

impl FallbackRule {
    pub(super) fn seal(
        normalized: &crate::ir::normalize::NormalizedQuery,
        plan: &crate::plan::fj::ValidatedPlan,
        var_type: impl Fn(VarId) -> ValueType,
    ) -> Self {
        let slots = plan
            .slot_spans()
            .into_iter()
            .map(|(var, slot, width)| {
                let kind = match var_type(var) {
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
        Self {
            occurrences,
            residuals: normalized.residuals.clone(),
            word_residuals: normalized.word_residuals.clone(),
            allen_residuals: normalized.allen_residuals.clone(),
            anti_probes: normalized.anti_probes.clone(),
            slots,
            slot_count: plan.slot_count(),
            resolved,
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

/// Everything one fallback run reads.
pub(super) struct FallbackCtx<'a> {
    pub(super) source: &'a QuerySource<'a>,
    pub(super) schema: &'a Schema,
    pub(super) interner: &'a InternerHandle<'a>,
    pub(super) params: &'a [Const],
    pub(super) missed: &'a [bool],
    /// Derived sources: interior/rec tables by the occurrence's bind.
    pub(super) derived: &'a dyn Fn(OccBind) -> Option<Arc<RelationImage>>,
}

/// Run one rule through the cursor fallback into `sink`.
/// # Errors
/// Storage/work failure, corrupt stored bytes, or a latch refusal. A
/// short-circuited resolution (empty set under positive `Eq`) is `Ok` —
/// the rule contributes nothing on this snapshot.
pub(super) fn run_fallback<S: Sink>(
    rule: &mut FallbackRule,
    ctx: &FallbackCtx<'_>,
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
        ..
    } = &mut *rule;
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
            if !crate::image::view::resolve_filter_into(
                ctx.interner,
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
    ctx: &'a FallbackCtx<'c>,
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
        let fields = rel.fields();
        let field_types: Vec<ValueType> = fields.iter().map(|f| f.value_type).collect();
        let mut row = RowWords::new(&field_types);
        let mut failure: Option<Error> = None;
        // The borrow shape wants one closure; errors tunnel out.
        let mut visit = |bytes: &[u8],
                         search: &mut Self,
                         bindings: &mut Bindings,
                         sink: &mut S|
         -> Result<()> {
            let mut text = TextWords::HandleIntern(search.ctx.interner);
            row.decode(fields, bytes, &mut text)?;
            search.try_row(depth, occ_idx, &row, bindings, sink)
        };
        // `QuerySource::scan` drives the iteration; descend re-enters from
        // inside, so the source must tolerate reentrant scans (multiple
        // LMDB read cursors on one transaction, and slice walks, both do).
        let source = self.ctx.source;
        let result = source.scan(relation, &mut |bytes| {
            if failure.is_some() {
                return Ok(());
            }
            // SAFETY of the self-reborrow dance: `visit` needs `self`,
            // `bindings`, `sink` — all disjoint from the scan's own state.
            match visit(bytes, self, bindings, sink) {
                Ok(()) => Ok(()),
                Err(error) => {
                    failure = Some(error);
                    Ok(())
                }
            }
        });
        result?;
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
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
        let image = (self.ctx.derived)(bind).ok_or(Error::Corruption(
            crate::error::CorruptionError::MalformedValue("fallback derived source"),
        ))?;
        self.scan_image_rows(depth, occ_idx, &image, bindings, sink)
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
            if !holds(filter, row, self.ctx.params)
                .map_err(Error::from)?
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
                if (0..width).any(|w| bindings.get(slot + w) != span_words[w]) {
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
            if !holds(residual, &ops, self.ctx.params)?.unwrap_or(false) {
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
        let _ = sink.emit(bindings);
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the anti-probe's bound/deferred arms read as one walk"
    )]
    fn negated_hit(
        &self,
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
        let check_row = |row: &dyn ErasedOperands| -> Result<bool> {
            for filter in &self.rule.resolved[occ_idx] {
                if !row.holds_filter(filter, self.ctx.params)?.unwrap_or(false) {
                    return Ok(false);
                }
            }
            let mut span_words = [0u64; 8];
            for (field, var) in probe_bindings {
                let (slot, width, _) = self.rule.locate(*var);
                let count = row.span(*field, &mut span_words)?;
                debug_assert_eq!(count, width, "probe width equals span width");
                if (0..width).any(|w| bindings.get(slot + w) != span_words[w]) {
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
                        if check_row(&row)? {
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
                self.ctx.source.scan(relation, &mut |bytes| {
                    if hit || failure.is_some() {
                        return Ok(());
                    }
                    let mut text = TextWords::HandleLookup(self.ctx.interner);
                    if let Err(error) = row.decode(fields, bytes, &mut text) {
                        failure = Some(error);
                        return Ok(());
                    }
                    match check_row(&row) {
                        Ok(matched) => hit |= matched,
                        Err(error) => failure = Some(error),
                    }
                    Ok(())
                })?;
                if let Some(error) = failure {
                    return Err(error);
                }
                Ok(hit)
            }
            OccBind::Finished(_) | OccBind::RecDelta(_) | OccBind::RecAcc(_) => {
                let image = (self.ctx.derived)(occurrence.bind).ok_or(Error::Corruption(
                    crate::error::CorruptionError::MalformedValue("fallback derived source"),
                ))?;
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
                    if check_row(&row)? {
                        hit = true;
                        break;
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
    fn holds_filter(&self, filter: &FilterPredicate, params: &[Const]) -> Result<Option<bool>>;
    fn span(&self, field: FieldId, out: &mut [u64; 8]) -> Result<usize>;
}

impl ErasedOperands for RowWords {
    fn holds_filter(&self, filter: &FilterPredicate, params: &[Const]) -> Result<Option<bool>> {
        holds(filter, self, params)
    }

    fn span(&self, field: FieldId, out: &mut [u64; 8]) -> Result<usize> {
        let words = self.span_words(field);
        out[..words.len()].copy_from_slice(words);
        Ok(words.len())
    }
}

impl ErasedOperands for ImageRow<'_> {
    fn holds_filter(&self, filter: &FilterPredicate, params: &[Const]) -> Result<Option<bool>> {
        match holds(filter, self, params) {
            Ok(verdict) => Ok(verdict),
            Err(infallible) => match infallible {},
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
}
