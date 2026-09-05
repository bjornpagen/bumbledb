//! HASH-02: forced local-fingerprint collisions through every equality,
//! admission, query, delete, reopen and spill path.
//!
//! Production 16-byte fingerprints cannot be collided by test inputs alone,
//! so the engine must expose a test-only fingerprint override (requested from
//! P01/P02: a `collision-probe`-style feature or injectable fingerprinter in
//! the core digest/storage seam — see the P14 packet file's dependency list).
//! This module owns everything that does not depend on that hook:
//!
//! - the deterministic adversarial operation schedule ([`schedule`]),
//! - the independent final-state oracle (a `BTreeMap`-keyed relation with a
//!   unique key law over plain integers — it never touches production
//!   equality, hashing or encoding, and the hash under attack plays no role
//!   in its ordering),
//! - the bounded-work checker (a collision bucket may add lookup work, never
//!   unbounded work, and never merges two distinct facts),
//! - the injection surface ([`EngineOps`]) that the F3 wiring implements over
//!   the real engine with the override engaged.
//!
//! Long values above LMDB's key-size bound belong in the same lane: the
//! schedule includes oversized-payload rows so collision buckets are checked
//! where exact comparison must fetch, not inline, the stored fact.

/// A probe row: `(key, payload_class)`. `payload_class` selects the encoded
/// payload the engine wiring writes — 0 = small inline, 1 = exactly at the
/// long-key boundary, 2 = above it (overflow representation).
pub type Row = (i64, u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Insert(Row),
    Delete(Row),
    /// Membership check; the checker compares against the model.
    Contains(Row),
    /// Two same-key candidate rows in one command — the unique-key law must
    /// judge them against each other and the store, with exact bytes deciding
    /// identity even though every fingerprint is equal.
    ConflictingPair(Row, Row),
    /// Close and reopen the store; collision buckets must survive the trip.
    Reopen,
    /// Clamp the scratch budget so the following operations spill to
    /// temporary LMDB; results must not change.
    ForceSpill,
    /// Distinct-count over everything — grouping/dedup under collisions.
    CountDistinct,
}

/// splitmix64, kept local so the schedule never depends on production code.
fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The deterministic adversarial schedule. `keys` bounds the key universe so
/// present/absent operations both occur; reopen and spill points are fixed
/// fractions of the stream so every path sees populated collision buckets.
///
/// # Panics
/// On `keys < 2` — the schedule needs present and absent keys.
#[must_use]
pub fn schedule(seed: u64, ops: usize, keys: u64) -> Vec<Op> {
    assert!(keys >= 2, "the schedule needs present and absent keys");
    let mut state = seed ^ 0x4841_5348_3032_5F31; // "HASH02_1"
    let mut out = Vec::with_capacity(ops + 4);
    for index in 0..ops {
        let key = i64::try_from(next(&mut state) % keys).expect("bounded");
        let class = u8::try_from(next(&mut state) % 3).expect("bounded");
        let row = (key, class);
        let draw = next(&mut state) % 100;
        out.push(match draw {
            0..=44 => Op::Insert(row),
            45..=64 => Op::Delete(row),
            65..=84 => Op::Contains(row),
            85..=94 => {
                // Alternate genuine same-key conflicts (different payload
                // class, must be rejected on exact bytes) with distinct-key
                // pairs (clean within themselves; the complete-final-state
                // judgment still checks each against the store).
                let other = if draw.is_multiple_of(2) {
                    (key, (class + 1) % 3)
                } else {
                    ((key + 1) % i64::try_from(keys).expect("bounded"), class)
                };
                Op::ConflictingPair(row, other)
            }
            _ => Op::CountDistinct,
        });
        if index == ops / 3 {
            out.push(Op::Reopen);
        }
        if index == (2 * ops) / 3 {
            out.push(Op::ForceSpill);
        }
    }
    out.push(Op::Reopen);
    out.push(Op::CountDistinct);
    out
}

/// The independent oracle: a relation with a **unique key law on the first
/// column**, modeled as a `BTreeMap` over plain integers. Ordering and
/// equality come from `i64`/`u8` `Ord` — the fingerprint under attack cannot
/// influence it. The judgment is complete-final-state: a command whose
/// proposed final state violates the key law commits nothing.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Model {
    rows: std::collections::BTreeMap<i64, u8>,
}

/// One command's judged outcome in the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Judged {
    /// Committed (including normalized no-ops).
    Committed,
    /// The whole command was rejected by the key law; nothing changed.
    Rejected,
}

impl Model {
    /// One-row insert command: rejected when a different-class row already
    /// holds the key; a no-op when the identical row exists.
    pub fn insert(&mut self, row: Row) -> Judged {
        match self.rows.get(&row.0) {
            Some(&class) if class != row.1 => Judged::Rejected,
            _ => {
                self.rows.insert(row.0, row.1);
                Judged::Committed
            }
        }
    }

    /// One-row delete command: removes only the exact row; deleting an
    /// absent or different-class row is a committed no-op, not a rejection.
    pub fn delete(&mut self, row: Row) -> Judged {
        if self.rows.get(&row.0) == Some(&row.1) {
            self.rows.remove(&row.0);
        }
        Judged::Committed
    }

    #[must_use]
    pub fn contains(&self, row: Row) -> bool {
        self.rows.get(&row.0) == Some(&row.1)
    }

    #[must_use]
    pub fn distinct(&self) -> u64 {
        self.rows.len() as u64
    }

