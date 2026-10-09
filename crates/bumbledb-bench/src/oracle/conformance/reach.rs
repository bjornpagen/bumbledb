//! Recursive-query fixtures are emitted after native execution and the
//! independent evaluators agree on the canonical answer set.
use std::collections::BTreeSet;

use bumbledb::ir::FindTerm;
use bumbledb::{AtomSource, InteriorId, Query, Rec, RelationId, Rule, Term, Value};

use crate::oracle::naive::Tuple;
use crate::oracle::querygen::{self, target};
use crate::oracle::sqlite::translate::{Inexpressible, LaneCase, sqlite_expressible, translate};
use crate::worlds::corpus_gen::Rng;

use super::{
    MAX_ANSWER_ROWS, NAIVE_BUDGET, SeededCase, World, push_fact, strings_block, world_blocks,
};

pub const REACH_SEEDED_CASES: usize = 20;

pub const REACH_CASE_SEED_BASE: u64 = 0x0014_0000;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ReachReport {
    pub attempted: u64,

    pub written: u64,

    pub sqlite_attested: u64,

    pub excluded_fold: u64,

    pub excluded_over_budget: u64,

    pub excluded_wide: u64,
}

impl ReachReport {
    #[must_use]
    pub fn coverage_line(&self) -> String {
        format!(
            "conformance reach arm: {}/{} written ({} sqlite-attested; excluded: \
             {} fold, {} over-budget, {} wide)",
            self.written,
            self.attempted,
            self.sqlite_attested,
            self.excluded_fold,
            self.excluded_over_budget,
            self.excluded_wide,
        )
    }
}

fn query_mentioned(query: &Query) -> BTreeSet<RelationId> {
    let mut set = BTreeSet::new();
    for rule in crate::oracle::walk::rules(query) {
        for atom in rule.atoms.iter().chain(&rule.negated) {
            if let AtomSource::Edb(relation) = atom.source {
                set.insert(relation);
            }
        }
    }
    set
}

fn carries_fold(query: &Query) -> bool {
    crate::oracle::walk::rules(query).any(|rule| {
        rule.finds.iter().any(|find| {
            matches!(
                find,
                FindTerm::Count | FindTerm::Aggregate { .. } | FindTerm::Pack { .. }
            )
        })
    })
}

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
        "reach case {name} leaves the org tree"
    );
    let Ok(answers) = world.naive.query_within(query, &[], NAIVE_BUDGET) else {
        report.excluded_over_budget += 1;
        return None;
    };
    let answers = answers.expect("org-tree queries raise no runtime error");
    if answers.len() > MAX_ANSWER_ROWS {
        report.excluded_wide += 1;
        return None;
    }
    let engine = crate::oracle::differential::engine_query(&world.db, query, &[]);
    assert_eq!(
        engine,
        crate::oracle::differential::Answers::Ok(answers.clone()),
        "engine and naive disagree on reach case {name}\n{query:#?}"
    );
    match sqlite_expressible(&LaneCase::Query(query)) {
        Ok(()) => {
            let sqlite = sqlite_answers(world, query);
            assert_eq!(
                sqlite, answers,
                "naive and SQLite disagree on reach case {name}\n{query:#?}"
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
    for statement in crate::oracle::sqlite::sqlmap::schema_ddl(target::schema()) {
        conn.execute(&statement, []).expect("ddl");
    }
    for rel in [target::ids::ORG, target::ids::ORG_PARENT] {
        let relation = target::schema().relation(rel);
        for fact in target::corpus_relation_rows(world.cfg, rel) {
            conn.execute(
                &crate::oracle::sqlite::sqlmap::insert_sql(relation),
                rusqlite::params_from_iter(crate::oracle::sqlite::sqlmap::to_sql_row(&fact)),
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

/// The curated recursive cases, in fixture order.
/// # Panics
/// When a curated case falls outside the format or the budget.
#[must_use]
pub fn hand_reach_corpus(world: &World) -> Vec<(String, String)> {
    let mut report = ReachReport::default();
    hand_queries()
        .into_iter()
        .map(|hand| {
            let provenance = format!(
                "{{\"hand\":\"{}\",\"world_seed\":{}}}",
                hand.name, world.cfg.seed
            );
            let document = one_reach_case(world, hand.name, &provenance, &hand.query, &mut report)
                .unwrap_or_else(|| panic!("hand query {} must be expressible", hand.name));
            (format!("{}.json", hand.name), document)
        })
        .collect()
}

/// The first [`REACH_SEEDED_CASES`] expressible random recursive queries.
#[must_use]
pub fn seeded_reach_corpus(world: &World) -> (ReachReport, Vec<SeededCase>) {
    let mut report = ReachReport::default();
    let mut cases = Vec::with_capacity(REACH_SEEDED_CASES);
    let mut attempt = 0u64;
    while cases.len() < REACH_SEEDED_CASES {
        let case_seed = REACH_CASE_SEED_BASE + attempt;
        attempt += 1;
        let mut rng = Rng::new(case_seed);
        let (query, variant) = querygen::random_reach_query(&mut rng, world.cfg);
        let name = format!("reach-seeded-{:04}", cases.len());
        let provenance = format!(
            "{{\"world_seed\":{},\"case_seed\":{case_seed},\"variant\":\"{variant:?}\"}}",
            world.cfg.seed
        );
        if let Some(document) = one_reach_case(world, &name, &provenance, &query, &mut report) {
            cases.push(SeededCase {
                name,
                case_seed,
                document,
            });
        }
    }
    (report, cases)
}

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
    let hand = hand_queries()
        .into_iter()
        .find(|hand| hand.name == name)
        .unwrap_or_else(|| panic!("unknown hand reach case {name}"));
    let provenance_line = format!("{{\"hand\":\"{name}\",\"world_seed\":{world_seed}}}");
    let mut report = ReachReport::default();
    one_reach_case(world, name, &provenance_line, &hand.query, &mut report)
        .unwrap_or_else(|| panic!("reach case {name}: excluded on replay"))
}
