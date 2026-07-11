use super::digest::digest_over;
use super::read::balance_query;
use super::*;

use crate::gen::{GenConfig, Scale, Sizes};
use crate::naive::ParamValue;
use crate::schema::{ids, schema};
use crate::translate::translate;

const CFG: GenConfig = GenConfig {
    seed: 1,
    scale: Scale::S,
};

/// The set-bound families pin their goldens under a fixed representative
/// set — documented in the param policy, independent of any seed.
fn golden_sets(family: &Family) -> Vec<(ParamId, Vec<Value>)> {
    if family.name == "entries_for_account_set" {
        vec![(
            ParamId(0),
            vec![Value::U64(3), Value::U64(7), Value::U64(9)],
        )]
    } else {
        vec![]
    }
}

#[test]
fn all_fifteen_validate_and_prepare() {
    let dir = std::env::temp_dir().join("bumbledb-bench-families");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let db = bumbledb::Db::create(&dir, crate::schema::Ledger).expect("create");
    assert_eq!(all().len(), 15);
    for family in all() {
        db.prepare(&(family.query)())
            .unwrap_or_else(|e| panic!("{} fails validation: {e:?}", family.name));
    }
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The golden criterion: every family's SQL golden is byte-pinned
/// against the translator (set-bound families under the documented
/// representative set).
#[test]
fn every_golden_equals_its_translation() {
    for family in all() {
        let t = translate(&(family.query)(), schema(), &golden_sets(family))
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
        let expected_sets = match family.name {
            "stats" | "spread" | "latest_posting_per_account" => 1,
            "skew" => 3,
            _ => 4,
        };
        assert_eq!(a.len(), expected_sets, "{}", family.name);
    }
    let draws = |name: &str| {
        (all()
            .iter()
            .find(|f| f.name == name)
            .expect("registered")
            .params)(&CFG)
    };
    // The documented misses.
    let point = draws("point");
    let ParamValue::Scalar(Value::U64(miss)) = point[3][0] else {
        panic!("point param")
    };
    assert!(miss >= sizes.postings, "point set 4 is a miss");
    for name in ["containment_walk", "postings_without_tag"] {
        let sets = draws(name);
        let ParamValue::Scalar(Value::U64(miss)) = sets[3][0] else {
            panic!("{name} param")
        };
        assert!(miss >= sizes.accounts, "{name} set 4 is a miss");
    }
    let string = draws("string");
    let ParamValue::Scalar(Value::String(raw)) = &string[3][0] else {
        panic!("string param")
    };
    assert!(raw.starts_with(b"missing-"), "string set 4 is a miss");
    // The set family's documented sizes: 1, 3 (hot account 0 in), 8, 0.
    let sets = draws("entries_for_account_set");
    let lens: Vec<usize> = sets
        .iter()
        .map(|draw| {
            let ParamValue::Set(values) = &draw[0] else {
                panic!("a set draw")
            };
            values.len()
        })
        .collect();
    assert_eq!(lens, vec![1, 3, 8, 0]);
    let ParamValue::Set(with_hot) = &sets[1][0] else {
        panic!("set draw")
    };
    assert!(with_hot.contains(&Value::U64(0)), "hot account 0 included");
    // The at-instant probes are real posting rows.
    let instants = draws("mandate_at_instant");
    for draw in &instants[..3] {
        assert!(matches!(draw[0], ParamValue::Scalar(Value::U64(_))));
        assert!(matches!(draw[1], ParamValue::Scalar(Value::I64(_))));
    }
    let ParamValue::Scalar(Value::U64(miss)) = instants[3][0] else {
        panic!("at-instant miss")
    };
    assert!(
        miss >= sizes.accounts,
        "at-instant set 4 misses the account"
    );
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

/// The family-owned index registry: deduplicated DDL (chain and range
/// share `idx_posting_at`), and every expected pair has DDL.
#[test]
fn the_index_registry_deduplicates_and_matches() {
    let ddl = index_ddl();
    let expected = expected_indexes();
    assert_eq!(ddl.len(), expected.len());
    for (table, name) in &expected {
        assert!(
            ddl.iter()
                .any(|s| s.contains(&format!("\"{name}\"")) && s.contains(&format!("\"{table}\""))),
            "{name} has no DDL"
        );
    }
    let mut names: Vec<&String> = expected.iter().map(|(_, name)| name).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), expected.len(), "names are distinct");
    // The shared shape appears exactly once.
    assert_eq!(
        ddl.iter().filter(|s| s.contains("idx_posting_at")).count(),
        1
    );
}

