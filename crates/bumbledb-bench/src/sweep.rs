//! The T8 commit-size sweep — the probe-order A/B the T8 gravestone
//! owes its curve. `storage/commit/judgment.rs :: check_source`
//! iterates `plan.inserts` in the delta's `(relation, fact_hash)`
//! `BTreeMap` order, so the source-side target probes land in effectively
//! random U-key order; the target and window check lists are already
//! BTree-sorted. T8 found the orders indistinguishable at bench commit
//! sizes but never swept commit size to find where sorting starts
//! paying. This lane sweeps the touched-parent count over ephemeral
//! windowed-twin stores ([`crate::windowed`] — the windowed corpus) and
//! times the judgment spans per commit through the engine's trace seam,
//! comparing today's DELTA order against a KEY-SORTED probe order —
//! without touching the engine: each commit's child ids are
//! **hash-graded** (ground until each fact's identity hash lands in its
//! parent's rank slab of the hash space), so the delta's own hash order
//! IS the engineered probe order. The engine-side sort lands later only
//! if this curve says it pays.
//!
//! THE CURVE, MEASURED — and the sort landed on it (2026-07-17, the W8
//! verdict run: three fresh seeds, 8 samples/cell, ambient 16384×8
//! ephemeral twins, this lane under `scripts/measure.sh`; store
//! DRAM-resident, upper pages cache-warm). Pure probe-order effect,
//! sorted/delta src p50: noise at k ≤ 64 (sign flips seed to seed),
//! 0.91–0.95 at 256, 0.81–0.86 at 1024, 0.75 at 4096 seed-stable —
//! ascending keys share B-tree upper pages across descents, and the
//! effect grows with the touched fraction of the tree. The source-side
//! sort now lives in `judgment.rs :: check_source`, so the engine sorts
//! BOTH arms' probes and this lane survives as the standing falsifier:
//! the printed ratio should sit at ~1.0 (the arms differ only in the
//! sort's input order), and a drift back toward the old curve at the
//! ladder's top means the sort quietly died. The witness pin below
//! moved with the sort — key-least, no longer hash-least.
//!
//! The hash model the grading assumes — canonical fact bytes are the
//! concatenated big-endian field words ([`child_fact_bytes`]); fact
//! identity is the full 32-byte blake3 ([`model_fact_hash`]) — is
//! pinned against the engine at every store's setup
//! ([`pin_hash_model`]): a deliberately rejected commit's surviving
//! witness must be the model's KEY-least violator (the landed sort's
//! discovery order made observable through `Violations::seal`'s stable
//! sort; the model's hash-least probe is a checked-different fact, so a
//! revert to delta-order discovery trips the same pin), or the lane
//! refuses to print numbers rather than mislabel its arms. The
//! twin determinism obligation — the sealed citation LIST is
//! probe-order-invariant; the witness choice is explicitly
//! non-normative — is asserted engine-side in
//! `crates/bumbledb/tests/witness_stability.rs`.
//!
//! Grading is setup cost, never inside a timed span: expected `k` hash
//! trials per child (`k²` per commit — ~17M 24-byte hashes at the
//! 4096-parent point). The measured span is the engine's own
//! `judgment_*` trace spans, summed per commit; the store is ephemeral
//! (`NOSYNC`) so no fsync shadows the judgment numbers.
//!
//! One command, under the measurement mutex:
//! `scripts/measure.sh cargo run --release -p bumbledb-bench --features obs -- sweep-commit`

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use bumbledb::digest::Digest;
use bumbledb::obs;
use bumbledb::{Db, Error, Violation};

use crate::corpus_gen::Rng;
use crate::harness::{Stats, stats};
use crate::windowed::{Mass, load, world};

#[cfg(test)]
mod tests;

/// The default commit-size ladder: touched parents per commit, one
/// inserted child (one source probe) per touched parent.
pub const DEFAULT_SIZES: &[u64] = &[4, 16, 64, 256, 1024, 4096];

/// Sample commits per (size, order) cell.
pub const DEFAULT_SAMPLES: u32 = 8;

/// The sample ceiling: a parent drawn in every sample commit of a cell
/// accumulates one child per sample on top of the seeded 8 — the total
/// must stay under the windowed twin's 64-cap with headroom, or the
/// sweep would measure refusals.
pub const MAX_SAMPLES: u32 = 48;

/// The ambient tree's floor: parents never drop below this, so every
/// cell's probes walk a real tree whatever the ladder's smallest size.
const PARENTS_FLOOR: u64 = 4_096;

