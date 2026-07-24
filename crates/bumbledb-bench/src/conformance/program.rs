//! The conformance lane's RECURSIVE arm (the shipping law,
//! `docs/architecture/60-validation.md` § the two oracles):
//! `evalProgram` — the proved fueled fixpoint,
//! `lean/Bumbledb/Exec/Fixpoint.lean: program_eval_sound` — judges the
//! same program cases the naive fixpoint and the `SQLite` recursive
//! lane already agree on: the THIRD oracle was wired for recursion
//! before the engine ran one program, and now holds the landed
//! fixpoint driver to the same cases.
//!
//! One `program-*.json` case per document (format:
//! `lean/conformance/README.md` § program cases): the shared
//! theory/instance blocks, the program (predicates as `{arity, rules}`,
//! rule heads as plain variable-id lists — `PRule.finds : List VarId`),
//! the recorded stratification witness (the Rust side computes ONE
//! witness, `NaiveDb`'s relaxation; the denotation is
//! witness-independent — the recorded narrowing in `Exec/Fixpoint.lean`),
//! and the agreed answers.
//!
//! ## Scope fences (counted in [`ProgramReport`], never silent)
//!
//! * **Folds excluded**: `PRule.finds` is a variable list — the Lean
//!   program cut is projection-shaped (fold-over-recursive coverage is
//!   the naive lane's alone, exactly as `Pack` is on the query side).
//! * **`SQLite` parity asserted where the `WITH RECURSIVE` gate
//!   admits** ([`crate::translate::sqlite_program_expressible`]): an
//!   expressible case is written only after naive and `SQLite` agree
//!   (a disagreement panics — a trophy). Mutual and non-linear cases
//!   are naive-attested and still written: the Lean side judges them
//!   too, which is precisely the coverage `SQLite` cannot give.
//! * The query lane's slow/wide budgets apply unchanged.
//!
//! The corpus programs read the org tree only (`Org`, `OrgParent` —
//! closure sizes bounded by construction, the generator's own rule),
//! asserted per case so the `SQLite` twin world stays two tables.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::time::Instant;

use bumbledb::ir::FindTerm;
use bumbledb::{AtomSource, Program, RelationId, Rule, Term, Value};

use crate::corpus_gen::Rng;
use crate::naive::Tuple;
use crate::querygen::{self, target};
use crate::translate::{Inexpressible, sqlite_program_expressible, translate_program};

use super::{
    MAX_ANSWER_ROWS, NAIVE_BUDGET_MS, World, push_condition, push_fact, push_term, strings_block,
    world_blocks,
};

/// The seeded program-case target (hand cases ride on top).
pub const PROGRAM_SEEDED_CASES: usize = 24;

/// Per-case seed base for the recursive arm — disjoint from the query
/// lane's, recorded in each case's provenance for the replay.
pub const PROGRAM_CASE_SEED_BASE: u64 = 0x0014_0000;

/// The recursive arm's coverage report — every exclusion named and
/// counted (the no-silent-caps rule).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ProgramReport {
    /// Candidate programs attempted.
    pub attempted: u64,
    /// Cases written to the corpus.
    pub written: u64,
    /// Written cases the `SQLite` lane also attested (the rest are
    /// naive-attested: mutual and non-linear shapes).
    pub sqlite_attested: u64,
    /// A fold-bearing program — outside `PRule`'s projection shape.
    pub excluded_fold: u64,
    /// Naive wall time over the query lane's budget.
    pub excluded_slow: u64,
    /// Answer set over the query lane's row cap.
    pub excluded_wide: u64,
}

impl ProgramReport {
    /// The coverage line the builder and comparator log.
    #[must_use]
    pub fn coverage_line(&self) -> String {
        format!(
            "conformance recursive arm: {}/{} written ({} sqlite-attested; excluded: \
             {} fold, {} slow, {} wide)",
            self.written,
            self.attempted,
            self.sqlite_attested,
            self.excluded_fold,
            self.excluded_slow,
            self.excluded_wide,
        )
    }
}

