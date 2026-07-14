use super::oracle::{LARGE_BOUNDARY, param_anchors, u64_domain};
use super::target::{self, Domains};
use super::*;
use bumbledb::Value;

use crate::corpus_gen::{GenConfig, Rng, Scale};
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
    let db = bumbledb::Db::create(&dir, target::Target).expect("create");
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
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // the contract's assertion roster, one row per construct
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
    band("key_probe", cov.key_probe, 10);
    band("star", cov.star, 15);
    band("chain", cov.chain, 15);
    band("self_join", cov.self_join, 8);
    band("gated", cov.gated, 8);
    band("aggregate", cov.aggregate, 14);
    band("membership", cov.membership, 10);
    band("interval_join", cov.interval_join, 10);
    band("boundary", cov.boundary, 6);
    band("count_distinct", cov.count_distinct, 10);
    band("arg", cov.arg, 8);
    band("existence_walk", cov.existence_walk, 8);
    band("du_walk", cov.du_walk, 6);
    band("rules", cov.rules, 10);
    band("measure", cov.measure, 8);
    band("closed_join", cov.closed_join, 8);
    band("closed_fold", cov.closed_fold, 7);
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
        ("allen_u64", cov.allen_u64),
        ("allen_i64", cov.allen_i64),
        ("allen_composite", cov.allen_composite),
        ("allen_singleton", cov.allen_singleton),
        ("allen_random_mask", cov.allen_random_mask),
        ("point_in_u64", cov.point_in_u64),
        ("point_in_i64", cov.point_in_i64),
        ("adjacent_left", cov.adjacent_left),
        ("adjacent_right", cov.adjacent_right),
        // The boundary-shape ladder, drawn for every interval literal:
        // equal/adjacent/nested/ray each appear per run.
        ("ladder_equal", cov.ladder[0]),
        ("ladder_adjacent", cov.ladder[1]),
        ("ladder_nested", cov.ladder[2]),
        ("ladder_ray", cov.ladder[3]),
        // Multi-rule programs: every arm count and every variant.
        ("rules_two_arms", cov.rules_arms[0]),
        ("rules_three_arms", cov.rules_arms[1]),
        ("rules_four_arms", cov.rules_arms[2]),
        ("rules_disjoint", cov.rules_disjoint),
        ("rules_overlap", cov.rules_overlap),
        ("rules_aggregate", cov.rules_aggregate),
        // The measure's three construct kinds.
        ("duration_find", cov.duration_find),
        ("duration_predicate", cov.duration_predicate),
        ("duration_fold", cov.duration_fold),
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
        // Wide projections (the >8-word class the executor's hoist
        // paths must never cap): all-scalar width and the
        // ≥4-interval-find flavor, both drawn per run.
        ("wide_scalar", cov.wide_scalar),
        ("wide_interval", cov.wide_interval),
        // The grounding shapes' structural assertion: both an eliminated
        // (existence walks and both DU `==` directions) and a refused
        // (extra projected target field; missing φ) shape appear per
        // run — the engine-backed verdict test holds the tags honest.
        ("ground_eliminable", cov.ground_eliminable),
        ("ground_extra_field", cov.ground_extra_field),
        ("ground_missing_phi", cov.ground_missing_phi),
        ("du_header_falls", cov.du_header_falls),
        ("du_child_falls", cov.du_child_falls),
        // The closed-relation classes (shapes_closed.rs) — restated in
        // full by `the_closed_relation_classes_are_emitted`.
        ("closed_join_plain", cov.closed_join_plain),
        ("closed_join_selected", cov.closed_join_selected),
        ("closed_handle_literal", cov.closed_handle_literal),
        ("closed_handle_set", cov.closed_handle_set),
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
    // Every Allen basic is reachable through some literal mask per run
    // (singletons, composites, and random masks jointly).
    for (index, count) in cov.allen_basics.iter().enumerate() {
        assert!(*count > 0, "Allen basic {index} never appeared in a mask");
    }
    // The structural compositions where bugs hide: at least one query
    // per run carries each.
    assert!(cov.neg_and_aggregate > 0, "negation ∧ aggregate missing");
    assert!(cov.set_and_negation > 0, "param set ∧ negation missing");
    assert!(cov.membership_and_allen > 0, "membership ∧ Allen missing");
    assert!(cov.mask_and_negation > 0, "mask ∧ negation missing");
    assert!(
        cov.rules_aggregate > 0,
        "rules ∧ aggregate missing (asserted above; restated as the composite)"
    );
    // The equality-spine cost bound (60-validation.md § the generator
    // contract): every emitted membership/overlap construct rides an
    // equality-connected spine — the keyless Cartesian degenerate
    // (40-execution.md) is unemittable.
    assert_eq!(
        cov.spine_violations, 0,
        "a membership/overlap construct escaped the equality spine"
    );
}

