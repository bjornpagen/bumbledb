//! The calendar lanes of the verify pass
//! (docs/architecture/60-validation.md § the calendar benchmark): the
//! second corpus joins the stamp's evidence **before any timing** —
//! every calendar family × its fixed rotation plus a seeded randomized
//! slice against the `SQLite` mirror (the `free_busy` hand coalesce
//! included — the translator-unpaired case is checked, never skipped),
//! and a unit-scale naive differential slice replaying the corpus
//! stream, four judgment-violating deltas (one per statement family:
//! room exclusion, `==` totality, `==` arm validity, working-hours
//! coverage), and every family query against the brute-force model.

use super::{Case, Db, Run, VerifyConfig};

use bumbledb::schema::{Generation, RelationDescriptor, SchemaDescriptor};
use bumbledb::{RelationId, Value};

use crate::calendar::gen::{chain, du_cluster_rows, relation_rows_sized, CalSizes, CAL_BASE, HOUR};
use crate::calendar::{families, ids, schema, Scheduling, ARM_BUSY, RSVP_ACCEPTED};
use crate::differential::{self, Op};
use crate::gen::Rng;
use crate::naive::{Delta, NaiveDb};

/// The calendar family lane: every family × (its fixed rotation, plus
/// the randomized slice when `randomized`), the `SQLite` side per
/// [`families::CalFamily::sql_for`] — translator output for the paired
/// families, the hand-written coalesce for `free_busy`. The empty-store
/// pass runs the fixed rotations only.
pub(super) fn calendar_lane(
    run: &mut Run<'_, Scheduling>,
    cfg: &VerifyConfig,
    label: &str,
    randomized: bool,
) {
    let mut rng = Rng::new(cfg.gen.seed ^ 0x0116_0001);
    'families: for family in families::all() {
        let query = (family.query)();
        let mut draws = (family.params)(&cfg.gen);
        if randomized {
            for _ in 0..families::RANDOM_DRAWS {
                if let Some(draw) = families::random_draw(family.name, &mut rng, &cfg.gen) {
                    draws.push(draw);
                }
            }
        }
        for params in draws {
            let translated = family
                .sql_for(&query, &params)
                .expect("calendar families translate");
            let case = Case {
                label: format!("{label} {}", family.name),
                query: &query,
                sql: &translated.sql,
                golden_sql: Some(family.golden_sql),
            };
            if !run.check(&case, &translated.params, &params) {
                break 'families;
            }
        }
    }
}

/// The lane's case count for the progress total: fixed rotations plus
/// the randomized slice (param-less families have no random axis).
pub(super) fn calendar_case_count(cfg: &VerifyConfig) -> u64 {
    families::all()
        .iter()
        .map(|family| {
            let fixed = (family.params)(&cfg.gen).len() as u64;
            let random = if matches!(family.name, "rsvp_union" | "claim_hours") {
                0
            } else {
                u64::from(families::RANDOM_DRAWS)
            };
            fixed + random
        })
        .sum()
}

/// The fixed rotations alone (the empty-store pass's share).
pub(super) fn calendar_fixed_count(cfg: &VerifyConfig) -> u64 {
    families::all()
        .iter()
        .map(|family| (family.params)(&cfg.gen).len() as u64)
        .sum()
}

