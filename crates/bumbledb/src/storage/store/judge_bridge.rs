//! The judge's view of store state: a candidate transaction for incremental
//! judgment, or a committed snapshot for complete judgment.

use std::ops::ControlFlow::Continue;

use bumbledb_theory::schema::RelationId;

use super::candidate::CandidateState;
use super::snapshot::OwnedSnapshot;
use crate::Value;
use crate::canonical::DecodeScratch;
use crate::changes::{ChangeKind, DeltaShape};
use crate::error::Result;
use crate::schema::compiled::CompiledProjection;
use crate::schema::judge::{
    DeltaFacts, Facts, Indexed, JudgeBudget, Judgment, RankedRowVisitor, RowVisitor,
    judge_complete, judge_incremental,
};
use crate::schema::{Schema, StatementId};
use crate::work::WorkContext;

/// Incremental judgment of a candidate over its lawful parent.
pub(crate) fn judge_candidate(
    schema: &Schema,
    state: &CandidateState<'_>,
    work: &WorkContext,
) -> Result<Judgment> {
    let view = CandidateView {
        state,
        schema,
        work,
    };
    judge_incremental(schema, &view, work, JudgeBudget::default())
}

/// Complete judgment of a candidate's proposed state.
#[cfg(test)]
pub(crate) fn judge_candidate_complete(
    schema: &Schema,
    state: &CandidateState<'_>,
    work: &WorkContext,
) -> Result<Judgment> {
    let view = CandidateView {
        state,
        schema,
        work,
    };
    judge_complete(schema, &view, work, JudgeBudget::default())
}

/// Complete judgment of a committed snapshot.
pub(crate) fn judge_snapshot(
    schema: &Schema,
    snapshot: &OwnedSnapshot,
    work: &WorkContext,
) -> Result<Judgment> {
    let facts = SnapshotFacts {
        snapshot,
        schema,
        work,
    };
    judge_complete(schema, &facts, work, JudgeBudget::default())
}

pub(crate) struct SnapshotFacts<'a> {
    pub(crate) snapshot: &'a OwnedSnapshot,
    pub(crate) schema: &'a Schema,
    pub(crate) work: &'a WorkContext,
}

impl Facts for SnapshotFacts<'_> {
    fn visit_rows(&self, relation: RelationId, visit: RowVisitor<'_>) -> Result<()> {
        self.visit_ranked_rows(relation, &mut |_, row| visit(row))
    }

    fn visit_ranked_rows(&self, relation: RelationId, visit: RankedRowVisitor<'_>) -> Result<()> {
        let fields = self.schema.relation(relation).fields();
        let mut decode = DecodeScratch::new(self.work);
        for entry in self.snapshot.rows(relation)? {
            let (id, bytes) = entry?;
            if decode
                .with_decoded(fields, bytes, |row| visit(id.0, row))?
                .is_break()
            {
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
}

impl CandidateView<'_, '_> {
    fn visit_change_kind(
        &self,
        relation: RelationId,
        kind: ChangeKind,
        visit: RowVisitor<'_>,
    ) -> Result<()> {
        let fields = self.schema.relation(relation).fields();
        // Each visit owns its workspace: a callback may re-enter another
        // stream without aliasing the borrowed row.
        let mut decode = DecodeScratch::new(self.work);
        for record in self.state.changes().records_of(relation) {
            if record.kind == kind
                && decode
                    .with_decoded(fields, record.row, &mut *visit)?
                    .is_break()
            {
                break;
            }
        }
        Ok(())
    }

    fn visit_bucket(
        &self,
        compiled: &CompiledProjection,
        determinant: &[Value],
        visit: RankedRowVisitor<'_>,
    ) -> Result<Indexed> {
        let projected = super::det_index::determinant_bytes(compiled, determinant, self.work)?;
        let fields = self.schema.relation(compiled.relation).fields();
        // The bucket visit owns its decode workspace; a nested visit
        // allocates its own and cannot overwrite this borrowed row.
        let mut decode = DecodeScratch::new(self.work);
        self.state.visit_determinant_bucket(
            compiled,
            &projected,
            self.work,
            &mut |id, bytes| {
                let flow = decode.with_decoded(fields, bytes, |values| {
                    let member = determinant.len() == compiled.scalar_positions.len()
                        && compiled.scalar_positions.iter().zip(determinant).all(
                            |(&position, expected)| {
                                &values[usize::from(compiled.projection[position].0)] == expected
                            },
                        );
                    if member {
                        visit(id.0, values)
                    } else {
                        Ok(Continue(()))
                    }
                })?;
                Ok(flow.is_continue())
            },
        )?;
        Ok(Indexed::Walked)
    }
}

impl Facts for CandidateView<'_, '_> {
    fn visit_rows(&self, relation: RelationId, visit: RowVisitor<'_>) -> Result<()> {
        self.visit_ranked_rows(relation, &mut |_, row| visit(row))
    }

    fn visit_ranked_rows(&self, relation: RelationId, visit: RankedRowVisitor<'_>) -> Result<()> {
        let fields = self.schema.relation(relation).fields();
        let mut decode = DecodeScratch::new(self.work);
        for entry in self.state.rows(relation)? {
            let (id, bytes) = entry?;
            if decode
                .with_decoded(fields, bytes, |row| visit(id.0, row))?
                .is_break()
            {
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
        self.state.changes().shape(relation)
    }

    fn visit_added_rows(&self, relation: RelationId, visit: RowVisitor<'_>) -> Result<()> {
        self.visit_change_kind(relation, ChangeKind::Add, visit)
    }

    fn visit_removed_rows(&self, relation: RelationId, visit: RowVisitor<'_>) -> Result<()> {
        self.visit_change_kind(relation, ChangeKind::Remove, visit)
    }

    fn visit_key_competitors(
        &self,
        statement: StatementId,
        determinant: &[Value],
        visit: RowVisitor<'_>,
    ) -> Result<Indexed> {
        let theory = self.schema.compiled_theory()?;
        let Some(compiled) = theory.projection_of_statement(statement) else {
            return Ok(Indexed::Unindexed);
        };
        self.visit_bucket(compiled, determinant, &mut |_, row| visit(row))
    }

    fn visit_compiled_group(
        &self,
        projection: &CompiledProjection,
        determinant: &[Value],
        visit: RowVisitor<'_>,
    ) -> Result<Indexed> {
        self.visit_bucket(projection, determinant, &mut |_, row| visit(row))
    }

    fn visit_ranked_compiled_group(
        &self,
        projection: &CompiledProjection,
        determinant: &[Value],
        visit: RankedRowVisitor<'_>,
    ) -> Result<Indexed> {
        self.visit_bucket(projection, determinant, visit)
    }
}
