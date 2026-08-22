//! This lane sweeps the touched-parent count over ephemeral
//! probe-order-invariant; the witness choice is explicitly

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use bumbledb::digest::Digest;
use bumbledb::obs;
use bumbledb::{Admission, Db, Violation};

use crate::corpus_gen::Rng;
use crate::harness::{Stats, stats};
use crate::windowed::{Mass, load, world};

#[cfg(test)]
mod tests;

pub const DEFAULT_SIZES: &[u64] = &[4, 16, 64, 256, 1024, 4096];

pub const DEFAULT_SAMPLES: u32 = 8;

pub const MAX_SAMPLES: u32 = 48;

/// The ambient tree's floor: parents never drop below this, so every cell's
/// probes walk a real tree whatever the ladder's smallest size.
const PARENTS_FLOOR: u64 = 4_096;

const ID_BASE: u64 = 1 << 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeOrder {

    Delta,

    KeySorted,
}

impl ProbeOrder {

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Delta => "delta",
            Self::KeySorted => "sorted",
        }
    }
}

#[must_use]
pub fn child_fact_bytes(id: u64, parent: u64, flag: u64) -> [u8; 24] {
    let mut out = [0u8; 24];
    out[..8].copy_from_slice(&id.to_be_bytes());
    out[8..16].copy_from_slice(&parent.to_be_bytes());
    out[16..].copy_from_slice(&flag.to_be_bytes());
    out
}

#[must_use]
pub fn model_fact_hash(fact_bytes: &[u8]) -> [u8; 32] {
    let mut digest = Digest::new();
    digest.update(fact_bytes);
    digest.finalize()
}

fn hash_rank_word(hash: &[u8; 32]) -> u64 {
    u64::from_be_bytes(hash[..8].try_into().expect("8 bytes"))
}

fn slab(word: u64, k: u64) -> u64 {
    u64::try_from((u128::from(word) * u128::from(k)) >> 64).expect("bucket < k")
}

fn grind_children(parents: &[u64], ranks: &[u64], next_id: &mut u64) -> Vec<(u64, u64)> {
    let k = u64::try_from(parents.len()).expect("64-bit usize");
    parents
        .iter()
        .zip(ranks)
        .map(|(&parent, &rank)| {
            loop {
                let id = *next_id;
                *next_id += 1;
                let hash = model_fact_hash(&child_fact_bytes(id, parent, 0));
                if slab(hash_rank_word(&hash), k) == rank {
                    return (id, parent);
                }
            }
        })
        .collect()
}

fn shuffled_ranks(k: usize, rng: &mut Rng) -> Vec<u64> {
    let mut ranks: Vec<u64> = (0..u64::try_from(k).expect("64-bit usize")).collect();
    for i in (1..k).rev() {
        let j = usize::try_from(rng.range(u64::try_from(i).expect("64-bit usize") + 1))
            .expect("index fits");
        ranks.swap(i, j);
    }
    ranks
}

fn draw_parents(k: u64, pool: u64, rng: &mut Rng) -> Vec<u64> {
    let want = usize::try_from(k).expect("64-bit usize");
    let mut set = BTreeSet::new();
    while set.len() < want {
        set.insert(rng.range(pool));
    }
    set.into_iter().collect()
}

