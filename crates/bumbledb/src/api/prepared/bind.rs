use super::{
    BindValue, Const, Executor, FilterPredicate, ParamArg, ParamSpec, PreparedQuery, PreparedRule,
    ValueType,
};

use crate::error::{Error, Mismatch, Result};
use crate::image::intern::InternerHandle;
use crate::ir::{ParamId, Value};
use crate::obs;
use crate::work::WorkContext;
use bumbledb_theory::schema::IntervalElement;

impl<S> PreparedQuery<S> {
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

    /// typed error before anything else runs. One compare — with the
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
                let interner = InternerHandle::new(self.cache.interner(), work);
                let Some(resolved) = convert_scalar(&interner, value, ty)? else {
                    return Err(Error::ParamTypeMismatch {
                        param,
                        expected: *ty,
                    });
                };
                if let (BindValue::Str(text), ValueType::String) = (value, ty)
                    && let Const::Word(word) = &resolved
                {
                    // A latch is final: the interner is append-only, so a
                    // memoized (text → token) pair never invalidates.
                    let memo = &mut self.param_word_memo[idx];
                    memo.text.clear();
                    memo.text.push_str(text);
                    memo.word = Some(*word);
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
            ValueType::Id128 => 2,
            _ => 1,
        };

        let mut words = match std::mem::replace(&mut self.resolved_params[idx], Const::Word(0)) {
            Const::WordSet(mut words) => {
                words.clear();
                words
            }
            _ => Vec::new(),
        };
        let interner = InternerHandle::new(self.cache.interner(), work);
        for (element, value) in values.iter().enumerate() {
            let Some(word_count) = element_words(&interner, value, expected, &mut words)? else {
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
    interner: &InternerHandle<'_>,
    value: &Value,
    expected: &ValueType,
    out: &mut Vec<u64>,
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
    let Some(resolved) = convert_scalar(interner, element_view(value), expected)? else {
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
        // Id128 elements are two-word spans, exactly like bytes<16>.
        Const::Words(words) => {
            out.extend_from_slice(&words);
            words.len()
        }
        Const::Param(_) | Const::ParamSet(_) | Const::WordSet(_) | Const::PendingIntern { .. } => {
            unreachable!("convert_scalar resolves scalar kinds to inline column form")
        }
    }))
}

pub(super) fn resolve_filters(
    interner: &InternerHandle<'_>,
    plan: &mut crate::plan::fj::ValidatedPlan,
    params: &[Const],
    missed: &[bool],
    out_filters: &mut [Vec<FilterPredicate>],
    out_selections: &mut [Vec<Vec<u64>>],
    latched: &mut u32,
) -> Result<bool> {
    for (occ_idx, occurrence) in plan.occurrences_mut().iter_mut().enumerate() {
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
        for (template, slot) in occurrence.filters.iter_mut().zip(filters.iter_mut()) {
            if !crate::image::view::resolve_filter_into(
                interner, template, params, missed, negated, slot, latched,
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
            if !resolve_selection_into(interner, selection, params, missed, words, latched)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn resolve_selection_into(
    interner: &InternerHandle<'_>,
    selection: &mut crate::plan::fj::Selection,
    params: &[Const],
    missed: &[bool],
    out: &mut Vec<u64>,
    latched: &mut u32,
) -> Result<bool> {
    out.clear();

    if let Const::PendingIntern { bytes } = &selection.value {
        // A latch is final (append-only interner): the template constant
        // becomes a plain word on its first resolution.
        let word = interner.latch(bytes)?;
        selection.value = Const::Word(word);
        *latched += 1;
        obs::event(obs::names::LITERAL_LATCH, obs::TraceArgs::Count(word));
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
                return Ok(false);
            }
            let Const::WordSet(words) = &params[usize::from(param.0)] else {
                unreachable!("validated: a set param resolves to a word set")
            };
            out.extend_from_slice(words);
        }

        Const::WordSet(words) => out.extend_from_slice(words),
        Const::PendingIntern { .. } => unreachable!("latched above"),
    }
    Ok(true)
}

fn element_view(value: &Value) -> BindValue<'_> {
    match value {
        Value::Bool(v) => BindValue::Bool(*v),
        Value::U64(v) => BindValue::U64(*v),
        Value::I64(v) => BindValue::I64(*v),
        Value::F64(v) => BindValue::F64(*v),
        Value::Id128(id) => BindValue::Id128(*id),
        Value::String(text) => BindValue::Str(text),
        Value::FixedBytes(raw) => BindValue::FixedBytes(raw),
        Value::IntervalU64(interval) => BindValue::IntervalU64(interval.start(), interval.end()),
        Value::IntervalI64(interval) => BindValue::IntervalI64(interval.start(), interval.end()),
        Value::IntervalF64(interval) => BindValue::IntervalF64(*interval),
    }
}

fn convert_scalar(
    interner: &InternerHandle<'_>,
    value: BindValue<'_>,
    expected: &ValueType,
) -> Result<Option<Const>> {
    let resolved = match (value, expected) {
        (BindValue::Bool(v), ValueType::Bool) => Const::Byte(u8::from(v)),
        (BindValue::U64(v), ValueType::U64) => Const::Word(v),
        (BindValue::I64(v), ValueType::I64) => Const::Word(i64_word(v)),
        (BindValue::F64(v), ValueType::F64) => Const::Word(v.to_order_key()),
        (BindValue::Id128(id), ValueType::Id128) => {
            let bytes = id.to_bytes();
            let hi = u64::from_be_bytes(bytes[..8].try_into().expect("sixteen bytes"));
            let lo = u64::from_be_bytes(bytes[8..].try_into().expect("sixteen bytes"));
            Const::Words(Box::from([hi, lo]))
        }

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
        (BindValue::Str(text), ValueType::String) => Const::Word(interner.intern_text(text)?),

        _ => return Ok(None),
    };
    Ok(Some(resolved))
}

fn i64_word(value: i64) -> u64 {
    u64::from_be_bytes(crate::encoding::encode_i64(value))
}
