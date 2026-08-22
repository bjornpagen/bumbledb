//! The commit plan: every
//! derivable key byte and check set of one commit, computed as a **pure
//! representation over control flow applied to the write path. Per fact:
//! its determinant bytes per key statement (pointwise keys marked for the
//! ordered-neighbor probe) and its reverse-edge key bytes per containment
//! `R` put/delete and the insert's source probe. Aggregated: the
//! function of (delta, schema)** before a single LMDB page is touched —

use std::collections::{BTreeMap, BTreeSet};

use crate::schema::{
    AxiomIndex, CapacityEnforcement, CapacityId, ContainmentId, Enforcement, KeyForm, KeyId,
    Schema, ValueType,
};
use crate::storage::delta::WriteDelta;
use crate::storage::keys::{self, DeterminantImage};
use bumbledb_theory::schema::{RelationId, StatementId};

use super::judgment::{SelectionCheck, Selections, capacity_child_image, child_weight, satisfies};
use crate::error::{Check, Direction, Result, Violation};

pub(crate) struct CommitPlan<'d> {
    pub(crate) selections: Selections<'d>,

    pub(crate) deletes: Box<[DeleteOp<'d>]>,

    pub(crate) inserts: Box<[InsertOp<'d>]>,

    inserted: Box<[(RelationId, &'d [u8])]>,

    pub(crate) target_checks: Box<[DeterminantCheck]>,

    /// (`lean/Bumbledb/Txn/DeltaRestriction.lean: touchedParents`) — one
    pub(crate) capacity_checks: Box<[CapacityCheck]>,
}

impl CommitPlan<'_> {
    pub(crate) fn incremental_obligations(&self) -> IncrementalObligations<'_, '_> {
        IncrementalObligations { plan: self }
    }

    pub(crate) fn inserts_fact(&self, relation: RelationId, fact: &[u8]) -> bool {
        self.inserted
            .binary_search_by(|&(rel, bytes)| (rel, bytes).cmp(&(relation, fact)))
            .is_ok()
    }
}

pub(crate) struct IncrementalObligations<'p, 'd> {
    plan: &'p CommitPlan<'d>,
}

impl<'p, 'd> IncrementalObligations<'p, 'd> {
    pub(crate) fn source_edges(
        &self,
    ) -> impl Iterator<Item = (ContainmentId, &'p RKeyOp<MarkWeight>, &'d [u8])> {
        self.plan.inserts.iter().flat_map(|op| {
            op.r_keys.iter().filter_map(move |edge| {
                edge.containment()
                    .map(|containment| (containment, edge, op.core.fact))
            })
        })
    }

    pub(crate) fn memberships(&self) -> impl Iterator<Item = &'p MembershipOp> {
        self.plan
            .inserts
            .iter()
            .flat_map(|op| op.memberships.iter())
    }

    pub(crate) fn target_checks(&self) -> &'p [DeterminantCheck] {
        &self.plan.target_checks
    }

    pub(crate) fn capacity_checks(&self) -> &'p [CapacityCheck] {
        &self.plan.capacity_checks
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn is_empty(&self) -> bool {
        self.plan
            .inserts
            .iter()
            .all(|op| op.containment_r_keys().next().is_none() && op.memberships.is_empty())
            && self.plan.target_checks.is_empty()
            && self.plan.capacity_checks.is_empty()
    }
}

pub(crate) struct CapacityCheck {
    pub(crate) capacity: CapacityId,

    pub(crate) parent: DeterminantImage,
}

pub(crate) struct FactCore<'d> {
    pub(crate) relation: RelationId,
    pub(crate) fact: &'d [u8],
    pub(crate) fact_hash: &'d [u8; 32],
    pub(crate) determinants: Box<[DeterminantOp]>,
}

pub(crate) struct DeleteOp<'d> {
    pub(crate) core: FactCore<'d>,
    pub(crate) r_keys: Box<[RKeyOp]>,
}

pub(crate) struct InsertOp<'d> {
    pub(crate) core: FactCore<'d>,
    pub(crate) fresh_row: Option<FreshRowOp>,
    pub(crate) r_keys: Box<[RKeyOp<MarkWeight>]>,
    pub(crate) memberships: Box<[MembershipOp]>,
}

