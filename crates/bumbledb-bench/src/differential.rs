use std::collections::BTreeSet;

#[cfg(test)]
use bumbledb::ConditionalWrite;
#[cfg(test)]
use bumbledb::Witness;
use bumbledb::schema::{Schema, SchemaDescriptor, ValidateDescriptor as _};
use bumbledb::{Admission, AnswerValue, Db, Error, InstanceBuilder, Query, RelationId, Value};

#[cfg(test)]
use crate::naive::ConditionalAbort;
use crate::naive::query::{ParamValue, QueryError};
use crate::naive::{Delta, NaiveDb, Tuple, Violation};

#[cfg(test)]
mod tests;

#[expect(
    clippy::large_enum_variant,
    reason = "Query is the public IR shape; boxing would split the differential Op"
)]
#[derive(Debug, Clone)]
pub enum Op {
    Write(Delta),
    Query {
        query: Query,
        params: Vec<ParamValue>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Committed,
    Aborted(Vec<Violation>),
}

/// One conditional write's outcome, on either side: [`Verdict`] plus the
/// witness refusal with its payload — compared whole, so verdict *and*
/// generations must agree (error parity including typed identity, the
/// direction-divergence lesson applied from birth).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionalVerdict {
    Committed,
    Aborted(Vec<Violation>),
    Moved { witnessed: u64, current: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answers {
    Ok(BTreeSet<Tuple>),
    Overflow,

    DerivedBudget,
}

#[derive(Debug)]
pub enum Divergence {
    Write {
        op: usize,
        engine: Verdict,
        naive: Verdict,
    },
    Query {
        op: usize,
        engine: Answers,
        naive: Answers,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    pub commits: u64,
    pub aborts: u64,
    pub queries: u64,
}

/// # Errors
/// # Panics
pub fn run<S>(db: &Db<S>, naive: &mut NaiveDb, ops: &[Op]) -> Result<Summary, Divergence> {
    let mut summary = Summary::default();
    for (index, op) in ops.iter().enumerate() {
        match op {
            Op::Write(delta) => {
                let engine = engine_write(db, delta);
                let model = match naive.apply(delta) {
                    Ok(()) => Verdict::Committed,
                    Err(violations) => Verdict::Aborted(violations),
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
                    Ok(answers) => Answers::Ok(answers),
                    Err(QueryError::Overflow { .. }) => Answers::Overflow,
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

#[must_use]
pub fn cited(violations: &bumbledb::Violations, schema: &Schema) -> Vec<Violation> {
    violations
        .iter()
        .map(|violation| match violation {
            bumbledb::Violation::Functionality { .. } => Violation::Functionality {
                statement: violation.statement_id(schema),
            },
            bumbledb::Violation::Containment { direction, .. } => Violation::Containment {
                statement: violation.statement_id(schema),
                direction: *direction,
            },
            bumbledb::Violation::Capacity { measure, .. } => Violation::Capacity {
                statement: violation.statement_id(schema),
                measure: *measure,
            },
        })
        .collect()
}

pub(crate) fn engine_write<S>(db: &Db<S>, delta: &Delta) -> Verdict {
    let outcome = db.write(|tx| {
        for (rel, fact) in &delta.deletes {
            tx.delete_dyn(*rel, [fact])?;
        }
        for (rel, fact) in &delta.inserts {
            tx.insert_dyn(*rel, [fact])?;
        }
        Ok(())
    });
    match outcome {
        Ok(Admission::Accepted(_)) => Verdict::Committed,
        Ok(Admission::Rejected(violations)) => Verdict::Aborted(cited(&violations, db.schema())),
        Err(Error::ClosedRelationWrite { relation }) => {
            Verdict::Aborted(vec![Violation::ClosedRelationWrite { relation }])
        }

        // refusal, not a violation set; the witness fact bytes are
        Err(Error::CapacityRayMeasure { statement, .. }) => {
            Verdict::Aborted(vec![Violation::CapacityRayMeasure { statement }])
        }
        Err(other) => panic!("engine refused a differential write: {other:?}"),
    }
}

pub(crate) fn engine_admit(
    schema: SchemaDescriptor,
    facts: &[(RelationId, Vec<Value>)],
) -> Verdict {
    let sealed = schema
        .clone()
        .validate()
        .unwrap_or_else(|err| panic!("complete-admission descriptor re-validates: {err}"));
    let mut builder = InstanceBuilder::new(schema)
        .unwrap_or_else(|err| panic!("engine refused a complete-admission schema: {err}"));
    for (rel, fact) in facts {
        if let Err(err) = builder.load_dyn(*rel, [fact.as_slice()]) {
            return admit_load_error(err);
        }
    }
    match builder.admit() {
        Ok(Admission::Accepted(_)) => Verdict::Committed,
        Ok(Admission::Rejected(violations)) => Verdict::Aborted(cited(&violations, &sealed)),
        Err(Error::ClosedRelationWrite { relation }) => {
            Verdict::Aborted(vec![Violation::ClosedRelationWrite { relation }])
        }
        Err(Error::CapacityRayMeasure { statement, .. }) => {
            Verdict::Aborted(vec![Violation::CapacityRayMeasure { statement }])
        }
        Err(other) => panic!("engine refused complete admission: {other:?}"),
    }
}

fn admit_load_error(err: Error) -> Verdict {
    match err {
        Error::ClosedRelationWrite { relation } => {
            Verdict::Aborted(vec![Violation::ClosedRelationWrite { relation }])
        }
        Error::CapacityRayMeasure { statement, .. } => {
            Verdict::Aborted(vec![Violation::CapacityRayMeasure { statement }])
        }
        other => panic!("engine refused a complete-admission load: {other:?}"),
    }
}

#[cfg(test)]
pub(crate) fn engine_write_from<S>(
    db: &Db<S>,
    witness: &Witness<S>,
    delta: &Delta,
) -> ConditionalVerdict {
    let outcome = db.write_from(witness, |tx| {
        for (rel, fact) in &delta.deletes {
            tx.delete_dyn(*rel, [fact])?;
        }
        for (rel, fact) in &delta.inserts {
            tx.insert_dyn(*rel, [fact])?;
        }
        Ok(())
    });
    match outcome {
        Ok(ConditionalWrite::Accepted(_)) => ConditionalVerdict::Committed,
        Ok(ConditionalWrite::Rejected(violations)) => {
            ConditionalVerdict::Aborted(cited(&violations, db.schema()))
        }
        Ok(ConditionalWrite::Moved { witnessed, current }) => ConditionalVerdict::Moved {
            witnessed: witnessed.value(),
            current: current.value(),
        },
        Err(Error::ClosedRelationWrite { relation }) => {
            ConditionalVerdict::Aborted(vec![Violation::ClosedRelationWrite { relation }])
        }
        Err(Error::CapacityRayMeasure { statement, .. }) => {
            ConditionalVerdict::Aborted(vec![Violation::CapacityRayMeasure { statement }])
        }
        Err(other) => panic!("engine refused a differential conditional write: {other:?}"),
    }
}

#[cfg(test)]
pub(crate) fn naive_write_from(
    naive: &mut NaiveDb,
    witnessed: u64,
    delta: &Delta,
) -> ConditionalVerdict {
    match naive.apply_from(witnessed, delta) {
        Ok(()) => ConditionalVerdict::Committed,
        Err(ConditionalAbort::Moved { witnessed, current }) => {
            ConditionalVerdict::Moved { witnessed, current }
        }
        Err(ConditionalAbort::Violations(violations)) => ConditionalVerdict::Aborted(violations),
    }
}

pub(crate) fn engine_query<S>(db: &Db<S>, query: &Query, params: &[ParamValue]) -> Answers {
    let mut prepared = db.prepare(query).expect("differential queries validate");
    let args = crate::families::param_args(params);
    let outcome = db.read(|snap| snap.execute_collect(&mut prepared, &args));
    match outcome {
        Ok(buffer) => Answers::Ok(
            buffer
                .answers()
                .map(|answer| {
                    Tuple(
                        (0..buffer.arity())
                            .map(|column| owned_value(answer.get(column)))
                            .collect(),
                    )
                })
                .collect(),
        ),
        Err(Error::Overflow { .. }) => Answers::Overflow,
        Err(Error::DerivedBudgetExceeded { .. }) => Answers::DerivedBudget,
        Err(other) => panic!("engine refused a differential query: {other:?}"),
    }
}

fn owned_value(value: AnswerValue<'_>) -> Value {
    match value {
        AnswerValue::Bool(v) => Value::Bool(v),
        AnswerValue::U64(v) => Value::U64(v),
        AnswerValue::I64(v) => Value::I64(v),
        AnswerValue::String(v) => Value::String(v.into()),
        AnswerValue::FixedBytes(v) => Value::FixedBytes(Box::from(v)),
        AnswerValue::IntervalU64(iv) => Value::IntervalU64(iv),
        AnswerValue::IntervalI64(iv) => Value::IntervalI64(iv),
    }
}
