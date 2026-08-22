use bumbledb::{Query, RelationId, Value};

use crate::naive::{Delta, ParamValue};
use crate::querygen::target::{self, Domains};
use crate::querygen::writes::closed_write_cases;
use crate::querygen::{ParamDraw, interval_data, params_for, random_query};

use super::{GenConfig, Rng, Scale};

#[derive(Debug, Clone)]
pub enum FuzzOp {
    Insert(Delta),

    Delete(Delta),

    Mixed(Delta),

    Commit,

    Rollback,

    Execute {
        slot: usize,
        params: Vec<ParamValue>,
    },

    Reprepare {
        slot: usize,
    },

    ViewRead {
        relation: RelationId,
    },
    /// Drop the environment and reopen the store from disk (the pending
    Reopen,

    VerifyStore,
}

#[derive(Debug, Clone)]
pub struct OpScenario {
    pub queries: Vec<Query>,
    pub ops: Vec<FuzzOp>,
}

pub fn random_scenario(rng: &mut Rng) -> OpScenario {
    let cfg = GenConfig {
        seed: rng.u64(),
        scale: Scale::Tiny,
    };
    let world = world(rng);
    let queries: Vec<Query> = (0..=rng.range(3)).map(|_| random_query(rng, cfg)).collect();
    let mut ops = vec![FuzzOp::Insert(seed_world(cfg, &world)), FuzzOp::Commit];
    let streak = rng.chance(1, 4);
    for _ in 0..6 + rng.range(19) {
        ops.push(if streak {
            streak_step(rng, cfg, &world, &queries)
        } else {
            step(rng, cfg, &world, &queries)
        });
    }
    OpScenario { queries, ops }
}

#[derive(Debug, Clone)]
pub struct CrashScenario {
    pub prefix: Vec<Delta>,
    pub victim: Delta,
}

pub fn random_crash_scenario(rng: &mut Rng) -> CrashScenario {
    let cfg = GenConfig {
        seed: rng.u64(),
        scale: Scale::Tiny,
    };
    let world = world(rng);
    let mut prefix = vec![seed_world(cfg, &world)];
    for _ in 0..rng.range(3) {
        prefix.push(batch(rng, cfg, &world, Kind::Mixed));
    }
    let kind = if rng.chance(1, 4) {
        Kind::Mixed
    } else {
        Kind::Inserts
    };
    let victim = batch(rng, cfg, &world, kind);
    CrashScenario { prefix, victim }
}

pub const CRASH_MATRIX_CELLS: usize = 3;

/// # Panics
#[must_use]
pub fn crash_matrix_scenario(cell: usize) -> CrashScenario {
    let a = matrix_world_seed(0);
    let b = matrix_world_seed(1);
    let c = matrix_world_seed(2);
    match cell {
        0 => CrashScenario {
            prefix: vec![],
            victim: a,
        },
        1 => CrashScenario {
            victim: replace(&a, b),
            prefix: vec![a],
        },
        2 => CrashScenario {
            victim: replace(&b, c),
            prefix: vec![a.clone(), replace(&a, b)],
        },
        _ => panic!("crash matrix cell {cell} out of range"),
    }
}

fn matrix_world_seed(step: u64) -> Delta {
    let accounts = 2 + step;
    let world = Domains {
        postings: 8 + 3 * step,
        entries: 4 + 2 * step,
        accounts,
        holders: 1 + step,
        instruments: 2 + step,
        orgs: 2 + step,
        mandates: accounts * interval_data::PER_GROUP,
        transfers: 3 + step,
        posting_tags: 8 + 3 * step,
    };
    let cfg = GenConfig {
        seed: 0x14CA_5C4D ^ step.wrapping_mul(0x9E37_79B9_7F4A_7C15),
        scale: Scale::Tiny,
    };
    seed_world(cfg, &world)
}

fn replace(from: &Delta, to: Delta) -> Delta {
    Delta {
        deletes: from.inserts.clone(),
        inserts: to.inserts,
    }
}

