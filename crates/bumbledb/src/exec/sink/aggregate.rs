//! The aggregate sink's construction, group map, folds, and finalize.

mod finalize;
mod fold_batch;
mod fold_row;
mod groups;
mod new;
mod sink;

pub(in crate::exec::sink) use new::{parse_finds, parse_finds_into};
