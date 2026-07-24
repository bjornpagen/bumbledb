//! The churn protocol layer — pure data, pure functions. Every cycle's
//! operations derive from `(seed, cycle, live_len)` alone: no wall
//! clock, no engine state — the resumability law is a property of the
//! function signatures, not a discipline (this module is mechanically
//! banned from `std::time`).
//!
//! Steady state is unrepresentable to violate: the [`Mix`] carries ONE
//! `churn` field meaning "this many postings enter AND this many leave
//! per cycle" — there are no separate insert/delete counts to drift
//! apart; growth is its own explicit field.

use std::collections::BTreeSet;

use bumbledb::Value;

use crate::corpus_gen::{self, GenConfig, Rng, Scale, Sizes};
use crate::schema::{AccountId, InstrumentId, JournalEntryId, Posting, PostingId, ids};

/// The per-cycle operation mix. Per cycle, `churn` fresh postings enter
/// and `churn` live postings leave — steady state by construction (one
/// field, not two counts to keep equal); `updates` live postings swap
/// (delete + fresh reinsert — the recipe-20/attemptText majority write
/// shape); `growth` postings enter with no matching exit (0 = steady,
/// >0 = the slow-growth mode).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mix {
    /// Postings entering AND leaving per cycle — the one steady-state
    /// number.
    pub churn: u64,
    /// Live postings swapped per cycle (delete + fresh reinsert).
    pub updates: u64,
    /// Postings entering with no matching exit per cycle.
    pub growth: u64,
}

impl Mix {
    /// Live postings leaving per cycle: the churn exits plus the
    /// updates' deletes.
    #[must_use]
    pub fn removals(&self) -> u64 {
        self.churn + self.updates
    }

    /// Fresh postings entering per cycle: the churn entries, the
    /// updates' reinserts, and the growth entries.
    #[must_use]
    pub fn arrivals(&self) -> u64 {
        self.churn + self.updates + self.growth
    }
}

/// The documented night-run default: a steady working set with a
/// swap-heavy write shape riding along.
pub const STEADY: Mix = Mix {
    churn: 64,
    updates: 32,
    growth: 0,
};

/// The documented delete-heavy night-run mix: churns half the Tiny
/// working set per cycle — the compact-on-delete vs freelist-growth
/// story.
pub const DELETE_HEAVY: Mix = Mix {
    churn: 512,
    updates: 0,
    growth: 0,
};

/// The night session's default cycle count — sized so the owner's night
/// session finishes: at S scale a steady cycle is two ~5 ms fsync
/// commits, so 10k cycles ≈ minutes per lane.
pub const DEFAULT_CYCLES: u64 = 10_000;
/// The night session's default sampling stride (probes land on cycle
/// boundaries — see [`validate`]).
pub const DEFAULT_SAMPLE_EVERY: u64 = 250;
/// The night session's default `SQLite` VACUUM stride.
pub const DEFAULT_VACUUM_EVERY: u64 = 500;
/// The night session's default `SQLite` ANALYZE stride.
pub const DEFAULT_ANALYZE_EVERY: u64 = 500;

/// One churn run's identity: the corpus config plus the cycle schedule.
/// The maintenance strides (`vacuum_every`, `analyze_every`) belong to
/// the driver packets; they validate here so a bad schedule refuses
/// before any store exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChurnConfig {
    /// The corpus identity the working set generates from.
    pub r#gen: GenConfig,
    /// Total cycles to drive.
    pub cycles: u64,
    /// Probe stride, in cycles.
    pub sample_every: u64,
    /// `SQLite` VACUUM stride, in cycles.
    pub vacuum_every: u64,
    /// `SQLite` ANALYZE stride, in cycles.
    pub analyze_every: u64,
}

impl ChurnConfig {
    /// The cargo-test config: correctness only, milliseconds — `Tiny`
    /// scale, six cycles.
    #[must_use]
    pub fn smoke(seed: u64) -> Self {
        Self {
            r#gen: GenConfig {
                seed,
                scale: Scale::Tiny,
            },
            cycles: 6,
            sample_every: 3,
            vacuum_every: 2,
            analyze_every: 3,
        }
    }
}

