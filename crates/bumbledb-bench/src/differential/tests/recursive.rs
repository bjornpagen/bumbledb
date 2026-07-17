//! The closure goldens (the shipping law: the oracles landed BEFORE
//! the evaluator — `docs/architecture/60-validation.md` § the two
//! oracles): hand-verified reachability
//! answers over a fixed tree and a fixed cyclic graph, held against
//! every oracle that can run a program — the naive stratified fixpoint
//! ([`NaiveDb::program`]), the `SQLite` recursive lane
//! ([`translate_program`] executed against the same facts), and the
//! ENGINE's per-stratum fixpoint driver (`api/prepared/fixpoint.rs` —
//! the goldens went three-way the day the driver landed). The
//! recursive conformance corpus (`crate::conformance`) carries the
//! Lean side (`lean/Bumbledb/Exec/Fixpoint.lean: evalProgram`) over
//! the Tiny worlds.

use bumbledb::schema::ValidateDescriptor as _;
use std::collections::BTreeSet;

use bumbledb::schema::{RelationDescriptor, SchemaDescriptor, ValueType};
use bumbledb::{
    Atom, AtomSource, FieldId, FindTerm, HeadTerm, PredId, PredicateDef, Program, Rule, Term,
    Value, VarId,
};

use crate::fixture::field;
use crate::naive::{Delta, NaiveDb, Tuple};
use crate::translate::{sqlite_program_expressible, translate_program};

/// The goldens' graph descriptor: `Node(id)`, `Edge(src, dst)` — no
/// statements (nothing here judges writes; the graphs are fixed data).
fn graph_descriptor() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Node".into(),
                fields: vec![field("id", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Edge".into(),
                fields: vec![field("src", ValueType::U64), field("dst", ValueType::U64)],
            },
        ],
        statements: vec![],
    }
}

/// The descriptor, validated.
fn graph_schema() -> bumbledb::Schema {
    graph_descriptor()
        .validate()
        .expect("the graph schema validates")
}

const NODE: bumbledb::RelationId = bumbledb::RelationId(0);
const EDGE: bumbledb::RelationId = bumbledb::RelationId(1);

/// The fixed tree, edges child → parent: `0 ← {1, 2}`, `1 ← {3, 4}`,
/// `2 ← {5}` — closure sizes bounded by depth 2.
const TREE: [(u64, u64); 5] = [(1, 0), (2, 0), (3, 1), (4, 1), (5, 2)];

/// The fixed cyclic graph: the 3-cycle `0 → 1 → 2 → 0` with the tail
/// `2 → 3` — the fixpoint must saturate the cycle and stop.
const CYCLE: [(u64, u64); 4] = [(0, 1), (1, 2), (2, 0), (2, 3)];

/// The transitive closure: `p0(x, y) | Edge(x, y); p0(x, z) |
/// Edge(x, y), p0(y, z)` — linear, single recursive atom.
fn closure_program() -> Program {
    let v = |id: u16| Term::Var(VarId(id));
    Program {
        predicates: vec![PredicateDef {
            head: vec![HeadTerm::Var, HeadTerm::Var],
            rules: vec![
                Rule {
                    finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
                    atoms: vec![Atom {
                        source: AtomSource::Edb(EDGE),
                        bindings: vec![(FieldId(0), v(0)), (FieldId(1), v(1))],
                    }],
                    negated: vec![],
                    conditions: vec![],
                },
                Rule {
                    finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
                    atoms: vec![
                        Atom {
                            source: AtomSource::Edb(EDGE),
                            bindings: vec![(FieldId(0), v(0)), (FieldId(1), v(1))],
                        },
                        Atom {
                            source: AtomSource::Idb(PredId(0)),
                            bindings: vec![(FieldId(0), v(1)), (FieldId(1), v(2))],
                        },
                    ],
                    negated: vec![],
                    conditions: vec![],
                },
            ],
        }],
        output: PredId(0),
    }
}

