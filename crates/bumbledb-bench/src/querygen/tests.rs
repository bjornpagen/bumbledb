use super::oracle::{param_anchors, u64_domain};
use super::*;
use bumbledb::Value;

use crate::gen::{GenConfig, Rng, Scale, Sizes};
use crate::schema::schema;
use crate::translate::translate;

const SEED: u64 = 11;
const N: u64 = 1000;

const CFG: GenConfig = GenConfig {
    seed: 1,
    scale: Scale::S,
};

/// Every generated query passes the engine's validate (via prepare on
/// an empty schema-loaded db) AND translates to SQL.
#[test]
fn a_thousand_queries_validate_and_translate() {
    let dir = std::env::temp_dir().join("bumbledb-bench-querygen");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let db = bumbledb::Db::create(&dir, schema()).expect("create");
    let mut rng = Rng::new(SEED);
    for i in 0..N {
        let query = random_query(&mut rng, CFG);
        if let Err(error) = db.prepare(&query) {
            panic!("query {i} fails validation: {error:?}\n{query:#?}");
        }
        if let Err(error) = translate(&query, schema()) {
            panic!("query {i} fails translation: {error}\n{query:#?}");
        }
    }
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Every construct appears at n = 1000, every *legal* cell of the
/// per-(op, type) comparison matrix is nonzero, and shape counts sit
/// within ±30% of their weight expectations (weight regressions
/// surface). This is 50-validation's coverage contract, asserted.
#[test]
fn the_coverage_contract_holds_at_a_thousand() {
    let cov = coverage(N, SEED, CFG);
    let band = |count: u64, weight: u64| {
        let expected = N * weight / 90;
        assert!(
            count * 10 >= expected * 7 && count * 10 <= expected * 13,
            "count {count} outside ±30% of {expected}"
        );
    };
    band(cov.guard, 10);
    band(cov.star, 20);
    band(cov.chain, 20);
    band(cov.self_join, 10);
    band(cov.gated, 10);
    band(cov.aggregate, 20);
    for (name, count) in [
        ("gates", cov.gates),
        ("misses", cov.misses),
        ("params", cov.params),
        ("repeated_vars", cov.repeated_vars),
        ("agg_sum", cov.agg_sum),
        ("agg_min", cov.agg_min),
        ("agg_max", cov.agg_max),
        ("agg_count", cov.agg_count),
        ("agg_u64", cov.agg_u64),
        ("multi_aggregate", cov.multi_aggregate),
        ("cross_residuals", cov.cross_residuals),
        ("bytes_hits", cov.bytes_hits),
        ("bytes_misses", cov.bytes_misses),
    ] {
        assert!(count > 0, "{name} never generated");
    }
    for (op_idx, op) in CMP_OPS.iter().enumerate() {
        for (type_idx, ty) in CMP_TYPES.iter().enumerate() {
            let count = cov.matrix[op_idx][type_idx];
            if cmp_cell_legal(op_idx, type_idx) {
                assert!(count > 0, "({op:?}, {ty}) never generated");
            } else {
                assert_eq!(count, 0, "({op:?}, {ty}) is illegal by the roster");
            }
        }
    }
}

/// PRD 07 (docs/hardening): the grammar never emits a NUL — the
/// translator rejects NUL string literals by name, and this property
/// keeps that boundary unreachable from generated queries.
#[test]
fn generated_string_literals_are_nul_free() {
    let mut rng = Rng::new(SEED);
    for _ in 0..N {
        let query = random_query(&mut rng, CFG);
        for atom in &query.atoms {
            for (_, term) in &atom.bindings {
                if let bumbledb::Term::Literal(bumbledb::Value::String(raw)) = term {
                    assert!(!raw.contains(&0), "a generated literal carries NUL");
                }
            }
        }
        for comparison in &query.predicates {
            for term in [&comparison.lhs, &comparison.rhs] {
                if let bumbledb::Term::Literal(bumbledb::Value::String(raw)) = term {
                    assert!(!raw.contains(&0), "a generated literal carries NUL");
                }
            }
        }
    }
}

/// Same seed ⇒ identical query stream (pinned on #500's rendering).
#[test]
fn generation_is_deterministic() {
    let query_500 = |seed| {
        let mut rng = Rng::new(seed);
        let mut last = None;
        for _ in 0..=500 {
            last = Some(random_query(&mut rng, CFG));
        }
        format!("{:?}", last.expect("generated"))
    };
    assert_eq!(query_500(SEED), query_500(SEED));
    assert_ne!(query_500(SEED), query_500(SEED + 1));
}

/// Four sets, with every string, bytes, and u64 param a guaranteed
/// miss in the last (out of vocabulary or out of domain).
#[test]
fn params_for_produces_the_documented_sets() {
    let mut rng = Rng::new(SEED);
    let sizes = Sizes::of(CFG.scale);
    let (mut saw_string, mut saw_u64, mut saw_bytes) = (false, false, false);
    for _ in 0..200 {
        let query = random_query(&mut rng, CFG);
        let sets = params_for(&query, &mut rng, CFG);
        assert_eq!(sets.len(), 4);
        let anchors = param_anchors(&query);
        for set in &sets {
            assert_eq!(set.len(), anchors.len());
        }
        for (value, anchor) in sets[3].iter().zip(&anchors) {
            match value {
                Value::String(raw) => {
                    saw_string = true;
                    assert!(
                        raw.starts_with(b"missing-"),
                        "set 3 string params are guaranteed misses"
                    );
                }
                Value::U64(v) => {
                    saw_u64 = true;
                    let domain = u64_domain(anchor.0, anchor.1, &sizes);
                    assert!(*v > domain, "set 3 u64 params are out of domain");
                }
                Value::Bytes(raw) => {
                    saw_bytes = true;
                    assert_eq!(raw.len(), 16, "a fresh 16-byte miss value");
                }
                _ => {}
            }
        }
    }
    assert!(saw_string, "the batch produced string params");
    assert!(saw_u64, "the batch produced u64 params");
    assert!(saw_bytes, "the batch produced bytes params");
}