/// The grounding tags are engine-verified: prepared against the target
/// schema (statements included — the grounding runs at prepare, data-free),
/// every eliminable variant's profile carries `Role::Eliminated` marks
/// (the DU directions naming their fallen side) and every near-miss
/// carries none — the structural assertion that both an eliminated and
/// a refused shape appear per run, held to the engine's verdict.
#[test]
fn grounding_shapes_eliminate_and_near_misses_refuse() {
    use super::GroundVariant;
    use super::construct::random_query_tagged;
    let dir = std::env::temp_dir().join("bumbledb-bench-querygen-grounding");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let db = bumbledb::Db::create(&dir, target::Target).expect("create");
    let mut rng = Rng::new(SEED);
    let (mut eliminated, mut refused) = (0u32, 0u32);
    for i in 0..N {
        let (query, _, tags) = random_query_tagged(&mut rng, CFG);
        let Some(variant) = tags.ground else { continue };
        let mut prepared = db.prepare(&query).expect("grounding shapes validate");
        let (_, stats) = db
            .read(|snap| snap.profile(&mut prepared, &[]))
            .expect("grounding shapes execute (empty store)");
        match variant {
            GroundVariant::Walk => {
                assert_eq!(
                    stats.rules[0].eliminated.len(),
                    1,
                    "walk {i} must eliminate"
                );
                eliminated += 1;
            }
            GroundVariant::DuHeader | GroundVariant::DuChild => {
                let fallen = if variant == GroundVariant::DuHeader {
                    "JournalEntry"
                } else {
                    "ImportBatch"
                };
                assert_eq!(
                    stats.rules[0].eliminated.len(),
                    1,
                    "DU walk {i} must eliminate"
                );
                assert_eq!(
                    stats.rules[0].eliminated[0].relation, fallen,
                    "DU walk {i} fells the wrong side"
                );
                eliminated += 1;
            }
            GroundVariant::WalkExtraField | GroundVariant::DuMissingPhi => {
                assert!(
                    stats.rules[0].eliminated.is_empty(),
                    "near-miss {i} must refuse: {:?}",
                    stats.rules[0].eliminated
                );
                refused += 1;
            }
        }
    }
    assert!(
        eliminated > 0 && refused > 0,
        "both an eliminated ({eliminated}) and a refused ({refused}) shape appear per run"
    );
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The four closed-relation pattern classes are all emitted (the
/// counting pattern): (a) joins against closed relations with and
/// without payload-column selections, (b) handle literals and handle
/// param sets on referencing fields, (c) the fold-shaped pattern under
/// its own family knob (PRD 07 points here), and (d) the judgment
/// write scenarios — closed writes, dangling handles below and beyond
/// the roster cap, and the ψ-subset exclusions, each carrying its
/// hand-derived typed violation.
#[test]
fn the_closed_relation_classes_are_emitted() {
    use crate::querygen::writes::{ClosedWriteKind, closed_write_cases};

    let cov = coverage(N, SEED, CFG);
    // (a) joins, with and without the payload-column selection.
    assert!(cov.closed_join_plain > 0, "plain closed joins");
    assert!(cov.closed_join_selected > 0, "payload-column selections");
    // (b) handle bindings on referencing fields.
    assert!(cov.closed_handle_literal > 0, "handle literals");
    assert!(cov.closed_handle_set > 0, "handle param sets");
    // (c) the fold-shaped pattern — its own family knob.
    assert!(cov.closed_fold > 0, "the PRD 07 fold shape");

    // (d) the judgment write scenarios, all six kinds per batch.
    let mut rng = Rng::new(SEED);
    let cases = closed_write_cases(&mut rng, 24);
    for kind in [
        ClosedWriteKind::ClosedInsert,
        ClosedWriteKind::ClosedDelete,
        ClosedWriteKind::DanglingHandle,
        ClosedWriteKind::BeyondRosterCap,
        ClosedWriteKind::PsiExcluded,
        ClosedWriteKind::PsiOutOfRange,
    ] {
        assert!(
            cases.iter().any(|case| case.kind == kind),
            "write kind {kind:?} never generated"
        );
    }
    // The out-of-range ids genuinely straddle the roster cap.
    for case in &cases {
        let id = |fact: &[Value], index: usize| match fact[index] {
            Value::U64(v) => v,
            ref other => panic!("a handle is u64, got {other:?}"),
        };
        match case.kind {
            ClosedWriteKind::DanglingHandle => {
                let v = id(&case.fact, 1);
                assert!((3..256).contains(&v), "in the word, off the extension");
            }
            ClosedWriteKind::BeyondRosterCap => {
                assert!(id(&case.fact, 1) >= 256, "beyond the member-set width");
            }
            ClosedWriteKind::PsiExcluded => {
                assert!(id(&case.fact, 0) < 2, "a real row outside psi");
            }
            _ => {}
        }
    }
}

/// The grammar never emits a NUL — the translator rejects NUL string
/// literals by name, and this property keeps that boundary unreachable
/// from generated queries.
#[test]
fn generated_string_literals_are_nul_free() {
    let mut rng = Rng::new(SEED);
    for _ in 0..N {
        let query = random_query(&mut rng, CFG);
        for rule in &query.rules {
            for atom in rule.atoms.iter().chain(&rule.negated) {
                for (_, term) in &atom.bindings {
                    if let bumbledb::Term::Literal(bumbledb::Value::String(raw)) = term {
                        assert!(!raw.contains(&0), "a generated literal carries NUL");
                    }
                }
            }
            for comparison in rule.conditions.iter().map(crate::querygen::leaf) {
                for term in [&comparison.lhs, &comparison.rhs] {
                    if let bumbledb::Term::Literal(bumbledb::Value::String(raw)) = term {
                        assert!(!raw.contains(&0), "a generated literal carries NUL");
                    }
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
        Value::FixedBytes(raw) => {
            *saw_bytes = true;
            // Adversarial single-byte-delta misses: a real digest of the
            // anchored width with byte 0 flipped out of the corpus range.
            assert!(
                matches!(raw.len(), 7 | 8 | 9 | 16 | 32 | 63 | 64),
                "miss digests carry an anchored width, got {}",
                raw.len()
            );
            assert_eq!(raw[0], 0xA5, "the flipped byte marks the miss");
        }
        _ => {}
    }
}
