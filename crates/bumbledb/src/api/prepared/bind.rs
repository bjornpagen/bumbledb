use super::{Const, ExecPlan, Executor, FilterPredicate, ParamArg, PreparedQuery, ValueType};

use crate::error::{Error, Result};
use crate::image::view::ResolvedWordSource;
use crate::ir::{CmpOp, ParamId, Value};
use crate::storage::dict;
use crate::storage::env::ReadTxn;

impl PreparedQuery<'_> {
    /// Rebuilds the executor scratch at a different batch size — the
    /// tuning/test surface for D4's measurement-owned constant. Allocation
    /// happens here, outside any measured window. A no-op for guard
    /// probes. Hidden: a measurement affordance, not a knob on the
    /// no-knobs surface (`docs/architecture/00-product.md`).
    #[doc(hidden)]
    pub fn set_batch_size(&mut self, batch: usize) {
        if let ExecPlan::FreeJoin(plan) = &self.plan {
            self.executor = Some(Executor::with_batch_size(plan, batch));
        }
    }

    /// The identity check at every execution entry (`execute` and
    /// `profile`; `execute_collect` and `explain` route through them):
    /// a snapshot of any environment other than the preparing one is a
    /// typed error before anything else runs. One u64 compare — with the
    /// entry guarded, the view memo needs no environment epoch in its
    /// generation keys.
    pub(super) fn check_snapshot(&self, txn: &ReadTxn<'_>) -> Result<()> {
        if txn.env_instance() == self.env_instance {
            Ok(())
        } else {
            Err(Error::ForeignPreparedQuery)
        }
    }

    /// Binds and converts all-scalar parameters (the `&[Value]` entry;
    /// set-typed params reject the scalar shape — the internal set path
    /// is [`PreparedQuery::bind_param_args`], PRD 20 owns its public
    /// rendering).
    pub(super) fn bind_params(&mut self, txn: &ReadTxn<'_>, params: &[Value]) -> Result<()> {
        self.begin_bind(params.len())?;
        for (idx, value) in params.iter().enumerate() {
            self.bind_one(txn, idx, ParamArg::Scalar(value))?;
        }
        Ok(())
    }

    /// Binds mixed scalar/set parameter arguments.
    #[allow(dead_code)] // reader: PRD 20's public bind rendering
                        // (tests drive it meanwhile)
    pub(crate) fn bind_param_args(
        &mut self,
        txn: &ReadTxn<'_>,
        args: &[ParamArg<'_>],
    ) -> Result<()> {
        self.begin_bind(args.len())?;
        for (idx, arg) in args.iter().enumerate() {
            self.bind_one(txn, idx, *arg)?;
        }
        Ok(())
    }

    /// Count check + slot sizing (pooled: the resolved/missed slots keep
    /// their capacity — and a set slot its `WordSet` `Vec` — across
    /// executions).
    fn begin_bind(&mut self, supplied: usize) -> Result<()> {
        if supplied != self.param_types.len() {
            return Err(Error::ParamCountMismatch {
                expected: self.param_types.len(),
                supplied,
            });
        }
        if self.resolved_params.len() != supplied {
            self.resolved_params.resize(supplied, Const::Word(0));
            self.missed_params.resize(supplied, false);
        }
        Ok(())
    }

    /// Binds one parameter slot in place.
    fn bind_one(&mut self, txn: &ReadTxn<'_>, idx: usize, arg: ParamArg<'_>) -> Result<()> {
        let expected = &self.param_types[idx];
        let mismatch = || Error::ParamTypeMismatch {
            param: ParamId(u16::try_from(idx).expect("param ids fit u16")),
            expected: expected.clone(),
        };
        match (self.param_is_set[idx], arg) {
            (false, ParamArg::Scalar(value)) => {
                let (resolved, missed) = bind_scalar(txn, idx, value, expected)?;
                self.resolved_params[idx] = resolved;
                self.missed_params[idx] = missed;
                Ok(())
            }
            (true, ParamArg::Set(values)) => {
                // Pooled storage: steal the slot's previous `WordSet` so
                // a warm re-bind (any size within the documented
                // assumption) reuses its capacity.
                let mut words =
                    match std::mem::replace(&mut self.resolved_params[idx], Const::Word(0)) {
                        Const::WordSet(mut words) => {
                            words.clear();
                            words
                        }
                        _ => Vec::new(),
                    };
                for value in values {
                    words.push(element_word(txn, idx, value, expected)?);
                }
                // Sets are sets: sorted, deduplicated
                // (docs/architecture/20-query-ir.md, § param sets).
                words.sort_unstable();
                words.dedup();
                // Per-element intern misses resolved to the never-minted
                // sentinel; a sentinel matches nothing under `Eq`, so
                // dropping it here is the same semantics with a smaller
                // probe set ("out-of-vocabulary elements contribute
                // nothing"). Only the intern path mints sentinels —
                // numeric u64::MAX elements are real values and stay.
                if matches!(expected, ValueType::String | ValueType::Bytes) {
                    while words.last() == Some(&dict::SENTINEL_ID) {
                        words.pop();
                    }
                }
                // The empty set matches nothing — the `Eq`-miss
                // short-circuit machinery, applied where sound
                // (positive occurrences; `resolve_predicates` reads the
                // polarity).
                self.missed_params[idx] = words.is_empty();
                self.resolved_params[idx] = Const::WordSet(words);
                Ok(())
            }
            _ => Err(mismatch()),
        }
    }
}

