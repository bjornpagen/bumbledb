use super::digest::digest_over;
use super::read::balance_query;
use super::*;

use crate::gen::{self, Scale, Sizes};
use crate::schema::{ids, schema};
use crate::translate::{goldens, translate};

const CFG: GenConfig = GenConfig {
    seed: 1,
    scale: Scale::S,
};

#[test]
fn all_ten_validate_and_prepare() {
    let dir = std::env::temp_dir().join("bumbledb-bench-families");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let db = bumbledb::Db::create(&dir, schema()).expect("create");
    assert_eq!(all().len(), 10);
    for family in all() {
        db.prepare(&(family.query)())
            .unwrap_or_else(|e| panic!("{} fails validation: {e:?}", family.name));
    }
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// PRD 09 (docs/perf/): the skip-free roster, pinned from real plans
/// (the classification test the PRD orders written FIRST — its output
/// decides which families gate PRD 09 vs PRD 10). The result moved
/// the suite's plan: every skip-free family is a ≤2-node plan whose
/// leaf already runs fused (cross-node batching has no parents to
/// batch), while the deep-node families — triangle, chain, skew,
/// `fk_walk` — all carry D2-crossing nodes and gate PRD 10.
#[test]
fn skip_free_classification_is_pinned() {
    let dir = std::env::temp_dir().join("bumbledb-bench-families-skipfree");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let db = bumbledb::Db::create(&dir, schema()).expect("create");
    let mut seen: Vec<(&str, Option<bool>)> = Vec::new();
    for family in all() {
        let prepared = db.prepare(&(family.query)()).expect("prepares");
        seen.push((family.name, prepared.skip_free()));
    }
    assert_eq!(
        seen,
        vec![
            ("point", None),
            ("fk_walk", Some(false)),
            ("chain", Some(false)),
            ("range", Some(true)),
            ("balance", Some(true)),
            ("stats", Some(true)),
            ("string", Some(true)),
            ("skew", Some(false)),
            ("spread", Some(true)),
            ("triangle", Some(false)),
        ],
        "the skip-free roster and with it the PRD 09/10 gate split"
    );
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// PRD 02 (docs/perf/): the aggregate families' fold regimes, pinned.
/// balance binds the posting serial — distinct bindings proven, the
/// seen-set elided, the constant-group fast path bare. stats binds
/// no unique coverage **by design** (collapsing duplicate
/// (kind, amount, at, instrument) bindings is the family's set
/// semantics), so its dedup pass is semantically required and the
/// batch fold runs the dedup-then-gather arm. A planner change that
/// flips either regime is a semantics bug, not a tuning change.
#[test]
fn aggregate_family_fold_regimes_are_pinned() {
    let dir = std::env::temp_dir().join("bumbledb-bench-families-elide");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let db = bumbledb::Db::create(&dir, schema()).expect("create");
    let mut seen: Vec<(&str, bool)> = Vec::new();
    for family in all() {
        let query = (family.query)();
        if !query
            .finds
            .iter()
            .any(|f| matches!(f, bumbledb::FindTerm::Aggregate { .. }))
        {
            continue;
        }
        let prepared = db.prepare(&query).expect("prepares");
        seen.push((family.name, prepared.distinct_bindings()));
    }
    assert_eq!(
        seen,
        vec![("balance", true), ("stats", false)],
        "the aggregate roster and its fold regimes"
    );
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn every_golden_equals_its_translation() {
    for family in all() {
        let t = translate(&(family.query)(), schema())
            .unwrap_or_else(|e| panic!("{} fails translation: {e}", family.name));
        assert_eq!(t.sql, family.golden_sql, "family {}", family.name);
    }
}

#[test]
fn params_are_deterministic_with_the_documented_misses() {
    let sizes = Sizes::of(CFG.scale);
    for family in all() {
        let a = (family.params)(&CFG);
        let b = (family.params)(&CFG);
        assert_eq!(a, b, "{} params must be seeded", family.name);
        let expected_sets = if matches!(family.name, "stats" | "spread") {
            1
        } else {
            4
        };
        assert_eq!(a.len(), expected_sets, "{}", family.name);
    }
    // The documented misses.
    let point = (all()[0].params)(&CFG);
    let Value::U64(miss) = point[3][0] else {
        panic!("point param")
    };
    assert!(miss >= sizes.postings, "point set 4 is a miss");
    let fk_walk = (all()[1].params)(&CFG);
    let Value::U64(miss) = fk_walk[3][0] else {
        panic!("fk_walk param")
    };
    assert!(miss >= sizes.accounts, "fk_walk set 4 is a miss");
    let string = (all()[6].params)(&CFG);
    let Value::String(raw) = &string[3][0] else {
        panic!("string param")
    };
    assert!(raw.starts_with(b"missing-"), "string set 4 is a miss");
}

#[test]
fn the_digest_tracks_every_ingredient() {
    let baseline = digest();
    assert_eq!(baseline, digest(), "deterministic");
    // Perturb each ingredient of one family on a copy of the items.
    let items = |perturb: usize| {
        all().iter().enumerate().map(move |(i, f)| {
            let mut name = f.name;
            let mut debug = format!("{:?}", (f.query)());
            let mut sql = f.golden_sql;
            if i == 2 {
                match perturb {
                    0 => name = "renamed",
                    1 => debug.push('!'),
                    _ => sql = "SELECT 1",
                }
            }
            (name, debug, sql)
        })
    };
    for perturb in 0..3 {
        assert_ne!(
            digest_over(items(perturb)),
            baseline,
            "perturbation {perturb} must change the digest"
        );
    }
}

/// Estimate honesty over the pinned S corpus (docs/architecture/30-execution.md): with
/// images resident, every family's worst per-node est/actual factor
/// sits under its pin — the "for good" tripwire for the 114,679x
/// dishonesty the first benchmark run measured.
#[test]
fn estimates_are_honest_over_the_pinned_corpus() {
    let dir = std::env::temp_dir().join("bumbledb-bench-honesty");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let db = bumbledb::Db::create(&dir, schema()).expect("create");
    crate::corpus::load_bumbledb(&db, CFG).expect("load");

    let pin = |name: &str| -> f64 {
        match name {
            "point" | "string" | "fk_walk" | "balance" => 16.0,
            // The cyclic family: the closing edge is fully correlated
            // with the opening one, which a per-step fanout model
            // cannot see — the paper's triangle exists precisely
            // because pairwise estimates explode on cycles. The pin
            // documents the class rather than pretending the
            // estimator can beat it (measured 5.2e3 at S).
            "triangle" => 8192.0,
            _ => 64.0,
        }
    };
    // Estimates are per-plan statics: honesty is judged on each
    // family's *typical* param set — an unskewed hit. The hot sets
    // (balance 0, skew 0/1) and the misses measure execution
    // behavior under skew, which no static estimate can or should
    // match.
    let typical = |name: &str| -> usize {
        match name {
            "balance" => 1,
            "skew" => 2,
            _ => 0,
        }
    };
    for family in all() {
        let query = (family.query)();
        let mut prepared = db.prepare(&query).expect("prepare");
        let sets = (family.params)(&CFG);
        // Warm: images + views resident before the measured profile.
        for params in &sets {
            db.read(|snap| snap.execute_collect(&mut prepared, params).map(|_| ()))
                .expect("warm");
        }
        let (_, stats) = db
            .read(|snap| snap.profile(&mut prepared, &sets[typical(family.name)]))
            .expect("profile");
        let mut worst = 1.0_f64;
        #[allow(clippy::cast_precision_loss)]
        for node in &stats.nodes {
            let (est, act) = (node.estimate.max(1) as f64, node.actual.max(1) as f64);
            worst = worst.max((est / act).max(act / est));
        }
        eprintln!("honesty {}: worst factor {worst:.1}", family.name);
        assert!(
            worst <= pin(family.name),
            "{}: worst est/actual factor {worst:.1} exceeds the pin {}\n{stats:?}",
            family.name,
            pin(family.name),
        );
    }
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// PRD 08 (docs/hardening): the balance family is a *true balance* —
/// two equal-amount postings on one account sum to both, engine and
/// translated SQL alike (the pre-rebind query collapsed them into
/// one distinct (account, amount) pair).
/// The minimal consistent slice: one holder/account, two postings of
/// amount 5 on distinct transfers (every FK target present).
fn equal_amount_slice() -> Vec<(bumbledb::RelationId, Vec<Value>)> {
    vec![
        (ids::CURRENCY, vec![Value::U64(0), s("USD")]),
        (ids::HOLDER, vec![Value::U64(0), s("h"), Value::Enum(0)]),
        (
            ids::INSTRUMENT,
            vec![Value::U64(0), s("SYM"), Value::U64(0), Value::Enum(0)],
        ),
        (
            ids::ACCOUNT,
            vec![
                Value::U64(0),
                Value::U64(0),
                Value::U64(0),
                Value::Enum(0),
                Value::I64(0),
            ],
        ),
        (
            ids::TRANSFER,
            vec![
                Value::U64(0),
                Value::I64(0),
                Value::Bytes(vec![0; 16].into()),
            ],
        ),
        (
            ids::TRANSFER,
            vec![
                Value::U64(1),
                Value::I64(1),
                Value::Bytes(vec![1; 16].into()),
            ],
        ),
        (
            ids::POSTING,
            vec![
                Value::U64(0),
                Value::U64(0),
                Value::U64(0),
                Value::U64(0),
                Value::I64(5),
                Value::I64(0),
                s("m"),
                Value::Bool(false),
            ],
        ),
        (
            ids::POSTING,
            vec![
                Value::U64(1),
                Value::U64(1),
                Value::U64(0),
                Value::U64(0),
                Value::I64(5),
                Value::I64(1),
                s("m"),
                Value::Bool(false),
            ],
        ),
    ]
}

#[test]
fn balance_counts_equal_amounts_separately() {
    let rows = equal_amount_slice();
    // Engine side.
    let dir = std::env::temp_dir().join("bumbledb-bench-true-balance");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let db = bumbledb::Db::create(&dir, schema()).expect("create");
    db.write(|tx| {
        for (rel, values) in &rows {
            tx.insert_dyn(*rel, values)?;
        }
        Ok(())
    })
    .expect("seed");
    let mut prepared = db.prepare(&balance_query()).expect("prepare");
    let out = db
        .read(|snap| snap.execute_collect(&mut prepared, &[Value::U64(0)]))
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(
        out.get(0, 1),
        bumbledb::ResultValue::I64(10),
        "both amount-5 postings count"
    );

    // Translated-SQL side, over the identical rows.
    let conn = rusqlite::Connection::open_in_memory().expect("sqlite");
    for statement in crate::sqlmap::ddl(schema()) {
        conn.execute(&statement, []).expect("ddl");
    }
    for (rel, values) in &rows {
        let relation = schema().relation(*rel);
        let placeholders = (1..=relation.fields().len())
            .map(|i| format!("?{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let params: Vec<rusqlite::types::Value> =
            values.iter().map(crate::sqlmap::to_sql_value).collect();
        conn.execute(
            &format!(
                "INSERT INTO \"{}\" VALUES ({placeholders})",
                relation.name()
            ),
            rusqlite::params_from_iter(params),
        )
        .expect("insert");
    }
    let sum: i64 = conn
        .query_row(goldens::BALANCE, [0i64], |row| row.get(1))
        .expect("query");
    assert_eq!(sum, 10, "the golden agrees");

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

fn s(text: &str) -> Value {
    Value::String(text.as_bytes().into())
}

/// The generator attaches the skew params' tags to hot accounts at S.
#[test]
fn skew_tags_are_hot_attached() {
    let sizes = Sizes::of(CFG.scale);
    let hot = sizes.hot_accounts();
    let attached: std::collections::HashSet<u64> = (0..sizes.account_tags)
        .map(|i| gen::account_tag_pair(&sizes, i))
        .filter(|(account, _)| *account < hot)
        .map(|(_, tag)| tag)
        .collect();
    for tag in SKEW_HOT_TAGS {
        assert!(attached.contains(&tag), "tag {tag} not hot-attached");
    }
}

#[test]
fn the_query_list_renders_all_ten_sections() {
    let md = render_queries_md();
    assert!(md.starts_with("# The read query families"));
    for family in all() {
        assert!(
            md.contains(&format!("## {}", family.name)),
            "{}",
            family.name
        );
        assert!(md.contains(family.golden_sql), "{} sql", family.name);
        assert!(md.contains(family.param_policy), "{} policy", family.name);
    }
    assert!(md.contains("Family-list digest: `"));
    assert_eq!(md.matches("```sql").count(), 10);
}
