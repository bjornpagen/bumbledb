//! Closed-relation evaluation: a closed relation's rows are sealed at validate,
//! so a query atom over it whose filters are prepare-resolvable is not a join to
//! plan. The evaluator runs the filters against the sealed rows and folds the atom
//! into the id set that survives (or proves the rule empty).
use std::collections::BTreeSet;

use crate::error::Error;
use crate::image::TextEq;
use crate::image::view::{Const, FilterPredicate, Loaded, OperandAddr};
use crate::ir::normalize::{FoldedMark, NormalizedQuery, Role};
use crate::ir::{VarId, WordCmp};
use crate::plan::fj::OccBind;
use crate::schema::{Relation, Schema};
use bumbledb_theory::schema::{FieldId, RelationId};

use super::var_is_dead;

pub(super) fn fold_step(
    normalized: &mut NormalizedQuery,
    schema: &Schema,
    output_vars: &BTreeSet<VarId>,
) -> bool {
    for c_idx in 0..normalized.occurrences.len() {
        let folded = match &normalized.occurrences[c_idx].role {
            Role::Positive => fold_positive(normalized, schema, output_vars, c_idx),
            Role::Negated => fold_negated(normalized, schema, c_idx),
            Role::Eliminated(_) | Role::Folded(_) => false,
        };
        if folded {
            return true;
        }
    }
    false
}

fn fold_positive(
    normalized: &mut NormalizedQuery,
    schema: &Schema,
    output_vars: &BTreeSet<VarId>,
    c_idx: usize,
) -> bool {
    let occurrence = &normalized.occurrences[c_idx];

    let OccBind::Edb(relation_id) = OccBind::of_occurrence(occurrence) else {
        return false;
    };
    let relation = schema.relation(relation_id);
    if relation.body().closed_rows().is_none() {
        return false;
    }
    if !occurrence
        .filters
        .iter()
        .all(crate::image::view::is_prepare_resolvable)
    {
        return false; // condition 2 refusal (params, measures)
    }
    if payload_escapes(normalized, c_idx, output_vars) {
        return false; // condition 1 refusal: the payload projection keeps its join
    }
    let binders = if let Some(k) = join_id_var(normalized, c_idx, output_vars) {
        let binders = membership_binders(normalized, c_idx, k);
        if binders.is_empty() {
            return false;
        }
        binders
    } else {
        if !normalized.occurrences[c_idx].vars.is_empty() {
            return false;
        }

        if !normalized
            .occurrences
            .iter()
            .enumerate()
            .any(|(idx, occ)| idx != c_idx && occ.role.participates())
        {
            return false;
        }
        Vec::new()
    };
    let Ok(survivors) = surviving_ids(relation, &normalized.occurrences[c_idx].filters) else {
        return false;
    };
    if survivors.is_empty() {
        normalized.dead = Some(format!(
            "folded to ∅: {}",
            folded_picture(schema, relation_id, &normalized.occurrences[c_idx].filters,)
        ));
        return true;
    }
    attach_membership(normalized, &binders, &survivors);
    normalized.occurrences[c_idx].role = Role::Folded(folded_positive(relation_id, survivors));
    true
}

fn fold_negated(normalized: &mut NormalizedQuery, schema: &Schema, c_idx: usize) -> bool {
    let occurrence = &normalized.occurrences[c_idx];

    let OccBind::Edb(relation_id) = OccBind::of_occurrence(occurrence) else {
        return false;
    };
    let relation = schema.relation(relation_id);
    let Some(rows) = relation.body().closed_rows() else {
        return false;
    };
    if !occurrence
        .filters
        .iter()
        .all(crate::image::view::is_prepare_resolvable)
    {
        return false;
    }
    let Ok(survivors) = surviving_ids(relation, &normalized.occurrences[c_idx].filters) else {
        return false;
    };
    if survivors.is_empty() {
        remove_anti_probe(normalized, c_idx);
        normalized.occurrences[c_idx].role = Role::Folded(folded_negated(relation_id, Vec::new()));
        return true;
    }
    if occurrence.vars.is_empty() {
        normalized.dead = Some(format!(
            "folded: !{} rejects every binding",
            folded_picture(schema, relation_id, &occurrence.filters)
        ));
        return true;
    }

    // A negated closed atom folds only when it binds its id alone; other
    // shapes would need multi-column set reasoning, so the anti-probe stays.
    let &[(FieldId(0), k)] = occurrence.vars.as_slice() else {
        return false;
    };
    let closed = relation_id;
    let binders = membership_binders(normalized, c_idx, k);
    if binders.is_empty() {
        return false;
    }
    if !domain_within_ids(normalized, schema, c_idx, k, closed) {
        // The complement is only sound when `k` ranges over the closed ids.
        return false;
    }
    let extension_len = u64::try_from(rows.len()).expect("extensions cap at 256 rows");
    let complement: Vec<u64> = (0..extension_len)
        .filter(|id| survivors.binary_search(id).is_err())
        .collect();
    if complement.is_empty() {
        normalized.dead = Some(format!(
            "folded: !{} rejects every binding",
            folded_picture(schema, closed, &normalized.occurrences[c_idx].filters)
        ));
        return true;
    }
    let mark = folded_negated(closed, survivors);
    attach_membership(normalized, &binders, &complement);
    remove_anti_probe(normalized, c_idx);
    normalized.occurrences[c_idx].role = Role::Folded(mark);
    true
}