/// One set element's column word; a String/Bytes miss resolves to the
/// never-minted sentinel intern id (per-element miss semantics,
/// `docs/architecture/20-query-ir.md`).
fn element_word(
    txn: &ReadTxn<'_>,
    index: usize,
    value: &Value,
    expected: &ValueType,
) -> Result<u64> {
    let (resolved, _) = bind_scalar(txn, index, value, expected)?;
    Ok(match resolved {
        Const::Word(word) => word,
        Const::Byte(byte) => u64::from(byte),
        Const::Interval { .. } => {
            unreachable!("validated: no interval-typed param sets (IntervalParamSet)")
        }
        Const::Param(_) | Const::ParamSet(_) | Const::WordSet(_) | Const::PendingIntern { .. } => {
            unreachable!("bind_scalar resolves to column form")
        }
    })
}

/// Resolves every occurrence's symbolic predicate constants for this
/// execution — residual filters into `out_filters`, selection key words
/// into `out_selections`, both **in place** (the lists' shapes are plan
/// constants, so a warm execution rewrites slots and reuses every
/// `WordSet` capacity). `Ok(false)` = a dictionary miss or empty set
/// under an `Eq` predicate of a **positive** occurrence, which empties
/// the whole conjunctive query (sound for `Eq` on positive occurrences
/// only — on a negated occurrence the same miss just matches nothing,
/// so its anti-probe never rejects; a missed value under `Ne` resolves
/// to the sentinel id and matches everything).
pub(super) fn resolve_predicates(
    txn: &ReadTxn<'_>,
    plan: &crate::plan::fj::ValidatedPlan,
    params: &[Const],
    missed: &[bool],
    out_filters: &mut [Vec<FilterPredicate>],
    out_selections: &mut [Vec<Vec<u64>>],
) -> Result<bool> {
    for (occ_idx, occurrence) in plan.occurrences().iter().enumerate() {
        let negated = plan.is_negated(occurrence.occ_id);
        let filters = &mut out_filters[occ_idx];
        if filters.len() != occurrence.filters.len() {
            // First execution (or a plan-shape change, which cannot
            // happen): populate the slots; every later pass rewrites
            // them in place.
            filters.clear();
            filters.extend(occurrence.filters.iter().cloned());
        }
        for (template, slot) in occurrence.filters.iter().zip(filters.iter_mut()) {
            if !resolve_filter_into(txn, template, params, missed, negated, slot)? {
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
            if !resolve_selection_into(txn, selection, params, missed, words)? {
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
    selection: &crate::plan::fj::Selection,
    params: &[Const],
    missed: &[bool],
    out: &mut Vec<u64>,
) -> Result<bool> {
    out.clear();
    let push_const = |constant: &Const, out: &mut Vec<u64>| match constant {
        Const::Word(word) => out.push(*word),
        Const::Byte(byte) => out.push(u64::from(*byte)),
        Const::Interval { start, end } => out.extend([*start, *end]),
        Const::WordSet(_) | Const::Param(_) | Const::ParamSet(_) | Const::PendingIntern { .. } => {
            unreachable!("bind resolved params to column form")
        }
    };
    match &selection.value {
        value @ (Const::Word(_) | Const::Byte(_) | Const::Interval { .. }) => {
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
        Const::WordSet(_) => unreachable!("lowering emits ParamSet markers, never resolved sets"),
        Const::PendingIntern { tag, bytes } => match dict::lookup_tagged(txn, *tag, bytes)? {
            Some(word) => out.push(word),
            None => return Ok(false),
        },
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
fn resolve_filter_into(
    txn: &ReadTxn<'_>,
    template: &FilterPredicate,
    params: &[Const],
    missed: &[bool],
    negated: bool,
    dst: &mut FilterPredicate,
) -> Result<bool> {
    match template {
        FilterPredicate::Compare { field, op, value } => {
            let resolved = match value {
                Const::Word(_) | Const::Byte(_) | Const::Interval { .. } => value.clone(),
                Const::Param(param) => {
                    if missed[usize::from(param.0)] && *op == CmpOp::Eq && !negated {
                        return Ok(false);
                    }
                    // A negated Eq miss keeps the sentinel word bind
                    // stored — it matches nothing.
                    params[usize::from(param.0)].clone()
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
                Const::WordSet(_) => unreachable!("templates carry ParamSet markers"),
                Const::PendingIntern { tag, bytes } => match dict::lookup_tagged(txn, *tag, bytes)?
                {
                    Some(id) => Const::Word(id),
                    None if *op == CmpOp::Eq && !negated => return Ok(false),
                    None => Const::Word(dict::SENTINEL_ID),
                },
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
        // Constant-free kinds copy through (cheap: field ids only).
        FilterPredicate::FieldsCompare { .. }
        | FilterPredicate::FieldsOverlap { .. }
        | FilterPredicate::FieldsContain { .. }
        | FilterPredicate::FieldsContainPoint { .. } => dst.clone_from(template),
    }
    Ok(true)
}

/// Writes a `Compare` shape into the slot. `value: None` keeps the
/// slot's existing value word untouched (the WordSet-reuse path writes
/// it separately).
fn write_compare(
    dst: &mut FilterPredicate,
    field: crate::schema::FieldId,
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

/// Converts a bound scalar param value to column form. A String or Bytes
/// value that was never interned resolves to the sentinel intern id,
/// flagged `missed` so `Eq` uses can short-circuit to the empty result.
fn bind_scalar(
    txn: &ReadTxn<'_>,
    index: usize,
    value: &Value,
    expected: &ValueType,
) -> Result<(Const, bool)> {
    // The shared compatibility check (kind, enum range, UTF-8) — one rule
    // with validation and the dynamic write path.
    if crate::ir::value_matches(value, expected).is_err() {
        return Err(Error::ParamTypeMismatch {
            param: ParamId(u16::try_from(index).expect("param ids fit u16")),
            expected: expected.clone(),
        });
    }
    let resolved = match value {
        Value::Bool(v) => Const::Byte(u8::from(*v)),
        Value::Enum(ordinal) => Const::Byte(*ordinal),
        Value::U64(v) => Const::Word(*v),
        Value::I64(v) => Const::Word(i64_word(*v)),
        Value::IntervalU64(start, end) => Const::Interval {
            start: *start,
            end: *end,
        },
        Value::IntervalI64(start, end) => Const::Interval {
            start: i64_word(*start),
            end: i64_word(*end),
        },
        Value::String(bytes) => {
            let text = std::str::from_utf8(bytes).expect("value_matches validated UTF-8");
            match dict::lookup_str(txn, text)? {
                Some(id) => Const::Word(id),
                None => return Ok((Const::Word(dict::SENTINEL_ID), true)),
            }
        }
        Value::Bytes(bytes) => match dict::lookup_bytes(txn, bytes)? {
            Some(id) => Const::Word(id),
            None => return Ok((Const::Word(dict::SENTINEL_ID), true)),
        },
    };
    Ok((resolved, false))
}

/// The biased I64 column word (u64 word order equals i64 value order).
fn i64_word(value: i64) -> u64 {
    u64::from_be_bytes(crate::encoding::encode_i64(value))
}
