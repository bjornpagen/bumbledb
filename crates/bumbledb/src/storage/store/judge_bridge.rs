//! The judge's view of store state: a candidate transaction for incremental
//! judgment, or a committed snapshot for complete judgment.

use bumbledb_theory::schema::RelationId;

use super::candidate::CandidateState;
use super::snapshot::OwnedSnapshot;
use crate::Value;
use crate::canonical::DecodeScratch;
use crate::changes::ChangeKind;
use crate::error::{Error, Result};
use crate::schema::compiled::CompiledProjection;
use crate::schema::judge::{
    CandidateFacts, DeltaFacts, DeltaShape, JudgeBudget, JudgeError, JudgedViolation, Judgment,
    RankedRowVisitor, judge_final_state, judge_final_state_delta_local,
};
use crate::schema::{Schema, StatementId};
use crate::work::WorkContext;

/// A judgment's rejection, if any.
pub(crate) type Verdict = Option<Box<[JudgedViolation]>>;

fn verdict(judged: Result<Judgment, JudgeError<Error>>) -> Result<Verdict> {
    match judged {
        Ok(Judgment::Admitted) => Ok(None),
        Ok(Judgment::Rejected(violations)) => Ok(Some(violations)),
        Err(JudgeError::Work(error)) => Err(Error::from(error)),
        Err(JudgeError::State(error)) => Err(error),
        Err(JudgeError::UndefinedDuration { statement }) => {
            Err(Error::CapacityRayMeasure { statement })
        }
        Err(JudgeError::MeasureOverflow { statement }) => Err(Error::MeasureOverflow { statement }),
        Err(JudgeError::Compile(error)) => Err(Error::Compile(error)),
    }
}

/// Incremental judgment of a candidate over its lawful parent.
pub(crate) fn judge_incremental(
    schema: &Schema,
    state: &CandidateState<'_>,
    work: &WorkContext,
) -> Result<Verdict> {
    let view = CandidateView::new(state, schema, work);
    verdict(judge_final_state_delta_local(
        schema,
        &view,
        work,
        JudgeBudget::default(),
    ))
}

/// Complete judgment of a candidate's proposed state.
#[cfg(test)]
pub(crate) fn judge_complete_candidate(
    schema: &Schema,
    state: &CandidateState<'_>,
    work: &WorkContext,
) -> Result<Verdict> {
    let view = CandidateView::new(state, schema, work);
    verdict(judge_final_state(
        schema,
        &view,
        work,
        JudgeBudget::default(),
    ))
}

/// Complete judgment of a committed snapshot.
pub(crate) fn judge_snapshot(
    schema: &Schema,
    snapshot: &OwnedSnapshot,
    work: &WorkContext,
) -> Result<Verdict> {
    let facts = SnapshotFacts {
        snapshot,
        schema,
        work,
    };
    verdict(judge_final_state(
        schema,
        &facts,
        work,
        JudgeBudget::default(),
    ))
}

pub(crate) struct SnapshotFacts<'a> {
    pub(crate) snapshot: &'a OwnedSnapshot,
    pub(crate) schema: &'a Schema,
    pub(crate) work: &'a WorkContext,
}

impl CandidateFacts for SnapshotFacts<'_> {
    type Error = Error;

    fn visit_rows(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Self::Error>,
    ) -> Result<(), Self::Error> {
        self.visit_ranked_rows(relation, &mut |_, row| visit(row))
    }

    fn visit_ranked_rows(
        &self,
        relation: RelationId,
        visit: RankedRowVisitor<'_, Self::Error>,
    ) -> Result<(), Self::Error> {
        let fields = self.schema.relation(relation).fields();
        let mut decode = DecodeScratch::new(self.work);
        for entry in self.snapshot.rows(relation)? {
            let (id, bytes) = entry?;
            if !decode.with_decoded(fields, bytes, &mut |row: &[Value]| visit(id.0, row))? {
                break;
            }
        }
        Ok(())
    }
}

struct CandidateView<'v, 'a> {
    state: &'v CandidateState<'a>,
    schema: &'v Schema,
    work: &'v WorkContext,
    delta: Vec<(RelationId, DeltaShape)>,
}

impl<'v, 'a> CandidateView<'v, 'a> {
    fn new(state: &'v CandidateState<'a>, schema: &'v Schema, work: &'v WorkContext) -> Self {
        let mut delta: Vec<(RelationId, DeltaShape)> = Vec::new();
        for record in state.changes().records() {
            let shape = match delta.binary_search_by_key(&record.relation, |&(id, _)| id) {
                Ok(at) => &mut delta[at].1,
                Err(at) => {
                    delta.insert(at, (record.relation, DeltaShape::default()));
                    &mut delta[at].1
                }
            };
            match record.kind {
                ChangeKind::Add => shape.adds = true,
                ChangeKind::Remove => shape.removes = true,
            }
        }
        Self {
            state,
            schema,
            work,
            delta,
        }
    }