fn step(rng: &mut Rng, cfg: GenConfig, world: &Domains, queries: &[Query]) -> FuzzOp {
    match rng.range(20) {
        0..=3 => FuzzOp::Insert(batch(rng, cfg, world, Kind::Inserts)),
        4..=5 => FuzzOp::Delete(batch(rng, cfg, world, Kind::Deletes)),
        6..=7 => FuzzOp::Mixed(batch(rng, cfg, world, Kind::Mixed)),
        8..=11 => FuzzOp::Commit,
        12 => FuzzOp::Rollback,
        13..=15 => execute_step(rng, cfg, queries),
        16 => FuzzOp::Reprepare {
            slot: index(rng, queries.len()),
        },
        17 => FuzzOp::ViewRead {
            relation: ordinary_relation(rng),
        },
        18 => FuzzOp::Reopen,
        _ => FuzzOp::VerifyStore,
    }
}

/// No delete, mixed, rollback, or reopen arm: a delete would fork the lineage
/// back to a rebuild, and a reopen drops the process-local cache — both
/// well-covered by the general alphabet.
fn streak_step(rng: &mut Rng, cfg: GenConfig, world: &Domains, queries: &[Query]) -> FuzzOp {
    match rng.range(20) {
        0..=6 => FuzzOp::Insert(batch(rng, cfg, world, Kind::Inserts)),
        7..=12 => FuzzOp::Commit,
        13..=17 => execute_step(rng, cfg, queries),
        18 => FuzzOp::ViewRead {
            relation: ordinary_relation(rng),
        },
        _ => FuzzOp::VerifyStore,
    }
}

fn world(rng: &mut Rng) -> Domains {
    let accounts = 2 + rng.range(3);
    let postings = 8 + rng.range(17);
    Domains {
        postings,
        entries: 4 + rng.range(8),
        accounts,
        holders: 1 + rng.range(2),
        instruments: 2 + rng.range(4),
        orgs: 2 + rng.range(3),
        mandates: accounts * interval_data::PER_GROUP,
        transfers: 3 + rng.range(6),
        posting_tags: postings,
    }
}

fn seed_world(cfg: GenConfig, world: &Domains) -> Delta {
    let mut delta = Delta::default();
    for rel in 0..target::TARGET_RELATIONS {
        let rel = RelationId(rel);
        for i in 0..target::corpus_rows(world, rel) {
            delta
                .inserts
                .push((rel, target::corpus_row(cfg, world, rel, i)));
        }
    }
    delta
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Inserts,
    Deletes,
    Mixed,
}

fn batch(rng: &mut Rng, cfg: GenConfig, world: &Domains, kind: Kind) -> Delta {
    let mut delta = Delta::default();
    for _ in 0..=rng.range(3) {
        if rng.chance(1, 10) {
            let mut cases = closed_write_cases(rng, 6);
            if kind == Kind::Inserts {
                cases.retain(|case| !case.delete);
            }
            let case = cases.swap_remove(index(rng, cases.len()));
            if case.delete {
                delta.deletes.push((case.relation, case.fact));
            } else {
                delta.inserts.push((case.relation, case.fact));
            }
            continue;
        }
        let deletes = match kind {
            Kind::Inserts => false,
            Kind::Deletes => true,
            Kind::Mixed => rng.chance(1, 2),
        };
        if deletes {
            push_delete(rng, cfg, world, &mut delta);
        } else {
            push_insert(rng, cfg, world, &mut delta);
        }
    }
    delta
}

fn push_insert(rng: &mut Rng, cfg: GenConfig, world: &Domains, delta: &mut Delta) {
    let rel = ordinary_relation(rng);
    let rows = target::corpus_rows(world, rel);
    match rng.range(8) {
        0..=4 => {
            let i = rows + rng.range(4);
            delta
                .inserts
                .push((rel, target::corpus_row(cfg, world, rel, i)));
            if rel == target::ids::JOURNAL_ENTRY
                && i % 3 == target::SOURCE_IMPORT
                && rng.chance(1, 2)
            {
                let sibling = (i - 1) / 3;
                delta.inserts.push((
                    target::ids::IMPORT_BATCH,
                    target::corpus_row(cfg, world, target::ids::IMPORT_BATCH, sibling),
                ));
            }
        }
        5 => {
            let i = rng.range(rows.max(1));
            delta
                .inserts
                .push((rel, target::corpus_row(cfg, world, rel, i)));
        }
        6 => {
            let twisted = GenConfig {
                seed: cfg.seed ^ 0xC2B2_AE3D_27D4_EB4F,
                ..cfg
            };
            let i = rng.range(rows.max(1));
            delta
                .inserts
                .push((rel, target::corpus_row(twisted, world, rel, i)));
        }
        _ => {
            let i = rows + rng.range(4);
            delta
                .inserts
                .push((rel, target::corpus_row(cfg, &inflated(world), rel, i)));
        }
    }
}

