//! Host-side answer ordering — the census-fired convenience
//! (docs/architecture/70-api.md § the freeze ledger: four hand-rolled
//! bigint comparators, every rank/pos consumer sorting host-side).
//! Answers are SETS and the ENGINE NEVER ORDERS; the language owns the
//! sort ([`Vec::sort_by`]). What the host lacks is an order over the
//! borrowed cells — [`AnswerValue`] carries no [`Ord`], because a
//! column's order lives in its domain — so the quarantine ships sort
//! keys as DATA ([`SortKey`]) folded by [`by`] into one comparator.
//! Limit is the language's own `truncate`/`take`: no operator is
//! invented where one already exists (the drizzle law).
//!
//! ```ignore
//! let mut rows: Vec<bumbledb::Answer<'_>> = out.answers().collect();
//! rows.sort_by(bumbledb_query::order::by(&[SortKey::Asc(1), SortKey::Desc(0)]));
//! rows.truncate(10); // limit is the language's own
//! ```

use std::cmp::Ordering;

use bumbledb::{Answer, AnswerValue};

/// One sort key as data: the answer column (a find-term index) with its
/// direction as the variant — direction is a case of the sum, never a
/// flag riding a struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    /// Ascending over the column at this find-term index.
    Asc(usize),
    /// Descending over the column at this find-term index.
    Desc(usize),
}

/// The variant rank that keeps [`value_cmp`] total: cross-variant pairs
/// cannot arise within one column, so their order carries no meaning —
/// it exists only so host misuse never panics.
const fn rank(value: &AnswerValue<'_>) -> u8 {
    match value {
        AnswerValue::Bool(_) => 0,
        AnswerValue::U64(_) => 1,
        AnswerValue::I64(_) => 2,
        AnswerValue::String(_) => 3,
        AnswerValue::FixedBytes(_) => 4,
        AnswerValue::IntervalU64(_) => 5,
        AnswerValue::IntervalI64(_) => 6,
    }
}

/// The canonical TOTAL order over two cells of ONE column: `Bool`
/// false < true, `U64`/`I64` numeric (the engine's order-preserving word
/// encoding already decoded to native ints at materialization), `String`
/// byte order, `FixedBytes` lexicographic, intervals by `(start, end)`.
/// Cross-variant pairs — impossible within one column — order by a
/// private variant rank (`Bool` < `U64` < `I64` < `String` <
/// `FixedBytes` < `IntervalU64` < `IntervalI64`), so incomparability is
/// unrepresentable: no panic branch, no `Option`, no guard downstream.
#[must_use]
pub fn value_cmp(left: &AnswerValue<'_>, right: &AnswerValue<'_>) -> Ordering {
    match (left, right) {
        (AnswerValue::Bool(a), AnswerValue::Bool(b)) => a.cmp(b),
        (AnswerValue::U64(a), AnswerValue::U64(b)) => a.cmp(b),
        (AnswerValue::I64(a), AnswerValue::I64(b)) => a.cmp(b),
        (AnswerValue::String(a), AnswerValue::String(b)) => a.cmp(b),
        (AnswerValue::FixedBytes(a), AnswerValue::FixedBytes(b)) => a.cmp(b),
        (AnswerValue::IntervalU64(a), AnswerValue::IntervalU64(b)) => a.bounds().cmp(&b.bounds()),
        (AnswerValue::IntervalI64(a), AnswerValue::IntervalI64(b)) => a.bounds().cmp(&b.bounds()),
        _ => rank(left).cmp(&rank(right)),
    }
}

/// Folds the keys, left to right, into ONE comparator for the language's
/// own [`Vec::sort_by`]: per key, [`value_cmp`] on the key's column,
/// reversed under [`SortKey::Desc`]; the first non-[`Ordering::Equal`]
/// key wins, and all-equal answers compare [`Ordering::Equal`].
///
/// # Panics
///
/// The returned comparator inherits [`Answer::get`]'s one panic: a key
/// naming an out-of-range column (a find-term index at or beyond the
/// answer arity).
#[must_use = "the comparator sorts nothing until handed to `sort_by`"]
pub fn by(keys: &[SortKey]) -> impl Fn(&Answer<'_>, &Answer<'_>) -> Ordering + '_ {
    move |left, right| {
        keys.iter().fold(Ordering::Equal, |decided, key| {
            decided.then_with(|| match *key {
                SortKey::Asc(column) => value_cmp(&left.get(column), &right.get(column)),
                SortKey::Desc(column) => value_cmp(&left.get(column), &right.get(column)).reverse(),
            })
        })
    }
}
