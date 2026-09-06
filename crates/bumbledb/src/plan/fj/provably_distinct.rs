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
/// Three arms prove key coverage, each per participating EDB occurrence:
///
/// 1. **Compiled scalar key**: the occurrence's bound fields (variable
///    bindings plus equality-pinned constants/params) are a superset of
///    some interned [`DistinctnessWitness::ScalarKeyUnique`] projection.
/// 2. **Complete interval key**: all scalar fields AND the interval value
///    are bound under [`DistinctnessWitness::IntervalKeyUnique`]. Distinct
///    rows with one scalar group must have disjoint intervals. Since every
///    valid interval is nonempty, equal intervals overlap, proving uniqueness
///    of the complete projected tuple. The scalar bucket alone is not unique.
/// 3. **Whole-row implicit key**: the bound fields cover every compiled
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
/// Derived occurrences (`Finished`/`RecDelta`) are never proven
/// here: their column vocabulary is not a schema relation, so these arms
/// do not apply (a sound extension for fully-bound sealed interiors would need
/// the interior arity threaded in — not required by any consumer today).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DistinctWitness(());

/// A plain output tuple determines every participating stored fact, not
/// merely the complete binding. Valid only for this exact rule/head under
/// the same lawful source premises as `DistinctWitness`. Combining rules,
/// retargeting the head, or iterating recursive arms requires fresh proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProjectionDistinctWitness(());

pub(crate) fn provably_distinct_projection(
    normalized: &NormalizedQuery,
    schema: &Schema,
    finds: &[crate::ir::FindTerm],
) -> Option<ProjectionDistinctWitness> {
    // Computation may erase distinctions; aggregate multiplicity has its
    // own proof. Empty heads stay on the ordinary unit-tuple set path.
    let mut determined: BTreeSet<_> = finds
        .iter()
        .map(|find| match find {
            crate::ir::FindTerm::Var(var) => Some(*var),
            _ => None,
        })
        .collect::<Option<_>>()?;
    if determined.is_empty() {
        return None;
    }
    let theory = schema.compiled_theory().ok()?;
    let mut remaining: Vec<_> = normalized
        .occurrences
        .iter()
        .filter(|occurrence| occurrence.role.participates())
        .collect();
    // A covered key determines its entire fact, including ordinary bound
    // variables hidden from the head. Those variables may cover another
    // occurrence's key. Resolve to a fixed point, independent of atom order.
    // No containment premise is needed: missing targets yield no output.
    while !remaining.is_empty() {
        let before = remaining.len();
        remaining.retain(|occurrence| {
            if !occurrence_covers_compiled_key(occurrence, schema, theory, Some(&determined)) {
                return true;
            }
            // A determined interval does NOT determine a point inside it.
            // Only exact field bindings propagate; never `point_vars`.
            determined.extend(occurrence.vars.iter().map(|(_, var)| *var));
            false
        });
        if remaining.len() == before {
            return None;
        }
    }
    Some(ProjectionDistinctWitness(()))
}

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
        .all(|occurrence| occurrence_covers_compiled_key(occurrence, schema, theory, None))
        .then_some(DistinctWitness(()))
}

fn occurrence_covers_compiled_key(
    occurrence: &crate::ir::normalize::Occurrence,
    schema: &Schema,
    theory: &CompiledTheory,
    output: Option<&BTreeSet<crate::ir::VarId>>,
) -> bool {
    let OccBind::Edb(stored) = OccBind::of_occurrence(occurrence) else {
        return false;
    };
    let bound_fields: BTreeSet<FieldId> = occurrence
        .vars
        .iter()
        .filter(|(_, var)| output.is_none_or(|output| output.contains(var)))
        .map(|(f, _)| *f)
        .chain(pinned_fields(occurrence).map(|(field, _)| field))
        .collect();
    let covers_declared_key = theory.key_projections_of(stored).iter().any(|id| {
        let Some(
            DistinctnessWitness::ScalarKeyUnique { .. }
            | DistinctnessWitness::IntervalKeyUnique { .. },
        ) = theory.distinctness_witness(*id)
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
            fields.iter().enumerate().all(|(ordinal, _)| {
                u16::try_from(ordinal).is_ok_and(|ordinal| bound_fields.contains(&FieldId(ordinal)))
            })
        });
    covers_declared_key || covers_whole_row
}