/// The stored relations a program mentions (`Edb` atoms, positive and
/// negated).
fn program_mentioned(program: &Program) -> BTreeSet<RelationId> {
    let mut set = BTreeSet::new();
    for def in &program.predicates {
        for rule in &def.rules {
            for atom in rule.atoms.iter().chain(&rule.negated) {
                if let AtomSource::Edb(relation) = atom.source {
                    set.insert(relation);
                }
            }
        }
    }
    set
}

/// Whether any rule carries a fold — outside the Lean program cut's
/// projection shape (module doc).
fn carries_fold(program: &Program) -> bool {
    program.predicates.iter().any(|def| {
        def.rules.iter().any(|rule| {
            rule.finds.iter().any(|find| {
                matches!(
                    find,
                    FindTerm::Aggregate { .. } | FindTerm::AggregateMeasure { .. }
                )
            })
        })
    })
}

/// Serializes one program rule (`finds` as the plain variable-id list —
/// `PRule.finds : List VarId`).
fn push_rule(
    world: &World,
    used: &mut BTreeSet<u64>,
    out: &mut String,
    rule: &Rule,
) -> Result<(), super::Exclusion> {
    out.push_str("{\"finds\":[");
    for (position, find) in rule.finds.iter().enumerate() {
        if position > 0 {
            out.push(',');
        }
        match find {
            FindTerm::Var(var) => {
                let _ = write!(out, "{}", var.0);
            }
            other => unreachable!("fold-bearing programs are excluded before render: {other:?}"),
        }
    }
    out.push_str("],\"atoms\":[");
    for (position, atom) in rule.atoms.iter().enumerate() {
        if position > 0 {
            out.push(',');
        }
        push_program_atom(world, used, out, atom)?;
    }
    out.push_str("],\"negated\":[");
    for (position, atom) in rule.negated.iter().enumerate() {
        if position > 0 {
            out.push(',');
        }
        push_program_atom(world, used, out, atom)?;
    }
    out.push_str("],\"conditions\":[");
    for (position, tree) in rule.conditions.iter().enumerate() {
        if position > 0 {
            out.push(',');
        }
        push_condition(world, used, out, tree)?;
    }
    out.push_str("]}");
    Ok(())
}

/// Serializes one program atom: the source arm spelled (`edb`/`idb`),
/// bindings as the query lane's `[field, term]` pairs.
fn push_program_atom(
    world: &World,
    used: &mut BTreeSet<u64>,
    out: &mut String,
    atom: &bumbledb::Atom,
) -> Result<(), super::Exclusion> {
    match atom.source {
        AtomSource::Edb(relation) => {
            let _ = write!(out, "{{\"edb\":{},\"bindings\":[", relation.0);
        }
        AtomSource::Idb(pred) => {
            let _ = write!(out, "{{\"idb\":{},\"bindings\":[", pred.0);
        }
    }
    for (index, (field, term)) in atom.bindings.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let _ = write!(out, "[{},", field.0);
        push_term(world, used, out, term)?;
        out.push(']');
    }
    out.push_str("]}");
    Ok(())
}

/// Serializes one full program-case document (module doc: the shared
/// world blocks, the program, the recorded stratification witness, the
/// agreed answers).
fn render_program_case(
    world: &World,
    name: &str,
    provenance: &str,
    program: &Program,
    answers: &BTreeSet<Tuple>,
) -> Result<String, super::Exclusion> {
    let mut used = BTreeSet::new();

    let mut program_block = String::from("{\"predicates\":[\n");
    for (index, def) in program.predicates.iter().enumerate() {
        if index > 0 {
            program_block.push_str(",\n");
        }
        let _ = write!(program_block, "{{\"arity\":{},\"rules\":[", def.head.len());
        program_block.push('\n');
        for (position, rule) in def.rules.iter().enumerate() {
            if position > 0 {
                program_block.push_str(",\n");
            }
            push_rule(world, &mut used, &mut program_block, rule)?;
        }
        program_block.push_str("\n]}");
    }
    let strata = crate::naive::query::model_strata(program);
    let strata_block: Vec<String> = strata.iter().map(ToString::to_string).collect();
    let _ = write!(
        program_block,
        "\n],\"output\":{},\"strata\":[{}]}}",
        program.output.0,
        strata_block.join(",")
    );

    let mut rows: Vec<String> = Vec::with_capacity(answers.len());
    for tuple in answers {
        let mut row = String::new();
        push_fact(world, &mut used, &mut row, &tuple.0, &[])?;
        rows.push(row);
    }
    rows.sort_unstable();
    let answers_block = if rows.is_empty() {
        String::from("[]")
    } else {
        format!("[\n{}\n]", rows.join(",\n"))
    };

    let (relations_block, instance_block, axioms_block) =
        world_blocks(world, &mut used, program_mentioned(program))?;
    let strings_block = strings_block(world, &used);

    Ok(format!(
        "{{\n\"case\":\"{name}\",\n\"provenance\":{provenance},\n\"strings\":{strings_block},\n\
         \"theory\":{{\"relations\":{relations_block},\n\"ground_axioms\":{axioms_block}}},\n\
         \"instance\":{instance_block},\n\"program\":{program_block},\n\"params\":[],\n\
         \"answers\":{answers_block}\n}}\n"
    ))
}