impl<'d> DeleteOp<'d> {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn relation(&self) -> RelationId {
        self.core.relation
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn fact(&self) -> &'d [u8] {
        self.core.fact
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn fact_hash(&self) -> &'d [u8; 32] {
        self.core.fact_hash
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn determinants(&self) -> &[DeterminantOp] {
        &self.core.determinants
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn containment_r_keys(&self) -> impl Iterator<Item = &RKeyOp> {
        self.r_keys
            .iter()
            .filter(|edge| edge.containment().is_some())
    }
}

impl<'d> InsertOp<'d> {
    pub(crate) fn relation(&self) -> RelationId {
        self.core.relation
    }

    pub(crate) fn fact(&self) -> &'d [u8] {
        self.core.fact
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn fact_hash(&self) -> &'d [u8; 32] {
        self.core.fact_hash
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn determinants(&self) -> &[DeterminantOp] {
        &self.core.determinants
    }

    pub(crate) fn containment_r_keys(&self) -> impl Iterator<Item = &RKeyOp<MarkWeight>> {
        self.r_keys
            .iter()
            .filter(|edge| edge.containment().is_some())
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn capacity_r_keys(&self) -> impl Iterator<Item = &RKeyOp<MarkWeight>> {
        self.r_keys.iter().filter(|edge| edge.capacity().is_some())
    }
}

#[derive(Clone, Copy)]
pub(crate) enum RKeyKind {
    Containment(ContainmentId),
    Capacity(CapacityId),
}

pub(crate) struct RKeyOp<W = ()> {
    pub(crate) kind: RKeyKind,
    pub(crate) key_bytes: DeterminantImage,
    pub(crate) weight: W,
}

impl<W> RKeyOp<W> {
    pub(crate) fn containment(&self) -> Option<ContainmentId> {
        match self.kind {
            RKeyKind::Containment(id) => Some(id),
            RKeyKind::Capacity(_) => None,
        }
    }

    pub(crate) fn capacity(&self) -> Option<CapacityId> {
        match self.kind {
            RKeyKind::Capacity(id) => Some(id),
            RKeyKind::Containment(_) => None,
        }
    }

