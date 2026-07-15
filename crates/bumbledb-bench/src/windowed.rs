//! The window-judgment write lane — the roster extension's measurement
//! infrastructure for the cardinality window
//! (`docs/architecture/30-dependencies.md` § cardinality window): what
//! does ADMISSION cost when the schema carries window statements and
//! every commit's touched parents must be counted?
//!
//! Three engine-only report rows over two twin worlds:
//! - `commit_window_baseline` — the twin theory WITHOUT window
//!   statements (same relations, same containment, same seeded mass):
//!   the control every window number is read against;
//! - `commit_window_admission` — one child insert per commit under the
//!   `WParent(id) <={0..64} WChild(parent)` window: the touched-
//!   parent count probe on the hot path;
//! - `commit_window_exclusion` — one φ-selected child (`flag == 1`)
//!   per commit under the **`{0}` exclusion**
//!   `WParent(id | kind == 1) <={0} WChild(parent | flag == 1)`
//!   (the `{0}` exclusion: no selected child may exist per selected parent —
//!   the exclusion window as a count), inserted under unselected
//!   parents so every commit is legal and the measured cost is the
//!   judge, not a refusal.
//!
//! `SQLite`-unpaired by decision (the `commit_witnessed` precedent): a
//! trigger emulation would time the emulation, not the engine. Naive
//! parity runs in tests — [`crate::naive::NaiveDb`] judges the same
//! windows, verdicts and citations compared through the differential
//! runner.

use std::path::Path;

use bumbledb::{Db, RelationId, Value};

use crate::corpus_gen::{GenConfig, Rng};
use crate::harness::{self, Measurement};
use crate::writebench::write_protocol;

#[cfg(test)]
mod tests;

/// The windowed twin: the containment plus the two window statements —
/// the bounded fan-cap and the `{0}` exclusion.
pub mod world {
    bumbledb::schema! {
        pub WindowedWorld;

        relation WParent {
            id: u64 as WParentId, fresh,
            kind: u64,
        }
        relation WChild {
            id: u64 as WChildId, fresh,
            parent: u64 as WParentId,
            flag: u64,
        }

        WChild(parent) <= WParent(id);
        WParent(id) <={0..64} WChild(parent);
        WParent(id | kind == 1) <={0} WChild(parent | flag == 1);
    }
}

/// The control twin: same relations, same containment, NO windows —
/// the admission delta prices the window judge alone.
pub mod baseline {
    bumbledb::schema! {
        pub UnwindowedWorld;

        relation WParent {
            id: u64 as WParentId, fresh,
            kind: u64,
        }
        relation WChild {
            id: u64 as WChildId, fresh,
            parent: u64 as WParentId,
            flag: u64,
        }

        WChild(parent) <= WParent(id);
    }
}

/// Relation ids (both twins declare identically).
pub mod ids {
    use bumbledb::RelationId;

    pub const PARENT: RelationId = RelationId(0);
    pub const CHILD: RelationId = RelationId(1);
}

/// The seeded mass — parameterized so the naive parity slice can
/// shrink every axis (the brute-force model is O(parents × children)
/// per judged delta; the unit-corpus discipline every naive lane
/// follows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mass {
    pub parents: u64,
    /// Seeded children per parent (far under the 64 cap, so every
    /// measured commit is legal).
    pub children_per_parent: u64,
}

impl Mass {
    /// The timed lane's mass: enough parents that per-sample probes hit
    /// a real tree, enough children that every touched parent's count
    /// probe walks occupied pages — "under load", not an empty store.
    pub const BENCH: Self = Self {
        parents: 4_096,
        children_per_parent: 8,
    };

    /// The naive slice's unit mass.
    #[must_use]
    pub const fn unit() -> Self {
        Self {
            parents: 16,
            children_per_parent: 4,
        }
    }
}

/// The timed lane's parent count (the sample RNG's draw domain).
pub const PARENTS: u64 = Mass::BENCH.parents;

/// Every eighth parent is ψ-selected by the exclusion (`kind == 1`).
#[must_use]
pub fn parent_kind(i: u64) -> u64 {
    u64::from(i.is_multiple_of(8))
}

/// One relation's seeded row stream (both twins share it — the corpus
/// is the theory-independent mass).
#[must_use]
pub fn relation_rows(mass: Mass, rel: RelationId) -> Box<dyn Iterator<Item = Vec<Value>>> {
    match rel {
        ids::PARENT => {
            Box::new((0..mass.parents).map(|i| vec![Value::U64(i), Value::U64(parent_kind(i))]))
        }
        ids::CHILD => Box::new((0..mass.parents * mass.children_per_parent).map(move |i| {
            // Seeded children carry flag 0: the exclusion's source
            // selection matches nothing at load, so the seed commits
            // under both twins identically.
            vec![
                Value::U64(i),
                Value::U64(i / mass.children_per_parent),
                Value::U64(0),
            ]
        })),
        _ => unreachable!("two windowed relations"),
    }
}

/// Loads one twin's seeded mass (schema-generic: the corpus is shared).
///
/// # Errors
///
/// Engine errors, stringified.
pub fn load<S>(db: &Db<S>, mass: Mass) -> Result<(), String> {
    for rel in [ids::PARENT, ids::CHILD] {
        db.bulk_load_dyn(rel, relation_rows(mass, rel))
            .map_err(|e| format!("windowed load: {e:?}"))?;
    }
    Ok(())
}

