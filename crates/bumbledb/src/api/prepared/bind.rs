use super::{
    BindValue, Const, Executor, FilterPredicate, ParamArg, ParamSpec, PreparedQuery, PreparedRule,
    ValueType,
};

use crate::error::{Error, Mismatch, Result};
use crate::ir::{ParamId, Value};
use crate::obs;
use crate::storage::catalog::CatalogRead;
use crate::storage::dict;
use crate::storage::env::ReadTxn;
use bumbledb_theory::schema::IntervalElement;

impl<S> PreparedQuery<S> {
    /// Rebuilds the executor scratch at a different batch size — the
    /// tuning/test surface for D4's measurement-owned constant. Allocation
    /// happens here, outside any measured window. A no-op for key_probe
    /// probes. Hidden: a measurement affordance, not a knob on the
    /// no-knobs surface (`docs/architecture/00-product.md`).
    #[doc(hidden)]
    pub fn set_batch_size(&mut self, batch: usize) {
        self.visit_rules_mut(|rule| match rule {
            PreparedRule::FreeJoin(rule) => {
                rule.executor = Executor::with_batch_size(&rule.plan, batch);
            }
            PreparedRule::KeyProbe(_) => {}
        });
        self.visit_rec_arms_mut(|arm| {
            arm.rule.executor = Executor::with_batch_size(&arm.rule.plan, batch);
        });
    }

    /// The identity check at every execution entry (`execute` and
    /// `profile`; `execute_collect` and `introspect` route through them):
    /// a snapshot of any environment other than the preparing one is a
    /// typed error before anything else runs. One u64 compare — with the
    /// entry protected, the view memo needs no environment epoch in its
    /// generation keys.
    pub(super) fn check_identity(
        &self,
        identity: &crate::storage::env::CatalogIdentity,
    ) -> Result<()> {
        if self.identity.same(identity) {
            Ok(())
        } else {
            Err(Error::ForeignPreparedQuery)
        }
    }

    pub(super) fn check_snapshot(&self, txn: &ReadTxn<'_>) -> Result<()> {
        self.check_identity(txn.identity())
    }

