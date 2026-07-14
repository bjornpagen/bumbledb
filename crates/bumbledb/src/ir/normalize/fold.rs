//! The statically-empty fold (docs/architecture/20-query-ir.md,
//! § normalization): the database analog of comptime-unreachable, run at
//! the end of each rule's lowering, over each **participating**
//! occurrence's own filter list.
//!
//! Two jobs, one pass:
//!
//! 1. **Range folding** — a conjunction of constant order filters on one
//!    u64/i64 slot collapses into a single `[lo, hi]` summary over
//!    **encoded words** (the sign-flip I64 encoding makes one unsigned
//!    comparison domain serve both integer types — the configuration
//!    kernel's precedent, `exec/kernel`). The summary REPLACES its
//!    constituents, lowered back to at most two order filters + one Eq
//!    per slot — existing [`FilterPredicate`] shapes, no new filter kind,
//!    no new kernel. Fewer residuals means fewer keep-fraction
//!    multiplications (`plan/selectivity.rs` counts a folded summary
//!    once) and fewer kernel passes.
//! 2. **Contradiction detection** — each rule judged on **constants
//!    only**, each producing a statically-empty verdict for the RULE
//!    ([`super::NormalizedQuery::dead`]), with the killing condition
//!    rendered for introspection: an empty range summary; `Eq` to two distinct
//!    constants on one slot; an `Eq` constant outside the range summary;
//!    a membership set empty after sentinel-trim, or intersected with an
//!    `Eq` constant not in it; an `Allen` literal-vs-literal condition
//!    `classify` refutes (both operands constant intervals); a constant
//!    point in a constant interval that fails.
//!
//! `Ne` and param-bearing conditions never fold — `Ne` prunes nothing
//! statically, and params are stage-3 (bind-time) values a stage-2 pass
//! must not judge. Interval variables fold via their two slot summaries
//! independently — no cross-slot reasoning in v0 (the constructor
//! invariant `start < end` is data, not plan knowledge). Negated
//! occurrences neither fold nor yield verdicts: a contradictory filter
//! list on a negated atom matches nothing, so its anti-probe never
//! rejects — the rule is NOT empty.

use std::collections::BTreeMap;

use super::Occurrence;
use crate::allen::AllenMask;
use crate::encoding::decode_i64;
use crate::image::view::{Const, FilterPredicate, MaskConst, ResolvedWordSource};
use crate::ir::render::{literal, mask_names};
use crate::ir::{CmpOp, Value};
use crate::schema::{FieldId, IntervalElement, Relation, Schema, ValueType};

#[cfg(any(test, feature = "fold-off"))]
thread_local! {
    /// The test-only off switch (the ground-off-switch precedent,
    /// `plan/ground.rs`): the fold-preservation differential runs the
    /// same query folded and unfolded. Reachable from this crate's own
    /// tests and — through the `fold-off` fuzz-oracle feature, enabled
    /// only by the detached fuzz crate's `rewrites` dual-pipeline
    /// differential — from nowhere a production build can see: no
    /// runtime mode ships.
    /// Thread-local because the test harness runs tests concurrently.
    static DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `f` with the fold bypassed on this thread — the fold-preservation
/// differential's off switch. Restores on unwind.
#[cfg(any(test, feature = "fold-off"))]
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

/// Folds every participating occurrence's filters in place and returns
/// the first contradiction's rendered picture — the rule's
/// statically-empty verdict ([`super::NormalizedQuery::dead`]).
pub(super) fn fold(schema: &Schema, occurrences: &mut [Occurrence]) -> Option<String> {
    #[cfg(any(test, feature = "fold-off"))]
    if DISABLED.with(std::cell::Cell::get) {
        return None;
    }
    for occurrence in occurrences.iter_mut() {
        // Verdicts and folding are for participating occurrences only:
        // a negated occurrence's contradictory filters match nothing, so
        // its anti-probe never rejects — the rule is NOT empty (module
        // doc); leaving its filters untouched keeps that semantics.
        if !occurrence.role.participates() {
            continue;
        }
        if let Some(reason) = fold_occurrence(schema, occurrence) {
            return Some(reason);
        }
    }
    None
}

/// One (occurrence, slot)'s range summary over **encoded words** —
/// inclusive `[lo, hi]`, empty iff `lo > hi`. Both integer encodings are
/// order-preserving maps onto u64 words (u64 the identity, I64 the
/// sign-flip bias — `docs/architecture/50-storage.md`), so one unsigned
/// domain serves both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RangeSummary {
    lo: u64,
    hi: u64,
}