    fn visit_change_kind(
        &self,
        relation: RelationId,
        kind: ChangeKind,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Error>,
    ) -> Result<()> {
        let shape = self.delta_shape(relation);
        if match kind {
            ChangeKind::Add => !shape.adds,
            ChangeKind::Remove => !shape.removes,
        } {
            return Ok(());
        }
        let fields = self.schema.relation(relation).fields();
        // This visit owns its workspace: a callback may re-enter another
        // stream without aliasing the live borrowed row.
        let mut decode = DecodeScratch::new(self.work);
        for record in self.state.changes().records() {
            if record.relation > relation {
                break;
            }
            if record.relation == relation
                && record.kind == kind
                && !decode.with_decoded(fields, record.row, &mut *visit)?
            {
                break;
            }
        }
        Ok(())
    }
}

impl CandidateFacts for CandidateView<'_, '_> {
    type Error = Error;

    fn visit_rows(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Self::Error>,
    ) -> Result<(), Self::Error> {
        self.visit_ranked_rows(relation, &mut |_, row| visit(row))
    }

    fn visit_ranked_rows(
        &self,
        relation: RelationId,
        visit: RankedRowVisitor<'_, Self::Error>,
    ) -> Result<(), Self::Error> {
        let fields = self.schema.relation(relation).fields();
        let mut decode = DecodeScratch::new(self.work);
        for entry in self.state.rows(relation)? {
            let (id, bytes) = entry?;
            if !decode.with_decoded(fields, bytes, &mut |row: &[Value]| visit(id.0, row))? {
                break;
            }
        }
        Ok(())
    }
}

impl DeltaFacts for CandidateView<'_, '_> {
    fn scalar_key_preserved(&self, statement: StatementId) -> bool {
        self.state.preserves_home_key(statement)
    }

    fn delta_shape(&self, relation: RelationId) -> DeltaShape {
        self.delta
            .binary_search_by_key(&relation, |&(id, _)| id)
            .map_or_else(|_| DeltaShape::default(), |at| self.delta[at].1)
    }

    fn visit_added_rows(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Error>,
    ) -> Result<(), Error> {
        self.visit_change_kind(relation, ChangeKind::Add, visit)
    }

    fn visit_removed_rows(
        &self,
        relation: RelationId,
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Error>,
    ) -> Result<(), Error> {
        self.visit_change_kind(relation, ChangeKind::Remove, visit)
    }

    fn visit_key_competitors(
        &self,
        statement: StatementId,
        determinant: &[Value],
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Error>,
    ) -> Result<Option<()>, Error> {
        let theory = self.schema.compiled_theory().map_err(Error::Compile)?;
        let Some(compiled) = theory.projection_of_statement(statement) else {
            return Ok(None);
        };
        visit_compiled_bucket(self, compiled, determinant, &mut |_, row| visit(row))
    }

    fn visit_compiled_group(
        &self,
        projection: &CompiledProjection,
        determinant: &[Value],
        visit: &mut dyn FnMut(&[Value]) -> Result<bool, Error>,
    ) -> Result<Option<()>, Error> {
        visit_compiled_bucket(self, projection, determinant, &mut |_, row| visit(row))
    }

    fn visit_ranked_compiled_group(
        &self,
        projection: &CompiledProjection,
        determinant: &[Value],
        visit: RankedRowVisitor<'_, Self::Error>,
    ) -> Result<Option<()>, Self::Error> {
        visit_compiled_bucket(self, projection, determinant, visit)
    }
}

fn visit_compiled_bucket(
    view: &CandidateView<'_, '_>,
    compiled: &CompiledProjection,
    determinant: &[Value],
    visit: RankedRowVisitor<'_, Error>,
) -> Result<Option<()>, Error> {
    let projected = super::det_index::determinant_bytes(compiled, determinant, view.work)?;
    let fields = view.schema.relation(compiled.relation).fields();
    // The bucket visit owns its decode workspace; a nested visit allocates
    // its own and cannot overwrite this borrowed row.
    let mut decode = DecodeScratch::new(view.work);
    view.state
        .visit_determinant_bucket(compiled, &projected, view.work, &mut |id, bytes| {
            decode.with_decoded(fields, bytes, |values| {
                if determinant.len() == compiled.scalar_positions.len()
                    && compiled.scalar_positions.iter().zip(determinant).all(
                        |(&position, expected)| {
                            &values[usize::from(compiled.projection[position].0)] == expected
                        },
                    )
                {
                    visit(id.0, values)
                } else {
                    Ok(true)
                }
            })
        })?;
    Ok(Some(()))
}