/// A parent the exclusion does NOT select (`kind == 0`) — rejection
/// draw at 7/8 acceptance.
fn unselected_parent(rng: &mut Rng) -> u64 {
    loop {
        let p = rng.range(PARENTS);
        if parent_kind(p) == 0 {
            return p;
        }
    }
}

/// `commit_window_admission`: one flag-0 child per commit under the
/// full window roster.
///
/// # Errors
///
/// Engine errors, stringified.
pub fn commit_window_admission(db: &Db<world::WindowedWorld>) -> Result<Measurement, String> {
    let mut rng = Rng::new(0x0117_0001);
    harness::measure(write_protocol("commit_window_admission"), || {
        let parent = world::WParentId(rng.range(PARENTS));
        db.write(|tx| {
            let id: world::WChildId = tx.alloc()?;
            tx.insert(&world::WChild {
                id,
                parent,
                flag: 0,
            })
        })
        .map(|_| 1)
        .map_err(|e| format!("commit_window_admission: {e:?}"))
    })
}

/// `commit_window_baseline`: the identical insert against the
/// window-free twin — the control.
///
/// # Errors
///
/// Engine errors, stringified.
pub fn commit_window_baseline(db: &Db<baseline::UnwindowedWorld>) -> Result<Measurement, String> {
    let mut rng = Rng::new(0x0117_0001);
    harness::measure(write_protocol("commit_window_baseline"), || {
        let parent = baseline::WParentId(rng.range(PARENTS));
        db.write(|tx| {
            let id: baseline::WChildId = tx.alloc()?;
            tx.insert(&baseline::WChild {
                id,
                parent,
                flag: 0,
            })
        })
        .map(|_| 1)
        .map_err(|e| format!("commit_window_baseline: {e:?}"))
    })
}

/// `commit_window_exclusion`: one φ-selected (`flag == 1`) child per
/// commit under an unselected parent — the `{0}` exclusion's judge on
/// the hot path, every commit legal.
///
/// # Errors
///
/// Engine errors, stringified.
pub fn commit_window_exclusion(db: &Db<world::WindowedWorld>) -> Result<Measurement, String> {
    let mut rng = Rng::new(0x0117_0002);
    harness::measure(write_protocol("commit_window_exclusion"), || {
        let parent = world::WParentId(unselected_parent(&mut rng));
        db.write(|tx| {
            let id: world::WChildId = tx.alloc()?;
            tx.insert(&world::WChild {
                id,
                parent,
                flag: 1,
            })
        })
        .map(|_| 1)
        .map_err(|e| format!("commit_window_exclusion: {e:?}"))
    })
}

/// The lane: seed both twins under `scratch`, run the three rows —
/// engine-only [`crate::report::WriteFamilyReport`]s, `theirs: None`
/// (unpaired by decision).
///
/// # Errors
///
/// Refusals and engine errors, stringified.
pub fn write_families(
    _cfg: GenConfig,
    scratch: &Path,
    selected: &dyn Fn(&str) -> bool,
    mode: crate::storemode::StoreMode,
) -> Result<Vec<crate::report::WriteFamilyReport>, String> {
    let names = [
        "commit_window_admission",
        "commit_window_baseline",
        "commit_window_exclusion",
    ];
    if !names.iter().any(|name| selected(name)) {
        return Ok(Vec::new());
    }
    // The caller (driver::write_families) already asserted the scratch
    // is disk-backed; these rows are fsync-bound like every commit row.
    std::fs::create_dir_all(scratch).map_err(|e| format!("windowed scratch: {e}"))?;
    eprintln!("bench: loading the windowed twin worlds");
    let windowed = mode.create(&scratch.join("windowed"), world::WindowedWorld)?;
    load(&windowed, Mass::BENCH)?;
    let unwindowed = mode.create(&scratch.join("baseline"), baseline::UnwindowedWorld)?;
    load(&unwindowed, Mass::BENCH)?;

    let mut out = Vec::new();
    let mut push =
        |name: &str, run: &mut dyn FnMut() -> Result<Measurement, String>| -> Result<(), String> {
            if !selected(name) {
                return Ok(());
            }
            eprintln!("bench: {name}");
            let (ours, ghz) = crate::clockproxy::stamped(run)?;
            out.push(crate::report::WriteFamilyReport {
                name: name.to_owned(),
                ours: ours.stats,
                theirs: None,
                facts_per_sec: None,
                ghz: Some(crate::report::GhzReport {
                    pre: ghz.pre,
                    post: ghz.post,
                    retried: ghz.retried,
                    contaminated: ghz.contaminated(),
                }),
            });
            Ok(())
        };
    // Baseline first: the control's clock shadow must not carry the
    // windowed rows' fsyncs (symmetry — every row is fsync-bound and
    // equally shadowed by its predecessor).
    push("commit_window_baseline", &mut || {
        commit_window_baseline(&unwindowed)
    })?;
    push("commit_window_admission", &mut || {
        commit_window_admission(&windowed)
    })?;
    push("commit_window_exclusion", &mut || {
        commit_window_exclusion(&windowed)
    })?;
    Ok(out)
}
