//! Real Free Join, the full relation program, and exact conditional observations.
use super::observation::*;
use super::relational_core::Algebra;
use super::*;
use num_bigint::{BigInt, BigUint};
use num_rational::BigRational;
use num_traits::Zero;

struct Answer {
    regions: Vec<Id>,
    coefficients: Vec<Spectrum>,
    evidence: Spectrum,
    values: Vec<[Option<BigRational>; 3]>,
    phases: [f64; 3],
    rows: usize,
    iterations: usize,
}
fn evaluate(law: &ExactLaw, h: &Spectrum, denominator: &BigUint) -> Option<BigRational> {
    (!denominator.is_zero()).then(|| {
        BigRational::new(
            BigInt::from(law.numerator(h)),
            BigInt::from(denominator.clone()),
        )
    })
}
fn query<C: Carrier>(
    engine: &mut native::Native,
    c: &mut Algebra<C>,
    groups: usize,
    evidence: Id,
    plan: &CountPlan,
    laws: &[ExactLaw; 3],
) -> Answer {
    let start = Instant::now();
    let (regions, rows, iterations) = relations::query_regions(engine, c, groups);
    let regions: Vec<_> = regions
        .into_iter()
        .map(|r| c.base.op(8, r, evidence))
        .collect();
    let algebra = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let coefficients: Vec<_> = regions
        .iter()
        .map(|&r| c.base.inner.spectrum(r, plan))
        .collect();
    let evidence = c.base.inner.spectrum(evidence, plan);
    let contraction = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let denominators = laws.each_ref().map(|law| law.numerator(&evidence));
    let values = coefficients
        .iter()
        .map(|h| std::array::from_fn(|i| evaluate(&laws[i], h, &denominators[i])))
        .collect();
    let evaluation = start.elapsed().as_secs_f64();
    Answer {
        regions,
        coefficients,
        evidence,
        values,
        phases: [algebra, contraction, evaluation],
        rows,
        iterations,
    }
}

