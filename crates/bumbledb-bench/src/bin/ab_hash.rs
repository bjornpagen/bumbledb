//! TEMPORARY interleaved A/B falsifier for the probe-pass hash
//! const-arity dispatch (perf/probe-hash-dispatch). Not for landing:
//! stripped once the verdict is recorded.
//!
//! A arm = runtime-arity hash loop (the incumbent), B arm = const-arity
//! `gather_hash_core::<K>` dispatch. Both arms live in ONE binary behind
//! a relaxed atomic switch and alternate within ONE process — only the
//! per-pair A/B ratio is reported (absolute numbers are void under
//! co-tenancy; m2max.method.interleaved-ab).

use std::path::Path;
use std::time::Instant;

use bumbledb::{Answers, Db};
use bumbledb_bench::corpus_gen::{GenConfig, Scale};
use bumbledb_bench::driver::ensure_corpus;
use bumbledb_bench::harness::Rotation;
use bumbledb_bench::schema::Ledger;
use bumbledb_bench::{corpus, families};

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let fams = argv.get(1).cloned().unwrap_or_else(|| "triangle".into());
    let pairs: usize = argv.get(2).and_then(|s| s.parse().ok()).unwrap_or(200);
    let scale = match argv.get(3).map(String::as_str) {
        Some("M") => Scale::M,
        Some("L") => Scale::L,
        _ => Scale::S,
    };
    let cfg = GenConfig { seed: 1, scale };
    let paths = ensure_corpus(Path::new("bench-data"), cfg).expect("corpus");
    let scratch = paths.root.join("ab-hash-scratch");
    let _ = std::fs::remove_dir_all(&scratch);
    let db = Db::create(&scratch.join("db"), Ledger).expect("create");
    corpus::load_bumbledb(&db, cfg).expect("load");

    for name in fams.split(',') {
        let family = families::all()
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("unknown family {name}"));
        let query = (family.query)();
        let mut prepared = db.prepare(&query).expect("prepare");
        let mut rotation = Rotation::new((family.params)(&cfg));
        let mut buffer = Answers::new();

        let mut run = |dyn_arm: bool, draw: &[bumbledb_bench::naive::ParamValue]| -> (f64, u64) {
            bumbledb::__hash_ab_set_dyn(dyn_arm);
            let args = families::param_args(draw);
            let t = Instant::now();
            db.read(|snap| snap.execute_args(&mut prepared, &args, &mut buffer))
                .expect("execute");
            (t.elapsed().as_secs_f64() * 1e6, buffer.len() as u64)
        };

        // Per-draw alternating sequences: within one draw the two arms
        // alternate A,B,A,B,... in one warmed world, so cache/predictor
        // order effects hit both arms equally; the per-draw arm medians
        // are the comparable pair. `pairs` = alternations per draw.
        let n_draws = (family.params)(&cfg).len();
        let q = |v: &mut Vec<f64>, p: f64| -> f64 {
            v.sort_by(|x, y| x.partial_cmp(y).expect("no NaN"));
            v[((v.len() - 1) as f64 * p) as usize]
        };
        let mut sum_a = 0.0;
        let mut sum_b = 0.0;
        let mut detail = String::new();
        for d in 0..n_draws {
            let draw = rotation.next_set().clone();
            // Warm this draw's world under both arms.
            for _ in 0..3 {
                let (_, rows_a) = run(true, &draw);
                let (_, rows_b) = run(false, &draw);
                assert_eq!(rows_a, rows_b, "arms disagree on {name} draw {d}");
            }
            let mut a_us: Vec<f64> = Vec::with_capacity(pairs);
            let mut b_us: Vec<f64> = Vec::with_capacity(pairs);
            for _ in 0..pairs {
                a_us.push(run(true, &draw).0);
                b_us.push(run(false, &draw).0);
            }
            let med_a = q(&mut a_us, 0.50);
            let med_b = q(&mut b_us, 0.50);
            sum_a += med_a;
            sum_b += med_b;
            detail.push_str(&format!(
                " d{d}: A={med_a:.1}us B={med_b:.1}us r={:.4}",
                med_a / med_b
            ));
        }
        println!(
            "{name}: alternations/draw={pairs} family ratio A(dyn)/B(const) = {:.4} \
             (sumA={sum_a:.1}us sumB={sum_b:.1}us){detail}",
            sum_a / sum_b
        );
    }
    bumbledb::__hash_ab_set_dyn(false);
    drop(db);
    let _ = std::fs::remove_dir_all(&scratch);
}