/// The stratified extension: `p1(x) | Node(id = x), ¬p0(c1 = x)` —
/// nodes that are nobody's reachable target (negation OF the finished
/// closure stratum).
fn unreached_program() -> Program {
    let mut program = closure_program();
    program.predicates.push(PredicateDef {
        head: vec![HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: AtomSource::Edb(NODE),
                bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
            }],
            negated: vec![Atom {
                source: AtomSource::Idb(PredId(0)),
                bindings: vec![(FieldId(1), Term::Var(VarId(0)))],
            }],
            conditions: vec![],
        }],
    });
    program.output = PredId(1);
    program
}

/// The naive model over one fixed graph: nodes 0..n plus the edges.
fn naive_world(nodes: u64, edges: &[(u64, u64)]) -> NaiveDb {
    let descriptor = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Node".into(),
                fields: vec![field("id", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Edge".into(),
                fields: vec![field("src", ValueType::U64), field("dst", ValueType::U64)],
            },
        ],
        statements: vec![],
    };
    let mut naive = NaiveDb::new(&descriptor);
    let mut delta = Delta::default();
    for node in 0..nodes {
        delta.inserts.push((NODE, vec![Value::U64(node)]));
    }
    for (src, dst) in edges {
        delta
            .inserts
            .push((EDGE, vec![Value::U64(*src), Value::U64(*dst)]));
    }
    naive
        .apply(&delta)
        .expect("no statements: every write lands");
    naive
}

/// The `SQLite` oracle over the same graph: DDL from the shared
/// mapping, the fixed facts, the translated `WITH RECURSIVE` executed.
fn sqlite_answers(nodes: u64, edges: &[(u64, u64)], program: &Program) -> BTreeSet<Tuple> {
    let schema = graph_schema();
    let conn = rusqlite::Connection::open_in_memory().expect("open");
    for statement in crate::sqlmap::schema_ddl(&schema) {
        conn.execute(&statement, []).expect("ddl");
    }
    for node in 0..nodes {
        conn.execute(
            "INSERT INTO \"Node\" VALUES (?1)",
            [i64::try_from(node).expect("small")],
        )
        .expect("insert node");
    }
    for (src, dst) in edges {
        conn.execute(
            "INSERT INTO \"Edge\" VALUES (?1, ?2)",
            [
                i64::try_from(*src).expect("small"),
                i64::try_from(*dst).expect("small"),
            ],
        )
        .expect("insert edge");
    }
    let translated = translate_program(program, &schema, &[]).expect("translates");
    let arity = program.predicates[usize::from(program.output.0)].head.len();
    let mut statement = conn.prepare(&translated.sql).expect("prepare");
    let rows = statement
        .query_map([], |row| {
            let mut values = Vec::with_capacity(arity);
            for column in 0..arity {
                let raw: i64 = row.get(column)?;
                values.push(Value::U64(u64::try_from(raw).expect("node ids are small")));
            }
            Ok(Tuple(values))
        })
        .expect("query");
    rows.map(|row| row.expect("row decodes")).collect()
}

/// The engine over the same graph: a real store, the program prepared
/// through `Db::prepare` and executed under the fixpoint driver
/// — [`crate::differential::engine_program`] is the shared leg.
fn engine_answers(nodes: u64, edges: &[(u64, u64)], program: &Program) -> BTreeSet<Tuple> {
    // Parallel golden tests each get their own store (the fixture
    // TempDir is tag-keyed).
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let descriptor = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Node".into(),
                fields: vec![field("id", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Edge".into(),
                fields: vec![field("src", ValueType::U64), field("dst", ValueType::U64)],
            },
        ],
        statements: vec![],
    };
    let tag = format!(
        "recursive-goldens-{}",
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let dir = crate::fixture::TempDir::new(&tag);
    let db = bumbledb::Db::create(dir.path(), descriptor).expect("create engine store");
    db.write(|tx| {
        for node in 0..nodes {
            tx.insert_dyn(NODE, &[Value::U64(node)])?;
        }
        for (src, dst) in edges {
            tx.insert_dyn(EDGE, &[Value::U64(*src), Value::U64(*dst)])?;
        }
        Ok(())
    })
    .expect("no statements: every write lands");
    crate::differential::engine_program(&db, program, &[])
}

