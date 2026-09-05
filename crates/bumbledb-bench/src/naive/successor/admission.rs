//! Mutable-consulted-relation support and the one-command delta algebra —
//! the ASS-001 successor's independent model side (Lean twin:
//! `lean/Bumbledb/Txn/Support.lean`; chapter 13 §4, chapter 02
//! counterexamples; gates CONC-01/CONC-02/CONC-05, E-DELTA, E-ADMIT).
//!
//! The support derivation here is INDEPENDENT: it reads the schema
//! descriptor as data and computes the consulted/mutable relation sets
//! itself; judgments come from the [`NaiveDb`] reference judge, never the
//! engine. When P01/P03 land the production support planner, the F3
//! differential compares it against [`mutable_support`] per accepted
//! statement form — shared closed targets, closed sources, selections,
//! capacity weights and isolated relations included.

use std::collections::BTreeSet;

use bumbledb::RelationId;
use bumbledb::schema::{SchemaDescriptor, StatementDescriptor};

/// The relations one statement consults — its whole read footprint.
#[must_use]
pub fn consulted(statement: &StatementDescriptor) -> Vec<RelationId> {
    match statement {
        StatementDescriptor::Functionality { relation, .. } => vec![*relation],
        StatementDescriptor::Containment { source, target }
        | StatementDescriptor::Capacity { target, source, .. } => {
            vec![source.relation, target.relation]
        }
    }
}

/// **The mutable consulted support**: consulted relations that are not
/// closed. A closed (ground-axiom) relation denotes a theory constant and
/// contributes no mutable edge — the retired braid model's
/// `ComponentClosed` premise over ALL consulted relations (closed targets
/// included) is exactly what this replaces.
#[must_use]
pub fn mutable_support(
    descriptor: &SchemaDescriptor,
    statement: &StatementDescriptor,
) -> BTreeSet<RelationId> {
    consulted(statement)
        .into_iter()
        .filter(|relation| {
            descriptor.relations[relation.0 as usize]
                .extension
                .is_none()
        })
        .collect()
}

/// The statements of a materialized roster whose mutable support avoids
/// every relation in `touched` — the set whose verdicts a delta over
/// `touched` cannot move.
#[must_use]
pub fn untouched_statements(
    descriptor: &SchemaDescriptor,
    materialized: &[StatementDescriptor],
    touched: &BTreeSet<RelationId>,
) -> Vec<usize> {
    materialized
        .iter()
        .enumerate()
        .filter(|(_, statement)| {
            mutable_support(descriptor, statement)
                .intersection(touched)
                .next()
                .is_none()
        })
        .map(|(index, _)| index)
        .collect()
}

