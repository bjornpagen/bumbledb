//! The aggregate sink's construction, group map, folds, and finalize.
mod finalize;
mod fold_batch;
mod fold_row;
mod groups;
mod new;
pub(in crate::exec::sink) mod reduce;
mod sink;

/// The group state never leaves RAM: `AggregateSink::spill` is always `None`.
pub(in crate::exec::sink) mod spill {
    #[derive(Debug)]
    pub(crate) enum GroupSpill {}
}

pub(in crate::exec::sink) use new::{parse_finds, parse_finds_into};
