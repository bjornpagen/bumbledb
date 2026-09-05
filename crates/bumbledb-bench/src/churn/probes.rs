use bumbledb::{
    Answers, Atom, AtomSource, CmpOp, Comparison, ConditionTree, Db, FindTerm, FoldOp, ParamId,
    Query, Rule, Term, Value, VarId,
};

use crate::compare;
use crate::corpus_gen::{self, GenConfig, Rng, Sizes};
use crate::families::{Draw, param_args, scalar_draw};
use crate::fixture::var;
use crate::harness::{self, Modes, Protocol, Rotation};
use crate::schema::{Ledger, ids};
use crate::sqlite_run;

use super::ops;

pub struct Probe {
    pub name: &'static str,
    pub query: fn() -> Query,
    pub about: &'static str,
}

fn param(id: u16) -> Term {
    Term::Param(ParamId(id))
}

fn point_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: AtomSource::Edb(ids::POSTING),
            bindings: vec![
                (ids::posting::ID, param(0)),
                (ids::posting::AMOUNT, var(0)),
                (ids::posting::AT, var(1)),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn balance_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Aggregate {
            op: FoldOp::Sum,
            over: VarId(1),
        }],
        atoms: vec![Atom {
            source: AtomSource::Edb(ids::POSTING),
            bindings: vec![
                (ids::posting::ID, var(2)),
                (ids::posting::ACCOUNT, param(0)),
                (ids::posting::AMOUNT, var(1)),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn window_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: AtomSource::Edb(ids::POSTING),
            bindings: vec![
                (ids::posting::ID, var(0)),
                (ids::posting::AMOUNT, var(1)),
                (ids::posting::AT, var(2)),
            ],
        }],
        negated: vec![],
        conditions: vec![
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Ge,
                lhs: var(2),
                rhs: param(0),
            }),
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Lt,
                lhs: var(2),
                rhs: param(1),
            }),
        ],
    })
}

#[must_use]
pub fn all() -> &'static [Probe] {
    &[
        Probe {
            name: "churn_point",
            query: point_query,
            about: "point read of one LIVE posting — O(1) cost as the id space burns monotonically",
        },
        Probe {
            name: "churn_balance",
            query: balance_query,
            about: "one account's balance fold — the aggregate path over the churned mass",
        },
        Probe {
            name: "churn_window",
            query: window_query,
            about: "a fixed ≈2% time window scan — stationary selectivity by the body law",
        },
    ]
}

/// The probe protocol — small on purpose: a sample point is a curve pixel, not
/// a gate; 16 exact-percentile samples after warmups that
pub const PROBE_PROTO: Protocol = Protocol {
    warmups: 4,
    samples: 16,
};