    /// Two-row insert command, judged as one proposed final state: rejected
    /// when the pair conflicts within itself (same key, different class) or
    /// either row conflicts with the store; otherwise both land (identical
    /// rows normalize).
    pub fn pair(&mut self, a: Row, b: Row) -> Judged {
        if Self::pair_conflicts((a, b)) || self.would_conflict(a) || self.would_conflict(b) {
            return Judged::Rejected;
        }
        self.rows.insert(a.0, a.1);
        self.rows.insert(b.0, b.1);
        Judged::Committed
    }

    fn would_conflict(&self, row: Row) -> bool {
        matches!(self.rows.get(&row.0), Some(&class) if class != row.1)
    }

    /// The within-command unique-key conflict: equal keys, different payload
    /// classes. Exact bytes decide — equal rows normalize to one.
    #[must_use]
    pub fn pair_conflicts(pair: (Row, Row)) -> bool {
        pair.0.0 == pair.1.0 && pair.0.1 != pair.1.1
    }
}

/// What the F3 wiring implements over the real engine with the fingerprint
/// override engaged. Every method reports `(rejected?, probes)` where
/// `probes` counts exact-comparison fetches so bounded-work is checkable.
pub struct EngineOps<'a> {
    /// One-row insert command → (rejected by the key law, probes).
    pub insert: &'a mut dyn FnMut(Row) -> Result<(bool, u64), String>,
    /// One-row delete command → probes (a missing row is a no-op).
    pub delete: &'a mut dyn FnMut(Row) -> Result<u64, String>,
    pub contains: &'a mut dyn FnMut(Row) -> Result<(bool, u64), String>,
    /// Submit both rows in one command → (rejected, probes).
    pub conflicting_pair: &'a mut dyn FnMut(Row, Row) -> Result<(bool, u64), String>,
    pub reopen: &'a mut dyn FnMut() -> Result<(), String>,
    pub force_spill: &'a mut dyn FnMut() -> Result<(), String>,
    pub count_distinct: &'a mut dyn FnMut() -> Result<(u64, u64), String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkBound {
    /// Max exact-comparison probes tolerated per operation as a multiple of
    /// the live colliding-bucket size (constant fingerprints put every live
    /// row in one bucket, so this is a linear bound, never quadratic).
    pub probes_per_bucket_row: u64,
    pub bucket_floor: u64,
}

impl WorkBound {
    #[must_use]
    pub const fn allows(self, probes: u64, live_rows: u64) -> bool {
        probes <= self.probes_per_bucket_row * live_rows + self.bucket_floor
    }
}

/// Drive one schedule against the engine and the model together. Any
/// divergence, unexpected error or bound violation is a failure with the
/// exact op index. This runs only in F3 (the wiring needs a built engine),
/// but its logic is complete now.
///
/// # Errors
pub fn drive(plan: &[Op], engine: &mut EngineOps<'_>, bound: WorkBound) -> Result<(), String> {
    let mut model = Model::default();
    for (index, op) in plan.iter().enumerate() {
        let fail = |what: &str| format!("op {index} ({op:?}): {what}");
        match *op {
            Op::Insert(row) => {
                let (rejected, probes) = (engine.insert)(row).map_err(|e| fail(&e))?;
                let expected = model.insert(row) == Judged::Rejected;
                if rejected != expected {
                    return Err(fail(&format!(
                        "insert judged rejected={rejected}, the independent key-law model \
                         says rejected={expected}"
                    )));
                }
                if !bound.allows(probes, model.distinct() + 1) {
                    return Err(fail(&format!("insert probes {probes} exceed the bound")));
                }
            }
            Op::Delete(row) => {
                let probes = (engine.delete)(row).map_err(|e| fail(&e))?;
                model.delete(row);
                if !bound.allows(probes, model.distinct() + 1) {
                    return Err(fail(&format!("delete probes {probes} exceed the bound")));
                }
            }
            Op::Contains(row) => {
                let (present, probes) = (engine.contains)(row).map_err(|e| fail(&e))?;
                if present != model.contains(row) {
                    return Err(fail(&format!(
                        "contains says {present}, the independent model says {}",
                        model.contains(row)
                    )));
                }
                if !bound.allows(probes, model.distinct()) {
                    return Err(fail(&format!("contains probes {probes} exceed the bound")));
                }
            }
            Op::ConflictingPair(a, b) => {
                let (rejected, probes) = (engine.conflicting_pair)(a, b).map_err(|e| fail(&e))?;
                let expected = model.pair(a, b) == Judged::Rejected;
                if rejected != expected {
                    return Err(fail(&format!(
                        "pair judged rejected={rejected}, the independent key-law model \
                         says rejected={expected}"
                    )));
                }
                if !bound.allows(probes, model.distinct() + 2) {
                    return Err(fail(&format!("pair probes {probes} exceed the bound")));
                }
            }
            Op::Reopen => (engine.reopen)().map_err(|e| fail(&e))?,
            Op::ForceSpill => (engine.force_spill)().map_err(|e| fail(&e))?,
            Op::CountDistinct => {
                let (count, probes) = (engine.count_distinct)().map_err(|e| fail(&e))?;
                if count != model.distinct() {
                    return Err(fail(&format!(
                        "distinct count {count}, the independent model says {}",
                        model.distinct()
                    )));
                }
                // A full distinct pass may touch every live row once-ish.
                if !bound.allows(probes, model.distinct().max(1) * 2) {
                    return Err(fail(&format!("distinct probes {probes} exceed the bound")));
                }
            }
        }
    }
    Ok(())
}
