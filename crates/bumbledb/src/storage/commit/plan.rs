//! The commit plan (`docs/architecture/50-storage.md` § Write path): every
//! derivable key byte and check set of one commit, computed as a **pure
//! function of (delta, schema)** before a single LMDB page is touched —
//! representation over control flow applied to the write path. Per fact:
//! its guard bytes per key statement (pointwise keys marked for the
//! ordered-neighbor probe) and its reverse-edge key bytes per containment
//! whose source selection it satisfies — the same permuted bytes serve the
//! `R` put/delete and the insert's source probe. Aggregated: the
//! per-statement disestablished-guard check sets (deleted − inserted,
//! with ψ-qualified re-establishment inputs marked for the judgment
//! phase). Selection literals arrive pre-encoded ([`Selections`]) and the
//! plan owns them for the rest of the commit.
//!
//! The honest boundary, stated up front: row ids are **not** derivable
//! (deletes need the `M` lookup; inserts mint from the high-water) and
//! judgment probe *results* need final-state reads. The plan owns key
//! material and check sets; the applier keeps the id plumbing and the
//! desync probes; the judgment keeps the final-state probes.

use std::collections::BTreeSet;

use crate::schema::{ContainmentId, Enforcement, KeyId, RelationId, Schema, StatementId};
use crate::storage::delta::WriteDelta;
use crate::storage::keys;

use super::judgment::{SelectionCheck, Selections, satisfies};

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
    pub(crate) target_checks: Box<[GuardCheck]>,
}

/// Everything derivable about one fact's application.
pub(crate) struct FactOp<'d> {
    pub(crate) relation: RelationId,
    /// The canonical fact bytes (identity = bytes, `10-data-model.md`).
    pub(crate) fact: &'d [u8],
    /// One per key statement of the relation, materialized order.
    pub(crate) guards: Box<[GuardOp]>,
    /// One per outgoing containment whose source selection the fact
    /// satisfies — a fact outside σ has no edge, by design.
    pub(crate) edges: Box<[EdgeOp]>,
    /// One per outgoing **closed-target** containment whose source
    /// selection the fact satisfies: no guard bytes, no `R` traffic —
    /// the compiled member set is the whole plan, and the judgment is
    /// one AND and one test on the insert side
    /// (`docs/architecture/30-dependencies.md`). Dead weight on a
    /// delete op (removing a reference cannot violate an inclusion);
    /// only the insert-side judgment consumes it.
    pub(crate) memberships: Box<[MembershipOp]>,
}

/// One key statement's guard material for one fact.
pub(crate) struct GuardOp {
    /// The `Functionality` statement.
    pub(crate) statement: StatementId,
    /// The projected fields' canonical encodings in statement order
    /// ([`keys::guard_bytes`]) — the `U` key's guard segment.
    pub(crate) guard: Box<[u8]>,
    /// Interval-carrying key: the exact `U` put cannot detect overlap, so
    /// the insert additionally runs the ordered-neighbor probe.
    pub(crate) pointwise: bool,
}

/// One closed-target containment of one fact: the membership judgment's
/// whole input. The id is the referencing field's decoded word — already
/// in hand during the derivation, never re-sliced at judgment.
pub(crate) struct MembershipOp {
    /// The validation-minted containment witness; the fingerprint identity
    /// is derived only when constructing an error.
    pub(crate) containment: ContainmentId,
    /// The referencing field's word — a row id into the target's sealed
    /// extension when the fact is legal; out of range is simply a miss.
    pub(crate) id: u64,
}

/// One containment edge of one fact: the `R` key material and, on the
/// insert side, the source-probe input.
pub(crate) struct EdgeOp {
    /// The typed containment used for sealed selections and enforcement.
    pub(crate) containment: ContainmentId,
    /// The `Containment` statement.
    pub(crate) statement: StatementId,
    /// The source projection laid down in the target key's guard order
    /// ([`keys::permuted_guard_bytes`]) — the `R` key-bytes segment and
    /// the source probe's target guard value.
    pub(crate) key_bytes: Box<[u8]>,
    pub(crate) target_relation: RelationId,
    /// The `Functionality` statement whose `U` guard the source probes.
    pub(crate) target_key: KeyId,
    /// Interval-position statement: the source probe is the coverage
    /// walk, not the scalar get.
    pub(crate) coverage: bool,
}

/// One disestablished key tuple and the dependent statements that must
/// re-check it (`deleted − inserted`, per statement).
pub(crate) struct GuardCheck {
    /// The key (`Functionality`) statement whose tuple left.
    pub(crate) key: KeyId,
    /// The tuple's guard bytes (interval keys carry the 16-byte tail).
    pub(crate) guard: Box<[u8]>,
    /// The dependent containments still owed a check, in materialized
    /// order — a dependent whose empty-ψ tuple re-lands in phase 2 is
    /// already dropped here.
    pub(crate) dependents: Box<[DependentCheck]>,
}

