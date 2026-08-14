//! The commit plan (`docs/architecture/50-storage.md` § Write path): every
//! derivable key byte and check set of one commit, computed as a **pure
//! function of (delta, schema)** before a single LMDB page is touched —
//! representation over control flow applied to the write path. Per fact:
//! its determinant bytes per key statement (pointwise keys marked for the
//! ordered-neighbor probe) and its reverse-edge key bytes per containment
//! whose source selection it satisfies — the same permuted bytes serve the
//! `R` put/delete and the insert's source probe. Aggregated: the
//! per-statement disestablished-determinant check sets (deleted − inserted,
//! with ψ-qualified re-establishment inputs marked for the judgment
//! phase). Selection literals arrive pre-encoded ([`Selections`]) and the
//! plan owns them for the rest of the commit.
//!
//! The honest boundary, stated up front: delete row ids are **not**
//! derivable (they need the `M` lookup), fresh-less insert row ids mint
//! from the high-water, and judgment probe *results* need final-state
//! reads. On a **fresh-keyed** relation the insert row id IS derivable —
//! the first fresh field's value is the `F` row id (the one id
//! allocator, `docs/architecture/50-storage.md` § key layout; R16) — so
//! the plan carries it ([`FactOp::fresh_row`]). The plan owns key
//! material and check sets; the applier keeps the remaining id plumbing
//! and the desync probes; the judgment keeps the final-state probes.

use std::collections::{BTreeMap, BTreeSet};

use crate::schema::{
    AxiomIndex, CapacityEnforcement, CapacityId, ContainmentId, Enforcement, IntervalTail, KeyId,
    Schema,
};
use crate::storage::delta::{Disposition, WriteDelta};
use crate::storage::keys::{self, DeterminantImage};
use bumbledb_theory::schema::{RelationId, StatementId};

use super::judgment::{SelectionCheck, Selections, capacity_child_image, child_weight, satisfies};
use crate::error::Result;

