//! Steady state is unrepresentable to violate: the [`Mix`] carries ONE `churn`
//! field meaning "this many postings enter AND this many leave The churn
//! protocol layer — pure data, pure functions.

use std::collections::BTreeSet;

use bumbledb::Value;

use crate::corpus_gen::{self, GenConfig, Rng, Scale, Sizes};
use crate::schema::{AccountId, InstrumentId, JournalEntryId, Posting, PostingId, ids};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mix {
    pub churn: u64,

    pub updates: u64,

    pub growth: u64,
}

impl Mix {
    #[must_use]
    pub fn removals(&self) -> u64 {
        self.churn + self.updates
    }

    #[must_use]
    pub fn arrivals(&self) -> u64 {
        self.churn + self.updates + self.growth
    }
}

pub const STEADY: Mix = Mix {
    churn: 64,
    updates: 32,
    growth: 0,
};

pub const DELETE_HEAVY: Mix = Mix {
    churn: 512,
    updates: 0,
    growth: 0,
};

pub const DEFAULT_CYCLES: u64 = 10_000;

pub const DEFAULT_SAMPLE_EVERY: u64 = 250;

pub const DEFAULT_VACUUM_EVERY: u64 = 500;

pub const DEFAULT_ANALYZE_EVERY: u64 = 500;

/// The maintenance strides (`vacuum_every`, `analyze_every`) belong to the
/// driver packets; they validate here so a bad schedule refuses before any
/// store exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChurnConfig {
    pub r#gen: GenConfig,

    pub cycles: u64,

    pub sample_every: u64,

    pub vacuum_every: u64,

    pub analyze_every: u64,
}

impl ChurnConfig {
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

/// # Errors
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostingBody {
    pub entry: u64,

    pub account: u64,

    pub instrument: u64,

    pub amount: i64,

    pub at: i64,
}

/// # Panics
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CyclePlan {
    pub updates: Vec<usize>,

    pub deletes: Vec<usize>,

    pub bodies: Vec<PostingBody>,
}

/// # Panics
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

#[derive(Debug)]
pub struct LiveSet {
    rows: Vec<Posting>,
}

impl LiveSet {
    #[must_use]
    pub fn from_corpus(r#gen: GenConfig) -> Self {
        let sizes = Sizes::of(r#gen.scale);
        Self {
            rows: (0..sizes.postings)
                .map(|i| posting_from_row(&corpus_gen::row(&r#gen, &sizes, ids::POSTING, i)))
                .collect(),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    #[must_use]
    pub fn rows(&self) -> &[Posting] {
        &self.rows
    }

    /// deletes' rows, cloned before any mutation.
    #[must_use]
    pub fn resolve(&self, plan: &CyclePlan) -> Vec<Posting> {
        plan.updates
            .iter()
            .chain(plan.deletes.iter())
            .map(|&index| self.rows[index])
            .collect()
    }

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
