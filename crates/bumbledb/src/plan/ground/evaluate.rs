//! The grounding-evaluator: folding stage-zero atoms
//! .
//! A closed relation's extension is sealed at validate — stage-0 data
//! . A query atom over
//! it whose filters are prepare-resolvable constants is therefore not a
//! join to plan: the evaluator runs the filters against the sealed rows
use std::collections::BTreeSet;

use crate::error::Error;
use crate::image::view::{Const, FilterPredicate};
use crate::image::TextEq;
use crate::ir::normalize::{FoldedMark, NormalizedQuery, Role};
use crate::ir::{VarId, WordCmp};
use crate::plan::fj::OccBind;
use crate::schema::{Relation, Schema};
use crate::work::{CacheLedger, GenerationHandle, GenerationState};
use bumbledb_theory::schema::{FieldId, RelationId, ValueType};

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
    let Some(survivors) = fold_surviving_ids(relation, &normalized.occurrences[c_idx].filters)
    else {
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
    let Some(survivors) = fold_surviving_ids(relation, &normalized.occurrences[c_idx].filters)
    else {
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

    // need multi-column set reasoning; REFUSED v0, recorded (trigger: a

    let &[(FieldId(0), k)] = occurrence.vars.as_slice() else {
        return false;
    };
    let closed = relation_id;
    let binders = membership_binders(normalized, c_idx, k);
    if binders.is_empty() {
        return false;
    }
    if !domain_within_ids(normalized, schema, c_idx, k, closed) {
        // direction this refusal pins). The anti-probe stays.
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

struct SealedRow<'a> {
    fact: crate::encoding::FactView<'a, 'a>,
}

impl crate::image::view::Operands for SealedRow<'_> {
    type Error = std::convert::Infallible;

    fn word(&self, at: crate::image::view::OperandAddr) -> Result<u64, Self::Error> {
        Ok(match self.loaded(at)? {
            crate::image::view::Loaded::Word(w) => w,
            crate::image::view::Loaded::Byte(b) => u64::from(b),
            crate::image::view::Loaded::Pair(..) | crate::image::view::Loaded::Block { .. } => {
                unreachable!("validated: word operands are scalar")
            }
        })
    }

    fn pair(&self, at: crate::image::view::OperandAddr) -> Result<(u64, u64), Self::Error> {
        Ok(match self.loaded(at)? {
            crate::image::view::Loaded::Pair(s, e) => (s, e),
            crate::image::view::Loaded::Word(_)
            | crate::image::view::Loaded::Byte(_)
            | crate::image::view::Loaded::Block { .. } => {
                unreachable!("validated: interval predicates read interval fields")
            }
        })
    }

    fn block(&self, at: crate::image::view::OperandAddr) -> Result<([u64; 8], u8), Self::Error> {
        Ok(match self.loaded(at)? {
            crate::image::view::Loaded::Block { words, count } => (words, count),
            _ => unreachable!("validated: block operands are bytes<N>"),
        })
    }

    fn loaded(
        &self,
        at: crate::image::view::OperandAddr,
    ) -> Result<crate::image::view::Loaded, Self::Error> {
        Ok(match sealed_operand(self.fact, at.field()) {
            crate::exec::dispatch::FactOperand::Word(w) => crate::image::view::Loaded::Word(w),
            crate::exec::dispatch::FactOperand::Pair(s, e) => {
                crate::image::view::Loaded::Pair(s, e)
            }
            crate::exec::dispatch::FactOperand::Block { words, count } => {
                crate::image::view::Loaded::Block { words, count }
            }
        })
    }

    fn string_field(&self, at: crate::image::view::OperandAddr) -> bool {
        matches!(
            self.fact.layout().field_type(usize::from(at.field().0)),
            ValueType::String
        )
    }
}

/// One sealed row field as column words, sliced straight out of the dense
/// [`crate::encoding::FactLayout`] encoding the extension was sealed with
/// at validate. The stored bytes already carry the physical word
/// conventions (U64 big-endian, I64 sign-flipped, F64 total-order key,
/// Id128 two big-endian words, intervals two order words — a fixed-width
/// slot stores the start word and the layout width recovers the end), so
/// each word is a direct big-endian load; the span widths mirror
/// [`crate::image::column_spans`].
fn sealed_operand(
    fact: crate::encoding::FactView<'_, '_>,
    field: FieldId,
) -> crate::exec::dispatch::FactOperand {
    use crate::encoding::{field_bytes, interval_words};
    use crate::exec::dispatch::FactOperand;

    let idx = usize::from(field.0);
    let ty = fact.layout().field_type(idx);
    let bytes = field_bytes(fact, idx);
    match ty {
        ValueType::Bool => FactOperand::Word(u64::from(bytes[0])),
        ValueType::U64 | ValueType::I64 | ValueType::F64 | ValueType::String => FactOperand::Word(
            u64::from_be_bytes(bytes.try_into().expect("word field: layout-derived width")),
        ),
        ValueType::Id128 => {
            let mut words = [0u64; 8];
            words[0] = u64::from_be_bytes(bytes[..8].try_into().expect("id128 is sixteen bytes"));
            words[1] = u64::from_be_bytes(bytes[8..].try_into().expect("id128 is sixteen bytes"));
            FactOperand::Block { words, count: 2 }
        }
        ValueType::FixedBytes { .. } => {
            let count = bytes.len() / 8;
            debug_assert_eq!(bytes.len() % 8, 0, "padded bytes<N> is whole words");
            if count == 1 {
                FactOperand::Word(u64::from_be_bytes(
                    bytes.try_into().expect("one padded word"),
                ))
            } else {
                let mut words = [0u64; 8];
                let (chunks, rest) = bytes.as_chunks::<8>();
                debug_assert!(rest.is_empty(), "padded bytes<N> is whole words");
                for (word, chunk) in words.iter_mut().zip(chunks) {
                    *word = u64::from_be_bytes(*chunk);
                }
                FactOperand::Block {
                    words,
                    count: u8::try_from(count).expect("bytes width is at most 8 words"),
                }
            }
        }
        ValueType::Interval { .. } | ValueType::FixedInterval { .. } => {
            let (start, end) =
                interval_words(ty, bytes).expect("sealed rows are validated interval encodings");
            FactOperand::Pair(start, end)
        }
    }
}

/// Closed-row σ. String columns compare through [`TextEq`]; a resolver
/// `Err` is not a dropped id.
pub(crate) fn surviving_ids(
    relation: &Relation,
    filters: &[FilterPredicate],
    text: TextEq<'_>,
) -> Result<Vec<u64>, Error> {
    let layout = relation.layout();
    let mut ids = Vec::new();
    for (id, row) in relation
        .body()
        .closed_rows()
        .expect("callers checked closedness")
        .iter()
        .enumerate()
    {
        let ops = SealedRow {
            fact: layout.encoded(&row.fact),
        };
        if sealed_row_survives(&ops, filters, text)? {
            ids.push(id as u64);
        }
    }
    Ok(ids)
}

fn sealed_row_survives(
    ops: &SealedRow<'_>,
    filters: &[FilterPredicate],
    text: TextEq<'_>,
) -> Result<bool, Error> {
    for filter in filters {
        match crate::image::view::holds(filter, ops, &[], text)? {
            Some(true) => {}
            Some(false) | None => return Ok(false),
        }
    }
    Ok(true)
}

fn closed_layout_has_string(relation: &Relation) -> bool {
    let layout = relation.layout();
    (0..layout.field_count()).any(|idx| matches!(layout.field_type(idx), ValueType::String))
}

/// This fold has no image/generation. Numeric closed rows never consult
/// `TextEq`; String columns must not take a dummy resolver (raw word
/// compare / `Ok(false)`).
fn fold_surviving_ids(relation: &Relation, filters: &[FilterPredicate]) -> Option<Vec<u64>> {
    if closed_layout_has_string(relation) {
        return None;
    }
    let generation = GenerationHandle::new(GenerationState::new(
        crate::image::CacheGeneration::initial(),
        CacheLedger::unbounded(),
    ));
    match surviving_ids(relation, filters, generation.text_eq(None)) {
        Ok(ids) => Some(ids),
        Err(_) => None,
    }
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
                value: Const::WordSet(ids.to_vec()),
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
#[cfg(test)]
mod text_eq;