/// One commit's derivable bookkeeping, borrowed from the delta's arena.
pub(crate) struct CommitPlan<'d> {
    /// Selection literals pre-encoded once for this commit (the plan
    /// derivation gates the reverse edges with them; the judgment phase
    /// reuses them for its source and target checks).
    pub(crate) selections: Selections,
    /// Phase-1 ops, in the delta's deterministic `(relation, fact_hash)`
    /// order.
    pub(crate) deletes: Box<[FactOp<'d>]>,
    /// Phase-2 ops, same order.
    pub(crate) inserts: Box<[FactOp<'d>]>,
    /// The insert set re-sorted by `(relation, fact bytes)` — the
    /// target-side judgment's survivor-partition membership test
    /// ([`Self::inserts_fact`]). Its own index because the ops sit in
    /// `(relation, fact_hash)` order, which is NOT byte order.
    inserted: Box<[(RelationId, &'d [u8])]>,
    /// Phase-3 target-side check set: one entry per key tuple this commit
    /// disestablishes for at least one dependent statement.
    pub(crate) target_checks: Box<[DeterminantCheck]>,
    /// Phase-3 capacity check set: the TOUCHED PARENTS
    /// (`lean/Bumbledb/Txn/DeltaRestriction.lean: touchedParents`) — one
    /// entry per (capacity statement, parent key tuple) this delta may
    /// have moved, deduplicated, in scan order.
    pub(crate) capacity_checks: Box<[CapacityCheck]>,
}

impl CommitPlan<'_> {
    /// Whether this commit inserts `fact` into `relation` — canonical
    /// bytes, identity = bytes (`10-data-model.md`). Binary search over
    /// the byte-sorted insert index: no per-judgment set is built, and
    /// the plan stays immutable for `commit_bounded`'s re-runs.
    pub(crate) fn inserts_fact(&self, relation: RelationId, fact: &[u8]) -> bool {
        self.inserted
            .binary_search_by(|&(rel, bytes)| (rel, bytes).cmp(&(relation, fact)))
            .is_ok()
    }
}

/// One touched parent of one capacity statement — the judgment phase
/// probes the parent's ψ-selected holder, resolves its dependent bound,
/// and walks its child group's measure.
pub(crate) struct CapacityCheck {
    /// The validation-minted capacity witness.
    pub(crate) capacity: CapacityId,
    /// The parent key tuple, in target-key determinant order.
    pub(crate) parent: DeterminantImage,
}

/// Everything derivable about one fact's application.
pub(crate) struct FactOp<'d> {
    pub(crate) relation: RelationId,
    /// The canonical fact bytes (identity = bytes, `10-data-model.md`).
    pub(crate) fact: &'d [u8],
    /// The fact's blake3 hash, borrowed from the delta's map key — the
    /// delta computed it once to record the disposition, so the `M` key
    /// derivation at apply is free (the applier never re-hashes).
    pub(crate) fact_hash: &'d [u8; 32],
    /// The one id allocator's derivation on a fresh-keyed relation (R16):
    /// the first fresh field's value IS the `F` row id, and the named
    /// statement's functionality judgment is the `F` put-conflict.
    /// `None` = fresh-less; the applier mints from the `S` high-water.
    pub(crate) fresh_row: Option<FreshRowOp>,
    /// One per key statement of the relation, materialized order — the
    /// fresh-row auto-key excepted: it maintains no `U` tree (its entry
    /// would transcribe `F`), so no determinant op exists for it.
    pub(crate) determinants: Box<[DeterminantOp]>,
    /// One per outgoing containment whose source selection the fact
    /// satisfies — a fact outside σ has no edge, by design.
    pub(crate) edges: Box<[EdgeOp]>,
    /// One per outgoing **closed-target** containment whose source
    /// selection the fact satisfies: no determinant bytes, no `R` traffic —
    /// the compiled member set is the whole plan, and the judgment is
    /// one AND and one test on the insert side
    /// (`docs/architecture/30-dependencies.md`). Dead weight on a
    /// delete op (removing a reference cannot violate an inclusion);
    /// only the insert-side judgment consumes it.
    pub(crate) memberships: Box<[MembershipOp]>,
    /// One per capacity statement whose source (child) is this relation
    /// and whose φ the fact satisfies — the capacity `R` edge, written
    /// exactly as a containment edge (`docs/architecture/50-storage.md`
    /// § key layout: the child-group measure walk's reader).
    pub(crate) capacity_edges: Box<[MarkEdgeOp]>,
}

/// One capacity `R` edge of one fact: the statement-scoped key material,
/// KEY-symmetric between the insert put and the delete removal (the
/// applier consumes it exactly as a containment [`EdgeOp`]; the delete
/// removal is key-only, so a delete op's `weight` is `None` by
/// construction — never derived: the derive is fallible on a weighted
/// statement, and a value the applier never reads must not be able to
/// refuse the delete).
pub(crate) struct MarkEdgeOp {
    /// Prederived statement identity for the schema-free byte applier.
    pub(crate) statement: StatementId,
    /// The edge's key-bytes segment: the capacity statement's child
    /// projection in target-key determinant order.
    pub(crate) key_bytes: DeterminantImage,
    /// The insert put's value slot: a weighted statement's child weight
    /// (the C17 slot law), sliced from the source fact at plan time —
    /// the plan stays pure, zero LMDB reads. `None` = the empty value
    /// (unit edges).
    pub(crate) weight: Option<u64>,
}

/// The fresh-row derivation of one fact op (R16): the row id sliced from
/// the fact's first fresh field, and the auto-key statement the `F`
/// put-conflict convicts.
pub(crate) struct FreshRowOp {
    /// The fresh auto-key's fingerprint-pinned identity.
    pub(crate) statement: StatementId,
    /// The first fresh field's value — the `F` row id.
    pub(crate) row_id: u64,
}

/// One key statement's determinant material for one fact.
pub(crate) enum DeterminantOp {
    /// Scalar key: exact `U` put is the functionality judgment.
    Scalar {
        statement: StatementId,
        determinant: DeterminantImage,
    },
    /// Interval-carrying key: the exact `U` put cannot detect overlap, so
    /// the insert additionally runs the ordered-neighbor probe — the
    /// tail descriptor says how the determinant's trailing interval
    /// reads (16-byte `start ‖ end`, or the 8-byte fixed start whose end
    /// is the type's width).
    Pointwise {
        statement: StatementId,
        determinant: DeterminantImage,
        tail: IntervalTail,
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

    pub(crate) fn tail(&self) -> Option<IntervalTail> {
        match *self {
            Self::Pointwise { tail, .. } => Some(tail),
            Self::Scalar { .. } => None,
        }
    }
}

/// One closed-target containment of one fact: the membership judgment's
/// whole input. The id is the referencing field's decoded word — already
/// in hand during the derivation, never re-sliced at judgment.
pub(crate) struct MembershipOp {
    /// The validation-minted containment witness; the fingerprint identity
    /// is derived only when constructing an error.
    pub(crate) containment: ContainmentId,
    /// The referencing field narrowed to the closed extension's index
    /// domain. `None` is an out-of-range value and therefore a miss.
    pub(crate) axiom: Option<AxiomIndex>,
}

/// One containment edge of one fact: the `R` key material and, on the
/// insert side, the source-probe input.
pub(crate) struct EdgeOp {
    /// The typed containment supplies target relation, target key, and
    /// scalar-versus-interval enforcement at judgment.
    pub(crate) containment: ContainmentId,
    /// Prederived statement identity for the schema-free byte applier.
    pub(crate) statement: StatementId,
    /// The source projection laid down in the target key's determinant order
    /// ([`keys::permuted_determinant_image`]) — the `R` key-bytes segment and
    /// the source probe's target determinant value.
    pub(crate) key_bytes: DeterminantImage,
}

/// One disestablished key tuple and the dependent statements that must
/// re-check it (`deleted − inserted`, per statement).
pub(crate) struct DeterminantCheck {
    /// The key (`Functionality`) statement whose tuple left.
    pub(crate) key: KeyId,
    /// The tuple's determinant bytes (interval keys carry the 16-byte tail).
    pub(crate) determinant: DeterminantImage,
    /// The dependent containments still owed a check, in materialized
    /// order — a dependent whose empty-ψ tuple re-lands in phase 2 is
    /// already dropped here.
    pub(crate) dependents: Box<[DependentCheck]>,
}

/// One dependent statement's entry in a [`DeterminantCheck`].
pub(crate) struct DependentCheck {
    /// The validation-minted containment witness.
    pub(crate) containment: ContainmentId,
    /// The tuple's exact bytes re-land in phase 2 and this dependent
    /// carries a ψ: the check applies only if the establishing fact fails
    /// ψ — the judgment fetches it (one `F` get, shared across the
    /// tuple's ψ-carrying dependents) and decides. `false` = check
    /// unconditionally: the tuple never re-lands, or ψ is `Never` (no
    /// fact can satisfy it, so re-landing cannot help).
    pub(crate) psi_qualified: bool,
}

/// Derives one commit's plan — pure over `(delta, schema, selections)`:
/// no LMDB, no transactions, only byte slicing through the canonical key
/// derivations and set arithmetic over the delta's net dispositions.
///
/// # Errors
///
/// The one fallible slice is the weighted edge's weight derivation
/// ([`MarkEdgeOp::weight`], the C17 slot law), INSERT ops only: a
/// ray-valued Duration weight has no finite u64 for the value slot, so
/// it refuses typed at plan time (C20, ruled 2026-08-03: the write-time
/// refusal is doctrine — see the judgment constraint comment). Delete
/// ops never derive — their removal is key-only.
pub(crate) fn plan_commit<'d>(
    delta: &'d WriteDelta<'_>,
    schema: &Schema,
    selections: Selections,
) -> Result<CommitPlan<'d>> {
    // Determinant tuples of key statements some containment depends on — the
    // inputs of the target-side check set (`deleted − inserted`).
    let mut deleted_determinants: BTreeSet<(KeyId, DeterminantImage)> = BTreeSet::new();
    let mut inserted_determinants: BTreeSet<(KeyId, DeterminantImage)> = BTreeSet::new();
    // The touched notion of the capacity form
    // (`lean/Bumbledb/Txn/DeltaRestriction.lean`): every parent key tuple
    // any delta child fact projects to plus the delta's ψ-selected
    // parents (`touchedParents`) — a set by construction, deduplicated
    // here.
    let mut touched_parents: BTreeMap<CapacityId, BTreeSet<DeterminantImage>> = BTreeMap::new();
    let mut scratch = FactScratch::default();
    // The ONE exact sort per disposition: the delta's hash table has no
    // iteration order, so the deterministic `(relation, fact_hash)`
    // commit order the 50-storage doc requires is restored here — the
    // sort-license precedent is the T8 probe sort (`judgment.rs`,
    // `check_source`). The key is a total order (hash equality is fact
    // equality), so the op order is deterministic whatever the sort
    // algorithm does with equal keys — there are none.
    let mut delete_facts: Vec<(RelationId, &[u8; 32], &[u8])> = delta.deletes().collect();
    delete_facts.sort_unstable_by(|(a_rel, a_hash, _), (b_rel, b_hash, _)| {
        (a_rel, a_hash).cmp(&(b_rel, b_hash))
    });
    let mut deletes = Vec::with_capacity(delete_facts.len());
    for (rel, hash, fact) in delete_facts {
        deletes.push(fact_op(
            schema,
            &selections,
            Disposition::Delete,
            rel,
            hash,
            fact,
            &mut deleted_determinants,
            &mut touched_parents,
            &mut scratch,
        )?);
    }
    let deletes = deletes.into_boxed_slice();
    let mut insert_facts: Vec<(RelationId, &[u8; 32], &[u8])> = delta.inserts().collect();
    insert_facts.sort_unstable_by(|(a_rel, a_hash, _), (b_rel, b_hash, _)| {
        (a_rel, a_hash).cmp(&(b_rel, b_hash))
    });
    let mut inserts = Vec::with_capacity(insert_facts.len());
    for (rel, hash, fact) in insert_facts {
        inserts.push(fact_op(
            schema,
            &selections,
            Disposition::Insert,
            rel,
            hash,
            fact,
            &mut inserted_determinants,
            &mut touched_parents,
            &mut scratch,
        )?);
    }
    let inserts = inserts.into_boxed_slice();
    let mut inserted: Vec<(RelationId, &[u8])> = Vec::with_capacity(inserts.len());
    inserted.extend(inserts.iter().map(|op| (op.relation, op.fact)));
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

/// Per-fact derivation scratch, hoisted to the commit ([`plan_commit`]):
/// the staging Vecs grow to the commit's high-water once and drain into
/// each op's exact-size boxes — no per-fact growth or shrink reallocs.
#[derive(Default)]
struct FactScratch {
    image: DeterminantImage,
    edges: Vec<EdgeOp>,
    memberships: Vec<MembershipOp>,
    capacity_edges: Vec<MarkEdgeOp>,
}

/// Derives one fact's op: determinant bytes per key statement, reverse-edge key
/// bytes per satisfied containment. Determinant tuples of dependent-carrying
/// key statements are recorded into `dependent_determinants` for the check-set
/// difference.
#[expect(
    clippy::too_many_arguments,
    reason = "the one per-fact derivation chokepoint; every input is load-bearing"
)]
fn fact_op<'d>(
    schema: &Schema,
    selections: &Selections,
    disposition: Disposition,
    rel: RelationId,
    hash: &'d [u8; 32],
    fact: &'d [u8],
    dependent_determinants: &mut BTreeSet<(KeyId, DeterminantImage)>,
    touched_parents: &mut BTreeMap<CapacityId, BTreeSet<DeterminantImage>>,
    scratch: &mut FactScratch,
) -> Result<FactOp<'d>> {
    // Every F/M/U/R key byte originates from this derivation — the
    // refusal-hardening chokepoint (`keys::debug_assert_ordinary`).
    keys::debug_assert_ordinary(schema, rel);
    let relation = schema.relation(rel);
    let layout = relation.layout();
    let mut fresh_row = None;
    let mut determinants = Vec::with_capacity(relation.keys().len());
    for &key_id in relation.keys() {
        let statement = schema.key(key_id);
        // Determinant keys derived by slicing projected fields out of
        // fact_bytes — never a scan; interval fields slice as their
        // whole 16 bytes.
        keys::determinant_image(layout, &statement.projection, fact, &mut scratch.image);
        if !schema.dependents(key_id).is_empty() {
            dependent_determinants.insert((key_id, scratch.image.clone()));
        }
        // The fresh-row auto-key maintains no `U` tree (the one id
        // allocator, R16): its determinant IS the `F` row id, and the
        // `F` put-conflict is its functionality judgment — the applier
        // takes the derived id and the statement to convict.
        if let Some(field) = statement.form().as_fresh_row() {
            let word = crate::encoding::field_word_bytes(fact, layout, usize::from(field.0));
            fresh_row = Some(FreshRowOp {
                statement: statement.id,
                row_id: u64::from_be_bytes(word),
            });
            continue;
        }
        determinants.push(match statement.form().as_pointwise() {
            Some(tail) => DeterminantOp::Pointwise {
                statement: statement.id,
                determinant: scratch.image.clone(),
                tail,
            },
            None => DeterminantOp::Scalar {
                statement: statement.id,
                determinant: scratch.image.clone(),
            },
        });
    }
    let determinants = determinants.into_boxed_slice();
    // One edge per outgoing containment statement whose source selection
    // the fact satisfies — conditional containments get reverse edges
    // only for facts inside their σ (docs/architecture/50-storage.md
    // § key layout). The same derivation serves the insert-phase put, the
    // delete-phase removal (byte-symmetric), and the source probe. A
    // closed-target containment derives no key material at all: the
    // referencing word is already in hand, and the compiled member set is
    // its entire enforcement plan.
    for &containment_id in relation.outgoing() {
        let statement = schema.containment(containment_id);
        if !satisfies(&selections.containment(containment_id).source, layout, fact) {
            continue;
        }
        match &statement.enforcement {
            Enforcement::ScalarProbe {
                key_permutation, ..
            }
            | Enforcement::IntervalCoverage {
                key_permutation, ..
            } => {
                keys::permuted_determinant_image(
                    layout,
                    &statement.source.projection,
                    key_permutation,
                    fact,
                    &mut scratch.image,
                );
                scratch.edges.push(EdgeOp {
                    containment: containment_id,
                    statement: statement.id,
                    key_bytes: scratch.image.clone(),
                });
            }
            Enforcement::Closed { .. } => {
                let word = crate::encoding::field_word_bytes(
                    fact,
                    layout,
                    usize::from(statement.source.projection[0].0),
                );
                scratch.memberships.push(MembershipOp {
                    containment: containment_id,
                    axiom: AxiomIndex::try_from(u64::from_be_bytes(word)).ok(),
                });
            }
        }
    }
    let capacity_edges = mark_ops(
        schema,
        selections,
        disposition,
        relation,
        fact,
        touched_parents,
        scratch,
    )?;
    Ok(FactOp {
        relation: rel,
        fact,
        fact_hash: hash,
        fresh_row,
        determinants,
        edges: scratch.edges.drain(..).collect(),
        memberships: scratch.memberships.drain(..).collect(),
        capacity_edges,
    })
}

