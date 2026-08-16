//! The weighted-capacity write lanes — the measurement infrastructure
//! for the capacity statement's WEIGHTED instances
//! (`docs/architecture/30-dependencies.md` § capacity statement; the
//! unit-instance rows live in [`crate::windowed`]):
//!
//! - **The power budget** (dependent bound): `Pool(id) <=[watts]
//!   {0..supply} Device(pool)` — the per-parent bound read plus the
//!   weighted value-slot walk on the hot path. Rows
//!   `commit_capacity_baseline` (the statement-free control twin) and
//!   `commit_capacity_sum`. This lane was also the C17 measuring
//!   instrument: it decided slot-vs-fetch-per-child (2026-08-01, the
//!   slot arm landed; numbers at the CONSTRAINT comment in
//!   `storage/commit/judgment.rs`).
//! - **Calendar capacity** (`Duration` weight): `Room(id) <=
//!   [Duration(booked)]{0..Duration(span)} Booking(room)` — row
//!   `commit_capacity_duration`. A FRESH twin world by ruling (C15):
//!   the `calendar::Scheduling` corpus digests stand unmoved.
//!
//! Engine-only report rows, `theirs: None` (the windowed precedent: a
//! trigger emulation would time the emulation, not the engine). The
//! ORACLE GATE runs in tests: the same deltas through
//! [`crate::differential::run`] against [`crate::naive::NaiveDb`] —
//! verdicts, citations, and witnessed measures (C14) compared whole
//! before any timing — and the power lane additionally replays its
//! stream against the `SQLite` SUM-trigger twin ([`sqlite`]): the one
//! place `SQLite` speaks a weighted capacity law, verdict-parity only,
//! never timed.

use std::path::Path;

use bumbledb::{Db, RelationId, Value};

use crate::corpus_gen::{GenConfig, Rng};
use crate::harness::{self, Measurement, Protocol};
use crate::writebench::write_protocol;

#[cfg(test)]
mod tests;

/// The power-budget twin: the containment plus the weighted capacity
/// law under its dependent ceiling (the C1 errata'd spelling — the
/// target tuple stays the pure grouping key; `supply` resolves by name
/// against `Pool`'s roster).
pub mod power {
    bumbledb::schema! {
        pub PowerWorld;

        relation Pool {
            id: u64 as PoolId, fresh,
            supply: u64,
        }
        relation Device {
            id: u64 as DeviceId, fresh,
            pool: u64 as PoolId,
            watts: u64,
        }

        Device(pool) <= Pool(id);
        Pool(id) <=[watts]{0..supply} Device(pool);
    }
}

/// The control twin: same relations, NO declared statements (the
/// dossier's statement-free control) — the admission delta prices the
/// whole judged surface the power twin carries.
pub mod power_baseline {
    bumbledb::schema! {
        pub UnbudgetedWorld;

        relation Pool {
            id: u64 as PoolId, fresh,
            supply: u64,
        }
        relation Device {
            id: u64 as DeviceId, fresh,
            pool: u64 as PoolId,
            watts: u64,
        }
    }
}

/// The calendar twin — the `Duration` weight under the `Duration`
/// ceiling, both measures read through the R5 machinery.
pub mod calendar {
    bumbledb::schema! {
        pub CalendarCapacityWorld;

        relation Room {
            id: u64 as RoomId, fresh,
            span: interval<u64>,
        }
        relation Booking {
            id: u64 as BookingId, fresh,
            room: u64 as RoomId,
            booked: interval<u64>,
        }

        Booking(room) <= Room(id);
        Room(id) <=[Duration(booked)]{0..Duration(span)} Booking(room);
    }
}

/// Relation ids (each twin pair declares identically).
pub mod ids {
    use bumbledb::RelationId;

    pub const PARENT: RelationId = RelationId(0);
    pub const CHILD: RelationId = RelationId(1);
}

/// The seeded mass — parameterized so the naive parity slice can shrink
/// every axis (the brute-force model is O(parents × children) per
/// judged delta; the unit-corpus discipline every naive lane follows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mass {
    pub parents: u64,
    /// Seeded children per parent.
    pub children_per_parent: u64,
}

