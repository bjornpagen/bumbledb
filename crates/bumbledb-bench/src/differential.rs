//! The comparison runner: one op stream replayed against the engine and
//! the naive model, asserting per write the same verdict and the same
//! violating statement, and per query set-equal results
//! (`docs/architecture/60-validation.md` § the two oracles). The verify
//! command's naive lane (`verify::run_naive`) feeds [`run`] the corpus
//! op streams.
//!
//! This module lives beside `naive`, never inside it: the runner drives
//! the engine (`Db`), and the naive model's independence forbids
//! anything under `naive/` from importing engine machinery.

use std::collections::BTreeSet;

use bumbledb::{Db, Error, Query, ResultValue, Value};

use crate::naive::query::{ParamValue, QueryError};
use crate::naive::{Delta, NaiveDb, Tuple, Violation};

#[cfg(test)]
mod tests;

/// One operation of a differential stream.
#[derive(Debug, Clone)]
pub enum Op {
    Write(Delta),
    Query {
        query: Query,
        params: Vec<ParamValue>,
    },
}

/// One write's outcome, on either side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Committed,
    Aborted(Violation),
}

/// One query's outcome, on either side: the result set, or the one
/// defined runtime error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rows {
    Ok(BTreeSet<Tuple>),
    Overflow,
}

/// The first disagreement: which op, and what each side said.
#[derive(Debug)]
pub enum Divergence {
    Write {
        op: usize,
        engine: Verdict,
        naive: Verdict,
    },
    Query {
        op: usize,
        engine: Rows,
        naive: Rows,
    },
}

/// What a clean run exercised — callers assert the stream actually
/// covered both verdicts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    pub commits: u64,
    pub aborts: u64,
    pub queries: u64,
}

/// Replays the ops in order against both sides.
///
/// # Errors
///
/// The first [`Divergence`].
///
/// # Panics
///
/// On tool-level failures (storage errors, a query either side refuses) —
/// never on a disagreement.
pub fn run<S>(db: &Db<S>, naive: &mut NaiveDb, ops: &[Op]) -> Result<Summary, Divergence> {
    let mut summary = Summary::default();
    for (index, op) in ops.iter().enumerate() {
        match op {
            Op::Write(delta) => {
                let engine = engine_write(db, delta);
                let model = match naive.apply(delta) {
                    Ok(()) => Verdict::Committed,
                    Err(violation) => Verdict::Aborted(violation),
                };
                if engine != model {
                    return Err(Divergence::Write {
                        op: index,
                        engine,
                        naive: model,
                    });
                }
                match engine {
                    Verdict::Committed => summary.commits += 1,
                    Verdict::Aborted(_) => summary.aborts += 1,
                }
            }
            Op::Query { query, params } => {
                let engine = engine_query(db, query, params);
                let model = match naive.query(query, params) {
                    Ok(rows) => Rows::Ok(rows),
                    Err(QueryError::Overflow { .. }) => Rows::Overflow,
                };
                if engine != model {
                    return Err(Divergence::Query {
                        op: index,
                        engine,
                        naive: model,
                    });
                }
                summary.queries += 1;
            }
        }
    }
    Ok(summary)
}

/// One delta through the engine's write path: deletes then inserts (the
/// same order [`NaiveDb::apply`] uses, so no-op cancellation agrees).
fn engine_write<S>(db: &Db<S>, delta: &Delta) -> Verdict {
    let outcome = db.write(|tx| {
        for (rel, fact) in &delta.deletes {
            tx.delete_dyn(*rel, fact)?;
        }
        for (rel, fact) in &delta.inserts {
            tx.insert_dyn(*rel, fact)?;
        }
        Ok(())
    });
    match outcome {
        Ok(()) => Verdict::Committed,
        Err(Error::FunctionalityViolation { statement, .. }) => {
            Verdict::Aborted(Violation::Functionality { statement })
        }
        Err(Error::ContainmentViolation {
            statement,
            direction,
            ..
        }) => Verdict::Aborted(Violation::Containment {
            statement,
            direction,
        }),
        Err(other) => panic!("engine refused a differential write: {other:?}"),
    }
}

/// One query through the engine as a [`Rows`] verdict — shared with the
/// dual-run chase differential (`tests/chase.rs`), which compares
/// chase-on, chase-off, and model rows three ways.
pub(crate) fn engine_query<S>(db: &Db<S>, query: &Query, params: &[ParamValue]) -> Rows {
    let mut prepared = db.prepare(query).expect("differential queries validate");
    let args = crate::families::param_args(params);
    let outcome = db.read(|snap| snap.execute_collect_args(&mut prepared, &args));
    match outcome {
        Ok(buffer) => Rows::Ok(
            buffer
                .rows()
                .map(|row| {
                    Tuple(
                        (0..buffer.arity())
                            .map(|column| owned_value(row.get(column)))
                            .collect(),
                    )
                })
                .collect(),
        ),
        Err(Error::Overflow { .. }) => Rows::Overflow,
        Err(other) => panic!("engine refused a differential query: {other:?}"),
    }
}

fn owned_value(value: ResultValue<'_>) -> Value {
    match value {
        ResultValue::Bool(v) => Value::Bool(v),
        ResultValue::U64(v) => Value::U64(v),
        ResultValue::I64(v) => Value::I64(v),
        ResultValue::Enum(v) => Value::Enum(v),
        ResultValue::String(v) => Value::String(Box::from(v.as_bytes())),
        ResultValue::Bytes(v) => Value::Bytes(Box::from(v)),
        ResultValue::IntervalU64(iv) => Value::IntervalU64(iv.start(), iv.end()),
        ResultValue::IntervalI64(iv) => Value::IntervalI64(iv.start(), iv.end()),
    }
}