    pub(crate) fn statement_id(&self, schema: &Schema) -> StatementId {
        match self.kind {
            RKeyKind::Containment(id) => schema.containment(id).id,
            RKeyKind::Capacity(id) => schema.capacity(id).id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarkWeight {
    Unit,
    Weighted(u64),
}

#[derive(Clone, Copy)]
pub(crate) struct FreshRowOp {
    pub(crate) statement: StatementId,

    pub(crate) row_id: u64,
}

pub(crate) enum DeterminantOp {
    Scalar {
        statement: StatementId,
        determinant: DeterminantImage,
    },

    Pointwise {
        statement: StatementId,
        determinant: DeterminantImage,
        tail: ValueType,
    },
}

impl DeterminantOp {
    pub(crate) fn statement(&self) -> StatementId {
        match *self {
            Self::Scalar { statement, .. } | Self::Pointwise { statement, .. } => statement,
        }
    }

    pub(crate) fn determinant(&self) -> &DeterminantImage {
        match self {
            Self::Scalar { determinant, .. } | Self::Pointwise { determinant, .. } => determinant,
        }
    }
}

pub(crate) struct MembershipOp {
    pub(crate) check: Check,
}

pub(crate) struct DeterminantCheck {
    pub(crate) key: KeyId,

    pub(crate) determinant: DeterminantImage,

    pub(crate) dependents: Box<[DependentCheck]>,
}

pub(crate) struct DependentCheck {
    pub(crate) containment: ContainmentId,

    pub(crate) owed: Owed,
}

pub(crate) enum Owed {
    Unconditional,
    IfEstablisherFails,
}

/// # Errors
/// The one fallible slice is the weighted edge's weight derivation
/// ([`MarkWeight`] on an insert [`RKeyOp`]), INSERT ops only: a ray-valued
/// Duration weight has no finite u64 for the value slot, so it refuses typed at
/// plan time (C20, ruled 2026-08-03: the write-time refusal is doctrine — see
/// the judgment constraint comment).
pub(crate) fn plan_commit<'d>(
    delta: &'d WriteDelta<'_>,
    selections: Selections<'_>,
) -> Result<CommitPlan<'d>> {
    let selections = selections.bind(delta.schema());
    let schema = selections.schema();

    let mut deleted_determinants: BTreeSet<(KeyId, DeterminantImage)> = BTreeSet::new();
    let mut inserted_determinants: BTreeSet<(KeyId, DeterminantImage)> = BTreeSet::new();

    // (`lean/Bumbledb/Txn/DeltaRestriction.lean`): every parent key tuple

    let mut touched_parents: BTreeMap<CapacityId, BTreeSet<DeterminantImage>> = BTreeMap::new();
    let mut image = DeterminantImage::scratch();
    let mut delete_scratch = DeleteScratch::default();
    let mut insert_scratch = InsertScratch::default();

    let mut delete_facts: Vec<(RelationId, &[u8; 32], &[u8])> = delta.deletes().collect();
    delete_facts.sort_unstable_by(|(a_rel, a_hash, _), (b_rel, b_hash, _)| {
        (a_rel, a_hash).cmp(&(b_rel, b_hash))
    });
    let mut deletes = Vec::with_capacity(delete_facts.len());
    for (rel, hash, fact) in delete_facts {
        deletes.push(delete_op(
            schema,
            &selections,
            rel,
            hash,
            fact,
            &mut deleted_determinants,
            &mut touched_parents,
            &mut image,
            &mut delete_scratch,
        ));
    }
    let deletes = deletes.into_boxed_slice();
    let mut insert_facts: Vec<(RelationId, &[u8; 32], &[u8])> = delta.inserts().collect();
    insert_facts.sort_unstable_by(|(a_rel, a_hash, _), (b_rel, b_hash, _)| {
        (a_rel, a_hash).cmp(&(b_rel, b_hash))
    });
    let mut inserts = Vec::with_capacity(insert_facts.len());
    for (rel, hash, fact) in insert_facts {
        inserts.push(insert_op(
            schema,
            &selections,
            rel,
            hash,
            fact,
            &mut inserted_determinants,
            &mut touched_parents,
            &mut image,
            &mut insert_scratch,
        )?);
    }
    let inserts = inserts.into_boxed_slice();
    let mut inserted: Vec<(RelationId, &[u8])> = Vec::with_capacity(inserts.len());
    inserted.extend(inserts.iter().map(|op| (op.relation(), op.fact())));
    inserted.sort_unstable();
    let target_checks = target_checks(
        schema,
        &selections,
        deleted_determinants,
        &inserted_determinants,
    );
    let mut capacity_checks =
        Vec::with_capacity(touched_parents.values().map(BTreeSet::len).sum::<usize>());
    capacity_checks.extend(touched_parents.into_iter().flat_map(|(capacity, parents)| {
        parents
            .into_iter()
            .map(move |parent| CapacityCheck { capacity, parent })
    }));
    Ok(CommitPlan {
        selections,
        deletes,
        inserts,
        inserted: inserted.into_boxed_slice(),
        target_checks,
        capacity_checks: capacity_checks.into_boxed_slice(),
    })
}

#[derive(Default)]
struct DeleteScratch {
    r_keys: Vec<RKeyOp>,
}

#[derive(Default)]
struct InsertScratch {
    r_keys: Vec<RKeyOp<MarkWeight>>,
    memberships: Vec<MembershipOp>,
}

#[expect(
    clippy::too_many_arguments,
    reason = "the one per-delete derivation chokepoint; every input is load-bearing"
)]
fn delete_op<'d>(
    schema: &Schema,
    selections: &Selections<'_>,
    rel: RelationId,
    hash: &'d [u8; 32],
    fact: &'d [u8],
    dependent_determinants: &mut BTreeSet<(KeyId, DeterminantImage)>,
    touched_parents: &mut BTreeMap<CapacityId, BTreeSet<DeterminantImage>>,
    image: &mut DeterminantImage,
    scratch: &mut DeleteScratch,
) -> DeleteOp<'d> {
    keys::debug_assert_ordinary(schema, rel);
    let relation = schema.relation(rel);
    let layout = relation.layout();
    let determinants =
        derive_determinants(schema, relation, fact, false, image, dependent_determinants).0;
    for &containment_id in relation.outgoing() {
        let statement = schema.containment(containment_id);
        if !satisfies(&selections.containment(containment_id).source, layout, fact) {
            continue;
        }
        match &statement.enforcement {
            Enforcement::ScalarProbe { key_projection, .. }
            | Enforcement::IntervalCoverage { key_projection, .. } => {
                keys::determinant_image(layout.encoded(fact), key_projection, image);
                scratch.r_keys.push(RKeyOp {
                    kind: RKeyKind::Containment(containment_id),
                    key_bytes: image.clone(),
                    weight: (),
                });
            }
            Enforcement::Closed { .. } => {}
        }
    }
    mark_delete(
        schema,
        selections,
        relation,
        fact,
        touched_parents,
        image,
        scratch,
    );
    DeleteOp {
        core: FactCore {
            relation: rel,
            fact,
            fact_hash: hash,
            determinants,
        },
        r_keys: scratch.r_keys.drain(..).collect(),
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the one per-insert derivation chokepoint; every input is load-bearing"
)]
fn insert_op<'d>(
    schema: &Schema,
    selections: &Selections<'_>,
    rel: RelationId,
    hash: &'d [u8; 32],
    fact: &'d [u8],
    dependent_determinants: &mut BTreeSet<(KeyId, DeterminantImage)>,
    touched_parents: &mut BTreeMap<CapacityId, BTreeSet<DeterminantImage>>,
    image: &mut DeterminantImage,
    scratch: &mut InsertScratch,
) -> Result<InsertOp<'d>> {
    keys::debug_assert_ordinary(schema, rel);
    let relation = schema.relation(rel);
    let layout = relation.layout();
    let (determinants, fresh_row) =
        derive_determinants(schema, relation, fact, true, image, dependent_determinants);
    for &containment_id in relation.outgoing() {
        let statement = schema.containment(containment_id);
        if !satisfies(&selections.containment(containment_id).source, layout, fact) {
            continue;
        }
        match &statement.enforcement {
            Enforcement::ScalarProbe { key_projection, .. }
            | Enforcement::IntervalCoverage { key_projection, .. } => {
                keys::determinant_image(layout.encoded(fact), key_projection, image);
                scratch.r_keys.push(RKeyOp {
                    kind: RKeyKind::Containment(containment_id),
                    key_bytes: image.clone(),
                    weight: MarkWeight::Unit,
                });
            }
            Enforcement::Closed { members } => {
                let word = crate::encoding::field_word_bytes(
                    layout.encoded(fact),
                    usize::from(statement.source.projection[0].0),
                );
                let axiom = AxiomIndex::try_from(u64::from_be_bytes(word)).ok();
                let check = if axiom.is_some_and(|index| members.contains(index)) {
                    Check::Holds
                } else {
                    Check::Violated(Violation::containment(
                        crate::schema::StatementRef::Containment(containment_id),
                        Direction::SourceUnsatisfied,
                        fact.into(),
                    ))
                };
                scratch.memberships.push(MembershipOp { check });
            }
        }
    }
    mark_insert(
        schema,
        selections,
        relation,
        fact,
        touched_parents,
        image,
        scratch,
    )?;
    Ok(InsertOp {
        core: FactCore {
            relation: rel,
            fact,
            fact_hash: hash,
            determinants,
        },
        fresh_row,
        r_keys: scratch.r_keys.drain(..).collect(),
        memberships: scratch.memberships.drain(..).collect(),
    })
}