/// # Panics
#[must_use]
pub fn draws(probe: &Probe, r#gen: GenConfig, live: &ops::LiveSet, cycle: u64) -> Vec<Draw> {
    match probe.name {
        "churn_point" => {
            let mut rng = Rng::new(r#gen.seed ^ 0xC10C_0002 ^ cycle.rotate_left(23));
            let len = u64::try_from(live.len()).expect("fits u64");
            (0..4)
                .map(|_| {
                    let index = usize::try_from(rng.range(len)).expect("64-bit usize");
                    scalar_draw(vec![Value::U64(live.rows()[index].id.0)])
                })
                .collect()
        }
        "churn_balance" => vec![scalar_draw(vec![Value::U64(0)])],
        "churn_window" => {
            let span =
                i64::try_from(Sizes::of(r#gen.scale).postings).expect("fits") * corpus_gen::AT_STEP;
            let start = corpus_gen::AT_BASE + span * 5 / 16;
            let end = start + span / 50;
            vec![scalar_draw(vec![Value::I64(start), Value::I64(end)])]
        }
        _ => unreachable!("three pinned probes"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeSample {
    pub name: String,

    pub p50_ns: u64,

    pub answers: u64,
}

#[derive(Debug, Clone)]
pub struct ProbeRun {
    pub sample: ProbeSample,

    pub reference: Vec<Vec<compare::Answer>>,

    pub types: Vec<bumbledb::schema::ValueType>,
}

/// The churn probe protocol:
/// # Errors
pub fn sample_ours(db: &Db<Ledger>, probe: &Probe, sets: &[Draw]) -> Result<ProbeRun, String> {
    let query = (probe.query)();
    let mut prepared = db
        .prepare(&query)
        .map_err(|e| format!("{}: prepare: {e:?}", probe.name))?;
    let types: Vec<bumbledb::schema::ValueType> = prepared
        .signature()
        .columns
        .iter()
        .map(|column| *column.ty())
        .collect();
    let mut buffer = Answers::new();
    let mut reference = Vec::with_capacity(sets.len());
    for draw in sets {
        let args = param_args(draw);
        db.read(|snap| snap.execute(&mut prepared, &args, &mut buffer))
            .map_err(|e| format!("{}: execute: {e:?}", probe.name))?;
        reference.push(compare::from_answers(&buffer, &types));
    }
    let mut rotation = Rotation::new(sets.to_vec());
    let mut run = || {
        let args = param_args(rotation.next_set());
        db.read(|snap| snap.execute(&mut prepared, &args, &mut buffer))
            .map_err(|e| format!("execute: {e:?}"))?;
        Ok(buffer.len() as u64)
    };
    let plain = harness::measure(PROBE_PROTO, &mut run)?;
    let answers = plain.work / u64::from(PROBE_PROTO.samples);
    let p50_ns = if plain.stats.p50 < harness::QUANTUM_FLOOR_NS {
        harness::measure_batched(PROBE_PROTO, Modes::default(), 16, &mut run)?
            .stats
            .p50
    } else {
        plain.stats.p50
    };
    Ok(ProbeRun {
        sample: ProbeSample {
            name: probe.name.to_owned(),
            p50_ns,
            answers,
        },
        reference,
        types,
    })
}

/// result multiset must equal the ours-side reference before a single protocol
/// — see [`sample_ours`]), with the same quantum-floor
/// # Errors
/// # Panics
pub fn sample_sqlite(
    conn: &rusqlite::Connection,
    probe: &Probe,
    sets: &[Draw],
    run: &ProbeRun,
    lane: &str,
) -> Result<ProbeSample, String> {
    let query = (probe.query)();
    let translated = crate::translate::translate(&query, crate::schema::schema(), &[])
        .map_err(|e| format!("{}: translate: {e}", probe.name))?;
    assert_eq!(
        run.reference.len(),
        sets.len(),
        "the gate's carrier holds one reference multiset per draw"
    );
    for (index, draw) in sets.iter().enumerate() {
        let mut stmt = conn
            .prepare(&translated.sql)
            .map_err(|e| format!("{lane}/{}: mirror prepare: {e}", probe.name))?;
        let theirs = compare::from_sqlite(&mut stmt, &translated.params, draw, &run.types)
            .map_err(|e| format!("{lane}/{}: mirror execute: {e}", probe.name))?;
        compare::multisets(run.reference[index].clone(), theirs).map_err(|mismatch| {
            format!(
                "{lane}/{} draw {index}: ENGINES DISAGREE — not timing a wrong answer\n{mismatch}",
                probe.name
            )
        })?;
    }
    let mut mirror = sqlite_run::PreparedFamily::new(conn, &translated, run.types.clone())?;
    let mut cursor = 0usize;
    let mut sample = || {
        let index = cursor;
        cursor = (cursor + 1) % sets.len();
        sqlite_run::sample_args(&mut mirror, &sets[index])
    };
    let plain = harness::measure(PROBE_PROTO, &mut sample)?;
    let answers = plain.work / u64::from(PROBE_PROTO.samples);
    let p50_ns = if plain.stats.p50 < harness::QUANTUM_FLOOR_NS {
        harness::measure_batched(PROBE_PROTO, Modes::default(), 16, &mut sample)?
            .stats
            .p50
    } else {
        plain.stats.p50
    };
    Ok(ProbeSample {
        name: probe.name.to_owned(),
        p50_ns,
        answers,
    })
}

#[cfg(test)]
mod tests {
    use crate::churn::engines::{self, OursLane};
    use crate::corpus_gen::{GenConfig, Scale};
    use crate::naive::ParamValue;
    use crate::storemode::StoreMode;

    use super::*;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("bumbledb-bench-churn-probes-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn tiny() -> GenConfig {
        GenConfig {
            seed: 1,
            scale: Scale::Tiny,
        }
    }

    fn fresh_pair(
        tag: &str,
    ) -> (
        OursLane,
        rusqlite::Connection,
        ops::LiveSet,
        std::path::PathBuf,
    ) {
        let dir = scratch(tag);
        let lane =
            engines::create_ours(&dir.join("ours"), tiny(), StoreMode::Durable).expect("ours lane");
        let conn = engines::create_sqlite(&dir.join("mirror.sqlite"), tiny()).expect("mirror");
        let live = ops::LiveSet::from_corpus(tiny());
        (lane, conn, live, dir)
    }

    #[test]
    fn churn_probes_gate_and_sample_on_a_fresh_tiny_pair() {
        let (lane, conn, live, dir) = fresh_pair("fresh");
        for probe in all() {
            let sets = draws(probe, tiny(), &live, 0);
            let run = sample_ours(&lane.db, probe, &sets).expect("ours samples");
            let theirs =
                sample_sqlite(&conn, probe, &sets, &run, "sqlite-bare").expect("mirror samples");
            assert!(run.sample.p50_ns > 0, "{}: ours p50 > 0", probe.name);
            assert!(theirs.p50_ns > 0, "{}: mirror p50 > 0", probe.name);
            match probe.name {
                "churn_point" => {
                    assert_eq!(run.sample.answers, 1, "a live id hits exactly one row");
                }
                "churn_window" => {
                    assert!(run.sample.answers > 0, "the fixed window is nonempty");
                }
                _ => {}
            }
        }
        drop((lane, conn));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn churn_probe_draws_are_deterministic_and_live() {
        let live = ops::LiveSet::from_corpus(tiny());
        let point = &all()[0];
        let first = draws(point, tiny(), &live, 5);
        assert_eq!(
            first,
            draws(point, tiny(), &live, 5),
            "point draws are a pure function of (seed, cycle, live set)"
        );
        let ids: std::collections::BTreeSet<u64> =
            live.rows().iter().map(|posting| posting.id.0).collect();
        for draw in &first {
            let ParamValue::Scalar(Value::U64(id)) = &draw[0] else {
                panic!("a point draw is one scalar u64 id");
            };
            assert!(ids.contains(id), "drawn id {id} is live");
        }
        for probe in &all()[1..] {
            assert_eq!(
                draws(probe, tiny(), &live, 5),
                draws(probe, tiny(), &live, 99),
                "{}: the FIXED-param law — identical draws at every cycle",
                probe.name
            );
        }
    }

    #[test]
    fn churn_probe_gate_catches_a_divergent_mirror() {
        let (lane, conn, live, dir) = fresh_pair("divergent");
        let window = &all()[2];
        let sets = draws(window, tiny(), &live, 0);
        let run = sample_ours(&lane.db, window, &sets).expect("ours samples");
        conn.execute("DELETE FROM \"Posting\" WHERE \"id\" = 330", [])
            .expect("the divergence injects");
        let refusal = sample_sqlite(&conn, window, &sets, &run, "sqlite-bare")
            .expect_err("the gate must refuse a divergent mirror");
        assert!(refusal.contains("ENGINES DISAGREE"), "{refusal}");
        drop((lane, conn));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn churn_probe_registry_is_fixed() {
        assert_eq!(all().len(), 3, "exactly three pinned probes");
        let names: Vec<&str> = all().iter().map(|probe| probe.name).collect();
        assert_eq!(names, ["churn_point", "churn_balance", "churn_window"]);
    }
}