impl RangeSummary {
    /// The full domain: every word satisfies it.
    fn new() -> Self {
        Self {
            lo: 0,
            hi: u64::MAX,
        }
    }

    /// Narrows by one order filter `slot <op> word`. Strict bounds at
    /// the domain edge (`> MAX`, `< 0`) have no satisfying word and
    /// empty the summary outright.
    fn narrow(&mut self, op: CmpOp, word: u64) {
        match op {
            CmpOp::Ge => self.lo = self.lo.max(word),
            CmpOp::Le => self.hi = self.hi.min(word),
            CmpOp::Gt => match word.checked_add(1) {
                Some(above) => self.lo = self.lo.max(above),
                None => self.mark_empty(),
            },
            CmpOp::Lt => match word.checked_sub(1) {
                Some(below) => self.hi = self.hi.min(below),
                None => self.mark_empty(),
            },
            CmpOp::Eq | CmpOp::Ne | CmpOp::Allen { .. } | CmpOp::PointIn => {
                unreachable!("only order filters narrow the summary")
            }
        }
    }

    /// The canonical empty summary (`lo > hi`); further narrowing keeps
    /// it empty (`lo` only grows, `hi` only shrinks).
    fn mark_empty(&mut self) {
        self.lo = 1;
        self.hi = 0;
    }
}

// The contradiction rules, one function each (the grounding conditions'
// naming discipline, `plan/ground.rs`) — every one judged on constants
// only, each a statically-empty verdict for the rule.

/// Rule (a): the folded order filters admit no word.
fn range_is_empty(summary: &RangeSummary) -> bool {
    summary.lo > summary.hi
}

/// Rule (b): `Eq` to two distinct constants on one slot. Encodings are
/// canonical (one word form per value — `encoding`), so distinct
/// same-shape constants are distinct values; two pending `str` literals
/// compare by their bytes for the same reason. Shapes never mix on one
/// field's lowering, and the mixed-shape arm stays conservative.
fn eq_conflicts(first: &Const, second: &Const) -> bool {
    match (first, second) {
        // Rule (d)'s pair form: the set intersected with a scalar `Eq`.
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

/// Rule (c): the `Eq` constant lies outside the slot's range summary.
fn eq_outside_range(word: u64, summary: &RangeSummary) -> bool {
    word < summary.lo || word > summary.hi
}

/// Rule (d): the membership set, sentinel-trimmed (a never-minted
/// intern id matches nothing — `storage/dict`), is empty, or an `Eq`
/// constant on the same slot is not among its words.
fn set_refutes_eq(words: &[u64], eq: Option<u64>) -> bool {
    let mut live = words
        .iter()
        .filter(|word| **word != crate::storage::dict::SENTINEL_ID);
    match eq {
        Some(eq) => !live.any(|word| *word == eq),
        None => live.next().is_none(),
    }
}

/// Rule (e): a literal-vs-literal `Allen` condition `classify` refutes —
/// both operands constant intervals (encoded endpoint words preserve
/// value order, so classification over words equals classification over
/// values — `crate::allen::classify_bounds`). Degenerate encoded pairs
/// (`start >= end` — unconstructible from validated literals) refute
/// nothing: the fold is conservative, never wrong.
fn allen_refuted(lhs: (u64, u64), mask: AllenMask, rhs: (u64, u64)) -> bool {
    if lhs.0 >= lhs.1 || rhs.0 >= rhs.1 {
        return false;
    }
    !mask.contains(crate::allen::classify_bounds(
        &lhs.0, &lhs.1, &rhs.0, &rhs.1,
    ))
}

/// Rule (f): a constant point outside a constant interval — the
/// membership composition `start <= p AND p < end` on encoded words.
fn point_outside(interval: (u64, u64), point: u64) -> bool {
    point < interval.0 || point >= interval.1
}

/// One occurrence's fold: Eq pins, range summaries, the contradiction
/// rules, then emission — the folded summaries replace their constituent
/// order filters in place.
fn fold_occurrence(schema: &Schema, occurrence: &mut Occurrence) -> Option<String> {
    let relation = schema.relation(occurrence.relation);

    // Pass 1 — the Eq pins: the first constant Eq per field (params are
    // stage-3 and never fold), judging rules (b) and (d) as later
    // constants arrive.
    let mut eqs: BTreeMap<FieldId, Const> = BTreeMap::new();
    for filter in &occurrence.filters {
        let FilterPredicate::Compare {
            field,
            op: CmpOp::Eq,
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
                    relation.field(*field).name
                ));
            }
        }
        match eqs.get(field) {
            None => {
                eqs.insert(*field, value.clone());
            }
            Some(prior) => {
                // Rules (b) and (d): the pinned constant against the
                // later one.
                if eq_conflicts(prior, value) {
                    return Some(eq_pair_picture(relation, *field, prior, value));
                }
            }
        }
    }

    // Pass 2 — per-slot range summaries over the constant order filters
    // (interval variables have no order filters — their two slots fold
    // through the interval pins below, independently; no cross-slot
    // reasoning in v0).
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
        // Rule (a): the summary admits no word.
        if range_is_empty(summary) {
            return Some(order_filters_picture(relation, *field, &occurrence.filters));
        }
        // Rule (c): the pinned Eq constant lies outside it.
        if let Some(Const::Word(eq_word)) = eqs.get(field)
            && eq_outside_range(*eq_word, summary)
        {
            return Some(eq_outside_picture(relation, *field, summary, *eq_word));
        }
    }

    // Pass 3 — the constant-interval rules (e) and (f).
    if let Some(reason) = interval_contradictions(relation, &eqs, &occurrence.filters) {
        return Some(reason);
    }

    emit(occurrence, &eqs, &ranges);
    None
}

