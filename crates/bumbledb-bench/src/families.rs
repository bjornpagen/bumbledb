//! The gated read families (docs/architecture/60-validation.md § the
//! primary benchmark, normative): exact IR, exact param policy,
//! hand-written SQL golden, per-family `SQLite` index DDL, gate
//! classification. This file of queries **is** the benchmark's identity —
//! `digest()` keys the verify stamp and every report on it.

use bumbledb::{BindValue, ParamArg, ParamId, Query, Value};

use crate::corpus_gen::GenConfig;
use crate::naive::ParamValue;

mod digest;
mod read;
mod render_queries_md;
#[cfg(test)]
mod tests;
mod write;

pub use digest::digest;
pub use read::all;
pub use render_queries_md::render_queries_md;
pub use write::write_families;

/// Whether a family gates the suite (loses ⇒ the run fails) or merely
/// reports. Every read family gates (`60-validation.md`: every family
/// must win).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Gate,
    Report,
}

/// One rotation draw: one [`ParamValue`] per dense `ParamId` position —
/// scalars as values, param sets as element lists (the `ParamSet` IR
/// term; the host-side union convention is retired with it).
pub type Draw = Vec<ParamValue>;

/// A scalar-only draw (most families).
#[must_use]
pub fn scalar_draw(values: Vec<Value>) -> Draw {
    values.into_iter().map(ParamValue::Scalar).collect()
}

/// One owned scalar as the engine's borrowed [`BindValue`] — str/bytes
/// payloads by reference (the bind surface borrows; the draws own).
///
/// # Panics
///
/// On non-UTF-8 `Value::String` bytes — the corpus interns text.
#[must_use]
pub fn bind_value(value: &Value) -> BindValue<'_> {
    match value {
        Value::Bool(v) => BindValue::Bool(*v),
        Value::U64(v) => BindValue::U64(*v),
        Value::I64(v) => BindValue::I64(*v),
        Value::String(raw) => {
            BindValue::Str(std::str::from_utf8(raw).expect("corpus strings are UTF-8"))
        }
        Value::FixedBytes(raw) => BindValue::FixedBytes(raw),
        Value::IntervalU64(interval) => BindValue::IntervalU64(interval.start(), interval.end()),
        Value::IntervalI64(interval) => BindValue::IntervalI64(interval.start(), interval.end()),
        Value::AllenMask(mask) => BindValue::AllenMask(*mask),
    }
}

/// A borrowed view of owned scalar values as the engine's bind slice.
#[must_use]
pub fn bind_values(values: &[Value]) -> Vec<BindValue<'_>> {
    values.iter().map(bind_value).collect()
}

/// One draw as the engine's borrowed [`ParamArg`] positions.
#[must_use]
pub fn param_args(draw: &[ParamValue]) -> Vec<ParamArg<'_>> {
    draw.iter()
        .map(|arg| match arg {
            ParamValue::Scalar(value) => ParamArg::Scalar(bind_value(value)),
            ParamValue::Set(values) => ParamArg::Set(values),
        })
        .collect()
}

/// One draw's scalar positions, positionally — the profile/introspect path
/// (scalar params only).
///
/// # Panics
///
/// On a set position — callers route set-bound draws through
/// [`param_args`].
#[must_use]
pub fn scalar_values(draw: &[ParamValue]) -> Vec<BindValue<'_>> {
    draw.iter()
        .map(|arg| match arg {
            ParamValue::Scalar(value) => bind_value(value),
            ParamValue::Set(_) => panic!("a set param has no scalar position"),
        })
        .collect()
}

/// One draw's set bindings — the translator's re-render input
/// (set params render as literal `IN` lists per execution).
///
/// # Panics
///
/// Never in practice: dense `ParamId`s fit `u16` by IR construction.
#[must_use]
pub fn set_bindings(draw: &[ParamValue]) -> Vec<(ParamId, Vec<Value>)> {
    draw.iter()
        .enumerate()
        .filter_map(|(index, arg)| match arg {
            ParamValue::Set(values) => Some((
                ParamId(u16::try_from(index).expect("dense params fit")),
                values.clone(),
            )),
            ParamValue::Scalar(_) => None,
        })
        .collect()
}

/// Whether any position of any draw binds a set (the family re-renders
/// its SQL per draw; prepared-statement parity is not claimed —
/// `60-validation.md` says so).
#[must_use]
pub fn has_sets(draws: &[Draw]) -> bool {
    draws
        .iter()
        .any(|draw| draw.iter().any(|arg| matches!(arg, ParamValue::Set(_))))
}

/// One family-owned `SQLite` index: `(name, table, columns)` — the
/// honest opponent gets every index its query rewards, beyond the
/// statement-derived set (`crate::sqlmap`). Interval families' composite
/// `(account, active_start, active_end)` comes from the pointwise key's
/// statement-derived index; family entries add the shapes statements do
/// not imply.
pub type FamilyIndex = (&'static str, &'static str, &'static [&'static str]);

/// One read family: the benchmark's unit of identity.
pub struct Family {
    pub name: &'static str,
    pub kind: Kind,
    pub query: fn() -> Query,
    /// The seeded param draws — verify and bench call this with the same
    /// `GenConfig` and therefore see identical rotations.
    pub params: fn(&GenConfig) -> Vec<Draw>,
    /// Hand-written (docs/architecture/60-validation.md) — never
    /// regenerated from the translator; pinned equal to `translate`
    /// output by test (set-bound families pin under the documented
    /// representative set).
    pub golden_sql: &'static str,
    /// The documented param policy, rendered into the versioned query
    /// list.
    pub param_policy: &'static str,
    /// Per-family index DDL beyond the statement-derived indexes.
    pub indexes: &'static [FamilyIndex],
}

/// Every family-owned index, deduplicated by name (families may share a
/// shape), as `CREATE INDEX` statements.
#[must_use]
pub fn index_ddl() -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for family in all() {
        for (name, table, columns) in family.indexes {
            if !seen.insert(*name) {
                continue;
            }
            let cols = columns
                .iter()
                .map(|c| format!("\"{c}\""))
                .collect::<Vec<_>>()
                .join(", ");
            out.push(format!("CREATE INDEX \"{name}\" ON \"{table}\" ({cols})"));
        }
    }
    out
}

/// The family-owned indexes as `(table, name)` pairs — the fairness
/// contract's registry beside `sqlmap::expected_indexes`.
#[must_use]
pub fn expected_indexes() -> Vec<(String, String)> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for family in all() {
        for (name, table, _) in family.indexes {
            if seen.insert(*name) {
                out.push(((*table).to_owned(), (*name).to_owned()));
            }
        }
    }
    out
}

/// One write/cold family (docs/architecture/60-validation.md): a name,
/// its report-only classification, and its write-appropriate protocol.
/// The runners live in `writebench` — these are identities, not
/// closures.
pub struct WriteFamily {
    pub name: &'static str,
    pub kind: Kind,
    pub protocol: crate::harness::Protocol,
}