/// Ground child ids start far above the seeded id range (seeded ids are
/// `0..parents × 8`), so a ground fact never collides with the corpus
/// and every insert is a genuine delta insert.
const ID_BASE: u64 = 1 << 32;

/// Which probe order a cell engineers through its rank assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeOrder {
    /// The delta's hash order as the sort's INPUT — engineered as a
    /// seeded random rank permutation, so both arms pay identical
    /// grading. Before the W8 sort landed this WAS the probe order;
    /// now `check_source` erases it inside the span, and the arm
    /// carries the sort's random-input cost.
    Delta,
    /// Ascending target-key order as the sort's input — the engine's
    /// probe order either way; the sort sees a pre-sorted worklist.
    KeySorted,
}

impl ProbeOrder {
    /// The arm's name, as the table and scratch paths print it.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Delta => "delta",
            Self::KeySorted => "sorted",
        }
    }
}

/// Canonical fact bytes of one windowed-twin child `(id, parent, flag)`
/// — three big-endian words, the engine's `encoding::encode_fact` for
/// an all-u64 relation. Drift from the engine's encoding is caught by
/// [`pin_hash_model`], never silently mis-graded.
#[must_use]
pub fn child_fact_bytes(id: u64, parent: u64, flag: u64) -> [u8; 24] {
    let mut out = [0u8; 24];
    out[..8].copy_from_slice(&id.to_be_bytes());
    out[8..16].copy_from_slice(&parent.to_be_bytes());
    out[16..].copy_from_slice(&flag.to_be_bytes());
    out
}

/// The fact identity the delta orders by: the full 32-byte blake3 of
/// the canonical fact bytes (`encoding/fact_hash.rs`), reached through
/// the digest seam — the dependency quarantine keeps blake3 itself out
/// of this crate.
#[must_use]
pub fn model_fact_hash(fact_bytes: &[u8]) -> [u8; 32] {
    let mut digest = Digest::new();
    digest.update(fact_bytes);
    digest.finalize()
}

/// The hash's leading word: 32-byte lexicographic order is decided by
/// it whenever two hashes differ there — and the slabs below are
/// disjoint leading-word ranges, so slab rank order IS delta order.
fn hash_rank_word(hash: &[u8; 32]) -> u64 {
    u64::from_be_bytes(hash[..8].try_into().expect("8 bytes"))
}

/// Which of `k` equal slabs of the u64 space a leading word falls in —
/// multiplicative bucketing: an exact, order-preserving partition.
fn slab(word: u64, k: u64) -> u64 {
    u64::try_from((u128::from(word) * u128::from(k)) >> 64).expect("bucket < k")
}

/// Grinds one commit's children: `parents[i]` (ascending) gets a child
/// whose fact hash lands in slab `ranks[i]`, so the delta's hash order
/// visits parents exactly in the arm's engineered order. Expected `k`
/// hash trials per child; ids are consumed monotonically from
/// `next_id`, never reused.
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

/// A random rank permutation (Fisher–Yates over the rng seam) — the
/// delta arm's assignment: hashes as uniformly slabbed as the sorted
/// arm's, parent-visit order random.
fn shuffled_ranks(k: usize, rng: &mut Rng) -> Vec<u64> {
    let mut ranks: Vec<u64> = (0..u64::try_from(k).expect("64-bit usize")).collect();
    for i in (1..k).rev() {
        let j = usize::try_from(rng.range(u64::try_from(i).expect("64-bit usize") + 1))
            .expect("index fits");
        ranks.swap(i, j);
    }
    ranks
}

/// `k` distinct parents drawn from `0..pool`, ascending — the commit's
/// touched-parent set (rejection draws; the pool is ≥ 4× oversized).
fn draw_parents(k: u64, pool: u64, rng: &mut Rng) -> Vec<u64> {
    let want = usize::try_from(k).expect("64-bit usize");
    let mut set = BTreeSet::new();
    while set.len() < want {
        set.insert(rng.range(pool));
    }
    set.into_iter().collect()
}