impl Mass {
    /// The timed lane's mass: enough parents that per-sample probes hit
    /// a real tree, enough children that every touched parent's measure
    /// walk reads occupied value slots — "under load", not an empty
    /// store.
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

/// Every pool's supply — far above what the seed plus every timed
/// sample can accumulate, so every measured commit is legal and the
/// measured cost is the judge (bound read + weighted walk), never a
/// refusal.
pub const SUPPLY: u64 = 1_000_000;

/// A seeded device's watts — the weight column's live data, zero
/// included (the § 6 Sum-vs-Count split rides the corpus).
#[must_use]
pub fn seeded_watts(i: u64) -> u64 {
    i % 9
}

/// Every room's span measure (the calendar ceiling); bookings spend it
/// in [`BOOKED_LEN`]-long slices.
pub const SPAN: u64 = 1_000_000;

/// A seeded booking's length — the `Duration` weight's live data.
pub const BOOKED_LEN: u64 = 10;

/// One power-twin relation's seeded row stream (both twins share it —
/// the corpus is the theory-independent mass).
pub fn power_rows(mass: Mass, rel: RelationId) -> Box<dyn Iterator<Item = Vec<Value>>> {
    match rel {
        ids::PARENT => Box::new((0..mass.parents).map(|i| vec![Value::U64(i), Value::U64(SUPPLY)])),
        ids::CHILD => Box::new((0..mass.parents * mass.children_per_parent).map(move |i| {
            vec![
                Value::U64(i),
                Value::U64(i / mass.children_per_parent),
                Value::U64(seeded_watts(i)),
            ]
        })),
        _ => unreachable!("two power relations"),
    }
}

/// One calendar-twin relation's seeded row stream. Booking `i` occupies
/// its own [`BOOKED_LEN`]-long slice of the room's span, so the seeded
/// measure per room is `children_per_parent × BOOKED_LEN` — far under
/// [`SPAN`].
///
/// # Panics
///
/// Never: every constructed slice is nonempty by [`BOOKED_LEN`].
pub fn calendar_rows(mass: Mass, rel: RelationId) -> Box<dyn Iterator<Item = Vec<Value>>> {
    let interval = |start: u64, end: u64| {
        Value::IntervalU64(bumbledb::Interval::<u64>::new(start, end).expect("nonempty interval"))
    };
    match rel {
        ids::PARENT => {
            Box::new((0..mass.parents).map(move |i| vec![Value::U64(i), interval(0, SPAN)]))
        }
        ids::CHILD => Box::new((0..mass.parents * mass.children_per_parent).map(move |i| {
            let slot = i % mass.children_per_parent;
            let start = slot * BOOKED_LEN;
            vec![
                Value::U64(i),
                Value::U64(i / mass.children_per_parent),
                interval(start, start + BOOKED_LEN),
            ]
        })),
        _ => unreachable!("two calendar relations"),
    }
}

/// Loads one twin's seeded mass (schema-generic: the corpus is shared).
///
/// # Errors
///
/// Engine errors, stringified.
pub fn load<S>(
    db: &Db<S>,
    mass: Mass,
    rows: fn(Mass, RelationId) -> Box<dyn Iterator<Item = Vec<Value>>>,
) -> Result<(), String> {
    for rel in [ids::PARENT, ids::CHILD] {
        db.write(|tx| tx.insert_dyn(rel, rows(mass, rel)).map(|r| r.changed))
            .map_err(|e| format!("capacity load: {e:?}"))?;
    }
    Ok(())
}

/// `commit_capacity_sum`: one watts-1 device per commit under the
/// weighted law — the dependent-bound read plus the value-slot measure
/// walk on the hot path, every commit legal. The protocol threads in
/// (the registry's at orchestration, `TRACED_ONE` for the traced solo
/// sample — a re-seeded rng re-draws pool ids, legal in this
/// gate-free engine-only world).
///
/// # Errors
///
/// Engine errors, stringified.
///
/// # Panics
///
/// Panics if `reserve(1)` returns an empty range, which the engine never does.
pub fn commit_capacity_sum(
    db: &Db<power::PowerWorld>,
    proto: Protocol,
) -> Result<Measurement, String> {
    let mut rng = Rng::new(0x0CA9_0001);
    harness::measure(proto, || {
        let pool = power::PoolId(rng.range(PARENTS));
        db.write(|tx| {
            let id: power::DeviceId = tx.reserve(1)?.start().expect("nonempty");
            tx.insert([&power::Device { id, pool, watts: 1 }])
        })
        .map(|_| 1)
        .map_err(|e| format!("commit_capacity_sum: {e:?}"))
    })
}

/// `commit_capacity_baseline`: the identical insert against the
/// statement-free twin — the control.
///
/// # Errors
///
/// Engine errors, stringified.
///
/// # Panics
///
/// Panics if `reserve(1)` returns an empty range, which the engine never does.
pub fn commit_capacity_baseline(
    db: &Db<power_baseline::UnbudgetedWorld>,
    proto: Protocol,
) -> Result<Measurement, String> {
    let mut rng = Rng::new(0x0CA9_0001);
    harness::measure(proto, || {
        let pool = power_baseline::PoolId(rng.range(PARENTS));
        db.write(|tx| {
            let id: power_baseline::DeviceId = tx.reserve(1)?.start().expect("nonempty");
            tx.insert([&power_baseline::Device { id, pool, watts: 1 }])
        })
        .map(|_| 1)
        .map_err(|e| format!("commit_capacity_baseline: {e:?}"))
    })
}

/// `commit_capacity_duration`: one length-1 booking per commit under
/// the calendar law — both `Duration` measures (weight and ceiling)
/// read on the hot path, every commit legal.
///
/// # Errors
///
/// Engine errors, stringified.
///
/// # Panics
///
/// Panics if `reserve(1)` returns an empty range, which the engine never does.
/// Never: every sampled one-unit slice is nonempty.
pub fn commit_capacity_duration(
    db: &Db<calendar::CalendarCapacityWorld>,
    proto: Protocol,
) -> Result<Measurement, String> {
    let mut rng = Rng::new(0x0CA9_0002);
    let mut sample = 0u64;
    harness::measure(proto, || {
        let room = calendar::RoomId(rng.range(PARENTS));
        // Fresh slices high in the span: seeded bookings occupy
        // `[0, children × BOOKED_LEN)`; samples land above them, one
        // unit of measure each — the running total stays far under
        // SPAN across every warmup and sample.
        let start = SPAN / 2 + sample;
        sample += 1;
        db.write(|tx| {
            let id: calendar::BookingId = tx.reserve(1)?.start().expect("nonempty");
            tx.insert([&calendar::Booking {
                id,
                room,
                booked: bumbledb::Interval::<u64>::new(start, start + 1)
                    .expect("nonempty interval"),
            }])
        })
        .map(|_| 1)
        .map_err(|e| format!("commit_capacity_duration: {e:?}"))
    })
}

/// The `SQLite` twin of the power-budget law — THE one place `SQLite`
/// speaks a weighted capacity law, and it speaks it as enforcement
/// (the lawful pattern), never as a pinned verdict
/// (`crate::translate::Inexpressible::CapacityJudgment` routes the
/// judgment lanes naive-side). The POLARITY TABLE decides the trigger
/// set per law: a non-negative-weight ceiling is insert-violable on the
/// weighed side (a new device) AND on the bound-carrying side (a
/// re-inserted pool lowering its own supply — the engine models an
/// update as delete+insert), and delete-violable nowhere, so the
/// complete set is the two BEFORE INSERT triggers; a floor law would
/// owe a BEFORE DELETE twin instead.
pub mod sqlite {
    /// The twin DDL: STRICT tables mirroring the power twins, the FK
    /// mirroring the containment, and the SUM + correlated-subselect
    /// trigger pair — `NEW`'s own weight added on the device side
    /// because BEFORE INSERT sees pre-row state.
    pub const DDL: &[&str] = &[
        "CREATE TABLE \"Pool\" (\"id\" INTEGER NOT NULL, \"supply\" INTEGER NOT NULL, \
         PRIMARY KEY (\"id\")) STRICT",
        "CREATE TABLE \"Device\" (\"id\" INTEGER NOT NULL, \"pool\" INTEGER NOT NULL, \
         \"watts\" INTEGER NOT NULL, PRIMARY KEY (\"id\"), \
         FOREIGN KEY (\"pool\") REFERENCES \"Pool\" (\"id\")) STRICT",
        "CREATE TRIGGER \"capacity_power_budget\" BEFORE INSERT ON \"Device\" WHEN \
         (SELECT COALESCE(SUM(\"watts\"), 0) FROM \"Device\" WHERE \"pool\" = NEW.\"pool\") \
         + NEW.\"watts\" > \
         (SELECT \"supply\" FROM \"Pool\" WHERE \"id\" = NEW.\"pool\") \
         BEGIN SELECT RAISE(ABORT, 'power budget exceeded'); END",
        "CREATE TRIGGER \"capacity_power_rebound\" BEFORE INSERT ON \"Pool\" WHEN \
         (SELECT COALESCE(SUM(\"watts\"), 0) FROM \"Device\" WHERE \"pool\" = NEW.\"id\") \
         > NEW.\"supply\" \
         BEGIN SELECT RAISE(ABORT, 'power budget exceeded'); END",
    ];
}

/// The lane: seed the three twins under `scratch`, run the three rows —
/// engine-only [`crate::report::WriteFamilyReport`]s, `theirs: None`
/// (unpaired by decision; the `SQLite` twin gates verdicts in tests,
/// never clocks).
///
/// # Errors
///
/// Refusals and engine errors, stringified.
pub fn write_families(
    _cfg: GenConfig,
    scratch: &Path,
    selected: &dyn Fn(&str) -> bool,
    mode: crate::storemode::StoreMode,
    trace_dir: Option<&Path>,
    flames: &mut Vec<crate::report::FlameEmbed>,
) -> Result<Vec<crate::report::WriteFamilyReport>, String> {
    let names = [
        "commit_capacity_baseline",
        "commit_capacity_sum",
        "commit_capacity_duration",
    ];
    if !names.iter().any(|name| selected(name)) {
        return Ok(Vec::new());
    }
    std::fs::create_dir_all(scratch).map_err(|e| format!("capacity scratch: {e}"))?;
    eprintln!("bench: loading the capacity twin worlds");
    let budgeted = mode.create(&scratch.join("power"), power::PowerWorld)?;
    load(&budgeted, Mass::BENCH, power_rows)?;
    let unbudgeted = mode.create(&scratch.join("baseline"), power_baseline::UnbudgetedWorld)?;
    load(&unbudgeted, Mass::BENCH, power_rows)?;
    let rooms = mode.create(&scratch.join("calendar"), calendar::CalendarCapacityWorld)?;
    load(&rooms, Mass::BENCH, calendar_rows)?;

    let mut out = Vec::new();
    let mut push = |name: &str,
                    run: &mut dyn FnMut(Protocol) -> Result<Measurement, String>|
     -> Result<(), String> {
        if !selected(name) {
            return Ok(());
        }
        eprintln!("bench: {name}");
        let (ours, ghz) = crate::clockproxy::stamped(|| run(write_protocol(name)))?;
        // The traced solo sample (--trace): AFTER the timed window,
        // one captured commit — the weighted-capacity judgment spans,
        // readable from disk (engine-only lane, no mirror to keep in
        // lockstep).
        if let Some(table) = crate::trace_out::traced_solo(trace_dir, name, run)? {
            flames.push(crate::report::FlameEmbed {
                name: name.to_owned(),
                table,
            });
        }
        out.push(crate::report::WriteFamilyReport {
            name: name.to_owned(),
            ours: ours.stats,
            theirs: None,
            facts_per_sec: None,
            ghz: Some(ghz.into()),
        });
        Ok(())
    };
    // Baseline first (the windowed symmetry rule: the control's clock
    // shadow must not carry the judged rows' fsyncs).
    push("commit_capacity_baseline", &mut |proto| {
        commit_capacity_baseline(&unbudgeted, proto)
    })?;
    push("commit_capacity_sum", &mut |proto| {
        commit_capacity_sum(&budgeted, proto)
    })?;
    push("commit_capacity_duration", &mut |proto| {
        commit_capacity_duration(&rooms, proto)
    })?;
    Ok(out)
}
