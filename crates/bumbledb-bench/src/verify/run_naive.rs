use super::{Db, Run, VerifyConfig};

use bumbledb::schema::{Generation, RelationDescriptor, SchemaDescriptor};
use bumbledb::{Interval, RelationId, Value};

use crate::families::{self, scalar_draw, Draw};
use crate::gen::{self, mandate_segments, Sizes, MANDATE_SEGMENTS};
use crate::naive::differential::{self, Op};
use crate::naive::{Delta, NaiveDb, ParamValue};
use crate::schema::{ids, schema};

/// The unit-scale corpus of the naive lane: small enough for the
/// brute-force model's nested loops, large enough that every family's
/// joins have witnesses.
fn unit_sizes() -> Sizes {
    Sizes {
        postings: 120,
        entries: 60,
        accounts: 8,
        holders: 2,
        instruments: 8,
        orgs: 4,
        org_parents: 3,
        posting_tags: 120,
        mandates: 8 * MANDATE_SEGMENTS,
    }
}

/// The bench schema as the raw descriptor the naive model consumes —
/// reconstructed from the sealed schema: the declared statements are the
/// materialized list minus the leading serial auto-keys (the model
/// re-materializes them itself).
fn bench_descriptor() -> SchemaDescriptor {
    let sealed = schema();
    let autos = sealed
        .relations()
        .iter()
        .flat_map(|relation| relation.fields().iter())
        .filter(|field| field.generation == Generation::Serial)
        .count();
    SchemaDescriptor {
        relations: sealed
            .relations()
            .iter()
            .map(|relation| RelationDescriptor {
                name: relation.name().into(),
                fields: relation.fields().to_vec(),
            })
            .collect(),
        statements: sealed.statements()[autos..]
            .iter()
            .map(|statement| statement.descriptor.clone())
            .collect(),
    }
}

/// The insert stream as write deltas, chunked — every chunk judged over
/// the full final state on both sides.
fn load_ops(seed: u64, sizes: &Sizes) -> Vec<Op> {
    let cfg = gen::GenConfig {
        seed,
        scale: gen::Scale::S, // unused: rows take explicit unit sizes
    };
    let mut ops = Vec::new();
    for rel in 0..ids::RELATIONS {
        let rel = RelationId(rel);
        let mut delta = Delta::default();
        for i in 0..sizes.rows(rel) {
            delta.inserts.push((rel, gen::row(&cfg, sizes, rel, i)));
            if delta.inserts.len() == 32 {
                ops.push(Op::Write(std::mem::take(&mut delta)));
            }
        }
        if !delta.inserts.is_empty() {
            ops.push(Op::Write(delta));
        }
    }
    ops
}

/// Deltas that must ABORT, verdict and violating statement agreeing on
/// both sides: a dangling containment source, a pointwise-key overlap,
/// a scalar-key duplicate, and a target-required delete.
fn violating_ops(seed: u64, sizes: &Sizes) -> Vec<Op> {
    let segment = mandate_segments(seed, sizes, 0)[0];
    let overlap = Interval::<i64>::new(segment.start, segment.start + 1).expect("nonempty");
    vec![
        // Posting -> Account containment, source side.
        Op::Write(Delta {
            deletes: vec![],
            inserts: vec![(
                ids::POSTING,
                vec![
                    Value::U64(sizes.postings + 1),
                    Value::U64(0),
                    Value::U64(sizes.accounts + 3),
                    Value::U64(0),
                    Value::I64(1),
                    Value::I64(gen::AT_BASE),
                ],
            )],
        }),
        // The pointwise Mandate key: a one-point overlap on account 0
        // under a different org (the identical fact would be a no-op).
        Op::Write(Delta {
            deletes: vec![],
            inserts: vec![(
                ids::MANDATE,
                vec![
                    Value::U64(0),
                    Value::U64((segment.org + 1) % sizes.orgs),
                    Value::from(overlap),
                ],
            )],
        }),
        // The Holder serial key: a second fact under id 0.
        Op::Write(Delta {
            deletes: vec![],
            inserts: vec![(
                ids::HOLDER,
                vec![
                    Value::U64(0),
                    Value::String(b"holder-duplicate".to_vec().into()),
                ],
            )],
        }),
        // Deleting account 0 strands its postings and mandates: the
        // target-required direction.
        Op::Write(Delta {
            deletes: vec![(ids::ACCOUNT, {
                let cfg = gen::GenConfig {
                    seed,
                    scale: gen::Scale::S,
                };
                gen::row(&cfg, sizes, ids::ACCOUNT, 0)
            })],
            inserts: vec![],
        }),
    ]
}

