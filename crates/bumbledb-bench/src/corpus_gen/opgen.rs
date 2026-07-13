//! The op-stream arm (docs/architecture/60-validation.md § the fuzzing
//! charter; the crucible packet (git ecec1dc3)): "a legal interaction with
//! a bumbledb store" reified as a generated op sequence — the flagship
//! lifecycle fuzz target's generation half. The runner (the `ops` lane
//! in the detached `fuzz/` crate) replays the sequence against the live
//! engine with the naive model in lockstep; nothing here knows a
//! verdict — the generator draws shapes, the two oracles judge
//! (refusal: a generator that knows the rules can only confirm them).
//!
//! The theory is the querygen target ledger (`querygen::target`) so the
//! query pool comes straight from [`random_query`]; the data world is a
//! shrunken [`Domains`] drawn per scenario, streamed through the same
//! [`target::corpus_row`] functions the seeded lanes use — every draw
//! rides the entropy seam ([`Rng`]), so fuzzer bytes steer the whole
//! scenario and the byte string IS the reproduction.

use bumbledb::{Query, RelationId, Value};

use crate::naive::{Delta, ParamValue};
use crate::querygen::target::{self, Domains};
use crate::querygen::writes::closed_write_cases;
use crate::querygen::{ParamDraw, interval_data, params_for, random_query};

use super::{GenConfig, Rng, Scale};

/// The op alphabet — the ten lifecycle verbs. Batches STAGE facts into
/// a pending delta (batching is the transaction); `Commit` is the one
/// verb that judges, `Rollback` the one that abandons. The runner's
/// model-mapping table (fuzz/src/lib.rs, the `ops` runner) states what
/// each verb means on the engine and on the naive model.
#[derive(Debug, Clone)]
pub enum FuzzOp {
    /// Stage inserts into the pending delta.
    InsertBatch(Delta),
    /// Stage deletes into the pending delta.
    DeleteBatch(Delta),
    /// Stage deletes and inserts together.
    MixedBatch(Delta),
    /// Send the pending delta through one write transaction — the
    /// dependency judgment fires here, verdicts compared typed.
    Commit,
    /// Abandon the pending delta: the engine applies it inside a write
    /// closure and returns `Err` (the documented abandon), the model
    /// discards it — both sides must be untouched.
    Rollback,
    /// Execute a pooled prepared query with live params.
    Execute {
        slot: usize,
        params: Vec<ParamValue>,
    },
    /// Re-prepare one pool slot from its `Query` — the prepared-state
    /// lifecycle verb.
    Reprepare { slot: usize },
    /// Read one relation's full contents through a snapshot scan.
    ViewRead { relation: RelationId },
    /// Drop the environment and reopen the store from disk (the pending
    /// delta and the prepared pool die with the env).
    Reopen,
    /// Run the store's own internal auditor.
    VerifyStore,
}

/// One generated lifecycle scenario: the prepared-query pool and the op
/// sequence over it. `Execute`/`Reprepare` slots index `queries`.
#[derive(Debug, Clone)]
pub struct OpScenario {
    pub queries: Vec<Query>,
    pub ops: Vec<FuzzOp>,
}

/// A whole scenario from one entropy stream, Tiny-bounded: a seed world
/// (corpus-valid by construction, committed first so later judgments
/// run against real state), a 1–3 query pool, then 6–24 drawn steps.
pub fn random_scenario(rng: &mut Rng) -> OpScenario {
    let cfg = GenConfig {
        seed: rng.u64(),
        scale: Scale::Tiny,
    };
    let world = world(rng);
    let queries: Vec<Query> = (0..=rng.range(3)).map(|_| random_query(rng, cfg)).collect();
    let mut ops = vec![FuzzOp::InsertBatch(seed_world(cfg, &world)), FuzzOp::Commit];
    for _ in 0..6 + rng.range(19) {
        ops.push(step(rng, cfg, &world, &queries));
    }
    OpScenario { queries, ops }
}

/// One generated crash scenario (the crucible packet (git ecec1dc3)):
/// an ops prefix of commit-shaped deltas (each one write transaction)
/// and ONE victim commit — the commit the crash harness kills at a
/// drawn crashpoint. The generator draws shapes only; which crashpoints
/// the victim actually reaches is the engine's business (a delta with
/// no inserts never lies on the insert-path hooks — the runner's
/// clean-exit arm owns that case).
#[derive(Debug, Clone)]
pub struct CrashScenario {
    pub prefix: Vec<Delta>,
    pub victim: Delta,
}

/// A crash scenario from one entropy stream, Tiny-bounded: the seed
/// world as the opening prefix commit (real state on both sides), 0–2
/// further mixed commits, then an insert-leaning victim (the
/// namespace-write hooks lie on the insert path).
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

/// The deterministic crash-sweep matrix's cell count — the small
/// ops-prefix matrix every crashpoint is killed against at least once
/// (the sweep in `fuzz/tests/crash.rs`), never left to fuzzer luck.
pub const CRASH_MATRIX_CELLS: usize = 3;

/// One deterministic sweep cell: a fixed ops prefix and a victim commit
/// CONSTRUCTED to lie on every crashpoint's path — accepted by the
/// judgment (the co-located test pins it against the naive model) and
/// touching every namespace family (inserts with guards and edges,
/// deletes, moved row counts). The victims are whole-world replacements:
/// delete one complete seed world, insert another — the final state is
/// exactly the incoming seed world, valid by construction.
///
/// # Panics
///
/// On a cell index at or beyond [`CRASH_MATRIX_CELLS`].
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

/// The sweep's fixed world ladder: strictly growing domains (every axis
/// differs between steps, so a whole-world replacement always moves at
/// least one row count — the `mid-write-s` hook's path) with per-step
/// seeds (so replaced rows genuinely differ, never a no-op delta).
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