/// Every oracle that can answer a program, by name — the goldens below
/// loop over this list, so every oracle answers every case: the naive
/// stratified fixpoint, `SQLite`'s `WITH RECURSIVE` lane, and the
/// engine's fixpoint driver (three-way, the shipping law closed).
fn oracle_answers(
    nodes: u64,
    edges: &[(u64, u64)],
    program: &Program,
) -> Vec<(&'static str, BTreeSet<Tuple>)> {
    assert_eq!(
        sqlite_program_expressible(program),
        Ok(()),
        "the goldens' programs stay inside the SQLite lane"
    );
    vec![
        (
            "naive",
            naive_world(nodes, edges)
                .program(program, &[])
                .expect("closure programs raise no runtime error"),
        ),
        ("sqlite", sqlite_answers(nodes, edges, program)),
        ("engine", engine_answers(nodes, edges, program)),
    ]
}

fn pairs(expected: &[(u64, u64)]) -> BTreeSet<Tuple> {
    expected
        .iter()
        .map(|(a, b)| Tuple(vec![Value::U64(*a), Value::U64(*b)]))
        .collect()
}

fn singletons(expected: &[u64]) -> BTreeSet<Tuple> {
    expected
        .iter()
        .map(|node| Tuple(vec![Value::U64(*node)]))
        .collect()
}

/// The tree closure, verified BY HAND: each node's ancestors up the
/// two-level tree — 8 pairs, no more.
#[test]
fn tree_closure_matches_the_hand_answer_on_every_oracle() {
    let expected = pairs(&[
        (1, 0),
        (2, 0),
        (3, 1),
        (3, 0),
        (4, 1),
        (4, 0),
        (5, 2),
        (5, 0),
    ]);
    for (oracle, answers) in oracle_answers(6, &TREE, &closure_program()) {
        assert_eq!(answers, expected, "{oracle} disagrees with the hand answer");
    }
}

/// The cyclic closure, verified BY HAND: everyone on the 3-cycle
/// reaches everyone (self included) plus the tail; the tail reaches
/// nothing — 12 pairs, and the fixpoint terminates on the cycle.
#[test]
fn cyclic_closure_matches_the_hand_answer_on_every_oracle() {
    let expected = pairs(&[
        (0, 0),
        (0, 1),
        (0, 2),
        (0, 3),
        (1, 0),
        (1, 1),
        (1, 2),
        (1, 3),
        (2, 0),
        (2, 1),
        (2, 2),
        (2, 3),
    ]);
    for (oracle, answers) in oracle_answers(4, &CYCLE, &closure_program()) {
        assert_eq!(answers, expected, "{oracle} disagrees with the hand answer");
    }
}

/// Empty-store recursion: the fixpoint over zero facts is the empty
/// set on every oracle — round 0 derives nothing, the first Δ is empty,
/// and the driver's stratum closes without a round.
#[test]
fn recursion_over_the_empty_store_is_empty_on_every_oracle() {
    for (oracle, answers) in oracle_answers(0, &[], &closure_program()) {
        assert!(answers.is_empty(), "{oracle} answered a fact-free store");
    }
    for (oracle, answers) in oracle_answers(0, &[], &unreached_program()) {
        assert!(answers.is_empty(), "{oracle} answered a fact-free store");
    }
}

/// Negation of the finished lower stratum, verified BY HAND: on the
/// tree the leaves `{3, 4, 5}` are nobody's reachable target; on the
/// cyclic graph every node is reached (the cycle reaches itself and the
/// tail), so the answer is empty.
#[test]
fn stratified_negation_matches_the_hand_answers_on_every_oracle() {
    let expected = singletons(&[3, 4, 5]);
    for (oracle, answers) in oracle_answers(6, &TREE, &unreached_program()) {
        assert_eq!(answers, expected, "{oracle} disagrees with the hand answer");
    }
    let expected = singletons(&[]);
    for (oracle, answers) in oracle_answers(4, &CYCLE, &unreached_program()) {
        assert_eq!(answers, expected, "{oracle} disagrees with the hand answer");
    }
}

