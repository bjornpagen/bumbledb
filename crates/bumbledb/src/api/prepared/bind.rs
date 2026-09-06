use super::{
    BindValue, Const, Executor, FilterPredicate, ParamArg, ParamSpec, PreparedQuery, PreparedRule,
    ValueType,
};

use crate::error::{Error, Mismatch, Result};
use crate::image::intern::InternerHandle;
use crate::ir::{ParamId, Value};
use crate::work::WorkContext;
use bumbledb_theory::schema::IntervalElement;

impl<S> PreparedQuery<S> {
    /// Establish one resolver namespace before binding any parameter.
    /// The caller holds `generation` for the entire operation, including
    /// cache pressure or a sibling query's concurrent trim.
    #[inline]
    pub(super) fn bind_text_generation(&mut self, generation: &crate::work::GenerationHandle) {
        if self
            .text_generation
            .as_ref()
            .is_some_and(|previous| previous.identity() == generation.identity())
        {
            return;
        }
        self.rotate_text_generation(generation);
    }

    #[cold]
    #[inline(never)]
    fn rotate_text_generation(&mut self, generation: &crate::work::GenerationHandle) {
        self.visit_rules_mut(|rule| {
            if let PreparedRule::FreeJoin(rule) = rule {
                forget_resolved_text(rule);
                rule.memo.trim();
            }
        });
        self.visit_rec_arms_mut(|arm| {
            forget_resolved_text(arm);
            arm.memo.trim();
        });
        self.derived = super::reach::DerivedImages::default();
        self.resolve_memo.forget_scratch();
        for memo in &mut self.param_word_memo {
            memo.word = None;
            memo.epoch = None;
        }
        self.text_generation = Some(generation.downgrade());
    }

    #[doc(hidden)]
    pub fn set_batch_size(&mut self, batch: usize) {
        self.visit_rules_mut(|rule| match rule {
            PreparedRule::FreeJoin(rule) => {
                rule.executor = Executor::with_batch_size(&rule.plan, batch);
            }
            PreparedRule::KeyProbe(_) => {}
        });
        self.visit_rec_arms_mut(|arm| {
            arm.executor = Executor::with_batch_size(&arm.plan, batch);
        });
    }

    /// Drop the execution-local scratch store only after forgetting every
    /// memo that named its tokens. Scratch literal templates keep their
    /// original bytes; only the per-execution resolved slots contain tags.
    pub(super) fn release_text_store(&mut self) {
        if self.nonresident.is_some() {
            self.visit_rules_mut(|rule| {
                if let PreparedRule::FreeJoin(fj) = rule {
                    forget_resolved_text(fj);
                }
            });
            self.visit_rec_arms_mut(forget_resolved_text);
        }
        for memo in &mut self.param_word_memo {
            if memo.word.is_some_and(crate::image::is_scratch_token) {
                memo.word = None;
                memo.epoch = None;
            }
        }
        self.resolve_memo.forget_scratch();
        self.nonresident = None;
    }

