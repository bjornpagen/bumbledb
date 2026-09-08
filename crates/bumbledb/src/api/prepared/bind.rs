use super::{
    BindValue, Const, Executor, FilterPredicate, ParamArg, ParamSpec, PreparedQuery, PreparedRule,
    ValueType,
};

use crate::error::{Error, Mismatch, Result};
use crate::image::intern::InternerHandle;
use crate::image::view::ResolvedWords;
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
                rule.memo.invalidate();
            }
        });
        self.visit_rec_arms_mut(|arm| {
            forget_resolved_text(arm);
            arm.memo.invalidate();
        });
        self.derived = super::reach::DerivedImages::default();
        self.resolve_memo.clear();
        for memo in &mut self.param_word_memo {
            memo.text = None;
            memo.word = None;
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
                    && let Some(resolved) = self.param_word_memo[idx].resolved(text)
                {
                    self.resolved_params[idx] = resolved;
                    self.missed_params[idx] = false;
                    return Ok(());
                }
                let owner = &self.text_generation;
                let cache = &self.cache;
                let Some(resolved) = convert_scalar(value, ty, |text| {
                    let generation = owner
                        .as_ref()
                        .and_then(crate::work::cache::WeakGenerationHandle::upgrade)
                        .unwrap_or_else(|| cache.acquire());
                    let interner = InternerHandle::new(&generation, work);
                    interner.intern(text).map(Const::Text)
                })?
                else {
                    return Err(Error::ParamTypeMismatch {
                        param,
                        expected: *ty,
                    });
                };
                if let (BindValue::Str(_), ValueType::String) = (value, ty) {
                    self.param_word_memo[idx].remember(&resolved);
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
            _ => Box::default(),
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
                interner.intern(text).map(Const::Text)
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

            if point && words.words.last() == Some(&u64::MAX) {
                words.clear();
                self.resolved_params[idx] = Const::WordSet(words);
                return Err(Error::PointParamAtCeiling { param });
            }
        }

        if element_width == 1 {
            words.words.sort_unstable();
            words.words.dedup();
            words.dedup_texts();
        } else {
            match element_width {
                2 => sort_dedup_spans::<2>(&mut words.words),
                3 => sort_dedup_spans::<3>(&mut words.words),
                4 => sort_dedup_spans::<4>(&mut words.words),
                5 => sort_dedup_spans::<5>(&mut words.words),
                6 => sort_dedup_spans::<6>(&mut words.words),
                7 => sort_dedup_spans::<7>(&mut words.words),
                8 => sort_dedup_spans::<8>(&mut words.words),
                _ => unreachable!("bytes<N> spans are 2..=8 words (N ≤ 64)"),
            }
        }

        // The empty set matches nothing under Eq on a positive occurrence;
        // the miss short-circuit machinery carries exactly that.
        self.missed_params[idx] = words.words.is_empty();
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
    out: &mut ResolvedWords,
    intern_text: impl FnOnce(&str) -> Result<Const>,
) -> Result<Option<usize>> {
    if let ValueType::FixedBytes { len } = expected {
        let Value::FixedBytes(raw) = value else {
            return Ok(None);
        };
        if raw.len() != usize::from(*len) {
            return Ok(None);
        }
        let (words, count) = crate::ir::normalize::fixed_bytes_word_buf(raw);
        out.words.extend_from_slice(&words[..count]);
        return Ok(Some(count));
    }
    let Some(resolved) = convert_scalar(element_view(value), expected, intern_text)? else {
        return Ok(None);
    };
    Ok(Some(match resolved {
        Const::Word(scalar) => {
            out.words.push(scalar);
            1
        }
        Const::Byte(byte) => {
            out.words.push(u64::from(byte));
            1
        }
        Const::Interval { .. } => {
            unreachable!("validated: no interval-typed param sets (IntervalParamSet)")
        }
        Const::Text(text) => {
            out.push_text(text);
            1
        }
        // Uuid elements are two-word spans, exactly like bytes<16>.
        Const::Words(words) => {
            out.words.extend_from_slice(&words);
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
    pub work: &'a WorkContext,
    pub params: &'a [Const],
    pub missed: &'a [bool],
}

impl LiteralResolution<'_, '_> {
    pub(super) fn filters(
        &mut self,
        plan: &crate::plan::fj::ValidatedPlan,
        out_filters: &mut [Vec<FilterPredicate>],
        out_selections: &mut [Vec<ResolvedWords>],
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
                selections.resize_with(occurrence.selections.len(), ResolvedWords::default);
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
        self.work.checkpoint().map_err(super::source::work_error)?;
        crate::image::view::resolve_filter_into(
            self.interner,
            template,
            self.params,
            self.missed,
            negated,
            slot,
        )
    }

    pub(super) fn selection(
        &mut self,
        selection: &crate::plan::fj::Selection,
        out: &mut ResolvedWords,
    ) -> Result<bool> {
        self.work.checkpoint().map_err(super::source::work_error)?;
        if let Const::PendingIntern { bytes } = &selection.value {
            if matches!(out.texts.as_slice(), [text] if out.words.as_slice() == [text.word]) {
                return Ok(true);
            }
            out.clear();
            let text = std::str::from_utf8(bytes)
                .expect("IR string literals are UTF-8 by construction (Value::String)");
            out.push_text(self.interner.intern(text)?);
            return Ok(true);
        }
        out.clear();
        let push_const = |constant: &Const, out: &mut ResolvedWords| match constant {
            Const::Word(word) => out.words.push(*word),
            Const::Text(text) => out.push_text(text.clone()),
            Const::Byte(byte) => out.words.push(u64::from(*byte)),
            Const::Words(words) => out.words.extend_from_slice(words),
            Const::Interval { start, end } => out.words.extend([*start, *end]),
            Const::WordSet(_)
            | Const::Param(_)
            | Const::ParamSet(_)
            | Const::PendingIntern { .. } => {
                unreachable!("bind resolved parameters to column form")
            }
        };
        match &selection.value {
            value @ (Const::Word(_)
            | Const::Text(_)
            | Const::Byte(_)
            | Const::Words(_)
            | Const::Interval { .. }) => {
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
                out.clone_from(words);
            }

            Const::WordSet(words) => out.clone_from(words),
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
    intern_text: impl FnOnce(&str) -> Result<Const>,
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
        // The resolved text carries its canonical owner. An unstored text
        // is an ordinary unequal token, not a missing parameter.
        (BindValue::Str(text), ValueType::String) => intern_text(text)?,

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

impl super::ParamWordMemo {
    fn resolved(&self, text: &str) -> Option<Const> {
        let word = self.word?;
        if self.text.as_deref() != Some(text) {
            return None;
        }
        Some(Const::Text(crate::image::intern::InternedText {
            word,
            text: std::sync::Arc::clone(self.text.as_ref()?),
        }))
    }

    fn remember(&mut self, resolved: &Const) {
        let Const::Text(token) = resolved else {
            unreachable!("text conversion pins its token")
        };
        self.text = Some(std::sync::Arc::clone(&token.text));
        self.word = Some(token.word);
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
    fn resolved_text_filters_and_selections_keep_owners_and_reuse_clone_storage() {
        let generation = crate::image::test_generation();
        let work = crate::api::prepared::source::unbounded_work();
        let handle = InternerHandle::new(&generation, &work);
        let mut source = ResolvedWords::default();
        for text in ["alpha", "beta", "alpha"] {
            let owned = handle.intern(text).unwrap();
            source.push_text(owned);
        }
        source.words.sort_unstable();
        source.words.dedup();
        source.dedup_texts();
        assert_eq!(source.words.len(), 2);
        assert_eq!(source.texts.len(), 2, "one owner per distinct set element");
        let params = [Const::WordSet(Box::new(source))];
        let template = FilterPredicate::Compare {
            field: crate::image::view::OperandAddr::from_slot(0),
            op: crate::ir::WordCmp::Eq,
            value: Const::ParamSet(ParamId(0)),
        };
        let selection = crate::plan::fj::Selection {
            field: bumbledb_theory::schema::FieldId(0),
            value: Const::ParamSet(ParamId(0)),
        };
        let mut filter = template.clone();
        let mut keys = ResolvedWords::default();
        let mut resolution = LiteralResolution {
            interner: &handle,
            work: &work,
            params: &params,
            missed: &[false],
        };
        assert!(resolution.filter(&template, false, &mut filter).unwrap());
        assert!(resolution.selection(&selection, &mut keys).unwrap());
        let before = crate::alloc_counter::snapshot().window;
        for _ in 0..32 {
            assert!(resolution.filter(&template, false, &mut filter).unwrap());
            assert!(resolution.selection(&selection, &mut keys).unwrap());
        }
        let after = crate::alloc_counter::snapshot().window;
        eprintln!(
            "resolved text set, 32 filter+selection copies: allocations={}, bytes={}",
            after.allocs - before.allocs,
            after.alloc_bytes - before.alloc_bytes
        );
        #[cfg(feature = "alloc-counter")]
        assert_eq!(
            after.allocs - before.allocs,
            0,
            "warm resolution must clone into reusable vectors"
        );
        drop(params);
        generation.lock_resolver().reclaim_unowned();
        assert!(generation.resolver().lookup("alpha").is_some());
        assert!(generation.resolver().lookup("beta").is_some());
        drop(filter);
        generation.lock_resolver().reclaim_unowned();
        assert_eq!(
            generation.lock_resolver().len(),
            2,
            "selection owns its text independently"
        );
        drop(keys);
        generation.lock_resolver().reclaim_unowned();
        assert_eq!(generation.lock_resolver().len(), 0);
    }

    #[test]
    fn resolved_scalar_text_filter_and_literal_selection_keep_the_same_text_owner() {
        let generation = crate::image::test_generation();
        let work = crate::api::prepared::source::unbounded_work();
        let handle = InternerHandle::new(&generation, &work);
        let params = [Const::Text(handle.intern("needle").unwrap())];
        let template = FilterPredicate::Compare {
            field: crate::image::view::OperandAddr::from_slot(0),
            op: crate::ir::WordCmp::Eq,
            value: Const::Param(ParamId(0)),
        };
        let selection = crate::plan::fj::Selection {
            field: bumbledb_theory::schema::FieldId(0),
            value: Const::PendingIntern {
                bytes: Box::from(b"needle".as_slice()),
            },
        };
        let mut filter = template.clone();
        let mut keys = ResolvedWords::default();
        let mut resolution = LiteralResolution {
            interner: &handle,
            work: &work,
            params: &params,
            missed: &[false],
        };
        assert!(resolution.filter(&template, false, &mut filter).unwrap());
        assert!(resolution.selection(&selection, &mut keys).unwrap());
        let Const::Text(parameter) = &params[0] else {
            panic!("owned scalar text")
        };
        let FilterPredicate::Compare {
            value: Const::Text(resolved),
            ..
        } = &filter
        else {
            panic!("owned filter text")
        };
        assert!(std::sync::Arc::ptr_eq(&parameter.text, &resolved.text));
        assert!(std::sync::Arc::ptr_eq(&parameter.text, &keys.texts[0].text));
        drop(params);
        generation.lock_resolver().reclaim_unowned();
        assert!(generation.resolver().lookup("needle").is_some());
        drop(filter);
        generation.lock_resolver().reclaim_unowned();
        assert!(generation.resolver().lookup("needle").is_some());
        drop(keys);
        generation.lock_resolver().reclaim_unowned();
        assert_eq!(generation.resolver().lookup("needle"), None);
        #[cfg(target_pointer_width = "64")]
        assert_eq!(
            std::mem::size_of::<Const>(),
            32,
            "text-set ownership cannot inflate every scalar constant"
        );
    }

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
        let mut words = ResolvedWords::default();
        assert_eq!(
            element_words(&Value::U64(7), &ValueType::U64, &mut words, |_| panic!(
                "nontext set resolver"
            ))
            .unwrap(),
            Some(1)
        );
        assert_eq!(words.words, [7]);
    }

    #[test]
    fn text_conversion_uses_the_supplied_resolver_and_propagates_refusal() {
        let result = convert_scalar(BindValue::Str("needle"), &ValueType::String, |text| {
            assert_eq!(text, "needle");
            Ok(Const::Word(42))
        })
        .unwrap();
        assert!(matches!(result, Some(Const::Word(42))));
        let refused = convert_scalar(BindValue::Str("needle"), &ValueType::String, |_| {
            Err(Error::ForeignPreparedQuery)
        });
        assert!(matches!(refused, Err(Error::ForeignPreparedQuery)));
    }
}
