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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Gate,
    Report,
}

pub type Draw = Vec<ParamValue>;

#[must_use]
pub fn scalar_draw(values: Vec<Value>) -> Draw {
    values.into_iter().map(ParamValue::Scalar).collect()
}

#[must_use]
pub fn bind_value(value: &Value) -> BindValue<'_> {
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

#[must_use]
pub fn bind_values(values: &[Value]) -> Vec<BindValue<'_>> {
    values.iter().map(bind_value).collect()
}

#[must_use]
pub fn param_args(draw: &[ParamValue]) -> Vec<ParamArg<'_>> {
    draw.iter()
        .map(|arg| match arg {
            ParamValue::Scalar(value) => ParamArg::Scalar(bind_value(value)),
            ParamValue::Set(values) => ParamArg::Set(values),
        })
        .collect()
}

/// # Panics
#[must_use]
pub fn scalar_values(draw: &[ParamValue]) -> Vec<BindValue<'_>> {
    draw.iter()
        .map(|arg| match arg {
            ParamValue::Scalar(value) => bind_value(value),
            ParamValue::Set(_) => panic!("a set param has no scalar position"),
        })
        .collect()
}

/// # Panics
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

#[must_use]
pub fn has_sets(draws: &[Draw]) -> bool {
    draws
        .iter()
        .any(|draw| draw.iter().any(|arg| matches!(arg, ParamValue::Set(_))))
}

/// Interval families' composite `(account, active_start, active_end)` comes
/// from the pointwise key's statement-derived index; family entries add the
/// shapes statements do not imply.
pub type FamilyIndex = (&'static str, &'static str, &'static [&'static str]);

pub struct Family {
    pub name: &'static str,
    pub kind: Kind,
    pub query: fn() -> Query,

    pub params: fn(&GenConfig) -> Vec<Draw>,

    pub golden_sql: &'static str,

    pub param_policy: &'static str,

    pub indexes: &'static [FamilyIndex],
}

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

/// its report-only classification, and its write-appropriate protocol.
pub struct WriteFamily {
    pub name: &'static str,
    pub kind: Kind,
    pub protocol: crate::harness::Protocol,
}