fn push_delete(rng: &mut Rng, cfg: GenConfig, world: &Domains, delta: &mut Delta) {
    let rel = ordinary_relation(rng);
    let rows = target::corpus_rows(world, rel).max(1);
    let i = rng.range(rows + 2);
    delta
        .deletes
        .push((rel, target::corpus_row(cfg, world, rel, i)));
}

fn execute_step(rng: &mut Rng, cfg: GenConfig, queries: &[Query]) -> FuzzOp {
    let slot = index(rng, queries.len());
    let draws = params_for(&queries[slot], rng, cfg);
    let params = if draws.is_empty() {
        Vec::new()
    } else {
        positional(&draws[index(rng, draws.len())])
    };
    FuzzOp::Execute { slot, params }
}

fn positional(draw: &ParamDraw) -> Vec<ParamValue> {
    let len = draw.scalars.len() + draw.sets.len();
    let mut out: Vec<ParamValue> = vec![ParamValue::Scalar(Value::Bool(false)); len];
    for (param, value) in &draw.scalars {
        out[usize::from(param.0)] = ParamValue::Scalar(value.clone());
    }
    for (param, values) in &draw.sets {
        out[usize::from(param.0)] = ParamValue::Set(values.clone());
    }
    out
}

/// The closed relations are ground axioms — their write surface is the
/// closed-case arm in [`batch`], and their contents are schema, not store
/// state, so the view-read and reopen comparisons range over the ordinary
/// relations.
fn ordinary_relation(rng: &mut Rng) -> RelationId {
    RelationId(u32::try_from(rng.range(u64::from(target::TARGET_RELATIONS))).expect("relation id"))
}

fn inflated(world: &Domains) -> Domains {
    Domains {
        postings: world.postings * 4 + 7,
        entries: world.entries * 4 + 7,
        accounts: world.accounts * 4 + 7,
        holders: world.holders * 4 + 7,
        instruments: world.instruments * 4 + 7,
        orgs: world.orgs * 4 + 7,
        mandates: world.mandates * 4 + 7,
        transfers: world.transfers * 4 + 7,
        posting_tags: world.posting_tags * 4 + 7,
    }
}

fn index(rng: &mut Rng, n: usize) -> usize {
    usize::try_from(rng.range(u64::try_from(n).expect("count fits u64"))).expect("index fits usize")
}

#[cfg(test)]
mod tests {
    use super::{
        CRASH_MATRIX_CELLS, FuzzOp, crash_matrix_scenario, random_crash_scenario, random_scenario,
    };
    use crate::corpus_gen::Rng;
    use crate::naive::NaiveDb;
    use crate::querygen::target;

