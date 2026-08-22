//! The statically-empty fold: the database analog of comptime-unreachable, run at
//! the end of each rule's lowering, over each **participating**
//! occurrence's own filter list.
//! Two jobs, one pass:
//! 1. **Range folding** — a conjunction of constant order filters on one
//! u64/i64 slot collapses into a single `[lo, hi]` summary over
//! no new kernel. The replacement's word-level soundness is proved
//! (`lean/Bumbledb/Exec/Rewrites.lean`: `range_summary_replacement`,
//! a membership set empty after sentinel-trim, or intersected with an
//! must not judge. Interval variables fold via their two slot summaries
//! invariant `start < end` is data, not plan knowledge). Negated

use std::collections::BTreeMap;

use super::Occurrence;
use crate::encoding::decode_i64;
use crate::image::view::{Const, FilterPredicate, IntervalConst, ViewWordSource};
use crate::ir::render::{literal, mask_names};
use crate::ir::{Value, WordCmp};
use crate::schema::{Relation, Schema};
use bumbledb_theory::allen::AllenMask;
use bumbledb_theory::schema::{FieldId, IntervalElement, ValueType};

#[cfg(test)]
thread_local! {

    /// 2026-07-20 hard-delete ruling,

    static DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `f` with the fold bypassed on this thread — the fold-preservation
/// differential's off switch. Restores on unwind.
#[cfg(test)]
pub fn with_fold_disabled<T>(f: impl FnOnce() -> T) -> T {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            DISABLED.with(|d| d.set(false));
        }
    }
    DISABLED.with(|d| d.set(true));
    let _reset = Reset;
    f()
}