/// The encoding-and-order pin: a deliberately rejected commit —
/// children under missing parents — whose surviving witness must be the
/// KEY-least violator (the landed W8 source sort's discovery order;
/// `Violations::seal` stable-sorts by citation, keeping the
/// first-discovered witness). The model's hash-least probe is checked
/// to be a DIFFERENT fact, so this refuses both drifts before a single
/// mislabeled number prints: the fact encoding leaving
/// [`child_fact_bytes`] / [`model_fact_hash`], and the source-side sort
/// silently reverting to delta hash order. The rejected commit aborts;
/// the store is untouched.
///
/// # Errors
///
/// The drift refusal (naming the seam to re-derive), an unexpected
/// verdict shape, or an engine error, stringified.
///
/// # Panics
///
/// Never in practice: the probe set is a nonempty constant.
pub fn pin_hash_model(db: &Db<world::WindowedWorld>) -> Result<(), String> {
    // Missing-parent keys far past any pool; probe ids just below the
    // ground range so nothing here collides with a sweep commit.
    const MISSING_BASE: u64 = 1 << 48;
    let probe: Vec<(u64, u64)> = (0..8)
        .map(|i| (ID_BASE - 64 + i, MISSING_BASE + i))
        .collect();
    // Key-least = least parent key: the probes' parents ascend with i.
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
            tx.insert(&world::WChild {
                id: world::WChildId(id),
                parent: world::WParentId(parent),
                flag: 0,
            })?;
        }
        Ok(())
    });
    let Err(Error::CommitRejected { violations }) = outcome else {
        return Err(format!(
            "hash-model pin: the probe commit was not rejected as expected: {outcome:?}"
        ));
    };
    let [
        Violation::Containment {
            direction: bumbledb::Direction::SourceUnsatisfied,
            fact,
            ..
        },
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

/// One commit's judgment spans, summed by name out of a trace capture.
struct JudgmentSpans {
    source: u64,
    windows: u64,
}

fn judgment_spans(events: &[obs::TraceEvent]) -> JudgmentSpans {
    let sum = |name: &str| -> u64 {
        events
            .iter()
            .filter(|event| event.name == name)
            .map(|event| event.dur_ns)
            .sum()
    };
    JudgmentSpans {
        source: sum(obs::names::JUDGMENT_SOURCE),
        windows: sum(obs::names::JUDGMENT_WINDOWS),
    }
}

/// One (size, order) cell's measured spans.
struct Cell {
    src: Stats,
    win: Stats,
}

/// Runs one cell: a fresh ephemeral windowed twin under `dir`, `samples`
/// commits of `k` hash-graded children each, judgment spans per commit.
/// `parents_rng` draws the touched-parent sets — the caller feeds both
/// arms the same seed, so the A/B is paired draw-for-draw and differs
/// only in rank assignment.
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
    std::fs::create_dir_all(dir).map_err(|e| format!("sweep scratch: {e}"))?;
    let db = Db::ephemeral(dir, world::WindowedWorld)
        .map_err(|e| format!("sweep ephemeral create: {e:?}"))?;
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
                tx.insert(&world::WChild {
                    id: world::WChildId(id),
                    parent: world::WParentId(parent),
                    flag: 0,
                })?;
            }
            Ok(())
        });
        let events = obs::finish_capture();
        outcome.map_err(|e| format!("sweep commit (size {k}, {}): {e:?}", order.label()))?;
        let spans = judgment_spans(&events);
        src.push(spans.source);
        win.push(spans.windows);
    }
    Ok(Cell {
        src: stats(&mut src),
        win: stats(&mut win),
    })
}

/// The sweep: for each commit size, both probe-order arms over fresh
/// ephemeral twins of identical ambient mass, rendered as one
/// per-commit-size table (nanoseconds; `src` is the sortable
/// `judgment_source` span, `win` the already-sorted `judgment_windows`
/// control — target side idles, these commits delete nothing).
///
/// # Errors
///
/// Refusals — a non-obs build (spans invisible), out-of-range knobs,
/// the hash-model drift — and engine errors, each naming the remedy.
pub fn run(scratch: &Path, sizes: &[u64], samples: u32, seed: u64) -> Result<String, String> {
    run_with_floor(scratch, sizes, samples, seed, PARENTS_FLOOR)
}

/// [`run`] with the ambient floor exposed — the smoke test shrinks it;
/// the CLI never does (constant ambient is what makes the curve read as
/// commit size, not store size).
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
    // Span visibility: without the engine's trace feature every capture
    // is empty and every number would honestly read zero — refuse with
    // the remedy instead.
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
    // The pool is ≥ 4× the largest commit so parent draws stay sparse;
    // it is CONSTANT across the ladder so the curve reads as commit
    // size against one fixed ambient tree.
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
         (hash-graded child ids); win = the already-sorted window walk, both arms"
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
            // Paired draws: both arms replay the identical parent-set
            // sequence; only the delta arm consumes the shuffle stream.
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