    fn verb(op: &FuzzOp) -> &'static str {
        match op {
            FuzzOp::Insert(_) => "insert",
            FuzzOp::Delete(_) => "delete",
            FuzzOp::Mixed(_) => "mixed",
            FuzzOp::Commit => "commit",
            FuzzOp::Rollback => "rollback",
            FuzzOp::Execute { .. } => "execute",
            FuzzOp::Reprepare { .. } => "reprepare",
            FuzzOp::ViewRead { .. } => "viewread",
            FuzzOp::Reopen => "reopen",
            FuzzOp::VerifyStore => "verifystore",
        }
    }

    #[test]
    fn the_same_bytes_yield_the_same_scenario() {
        let bytes: Vec<u8> = (1..=256u64)
            .flat_map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15).to_le_bytes())
            .collect();
        let first = format!("{:?}", random_scenario(&mut Rng::from_bytes(&bytes)));
        assert_eq!(
            first,
            format!("{:?}", random_scenario(&mut Rng::from_bytes(&bytes))),
            "same bytes, same scenario"
        );
        let other: Vec<u8> = (1..=256u64)
            .flat_map(|i| i.wrapping_mul(0xC2B2_AE3D_27D4_EB4F).to_le_bytes())
            .collect();
        assert_ne!(
            first,
            format!("{:?}", random_scenario(&mut Rng::from_bytes(&other))),
            "bytes steer the scenario"
        );
    }

    #[test]
    fn the_alphabet_reaches_all_ten_verbs() {
        let mut seen = std::collections::BTreeSet::new();
        for seed in 0..256u64 {
            let scenario = random_scenario(&mut Rng::new(seed));
            assert!(
                matches!(
                    scenario.ops.as_slice(),
                    [FuzzOp::Insert(_), FuzzOp::Commit, ..]
                ),
                "the seed world commits first"
            );
            assert!(!scenario.queries.is_empty(), "the pool is never empty");
            for op in &scenario.ops {
                seen.insert(verb(op));
                if let FuzzOp::Execute { slot, .. } | FuzzOp::Reprepare { slot } = op {
                    assert!(*slot < scenario.queries.len(), "slots index the pool");
                }
            }
        }
        let all = [
            "insert",
            "delete",
            "mixed",
            "commit",
            "rollback",
            "execute",
            "reprepare",
            "viewread",
            "reopen",
            "verifystore",
        ];
        for verb in all {
            assert!(seen.contains(verb), "verb {verb} never drawn in 256 seeds");
        }
    }

    #[test]
    fn insert_batches_are_delete_free() {
        for seed in 0..256u64 {
            let scenario = random_scenario(&mut Rng::new(seed));
            for op in &scenario.ops {
                if let FuzzOp::Insert(delta) = op {
                    assert!(
                        delta.deletes.is_empty(),
                        "seed {seed}: an insert batch staged a delete"
                    );
                }
            }
        }
    }

    /// chain WITH reads between — at least three commits after the seed
    /// reopen verb anywhere. That is the append-on-append stress: each
    #[test]
    fn the_streak_variant_reaches_long_read_interleaved_append_chains() {
        let streaks = (0..256u64)
            .map(|seed| random_scenario(&mut Rng::new(seed)))
            .filter(|scenario| {
                let commits = scenario
                    .ops
                    .iter()
                    .filter(|op| matches!(op, FuzzOp::Commit))
                    .count();
                let executes = scenario
                    .ops
                    .iter()
                    .filter(|op| matches!(op, FuzzOp::Execute { .. }))
                    .count();
                let forbidden = scenario.ops.iter().any(|op| {
                    matches!(
                        op,
                        FuzzOp::Delete(_) | FuzzOp::Mixed(_) | FuzzOp::Rollback | FuzzOp::Reopen
                    )
                });
                commits >= 4 && executes >= 1 && !forbidden
            })
            .count();
        assert!(
            streaks >= 8,
            "append-on-append chains are rare again: {streaks} streak scenarios in 256 seeds"
        );
    }

    #[test]
    fn the_same_bytes_yield_the_same_crash_scenario() {
        let bytes: Vec<u8> = (1..=256u64)
            .flat_map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15).to_le_bytes())
            .collect();
        let first = format!("{:?}", random_crash_scenario(&mut Rng::from_bytes(&bytes)));
        assert_eq!(
            first,
            format!("{:?}", random_crash_scenario(&mut Rng::from_bytes(&bytes))),
            "same bytes, same crash scenario"
        );
        for seed in 0..64u64 {
            let scenario = random_crash_scenario(&mut Rng::new(seed));
            assert!(
                !scenario.victim.deletes.is_empty() || !scenario.victim.inserts.is_empty(),
                "seed {seed}: an empty victim commit"
            );
            assert!(
                !scenario.prefix.is_empty(),
                "the seed world opens the prefix"
            );
        }
    }

    #[test]
    fn every_crash_matrix_victim_is_accepted_and_state_changing() {
        for cell in 0..CRASH_MATRIX_CELLS {
            let scenario = crash_matrix_scenario(cell);
            let mut model = NaiveDb::new(&target::descriptor());
            for (i, delta) in scenario.prefix.iter().enumerate() {
                assert!(
                    model.apply(delta).is_ok(),
                    "cell {cell}: prefix commit {i} rejected"
                );
            }
            let before = model.generation();
            assert!(
                model.apply(&scenario.victim).is_ok(),
                "cell {cell}: the victim commit is rejected"
            );
            assert!(
                model.generation() > before,
                "cell {cell}: the victim commit changed nothing"
            );
            assert!(
                !scenario.victim.inserts.is_empty(),
                "cell {cell}: the victim has no inserts (the F/M/U/R hooks are on the insert path)"
            );
            if cell > 0 {
                assert!(
                    !scenario.victim.deletes.is_empty(),
                    "cell {cell}: a replacement victim must delete"
                );
            }
        }
    }
}