/// The constant-interval contradiction pass — rules (e) and (f). An
/// interval slot pair is pinned by a value-equality binding (`Compare`
/// Eq against an interval constant) or by an `Allen(EQUALS)` literal
/// condition — the canonical form interval `Eq` lowers to
/// (`place_comparisons`); a scalar pin comes from the Eq table.
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
            other: Const::Interval { start, end },
            mask: MaskConst::Mask(mask),
        } = filter
            && *mask == AllenMask::EQUALS
        {
            interval_pins.entry(*field).or_insert((*start, *end));
        }
    }
    for filter in filters {
        match filter {
            // Rule (e), field-vs-constant: the pinned interval against
            // the literal operand under the literal mask. The pinning
            // EQUALS filter itself passes its own check (classify of the
            // pin against itself is EQUALS), so no self-exclusion is
            // needed.
            FilterPredicate::FieldAllen {
                field,
                other: Const::Interval { start, end },
                mask: MaskConst::Mask(mask),
            } => {
                if let Some(pin) = interval_pins.get(field)
                    && allen_refuted(*pin, *mask, (*start, *end))
                {
                    return Some(field_allen_picture(
                        relation,
                        *field,
                        *pin,
                        *mask,
                        (*start, *end),
                    ));
                }
            }
            // Rule (e), field-vs-field: both sides pinned.
            FilterPredicate::FieldsAllen {
                left,
                right,
                mask: MaskConst::Mask(mask),
            } => {
                if let (Some(lhs), Some(rhs)) = (interval_pins.get(left), interval_pins.get(right))
                    && allen_refuted(*lhs, *mask, *rhs)
                {
                    return Some(fields_allen_picture(
                        relation, *left, *lhs, *mask, *right, *rhs,
                    ));
                }
            }
            // Rule (f): a constant point against the pinned interval.
            FilterPredicate::PointIn {
                field,
                point: ResolvedWordSource::Word(point),
            } => {
                if let Some(pin) = interval_pins.get(field)
                    && point_outside(*pin, *point)
                {
                    return Some(point_in_picture(relation, *field, *pin, *point));
                }
            }
            // Rule (f), reversed: the pinned scalar against the constant
            // interval.
            FilterPredicate::FieldWithin {
                field,
                outer: Const::Interval { start, end },
            } => {
                if let Some(Const::Word(point)) = eqs.get(field)
                    && point_outside((*start, *end), *point)
                {
                    return Some(field_within_picture(
                        relation,
                        *field,
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

/// The constant order bound one filter contributes, if any: `Ne` prunes
/// nothing statically and params are stage-3, so exactly the
/// `Lt`/`Le`/`Gt`/`Ge`-against-`Const::Word` shape folds (order
/// operators are validated U64/I64-only, so the word IS the slot's
/// encoded comparison domain).
fn constant_order_bound(filter: &FilterPredicate) -> Option<(FieldId, CmpOp, u64)> {
    let FilterPredicate::Compare {
        field,
        op: op @ (CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge),
        value: Const::Word(word),
    } = filter
    else {
        return None;
    };
    Some((*field, *op, *word))
}

/// Emission: each folded summary replaces its constituent order filters
/// in place — at most two order filters per slot, or none at all where
/// a consistent Eq pin subsumes the range (the Eq filter itself stays; a
/// point implies every bound it survived). Vacuous bounds (the domain
/// edges) are simply dropped. Single constituents with no Eq pin stay
/// verbatim — nothing to merge, no churn.
fn emit(
    occurrence: &mut Occurrence,
    eqs: &BTreeMap<FieldId, Const>,
    ranges: &BTreeMap<FieldId, (RangeSummary, usize)>,
) {
    let mut replacements: BTreeMap<FieldId, Vec<FilterPredicate>> = BTreeMap::new();
    for (field, (summary, constituents)) in ranges {
        let pinned = matches!(eqs.get(field), Some(Const::Word(_)));
        if pinned {
            // Rule (c) held above, so the pin lies inside the summary
            // and the bounds are implied: drop every constituent.
            replacements.insert(*field, Vec::new());
        } else if *constituents >= 2 {
            let mut emitted = Vec::with_capacity(2);
            if summary.lo > 0 {
                emitted.push(FilterPredicate::Compare {
                    field: *field,
                    op: CmpOp::Ge,
                    value: Const::Word(summary.lo),
                });
            }
            if summary.hi < u64::MAX {
                emitted.push(FilterPredicate::Compare {
                    field: *field,
                    op: CmpOp::Le,
                    value: Const::Word(summary.hi),
                });
            }
            replacements.insert(*field, emitted);
        }
    }
    if replacements.is_empty() {
        return;
    }
    // The emitted bounds land at the first constituent's position, the
    // rest vanish — the filter-order law (`image/view.rs`) is preserved
    // because a bound compares exactly where its constituents compared.
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

// The verdict pictures — introspection's `statically empty:` payloads, in the
// rule notation's value formats (`ir::render`): decoded values, `..`
// intervals, named masks. Rendered here, at the one point where the
// schema, the killing constants, and the field types coexist.

/// One encoded word decoded through its field's value type (the biased
/// I64 word un-flips; everything else renders raw).
pub(crate) fn decoded_scalar(value_type: &ValueType, word: u64) -> Value {
    match value_type {
        ValueType::I64 => Value::I64(decode_i64(word.to_be_bytes())),
        ValueType::Bool => Value::Bool(word != 0),
        _ => Value::U64(word),
    }
}

/// An encoded interval pair decoded through its element type.
pub(crate) fn decoded_interval(value_type: &ValueType, pair: (u64, u64)) -> Value {
    match value_type {
        ValueType::Interval {
            element: IntervalElement::I64,
        } => Value::IntervalI64(
            crate::Interval::<i64>::new(
                decode_i64(pair.0.to_be_bytes()),
                decode_i64(pair.1.to_be_bytes()),
            )
            .expect("validated interval constant"),
        ),
        _ => Value::IntervalU64(
            crate::Interval::<u64>::new(pair.0, pair.1).expect("validated interval constant"),
        ),
    }
}

/// One Eq constant's picture, by shape (shapes never mix on one field).
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
        Const::PendingIntern { bytes } => literal(out, &Value::String(bytes.clone())),
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

/// Rules (b)/(d): the two pinned constants, side by side — a set renders
/// as membership, a scalar as equality.
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

/// Rule (a): the constituent order filters verbatim — the honest
/// picture of what refuted itself.
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
            CmpOp::Lt => " < ",
            CmpOp::Le => " <= ",
            CmpOp::Gt => " > ",
            CmpOp::Ge => " >= ",
            _ => unreachable!("constant_order_bound admits order operators only"),
        });
        literal(&mut out, &decoded_scalar(&descriptor.value_type, word));
    }
    out
}

/// Rule (c): the folded summary against the Eq pin — the PRD's
/// `x ∈ [8, 19] ∧ x == 3` picture.
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

/// Rule (e), field-vs-constant.
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

/// Rule (e), field-vs-field with both sides pinned.
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

/// Rule (f): the constant point against the pinned interval.
fn point_in_picture(relation: &Relation, field: FieldId, pin: (u64, u64), point: u64) -> String {
    let descriptor = relation.field(field);
    let element_type = match &descriptor.value_type {
        ValueType::Interval {
            element: IntervalElement::I64,
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

/// Rule (f), reversed: the pinned scalar against the constant interval.
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