/// A whole-world replacement delta: delete every fact of one seed
/// world, insert every fact of another — judged against the final
/// state, which is exactly the incoming world.
fn replace(from: &Delta, to: Delta) -> Delta {
    Delta {
        deletes: from.inserts.clone(),
        inserts: to.inserts,
    }
}

/// One drawn step — the weighted alphabet. Every verb is reachable
/// (the co-located coverage test pins it); commits outweigh the rest so
/// staged batches actually reach the judgment.
fn step(rng: &mut Rng, cfg: GenConfig, world: &Domains, queries: &[Query]) -> FuzzOp {
    match rng.range(20) {
        0..=3 => FuzzOp::InsertBatch(batch(rng, cfg, world, Kind::Inserts)),
        4..=5 => FuzzOp::DeleteBatch(batch(rng, cfg, world, Kind::Deletes)),
        6..=7 => FuzzOp::MixedBatch(batch(rng, cfg, world, Kind::Mixed)),
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

/// The scenario's data world: a shrunken domain table in the corpus
/// ladder's shape ([`Domains::of`]) — small enough that the naive
/// model's nested loops stay inside the Tiny per-iteration budget,
/// large enough that every relation has witnesses.
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

/// Every writable relation's full corpus stream as ONE delta — valid by
/// construction (references in-domain, the DU pair and the domain
/// quantification's backings land together), so the opening commit
/// establishes real state on both sides.
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

/// One staged batch of 1–3 facts. A tenth of the facts route through
/// the closed/judgment write-case generator ([`closed_write_cases`] —
/// the existing write-case arm, reused whole); the rest draw from the
/// corpus row functions under the fact policies below.
fn batch(rng: &mut Rng, cfg: GenConfig, world: &Domains, kind: Kind) -> Delta {
    let mut delta = Delta::default();
    for _ in 0..=rng.range(3) {
        if rng.chance(1, 10) {
            // The closed-relation surface: closed writes, dangling
            // handles, roster-cap and ψ-subset misses — all six kinds.
            let mut cases = closed_write_cases(rng, 6);
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

/// One insert draw. Policies (the generator owns no validity logic —
/// each one merely BIASES toward a verdict class, the engine judges):
/// growth rows just past the world (mostly commit; an import-source
/// entry pairs its `ImportBatch` sibling half the time, the DU
/// judgment fires the other half), re-inserts of existing rows
/// (no-ops), a twisted-seed twin of an existing row (same id, different
/// payload — the key judgments), and a row drawn against inflated
/// domains (dangling references — source-unsatisfied containments).
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

/// One delete draw: an in-world row (a real delete — target-required
/// containments may fire) or a just-past-the-world row (an absent-fact
/// no-op), both from the same corpus functions the inserts use.
fn push_delete(rng: &mut Rng, cfg: GenConfig, world: &Domains, delta: &mut Delta) {
    let rel = ordinary_relation(rng);
    let rows = target::corpus_rows(world, rel).max(1);
    let i = rng.range(rows + 2);
    delta
        .deletes
        .push((rel, target::corpus_row(cfg, world, rel, i)));
}

/// One prepared-execution draw: a pool slot plus live params from the
/// query's own param generator ([`params_for`] — hits, boundaries, and
/// misses), one of its draws picked per execution.
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

/// One randomized draw as positional [`ParamValue`]s (dense `ParamId`s).
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

/// A writable (ordinary) relation. The closed relations are ground
/// axioms — their write surface is the closed-case arm in [`batch`],
/// and their contents are schema, not store state, so the view-read
/// and reopen comparisons range over the ordinary relations.
fn ordinary_relation(rng: &mut Rng) -> RelationId {
    RelationId(u32::try_from(rng.range(u64::from(target::TARGET_RELATIONS))).expect("relation id"))
}

/// References beyond the world's domains — a growth row drawn against
/// this table mostly dangles.
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
            FuzzOp::InsertBatch(_) => "insert",
            FuzzOp::DeleteBatch(_) => "delete",
            FuzzOp::MixedBatch(_) => "mixed",
            FuzzOp::Commit => "commit",
            FuzzOp::Rollback => "rollback",
            FuzzOp::Execute { .. } => "execute",
            FuzzOp::Reprepare { .. } => "reprepare",
            FuzzOp::ViewRead { .. } => "viewread",
            FuzzOp::Reopen => "reopen",
            FuzzOp::VerifyStore => "verifystore",
        }
    }

    /// The arm is deterministic in its entropy: the same byte string
    /// yields the identical scenario, and a different one steers away.
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

    /// Every one of the ten verbs is reachable across a modest seed
    /// sweep — an alphabet with unreachable letters fuzzes less than it
    /// claims.
    #[test]
    fn the_alphabet_reaches_all_ten_verbs() {
        let mut seen = std::collections::BTreeSet::new();
        for seed in 0..256u64 {
            let scenario = random_scenario(&mut Rng::new(seed));
            assert!(
                matches!(
                    scenario.ops.as_slice(),
                    [FuzzOp::InsertBatch(_), FuzzOp::Commit, ..]
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

    /// The crash arm is deterministic in its entropy, and its victim is
    /// never empty (an empty victim is the no-op commit — off every
    /// crashpoint's path by the empty-delta gate).
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

    /// The sweep matrix's contract, pinned against the naive model:
    /// every cell's prefix commits are accepted, and every cell's victim
    /// is accepted AND state-changing (the generation moves) — so all
    /// ten crashpoints lie on the victim's path, deterministically,
    /// never by fuzzer luck. Cells past 0 also exercise both delta
    /// directions.
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