/// Refuses a config/mix pair that cannot drive honestly, naming the
/// remedy: positive strides, samples landing on cycle boundaries
/// (resumable-friendly), a non-empty mix, and the working-set floor
/// `postings >= 2 × removals` (distinct-index rejection draws stay
/// cheap; Tiny's 1024 postings admit [`DELETE_HEAVY`] exactly).
///
/// # Errors
///
/// The refusal, naming the offending knob and its remedy.
pub fn validate(cfg: &ChurnConfig, mix: &Mix) -> Result<(), String> {
    if cfg.cycles == 0 {
        return Err("churn: cycles must be positive — set cycles >= 1".to_owned());
    }
    if cfg.sample_every == 0 {
        return Err("churn: sample_every must be positive — set sample_every >= 1".to_owned());
    }
    if !cfg.cycles.is_multiple_of(cfg.sample_every) {
        return Err(format!(
            "churn: cycles ({}) must be a multiple of sample_every ({}) so samples land on \
             cycle boundaries — adjust one of them",
            cfg.cycles, cfg.sample_every
        ));
    }
    if cfg.vacuum_every == 0 {
        return Err("churn: vacuum_every must be positive — set vacuum_every >= 1".to_owned());
    }
    if cfg.analyze_every == 0 {
        return Err("churn: analyze_every must be positive — set analyze_every >= 1".to_owned());
    }
    if mix.arrivals() + mix.removals() == 0 {
        return Err(
            "churn: the mix is empty — set churn, updates, or growth above zero".to_owned(),
        );
    }
    let postings = Sizes::of(cfg.r#gen.scale).postings;
    if postings < 2 * mix.removals() {
        return Err(format!(
            "churn: the working-set floor refuses this mix — {} postings at this scale, but \
             the mix removes {} per cycle and the floor is postings >= 2 x removals (keep \
             distinct-index rejection draws cheap); shrink the mix or grow the scale",
            postings,
            mix.removals()
        ));
    }
    Ok(())
}

/// One seeded posting body (everything but the id), referencing
/// existing corpus rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostingBody {
    /// The referenced `JournalEntry` id.
    pub entry: u64,
    /// The referenced `Account` id.
    pub account: u64,
    /// The referenced `Instrument` id.
    pub instrument: u64,
    /// The posting amount.
    pub amount: i64,
    /// The posting timestamp.
    pub at: i64,
}

/// One seeded replacement body whose `at` is drawn from the corpus's
/// own timestamp span — `AT_BASE + [0, postings × AT_STEP)`. This
/// deliberately differs from `writebench::prepared_posting`'s
/// `1 << 30` law: replacements draw from the SAME span as the corpus,
/// so any fixed probe window's selectivity is STATIONARY across the
/// whole run — the degradation curve reads store state, never workload
/// drift.
///
/// # Panics
///
/// Never in practice: the documented size table keeps every derived
/// value inside `i64`.
pub fn stationary_body(rng: &mut Rng, sizes: &Sizes) -> PostingBody {
    PostingBody {
        entry: rng.range(sizes.entries),
        account: rng.range(sizes.accounts),
        instrument: rng.range(sizes.instruments),
        amount: i64::try_from(1 + rng.range(5_000_000)).expect("fits"),
        at: corpus_gen::AT_BASE
            + i64::try_from(rng.range(
                sizes.postings * u64::try_from(corpus_gen::AT_STEP).expect("positive step"),
            ))
            .expect("fits"),
    }
}

/// One cycle's operations: `updates` and `deletes` are DISTINCT
/// live-set indices (one rejection-sampled distinct set of size
/// `removals()`, split ascending: the first `updates` many become
/// updates, the rest deletes); `bodies` has `arrivals()` entries in the
/// fixed documented order — update replacements first, then churn
/// inserts, then growth inserts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CyclePlan {
    /// Live-set indices whose rows swap (delete + fresh reinsert).
    pub updates: Vec<usize>,
    /// Live-set indices whose rows leave with no replacement.
    pub deletes: Vec<usize>,
    /// The arriving posting bodies, in the documented order.
    pub bodies: Vec<PostingBody>,
}

