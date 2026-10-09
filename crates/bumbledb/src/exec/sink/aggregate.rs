//! The aggregate sink's construction, group map, folds, and finalize.
mod finalize;
mod fold_batch;
mod fold_row;
mod groups;
mod new;
pub(in crate::exec::sink) mod reduce;
mod sink;

pub(in crate::exec::sink) use new::{parse_finds, parse_finds_into};

/// The order key of F64 NaN, the largest key: MAX propagates it unchanged.
const NAN_KEY: u64 = bumbledb_theory::F64::NAN.to_order_key();

/// F64 MIN propagates NaN by folding it as key 0, which no canonical F64
/// has, so it is below every value.
fn min_key(word: u64) -> u64 {
    if word == NAN_KEY { 0 } else { word }
}

/// Restores NaN in an F64 MIN result.
fn min_output(word: u64) -> u64 {
    if word == 0 { NAN_KEY } else { word }
}
