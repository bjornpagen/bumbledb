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
//! The honest boundary, stated up front: row ids are **not** derivable
//! (deletes need the `M` lookup; inserts mint from the high-water) and
//! judgment probe *results* need final-state reads. The plan owns key
//! material and check sets; the applier keeps the id plumbing and the
//! desync probes; the judgment keeps the final-state probes.

use std::collections::{BTreeMap, BTreeSet};

use crate::schema::{
    AxiomIndex, ContainmentId, Enforcement, IntervalTail, KeyId, RelationId, Schema, StatementId,
    WindowId,
};
use crate::storage::delta::WriteDelta;
use crate::storage::keys::{self, DeterminantImage};

use super::judgment::{SelectionCheck, Selections, satisfies, window_child_image};

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
    /// Phase-3 target-side check set: one entry per key tuple this commit
    /// disestablishes for at least one dependent statement.
    pub(crate) target_checks: Box<[DeterminantCheck]>,
    /// Phase-3 window check set: the TOUCHED PARENTS
    /// (`lean/Bumbledb/Txn/DeltaRestriction.lean: touchedParents`) — one
    /// entry per (window, parent key tuple) this delta may have moved,
    /// deduplicated, in scan order.
    pub(crate) window_checks: Box<[WindowCheck]>,
}

/// One touched parent of one window statement — the judgment phase probes
/// the parent's ψ-selected holder and walks its child group.
pub(crate) struct WindowCheck {
    /// The validation-minted window witness.
    pub(crate) window: WindowId,
    /// The parent key tuple, in target-key determinant order.
    pub(crate) parent: DeterminantImage,
}

/// Everything derivable about one fact's application.
pub(crate) struct FactOp<'d> {
    pub(crate) relation: RelationId,
    /// The canonical fact bytes (identity = bytes, `10-data-model.md`).
    pub(crate) fact: &'d [u8],
    /// One per key statement of the relation, materialized order.
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
    /// One per window statement whose source (child) is this relation and
    /// whose φ the fact satisfies — the window's `R` edge, written exactly
    /// as a containment edge (`docs/architecture/50-storage.md` § key
    /// layout: the child-group walk's reader).
    pub(crate) window_edges: Box<[MarkEdgeOp]>,
}

/// One window `R` edge of one fact: the statement-scoped key
/// material, byte-symmetric between the insert put and the delete removal
/// (the applier consumes it exactly as a containment [`EdgeOp`]).
pub(crate) struct MarkEdgeOp {
    /// Prederived statement identity for the schema-free byte applier.
    pub(crate) statement: StatementId,
    /// The edge's key-bytes segment: the window's child projection in
    /// target-key determinant order.
    pub(crate) key_bytes: DeterminantImage,
}

/// One key statement's determinant material for one fact.
pub(crate) struct DeterminantOp {
    /// The `Functionality` statement.
    pub(crate) statement: StatementId,
    /// The projected fields' canonical encodings in statement order
    /// ([`keys::determinant_image`]) — the `U` key's determinant segment.
    pub(crate) determinant: DeterminantImage,
    /// Interval-carrying key: the exact `U` put cannot detect overlap, so
    /// the insert additionally runs the ordered-neighbor probe — the
    /// tail descriptor says how the determinant's trailing interval
    /// reads (16-byte `start ‖ end`, or the 8-byte fixed start whose end
    /// is the type's width). `None` = scalar key.
    pub(crate) pointwise: Option<IntervalTail>,
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
pub(crate) fn plan_commit<'d>(
    delta: &'d WriteDelta<'_>,
    schema: &Schema,
    selections: Selections,
) -> CommitPlan<'d> {
    // Determinant tuples of key statements some containment depends on — the
    // inputs of the target-side check set (`deleted − inserted`).
    let mut deleted_determinants: BTreeSet<(KeyId, DeterminantImage)> = BTreeSet::new();
    let mut inserted_determinants: BTreeSet<(KeyId, DeterminantImage)> = BTreeSet::new();
    // The touched notion of the window form
    // (`lean/Bumbledb/Txn/DeltaRestriction.lean`): every parent key tuple
    // any delta child fact projects to plus the delta's ψ-selected
    // parents (`touchedParents`) — a set by construction, deduplicated
    // here.
    let mut touched_parents: BTreeMap<WindowId, BTreeSet<DeterminantImage>> = BTreeMap::new();
    let mut scratch = DeterminantImage::scratch();
    let deletes = delta
        .deletes()
        .map(|(rel, fact)| {
            fact_op(
                schema,
                &selections,
                rel,
                fact,
                &mut deleted_determinants,
                &mut touched_parents,
                &mut scratch,
            )
        })
        .collect();
    let inserts = delta
        .inserts()
        .map(|(rel, fact)| {
            fact_op(
                schema,
                &selections,
                rel,
                fact,
                &mut inserted_determinants,
                &mut touched_parents,
                &mut scratch,
            )
        })
        .collect();
    let target_checks = target_checks(
        schema,
        &selections,
        deleted_determinants,
        &inserted_determinants,
    );
    let window_checks = touched_parents
        .into_iter()
        .flat_map(|(window, parents)| {
            parents
                .into_iter()
                .map(move |parent| WindowCheck { window, parent })
        })
        .collect();
    CommitPlan {
        selections,
        deletes,
        inserts,
        target_checks,
        window_checks,
    }
}