/// One fact's capacity-form derivations: the capacity `R` edges (a
/// weighted statement's edge carrying its slot weight), plus the fact's contributions
/// to the TOUCHED notion (`lean/Bumbledb/Txn/DeltaRestriction.lean`).
/// Dependent bounds need no marking of their own: a target-row
/// bound-field update is remove+add, both halves derive the SAME key
/// tuple through the ψ-gated target half below, and the `BTreeSet`
/// dedupes — `touchedParents` already covers them (the non-obvious
/// reason "plan phase unchanged" survives dependent bounds).
fn mark_ops(
    schema: &Schema,
    selections: &Selections,
    disposition: Disposition,
    relation: &crate::schema::Relation,
    fact: &[u8],
    touched_parents: &mut BTreeMap<CapacityId, BTreeSet<DeterminantImage>>,
    scratch: &mut FactScratch,
) -> Result<Box<[MarkEdgeOp]>> {
    let layout = relation.layout();
    // Capacity edges and touched parents (`touchedParents`' two halves).
    // The source half is φ-BLIND: every delta child touches its parent
    // tuple, φ-satisfying or not — the model's superset narrowing (a
    // non-φ fact never changes a child group; wider touched only
    // re-checks more). The edge itself is φ-gated exactly as a
    // containment's, so the child-group measure walk sums σφ members only.
    for &capacity_id in relation.capacity_sources() {
        let statement = schema.capacity(capacity_id);
        capacity_child_image(statement, layout, fact, &mut scratch.image);
        touched_parents
            .entry(capacity_id)
            .or_default()
            .insert(scratch.image.clone());
        if satisfies(&selections.capacity(capacity_id).source, layout, fact) {
            // The value slot is an INSERT-side concern: the delete
            // removal is key-only, so a delete op never derives —
            // the derive is fallible on a weighted statement (a
            // ray-valued Duration weight refuses), and a value the
            // applier never reads must not be able to refuse a delete.
            let weight = if matches!(statement.weight, crate::schema::SealedWeight::Unit)
                || disposition == Disposition::Delete
            {
                None
            } else {
                Some(child_weight(statement, layout, fact)?)
            };
            scratch.capacity_edges.push(MarkEdgeOp {
                statement: statement.id,
                key_bytes: scratch.image.clone(),
                weight,
            });
        }
    }
    // The target half: a delta parent inside ψ touches its own key tuple
    // (a group newly constrained or released — a dependent-bound field
    // update rides this same half as remove+add). Closed parents never
    // reach a fact op (writes refused), so only the keyed arm exists here.
    for &capacity_id in relation.capacity_targets() {
        let statement = schema.capacity(capacity_id);
        if let CapacityEnforcement::ScalarProbe { target_key, .. } = &statement.enforcement
            && satisfies(&selections.capacity(capacity_id).target, layout, fact)
        {
            let key_statement = schema.key(*target_key);
            keys::determinant_image(layout, &key_statement.projection, fact, &mut scratch.image);
            touched_parents
                .entry(capacity_id)
                .or_default()
                .insert(scratch.image.clone());
        }
    }
    Ok(scratch.capacity_edges.drain(..).collect())
}