/// One in-domain draw per family at unit scale (the S-scale rotations
/// are mostly misses here; these make the joins produce witnesses).
fn unit_draw(name: &str, seed: u64, sizes: &Sizes) -> Draw {
    let cfg = gen::GenConfig {
        seed,
        scale: gen::Scale::S,
    };
    let span = i64::try_from(sizes.postings).expect("fits") * gen::AT_STEP;
    match name {
        "point" => scalar_draw(vec![Value::U64(3)]),
        "containment_walk" | "postings_without_tag" => scalar_draw(vec![Value::U64(0)]),
        "chain" => scalar_draw(vec![Value::I64(gen::AT_BASE)]),
        "range" => scalar_draw(vec![
            Value::I64(gen::AT_BASE + span / 4),
            Value::I64(gen::AT_BASE + span / 2),
        ]),
        // orgs and holders both have >1 unit-scale id 1.
        "balance" | "mandate_overlap" => scalar_draw(vec![Value::U64(1)]),
        "stats" | "spread" | "latest_posting_per_account" => scalar_draw(vec![]),
        "string" => scalar_draw(vec![Value::String(b"SYM0003".to_vec().into())]),
        "skew" => scalar_draw(vec![Value::Enum(0)]),
        "triangle" => scalar_draw(vec![Value::U64(0), Value::U64(sizes.accounts)]),
        "entries_for_account_set" => vec![ParamValue::Set(vec![
            Value::U64(0),
            Value::U64(3),
            Value::U64(5),
        ])],
        "mandate_at_instant" => {
            let posting = gen::row(&cfg, sizes, ids::POSTING, 7);
            scalar_draw(vec![
                posting[usize::from(ids::posting::ACCOUNT.0)].clone(),
                posting[usize::from(ids::posting::AT.0)].clone(),
            ])
        }
        other => unreachable!("unregistered family {other}"),
    }
}

/// The naive-model differential slice (docs/architecture/60-validation.md
/// § the two oracles — the integration point PRD 21 marked): a fresh
/// unit-scale store replays the corpus stream, four judgment-violating
/// deltas, and every family query (its unit draw plus its seeded S
/// rotation) against [`NaiveDb`]; any verdict, violator, or result-set
/// disagreement is an arbitration bundle.
///
/// # Panics
///
/// On tool-level invariant violations — never on a disagreement.
pub(super) fn run_naive_slice(cfg: &VerifyConfig, run: &mut Run<'_>) {
    let sizes = unit_sizes();
    let mut ops = load_ops(cfg.gen.seed, &sizes);
    ops.extend(violating_ops(cfg.gen.seed, &sizes));
    for family in families::all() {
        let query = (family.query)();
        ops.push(Op::Query {
            query: query.clone(),
            params: unit_draw(family.name, cfg.gen.seed, &sizes),
        });
        for params in (family.params)(&cfg.gen) {
            ops.push(Op::Query {
                query: query.clone(),
                params,
            });
        }
    }

    let naive_dir = cfg.out_dir.join("naive-db");
    let _ = std::fs::remove_dir_all(&naive_dir);
    let db = Db::create(&naive_dir, schema()).expect("create naive-slice store");
    let mut naive = NaiveDb::new(&bench_descriptor());
    eprintln!("verify: naive differential slice ({} ops)", ops.len());
    match differential::run(&db, &mut naive, &ops) {
        Ok(summary) => {
            assert!(
                summary.aborts >= 4,
                "the violating deltas must abort (got {})",
                summary.aborts
            );
            run.cases += summary.commits + summary.aborts + summary.queries;
        }
        Err(divergence) => {
            let bundle = run.out_dir.join(format!("mismatch-{}", run.bundles.len()));
            std::fs::create_dir_all(&bundle).expect("bundle dir");
            std::fs::write(
                bundle.join("mismatch.txt"),
                format!("naive differential slice diverged:\n{divergence:#?}\n"),
            )
            .expect("bundle");
            eprintln!("verify: NAIVE MISMATCH -> {}", bundle.display());
            run.bundles.push(bundle);
        }
    }
}