/// One candidate program through the pipeline: naive (timed, budgeted),
/// the `SQLite` twin where the gate admits (agreement asserted — a
/// disagreement is a TROPHY and panics), then the serialized document
/// or the counted exclusion.
///
/// # Panics
///
/// On a naive-vs-`SQLite` disagreement, or a program mentioning
/// relations outside the org tree (the corpus fence, module doc).
fn one_program_case(
    world: &World,
    name: &str,
    provenance: &str,
    program: &Program,
    report: &mut ProgramReport,
) -> Option<String> {
    report.attempted += 1;
    if carries_fold(program) {
        report.excluded_fold += 1;
        return None;
    }
    assert!(
        program_mentioned(program)
            .iter()
            .all(|relation| *relation == target::ids::ORG || *relation == target::ids::ORG_PARENT),
        "program case {name} leaves the org tree — the corpus fence"
    );
    let started = Instant::now();
    let answers = world
        .naive
        .program(program, &[])
        .expect("org-tree programs raise no runtime error");
    let naive_ms = started.elapsed().as_millis();
    if naive_ms > NAIVE_BUDGET_MS {
        report.excluded_slow += 1;
        return None;
    }
    if answers.len() > MAX_ANSWER_ROWS {
        report.excluded_wide += 1;
        return None;
    }
    // The engine leg, mirroring the query and judgment arms: the landed
    // fixpoint driver is held to every corpus case on build AND replay
    // (finding 070 — the lane names itself three-way; now it is).
    let engine = crate::differential::engine_program(&world.db, program, &[]);
    assert_eq!(
        engine,
        crate::differential::Answers::Ok(answers.clone()),
        "TROPHY (engine vs naive) on program case {name}: triage per the fuzzing \
         charter\n{program:#?}"
    );
    match sqlite_program_expressible(program) {
        Ok(()) => {
            let sqlite = sqlite_answers(world, program);
            assert_eq!(
                sqlite, answers,
                "TROPHY (naive vs SQLite) on program case {name}: triage per the fuzzing \
                 charter\n{program:#?}"
            );
            report.sqlite_attested += 1;
        }
        Err(
            Inexpressible::MutualRecursion
            | Inexpressible::NonLinearRecursion
            | Inexpressible::RecursiveFold,
        ) => {}
        Err(other) => unreachable!("program routing hit a judgment class: {other:?}"),
    }
    let document = render_program_case(world, name, provenance, program, &answers)
        .expect("org-tree programs stay inside the format");
    report.written += 1;
    Some(document)
}

/// The `SQLite` twin: the org tables mirrored fresh from the corpus
/// stream, the translated `WITH RECURSIVE` executed.
fn sqlite_answers(world: &World, program: &Program) -> BTreeSet<Tuple> {
    let conn = rusqlite::Connection::open_in_memory().expect("sqlite");
    for statement in crate::sqlmap::schema_ddl(target::schema()) {
        conn.execute(&statement, []).expect("ddl");
    }
    for rel in [target::ids::ORG, target::ids::ORG_PARENT] {
        let relation = target::schema().relation(rel);
        for fact in target::corpus_relation_rows(world.cfg, rel) {
            conn.execute(
                &crate::sqlmap::insert_sql(relation),
                rusqlite::params_from_iter(crate::sqlmap::to_sql_row(&fact)),
            )
            .expect("insert");
        }
    }
    let translated = translate_program(program, target::schema(), &[]).expect("translates");
    let arity = program.predicates[usize::from(program.output.0)].head.len();
    let mut statement = conn.prepare(&translated.sql).expect("prepare");
    let rows = statement
        .query_map([], |row| {
            let mut values = Vec::with_capacity(arity);
            for column in 0..arity {
                let raw: i64 = row.get(column)?;
                values.push(Value::U64(u64::try_from(raw).expect("org ids are small")));
            }
            Ok(Tuple(values))
        })
        .expect("query");
    rows.map(|row| row.expect("row decodes")).collect()
}