/// The strata-refusal parity family (the `DnfExceedsRules` precedent
/// generalized to the recursion roster,
/// docs/architecture/60-validation.md § error parity): the naive side
/// computes the dependency facts FROM THE DEFINITION —
/// `lean/Bumbledb/Query/Syntax.lean: Program.StratifiedBy`'s cycle
/// conditions read off the reads graph by an obviously-correct
/// Floyd–Warshall closure, sharing nothing with the engine's iterative
/// Tarjan (`ir/validate/strata.rs`) — and the engine's verdict must
/// agree: accept exactly when no negated read and no fold read stays
/// inside its own SCC, and every typed rejection must name a `(pred,
/// via)` pair the definition convicts.
mod strata_parity {
    use super::*;
    use crate::corpus_gen::Rng;
    use bumbledb::error::ValidationError;
    use bumbledb::{AggOp, HeadOp};

    /// One seeded program: 2–3 binary predicates over `Edge`, rules
    /// carrying free positive reads, negated reads, and per-predicate
    /// fold shapes — cyclic topologies (legal and illegal) arise from
    /// the free predicate draws.
    fn random_program(rng: &mut Rng) -> Program {
        let pred_count = 2 + rng.range(2); // 2..=3
        let v = |id: u16| Term::Var(VarId(id));
        let edge = || Atom {
            source: AtomSource::Edb(EDGE),
            bindings: vec![(FieldId(0), v(0)), (FieldId(1), v(1))],
        };
        let idb_read = |rng: &mut Rng| Atom {
            source: AtomSource::Idb(PredId(u16::try_from(rng.range(pred_count)).expect("small"))),
            bindings: vec![(FieldId(0), v(0)), (FieldId(1), v(1))],
        };
        let predicates = (0..pred_count)
            .map(|index| {
                // A fold head is legal only AT the output (the
                // executable-class roster item,
                // `ValidationError::AggregateInteriorPredicate`) — the
                // generator stays inside that fence so every rejection
                // it draws is the cycle roster's.
                if index == 0 && rng.chance(1, 4) {
                    // The fold predicate: one rule, `Count` over the
                    // rule's bindings, with an optional read (positive
                    // or negated) whose target is free to land in the
                    // fold's own SCC.
                    let mut atoms = vec![edge()];
                    let mut negated = vec![];
                    if rng.chance(3, 4) {
                        if rng.chance(1, 4) {
                            negated.push(idb_read(rng));
                        } else {
                            atoms.push(idb_read(rng));
                        }
                    }
                    PredicateDef {
                        head: vec![HeadTerm::Var, HeadTerm::Aggregate(HeadOp::Count)],
                        rules: vec![Rule {
                            finds: vec![
                                FindTerm::Var(VarId(0)),
                                FindTerm::Aggregate {
                                    op: AggOp::Count,
                                    over: None,
                                },
                            ],
                            atoms,
                            negated,
                            conditions: vec![],
                        }],
                    }
                } else {
                    let rule_count = 1 + rng.range(2); // 1..=2
                    let rules = (0..rule_count)
                        .map(|_| {
                            let mut atoms = vec![edge()];
                            let mut negated = vec![];
                            if rng.chance(1, 2) {
                                atoms.push(idb_read(rng));
                            }
                            if rng.chance(1, 3) {
                                negated.push(idb_read(rng));
                            }
                            Rule {
                                finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
                                atoms,
                                negated,
                                conditions: vec![],
                            }
                        })
                        .collect();
                    PredicateDef {
                        head: vec![HeadTerm::Var, HeadTerm::Var],
                        rules,
                    }
                }
            })
            .collect();
        Program {
            predicates,
            output: PredId(0),
        }
    }

    /// `reach[p][q]`: a ≥1-edge path in the reads graph (positive and
    /// negated reads alike) — the naive Floyd–Warshall closure.
    fn reads_closure(program: &Program) -> Vec<Vec<bool>> {
        let n = program.predicates.len();
        let mut reach = vec![vec![false; n]; n];
        for (p, def) in program.predicates.iter().enumerate() {
            for rule in &def.rules {
                for atom in rule.atoms.iter().chain(&rule.negated) {
                    if let AtomSource::Idb(q) = atom.source {
                        reach[p][usize::from(q.0)] = true;
                    }
                }
            }
        }
        for k in 0..n {
            for i in 0..n {
                for j in 0..n {
                    reach[i][j] = reach[i][j] || (reach[i][k] && reach[k][j]);
                }
            }
        }
        reach
    }