pub fn bench_laws<C: Carrier>(nbits: u32) {
    let f = relations::fixture(nbits);
    // Evidence selects two actual outcome coordinates. It is structurally
    // possible, but has zero mass at the all-zero endpoint law.
    let evidence_bits = bits(f.n, |w| w & 3 == 3);
    let expected_bits: Vec<Vec<u64>> = f
        .expected
        .iter()
        .map(|b| b.iter().zip(&evidence_bits).map(|(a, g)| a & g).collect())
        .collect();
    for parameter_groups in [1, 2] {
        let mut prepared = None;
        let mut builds = vec![];
        for _ in 0..trials() {
            let start = Instant::now();
            let mut c = C::with_order(&bits(f.n, |_| true), f.n, relation_order(nbits));
            let ids: Vec<_> = f.bank.iter().map(|b| c.import(b)).collect();
            let evidence = c.import(&evidence_bits);
            let c = Algebra::new(c, f.width, false);
            let mut plan =
                CountPlan::new((0..nbits as usize).map(|i| i % parameter_groups).collect());
            c.base.inner.prepare_spectrum(&mut plan);
            let priors: Vec<_> = (0..parameter_groups).map(|g| (2 + g as u32, 3)).collect();
            let means: Vec<_> = priors.iter().map(|&(a, b)| (a, a + b)).collect();
            let laws = [
                ExactLaw::beta(&plan, &priors),
                ExactLaw::point(&plan, &means),
                ExactLaw::point(&plan, &vec![(0, 1); parameter_groups]),
            ];
            builds.push(start.elapsed().as_secs_f64());
            prepared = Some((c, ids, evidence, plan, laws));
        }
        let (input, ids, evidence, plan, laws) = prepared.unwrap();
        let expected_coefficients: Vec<_> =
            expected_bits.iter().map(|b| plan.oracle(b, f.n)).collect();
        let expected_evidence = plan.oracle(&evidence_bits, f.n);
        let expected_denominators = laws.each_ref().map(|law| law.numerator(&expected_evidence));
        let expected_values: Vec<_> = expected_coefficients
            .iter()
            .map(|h| std::array::from_fn(|i| evaluate(&laws[i], h, &expected_denominators[i])))
            .collect();
        assert!(
            expected_values.iter().any(|v| v[0] != v[1]),
            "fixture must expose shared-parameter dependence"
        );
        assert!(
            expected_values
                .iter()
                .all(|v| v[0].is_some() && v[1].is_some() && v[2].is_none())
        );
        assert_ne!(evidence, input.base.inner.empty());
        let evidence_admissible_worlds = input.base.inner.count(evidence);
        let data = f.raw.each_ref().map(|rows| {
            rows.iter()
                .map(|r| [r[0], r[1], r[2], ids[r[3] as usize]])
                .collect()
        });
        let mut engine = native::Native::new(&data, "clover");
        let mut join_only = vec![];
        for _ in 0..trials() {
            let mut rows = 0;
            let start = Instant::now();
            engine.run(|r| {
                black_box(r);
                rows += 1;
            });
            join_only.push(start.elapsed().as_secs_f64());
            assert_eq!(rows, f.expected_rows);
        }
        for memo in [false, true] {
            let mut base = input.clone();
            base.base.enabled = memo;
            let mut fresh = vec![];
            let mut phases: [Vec<f64>; 3] = std::array::from_fn(|_| vec![]);
            let mut retained = None;
            let mut iterations = 0;
            let mut input_stats = None;
            for _ in 0..trials() {
                let mut c = base.clone();
                let stats = (c.bytes(), c.base.inner.memory_stats());
                if let Some(previous) = input_stats {
                    assert_eq!(stats, previous);
                }
                input_stats = Some(stats);
                let start = Instant::now();
                let answer = query(&mut engine, &mut c, f.groups, evidence, &plan, &laws);
                fresh.push(start.elapsed().as_secs_f64());
                black_box(&answer.values);
                assert_eq!(answer.rows, f.expected_rows);
                assert_eq!(answer.coefficients, expected_coefficients);
                assert_eq!(answer.evidence, expected_evidence);
                assert_eq!(answer.values, expected_values);
                for (&id, bits) in answer.regions.iter().zip(&expected_bits) {
                    assert_eq!(c.base.inner.export(id), *bits);
                }
                for i in 0..3 {
                    phases[i].push(answer.phases[i]);
                }
                iterations = answer.iterations;
                retained = Some(c);
            }
            let mut c = retained.unwrap();
            let (input_bytes, input_storage) = input_stats.unwrap();
            let mut warm = vec![];
            for _ in 0..trials() {
                let start = Instant::now();
                let answer = query(&mut engine, &mut c, f.groups, evidence, &plan, &laws);
                warm.push(start.elapsed().as_secs_f64());
                black_box(&answer.values);
                assert_eq!(answer.values, expected_values);
                assert_eq!(answer.coefficients, expected_coefficients);
                assert_eq!(answer.rows, f.expected_rows);
                assert_eq!(answer.iterations, iterations);
            }
            let examples_beta: Vec<_> = expected_values
                .iter()
                .take(5)
                .map(|v| v[0].as_ref().unwrap().to_string())
                .collect();
            let examples_mean: Vec<_> = expected_values
                .iter()
                .take(5)
                .map(|v| v[1].as_ref().unwrap().to_string())
                .collect();
            println!(
                "EVENT_LAB {{\"kind\":\"law_free_join\",\"candidate\":\"{}\",\"bits\":{},\"worlds\":{},\"rows\":{},\"groups\":{},\"parameter_groups\":{},\"coefficient_bins\":{},\"memo\":{},\"build_s\":{:?},\"join_only_s\":{:?},\"fresh_s\":{:?},\"warm_s\":{:?},\"algebra_s\":{:?},\"contraction_s\":{:?},\"evaluation_s\":{:?},\"input_bytes_est\":{},\"final_bytes_est\":{},\"input_storage\":{},\"final_storage\":{},\"observation_plan_bytes_est\":{},\"law_bytes_est\":{},\"closure_iterations\":{},\"observations\":240,\"undefined_observations\":80,\"evidence_admissible_worlds\":{},\"examples_beta\":{:?},\"examples_mean\":{:?},\"verified\":true}}",
                C::NAME,
                nbits,
                f.n,
                f.expected_rows,
                f.groups,
                parameter_groups,
                plan.bins,
                memo,
                builds,
                join_only,
                fresh,
                warm,
                phases[0],
                phases[1],
                phases[2],
                input_bytes,
                c.bytes(),
                memory_json(input_storage),
                memory_json(c.base.inner.memory_stats()),
                plan.bytes(),
                laws.iter().map(ExactLaw::bytes).sum::<usize>(),
                iterations,
                evidence_admissible_worlds,
                examples_beta,
                examples_mean
            );
        }
    }
}