    /// Binds and converts all-scalar parameters (the `&[BindValue]`
    /// entry; a set-typed param rejects the scalar shape with
    /// [`Error::ParamSetExpected`] — the mixed entry is
    /// [`PreparedQuery::bind_param_args`]).
    pub(super) fn bind_params<C: CatalogRead>(
        &mut self,
        catalog: &C,
        params: &[BindValue<'_>],
    ) -> Result<()> {
        self.begin_bind(params.len())?;
        for (idx, value) in params.iter().enumerate() {
            self.bind_scalar_slot(catalog, idx, *value)?;
        }
        Ok(())
    }

    /// Binds mixed scalar/set parameter arguments (the public
    /// [`ParamArg`] entry — `docs/architecture/70-api.md` § facts and
    /// results).
    pub(crate) fn bind_param_args<C: CatalogRead>(
        &mut self,
        catalog: &C,
        args: &[ParamArg<'_>],
    ) -> Result<()> {
        self.begin_bind(args.len())?;
        for (idx, arg) in args.iter().enumerate() {
            match arg {
                ParamArg::Scalar(value) => self.bind_scalar_slot(catalog, idx, *value)?,
                ParamArg::Set(values) => self.bind_set_slot(catalog, idx, values)?,
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
                mismatch: Mismatch {
                    witnessed: supplied,
                    required: self.params.len(),
                },
            });
        }
        if self.resolved_params.len() != supplied {
            self.resolved_params.resize(supplied, Const::Word(0));
            self.missed_params.resize(supplied, false);
            self.param_word_memo
                .resize(supplied, super::ParamWordMemo::default());
        }
        Ok(())
    }

    /// Binds one scalar slot in place. Precise bind errors per position:
    /// a set-typed slot rejects the scalar shape before any conversion.
    fn bind_scalar_slot<C: CatalogRead>(
        &mut self,
        catalog: &C,
        idx: usize,
        value: BindValue<'_>,
    ) -> Result<()> {
        let param = param_id(idx);
        match &self.params[idx] {
            ParamSpec::Set { .. } => Err(Error::ParamSetExpected { param }),
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
                        expected: *ty,
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
                // The per-slot param-word memo: re-binding the same
                // text skips the dictionary descent whole. Sound
                // because the dictionary is append-only — a text's
                // resolved word is FINAL — while a prior MISS never
                // memoizes (a later write may intern the text, so the
                // miss re-probes every bind until it latches).
                if matches!(ty, ValueType::String)
                    && let BindValue::Str(text) = value
                {
                    let memo = &self.param_word_memo[idx];
                    if let Some(word) = memo.word
                        && memo.text == text
                    {
                        obs::event(
                            obs::names::PARAM_WORD_MEMO,
                            obs::TraceArgs::Pair(idx as u64, word),
                        );
                        self.resolved_params[idx] = Const::Word(word);
                        self.missed_params[idx] = false;
                        return Ok(());
                    }
                }
                let Some((resolved, missed)) = convert_scalar(catalog, value, ty)? else {
                    return Err(Error::ParamTypeMismatch {
                        param,
                        expected: *ty,
                    });
                };
                if let (BindValue::Str(text), ValueType::String) = (value, ty) {
                    let memo = &mut self.param_word_memo[idx];
                    if missed {
                        memo.word = None;
                    } else if let Const::Word(word) = &resolved {
                        memo.text.clear();
                        memo.text.push_str(text);
                        memo.word = Some(*word);
                    }
                }
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
    fn bind_set_slot<C: CatalogRead>(
        &mut self,
        catalog: &C,
        idx: usize,
        values: &[Value],
    ) -> Result<()> {
        let param = param_id(idx);
        let (expected, point) = match &self.params[idx] {
            ParamSpec::Set { elem, point } => (elem, *point),
            ParamSpec::Scalar { .. } => {
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
            let Some(word_count) = element_words(catalog, value, expected, &mut words)? else {
                // Park the pooled Vec back before erroring: the slot
                // keeps its capacity and the query stays bindable.
                words.clear();
                let expected = *expected;
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
fn element_words<C: CatalogRead>(
    catalog: &C,
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
    let Some((resolved, _)) = convert_scalar(catalog, element_view(value), expected)? else {
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
pub(super) fn resolve_filters<C: CatalogRead>(
    catalog: &C,
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
            if !crate::image::view::resolve_filter_into(
                catalog, template, params, missed, negated, slot, latched,
            )? {
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
            if !resolve_selection_into(catalog, selection, params, missed, words, latched)? {
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
fn resolve_selection_into<C: CatalogRead>(
    catalog: &C,
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
        let Some(id) = catalog.dict_lookup(bytes)? else {
            return Ok(false);
        };
        selection.value = Const::Word(id.raw());
        *latched += 1;
        obs::event(obs::names::LITERAL_LATCH, obs::TraceArgs::Count(id.raw()));
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

/// A set element viewed through the bind vocabulary — the borrow
/// adapter between owned set storage ([`Value`]) and the one conversion
/// rule ([`convert_scalar`]).
fn element_view(value: &Value) -> BindValue<'_> {
    match value {
        Value::Bool(v) => BindValue::Bool(*v),
        Value::U64(v) => BindValue::U64(*v),
        Value::I64(v) => BindValue::I64(*v),
        Value::String(text) => BindValue::Str(text),
        Value::FixedBytes(raw) => BindValue::FixedBytes(raw),
        Value::IntervalU64(interval) => BindValue::IntervalU64(interval.start(), interval.end()),
        Value::IntervalI64(interval) => BindValue::IntervalI64(interval.start(), interval.end()),
    }
}

/// Converts a bound scalar param value to column form, checking kind,
/// enum ordinal range, and interval non-emptiness in the same match
/// (UTF-8 needs no check: `BindValue::Str` is UTF-8 by type); `Ok(None)`
/// = type mismatch (the caller names the position — scalar slot or set
/// element). A str or bytes payload that was never interned resolves to
/// the sentinel intern id, flagged `missed` so `Eq` uses can
/// short-circuit to the empty result. The payload is only hashed and
/// probed here — the reason the bind surface borrows.
fn convert_scalar<C: CatalogRead>(
    catalog: &C,
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
            },
        ) if start < end => Const::Interval { start, end },
        (
            BindValue::IntervalU64(start, end),
            ValueType::FixedInterval {
                element: IntervalElement::U64,
                width,
            },
        ) if start < end
            && end - start == *width
            && end < bumbledb_theory::Interval::<u64>::MAX_END =>
        {
            Const::Interval { start, end }
        }
        (
            BindValue::IntervalI64(start, end),
            ValueType::Interval {
                element: IntervalElement::I64,
            },
        ) if start < end => Const::Interval {
            start: i64_word(start),
            end: i64_word(end),
        },
        (
            BindValue::IntervalI64(start, end),
            ValueType::FixedInterval {
                element: IntervalElement::I64,
                width,
            },
        ) if start < end
            && end.abs_diff(start) == *width
            && end < bumbledb_theory::Interval::<i64>::MAX_END =>
        {
            Const::Interval {
                start: i64_word(start),
                end: i64_word(end),
            }
        }
        (BindValue::Str(text), ValueType::String) => match catalog.dict_lookup(text.as_bytes())? {
            Some(id) => Const::Word(id.raw()),
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
