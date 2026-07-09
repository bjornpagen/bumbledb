use super::oracle::{param_anchors, u64_domain, LARGE_BOUNDARY};
use super::target::{self, Domains};
use super::*;
use bumbledb::Value;

use crate::gen::{GenConfig, Rng, Scale};
use crate::translate::translate;

const SEED: u64 = 11;
const N: u64 = 1000;

const CFG: GenConfig = GenConfig {
    seed: 1,
    scale: Scale::S,
};

/// Every generated query passes the engine's validate (via prepare on
/// an empty schema-loaded db) AND translates to SQL — under every param
/// draw (set-bound queries re-render per draw; the empty set takes the
/// `1 = 0` path).
#[test]
fn a_thousand_queries_validate_and_translate() {
    let dir = std::env::temp_dir().join("bumbledb-bench-querygen");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let db = bumbledb::Db::create(&dir, target::schema()).expect("create");
    let mut rng = Rng::new(SEED);
    for i in 0..N {
        let query = random_query(&mut rng, CFG);
        if let Err(error) = db.prepare(&query) {
            panic!("query {i} fails validation: {error:?}\n{query:#?}");
        }
        for draw in params_for(&query, &mut rng, CFG) {
            if let Err(error) = translate(&query, target::schema(), &draw.sets) {
                panic!("query {i} fails translation: {error}\n{query:#?}");
            }
        }
    }
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The coverage contract (`docs/architecture/60-validation.md`, the
/// exact asserted form): every shape within ±30% of its weight, every
/// *legal* cell of the per-(op, type) comparison matrix nonzero and
/// every illegal cell zero, every construct present — negation shapes,
/// param-set sizes, membership kinds, both interval element lanes, the
/// adjacent-touching boundary in both polarities, `CountDistinct` over
/// every type, Arg-restriction variants — and the structural
/// compositions at least once per run.
#[test]
fn the_coverage_contract_holds_at_a_thousand() {
    let cov = coverage(N, SEED, CFG);
    let total: u64 = SHAPE_WEIGHTS.iter().map(|(_, w)| w).sum();
    let band = |name: &str, count: u64, weight: u64| {
        let expected = N * weight / total;
        assert!(
            count * 10 >= expected * 7 && count * 10 <= expected * 13,
            "{name}: count {count} outside ±30% of {expected}"
        );
    };
    band("guard", cov.guard, 10);
    band("star", cov.star, 15);
    band("chain", cov.chain, 15);
    band("self_join", cov.self_join, 8);
    band("gated", cov.gated, 8);
    band("aggregate", cov.aggregate, 14);
    band("membership", cov.membership, 10);
    band("interval_join", cov.interval_join, 8);
    band("boundary", cov.boundary, 4);
    band("count_distinct", cov.count_distinct, 10);
    band("arg", cov.arg, 8);
    for (name, count) in [
        ("gates", cov.gates),
        ("misses", cov.misses),
        ("params", cov.params),
        ("param_sets", cov.param_sets),
        ("repeated_vars", cov.repeated_vars),
        ("agg_sum", cov.agg_sum),
        ("agg_min", cov.agg_min),
        ("agg_max", cov.agg_max),
        ("agg_count", cov.agg_count),
        ("agg_u64", cov.agg_u64),
        ("multi_aggregate", cov.multi_aggregate),
        ("arg_max", cov.arg_max),
        ("arg_min", cov.arg_min),
        ("arg_key_projected", cov.arg_key_projected),
        ("arg_global", cov.arg_global),
        ("arg_tie_key", cov.arg_tie_key),
        ("arg_tie_free_key", cov.arg_tie_free_key),
        ("membership_literal", cov.membership_literal),
        ("membership_param", cov.membership_param),
        ("membership_var", cov.membership_var),
        ("membership_u64", cov.membership_u64),
        ("membership_i64", cov.membership_i64),
        ("overlaps_u64", cov.overlaps_u64),
        ("overlaps_i64", cov.overlaps_i64),
        ("contains_u64", cov.contains_u64),
        ("contains_i64", cov.contains_i64),
        ("contains_element", cov.contains_element),
        ("adjacent_left", cov.adjacent_left),
        ("adjacent_right", cov.adjacent_right),
        ("negations", cov.negations),
        ("negation_key_covered", cov.negation_key_covered),
        ("negation_open", cov.negation_open),
        ("negation_literal", cov.negation_literal),
        ("negation_param", cov.negation_param),
        ("negation_set", cov.negation_set),
        ("negation_membership", cov.negation_membership),
        ("negation_gate", cov.negation_gate),
        ("negation_multi_witness", cov.negation_multi_witness),
        ("cross_residuals", cov.cross_residuals),
        ("bytes_hits", cov.bytes_hits),
        ("bytes_misses", cov.bytes_misses),
    ] {
        assert!(count > 0, "{name} never generated");
    }
    for (index, name) in CMP_TYPES.iter().enumerate() {
        assert!(
            cov.count_distinct_types[index] > 0,
            "CountDistinct over {name} never generated"
        );
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
    // The structural compositions where bugs hide: at least one query
    // per run carries each.
    assert!(cov.neg_and_aggregate > 0, "negation ∧ aggregate missing");
    assert!(cov.set_and_negation > 0, "param set ∧ negation missing");
    assert!(
        cov.membership_and_overlaps > 0,
        "membership ∧ Overlaps missing"
    );
}

/// The grammar never emits a NUL — the translator rejects NUL string
/// literals by name, and this property keeps that boundary unreachable
/// from generated queries.
#[test]
fn generated_string_literals_are_nul_free() {
    let mut rng = Rng::new(SEED);
    for _ in 0..N {
        let query = random_query(&mut rng, CFG);
        for atom in query.atoms.iter().chain(&query.negated) {
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

/// Same seed ⇒ identical query stream AND identical param draws — the
/// reproducibility property the oracle protocol depends on (pinned on
/// #500's rendering).
#[test]
fn generation_is_deterministic() {
    let stream_500 = |seed| {
        let mut rng = Rng::new(seed);
        let mut last = None;
        for _ in 0..=500 {
            let query = random_query(&mut rng, CFG);
            let draws = params_for(&query, &mut rng, CFG);
            last = Some((query, draws));
        }
        format!("{:?}", last.expect("generated"))
    };
    assert_eq!(stream_500(SEED), stream_500(SEED));
    assert_ne!(stream_500(SEED), stream_500(SEED + 1));
}

/// Four draws per query; the miss draw's string, bytes, and u64 params
/// — scalar or set element — are guaranteed misses (out of vocabulary
/// or out of domain); set sizes cover {0, 1, 2, [`LARGE_BOUNDARY`]};
/// injected duplicates occur.
#[test]
fn params_for_produces_the_documented_draws() {
    let mut rng = Rng::new(SEED);
    let domains = Domains::of(CFG.scale);
    let (mut saw_string, mut saw_u64, mut saw_bytes) = (false, false, false);
    let mut sizes_seen = std::collections::BTreeSet::new();
    let mut saw_duplicate = false;
    for _ in 0..300 {
        let query = random_query(&mut rng, CFG);
        let draws = params_for(&query, &mut rng, CFG);
        assert_eq!(draws.len(), 4);
        let anchors = param_anchors(&query);
        for draw in &draws {
            assert_eq!(draw.scalars.len() + draw.sets.len(), anchors.len());
            for (_, elements) in &draw.sets {
                sizes_seen.insert(elements.len());
                assert!(
                    elements.len() <= LARGE_BOUNDARY,
                    "set sizes stay at the boundary"
                );
                if elements.len() >= 2 && elements[0] == elements[1] {
                    saw_duplicate = true;
                }
            }
        }
        let miss = &draws[3];
        for (param, value) in &miss.scalars {
            let anchor = anchors[usize::from(param.0)];
            check_miss(
                value,
                anchor.relation,
                anchor.field,
                &domains,
                &mut saw_string,
                &mut saw_u64,
                &mut saw_bytes,
            );
        }
        for (param, elements) in &miss.sets {
            let anchor = anchors[usize::from(param.0)];
            for value in elements {
                check_miss(
                    value,
                    anchor.relation,
                    anchor.field,
                    &domains,
                    &mut saw_string,
                    &mut saw_u64,
                    &mut saw_bytes,
                );
            }
        }
    }
    assert!(saw_string, "the batch produced string params");
    assert!(saw_u64, "the batch produced u64 params");
    assert!(saw_bytes, "the batch produced bytes params");
    for size in [0usize, 1, 2, LARGE_BOUNDARY] {
        assert!(sizes_seen.contains(&size), "set size {size} never drawn");
    }
    assert!(saw_duplicate, "duplicate set elements never injected");
}

fn check_miss(
    value: &Value,
    relation: bumbledb::RelationId,
    field: bumbledb::FieldId,
    domains: &Domains,
    saw_string: &mut bool,
    saw_u64: &mut bool,
    saw_bytes: &mut bool,
) {
    match value {
        Value::String(raw) => {
            *saw_string = true;
            assert!(
                raw.starts_with(b"missing-"),
                "miss-draw string params are guaranteed misses"
            );
        }
        Value::U64(v) => {
            *saw_u64 = true;
            let domain = u64_domain(relation, field, domains);
            assert!(*v > domain, "miss-draw u64 params are out of domain");
        }
        Value::Bytes(raw) => {
            *saw_bytes = true;
            assert_eq!(raw.len(), 16, "a fresh 16-byte miss value");
        }
        _ => {}
    }
}
