use crate::ir::normalize::NormalizedQuery;
use crate::plan::fj::OccBind;
use crate::plan::pinned_fields;
use crate::schema::{CompiledTheory, DistinctnessWitness, Schema};
use bumbledb_theory::schema::FieldId;
use std::collections::BTreeSet;

/// Proof that distinct facts imply distinct bindings for this rule:
/// every participating occurrence's bound fields cover a compiled key
/// witness of its stored relation. Carrying this witness is the license
/// to construct an aggregate sink without a binding seen-set (chapter 12
/// §2's preserved and requalified elided-dedup regime).
///
/// Two arms prove key coverage, both per participating EDB occurrence:
///
/// 1. **Compiled scalar key**: the occurrence's bound fields (variable
///    bindings plus equality-pinned constants/params) are a superset of
///    some interned [`DistinctnessWitness::ScalarKeyUnique`] projection.
///    Pointwise keys are **not** this arm — L01 emits
///    [`DistinctnessWitness::FullRowEquality`] for them.
/// 2. **Whole-row implicit key**: the bound fields cover every compiled
///    field of the relation. [`CompiledTheory::full_row_witness`] is the
///    semantic premise (set identity), never a raw-schema re-read.
///
/// Soundness in both directions, per occurrence: a binding fixes the
/// value of every covered field, and key coverage means at most one fact
/// matches, so each full binding tuple has exactly one derivation
/// (multiplicity 1 — elision is exact). Conversely two distinct facts
/// must differ on a bound field (agreeing on all of them would make them
/// equal under the covered key), and pinned fields cannot differ, so they
/// differ on a variable-bound field and yield distinct bindings — the
/// license for raw-multiplicity constant-group folds.
///
/// Var-sourced point-membership probes (`point_vars`) never count as
/// binding their interval field: a point inside the interval does not
/// determine the interval.
///
/// Derived occurrences (`Finished`/`RecDelta`/`RecAcc`) are never proven
/// here: their column vocabulary is not a schema relation, so neither arm
/// applies (a sound extension for fully-bound sealed interiors would need
/// the interior arity threaded in — not required by any consumer today).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DistinctWitness(());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Distinctness {
    Proven(DistinctWitness),
    Unproven,
}

pub(crate) fn provably_distinct(
    normalized: &NormalizedQuery,
    schema: &Schema,
) -> Option<DistinctWitness> {
    let theory = schema.compiled_theory().ok()?;
    normalized
        .occurrences
        .iter()
        .filter(|occurrence| occurrence.role.participates())
        .all(|occurrence| occurrence_covers_compiled_key(occurrence, schema, theory))
        .then_some(DistinctWitness(()))
}

fn occurrence_covers_compiled_key(
    occurrence: &crate::ir::normalize::Occurrence,
    schema: &Schema,
    theory: &CompiledTheory,
) -> bool {
    let OccBind::Edb(stored) = OccBind::of_occurrence(occurrence) else {
        return false;
    };
    let bound_fields: BTreeSet<FieldId> = occurrence
        .vars
        .iter()
        .map(|(f, _)| *f)
        .chain(pinned_fields(occurrence).map(|(field, _)| field))
        .collect();
    let covers_scalar_key = theory.key_projections_of(stored).iter().any(|id| {
        let Some(DistinctnessWitness::ScalarKeyUnique { .. }) = theory.distinctness_witness(*id)
        else {
            return false;
        };
        let Some(projection) = theory.projection(*id) else {
            return false;
        };
        projection
            .projection
            .iter()
            .all(|field| bound_fields.contains(field))
    });
    let covers_whole_row = theory
        .fields_of(stored)
        .or_else(|| Some(schema.relation(stored).fields()))
        .is_some_and(|fields| {
            fields
                .iter()
                .enumerate()
                .all(|(ordinal, _)| {
                    u16::try_from(ordinal)
                        .is_ok_and(|ordinal| bound_fields.contains(&FieldId(ordinal)))
                })
                && matches!(
                    theory.full_row_witness(),
                    DistinctnessWitness::FullRowEquality
                )
        });
    covers_scalar_key || covers_whole_row
}
