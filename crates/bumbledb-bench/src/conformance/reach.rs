//! The conformance lane's REACH arm (the shipping law,
//! `docs/architecture/60-validation.md` § the two oracles):
//! `evalQueryList` — interiors then rec lfp then main,
//! `lean/Bumbledb/Exec/Reach.lean` — judges the same Query cases the
//! naive eval and the `SQLite` recursive lane already agree on.
//!
//! One `reach-*.json` case per document (format:
//! `lean/conformance/README.md` § reach cases): the shared
//! theory/instance blocks, the tagged Query (`cq` or `reach`; atoms
//! `edb` / `interior`), and the agreed answers.
//!
//! ## Scope fences (counted in [`ReachReport`], never silent)
//!
//! * **Folds excluded**: Lean reach rules are projection-shaped
//!   (`finds : List VarId`). Fold-over-rec coverage is the naive
//!   lane's alone.
//! * **`SQLite` parity asserted where the translator admits**
//!   ([`crate::translate::sqlite_expressible`]): an expressible
//!   case is written only after naive and `SQLite` agree (a
//!   disagreement panics — a trophy). Interval-typed derived columns
//!   are the remaining translator limit; the generator's rec corpus
//!   is scalar-shaped.
//! * The query lane's slow/wide budgets apply unchanged.
//!
//! The corpus queries read the org tree only (`Org`, `OrgParent`).

use std::collections::BTreeSet;
use std::time::Instant;

use bumbledb::ir::FindTerm;
use bumbledb::{AtomSource, InteriorId, Query, Rec, RelationId, Rule, Term, Value};

use crate::corpus_gen::Rng;
use crate::naive::Tuple;
use crate::querygen::{self, target};
use crate::translate::{Inexpressible, LaneCase, sqlite_expressible, translate};

use super::{MAX_ANSWER_ROWS, NAIVE_BUDGET_MS, World, push_fact, strings_block, world_blocks};

/// The seeded reach-case target (hand cases ride on top).
pub const REACH_SEEDED_CASES: usize = 20;

/// Per-case seed base for the recursive arm — disjoint from the query
/// lane's, recorded in each case's provenance for the replay.
pub const REACH_CASE_SEED_BASE: u64 = 0x0014_0000;

/// The recursive arm's coverage report — every exclusion named and
/// counted (the no-silent-caps rule).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ReachReport {
    /// Candidate queries attempted.
    pub attempted: u64,
    /// Cases written to the corpus.
    pub written: u64,
    /// Written cases the `SQLite` lane also attested.
    pub sqlite_attested: u64,
    /// A fold-bearing query — outside Lean's projection shape.
    pub excluded_fold: u64,
    /// Naive wall time over the query lane's budget.
    pub excluded_slow: u64,
    /// Answer set over the query lane's row cap.
    pub excluded_wide: u64,
}