pub(super) fn fold(schema: &Schema, occurrences: &mut [Occurrence]) -> Option<String> {
    #[cfg(test)]
    if DISABLED.with(std::cell::Cell::get) {
        return None;
    }
    for occurrence in occurrences.iter_mut() {
        if !occurrence.role.participates() {
            continue;
        }
        if let Some(reason) = fold_occurrence(schema, occurrence) {
            return Some(reason);
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RangeSummary {
    lo: u64,
    hi: u64,
}

impl RangeSummary {
    fn new() -> Self {
        Self {
            lo: 0,
            hi: u64::MAX,
        }
    }

    fn narrow(&mut self, op: WordCmp, word: u64) {
        match op {
            WordCmp::Ge => self.lo = self.lo.max(word),
            WordCmp::Le => self.hi = self.hi.min(word),
            WordCmp::Gt => match word.checked_add(1) {
                Some(above) => self.lo = self.lo.max(above),
                None => self.mark_empty(),
            },
            WordCmp::Lt => match word.checked_sub(1) {
                Some(below) => self.hi = self.hi.min(below),
                None => self.mark_empty(),
            },
            WordCmp::Eq | WordCmp::Ne => {
                unreachable!("only order filters narrow the summary")
            }
        }
    }

    fn mark_empty(&mut self) {
        self.lo = 1;
        self.hi = 0;
    }
}

fn range_is_empty(summary: &RangeSummary) -> bool {
    summary.lo > summary.hi
}

fn eq_conflicts(first: &Const, second: &Const) -> bool {
    match (first, second) {
        (Const::WordSet(words), Const::Word(word)) | (Const::Word(word), Const::WordSet(words)) => {
            set_refutes_eq(words, Some(*word))
        }
        (Const::Word(_), Const::Word(_))
        | (Const::Byte(_), Const::Byte(_))
        | (Const::Words(_), Const::Words(_))
        | (Const::Interval { .. }, Const::Interval { .. })
        | (Const::PendingIntern { .. }, Const::PendingIntern { .. }) => first != second,
        _ => false,
    }
}

fn eq_outside_range(word: u64, summary: &RangeSummary) -> bool {
    word < summary.lo || word > summary.hi
}

fn set_refutes_eq(words: &[u64], eq: Option<u64>) -> bool {
    let mut live = words
        .iter()
        .filter(|word| **word != crate::storage::dict::SENTINEL_ID);
    match eq {
        Some(eq) => !live.any(|word| *word == eq),
        None => live.next().is_none(),
    }
}

fn allen_refuted(lhs: (u64, u64), mask: AllenMask, rhs: (u64, u64)) -> bool {
    if lhs.0 >= lhs.1 || rhs.0 >= rhs.1 {
        return false;
    }
    !mask.contains(crate::allen::classify_bounds(
        &lhs.0, &lhs.1, &rhs.0, &rhs.1,
    ))
}

fn point_outside(interval: (u64, u64), point: u64) -> bool {
    point < interval.0 || point >= interval.1
}

fn fold_occurrence(schema: &Schema, occurrence: &mut Occurrence) -> Option<String> {
    let relation = schema.relation(occurrence.bind.edb()?);

    let mut eqs: BTreeMap<FieldId, Const> = BTreeMap::new();
    for filter in &occurrence.filters {
        let FilterPredicate::Compare {
            field,
            op: WordCmp::Eq,
            value,
        } = filter
        else {
            continue;
        };
        if matches!(value, Const::Param(_) | Const::ParamSet(_)) {
            continue;
        }
        if let Const::WordSet(words) = value {
            // Rule (d), the set alone: empty after sentinel-trim.
            if set_refutes_eq(words, None) {
                return Some(format!(
                    "{}: {} ∈ {{}}",
                    relation.name(),
                    relation.field(field.field()).name
                ));
            }
        }
        match eqs.get(&field.field()) {
            None => {
                eqs.insert(field.field(), value.clone());
            }
            Some(prior) => {
                if eq_conflicts(prior, value) {
                    return Some(eq_pair_picture(relation, field.field(), prior, value));
                }
            }
        }
    }

    let mut ranges: BTreeMap<FieldId, (RangeSummary, usize)> = BTreeMap::new();
    for filter in &occurrence.filters {
        let Some((field, op, word)) = constant_order_bound(filter) else {
            continue;
        };
        let (summary, constituents) = ranges
            .entry(field)
            .or_insert_with(|| (RangeSummary::new(), 0));
        summary.narrow(op, word);
        *constituents += 1;
    }
    for (field, (summary, _)) in &ranges {
        if range_is_empty(summary) {
            return Some(order_filters_picture(relation, *field, &occurrence.filters));
        }

        if let Some(Const::Word(eq_word)) = eqs.get(field)
            && eq_outside_range(*eq_word, summary)
        {
            return Some(eq_outside_picture(relation, *field, summary, *eq_word));
        }
    }

    if let Some(reason) = interval_contradictions(relation, &eqs, &occurrence.filters) {
        return Some(reason);
    }

    emit(occurrence, &eqs, &ranges);
    None
}

fn interval_contradictions(
    relation: &Relation,
    eqs: &BTreeMap<FieldId, Const>,
    filters: &[FilterPredicate],
) -> Option<String> {
    let mut interval_pins: BTreeMap<FieldId, (u64, u64)> = BTreeMap::new();
    for (field, value) in eqs {
        if let Const::Interval { start, end } = value {
            interval_pins.insert(*field, (*start, *end));
        }
    }
    for filter in filters {
        if let FilterPredicate::FieldAllen {
            field,
            other: IntervalConst::Interval { start, end },
            mask,
        } = filter
            && *mask == AllenMask::EQUALS
        {
            interval_pins.entry(field.field()).or_insert((*start, *end));
        }
    }
    for filter in filters {
        match filter {
            FilterPredicate::FieldAllen {
                field,
                other: IntervalConst::Interval { start, end },
                mask,
            } => {
                if let Some(pin) = interval_pins.get(&field.field())
                    && allen_refuted(*pin, *mask, (*start, *end))
                {
                    return Some(field_allen_picture(
                        relation,
                        field.field(),
                        *pin,
                        *mask,
                        (*start, *end),
                    ));
                }
            }

            FilterPredicate::FieldsAllen { left, right, mask } => {
                if let (Some(lhs), Some(rhs)) = (
                    interval_pins.get(&left.field()),
                    interval_pins.get(&right.field()),
                ) && allen_refuted(*lhs, *mask, *rhs)
                {
                    return Some(fields_allen_picture(
                        relation,
                        left.field(),
                        *lhs,
                        *mask,
                        right.field(),
                        *rhs,
                    ));
                }
            }

            FilterPredicate::PointIn {
                field,
                point: ViewWordSource::Word(point),
            } => {
                if let Some(pin) = interval_pins.get(&field.field())
                    && point_outside(*pin, *point)
                {
                    return Some(point_in_picture(relation, field.field(), *pin, *point));
                }
            }

            FilterPredicate::FieldWithin {
                field,
                outer: IntervalConst::Interval { start, end },
            } => {
                if let Some(Const::Word(point)) = eqs.get(&field.field())
                    && point_outside((*start, *end), *point)
                {
                    return Some(field_within_picture(
                        relation,
                        field.field(),
                        *point,
                        (*start, *end),
                    ));
                }
            }
            _ => {}
        }
    }
    None
}

fn constant_order_bound(filter: &FilterPredicate) -> Option<(FieldId, WordCmp, u64)> {
    let FilterPredicate::Compare {
        field,
        op: op @ (WordCmp::Lt | WordCmp::Le | WordCmp::Gt | WordCmp::Ge),
        value: Const::Word(word),
    } = filter
    else {
        return None;
    };
    Some((field.field(), *op, *word))
}

fn emit(
    occurrence: &mut Occurrence,
    eqs: &BTreeMap<FieldId, Const>,
    ranges: &BTreeMap<FieldId, (RangeSummary, usize)>,
) {
    let mut replacements: BTreeMap<FieldId, Vec<FilterPredicate>> = BTreeMap::new();
    for (field, (summary, constituents)) in ranges {
        let pinned = matches!(eqs.get(field), Some(Const::Word(_)));
        if pinned {
            // and the bounds are implied: drop every constituent.
            replacements.insert(*field, Vec::new());
        } else if *constituents >= 2 {
            let mut emitted = Vec::with_capacity(2);
            if summary.lo > 0 {
                emitted.push(FilterPredicate::Compare {
                    field: (*field).into(),
                    op: WordCmp::Ge,
                    value: Const::Word(summary.lo),
                });
            }
            if summary.hi < u64::MAX {
                emitted.push(FilterPredicate::Compare {
                    field: (*field).into(),
                    op: WordCmp::Le,
                    value: Const::Word(summary.hi),
                });
            }
            replacements.insert(*field, emitted);
        }
    }
    if replacements.is_empty() {
        return;
    }

    let mut emitted: Vec<FieldId> = Vec::new();
    let filters = std::mem::take(&mut occurrence.filters);
    for filter in filters {
        match constant_order_bound(&filter) {
            Some((field, ..)) if replacements.contains_key(&field) => {
                if !emitted.contains(&field) {
                    emitted.push(field);
                    occurrence
                        .filters
                        .extend(replacements[&field].iter().cloned());
                }
            }
            _ => occurrence.filters.push(filter),
        }
    }
}

pub(crate) fn decoded_scalar(value_type: &ValueType, word: u64) -> Value {
    match value_type {
        ValueType::I64 => Value::I64(decode_i64(word.to_be_bytes())),
        ValueType::Bool => Value::Bool(word != 0),
        _ => Value::U64(word),
    }
}

pub(crate) fn decoded_interval(value_type: &ValueType, pair: (u64, u64)) -> Value {
    match value_type {
        ValueType::Interval {
            element: IntervalElement::I64,
        }
        | ValueType::FixedInterval {
            element: IntervalElement::I64,
            ..
        } => Value::IntervalI64(
            bumbledb_theory::Interval::<i64>::new(
                decode_i64(pair.0.to_be_bytes()),
                decode_i64(pair.1.to_be_bytes()),
            )
            .expect("validated interval constant"),
        ),
        _ => Value::IntervalU64(
            bumbledb_theory::Interval::<u64>::new(pair.0, pair.1)
                .expect("validated interval constant"),
        ),
    }
}

pub(crate) fn render_const(out: &mut String, value_type: &ValueType, value: &Const) {
    match value {
        Const::Word(word) => literal(out, &decoded_scalar(value_type, *word)),
        Const::Byte(byte) => literal(out, &Value::Bool(*byte != 0)),
        Const::Interval { start, end } => {
            literal(out, &decoded_interval(value_type, (*start, *end)));
        }
        Const::Words(words) => {
            let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_be_bytes()).collect();
            let len = match value_type {
                ValueType::FixedBytes { len } => usize::from(*len).min(bytes.len()),
                _ => bytes.len(),
            };
            literal(out, &Value::FixedBytes(bytes[..len].into()));
        }
        Const::PendingIntern { bytes } => literal(
            out,
            &Value::String(
                std::str::from_utf8(bytes)
                    .expect("pending intern is UTF-8")
                    .into(),
            ),
        ),
        Const::WordSet(words) => {
            out.push('{');
            for (index, word) in words.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                literal(out, &decoded_scalar(value_type, *word));
            }
            out.push('}');
        }
        Const::Param(_) | Const::ParamSet(_) => unreachable!("params never fold"),
    }
}

fn eq_pair_picture(relation: &Relation, field: FieldId, first: &Const, second: &Const) -> String {
    let descriptor = relation.field(field);
    let mut out = format!("{}: ", relation.name());
    for (index, value) in [first, second].into_iter().enumerate() {
        if index > 0 {
            out.push_str(" ∧ ");
        }
        out.push_str(&descriptor.name);
        out.push_str(if matches!(value, Const::WordSet(_)) {
            " ∈ "
        } else {
            " == "
        });
        render_const(&mut out, &descriptor.value_type, value);
    }
    out
}

fn order_filters_picture(
    relation: &Relation,
    field: FieldId,
    filters: &[FilterPredicate],
) -> String {
    let descriptor = relation.field(field);
    let mut out = format!("{}: ", relation.name());
    let mut first = true;
    for filter in filters {
        let Some((bound_field, op, word)) = constant_order_bound(filter) else {
            continue;
        };
        if bound_field != field {
            continue;
        }
        if !first {
            out.push_str(" ∧ ");
        }
        first = false;
        out.push_str(&descriptor.name);
        out.push_str(match op {
            WordCmp::Lt => " < ",
            WordCmp::Le => " <= ",
            WordCmp::Gt => " > ",
            WordCmp::Ge => " >= ",
            _ => unreachable!("constant_order_bound admits order operators only"),
        });
        literal(&mut out, &decoded_scalar(&descriptor.value_type, word));
    }
    out
}

fn eq_outside_picture(
    relation: &Relation,
    field: FieldId,
    summary: &RangeSummary,
    eq_word: u64,
) -> String {
    let descriptor = relation.field(field);
    let mut out = format!("{}: {} ∈ [", relation.name(), descriptor.name);
    literal(
        &mut out,
        &decoded_scalar(&descriptor.value_type, summary.lo),
    );
    out.push_str(", ");
    literal(
        &mut out,
        &decoded_scalar(&descriptor.value_type, summary.hi),
    );
    out.push_str("] ∧ ");
    out.push_str(&descriptor.name);
    out.push_str(" == ");
    literal(&mut out, &decoded_scalar(&descriptor.value_type, eq_word));
    out
}

fn field_allen_picture(
    relation: &Relation,
    field: FieldId,
    pin: (u64, u64),
    mask: AllenMask,
    other: (u64, u64),
) -> String {
    let descriptor = relation.field(field);
    let mut out = format!("{}: {} == ", relation.name(), descriptor.name);
    literal(&mut out, &decoded_interval(&descriptor.value_type, pin));
    out.push_str(" ∧ Allen(");
    out.push_str(&descriptor.name);
    out.push_str(", ");
    mask_names(&mut out, mask);
    out.push_str(", ");
    literal(&mut out, &decoded_interval(&descriptor.value_type, other));
    out.push(')');
    out
}

fn fields_allen_picture(
    relation: &Relation,
    left: FieldId,
    lhs: (u64, u64),
    mask: AllenMask,
    right: FieldId,
    rhs: (u64, u64),
) -> String {
    let left_descriptor = relation.field(left);
    let right_descriptor = relation.field(right);
    let mut out = format!("{}: {} == ", relation.name(), left_descriptor.name);
    literal(
        &mut out,
        &decoded_interval(&left_descriptor.value_type, lhs),
    );
    out.push_str(" ∧ ");
    out.push_str(&right_descriptor.name);
    out.push_str(" == ");
    literal(
        &mut out,
        &decoded_interval(&right_descriptor.value_type, rhs),
    );
    out.push_str(" ∧ Allen(");
    out.push_str(&left_descriptor.name);
    out.push_str(", ");
    mask_names(&mut out, mask);
    out.push_str(", ");
    out.push_str(&right_descriptor.name);
    out.push(')');
    out
}

fn point_in_picture(relation: &Relation, field: FieldId, pin: (u64, u64), point: u64) -> String {
    let descriptor = relation.field(field);
    let element_type = match &descriptor.value_type {
        ValueType::Interval {
            element: IntervalElement::I64,
        }
        | ValueType::FixedInterval {
            element: IntervalElement::I64,
            ..
        } => ValueType::I64,
        _ => ValueType::U64,
    };
    let mut out = format!("{}: {} == ", relation.name(), descriptor.name);
    literal(&mut out, &decoded_interval(&descriptor.value_type, pin));
    out.push_str(" ∧ ");
    literal(&mut out, &decoded_scalar(&element_type, point));
    out.push_str(" in ");
    out.push_str(&descriptor.name);
    out
}

fn field_within_picture(
    relation: &Relation,
    field: FieldId,
    point: u64,
    outer: (u64, u64),
) -> String {
    let descriptor = relation.field(field);

    let outer_type = ValueType::Interval {
        element: match descriptor.value_type {
            ValueType::I64 => IntervalElement::I64,
            _ => IntervalElement::U64,
        },
    };
    let mut out = format!("{}: {} == ", relation.name(), descriptor.name);
    literal(&mut out, &decoded_scalar(&descriptor.value_type, point));
    out.push_str(" ∧ ");
    out.push_str(&descriptor.name);
    out.push_str(" in ");
    literal(&mut out, &decoded_interval(&outer_type, outer));
    out
}

#[cfg(test)]
mod tests;