/// Filter a complete-judgment citation list to the given statement ids —
/// the projection the stability tests compare.
#[must_use]
pub fn citations_for(
    violations: &[crate::naive::Violation],
    statements: &[usize],
) -> Vec<crate::naive::Violation> {
    use crate::naive::Violation;
    violations
        .iter()
        .filter(|violation| match violation {
            Violation::Functionality { statement }
            | Violation::Capacity { statement, .. }
            | Violation::CapacityRayMeasure { statement }
            | Violation::Containment { statement, .. } => {
                statements.contains(&(statement.0 as usize))
            }
            Violation::ClosedRelationWrite { .. } => false,
        })
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use bumbledb::schema::{
        Bound, RelationDescriptor, Row, SchemaDescriptor, StatementDescriptor, ValueType, Weight,
    };
    use bumbledb::{RelationId, Value};

    use crate::fixture::{field, side};
    use crate::naive::{Delta, NaiveDb, Violation};

    use super::{citations_for, mutable_support, untouched_statements};

    const A: RelationId = RelationId(0);
    const B: RelationId = RelationId(1);
    const C: RelationId = RelationId(2); // closed vocabulary

    /// A(x,u); B(y); closed C = {0,1}; statements:
    ///   0: A(x) -> A (key)
    ///   1: B(y) <= C(id)            — mutable support {B}: C is closed
    ///   2: A(x) <= C(id)            — mutable support {A}: shared closed C
    ///   3: A(x) <={0..1} B(y)       — capacity, support {A, B}
    fn descriptor() -> SchemaDescriptor {
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    extension: None,
                    name: "A".into(),
                    fields: vec![field("x", ValueType::U64), field("u", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: None,
                    name: "B".into(),
                    fields: vec![field("y", ValueType::U64)],
                },
                RelationDescriptor {
                    extension: Some(Box::new([
                        Row {
                            handle: "Zero".into(),
                            values: Box::new([]),
                        },
                        Row {
                            handle: "One".into(),
                            values: Box::new([]),
                        },
                    ])),
                    name: "C".into(),
                    fields: vec![],
                },
            ],
            statements: vec![
                StatementDescriptor::Functionality {
                    relation: A,
                    projection: Box::new([bumbledb::FieldId(0)]),
                },
                StatementDescriptor::Containment {
                    source: side(B, &[0], &[]),
                    target: side(C, &[0], &[]),
                },
                StatementDescriptor::Containment {
                    source: side(A, &[0], &[]),
                    target: side(C, &[0], &[]),
                },
                StatementDescriptor::Capacity {
                    target: side(A, &[0], &[]),
                    weight: Weight::Unit,
                    lo: 0,
                    hi: Some(Bound::Lit(1)),
                    source: side(B, &[0], &[]),
                },
            ],
        }
    }

    fn a_fact(x: u64, u: u64) -> (RelationId, Vec<Value>) {
        (A, vec![Value::U64(x), Value::U64(u)])
    }

    fn b_fact(y: u64) -> (RelationId, Vec<Value>) {
        (B, vec![Value::U64(y)])
    }

    #[test]
    fn judgment_stable_under_untouched_relations() {
        let descriptor = descriptor();
        let materialized = descriptor.materialized_statements();
        let mut db = NaiveDb::new(&descriptor);
        // B carries a violating containment row (y = 7 is outside closed
        // C = {0, 1}); A is clean.
        db.load_candidate(&[a_fact(0, 10), b_fact(7)]);
        let before = db.judge_complete();
        assert!(
            before
                .iter()
                .any(|v| matches!(v, Violation::Containment { .. })),
            "the fixture starts with a violated B-containment"
        );
        // The delta touches ONLY A. Every statement whose mutable support
        // avoids A must keep its exact citations.
        let touched: BTreeSet<RelationId> = [A].into_iter().collect();
        let stable = untouched_statements(&descriptor, &materialized, &touched);
        assert!(
            !stable.is_empty(),
            "the B-containment's mutable support avoids A"
        );
        let cited_before = citations_for(&before, &stable);
        db.load_candidate(&[a_fact(1, 11), a_fact(2, 12)]);
        let after = db.judge_complete();
        let cited_after = citations_for(&after, &stable);
        assert_eq!(
            cited_before, cited_after,
            "a delta outside a statement's mutable consulted support \
             leaves that statement's judgment unchanged"
        );
    }

    #[test]
    fn shared_closed_vocabulary_does_not_merge_supports() {
        let descriptor = descriptor();
        // Statements 1 and 2 share ONLY the closed vocabulary C; their
        // mutable supports are disjoint.
        let s1 = mutable_support(&descriptor, &descriptor.statements[1]);
        let s2 = mutable_support(&descriptor, &descriptor.statements[2]);
        assert_eq!(s1, [B].into_iter().collect::<BTreeSet<_>>());
        assert_eq!(s2, [A].into_iter().collect::<BTreeSet<_>>());
        assert!(
            s1.intersection(&s2).next().is_none(),
            "shared closed vocabulary never merges two mutable components"
        );
        // And the closed relation itself is in NO mutable support.
        for statement in &descriptor.statements {
            assert!(
                !mutable_support(&descriptor, statement).contains(&C),
                "a closed relation contributes no mutable edge"
            );
        }
        // Semantically: a violating A-delta moves statement 2's verdict
        // and leaves statement 1's untouched, though both cite C.
        let materialized = descriptor.materialized_statements();
        let mut db = NaiveDb::new(&descriptor);
        db.load_candidate(&[a_fact(9, 1)]); // x = 9 violates A <= C
        let verdict = db.judge_complete();
        let touched: BTreeSet<RelationId> = [A].into_iter().collect();
        let stable = untouched_statements(&descriptor, &materialized, &touched);
        let stable_citations = citations_for(&verdict, &stable);
        assert!(
            stable_citations.is_empty(),
            "no statement outside A's support is cited by an A-only state"
        );
    }

    #[test]
    fn same_command_tie_rule_add_wins() {
        let descriptor = descriptor();
        // Same exact fact on both sides of ONE command: add wins, whatever
        // the spelling order of the operation lists.
        let fact = b_fact(0);
        let forward = Delta {
            deletes: vec![fact.clone()],
            inserts: vec![fact.clone()],
        };
        // Base absent: the fact lands present.
        let mut db = NaiveDb::new(&descriptor);
        db.apply(&forward).expect("admissible");
        assert!(
            db.relation(B).iter().any(|t| t.0 == fact.1),
            "add wins inside one command"
        );
        // Base present: the same command is a no-op — same-command
        // normalization, not cross-command conflict resolution — and the
        // generation does not advance on a no-op.
        let before = db.generation();
        db.apply(&forward).expect("admissible");
        assert!(db.relation(B).iter().any(|t| t.0 == fact.1));
        assert_eq!(
            db.generation(),
            before,
            "a normalized no-op advances nothing"
        );
        // Removing an absent fact is an ordinary no-op.
        let absent = Delta {
            deletes: vec![b_fact(1)],
            inserts: vec![],
        };
        let before = db.generation();
        db.apply(&absent).expect("admissible");
        assert_eq!(db.generation(), before);
    }

    #[test]
    fn raw_commutation_does_not_commute_admission() {
        let descriptor = descriptor();
        // Two deltas with no cross add/remove conflicts: distinct B rows,
        // one shared capacity group (A(x=0) admits at most one B child).
        let d1 = Delta {
            deletes: vec![],
            inserts: vec![b_fact(0)],
        };
        let d2 = Delta {
            deletes: vec![],
            inserts: vec![b_fact(1)],
        };
        // The RAW set transformations commute: both orders land one final
        // fact set.
        let mut ab = NaiveDb::new(&descriptor);
        ab.load_candidate(&[a_fact(0, 1)]);
        let mut ba = ab.clone();
        ab.load_candidate(&d1.inserts);
        ab.load_candidate(&d2.inserts);
        ba.load_candidate(&d2.inserts);
        ba.load_candidate(&d1.inserts);
        assert_eq!(
            ab.relation(B),
            ba.relation(B),
            "raw set application commutes for conflict-free deltas"
        );
        // ADMISSION does not: each delta is admissible alone, and the
        // second always rejects through the shared capacity law — disjoint
        // effects still interact through group measures, which is why raw
        // commutation licenses no reordering of observed outcomes.
        for (first, second) in [(&d1, &d2), (&d2, &d1)] {
            let mut db = NaiveDb::new(&descriptor);
            db.apply(&Delta {
                deletes: vec![],
                inserts: vec![a_fact(0, 1)],
            })
            .expect("the parent row is admissible");
            db.apply(first).expect("the first child fits the capacity");
            let refused = db.apply(second).expect_err("the second must refuse");
            assert!(
                refused
                    .iter()
                    .any(|v| matches!(v, Violation::Capacity { measure: 2, .. })),
                "the union group measures two against a ceiling of one: {refused:?}"
            );
        }
        // The key law is not union-closed either (chapter 02): two
        // same-key rows are each admissible from the base, their union is
        // not — deduplication cannot hide distinct full tuples.
        let k1 = Delta {
            deletes: vec![],
            inserts: vec![a_fact(1, 100)],
        };
        let k2 = Delta {
            deletes: vec![],
            inserts: vec![a_fact(1, 200)],
        };
        for (first, second) in [(&k1, &k2), (&k2, &k1)] {
            let mut db = NaiveDb::new(&descriptor);
            db.apply(first).expect("one keyed row is admissible");
            let refused = db.apply(second).expect_err("the key must refuse");
            assert!(
                refused
                    .iter()
                    .any(|v| matches!(v, Violation::Functionality { .. })),
                "the same key with different payloads violates: {refused:?}"
            );
        }
    }
}
