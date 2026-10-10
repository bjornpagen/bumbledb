//! Judge output as the public rejection: every violated statement once with
//! its id, the exact capacity measure, every bounded example as a
//! [`CitedFact`] and the judge's truncation labels. The convicting fact
//! bytes are the first example's canonical row.

use crate::canonical::CanonicalRow;
use crate::error::{CitedFact, Result, Violation, Violations};
use crate::schema::judge::JudgedViolation;
use crate::schema::{Schema, StatementView};
use crate::work::WorkContext;

pub(crate) fn violations_from_judged(
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
                Violation::functionality(statement_ref, fact)
            }
            crate::schema::StatementRef::Containment(_) => {
                Violation::containment(statement_ref, fact)
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
        truncated.push(violation.examples_truncated);
    }
    assert!(!citations.is_empty(), "a judge rejection cites a violation");
    Ok(Violations::from_pairs_with_truncation(
        citations.into_boxed_slice(),
        truncated.into_boxed_slice(),
    ))
}