fn derive_determinants(
    schema: &Schema,
    relation: &crate::schema::Relation,
    fact: &[u8],
    insert: bool,
    image: &mut DeterminantImage,
    dependent_determinants: &mut BTreeSet<(KeyId, DeterminantImage)>,
) -> (Box<[DeterminantOp]>, Option<FreshRowOp>) {
    let layout = relation.layout();
    let mut fresh_row = None;
    let mut determinants = Vec::with_capacity(relation.keys().len());
    for &key_id in relation.keys() {
        let statement = schema.key(key_id);
        keys::determinant_image(layout.encoded(fact), &statement.projection, image);
        if !schema.dependents(key_id).is_empty() {
            dependent_determinants.insert((key_id, image.clone()));
        }
        match statement.form() {
            KeyForm::FreshRow { field } => {
                if insert {
                    let word = crate::encoding::field_word_bytes(
                        layout.encoded(fact),
                        usize::from(field.0),
                    );
                    fresh_row = Some(FreshRowOp {
                        statement: statement.id,
                        row_id: u64::from_be_bytes(word),
                    });
                }
            }
            KeyForm::Scalar => determinants.push(DeterminantOp::Scalar {
                statement: statement.id,
                determinant: image.clone(),
            }),
            KeyForm::Pointwise { tail, .. } => determinants.push(DeterminantOp::Pointwise {
                statement: statement.id,
                determinant: image.clone(),
                tail: *tail,
            }),
        }
    }
    (determinants.into_boxed_slice(), fresh_row)
}