    /// A roster of `(pred, via)` pairs a cycle condition convicts.
    type PairList = Vec<(PredId, PredId)>;

    /// The signature fixpoint from the definition (the least fixpoint of
    /// "some rule's `Idb` targets, negated included, are all sealed" —
    /// order-independent, so the naive chaotic loop shares nothing with
    /// the engine's pass structure): the predicates that never seal —
    /// the engine's `UnresolvedPredicateSignature` roster
    /// (`ir/validate/validate.rs: seal_signatures`).
    fn unsealed_predicates(program: &Program) -> Vec<PredId> {
        let n = program.predicates.len();
        let mut sealed = vec![false; n];
        loop {
            let mut progressed = false;
            for (p, def) in program.predicates.iter().enumerate() {
                if sealed[p] {
                    continue;
                }
                let can_seal = def.rules.iter().any(|rule| {
                    rule.atoms
                        .iter()
                        .chain(&rule.negated)
                        .all(|atom| match atom.source {
                            AtomSource::Idb(q) => sealed[usize::from(q.0)],
                            AtomSource::Edb(_) => true,
                        })
                });
                if can_seal {
                    sealed[p] = true;
                    progressed = true;
                }
            }
            if !progressed {
                break;
            }
        }
        sealed
            .iter()
            .enumerate()
            .filter(|(_, s)| !**s)
            .map(|(p, _)| PredId(u16::try_from(p).expect("small")))
            .collect()
    }

    /// The two cycle conditions from the definition: every `(pred, via)`
    /// pair a negated read keeps inside its own SCC, and every pair a
    /// fold rule's read (positive or negated) keeps inside its own SCC.
    fn offending_pairs(program: &Program) -> (PairList, PairList) {
        let reach = reads_closure(program);
        let same_scc = |p: usize, q: usize| -> bool { reach[p][q] && reach[q][p] };
        let mut negations = vec![];
        let mut folds = vec![];
        for (p, def) in program.predicates.iter().enumerate() {
            let pred = PredId(u16::try_from(p).expect("small"));
            for rule in &def.rules {
                for atom in &rule.negated {
                    if let AtomSource::Idb(via) = atom.source
                        && same_scc(p, usize::from(via.0))
                    {
                        negations.push((pred, via));
                    }
                }
                let has_fold = rule
                    .finds
                    .iter()
                    .any(|term| matches!(term, FindTerm::Aggregate { .. }));
                if has_fold {
                    for atom in rule.atoms.iter().chain(&rule.negated) {
                        if let AtomSource::Idb(via) = atom.source
                            && same_scc(p, usize::from(via.0))
                        {
                            folds.push((pred, via));
                        }
                    }
                }
            }
        }
        (negations, folds)
    }

