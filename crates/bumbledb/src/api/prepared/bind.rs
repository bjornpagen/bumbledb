use super::{
    BindValue, Const, Executor, FilterPredicate, ParamArg, ParamSpec, PreparedQuery, PreparedRule,
    ValueType,
};

use crate::error::{Error, Result};
use crate::image::view::{MaskConst, ResolvedWordSource};
use crate::ir::{CmpOp, ParamId, Value};
use crate::obs;
use crate::storage::dict;
use crate::storage::env::ReadTxn;
use bumbledb_theory::schema::IntervalElement;

impl<S> PreparedQuery<'_, S> {
    /// Rebuilds the executor scratch at a different batch size — the
    /// tuning/test surface for D4's measurement-owned constant. Allocation
    /// happens here, outside any measured window. A no-op for key_probe
    /// probes. Hidden: a measurement affordance, not a knob on the
    /// no-knobs surface (`docs/architecture/00-product.md`).
    #[doc(hidden)]
    pub fn set_batch_size(&mut self, batch: usize) {
        for rule in self.program.all_rules_mut() {
            match rule {
                PreparedRule::FreeJoin(rule) => {
                    rule.executor = Executor::with_batch_size(&rule.plan, batch);
                }
                PreparedRule::Recursive(rule) => {
                    for variant in &mut rule.variants {
                        variant.rule.executor =
                            Executor::with_batch_size(&variant.rule.plan, batch);
                    }
                }
                PreparedRule::KeyProbe(_) => {}
            }
        }
    }

    /// The identity check at every execution entry (`execute` and
    /// `profile`; `execute_collect` and `introspect` route through them):
    /// a snapshot of any environment other than the preparing one is a
    /// typed error before anything else runs. One u64 compare — with the
    /// entry protected, the view memo needs no environment epoch in its
    /// generation keys.
    pub(super) fn check_snapshot(&self, txn: &ReadTxn<'_>) -> Result<()> {
        if txn.env_instance() == self.env_instance {
            Ok(())
        } else {
            Err(Error::ForeignPreparedQuery)
        }
    }

    /// Binds and converts all-scalar parameters (the `&[BindValue]`
    /// entry; a set-typed param rejects the scalar shape with
    /// [`Error::ParamSetExpected`] — the mixed entry is
    /// [`PreparedQuery::bind_param_args`]).
    pub(super) fn bind_params(
        &mut self,
        txn: &ReadTxn<'_>,
        params: &[BindValue<'_>],
    ) -> Result<()> {
        self.begin_bind(params.len())?;
        for (idx, value) in params.iter().enumerate() {
            self.bind_scalar_slot(txn, idx, *value)?;
        }
        Ok(())
    }

    /// Binds mixed scalar/set parameter arguments (the public
    /// [`ParamArg`] entry — `docs/architecture/70-api.md` § facts and
    /// results).
    pub(crate) fn bind_param_args(
        &mut self,
        txn: &ReadTxn<'_>,
        args: &[ParamArg<'_>],
    ) -> Result<()> {
        self.begin_bind(args.len())?;
        for (idx, arg) in args.iter().enumerate() {
            match arg {
                ParamArg::Scalar(value) => self.bind_scalar_slot(txn, idx, *value)?,
                ParamArg::Set(values) => self.bind_set_slot(txn, idx, values)?,
            }
        }
        Ok(())
    }

    /// Count check + slot sizing (pooled: the resolved/missed slots keep
    /// their capacity — and a set slot its `WordSet` `Vec` — across
    /// executions).
    fn begin_bind(&mut self, supplied: usize) -> Result<()> {
        if supplied != self.params.len() {
            return Err(Error::ParamCountMismatch {
                expected: self.params.len(),
                supplied,
            });
        }
        if self.resolved_params.len() != supplied {
            self.resolved_params.resize(supplied, Const::Word(0));
            self.missed_params.resize(supplied, false);
        }
        Ok(())
    }

    /// Binds one scalar slot in place. Precise bind errors per position:
    /// a set-typed slot rejects the scalar shape before any conversion.
    fn bind_scalar_slot(
        &mut self,
        txn: &ReadTxn<'_>,
        idx: usize,
        value: BindValue<'_>,
    ) -> Result<()> {
        let param = param_id(idx);
        match &self.params[idx] {
            ParamSpec::Set { .. } => Err(Error::ParamSetExpected { param }),
            // A mask slot: the vacuity rules land here, where the value
            // exists — the bind-time sibling of validation's literal-mask
            // rejections. Resolves to the mask's bits as a word.
            ParamSpec::Mask => {
                let BindValue::AllenMask(mask) = value else {
                    return Err(Error::AllenMaskParamExpected { param });
                };
                if mask.is_empty() {
                    return Err(Error::EmptyAllenMaskParam { param });
                }
                if mask.is_full() {
                    return Err(Error::FullAllenMaskParam { param });
                }
                self.resolved_params[idx] = Const::Word(u64::from(mask.bits()));
                self.missed_params[idx] = false;
                Ok(())
            }
            ParamSpec::Scalar { ty, point } => {
                // The one non-inline scalar kind, resolved IN PLACE: a
                // `bytes<N>` param's padded words land in the slot's
                // pooled `Const::Words` box (N > 8; the width is the
                // type, so past the first bind the box always fits) or
                // an inline `Const::Word` — zero allocator traffic on a
                // warm re-bind (the steady-state clause; every other
                // scalar kind is inline by construction).
                if let ValueType::FixedBytes { len } = ty {
                    let mismatch = Error::ParamTypeMismatch {
                        param,
                        expected: ty.clone(),
                    };
                    let BindValue::FixedBytes(bytes) = value else {
                        return Err(mismatch);
                    };
                    if bytes.len() != usize::from(*len) {
                        return Err(mismatch);
                    }
                    let (words, count) = crate::ir::normalize::fixed_bytes_word_buf(bytes);
                    if *point && count == 1 && words[0] == u64::MAX {
                        return Err(Error::PointParamAtCeiling { param });
                    }
                    if count == 1 {
                        self.resolved_params[idx] = Const::Word(words[0]);
                    } else if let Const::Words(slot) = &mut self.resolved_params[idx]
                        && slot.len() == count
                    {
                        slot.copy_from_slice(&words[..count]);
                    } else {
                        self.resolved_params[idx] = Const::Words(words[..count].into());
                    }
                    self.missed_params[idx] = false;
                    return Ok(());
                }
                let Some((resolved, missed)) = convert_scalar(txn, value, ty)? else {
                    return Err(Error::ParamTypeMismatch {
                        param,
                        expected: ty.clone(),
                    });
                };
                // The point-domain law: a point-position param bound to
                // its domain ceiling can be inside no interval. Both
                // element encodings put the ceiling at the all-ones word.
                if *point && matches!(resolved, Const::Word(u64::MAX)) {
                    return Err(Error::PointParamAtCeiling { param });
                }
                self.resolved_params[idx] = resolved;
                self.missed_params[idx] = missed;
                Ok(())
            }
        }
    }

    /// Binds one set slot in place, deduplicating into the slot's pooled
    /// `WordSet`. Elements land as flat column-word spans — one word per
    /// scalar element, `⌈N/8⌉` per `bytes<N>` element — sorted and
    /// deduplicated span-wise (docs/architecture/20-query-ir.md, § param
    /// sets).
    fn bind_set_slot(&mut self, txn: &ReadTxn<'_>, idx: usize, values: &[Value]) -> Result<()> {
        let param = param_id(idx);
        let (expected, point) = match &self.params[idx] {
            ParamSpec::Set { elem, point } => (elem, *point),
            ParamSpec::Scalar { .. } | ParamSpec::Mask => {
                return Err(Error::ParamScalarExpected { param });
            }
        };
        // One element's column-word span — width fixed by the anchored
        // element type.
        let element_width = match expected {
            ValueType::FixedBytes { len } => crate::encoding::fixed_bytes_words(*len),
            _ => 1,
        };
        // Pooled storage: steal the slot's previous `WordSet` so a warm
        // re-bind (any size within the documented assumption) reuses its
        // capacity.
        let mut words = match std::mem::replace(&mut self.resolved_params[idx], Const::Word(0)) {
            Const::WordSet(mut words) => {
                words.clear();
                words
            }
            _ => Vec::new(),
        };
        for (element, value) in values.iter().enumerate() {
            let Some(word_count) = element_words(txn, value, expected, &mut words)? else {
                // Park the pooled Vec back before erroring: the slot
                // keeps its capacity and the query stays bindable.
                words.clear();
                let expected = expected.clone();
                self.resolved_params[idx] = Const::WordSet(words);
                return Err(Error::ParamElementTypeMismatch {
                    param,
                    element,
                    expected,
                });
            };
            debug_assert_eq!(word_count, element_width, "one span per element");
            // The point-domain law, per element: a point set's elements
            // are points, and the ceiling is the ray's ∞, not a point
            // (see `bind_scalar_slot` — the word compare is exact for
            // both element encodings, and point sets are numeric, hence
            // one word wide).
            if point && words.last() == Some(&u64::MAX) {
                words.clear();
                self.resolved_params[idx] = Const::WordSet(words);
                return Err(Error::PointParamAtCeiling { param });
            }
        }
        // Sets are sets: sorted, deduplicated — span-wise for multi-word
        // elements (docs/architecture/20-query-ir.md, § param sets),
        // and IN PLACE either way: the pooled `Vec` is the only storage
        // (a warm re-bind touches no allocator — the contract's pooled
        // set clause). The span width is a compile-time array size,
        // dispatched once per bind.
        if element_width == 1 {
            words.sort_unstable();
            words.dedup();
        } else {
            match element_width {
                2 => sort_dedup_spans::<2>(&mut words),
                3 => sort_dedup_spans::<3>(&mut words),
                4 => sort_dedup_spans::<4>(&mut words),
                5 => sort_dedup_spans::<5>(&mut words),
                6 => sort_dedup_spans::<6>(&mut words),
                7 => sort_dedup_spans::<7>(&mut words),
                8 => sort_dedup_spans::<8>(&mut words),
                _ => unreachable!("bytes<N> spans are 2..=8 words (N ≤ 64)"),
            }
        }
        // Per-element intern misses resolved to the never-minted
        // sentinel; a sentinel matches nothing under `Eq`, so
        // dropping it here is the same semantics with a smaller
        // probe set ("out-of-vocabulary elements contribute
        // nothing"). Only the intern path mints sentinels —
        // numeric u64::MAX elements are real values and stay, and
        // bytes<N> elements never touch the dictionary at all.
        if matches!(expected, ValueType::String) {
            while words.last() == Some(&dict::SENTINEL_ID) {
                words.pop();
            }
        }
        // The empty set matches nothing — the `Eq`-miss
        // short-circuit machinery, applied where sound
        // (positive occurrences; `resolve_filters` reads the
        // role).
        self.missed_params[idx] = words.is_empty();
        self.resolved_params[idx] = Const::WordSet(words);
        Ok(())
    }
}

fn param_id(idx: usize) -> ParamId {
    ParamId(u16::try_from(idx).expect("param ids fit u16"))
}

/// Sorts and deduplicates the pooled word `Vec` span-wise, in place: the
/// flat words reinterpreted as `[u64; K]` spans (lexicographic array
/// order IS span order over big-endian column words), `sort_unstable`
/// plus a manual dedup sweep, then a truncate — zero allocator traffic,
/// pooled capacity preserved.
fn sort_dedup_spans<const K: usize>(words: &mut Vec<u64>) {
    let (spans, tail) = words.as_chunks_mut::<K>();
    debug_assert!(tail.is_empty(), "one whole span per element");
    spans.sort_unstable();
    let mut kept = spans.len().min(1);
    for idx in 1..spans.len() {
        if spans[idx] != spans[kept - 1] {
            spans[kept] = spans[idx];
            kept += 1;
        }
    }
    words.truncate(kept * K);
}

/// One set element's column-word span, appended to `out`; `Ok(None)` =
/// element type mismatch (the caller names the position). A String miss
/// resolves to the never-minted sentinel intern id (per-element miss
/// semantics, `docs/architecture/20-query-ir.md`); a `bytes<N>` element
/// contributes its `⌈N/8⌉` padded words with no dictionary traffic.
/// Returns the span's word count.
fn element_words(
    txn: &ReadTxn<'_>,
    value: &Value,
    expected: &ValueType,
    out: &mut Vec<u64>,
) -> Result<Option<usize>> {
    // The `bytes<N>` element, straight into the pooled span storage —
    // no `Const` intermediary, no per-element heap (the scalar slot's
    // in-place discipline, span-shaped).
    if let ValueType::FixedBytes { len } = expected {
        let Value::FixedBytes(raw) = value else {
            return Ok(None);
        };
        if raw.len() != usize::from(*len) {
            return Ok(None);
        }
        let (words, count) = crate::ir::normalize::fixed_bytes_word_buf(raw);
        out.extend_from_slice(&words[..count]);
        return Ok(Some(count));
    }
    let Some(view) = element_view(value) else {
        return Ok(None);
    };
    let Some((resolved, _)) = convert_scalar(txn, view, expected)? else {
        return Ok(None);
    };
    Ok(Some(match resolved {
        Const::Word(word) => {
            out.push(word);
            1
        }
        Const::Byte(byte) => {
            out.push(u64::from(byte));
            1
        }
        Const::Interval { .. } => {
            unreachable!("validated: no interval-typed param sets (IntervalParamSet)")
        }
        Const::Words(_)
        | Const::Param(_)
        | Const::ParamSet(_)
        | Const::WordSet(_)
        | Const::PendingIntern { .. } => {
            unreachable!("convert_scalar resolves scalar kinds to inline column form")
        }
    }))
}

/// Resolves every occurrence's symbolic filter constants for this
/// execution — residual filters into `out_filters`, selection key words
/// into `out_selections`, both **in place** (the lists' shapes are plan
/// constants, so a warm execution rewrites slots and reuses every
/// `WordSet` capacity). `Ok(false)` = a dictionary miss or empty set
/// under an `Eq` filter of a **positive** occurrence, which empties
/// the whole conjunctive query (sound for `Eq` on positive occurrences
/// only — on a negated occurrence the same miss just matches nothing,
/// so its anti-probe never rejects; a missed value under `Ne` resolves
/// to the sentinel id and matches everything).
pub(super) fn resolve_filters(
    txn: &ReadTxn<'_>,
    plan: &mut crate::plan::fj::ValidatedPlan,
    params: &[Const],
    missed: &[bool],
    out_filters: &mut [Vec<FilterPredicate>],
    out_selections: &mut [Vec<Vec<u64>>],
    latched: &mut u32,
) -> Result<bool> {
    for (occ_idx, occurrence) in plan.occurrences_mut().iter_mut().enumerate() {
        // A discharged occurrence (grounding-eliminated or grounding-folded)
        // resolves nothing: an eliminated occurrence's lists are empty,
        // and a folded occurrence's retained filter list is introspection's
        // picture only — plan-constant by the fold's own conditions,
        // never evaluated, so its slots stay empty and never count
        // toward the latch (`plan/ground/evaluate.rs`).
        if occurrence.role.discharged() {
            debug_assert!(occurrence.selections.is_empty());
            continue;
        }
        // Templates are mutable for exactly one write: the literal latch
        // — a resolved `PendingIntern` becomes its `Const::Word` in
        // place, once, permanently (the dictionary is append-only, the
        // prepared query owns its plan — `!Sync`, environment-pinned — and ids
        // outlive the environment).
        let negated = occurrence.role == crate::ir::normalize::Role::Negated;
        let filters = &mut out_filters[occ_idx];
        if filters.len() != occurrence.filters.len() {
            // First execution (or a plan-shape change, which cannot
            // happen): populate the slots; every later pass rewrites
            // them in place.
            filters.clear();
            filters.extend(occurrence.filters.iter().cloned());
        }
        for (template, slot) in occurrence.filters.iter_mut().zip(filters.iter_mut()) {
            if !resolve_filter_into(txn, template, params, missed, negated, slot, latched)? {
                return Ok(false);
            }
        }
        let selections = &mut out_selections[occ_idx];
        if selections.len() != occurrence.selections.len() {
            selections.clear();
            selections.resize_with(occurrence.selections.len(), Vec::new);
        }
        debug_assert!(
            !negated || occurrence.selections.is_empty(),
            "negated occurrences keep Eq-constants in their filters"
        );
        for (selection, words) in occurrence.selections.iter_mut().zip(selections.iter_mut()) {
            if !resolve_selection_into(txn, selection, params, missed, words, latched)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Resolves one selection's constant into the key words its trie level
/// probes with: one word for a scalar, the encoded pair for an interval,
/// the sorted deduplicated element words for a set (probed once per
/// element — docs/architecture/40-execution.md, § selection levels).
/// `Ok(false)` = a dictionary miss or empty set — the `Eq`
/// short-circuit (selections exist on positive occurrences only).
fn resolve_selection_into(
    txn: &ReadTxn<'_>,
    selection: &mut crate::plan::fj::Selection,
    params: &[Const],
    missed: &[bool],
    out: &mut Vec<u64>,
    latched: &mut u32,
) -> Result<bool> {
    out.clear();
    // The literal latch: a dictionary hit rewrites the template once —
    // this selection never touches the dictionary again.
    if let Const::PendingIntern { bytes } = &selection.value {
        let Some(word) = dict::lookup(txn, bytes)? else {
            return Ok(false);
        };
        selection.value = Const::Word(word);
        *latched += 1;
        obs::event(obs::names::LITERAL_LATCH, obs::Category::Execute, word, 0);
    }
    let push_const = |constant: &Const, out: &mut Vec<u64>| match constant {
        Const::Word(word) => out.push(*word),
        Const::Byte(byte) => out.push(u64::from(*byte)),
        Const::Words(words) => out.extend_from_slice(words),
        Const::Interval { start, end } => out.extend([*start, *end]),
        Const::WordSet(_) | Const::Param(_) | Const::ParamSet(_) | Const::PendingIntern { .. } => {
            unreachable!("bind resolved params to column form")
        }
    };
    match &selection.value {
        value @ (Const::Word(_) | Const::Byte(_) | Const::Words(_) | Const::Interval { .. }) => {
            push_const(value, out);
        }
        Const::Param(param) => {
            if missed[usize::from(param.0)] {
                return Ok(false);
            }
            push_const(&params[usize::from(param.0)], out);
        }
        Const::ParamSet(param) => {
            if missed[usize::from(param.0)] {
                return Ok(false); // the empty set matches nothing
            }
            let Const::WordSet(words) = &params[usize::from(param.0)] else {
                unreachable!("validated: a set param resolves to a word set")
            };
            out.extend_from_slice(words);
        }
        // A plan-constant set (the grounding-evaluator's fold —
        // `plan/ground/evaluate.rs`): pre-resolved at prepare, copied
        // through verbatim; nothing to look up, nothing pending, and it
        // never counts as an unresolved literal (the latch's fast path
        // stays reachable). Never empty: |S| == 0 killed the rule.
        Const::WordSet(words) => out.extend_from_slice(words),
        Const::PendingIntern { .. } => unreachable!("latched or short-circuited above"),
    }
    Ok(true)
}

/// Substitutes one filter's symbolic constants into its resolved slot,
/// in place. `Ok(false)` = the positive-occurrence `Eq` short-circuit
/// (dictionary miss or empty set); on a negated occurrence the miss
/// resolves to the sentinel id / empty word set — matching nothing, so
/// the anti-probe never rejects. The `Eq`-Compare arms are unreachable
/// through positive occurrences (`split_filters` routes every
/// Eq-constant into selections) and live for negated ones, whose
/// Eq-constants ARE view filters.
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // one arm per filter kind, in kind order
fn resolve_filter_into(
    txn: &ReadTxn<'_>,
    template: &mut FilterPredicate,
    params: &[Const],
    missed: &[bool],
    negated: bool,
    dst: &mut FilterPredicate,
    latched: &mut u32,
) -> Result<bool> {
    match template {
        FilterPredicate::Compare { field, op, value } => {
            // The literal latch: a dictionary hit rewrites the template
            // once; a miss keeps the template pending (live — something
            // may intern it later) and resolves this execution's slot to
            // the miss semantics verbatim.
            if let Const::PendingIntern { bytes } = value {
                match dict::lookup(txn, bytes)? {
                    Some(id) => {
                        let word = Const::Word(id);
                        *value = word;
                        *latched += 1;
                        obs::event(obs::names::LITERAL_LATCH, obs::Category::Execute, id, 0);
                    }
                    None if *op == CmpOp::Eq && !negated => return Ok(false),
                    None => {
                        write_compare(dst, *field, *op, Some(Const::Word(dict::SENTINEL_ID)));
                        return Ok(true);
                    }
                }
            }
            let resolved = match value {
                Const::Word(_) | Const::Byte(_) | Const::Interval { .. } => value.clone(),
                // A multi-word `bytes<N>` constant: written into the
                // slot's pooled `Words` box in place — the `Words` twin
                // of the `WordSet` arms below (a `Box` clone is a heap
                // hit the steady-state contract forbids, and this arm
                // runs per execution whenever the query carries any
                // param).
                Const::Words(words) => {
                    write_compare(dst, *field, *op, None);
                    write_words_value(dst, words);
                    return Ok(true);
                }
                Const::Param(param) => {
                    if missed[usize::from(param.0)] && *op == CmpOp::Eq && !negated {
                        return Ok(false);
                    }
                    // A negated Eq miss keeps the sentinel word bind
                    // stored — it matches nothing.
                    match &params[usize::from(param.0)] {
                        Const::Words(words) => {
                            write_compare(dst, *field, *op, None);
                            write_words_value(dst, words);
                            return Ok(true);
                        }
                        other => other.clone(),
                    }
                }
                Const::ParamSet(param) => {
                    debug_assert_eq!(*op, CmpOp::Eq, "validated: sets only under Eq");
                    if missed[usize::from(param.0)] && !negated {
                        return Ok(false); // the empty set matches nothing
                    }
                    let Const::WordSet(words) = &params[usize::from(param.0)] else {
                        unreachable!("validated: a set param resolves to a word set")
                    };
                    // In-place: reuse the slot's WordSet capacity.
                    write_compare(dst, *field, *op, None);
                    write_word_set_value(dst, words);
                    return Ok(true);
                }
                // A plan-constant set (the grounding-evaluator's fold):
                // pre-resolved at prepare — copy through into the
                // slot's pooled `WordSet` exactly like a bound param
                // set, with no per-execution work and no latch traffic.
                // (Attached sets land on participating occurrences,
                // whose Eq compares `split_filters` routes into
                // selections — this arm exists for the shape's
                // completeness, not a live path.)
                Const::WordSet(words) => {
                    debug_assert_eq!(*op, CmpOp::Eq, "plan-constant sets ride Eq");
                    write_compare(dst, *field, *op, None);
                    write_word_set_value(dst, words);
                    return Ok(true);
                }
                Const::PendingIntern { .. } => unreachable!("latched or short-circuited above"),
            };
            write_compare(dst, *field, *op, Some(resolved));
        }
        FilterPredicate::PointIn { field, point } => {
            let word = match point {
                ResolvedWordSource::Word(word) => *word,
                // Point params are numeric (interval elements are
                // U64/I64) — never a dictionary miss.
                ResolvedWordSource::Param(param) => match &params[usize::from(param.0)] {
                    Const::Word(word) => *word,
                    _ => unreachable!("validated: a point param resolves to a word"),
                },
                ResolvedWordSource::Var(_) => {
                    unreachable!("plan validation routes var points to membership probes")
                }
            };
            *dst = FilterPredicate::PointIn {
                field: *field,
                point: ResolvedWordSource::Word(word),
            };
        }
        FilterPredicate::AnyPointIn { field, set } => {
            let Const::ParamSet(param) = set else {
                unreachable!("templates carry ParamSet markers")
            };
            // An empty point set matches nothing; the occurrence's view
            // empties and the join answers (no short-circuit needed —
            // and none would be sound for a negated occurrence).
            let Const::WordSet(words) = &params[usize::from(param.0)] else {
                unreachable!("validated: a set param resolves to a word set")
            };
            if let FilterPredicate::AnyPointIn {
                field: dst_field,
                set: Const::WordSet(dst_words),
            } = dst
            {
                *dst_field = *field;
                dst_words.clear();
                dst_words.extend_from_slice(words);
            } else {
                *dst = FilterPredicate::AnyPointIn {
                    field: *field,
                    set: Const::WordSet(words.clone()),
                };
            }
        }
        FilterPredicate::FieldWithin { field, outer } => {
            let resolved = match outer {
                Const::Interval { .. } => outer.clone(),
                Const::Param(param) => params[usize::from(param.0)].clone(),
                _ => unreachable!("validated: the outer side is an interval constant"),
            };
            *dst = FilterPredicate::FieldWithin {
                field: *field,
                outer: resolved,
            };
        }
        // The Allen kinds: the mask resolves to its literal (params were
        // vacuity-checked at bind; a mirrored param converses here —
        // `MaskConst`), and `FieldAllen`'s constant side resolves like
        // any interval constant. Views are built with fully resolved
        // filters, so nothing symbolic survives past this point.
        FilterPredicate::FieldsAllen { left, right, mask } => {
            *dst = FilterPredicate::FieldsAllen {
                left: *left,
                right: *right,
                mask: MaskConst::Mask(crate::image::view::mask_of(*mask, params)),
            };
        }
        FilterPredicate::FieldAllen { field, other, mask } => {
            let resolved = match other {
                Const::Interval { .. } => other.clone(),
                Const::Param(param) => params[usize::from(param.0)].clone(),
                _ => unreachable!("validated: the Allen constant side is an interval"),
            };
            *dst = FilterPredicate::FieldAllen {
                field: *field,
                other: resolved,
                mask: MaskConst::Mask(crate::image::view::mask_of(*mask, params)),
            };
        }
        // The measure-vs-constant kind: the u64 bound resolves like any
        // scalar param (numeric — never a dictionary miss, and order
        // operators have no Eq short-circuit).
        FilterPredicate::DurationCompare { field, op, value } => {
            let resolved = match value {
                Const::Word(_) => value.clone(),
                Const::Param(param) => params[usize::from(param.0)].clone(),
                _ => unreachable!("validated: a measure compares against a u64 word"),
            };
            *dst = FilterPredicate::DurationCompare {
                field: *field,
                op: *op,
                value: resolved,
            };
        }
        // Constant-free kinds copy through (cheap: field ids only).
        FilterPredicate::FieldsCompare { .. }
        | FilterPredicate::FieldsPointIn { .. }
        | FilterPredicate::DurationFieldsCompare { .. } => {
            dst.clone_from(template);
        }
    }
    Ok(true)
}

/// Writes a `Compare` shape into the slot. `value: None` keeps the
/// slot's existing value word untouched (the WordSet-reuse path writes
/// it separately).
fn write_compare(
    dst: &mut FilterPredicate,
    field: bumbledb_theory::schema::FieldId,
    op: CmpOp,
    value: Option<Const>,
) {
    if let FilterPredicate::Compare {
        field: dst_field,
        op: dst_op,
        value: dst_value,
    } = dst
    {
        *dst_field = field;
        *dst_op = op;
        if let Some(value) = value {
            *dst_value = value;
        }
        return;
    }
    *dst = FilterPredicate::Compare {
        field,
        op,
        value: value.unwrap_or(Const::WordSet(Vec::new())),
    };
}

/// Writes the word set into a `Compare` slot's value, reusing its
/// existing `WordSet` allocation.
fn write_word_set_value(dst: &mut FilterPredicate, words: &[u64]) {
    let FilterPredicate::Compare { value, .. } = dst else {
        unreachable!("write_compare just shaped the slot")
    };
    if let Const::WordSet(dst_words) = value {
        dst_words.clear();
        dst_words.extend_from_slice(words);
    } else {
        *value = Const::WordSet(words.to_vec());
    }
}

/// Writes a multi-word `bytes<N>` constant into a `Compare` slot's
/// value, reusing its existing `Words` box when the span width matches —
/// it always does past the first execution: the constant's width is a
/// plan constant (the type is the width).
fn write_words_value(dst: &mut FilterPredicate, words: &[u64]) {
    let FilterPredicate::Compare { value, .. } = dst else {
        unreachable!("write_compare just shaped the slot")
    };
    if let Const::Words(dst_words) = value
        && dst_words.len() == words.len()
    {
        dst_words.copy_from_slice(words);
    } else {
        *value = Const::Words(words.into());
    }
}

/// A set element viewed through the bind vocabulary — the borrow
/// adapter between owned set storage ([`Value`]) and the one conversion
/// rule ([`convert_scalar`]). `None` = non-UTF-8 `String` bytes: a
/// mismatch by construction, since [`BindValue::Str`] cannot carry them.
fn element_view(value: &Value) -> Option<BindValue<'_>> {
    Some(match value {
        Value::Bool(v) => BindValue::Bool(*v),
        Value::U64(v) => BindValue::U64(*v),
        Value::I64(v) => BindValue::I64(*v),
        Value::String(raw) => BindValue::Str(std::str::from_utf8(raw).ok()?),
        Value::FixedBytes(raw) => BindValue::FixedBytes(raw),
        Value::IntervalU64(interval) => BindValue::IntervalU64(interval.start(), interval.end()),
        Value::IntervalI64(interval) => BindValue::IntervalI64(interval.start(), interval.end()),
        // A mask is no element type — a set never holds masks; the
        // caller reports the element mismatch.
        Value::AllenMask(_) => return None,
    })
}

/// Converts a bound scalar param value to column form, checking kind,
/// enum ordinal range, and interval non-emptiness in the same match
/// (UTF-8 needs no check: `BindValue::Str` is UTF-8 by type); `Ok(None)`
/// = type mismatch (the caller names the position — scalar slot or set
/// element). A str or bytes payload that was never interned resolves to
/// the sentinel intern id, flagged `missed` so `Eq` uses can
/// short-circuit to the empty result. The payload is only hashed and
/// probed here — the reason the bind surface borrows.
fn convert_scalar(
    txn: &ReadTxn<'_>,
    value: BindValue<'_>,
    expected: &ValueType,
) -> Result<Option<(Const, bool)>> {
    let resolved = match (value, expected) {
        (BindValue::Bool(v), ValueType::Bool) => Const::Byte(u8::from(v)),
        (BindValue::U64(v), ValueType::U64) => Const::Word(v),
        (BindValue::I64(v), ValueType::I64) => Const::Word(i64_word(v)),
        // The interval family: the general type takes any nonempty
        // bounds; a fixed-width position demands exactly the declared
        // width and never a ray (Q2 — `crate::schema::value_matches`'
        // rule, applied to the bind vocabulary; the width is the type).
        (
            BindValue::IntervalU64(start, end),
            ValueType::Interval {
                element: IntervalElement::U64,
                width,
            },
        ) if start < end
            && width.is_none_or(|w| {
                end - start == w && end < bumbledb_theory::Interval::<u64>::MAX_END
            }) =>
        {
            Const::Interval { start, end }
        }
        (
            BindValue::IntervalI64(start, end),
            ValueType::Interval {
                element: IntervalElement::I64,
                width,
            },
        ) if start < end
            && width.is_none_or(|w| {
                end.abs_diff(start) == w && end < bumbledb_theory::Interval::<i64>::MAX_END
            }) =>
        {
            Const::Interval {
                start: i64_word(start),
                end: i64_word(end),
            }
        }
        (BindValue::Str(text), ValueType::String) => match dict::lookup_str(txn, text)? {
            Some(id) => Const::Word(id),
            None => return Ok(Some((Const::Word(dict::SENTINEL_ID), true))),
        },
        // `bytes<N>` never reaches here: both callers resolve it in
        // place through `fixed_bytes_word_buf` first (pooled slots, no
        // per-bind heap) — the arm's absence is deliberate, not a gap.
        _ => return Ok(None),
    };
    Ok(Some((resolved, false)))
}

/// The biased I64 column word (u64 word order equals i64 value order).
fn i64_word(value: i64) -> u64 {
    u64::from_be_bytes(crate::encoding::encode_i64(value))
}