/// One hand-picked program case.
struct HandProgram {
    name: &'static str,
    program: Program,
}

/// The hand roster: the ancestor closure whole, negation of the
/// finished closure stratum, and the mutual even/odd pair (naive- and
/// Lean-judged — the coverage `SQLite` cannot give).
fn hand_programs() -> Vec<HandProgram> {
    use bumbledb::{Atom, FieldId, HeadTerm, PredId, PredicateDef, VarId};
    let v = |id: u16| Term::Var(VarId(id));
    let fv = |id: u16| FindTerm::Var(VarId(id));
    let edge = |child: Term, parent: Term| Atom {
        source: AtomSource::Edb(target::ids::ORG_PARENT),
        bindings: vec![
            (target::ids::org_parent::CHILD, child),
            (target::ids::org_parent::PARENT, parent),
        ],
    };
    let idb = |pred: u16, bindings: Vec<(u16, Term)>| Atom {
        source: AtomSource::Idb(PredId(pred)),
        bindings: bindings
            .into_iter()
            .map(|(field, term)| (FieldId(field), term))
            .collect(),
    };
    let rule = |finds: Vec<FindTerm>, atoms: Vec<Atom>, negated: Vec<Atom>| Rule {
        finds,
        atoms,
        negated,
        conditions: vec![],
    };
    let closure = PredicateDef {
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![
            rule(vec![fv(0), fv(1)], vec![edge(v(0), v(1))], vec![]),
            rule(
                vec![fv(0), fv(2)],
                vec![edge(v(0), v(1)), idb(0, vec![(0, v(1)), (1, v(2))])],
                vec![],
            ),
        ],
    };
    vec![
        HandProgram {
            name: "program-hand-closure",
            program: Program {
                predicates: vec![closure.clone()],
                output: PredId(0),
            },
        },
        HandProgram {
            name: "program-hand-unreached",
            program: Program {
                predicates: vec![
                    closure,
                    PredicateDef {
                        head: vec![HeadTerm::Var],
                        rules: vec![rule(
                            vec![fv(0)],
                            vec![Atom {
                                source: AtomSource::Edb(target::ids::ORG),
                                bindings: vec![(target::ids::org::ID, v(0))],
                            }],
                            vec![idb(0, vec![(1, v(0))])],
                        )],
                    },
                ],
                output: PredId(1),
            },
        },
        HandProgram {
            name: "program-hand-mutual",
            program: Program {
                predicates: vec![
                    PredicateDef {
                        head: vec![HeadTerm::Var, HeadTerm::Var],
                        rules: vec![rule(
                            vec![fv(0), fv(2)],
                            vec![edge(v(0), v(1)), idb(1, vec![(0, v(1)), (1, v(2))])],
                            vec![],
                        )],
                    },
                    PredicateDef {
                        head: vec![HeadTerm::Var, HeadTerm::Var],
                        rules: vec![
                            rule(vec![fv(0), fv(1)], vec![edge(v(0), v(1))], vec![]),
                            rule(
                                vec![fv(0), fv(2)],
                                vec![edge(v(0), v(1)), idb(0, vec![(0, v(1)), (1, v(2))])],
                                vec![],
                            ),
                        ],
                    },
                ],
                output: PredId(1),
            },
        },
    ]
}