/// The target-side check set: every deleted determinant tuple, expanded per
/// dependent statement with **ψ-qualified re-establishment**
/// (`docs/architecture/50-storage.md` § commit step 3). A tuple whose
/// exact bytes re-land in phase 2 is re-established for an empty-ψ
/// dependent (the plain set difference — dropped here), stays owed for a
/// `Never`-ψ dependent (no establishing fact can satisfy ψ), and is
/// *conditionally* owed for a ψ-carrying dependent — marked for the
/// judgment phase, which alone can read the establishing fact.
fn target_checks(
    schema: &Schema,
    selections: &Selections,
    deleted_determinants: BTreeSet<(KeyId, DeterminantImage)>,
    inserted_determinants: &BTreeSet<(KeyId, DeterminantImage)>,
) -> Box<[DeterminantCheck]> {
    // Exact-capacity staging: the outer Vec never grows (every deleted
    // tuple is a candidate; `into_boxed_slice` shrinks at most once,
    // when a tuple drops whole), and the dependents scratch grows to the
    // widest dependent list once, draining into each check's exact box.
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
            let psi_qualified = if reestablished {
                match &selections.containment(containment_id).target {
                    SelectionCheck::Empty => continue,
                    SelectionCheck::Never => false,
                    SelectionCheck::Compare(_) => true,
                }
            } else {
                false
            };
            dependents.push(DependentCheck {
                containment: containment_id,
                psi_qualified,
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