fn assert_fold_cap(survivors: &[u64]) {
    assert!(survivors.len() <= 256, "extensions cap at 256 rows");
}

fn folded_positive(relation: RelationId, survivors: Vec<u64>) -> FoldedMark {
    assert_fold_cap(&survivors);
    FoldedMark::Positive {
        relation,
        survivors: survivors.into_boxed_slice(),
    }
}

fn folded_negated(relation: RelationId, survivors: Vec<u64>) -> FoldedMark {
    assert_fold_cap(&survivors);
    FoldedMark::Negated {
        relation,
        survivors: survivors.into_boxed_slice(),
    }
}

/// **Condition 1 (refusal half)** — whether any non-id variable of `c_idx` is
/// live outside it: a payload variable escaping to the head, another
/// occurrence, or a residual/anti-probe/membership-point read.
pub(super) fn payload_escapes(
    normalized: &NormalizedQuery,
    c_idx: usize,
    output_vars: &BTreeSet<VarId>,
) -> bool {
    normalized.occurrences[c_idx]
        .vars
        .iter()
        .any(|(field, var)| {
            *field != FieldId(0) && !var_is_dead(normalized, c_idx, *var, output_vars)
        })
}

pub(super) fn join_id_var(
    normalized: &NormalizedQuery,
    c_idx: usize,
    output_vars: &BTreeSet<VarId>,
) -> Option<VarId> {
    normalized.occurrences[c_idx]
        .vars
        .iter()
        .find(|(field, _)| *field == FieldId(0))
        .map(|(_, var)| *var)
        .filter(|var| !var_is_dead(normalized, c_idx, *var, output_vars))
}

/// One closed row's values as filter operands. A sealed value and a query
/// literal share one word convention, so each field lowers like a literal.
/// Closed relations hold no text, so no field compares through [`TextEq`].
struct ClosedRow<'a>(&'a [crate::ir::Value]);

impl crate::image::view::Operands for ClosedRow<'_> {
    type Error = std::convert::Infallible;

    fn word(&self, at: OperandAddr) -> Result<u64, Self::Error> {
        Ok(match self.loaded(at)? {
            Loaded::Word(w) => w,
            Loaded::Byte(b) => u64::from(b),
            Loaded::Pair(..) | Loaded::Block { .. } => {
                unreachable!("validated: word operands are scalar")
            }
        })
    }

    fn pair(&self, at: OperandAddr) -> Result<(u64, u64), Self::Error> {
        Ok(match self.loaded(at)? {
            Loaded::Pair(s, e) => (s, e),
            Loaded::Word(_) | Loaded::Byte(_) | Loaded::Block { .. } => {
                unreachable!("validated: interval predicates read interval fields")
            }
        })
    }

    fn loaded(&self, at: OperandAddr) -> Result<Loaded, Self::Error> {
        Ok(
            match crate::ir::normalize::lower_literal(&self.0[usize::from(at.field().0)]) {
                Const::Word(word) => Loaded::Word(word),
                Const::Byte(byte) => Loaded::Word(u64::from(byte)),
                Const::Interval { start, end } => Loaded::Pair(start, end),
                Const::Words(words) => {
                    let mut block = [0u64; 8];
                    block[..words.len()].copy_from_slice(&words);
                    Loaded::Block {
                        words: block,
                        count: u8::try_from(words.len()).expect("bytes width is at most 8 words"),
                    }
                }
                _ => unreachable!("closed relations hold no text"),
            },
        )
    }
}