/// The doc's interval protocol: the pointwise Mandate key's
/// statement-derived index IS the `(account, active_start, active_end)`
/// composite — the interval families' honest opponent — and the overlap
/// family adds the org-side composite.
#[test]
fn interval_families_get_their_composites() {
    let ddl = crate::sqlmap::ddl(schema());
    assert!(
        ddl.iter().any(|s| s.contains("ON \"Mandate\"")
            && s.contains("(\"account\", \"active_start\", \"active_end\")")),
        "the pointwise key's composite: {ddl:#?}"
    );
    assert!(
        ddl.iter().any(|s| s.contains("idx_mandate_org_active")
            && s.contains("(\"org\", \"active_start\", \"active_end\")")),
        "the overlap family's composite"
    );
}

/// The engine result of the balance family over a two-equal-amount
/// slice: both amount-5 postings count (the true-balance semantics),
/// engine and translated SQL alike.
/// The minimal consistent slice: one holder/account, two postings of
/// amount 5 on distinct entries (every containment target present).
fn equal_amount_slice() -> Vec<(bumbledb::RelationId, Vec<Value>)> {
    vec![
        (ids::HOLDER, vec![Value::U64(0), s("h")]),
        (
            ids::ACCOUNT,
            vec![Value::U64(0), Value::U64(0), Value::Enum(0)],
        ),
        (ids::INSTRUMENT, vec![Value::U64(0), s("SYM")]),
        (
            ids::JOURNAL_ENTRY,
            vec![Value::U64(0), Value::Enum(0), Value::I64(0)],
        ),
        (
            ids::JOURNAL_ENTRY,
            vec![Value::U64(1), Value::Enum(1), Value::I64(1)],
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
    let db = bumbledb::Db::create(&dir, crate::schema::Ledger).expect("create");
    db.write(|tx| {
        for (rel, values) in &rows {
            tx.insert_dyn(*rel, values)?;
        }
        Ok(())
    })
    .expect("seed");
    let mut prepared = db.prepare(&balance_query()).expect("prepare");
    let out = db
        .read(|snap| snap.execute_collect(&mut prepared, &[bumbledb::BindValue::U64(0)]))
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
        conn.execute(
            &crate::sqlmap::insert_sql(relation),
            rusqlite::params_from_iter(crate::sqlmap::to_sql_row(values)),
        )
        .expect("insert");
    }
    let sum: i64 = conn
        .query_row(crate::translate::goldens::BALANCE, [0i64], |row| row.get(1))
        .expect("query");
    assert_eq!(sum, 10, "the golden agrees");

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

fn s(text: &str) -> Value {
    Value::String(text.as_bytes().into())
}

#[test]
fn the_query_list_renders_every_section_of_both_theories() {
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
    assert!(md.contains("# The calendar query families"));
    for family in crate::calendar::families::all() {
        assert!(
            md.contains(&format!("## {}", family.name)),
            "{}",
            family.name
        );
        assert!(md.contains(family.golden_sql), "{} sql", family.name);
        assert!(md.contains(family.param_policy), "{} policy", family.name);
    }
    assert_eq!(md.matches("Family-list digest: `").count(), 2);
    // 15 ledger + 7 calendar sections, one ```sql block each.
    assert_eq!(
        md.matches("```sql").count(),
        all().len() + crate::calendar::families::all().len()
    );
}