fn mark_delete(
    schema: &Schema,
    selections: &Selections<'_>,
    relation: &crate::schema::Relation,
    fact: &[u8],
    touched_parents: &mut BTreeMap<CapacityId, BTreeSet<DeterminantImage>>,
    image: &mut DeterminantImage,
    scratch: &mut DeleteScratch,
) {
    let layout = relation.layout();
    for &capacity_id in relation.capacity_sources() {
        let statement = schema.capacity(capacity_id);
        capacity_child_image(statement, layout, fact, image);
        touched_parents
            .entry(capacity_id)
            .or_default()
            .insert(image.clone());
        if satisfies(&selections.capacity(capacity_id).source, layout, fact) {
            scratch.r_keys.push(RKeyOp {
                kind: RKeyKind::Capacity(capacity_id),
                key_bytes: image.clone(),
                weight: (),
            });
        }
    }
    touch_capacity_targets(schema, selections, relation, fact, touched_parents, image);
}

fn mark_insert(
    schema: &Schema,
    selections: &Selections<'_>,
    relation: &crate::schema::Relation,
    fact: &[u8],
    touched_parents: &mut BTreeMap<CapacityId, BTreeSet<DeterminantImage>>,
    image: &mut DeterminantImage,
    scratch: &mut InsertScratch,
) -> Result<()> {
    let layout = relation.layout();
    for &capacity_id in relation.capacity_sources() {
        let statement = schema.capacity(capacity_id);
        capacity_child_image(statement, layout, fact, image);
        touched_parents
            .entry(capacity_id)
            .or_default()
            .insert(image.clone());
        if satisfies(&selections.capacity(capacity_id).source, layout, fact) {
            let weight = match statement.weight {
                crate::schema::SealedWeight::Unit => MarkWeight::Unit,
                crate::schema::SealedWeight::Field(_)
                | crate::schema::SealedWeight::Duration { .. } => {
                    MarkWeight::Weighted(child_weight(statement, layout, fact)?)
                }
            };
            scratch.r_keys.push(RKeyOp {
                kind: RKeyKind::Capacity(capacity_id),
                key_bytes: image.clone(),
                weight,
            });
        }
    }
    touch_capacity_targets(schema, selections, relation, fact, touched_parents, image);
    Ok(())
}

fn touch_capacity_targets(
    schema: &Schema,
    selections: &Selections<'_>,
    relation: &crate::schema::Relation,
    fact: &[u8],
    touched_parents: &mut BTreeMap<CapacityId, BTreeSet<DeterminantImage>>,
    image: &mut DeterminantImage,
) {
    let layout = relation.layout();
    for &capacity_id in relation.capacity_targets() {
        let statement = schema.capacity(capacity_id);
        if let CapacityEnforcement::ScalarProbe { target_key, .. } = &statement.enforcement
            && satisfies(&selections.capacity(capacity_id).target, layout, fact)
        {
            let key_statement = schema.key(*target_key);
            keys::determinant_image(layout.encoded(fact), &key_statement.projection, image);
            touched_parents
                .entry(capacity_id)
                .or_default()
                .insert(image.clone());
        }
    }
}

fn target_checks(
    schema: &Schema,
    selections: &Selections<'_>,
    deleted_determinants: BTreeSet<(KeyId, DeterminantImage)>,
    inserted_determinants: &BTreeSet<(KeyId, DeterminantImage)>,
) -> Box<[DeterminantCheck]> {
    let mut checks = Vec::with_capacity(deleted_determinants.len());
    let mut dependents: Vec<DependentCheck> = Vec::new();
    for entry in deleted_determinants {
        let reestablished = inserted_determinants.contains(&entry);
        let (key, determinant) = entry;
        for &containment_id in schema.dependents(key) {
            let statement = schema.containment(containment_id);
            if matches!(statement.enforcement, Enforcement::Closed { .. }) {
                continue;
            }
            let owed = if reestablished {
                match &selections.containment(containment_id).target {
                    SelectionCheck::Empty => continue,
                    SelectionCheck::Never => Owed::Unconditional,
                    SelectionCheck::Compare(_) => Owed::IfEstablisherFails,
                }
            } else {
                Owed::Unconditional
            };
            dependents.push(DependentCheck {
                containment: containment_id,
                owed,
            });
        }
        if dependents.is_empty() {
            continue;
        }
        checks.push(DeterminantCheck {
            key,
            determinant,
            dependents: dependents.drain(..).collect(),
        });
    }
    checks.into_boxed_slice()
}
