//! Complete statement-phase roster: every instance-dependent containment
//! and capacity, judged through the shared [`Checker`].

use crate::encoding::{encode_u64, field_word_bytes};
use crate::error::{Check, Direction, Result, Violation};
use crate::schema::{
    AxiomIndex, CapacityEnforcement, CompleteObligation, Enforcement, RelationBody, Schema,
};
use crate::storage::catalog::{CatalogRead, FactCursor};
use crate::storage::commit::judgment::{Checker, Probe, Selections, collect, satisfies};
use crate::storage::keys::{self, DeterminantImage};

/// Source-to-target complete admission. Key obligations were judged in
/// the merge; this walk is containments and capacities only.
pub(crate) fn judge_complete<C: CatalogRead>(
    catalog: &C,
    schema: &Schema,
    selections: &Selections<'_>,
) -> Result<Vec<Violation>> {
    let mut checker = Checker::new(catalog, schema);
    let mut violations = Vec::new();
    let mut image = DeterminantImage::scratch();
    for obligation in schema.complete_obligations().iter() {
        match obligation {
            CompleteObligation::Key { .. } => {}
            CompleteObligation::Containment { id, statement } => {
                judge_containment(
                    catalog,
                    schema,
                    selections,
                    &mut checker,
                    &mut image,
                    id,
                    statement,
                    &mut violations,
                )?;
            }
            CompleteObligation::Capacity { id, statement } => {
                judge_capacity(
                    catalog,
                    schema,
                    selections,
                    &mut checker,
                    &mut image,
                    id,
                    statement,
                    &mut violations,
                )?;
            }
        }
    }
    Ok(violations)
}

#[allow(clippy::too_many_arguments)]
fn judge_containment<C: CatalogRead>(
    catalog: &C,
    schema: &Schema,
    selections: &Selections<'_>,
    checker: &mut Checker<'_, C>,
    image: &mut DeterminantImage,
    id: crate::schema::ContainmentId,
    statement: &crate::schema::ContainmentStatement,
    violations: &mut Vec<Violation>,
) -> Result<()> {
    let checks = selections.containment(id);
    let source = schema.relation(statement.source.relation);
    let layout = source.layout();
    if let Some(rows) = source.body().closed_rows() {
        for row in rows {
            if !satisfies(&checks.source, layout, &row.fact) {
                continue;
            }
            collect(
                probe_source(schema, checker, image, statement, &checks.target, &row.fact),
                violations,
            )?;
        }
        return Ok(());
    }
    let mut cursor = catalog.scan_facts(statement.source.relation)?;
    while let Some(entry) = FactCursor::next(&mut cursor)? {
        let fact = entry.bytes.to_vec();
        if !satisfies(&checks.source, layout, &fact) {
            continue;
        }
        collect(
            probe_source(schema, checker, image, statement, &checks.target, &fact),
            violations,
        )?;
    }
    Ok(())
}

/// Source-to-target: a miss is [`Direction::SourceUnsatisfied`].
/// [`Direction::TargetRequired`] is the reverse-edge / deleted-target
/// scan — complete admission does not run that walk.
fn probe_source<C: CatalogRead>(
    schema: &Schema,
    checker: &mut Checker<'_, C>,
    image: &mut DeterminantImage,
    statement: &crate::schema::ContainmentStatement,
    target_check: &crate::storage::commit::judgment::SelectionCheck,
    fact: &[u8],
) -> Result<Check> {
    let layout = schema.relation(statement.source.relation).layout();
    match &statement.enforcement {
        Enforcement::Closed { members } => {
            let id = u64::from_be_bytes(field_word_bytes(
                layout.encoded(fact),
                usize::from(statement.source.projection[0].0),
            ));
            if AxiomIndex::try_from(id).is_ok_and(|index| members.contains(index)) {
                Ok(Check::Holds)
            } else {
                Ok(Check::Violated(Violation::containment(
                    schema.cite(statement.id),
                    statement.id,
                    Direction::SourceUnsatisfied,
                    fact.into(),
                )))
            }
        }
        Enforcement::ScalarProbe { key_projection, .. } => {
            keys::determinant_image(layout.encoded(fact), key_projection, image);
            checker.check_scalar(&Probe::of(
                statement,
                target_check,
                image.as_bytes(),
                fact,
                Direction::SourceUnsatisfied,
            ))
        }
        Enforcement::IntervalCoverage {
            disjoint,
            source_tail,
            target_tail,
            key_projection,
            ..
        } => {
            keys::determinant_image(layout.encoded(fact), key_projection, image);
            let probe = Probe::of(
                statement,
                target_check,
                image.as_bytes(),
                fact,
                Direction::SourceUnsatisfied,
            );
            checker.check_coverage(*disjoint, *source_tail, *target_tail, &probe)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn judge_capacity<C: CatalogRead>(
    catalog: &C,
    schema: &Schema,
    selections: &Selections<'_>,
    checker: &mut Checker<'_, C>,
    image: &mut DeterminantImage,
    id: crate::schema::CapacityId,
    statement: &crate::schema::CapacityStatement,
    violations: &mut Vec<Violation>,
) -> Result<()> {
    let checks = selections.capacity(id);
    match &statement.enforcement {
        CapacityEnforcement::Closed { .. } => {
            let rows = schema
                .relation(statement.target.relation)
                .body()
                .closed_rows()
                .expect("closed capacity parent has sealed rows");
            for row_index in 0..rows.len() {
                let parent = encode_u64(u64::try_from(row_index).expect("row index fits u64"));
                collect(
                    checker.check_capacity(statement, checks, &parent),
                    violations,
                )?;
            }
        }
        CapacityEnforcement::ScalarProbe { target_key, .. } => {
            let target = schema.relation(statement.target.relation);
            if matches!(target.body(), RelationBody::Closed { .. }) {
                return Ok(());
            }
            let key_statement = schema.key(*target_key);
            let layout = target.layout();
            let mut cursor = catalog.scan_facts(statement.target.relation)?;
            while let Some(entry) = FactCursor::next(&mut cursor)? {
                let fact = entry.bytes.to_vec();
                if !satisfies(&checks.target, layout, &fact) {
                    continue;
                }
                keys::determinant_image(layout.encoded(&fact), &key_statement.projection, image);
                collect(
                    checker.check_capacity(statement, checks, image.as_bytes()),
                    violations,
                )?;
            }
        }
    }
    Ok(())
}