/// One dependent statement's entry in a [`GuardCheck`].
pub(crate) struct DependentCheck {
    /// The validation-minted containment witness.
    pub(crate) containment: ContainmentId,
    /// Interval-position statement: survivors re-run the coverage walk.
    pub(crate) coverage: bool,
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
    // Guard tuples of key statements some containment depends on — the
    // inputs of the target-side check set (`deleted − inserted`).
    let mut deleted_guards: BTreeSet<(KeyId, Box<[u8]>)> = BTreeSet::new();
    let mut inserted_guards: BTreeSet<(KeyId, Box<[u8]>)> = BTreeSet::new();
    let mut scratch = Vec::new();
    let deletes = delta
        .deletes()
        .map(|(rel, fact)| {
            fact_op(
                schema,
                &selections,
                rel,
                fact,
                &mut deleted_guards,
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
                &mut inserted_guards,
                &mut scratch,
            )
        })
        .collect();
    let target_checks = target_checks(schema, &selections, deleted_guards, &inserted_guards);
    CommitPlan {
        selections,
        deletes,
        inserts,
        target_checks,
    }
}

/// Derives one fact's op: guard bytes per key statement, reverse-edge key
/// bytes per satisfied containment. Guard tuples of dependent-carrying
/// key statements are recorded into `dependent_guards` for the check-set
/// difference.
fn fact_op<'d>(
    schema: &Schema,
    selections: &Selections,
    rel: RelationId,
    fact: &'d [u8],
    dependent_guards: &mut BTreeSet<(KeyId, Box<[u8]>)>,
    scratch: &mut Vec<u8>,
) -> FactOp<'d> {
    // Every F/M/U/R key byte originates from this derivation — the
    // refusal-hardening chokepoint (`keys::debug_assert_ordinary`).
    keys::debug_assert_ordinary(schema, rel);
    let relation = schema.relation(rel);
    let layout = relation.layout();
    let guards = relation
        .keys()
        .iter()
        .map(|&key_id| {
            let statement = schema.key(key_id);
            // Guard keys derived by slicing projected fields out of
            // fact_bytes — never a scan; interval fields slice as their
            // whole 16 bytes.
            keys::guard_bytes(layout, &statement.projection, fact, scratch);
            let guard: Box<[u8]> = scratch.as_slice().into();
            if !schema.dependents(key_id).is_empty() {
                dependent_guards.insert((key_id, guard.clone()));
            }
            GuardOp {
                statement: statement.id,
                guard,
                pointwise: statement.pointwise,
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
            Enforcement::Probe {
                target_key,
                key_permutation,
                coverage,
            } => {
                keys::permuted_guard_bytes(
                    layout,
                    &statement.source.projection,
                    key_permutation,
                    fact,
                    scratch,
                );
                edges.push(EdgeOp {
                    containment: containment_id,
                    statement: statement.id,
                    key_bytes: scratch.as_slice().into(),
                    target_relation: statement.target.relation,
                    target_key: *target_key,
                    coverage: *coverage,
                });
            }
            Enforcement::Closed { .. } => {
                let word = crate::encoding::field_bytes(
                    fact,
                    layout,
                    usize::from(statement.source.projection[0].0),
                );
                memberships.push(MembershipOp {
                    containment: containment_id,
                    id: u64::from_be_bytes(word.try_into().expect("u64 field is 8 bytes")),
                });
            }
        }
    }
    FactOp {
        relation: rel,
        fact,
        guards,
        edges: edges.into_boxed_slice(),
        memberships: memberships.into_boxed_slice(),
    }
}

/// The target-side check set: every deleted guard tuple, expanded per
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
    deleted_guards: BTreeSet<(KeyId, Box<[u8]>)>,
    inserted_guards: &BTreeSet<(KeyId, Box<[u8]>)>,
) -> Box<[GuardCheck]> {
    deleted_guards
        .into_iter()
        .filter_map(|entry| {
            let reestablished = inserted_guards.contains(&entry);
            let (key, guard) = entry;
            let dependents: Box<[DependentCheck]> = schema
                .dependents(key)
                .iter()
                .filter_map(|&containment_id| {
                    let statement = schema.containment(containment_id);
                    let coverage = match &statement.enforcement {
                        Enforcement::Probe { coverage, .. } => *coverage,
                        Enforcement::Closed { .. } => return None,
                    };
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
                        coverage,
                        psi_qualified,
                    })
                })
                .collect();
            if dependents.is_empty() {
                return None;
            }
            Some(GuardCheck {
                key,
                guard,
                dependents,
            })
        })
        .collect()
}