    /// After spill, rewrite live String params into the store so filter
    /// `Const::Param` and sink words share one namespace with row tokens.
    pub(super) fn rehome_bound_text(
        &mut self,
        interner: &InternerHandle<'_>,
        work: &WorkContext,
    ) -> Result<()> {
        let Some(store) = self.nonresident.as_mut() else {
            return Ok(());
        };
        for idx in 0..self.params.len() {
            let string_param = matches!(
                &self.params[idx],
                super::ParamSpec::Scalar {
                    ty: ValueType::String,
                    ..
                } | super::ParamSpec::Set {
                    elem: ValueType::String,
                    ..
                }
            );
            if !string_param {
                continue;
            }
            match &mut self.resolved_params[idx] {
                Const::Word(token) if crate::image::is_resident_token(*token) => {
                    let Some(text) = interner.with_text(*token, std::borrow::ToOwned::to_owned)
                    else {
                        continue;
                    };
                    *token = store.intern(&text, work)?;
                    if let Some(memo) = self.param_word_memo.get_mut(idx) {
                        memo.word = Some(*token);
                        memo.epoch = Some(store.epoch());
                    }
                }
                Const::WordSet(words) => {
                    for word in words.iter_mut() {
                        if crate::image::is_resident_token(*word) {
                            let Some(text) =
                                interner.with_text(*word, std::borrow::ToOwned::to_owned)
                            else {
                                continue;
                            };
                            *word = store.intern(&text, work)?;
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Foreign snapshot is a typed error before anything else runs.
    pub(super) fn check_identity(&self, source: super::source::PinnedSource) -> Result<()> {
        if self.pinned == source {
            Ok(())
        } else {
            Err(Error::ForeignPreparedQuery)
        }
    }

    pub(super) fn bind_params(
        &mut self,
        work: &WorkContext,
        params: &[BindValue<'_>],
    ) -> Result<()> {
        self.begin_bind(params.len())?;
        for (idx, value) in params.iter().enumerate() {
            self.bind_scalar_slot(work, idx, *value)?;
        }
        Ok(())
    }

    pub(crate) fn bind_param_args(
        &mut self,
        work: &WorkContext,
        args: &[ParamArg<'_>],
    ) -> Result<()> {
        self.begin_bind(args.len())?;
        for (idx, arg) in args.iter().enumerate() {
            match arg {
                ParamArg::Scalar(value) => self.bind_scalar_slot(work, idx, *value)?,
                ParamArg::Set(values) => self.bind_set_slot(work, idx, values)?,
            }
        }
        Ok(())
    }

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

    /// a set-typed slot rejects the scalar shape before any conversion.
    fn bind_scalar_slot(
        &mut self,
        work: &WorkContext,
        idx: usize,
        value: BindValue<'_>,
    ) -> Result<()> {
        let param = param_id(idx);
        match &self.params[idx] {
            ParamSpec::Set { .. } => Err(Error::ParamSetExpected { param }),
            ParamSpec::Scalar { ty, point } => {
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

                if let (ValueType::Uuid, BindValue::Uuid(id)) = (ty, value)
                    && let Const::Words(slot) = &mut self.resolved_params[idx]
                    && slot.len() == 2
                {
                    slot.copy_from_slice(&uuid_words(id));
                    self.missed_params[idx] = false;
                    return Ok(());
                }

                if matches!(ty, ValueType::String)
                    && let BindValue::Str(text) = value
                {
                    let memo = &self.param_word_memo[idx];
                    if let Some(word) = memo.word
                        && memo.text == text
                        && param_memo_live(memo, self.nonresident.as_ref())
                    {
                        self.resolved_params[idx] = Const::Word(word);
                        self.missed_params[idx] = false;
                        return Ok(());
                    }
                }
                let owner = &self.text_generation;
                let cache = &self.cache;
                let Some(resolved) = convert_scalar(value, ty, |text| {
                    let generation = owner
                        .as_ref()
                        .and_then(crate::work::cache::WeakGenerationHandle::upgrade)
                        .unwrap_or_else(|| cache.acquire());
                    let interner = InternerHandle::new(&generation, work);
                    super::text::intern_admitted(&interner, &mut self.nonresident, text, work)
                })?
                else {
                    return Err(Error::ParamTypeMismatch {
                        param,
                        expected: *ty,
                    });
                };
                if let (BindValue::Str(text), ValueType::String) = (value, ty)
                    && let Const::Word(word) = &resolved
                {
                    let memo = &mut self.param_word_memo[idx];
                    memo.text.clear();
                    memo.text.push_str(text);
                    if crate::image::is_resident_token(*word) {
                        memo.word = Some(*word);
                        memo.epoch = None;
                    } else if crate::image::is_scratch_token(*word) {
                        // Execution-local: bound to the minting store's owner epoch.
                        memo.word = Some(*word);
                        memo.epoch = self
                            .nonresident
                            .as_ref()
                            .map(crate::image::NonresidentTextStore::epoch);
                    } else {
                        memo.word = None;
                        memo.epoch = None;
                    }
                }

                if *point && matches!(resolved, Const::Word(u64::MAX)) {
                    return Err(Error::PointParamAtCeiling { param });
                }
                self.resolved_params[idx] = resolved;
                self.missed_params[idx] = false;
                Ok(())
            }
        }
    }

    fn bind_set_slot(&mut self, work: &WorkContext, idx: usize, values: &[Value]) -> Result<()> {
        let param = param_id(idx);
        let (expected, point) = match &self.params[idx] {
            ParamSpec::Set { elem, point } => (elem, *point),
            ParamSpec::Scalar { .. } => {
                return Err(Error::ParamScalarExpected { param });
            }
        };

        let element_width = match expected {
            ValueType::FixedBytes { len } => crate::encoding::fixed_bytes_words(*len),
            ValueType::Uuid => 2,
            _ => 1,
        };

        let mut words = match std::mem::replace(&mut self.resolved_params[idx], Const::Word(0)) {
            Const::WordSet(mut words) => {
                words.clear();
                words
            }
            _ => Vec::new(),
        };
        // Numeric sets never need a text generation. String sets acquire it
        // once, lazily, and keep the same resolver for every element.
        let owner = &self.text_generation;
        let cache = &self.cache;
        let generation = std::cell::LazyCell::new(|| {
            owner
                .as_ref()
                .and_then(crate::work::cache::WeakGenerationHandle::upgrade)
                .unwrap_or_else(|| cache.acquire())
        });
        for (element, value) in values.iter().enumerate() {
            let Some(word_count) = element_words(value, expected, &mut words, |text| {
                let interner = InternerHandle::new(&generation, work);
                super::text::intern_admitted(&interner, &mut self.nonresident, text, work)
            })?
            else {
                // Park the pooled Vec back before erroring: the slot

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

            if point && words.last() == Some(&u64::MAX) {
                words.clear();
                self.resolved_params[idx] = Const::WordSet(words);
                return Err(Error::PointParamAtCeiling { param });
            }
        }

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

        // The empty set matches nothing under Eq on a positive occurrence;
        // the miss short-circuit machinery carries exactly that.
        self.missed_params[idx] = words.is_empty();
        self.resolved_params[idx] = Const::WordSet(words);
        Ok(())
    }
}

fn param_id(idx: usize) -> ParamId {
    ParamId(u16::try_from(idx).expect("param ids fit u16"))
}

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

fn element_words(
    value: &Value,
    expected: &ValueType,
    out: &mut Vec<u64>,
    intern_text: impl FnOnce(&str) -> Result<u64>,
) -> Result<Option<usize>> {
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
    let Some(resolved) = convert_scalar(element_view(value), expected, intern_text)? else {
        return Ok(None);
    };
    Ok(Some(match resolved {
        Const::Word(scalar) => {
            out.push(scalar);
            1
        }
        Const::Byte(byte) => {
            out.push(u64::from(byte));
            1
        }
        Const::Interval { .. } => {
            unreachable!("validated: no interval-typed param sets (IntervalParamSet)")
        }
        // Uuid elements are two-word spans, exactly like bytes<16>.
        Const::Words(words) => {
            out.extend_from_slice(&words);
            words.len()
        }
        Const::Param(_) | Const::ParamSet(_) | Const::WordSet(_) | Const::PendingIntern { .. } => {
            unreachable!("convert_scalar resolves scalar kinds to inline column form")
        }
    }))
}

/// One execution's literal-resolution context, shared by resident and fallback paths.
pub(super) struct LiteralResolution<'a, 'generation> {
    pub interner: &'a InternerHandle<'generation>,
    pub store: &'a mut Option<crate::image::NonresidentTextStore>,
    pub work: &'a WorkContext,
    pub params: &'a [Const],
    pub missed: &'a [bool],
}

impl LiteralResolution<'_, '_> {
    pub(super) fn filters(
        &mut self,
        plan: &crate::plan::fj::ValidatedPlan,
        out_filters: &mut [Vec<FilterPredicate>],
        out_selections: &mut [Vec<Vec<u64>>],
    ) -> Result<bool> {
        for (occ_idx, occurrence) in plan.occurrences().iter().enumerate() {
            if occurrence.role.discharged() {
                debug_assert!(occurrence.selections.is_empty());
                continue;
            }

            let negated = occurrence.role == crate::ir::normalize::Role::Negated;
            let filters = &mut out_filters[occ_idx];
            if filters.len() != occurrence.filters.len() {
                filters.clear();
                filters.extend(occurrence.filters.iter().cloned());
            }
            for (template, slot) in occurrence.filters.iter().zip(filters.iter_mut()) {
                if !self.filter(template, negated, slot)? {
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
            for (selection, words) in occurrence.selections.iter().zip(selections.iter_mut()) {
                if !self.selection(selection, words)? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    pub(super) fn filter(
        &mut self,
        template: &FilterPredicate,
        negated: bool,
        slot: &mut FilterPredicate,
    ) -> Result<bool> {
        match crate::image::view::resolve_filter_into(
            self.interner,
            template,
            self.params,
            self.missed,
            negated,
            slot,
        )? {
            crate::image::ResidentAdmit::Ready(keep) => Ok(keep),
            crate::image::ResidentAdmit::BeyondMemory(exhausted) => {
                let opened = super::text::install(self.store, &exhausted, self.work)?;
                let (field, op, bytes) = match template {
                    FilterPredicate::Compare {
                        field,
                        op,
                        value: Const::PendingIntern { bytes },
                    } => (*field, *op, bytes),
                    _ => unreachable!("only a pending text literal can exhaust the interner"),
                };
                let text = std::str::from_utf8(bytes)
                    .expect("IR string literals are UTF-8 by construction");
                let word = opened.intern(text, self.work)?;
                // Scratch words are execution-local: write the slot, keep
                // PendingIntern on the template so the next execute re-interns.
                *slot = FilterPredicate::Compare {
                    field,
                    op,
                    value: Const::Word(word),
                };
                Ok(true)
            }
        }
    }

    pub(super) fn selection(
        &mut self,
        selection: &crate::plan::fj::Selection,
        out: &mut Vec<u64>,
    ) -> Result<bool> {
        if let Const::PendingIntern { bytes } = &selection.value {
            if matches!(out.as_slice(), [word] if crate::image::is_resident_token(*word)) {
                return Ok(true);
            }
            out.clear();
            let text = std::str::from_utf8(bytes)
                .expect("IR string literals are UTF-8 by construction (Value::String)");
            let word = super::text::intern_admitted(self.interner, self.store, text, self.work)?;
            out.push(word);
            return Ok(true);
        }
        out.clear();
        let push_const = |constant: &Const, out: &mut Vec<u64>| match constant {
            Const::Word(word) => out.push(*word),
            Const::Byte(byte) => out.push(u64::from(*byte)),
            Const::Words(words) => out.extend_from_slice(words),
            Const::Interval { start, end } => out.extend([*start, *end]),
            Const::WordSet(_)
            | Const::Param(_)
            | Const::ParamSet(_)
            | Const::PendingIntern { .. } => {
                unreachable!("bind resolved parameters to column form")
            }
        };
        match &selection.value {
            value
            @ (Const::Word(_) | Const::Byte(_) | Const::Words(_) | Const::Interval { .. }) => {
                push_const(value, out);
            }
            Const::Param(param) => {
                if self.missed[usize::from(param.0)] {
                    return Ok(false);
                }
                push_const(&self.params[usize::from(param.0)], out);
            }
            Const::ParamSet(param) => {
                if self.missed[usize::from(param.0)] {
                    return Ok(false);
                }
                let Const::WordSet(words) = &self.params[usize::from(param.0)] else {
                    unreachable!("validated: a set param resolves to a word set")
                };
                out.extend_from_slice(words);
            }

            Const::WordSet(words) => out.extend_from_slice(words),
            Const::PendingIntern { .. } => unreachable!("resolved above"),
        }
        Ok(true)
    }
}

fn element_view(value: &Value) -> BindValue<'_> {
    match value {
        Value::Bool(v) => BindValue::Bool(*v),
        Value::U64(v) => BindValue::U64(*v),
        Value::I64(v) => BindValue::I64(*v),
        Value::F64(v) => BindValue::F64(*v),
        Value::Uuid(id) => BindValue::Uuid(*id),
        Value::String(text) => BindValue::Str(text),
        Value::FixedBytes(raw) => BindValue::FixedBytes(raw),
        Value::IntervalU64(interval) => BindValue::IntervalU64(interval.start(), interval.end()),
        Value::IntervalI64(interval) => BindValue::IntervalI64(interval.start(), interval.end()),
        Value::IntervalF64(interval) => BindValue::IntervalF64(*interval),
    }
}

fn convert_scalar(
    value: BindValue<'_>,
    expected: &ValueType,
    intern_text: impl FnOnce(&str) -> Result<u64>,
) -> Result<Option<Const>> {
    let resolved = match (value, expected) {
        (BindValue::Bool(v), ValueType::Bool) => Const::Byte(u8::from(v)),
        (BindValue::U64(v), ValueType::U64) => Const::Word(v),
        (BindValue::I64(v), ValueType::I64) => Const::Word(i64_word(v)),
        (BindValue::F64(v), ValueType::F64) => Const::Word(v.to_order_key()),
        (BindValue::Uuid(id), ValueType::Uuid) => Const::Words(Box::from(uuid_words(id))),

        (
            BindValue::IntervalU64(start, end),
            ValueType::Interval {
                element: IntervalElement::U64,
            },
        ) if start < end => Const::Interval { start, end },
        (
            BindValue::IntervalU64(start, end),
            ValueType::FixedInterval {
                element: bumbledb_theory::schema::FixedIntervalElement::U64,
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
                element: bumbledb_theory::schema::FixedIntervalElement::I64,
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
        // The checked host type already carries canonical NaN-free
        // strictly-ordered endpoints; the physical words are order keys.
        (
            BindValue::IntervalF64(interval),
            ValueType::Interval {
                element: IntervalElement::F64,
            },
        ) => Const::Interval {
            start: interval.start().to_order_key(),
            end: interval.end().to_order_key(),
        },
        // Interning is append-only and never misses: the bound text's
        // token is final. A text absent from every image is an ordinary
        // unequal word — no sentinel machinery, no dictionary descent.
        (BindValue::Str(text), ValueType::String) => Const::Word(intern_text(text)?),

        _ => return Ok(None),
    };
    Ok(Some(resolved))
}

fn i64_word(value: i64) -> u64 {
    u64::from_be_bytes(crate::encoding::encode_i64(value))
}

fn uuid_words(id: crate::Uuid) -> [u64; 2] {
    let bytes = id.into_bytes();
    [
        u64::from_be_bytes(bytes[..8].try_into().expect("sixteen bytes")),
        u64::from_be_bytes(bytes[8..].try_into().expect("sixteen bytes")),
    ]
}

fn param_memo_live(
    memo: &super::ParamWordMemo,
    store: Option<&crate::image::NonresidentTextStore>,
) -> bool {
    match memo.epoch {
        None => memo.word.is_some_and(crate::image::is_resident_token),
        Some(epoch) => {
            let Some(store) = store else {
                return false;
            };
            let eq = store.text_eq().with_memo_stamp(epoch);
            eq.accepts_stamp(epoch) && memo.word.is_some_and(|word| store.live(word))
        }
    }
}

fn forget_resolved_text(rule: &mut super::FreeJoinRule) {
    for filters in &mut rule.resolved_filters {
        filters.clear();
    }
    for selections in &mut rule.resolved_selections {
        selections.clear();
    }
    rule.resolution = super::ResolutionState::Pending;
    rule.fallback.forget_resolved_text();
}

#[cfg(test)]
mod scalar_resolution_tests {
    use super::*;

    #[test]
    fn nontext_scalars_and_type_mismatches_never_acquire_a_text_resolver() {
        let id = crate::Uuid::from_bytes([42; 16]);
        for (value, expected) in [
            (BindValue::Bool(true), ValueType::Bool),
            (BindValue::U64(7), ValueType::U64),
            (BindValue::I64(-7), ValueType::I64),
            (BindValue::F64(crate::F64::from(1.5)), ValueType::F64),
            (BindValue::Uuid(id), ValueType::Uuid),
            (
                BindValue::IntervalU64(1, 3),
                ValueType::Interval {
                    element: IntervalElement::U64,
                },
            ),
            (
                BindValue::IntervalI64(-1, 3),
                ValueType::Interval {
                    element: IntervalElement::I64,
                },
            ),
        ] {
            assert!(
                convert_scalar(value, &expected, |_| panic!("nontext resolver"))
                    .unwrap()
                    .is_some()
            );
        }
        for (value, expected) in [
            (BindValue::Str("wrong type"), ValueType::U64),
            (BindValue::U64(7), ValueType::String),
        ] {
            assert!(
                convert_scalar(value, &expected, |_| panic!("mismatch resolver"))
                    .unwrap()
                    .is_none()
            );
        }
        let mut words = Vec::new();
        assert_eq!(
            element_words(&Value::U64(7), &ValueType::U64, &mut words, |_| panic!(
                "nontext set resolver"
            ))
            .unwrap(),
            Some(1)
        );
        assert_eq!(words, [7]);
    }

    #[test]
    fn text_conversion_uses_the_supplied_resolver_and_propagates_refusal() {
        let result = convert_scalar(BindValue::Str("needle"), &ValueType::String, |text| {
            assert_eq!(text, "needle");
            Ok(42)
        })
        .unwrap();
        assert!(matches!(result, Some(Const::Word(42))));
        let refused = convert_scalar(BindValue::Str("needle"), &ValueType::String, |_| {
            Err(Error::ForeignPreparedQuery)
        });
        assert!(matches!(refused, Err(Error::ForeignPreparedQuery)));
    }
}