/// The recursive corpus, deterministically: the hand programs, then
/// seeded generator programs (replayed from `Rng::new(case_seed)`,
/// recorded in provenance) until [`PROGRAM_SEEDED_CASES`] are written.
/// Returns the report and the `(file name, document)` pairs.
///
/// # Panics
///
/// On a naive-vs-`SQLite` trophy ([`one_program_case`]).
#[must_use]
pub fn generate_program_corpus(world: &World) -> (ProgramReport, Vec<(String, String)>) {
    let mut report = ProgramReport::default();
    let mut cases: Vec<(String, String)> = Vec::new();
    for hand in hand_programs() {
        let provenance = format!(
            "{{\"hand\":\"{}\",\"world_seed\":{}}}",
            hand.name, world.cfg.seed
        );
        let document = one_program_case(world, hand.name, &provenance, &hand.program, &mut report)
            .unwrap_or_else(|| panic!("hand program {} must be expressible", hand.name));
        cases.push((format!("{}.json", hand.name), document));
    }
    let mut attempt = 0u64;
    let mut written = 0usize;
    while written < PROGRAM_SEEDED_CASES {
        let case_seed = PROGRAM_CASE_SEED_BASE + attempt;
        attempt += 1;
        let mut rng = Rng::new(case_seed);
        let (program, variant) = querygen::random_program(&mut rng, world.cfg);
        let name = format!("program-seeded-{written:04}");
        let provenance = format!(
            "{{\"world_seed\":{},\"case_seed\":{case_seed},\"variant\":\"{variant:?}\"}}",
            world.cfg.seed
        );
        if let Some(document) = one_program_case(world, &name, &provenance, &program, &mut report) {
            cases.push((format!("{name}.json"), document));
            written += 1;
        }
    }
    (report, cases)
}

/// Regenerates the `program-*.json` cases in place, leaving the query
/// and judgment cases untouched — the recursive arm regenerates
/// independently (its generator and format can move without
/// re-measuring the query lane's wall-clock budgets).
///
/// # Panics
///
/// On filesystem failures, or a naive-vs-`SQLite` trophy.
#[must_use = "the coverage report is the recorded number"]
pub fn write_program_corpus(dir: &std::path::Path) -> ProgramReport {
    let world = super::build_world(super::WORLD_SEEDS[0]);
    let (report, cases) = generate_program_corpus(&world);
    std::fs::create_dir_all(dir).expect("create the corpus directory");
    for entry in std::fs::read_dir(dir).expect("list the corpus directory") {
        let path = entry.expect("corpus dir entry").path();
        let stale = path.extension().is_some_and(|ext| ext == "json")
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("program-"));
        if stale {
            std::fs::remove_file(&path).expect("clear a stale program case");
        }
    }
    for (name, document) in &cases {
        std::fs::write(dir.join(name), document).expect("write a program case");
    }
    report
}

/// One program case's fresh document from its recorded provenance —
/// the replay half ([`super::replay_checked_in_corpus`] dispatches
/// `program-*` files here). Naive re-runs fresh, the `SQLite` parity
/// re-asserts where expressible, and the caller holds the bytes.
pub(super) fn replay_program_case(
    worlds: &mut std::collections::BTreeMap<u64, World>,
    name: &str,
    text: &str,
) -> String {
    let parsed = crate::json::parse(text).expect("a program case parses as JSON");
    let provenance = parsed
        .get("provenance")
        .expect("a program case records provenance");
    let world_seed = super::read_u64(provenance, "world_seed");
    let world = worlds
        .entry(world_seed)
        .or_insert_with(|| super::build_world(world_seed));
    let (program, provenance_line) = if provenance.get("hand").and_then(crate::json::Value::as_str)
        == Some(name)
    {
        let hand = hand_programs()
            .into_iter()
            .find(|hand| hand.name == name)
            .unwrap_or_else(|| panic!("unknown hand program {name}: stale corpus"));
        let line = format!("{{\"hand\":\"{name}\",\"world_seed\":{world_seed}}}");
        (hand.program, line)
    } else {
        let case_seed = super::read_u64(provenance, "case_seed");
        let mut rng = Rng::new(case_seed);
        let (program, variant) = querygen::random_program(&mut rng, world.cfg);
        let line = format!(
            "{{\"world_seed\":{world_seed},\"case_seed\":{case_seed},\"variant\":\"{variant:?}\"}}"
        );
        (program, line)
    };
    let mut report = ProgramReport::default();
    one_program_case(world, name, &provenance_line, &program, &mut report).unwrap_or_else(|| {
        panic!("program case {name}: excluded on replay — stale corpus or trophy")
    })
}