/// to be a DIFFERENT fact, so this refuses both drifts before a single
/// # Errors
/// The drift refusal (naming the seam to re-derive), an unexpected verdict
/// shape, or an engine error, stringified.
/// # Panics
pub fn pin_hash_model(db: &Db<world::WindowedWorld>) -> Result<(), String> {

    const MISSING_BASE: u64 = 1 << 48;
    let probe: Vec<(u64, u64)> = (0..8)
        .map(|i| (ID_BASE - 64 + i, MISSING_BASE + i))
        .collect();

    let expected = probe[0];
    let hash_least = probe
        .iter()
        .copied()
        .min_by_key(|&(id, parent)| model_fact_hash(&child_fact_bytes(id, parent, 0)))
        .expect("nonempty probe");
    if hash_least == expected {
        return Err(
            "hash-model pin: the probe constants stopped discriminating — the model's \
             hash-least violator coincides with the key-least one, so a revert to \
             delta-order discovery would be invisible; re-pick the probe ids"
                .to_owned(),
        );
    }
    let outcome = db.write(|tx| {
        for &(id, parent) in &probe {
            tx.insert([&world::WChild {
                id: world::WChildId(id),
                parent: world::WParentId(parent),
                flag: 0,
            }])?;
        }
        Ok(())
    });
    let Ok(Admission::Rejected(violations)) = outcome else {
        return Err(format!(
            "hash-model pin: the probe commit was not rejected as expected: {outcome:?}"
        ));
    };
    let [
        (
            Violation::Containment {
                direction: bumbledb::Direction::SourceUnsatisfied,
                fact,
                ..
            },
            _,
        ),
    ] = violations.as_slice()
    else {
        return Err(format!(
            "hash-model pin: expected exactly one source-side containment citation, got {violations:?}"
        ));
    };
    let (id, parent) = expected;
    if fact.as_ref() != child_fact_bytes(id, parent, 0).as_slice() {
        return Err(
            "hash-model pin: the surviving witness is not the model's key-least violator — \
             either the canonical fact encoding drifted from the sweep's model (bumbledb \
             encoding/encode.rs; re-derive child_fact_bytes/model_fact_hash) or the \
             source-side sort (judgment.rs::check_source) reverted to delta order; \
             resolve before trusting any sweep number"
                .to_owned(),
        );
    }
    Ok(())
}

struct JudgmentSpans {
    source: u64,
    capacities: u64,
}

fn judgment_spans(events: &[obs::TraceEvent]) -> JudgmentSpans {
    let sum = |point: obs::TracePoint| -> u64 {
        events
            .iter()
            .filter(|event| event.point() == point)
            .map(|event| event.dur_ns())
            .sum()
    };
    JudgmentSpans {
        source: sum(obs::names::JUDGMENT_SOURCE),
        capacities: sum(obs::names::JUDGMENT_CAPACITIES),
    }
}

struct Cell {
    src: Stats,
    win: Stats,
}

fn run_cell(
    dir: &Path,
    mass: Mass,
    order: ProbeOrder,
    k: u64,
    samples: u32,
    parents_rng: &mut Rng,
    shuffle_rng: &mut Rng,
) -> Result<Cell, String> {
    let _ = std::fs::remove_dir_all(dir);
    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("sweep scratch: {e}"))?;
    }
    let db = Db::create_nosync(dir, world::WindowedWorld)
        .map_err(|e| format!("sweep nosync create: {e:?}"))?
        .expect("accepted");
    load(&db, mass)?;
    pin_hash_model(&db)?;
    let mut next_id = ID_BASE;
    let mut src = Vec::with_capacity(samples as usize);
    let mut win = Vec::with_capacity(samples as usize);
    for _ in 0..samples {
        let parents = draw_parents(k, mass.parents, parents_rng);
        let ranks: Vec<u64> = match order {
            ProbeOrder::KeySorted => (0..k).collect(),
            ProbeOrder::Delta => shuffled_ranks(parents.len(), shuffle_rng),
        };
        let children = grind_children(&parents, &ranks, &mut next_id);
        obs::start_capture();
        let outcome = db.write(|tx| {
            for &(id, parent) in &children {
                tx.insert([&world::WChild {
                    id: world::WChildId(id),
                    parent: world::WParentId(parent),
                    flag: 0,
                }])?;
            }
            Ok(())
        });
        let events = obs::finish_capture();
        outcome
            .map_err(|e| format!("sweep commit (size {k}, {}): {e:?}", order.label()))?
            .unwrap();
        let spans = judgment_spans(&events);
        src.push(spans.source);
        win.push(spans.capacities);
    }
    Ok(Cell {
        src: stats(&mut src),
        win: stats(&mut win),
    })
}

