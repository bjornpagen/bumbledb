use std::cmp::Ordering;

use bumbledb::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tuple(pub Vec<Value>);

impl Ord for Tuple {
    fn cmp(&self, other: &Self) -> Ordering {
        let by_value = self
            .0
            .iter()
            .zip(&other.0)
            .map(|(a, b)| cmp_value(a, b))
            .find(|ordering| ordering.is_ne());
        by_value.unwrap_or_else(|| self.0.len().cmp(&other.0.len()))
    }
}

impl PartialOrd for Tuple {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn rank(value: &Value) -> u8 {
    match value {
        Value::Bool(_) => 0,
        Value::U64(_) => 1,
        Value::I64(_) => 2,
        Value::String(_) => 3,
        Value::FixedBytes(_) => 4,
        Value::IntervalU64(..) => 5,
        Value::IntervalI64(..) => 6,
    }
}

pub(crate) fn cmp_value(a: &Value, b: &Value) -> Ordering {
    match (a, b) {
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        (Value::U64(x), Value::U64(y)) => x.cmp(y),
        (Value::I64(x), Value::I64(y)) => x.cmp(y),
        (Value::String(x), Value::String(y)) => x.cmp(y),
        (Value::FixedBytes(x), Value::FixedBytes(y)) => x.cmp(y),
        (Value::IntervalU64(x), Value::IntervalU64(y)) => {
            (x.start(), x.end()).cmp(&(y.start(), y.end()))
        }
        (Value::IntervalI64(x), Value::IntervalI64(y)) => {
            (x.start(), x.end()).cmp(&(y.start(), y.end()))
        }
        _ => rank(a).cmp(&rank(b)),
    }
}

/// # Panics
pub(crate) fn endpoints(value: &Value) -> (i128, i128) {
    match value {
        Value::IntervalU64(interval) => (i128::from(interval.start()), i128::from(interval.end())),
        Value::IntervalI64(interval) => (i128::from(interval.start()), i128::from(interval.end())),
        other => panic!("expected an interval value, got {other:?}"),
    }
}

pub(crate) fn point(value: &Value) -> Option<i128> {
    match value {
        Value::U64(v) => Some(i128::from(*v)),
        Value::I64(v) => Some(i128::from(*v)),
        _ => None,
    }
}

pub(crate) fn overlaps(a: (i128, i128), b: (i128, i128)) -> bool {
    a.0 < b.1 && b.0 < a.1
}

pub(crate) fn point_in(interval: (i128, i128), point: i128) -> bool {
    interval.0 <= point && point < interval.1
}