/// One cycle's plan — a pure function of its arguments (the
/// resumability law: no wall clock, no engine state), seeded per cycle
/// so any cycle's plan regenerates without replaying its predecessors.
///
/// # Panics
///
/// When `live_len` cannot cover the mix's removals — [`validate`]'s
/// working-set floor keeps a driven run away from this.
#[must_use]
pub fn cycle_plan(r#gen: GenConfig, mix: &Mix, cycle: u64, live_len: usize) -> CyclePlan {
    let sizes = Sizes::of(r#gen.scale);
    let mut rng = Rng::new(r#gen.seed ^ 0xC10C_0001 ^ cycle.rotate_left(17));
    let removals = usize::try_from(mix.removals()).expect("64-bit usize");
    assert!(
        removals <= live_len,
        "the churn plan needs {removals} removal targets but only {live_len} rows are live"
    );
    let live = u64::try_from(live_len).expect("fits u64");
    let mut drawn = BTreeSet::new();
    while drawn.len() < removals {
        drawn.insert(rng.range(live));
    }
    let indices: Vec<usize> = drawn
        .into_iter()
        .map(|index| usize::try_from(index).expect("64-bit usize"))
        .collect();
    let update_count = usize::try_from(mix.updates).expect("64-bit usize");
    let updates = indices[..update_count].to_vec();
    let deletes = indices[update_count..].to_vec();
    let arrivals = usize::try_from(mix.arrivals()).expect("64-bit usize");
    let mut bodies = Vec::with_capacity(arrivals);
    for _ in 0..arrivals {
        bodies.push(stationary_body(&mut rng, &sizes));
    }
    CyclePlan {
        updates,
        deletes,
        bodies,
    }
}

/// The driver's brute-force model of the `Posting` relation — the third
/// oracle view. The end gate compares this model, the engine, and the
/// `SQLite` mirrors as one three-way multiset equality
/// ([`super::verify_end::assert_end_state`]): the write-verification
/// pattern extended by representation, not by an extra checker.
#[derive(Debug)]
pub struct LiveSet {
    rows: Vec<Posting>,
}

impl LiveSet {
    /// The corpus's full posting relation, regenerated row by row — the
    /// model starts byte-identical to both stores' load.
    #[must_use]
    pub fn from_corpus(r#gen: GenConfig) -> Self {
        let sizes = Sizes::of(r#gen.scale);
        Self {
            rows: (0..sizes.postings)
                .map(|i| posting_from_row(&corpus_gen::row(&r#gen, &sizes, ids::POSTING, i)))
                .collect(),
        }
    }

    /// Live postings.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether the working set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The live rows (the probes packet reads live ids through this).
    #[must_use]
    pub fn rows(&self) -> &[Posting] {
        &self.rows
    }

    /// The plan's removal targets — the updates' old rows then the
    /// deletes' rows, cloned before any mutation.
    #[must_use]
    pub fn resolve(&self, plan: &CyclePlan) -> Vec<Posting> {
        plan.updates
            .iter()
            .chain(plan.deletes.iter())
            .map(|&index| self.rows[index])
            .collect()
    }

    /// Applies one committed cycle to the model: the merged
    /// updates+deletes index set removed in DESCENDING index order via
    /// `swap_remove` (safe because descending — every index still to
    /// remove is below the one just vacated), then the added postings
    /// appended.
    pub fn apply(&mut self, plan: &CyclePlan, added: Vec<Posting>) {
        let mut removed: Vec<usize> = plan
            .updates
            .iter()
            .chain(plan.deletes.iter())
            .copied()
            .collect();
        removed.sort_unstable();
        for &index in removed.iter().rev() {
            self.rows.swap_remove(index);
        }
        self.rows.extend(added);
    }
}

/// One generator posting row into the typed fact — the six cells are
/// `[U64 id, U64 entry, U64 account, U64 instrument, I64 amount,
/// I64 at]` by the schema's declaration order.
fn posting_from_row(row: &[Value]) -> Posting {
    let [
        Value::U64(id),
        Value::U64(entry),
        Value::U64(account),
        Value::U64(instrument),
        Value::I64(amount),
        Value::I64(at),
    ] = row
    else {
        unreachable!("a Posting row is six cells: four U64 ids, then I64 amount and at")
    };
    Posting {
        id: PostingId(*id),
        entry: JournalEntryId(*entry),
        account: AccountId(*account),
        instrument: InstrumentId(*instrument),
        amount: *amount,
        at: *at,
    }
}