/// Derives one fact's op: determinant bytes per key statement, reverse-edge key
/// bytes per satisfied containment. Determinant tuples of dependent-carrying
/// key statements are recorded into `dependent_determinants` for the check-set
/// difference.
fn fact_op<'d>(
    schema: &Schema,
    selections: &Selections,
    rel: RelationId,
    fact: &'d [u8],
    dependent_determinants: &mut BTreeSet<(KeyId, DeterminantImage)>,
    touched_parents: &mut BTreeMap<WindowId, BTreeSet<DeterminantImage>>,
    scratch: &mut DeterminantImage,
) -> FactOp<'d> {
    // Every F/M/U/R key byte originates from this derivation — the
    // refusal-hardening chokepoint (`keys::debug_assert_ordinary`).
    keys::debug_assert_ordinary(schema, rel);
    let relation = schema.relation(rel);
    let layout = relation.layout();
    let determinants = relation
        .keys()
        .iter()
        .map(|&key_id| {
            let statement = schema.key(key_id);
            // Determinant keys derived by slicing projected fields out of
            // fact_bytes — never a scan; interval fields slice as their
            // whole 16 bytes.
            keys::determinant_image(layout, &statement.projection, fact, scratch);
            let determinant = scratch.clone();
            if !schema.dependents(key_id).is_empty() {
                dependent_determinants.insert((key_id, determinant.clone()));
            }
            DeterminantOp {
                statement: statement.id,
                determinant,
                pointwise: statement.pointwise.then(|| {
                    schema
                        .key_tail(statement)
                        .expect("a pointwise key has a tail")
                }),
            }
        })
        .collect();
    // One edge per outgoing containment statement whose source selection
    // the fact satisfies — conditional containments get reverse edges
    // only for facts inside their σ (docs/architecture/50-storage.md
    // § key layout). The same derivation serves the insert-phase put, the
    // delete-phase removal (byte-symmetric), and the source probe. A
    // closed-target containment derives no key material at all: the
    // referencing word is already in hand, and the compiled member set is
    // its entire enforcement plan.
    let mut edges = Vec::new();
    let mut memberships = Vec::new();
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
                    scratch,
                );
                edges.push(EdgeOp {
                    containment: containment_id,
                    statement: statement.id,
                    key_bytes: scratch.clone(),
                });
            }
            Enforcement::Closed { .. } => {
                let word = crate::encoding::field_word_bytes(
                    fact,
                    layout,
                    usize::from(statement.source.projection[0].0),
                );
                memberships.push(MembershipOp {
                    containment: containment_id,
                    axiom: AxiomIndex::try_from(u64::from_be_bytes(word)).ok(),
                });
            }
        }
    }
    let window_edges = mark_ops(schema, selections, relation, fact, touched_parents, scratch);
    FactOp {
        relation: rel,
        fact,
        determinants,
        edges: edges.into_boxed_slice(),
        memberships: memberships.into_boxed_slice(),
        window_edges,
    }
}

/// One fact's window-form derivations: the window `R` edges, plus the
/// fact's contributions to the TOUCHED notion
/// (`lean/Bumbledb/Txn/DeltaRestriction.lean`).
fn mark_ops(
    schema: &Schema,
    selections: &Selections,
    relation: &crate::schema::Relation,
    fact: &[u8],
    touched_parents: &mut BTreeMap<WindowId, BTreeSet<DeterminantImage>>,
    scratch: &mut DeterminantImage,
) -> Box<[MarkEdgeOp]> {
    let layout = relation.layout();
    // Window edges and touched parents (`touchedParents`' two halves).
    // The source half is φ-BLIND: every delta child touches its parent
    // tuple, φ-satisfying or not — the model's superset narrowing (a
    // non-φ fact never changes a child group; wider touched only
    // re-checks more). The edge itself is φ-gated exactly as a
    // containment's, so the child-group walk counts σφ members only.
    let mut window_edges = Vec::new();
    for &window_id in relation.window_sources() {
        let statement = schema.window(window_id);
        window_child_image(statement, layout, fact, scratch);
        touched_parents
            .entry(window_id)
            .or_default()
            .insert(scratch.clone());
        if satisfies(&selections.window(window_id).source, layout, fact) {
            window_edges.push(MarkEdgeOp {
                statement: statement.id,
                key_bytes: scratch.clone(),
            });
        }
    }
    // The target half: a delta parent inside ψ touches its own key tuple
    // (a group newly constrained or released). Closed parents never reach
    // a fact op (writes refused), so only the keyed arm exists here.
    for &window_id in relation.window_targets() {
        let statement = schema.window(window_id);
        if let Enforcement::ScalarProbe { target_key, .. } = &statement.enforcement
            && satisfies(&selections.window(window_id).target, layout, fact)
        {
            let key_statement = schema.key(*target_key);
            keys::determinant_image(layout, &key_statement.projection, fact, scratch);
            touched_parents
                .entry(window_id)
                .or_default()
                .insert(scratch.clone());
        }
    }
    window_edges.into_boxed_slice()
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
    deleted_determinants
        .into_iter()
        .filter_map(|entry| {
            let reestablished = inserted_determinants.contains(&entry);
            let (key, determinant) = entry;
            let dependents: Box<[DependentCheck]> = schema
                .dependents(key)
                .iter()
                .filter_map(|&containment_id| {
                    let statement = schema.containment(containment_id);
                    if matches!(statement.enforcement, Enforcement::Closed { .. }) {
                        return None;
                    }
                    let psi_qualified = if reestablished {
                        match &selections.containment(containment_id).target {
                            SelectionCheck::Empty => return None,
                            SelectionCheck::Never => false,
                            SelectionCheck::Compare(_) => true,
                        }
                    } else {
                        false
                    };
                    Some(DependentCheck {
                        containment: containment_id,
                        psi_qualified,
                    })
                })
                .collect();
            if dependents.is_empty() {
                return None;
            }
            Some(DeterminantCheck {
                key,
                determinant,
                dependents,
            })
        })
        .collect()
}