/// Closed-row σ: the ids of the rows every filter holds on.
fn surviving_ids(relation: &Relation, filters: &[FilterPredicate]) -> Result<Vec<u64>, Error> {
    let mut ids = Vec::new();
    for (id, row) in relation
        .body()
        .closed_rows()
        .expect("callers checked closedness")
        .iter()
        .enumerate()
    {
        if closed_row_survives(&ClosedRow(&row.values), filters)? {
            ids.push(id as u64);
        }
    }
    Ok(ids)
}

fn closed_row_survives(row: &ClosedRow<'_>, filters: &[FilterPredicate]) -> Result<bool, Error> {
    for filter in filters {
        match crate::image::view::holds(filter, row, &[], TextEq::from_optional_generation(None))? {
            Some(true) => {}
            Some(false) | None => return Ok(false),
        }
    }
    Ok(true)
}

pub(super) fn membership_binders(
    normalized: &NormalizedQuery,
    c_idx: usize,
    var: VarId,
) -> Vec<(usize, FieldId)> {
    normalized
        .occurrences
        .iter()
        .enumerate()
        .filter(|(idx, occ)| *idx != c_idx && occ.role.participates())
        .filter_map(|(idx, occ)| {
            occ.vars
                .iter()
                .find(|(_, v)| *v == var)
                .map(|(field, _)| (idx, *field))
        })
        .collect()
}

pub(super) fn domain_within_ids(
    normalized: &NormalizedQuery,
    schema: &Schema,
    c_idx: usize,
    k: VarId,
    closed: RelationId,
) -> bool {
    normalized
        .occurrences
        .iter()
        .enumerate()
        .filter(|(idx, occ)| *idx != c_idx && occ.role.participates())
        .any(|(_, occ)| {
            occ.vars.iter().any(|(field, var)| {
                *var == k
                    && ((OccBind::of_occurrence(occ) == OccBind::Edb(closed)
                        && *field == FieldId(0))
                        || containment_into_id(schema, occ, *field, closed))
            })
        })
}

fn containment_into_id(
    schema: &Schema,
    occurrence: &crate::ir::normalize::Occurrence,
    field: FieldId,
    closed: RelationId,
) -> bool {
    schema.containments().iter().any(|statement| {
        OccBind::of_occurrence(occurrence) == OccBind::Edb(statement.source.relation)
            && statement.source.projection.as_ref() == [field]
            && statement.target.relation == closed
            && statement.target.projection.as_ref() == [FieldId(0)]
            && super::encoded_selection(&statement.source).is_some_and(|phi| {
                phi.iter().all(|(f, value)| {
                    occurrence.filters.iter().any(|filter| {
                        matches!(
                            filter,
                            FilterPredicate::Compare { field: ff, op: WordCmp::Eq, value: v }
                                if ff == f && v == value
                        )
                    })
                })
            })
    })
}

/// `ids` is sorted ascending (construction order), the `WordSet` invariant.
fn attach_membership(normalized: &mut NormalizedQuery, binders: &[(usize, FieldId)], ids: &[u64]) {
    debug_assert!(!ids.is_empty(), "empty sets take the rule-death path");
    debug_assert!(ids.windows(2).all(|w| w[0] < w[1]), "sorted, deduplicated");
    for (idx, field) in binders {
        normalized.occurrences[*idx]
            .filters
            .push(FilterPredicate::Compare {
                field: (*field).into(),
                op: WordCmp::Eq,
                value: Const::WordSet(ids.to_vec().into()),
            });
    }
}

fn remove_anti_probe(normalized: &mut NormalizedQuery, c_idx: usize) {
    let occ_id = normalized.occurrences[c_idx].occ_id;
    normalized
        .anti_probes
        .retain(|probe| probe.occurrence != occ_id);
}

pub(crate) fn folded_picture(
    schema: &Schema,
    relation: RelationId,
    filters: &[FilterPredicate],
) -> String {
    let relation = schema.relation(relation);
    let mut out = String::from(relation.name());
    out.push('{');
    for (index, filter) in filters.iter().enumerate() {
        if index > 0 {
            out.push_str(" ∧ ");
        }
        crate::image::view::render_filter(&mut out, relation, filter);
    }
    out.push('}');
    out
}

#[cfg(test)]
mod tests;
