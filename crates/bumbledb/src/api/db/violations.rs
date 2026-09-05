//! Bridge from the production judge's complete verdict
//! (`schema::judge::JudgedViolation`, C03) to the public rejection type
//! ([`crate::error::Violations`]).
//!
//! The conversion is faithful in the direction that matters: every violated
//! statement appears exactly once with its stable materialized-order
//! [`StatementId`] identity, capacity verdicts carry the exact widened
//! measure, and every bounded judge example survives as a decoded
//! [`CitedFact`] — including both competing rows of a key conflict, so the
//! historical shared-key counterexamples keep their full evidence. The
//! convicting fact bytes are the first cited example re-encoded through the
//! canonical codec (the same bytes a store row carries). The judge's
//! per-statement example-truncation label crosses too (parallel flags on
//! [`Violations`]), so decide-time evidence encoding never under-reports a
//! judge-level example drop.

use crate::canonical::CanonicalRow;
use crate::error::{CitedFact, Conflict, Direction, Error, Result, Violation, Violations};
use crate::schema::judge::{JudgedDirection, JudgedViolation};
use crate::schema::{Schema, StatementView};
use crate::work::WorkContext;

/// One mapping from the reference judge's refusal (over an infallible
/// in-memory state) to the public error surface — never a fabricated
/// domain rejection.
#[expect(
    clippy::needless_pass_by_value,
    reason = "an error adapter consumes the refusal it maps (`map_err` shape)"
)]
pub(super) fn judge_refusal(
    error: crate::schema::judge::JudgeError<std::convert::Infallible>,
) -> Error {
    match error {
        crate::schema::judge::JudgeError::Work(work) => {
            Error::from_store(crate::storage::store::StoreError::Work(work))
        }
        crate::schema::judge::JudgeError::State(impossible) => match impossible {},
        crate::schema::judge::JudgeError::UndefinedDuration { statement } => {
            Error::from_store(crate::storage::store::StoreError::JudgeRefused {
                statement,
                detail: "undefined ray duration in a measured position",
            })
        }
        crate::schema::judge::JudgeError::MeasureOverflow { statement } => {
            Error::from_store(crate::storage::store::StoreError::JudgeRefused {
                statement,
                detail: "grouped measure exceeded the widened accumulator",
            })
        }
    }
}

pub(super) fn violations_from_judged(
    schema: &Schema,
    judged: Box<[JudgedViolation]>,
    work: &WorkContext,
) -> Result<Violations> {
    debug_assert!(!judged.is_empty(), "a judge rejection is nonempty");
    let mut citations = Vec::with_capacity(judged.len());
    let mut truncated = Vec::with_capacity(judged.len());
    for violation in judged {
        let statement_ref = match schema.statement(violation.statement) {
            StatementView::Key(id, _) => crate::schema::StatementRef::Key(id),
            StatementView::Containment(id, _) => crate::schema::StatementRef::Containment(id),
            StatementView::Capacity(id, _) => crate::schema::StatementRef::Capacity(id),
        };
        let fact = match violation.examples.first() {
            Some(example) => {
                let fields = schema.relation(example.relation).fields();
                let row = CanonicalRow::encode(fields, &example.values, work)
                    .map_err(super::tx::row_error)?;
                Box::from(row.as_bytes())
            }
            None => Box::<[u8]>::from([]),
        };
        let typed = match statement_ref {
            crate::schema::StatementRef::Key(_) => {
                // The judge's evidence lists every competing proposal; the
                // physical scalar/pointwise split of the old engine is not
                // part of the verdict — the cited facts are.
                Violation::functionality(statement_ref, fact, Conflict::Scalar)
            }
            crate::schema::StatementRef::Containment(_) => {
                let direction = match violation.direction {
                    Some(JudgedDirection::SourceUnsatisfied) | None => Direction::SourceUnsatisfied,
                };
                Violation::containment(statement_ref, direction, fact)
            }
            crate::schema::StatementRef::Capacity(_) => {
                Violation::capacity(statement_ref, fact, violation.measure.unwrap_or(0))
            }
        };
        let cited: Box<[CitedFact]> = violation
            .examples
            .iter()
            .map(|example| {
                let field_count = schema.relation(example.relation).fields().len();
                CitedFact::new(example.relation, field_count, example.values.clone())
            })
            .collect();
        citations.push((typed, cited));
        // The judge's own per-statement example-budget label survives the
        // public boundary (one flag per citation, same order): decide-time
        // evidence encoding reads it back via `Violations::examples_truncated`.
        truncated.push(violation.examples_truncated);
    }
    if citations.is_empty() {
        // Unreachable by the judge contract; refuse loudly instead of
        // minting an empty rejection.
        return Err(Error::from_store(
            crate::storage::store::StoreError::Corruption(
                crate::storage::store::error::StoreCorruption::MalformedKey(
                    "empty judge rejection",
                ),
            ),
        ));
    }
    Ok(Violations::from_pairs_with_truncation(
        citations.into_boxed_slice(),
        truncated.into_boxed_slice(),
    ))
}