/// The calendar schema as the raw descriptor the naive model consumes —
/// the declared statements are the materialized list minus the leading
/// fresh auto-keys (the model re-materializes them itself).
fn calendar_descriptor() -> SchemaDescriptor {
    let sealed = schema();
    let autos = sealed
        .relations()
        .iter()
        .flat_map(|relation| relation.fields().iter())
        .filter(|field| field.generation == Generation::Fresh)
        .count();
    SchemaDescriptor {
        relations: sealed
            .relations()
            .iter()
            .map(|relation| RelationDescriptor {
                extension: None,
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

/// The engine loader's order and joint `==` cluster, as differential
/// write deltas — every chunk judged over the full final state on both
/// sides ([`crate::calendar::corpus`] is the loader twin).
fn load_ops(cfg: crate::gen::GenConfig, sizes: CalSizes) -> Vec<Op> {
    const ORDER: [RelationId; 7] = [
        ids::ACCOUNT,
        ids::PERSON,
        ids::CALENDAR,
        ids::WORK_HOURS,
        ids::EVENT,
        ids::ROOM,
        ids::BOOKING,
    ];
    let mut ops = Vec::new();
    for rel in ORDER {
        let mut delta = Delta::default();
        for row in relation_rows_sized(cfg, sizes, rel) {
            delta.inserts.push((rel, row));
            if delta.inserts.len() == 32 {
                ops.push(Op::Write(std::mem::take(&mut delta)));
            }
        }
        if !delta.inserts.is_empty() {
            ops.push(Op::Write(std::mem::take(&mut delta)));
        }
    }
    let mut delta = Delta::default();
    for (attendances, claim) in du_cluster_rows(cfg, sizes) {
        for row in attendances {
            delta.inserts.push((ids::ATTENDANCE, row));
        }
        delta.inserts.push((ids::CLAIM, claim));
        if delta.inserts.len() >= 32 {
            ops.push(Op::Write(std::mem::take(&mut delta)));
        }
    }
    if !delta.inserts.is_empty() {
        ops.push(Op::Write(delta));
    }
    ops
}

/// Deltas that must ABORT, verdict and violating statement agreeing on
/// both sides — one per statement family, each violating exactly one
/// statement (multi-violation deltas would make the picked violator an
/// implementation accident).
fn violating_ops(seed: u64, sizes: &CalSizes) -> Vec<Op> {
    let first = chain(seed, sizes, 0)[0];
    let overlap = Value::IntervalI64(first.start, first.start + 1);
    // A genuine free instant in person 0's chain: the first gapped
    // boundary (every third abuts; the rest leave a positive gap).
    let segments = chain(seed, sizes, 0);
    let gap = segments
        .windows(2)
        .find(|pair| pair[0].end < pair[1].start)
        .map(|pair| pair[0].end)
        .expect("gapped boundaries exist by construction");
    vec![
        // Room exclusion (the pointwise Booking key): room 0 claimed at
        // an instant its first booking already covers, under a different
        // event (the identical fact would be a no-op).
        Op::Write(Delta {
            deletes: vec![],
            inserts: vec![(ids::BOOKING, vec![Value::U64(0), Value::U64(1), overlap])],
        }),
        // `==` totality: an accepted attendance whose id owns no busy
        // claim (fresh id, person 3 — not on event 0's roster).
        Op::Write(Delta {
            deletes: vec![],
            inserts: vec![(
                ids::ATTENDANCE,
                vec![
                    Value::U64(sizes.attendances + 5),
                    Value::U64(0),
                    Value::U64(3),
                    Value::Enum(RSVP_ACCEPTED),
                ],
            )],
        }),
        // `==` arm validity: a busy claim whose source is no accepted
        // attendance — placed in a real chain gap inside working hours,
        // so the pointwise key and the coverage both hold and the one
        // violated statement is the `==` direction.
        Op::Write(Delta {
            deletes: vec![],
            inserts: vec![(
                ids::CLAIM,
                vec![
                    Value::U64(sizes.ooo_source_base() + sizes.claims + 7),
                    Value::U64(0),
                    Value::Enum(ARM_BUSY),
                    Value::IntervalI64(gap, gap + 1),
                ],
            )],
        }),
        // Working-hours coverage: a pre-epoch busy claim — its `==`
        // twin (event + accepted attendance + claim) rides in the same
        // delta, so the only unmet statement is the coverage. The
        // shared attendance-id/claim-source sits beyond every occupied
        // id range (attendances end at 3 × events; OOO sources end
        // below `ooo_source_base() + claims`).
        {
            let source = sizes.ooo_source_base() + sizes.claims + 100;
            Op::Write(Delta {
                deletes: vec![],
                inserts: vec![
                    (
                        ids::EVENT,
                        vec![
                            Value::U64(sizes.events),
                            Value::U64(0),
                            Value::IntervalI64(CAL_BASE - 2 * HOUR, CAL_BASE - HOUR),
                            Value::I64(CAL_BASE),
                            crate::calendar::gen::event_hash(seed, sizes.events),
                        ],
                    ),
                    (
                        ids::ATTENDANCE,
                        vec![
                            Value::U64(source),
                            Value::U64(sizes.events),
                            Value::U64(0),
                            Value::Enum(RSVP_ACCEPTED),
                        ],
                    ),
                    (
                        ids::CLAIM,
                        vec![
                            Value::U64(source),
                            Value::U64(0),
                            Value::Enum(ARM_BUSY),
                            Value::IntervalI64(CAL_BASE - 2 * HOUR, CAL_BASE - HOUR),
                        ],
                    ),
                ],
            })
        },
    ]
}

/// The calendar naive differential slice: a fresh unit-scale store
/// replays the corpus stream (joint `==` cluster included), the four
/// judgment-violating deltas, and every family query (its unit draw
/// plus its seeded S rotation) against [`NaiveDb`] — `free_busy`'s
/// `Pack` is `SQLite`-inexpressible and runs here and against the hand
/// coalesce, counted and reported, never silently skipped.
///
/// # Panics
///
/// On tool-level invariant violations — never on a disagreement.
pub(super) fn run_calendar_naive<S>(cfg: &VerifyConfig, run: &mut Run<'_, S>) {
    let sizes = CalSizes::unit();
    let mut ops = load_ops(cfg.gen, sizes);
    ops.extend(violating_ops(cfg.gen.seed, &sizes));
    for family in families::all() {
        let query = (family.query)();
        ops.push(Op::Query {
            query: query.clone(),
            params: families::unit_draw(family.name, cfg.gen.seed, &sizes),
        });
        for params in (family.params)(&cfg.gen) {
            ops.push(Op::Query {
                query: query.clone(),
                params,
            });
        }
    }
    eprintln!(
        "verify: calendar translator-unpaired families (hand-SQL + naive-checked, \
         never dropped): {}",
        families::translator_unpaired().join(", ")
    );

    let naive_dir = cfg.out_dir.join("cal-naive-db");
    let _ = std::fs::remove_dir_all(&naive_dir);
    let db = Db::create(&naive_dir, Scheduling).expect("create calendar naive-slice store");
    let mut naive = NaiveDb::new(&calendar_descriptor());
    eprintln!(
        "verify: calendar naive differential slice ({} ops)",
        ops.len()
    );
    match differential::run(&db, &mut naive, &ops) {
        Ok(summary) => {
            assert!(
                summary.aborts >= 4,
                "the violating calendar deltas must abort (got {})",
                summary.aborts
            );
            run.cases += summary.commits + summary.aborts + summary.queries;
        }
        Err(divergence) => {
            let bundle = run.out_dir.join(format!("mismatch-{}", run.bundles.len()));
            std::fs::create_dir_all(&bundle).expect("bundle dir");
            std::fs::write(
                bundle.join("mismatch.txt"),
                format!("calendar naive differential slice diverged:\n{divergence:#?}\n"),
            )
            .expect("bundle");
            eprintln!("verify: CALENDAR NAIVE MISMATCH -> {}", bundle.display());
            run.bundles.push(bundle);
        }
    }
}