/// # Errors
pub fn run(scratch: &Path, sizes: &[u64], samples: u32, seed: u64) -> Result<String, String> {
    run_with_floor(scratch, sizes, samples, seed, PARENTS_FLOOR)
}

fn run_with_floor(
    scratch: &Path,
    sizes: &[u64],
    samples: u32,
    seed: u64,
    parents_floor: u64,
) -> Result<String, String> {
    if sizes.is_empty() {
        return Err("`sweep-commit` needs at least one size (--sizes a,b,c)".to_owned());
    }
    if sizes.contains(&0) {
        return Err("`sweep-commit` sizes are positive touched-parent counts".to_owned());
    }
    if samples == 0 || samples > MAX_SAMPLES {
        return Err(format!(
            "`sweep-commit` --samples must be 1..={MAX_SAMPLES}: the seeded 8 children per \
             parent plus one per sample commit must stay under the windowed twin's 64-cap"
        ));
    }

    obs::start_capture();
    let tracing = obs::capturing();
    let _ = obs::finish_capture();
    if !tracing {
        return Err(
            "`sweep-commit` times the judgment spans through the engine's trace seam — \
             rebuild with the obs feature: \
             scripts/measure.sh cargo run --release -p bumbledb-bench --features obs -- sweep-commit"
                .to_owned(),
        );
    }
    let max = sizes.iter().copied().max().expect("nonempty sizes");

    let mass = Mass {
        parents: (4 * max).max(parents_floor),
        children_per_parent: 8,
    };
    let mut out = String::new();
    let _ = writeln!(
        out,
        "T8 commit-size sweep — judgment spans by touched-parent count (ns)"
    );
    let _ = writeln!(
        out,
        "world: windowed twin, ephemeral; ambient {} parents x {} children/parent; \
         seed {seed}; {samples} samples/cell",
        mass.parents, mass.children_per_parent
    );
    let _ = writeln!(
        out,
        "arms: delta = today's hash-order source probes; sorted = key-sorted probe order \
         (hash-graded child ids); win = the already-sorted capacity walk, both arms"
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "{:>6} | {:>13} {:>14} {:>12} | {:>13} {:>14} | {:>13} {:>14}",
        "size",
        "src p50 delta",
        "src p50 sorted",
        "sorted/delta",
        "src min delta",
        "src min sorted",
        "win p50 delta",
        "win p50 sorted"
    );
    for &k in sizes {
        let mut cells: Vec<Cell> = Vec::with_capacity(2);
        for order in [ProbeOrder::Delta, ProbeOrder::KeySorted] {
            eprintln!("sweep: size {k}, {} order", order.label());

            let mut parents_rng = Rng::new(seed ^ k.rotate_left(17));
            let mut shuffle_rng = Rng::new(seed ^ k.rotate_left(31) ^ 0xD155);
            let dir = scratch.join(format!("s{k}-{}", order.label()));
            cells.push(run_cell(
                &dir,
                mass,
                order,
                k,
                samples,
                &mut parents_rng,
                &mut shuffle_rng,
            )?);
        }
        let (delta, sorted) = (&cells[0], &cells[1]);
        #[expect(
            clippy::cast_precision_loss,
            reason = "reporting accepts lossy integer-to-float conversion"
        )]
        let ratio = sorted.src.p50 as f64 / delta.src.p50.max(1) as f64;
        let _ = writeln!(
            out,
            "{k:>6} | {:>13} {:>14} {ratio:>11.3}x | {:>13} {:>14} | {:>13} {:>14}",
            delta.src.p50,
            sorted.src.p50,
            delta.src.min,
            sorted.src.min,
            delta.win.p50,
            sorted.win.p50
        );
    }
    Ok(out)
}