    /// The seeded sweep: engine verdict vs the definition on every
    /// drawn program — plus the coverage floor (both verdict classes
    /// and all three refusal kinds must arise, or the sweep proves
    /// nothing). The refusal roster is the whole program-shape fence in
    /// pipeline order (`ir/validate/validate.rs`): the cycle conditions
    /// first (the strata judge), then the signature fixpoint's honest
    /// bottom — so a signature refusal also witnesses that the cycle
    /// definition convicted nothing.
    #[test]
    fn strata_refusals_agree_with_the_from_definition_witness_search() {
        let dir = crate::fixture::TempDir::new("strata-parity");
        let db = bumbledb::Db::create(dir.path(), graph_descriptor()).expect("create engine store");
        let mut accepted = 0u64;
        let mut negation_refusals = 0u64;
        let mut fold_refusals = 0u64;
        let mut signature_refusals = 0u64;
        for seed in 0..2_000u64 {
            let mut rng = Rng::new(seed.wrapping_mul(0xD134_2543_DE82_EF95).wrapping_add(1));
            let program = random_program(&mut rng);
            let (negations, folds) = offending_pairs(&program);
            let unsealed = unsealed_predicates(&program);
            match db.prepare(&program).map(|_| ()) {
                Ok(()) => {
                    accepted += 1;
                    assert!(
                        negations.is_empty() && folds.is_empty(),
                        "the engine accepted a program the definition convicts \
                         (seed {seed}): ¬{negations:?} / fold {folds:?}\n{program:#?}"
                    );
                    assert!(
                        unsealed.is_empty(),
                        "the engine accepted a program whose signature never seals \
                         under the definition (seed {seed}): {unsealed:?}\n{program:#?}"
                    );
                }
                Err(bumbledb::Error::Validation(ValidationError::NegationThroughCycle {
                    pred,
                    via,
                })) => {
                    negation_refusals += 1;
                    assert!(
                        negations.contains(&(pred, via)),
                        "the engine's NegationThroughCycle({pred:?}, {via:?}) is not an \
                         offending pair under the definition (seed {seed}): {negations:?}\n\
                         {program:#?}"
                    );
                }
                Err(bumbledb::Error::Validation(ValidationError::AggregationThroughCycle {
                    pred,
                    via,
                })) => {
                    fold_refusals += 1;
                    assert!(
                        folds.contains(&(pred, via)),
                        "the engine's AggregationThroughCycle({pred:?}, {via:?}) is not an \
                         offending pair under the definition (seed {seed}): {folds:?}\n\
                         {program:#?}"
                    );
                }
                Err(bumbledb::Error::Validation(
                    ValidationError::UnresolvedPredicateSignature { pred },
                )) => {
                    signature_refusals += 1;
                    // The strata judge runs first, so reaching the
                    // sealing loop witnesses a cycle-clean program.
                    assert!(
                        negations.is_empty() && folds.is_empty(),
                        "the engine reached the signature fixpoint past a cycle the \
                         definition convicts (seed {seed}): ¬{negations:?} / fold \
                         {folds:?}\n{program:#?}"
                    );
                    assert!(
                        unsealed.contains(&pred),
                        "the engine's UnresolvedPredicateSignature({pred:?}) is not an \
                         unsealed predicate under the definition (seed {seed}): \
                         {unsealed:?}\n{program:#?}"
                    );
                }
                Err(other) => panic!(
                    "the generator is valid-by-construction outside the program-shape \
                     roster, but the engine rejected seed {seed} with {other:?}\n{program:#?}"
                ),
            }
        }
        for (label, count) in [
            ("accepted", accepted),
            ("negation refusals", negation_refusals),
            ("fold refusals", fold_refusals),
            ("signature refusals", signature_refusals),
        ] {
            assert!(count > 0, "the strata sweep never reached: {label}");
        }
        eprintln!(
            "strata parity: {accepted} accepted, {negation_refusals} ¬-cycle, \
             {fold_refusals} fold-cycle, {signature_refusals} unsealed-signature \
             refusals over 2000 programs"
        );
    }

    /// The predicate-count cap, `DnfExceedsRules`-style: the payload's
    /// `count` must equal the from-definition predicate count.
    #[test]
    fn too_many_predicates_carries_the_definitional_count() {
        let dir = crate::fixture::TempDir::new("strata-parity-cap");
        let db = bumbledb::Db::create(dir.path(), graph_descriptor()).expect("create engine store");
        let over = bumbledb::MAX_PREDICATES + 1;
        let program = Program {
            predicates: (0..over)
                .map(|_| PredicateDef {
                    head: vec![HeadTerm::Var, HeadTerm::Var],
                    rules: vec![Rule {
                        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
                        atoms: vec![Atom {
                            source: AtomSource::Edb(EDGE),
                            bindings: vec![
                                (FieldId(0), Term::Var(VarId(0))),
                                (FieldId(1), Term::Var(VarId(1))),
                            ],
                        }],
                        negated: vec![],
                        conditions: vec![],
                    }],
                })
                .collect(),
            output: PredId(0),
        };
        match db.prepare(&program).map(|_| ()) {
            Err(bumbledb::Error::Validation(ValidationError::TooManyPredicates { count })) => {
                assert_eq!(count, over, "the payload is the definitional count");
            }
            other => panic!("expected TooManyPredicates, got {other:?}"),
        }
    }
}