impl ReachReport {
    /// The coverage line the builder and comparator log.
    #[must_use]
    pub fn coverage_line(&self) -> String {
        format!(
            "conformance reach arm: {}/{} written ({} sqlite-attested; excluded: \
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

/// The stored relations a query mentions (`Edb` atoms, positive and
/// negated).
fn query_mentioned(query: &Query) -> BTreeSet<RelationId> {
    let mut set = BTreeSet::new();
    for rule in crate::walk::rules(query) {
        for atom in rule.atoms.iter().chain(&rule.negated) {
            if let AtomSource::Edb(relation) = atom.source {
                set.insert(relation);
            }
        }
    }
    set
}

/// Whether any rule carries a fold — outside the Lean reach cut's
/// projection shape (module doc).
fn carries_fold(query: &Query) -> bool {
    crate::walk::rules(query).any(|rule| {
        rule.finds.iter().any(|find| {
            matches!(
                find,
                FindTerm::Count
                    | FindTerm::Aggregate { .. }
                    | FindTerm::Pack { .. }
                    | FindTerm::AggregateMeasure { .. }
            )
        })
    })
}

/// Serializes one full reach-case document.
fn render_reach_case(
    world: &World,
    name: &str,
    provenance: &str,
    query: &Query,
    answers: &BTreeSet<Tuple>,
) -> Result<String, super::Exclusion> {
    let mut used = BTreeSet::new();

    let query_block = super::render_reach_query_block(world, &mut used, query)?;

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
        format!("[{}]", rows.join(","))
    };

    let (relations_block, instance_block, axioms_block) =
        world_blocks(world, &mut used, query_mentioned(query))?;
    let strings_block = strings_block(world, &used);

    Ok(format!(
        "{{\n\"case\":\"{name}\",\n\"provenance\":{provenance},\n\"strings\":{strings_block},\n\
         \"theory\":{{\"relations\":{relations_block},\n\"ground_axioms\":{axioms_block}}},\n\
         \"instance\":{instance_block},\n\"query\":{query_block},\n\"params\":[],\n\
         \"answers\":{answers_block}\n}}\n"
    ))
}

/// One candidate query through the pipeline: naive (timed, budgeted),
/// the `SQLite` twin where the translator admits (agreement asserted —
/// a disagreement is a TROPHY and panics), then the serialized
/// document or the counted exclusion.
///
/// # Panics
///
/// On a naive-vs-`SQLite` disagreement, or a query mentioning
/// relations outside the org tree (the corpus fence, module doc).
fn one_reach_case(
    world: &World,
    name: &str,
    provenance: &str,
    query: &Query,
    report: &mut ReachReport,
) -> Option<String> {
    report.attempted += 1;
    if carries_fold(query) {
        report.excluded_fold += 1;
        return None;
    }
    assert!(
        query_mentioned(query)
            .iter()
            .all(|relation| *relation == target::ids::ORG || *relation == target::ids::ORG_PARENT),
        "reach case {name} leaves the org tree — the corpus fence"
    );
    let started = Instant::now();
    let answers = world
        .naive
        .query(query, &[])
        .expect("org-tree queries raise no runtime error");
    let naive_ms = started.elapsed().as_millis();
    if naive_ms > NAIVE_BUDGET_MS {
        report.excluded_slow += 1;
        return None;
    }
    if answers.len() > MAX_ANSWER_ROWS {
        report.excluded_wide += 1;
        return None;
    }
    let engine = crate::differential::engine_query(&world.db, query, &[]);
    assert_eq!(
        engine,
        crate::differential::Answers::Ok(answers.clone()),
        "TROPHY (engine vs naive) on reach case {name}: triage per the fuzzing \
         charter\n{query:#?}"
    );
    match sqlite_expressible(&LaneCase::Query(query)) {
        Ok(()) => {
            let sqlite = sqlite_answers(world, query);
            assert_eq!(
                sqlite, answers,
                "TROPHY (naive vs SQLite) on reach case {name}: triage per the fuzzing \
                 charter\n{query:#?}"
            );
            report.sqlite_attested += 1;
        }
        Err(Inexpressible::IntervalDerivedColumn) => {}
        Err(other) => unreachable!("reach routing hit a judgment class: {other:?}"),
    }
    let document = render_reach_case(world, name, provenance, query, &answers)
        .expect("org-tree queries stay inside the format");
    report.written += 1;
    Some(document)
}

fn sqlite_answers(world: &World, query: &Query) -> BTreeSet<Tuple> {
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
    let translated = translate(query, target::schema(), &[]).expect("translates");
    let arity = query.head().len();
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

struct HandReach {
    name: &'static str,
    query: Query,
}

/// The hand roster: the ancestor closure whole, and negation of the
/// finished rec in main. Mutual is unwritable this cut.
fn hand_queries() -> Vec<HandReach> {
    use bumbledb::{Atom, FieldId, HeadTerm, VarId};
    let v = |id: u16| Term::Var(VarId(id));
    let fv = |id: u16| FindTerm::Var(VarId(id));
    let edge = |child: Term, parent: Term| Atom {
        source: AtomSource::Edb(target::ids::ORG_PARENT),
        bindings: vec![
            (target::ids::org_parent::CHILD, child),
            (target::ids::org_parent::PARENT, parent),
        ],
    };
    let interior = |id: u32, bindings: Vec<(u16, Term)>| Atom {
        source: AtomSource::Interior(InteriorId(id)),
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
    let rec = Rec {
        base: bumbledb::NonEmpty::one(bumbledb::RecRule {
            finds: vec![VarId(0), VarId(1)],
            atoms: vec![edge(v(0), v(1))],
            conditions: vec![],
        }),
        rec: bumbledb::NonEmpty::one(bumbledb::RecStep {
            finds: vec![VarId(0), VarId(2)],
            self_bindings: vec![(FieldId(0), v(1)), (FieldId(1), v(2))],
            atoms: vec![edge(v(0), v(1))],
            conditions: vec![],
        }),
    };
    vec![
        HandReach {
            name: "reach-hand-closure",
            query: Query {
                interiors: vec![],
                rec: Some(rec.clone()),
                head: vec![HeadTerm::Var, HeadTerm::Var],
                rules: vec![rule(
                    vec![fv(0), fv(1)],
                    vec![interior(0, vec![(0, v(0)), (1, v(1))])],
                    vec![],
                )],
            },
        },
        HandReach {
            name: "reach-hand-unreached",
            query: Query {
                interiors: vec![],
                rec: Some(rec),
                head: vec![HeadTerm::Var],
                rules: vec![rule(
                    vec![fv(0)],
                    vec![Atom {
                        source: AtomSource::Edb(target::ids::ORG),
                        bindings: vec![(target::ids::org::ID, v(0))],
                    }],
                    vec![interior(0, vec![(1, v(0))])],
                )],
            },
        },
    ]
}

/// The recursive corpus, deterministically: the hand queries, then
/// seeded generator queries (replayed from `Rng::new(case_seed)`,
/// recorded in provenance) until [`REACH_SEEDED_CASES`] are written.
///
/// # Panics
///
/// On a naive-vs-`SQLite` trophy ([`one_reach_case`]).
#[must_use]
pub fn generate_reach_corpus(world: &World) -> (ReachReport, Vec<(String, String)>) {
    let mut report = ReachReport::default();
    let mut cases: Vec<(String, String)> = Vec::new();
    for hand in hand_queries() {
        let provenance = format!(
            "{{\"hand\":\"{}\",\"world_seed\":{}}}",
            hand.name, world.cfg.seed
        );
        let document = one_reach_case(world, hand.name, &provenance, &hand.query, &mut report)
            .unwrap_or_else(|| panic!("hand query {} must be expressible", hand.name));
        cases.push((format!("{}.json", hand.name), document));
    }
    let mut attempt = 0u64;
    let mut written = 0usize;
    while written < REACH_SEEDED_CASES {
        let case_seed = REACH_CASE_SEED_BASE + attempt;
        attempt += 1;
        let mut rng = Rng::new(case_seed);
        let (query, variant) = querygen::random_reach_query(&mut rng, world.cfg);
        let name = format!("reach-seeded-{written:04}");
        let provenance = format!(
            "{{\"world_seed\":{},\"case_seed\":{case_seed},\"variant\":\"{variant:?}\"}}",
            world.cfg.seed
        );
        if let Some(document) = one_reach_case(world, &name, &provenance, &query, &mut report) {
            cases.push((format!("{name}.json"), document));
            written += 1;
        }
    }
    (report, cases)
}

/// Regenerates the `reach-*.json` cases in place, leaving the query
/// and judgment cases untouched.
///
/// # Panics
///
/// On filesystem failures, or a naive-vs-`SQLite` trophy.
#[must_use = "the coverage report is the recorded number"]
pub fn write_reach_corpus(dir: &std::path::Path) -> ReachReport {
    let world = super::build_world(super::WORLD_SEEDS[0]);
    let (report, cases) = generate_reach_corpus(&world);
    std::fs::create_dir_all(dir).expect("create the corpus directory");
    for entry in std::fs::read_dir(dir).expect("list the corpus directory") {
        let path = entry.expect("corpus dir entry").path();
        let stale = path.extension().is_some_and(|ext| ext == "json")
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("reach-"));
        if stale {
            std::fs::remove_file(&path).expect("clear a stale reach case");
        }
    }
    for (name, document) in &cases {
        std::fs::write(dir.join(name), document).expect("write a reach case");
    }
    report
}

/// One reach case's fresh document from its recorded provenance —
/// the replay half ([`super::replay_checked_in_corpus`] dispatches
/// `reach-*` files here).
pub(super) fn replay_reach_case(
    worlds: &mut std::collections::BTreeMap<u64, World>,
    name: &str,
    text: &str,
) -> String {
    let parsed = crate::json::parse(text).expect("a reach case parses as JSON");
    let provenance = parsed
        .get("provenance")
        .expect("a reach case records provenance");
    let world_seed = super::read_u64(provenance, "world_seed");
    let world = worlds
        .entry(world_seed)
        .or_insert_with(|| super::build_world(world_seed));
    let (query, provenance_line) = if provenance.get("hand").and_then(crate::json::Value::as_str)
        == Some(name)
    {
        let hand = hand_queries()
            .into_iter()
            .find(|hand| hand.name == name)
            .unwrap_or_else(|| panic!("unknown hand reach {name}: stale corpus"));
        let line = format!("{{\"hand\":\"{name}\",\"world_seed\":{world_seed}}}");
        (hand.query, line)
    } else {
        let case_seed = super::read_u64(provenance, "case_seed");
        let mut rng = Rng::new(case_seed);
        let (query, variant) = querygen::random_reach_query(&mut rng, world.cfg);
        let line = format!(
            "{{\"world_seed\":{world_seed},\"case_seed\":{case_seed},\"variant\":\"{variant:?}\"}}"
        );
        (query, line)
    };
    let mut report = ReachReport::default();
    one_reach_case(world, name, &provenance_line, &query, &mut report)
        .unwrap_or_else(|| panic!("reach case {name}: excluded on replay — stale corpus or trophy"))
}
