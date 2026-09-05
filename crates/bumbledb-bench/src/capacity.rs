use std::path::Path;

use bumbledb::{Db, RelationId, Value};

use crate::corpus_gen::{GenConfig, Rng};
use crate::harness::{self, Measurement, Protocol};
use crate::writebench::write_protocol;

#[cfg(test)]
mod tests;

pub mod power {
    bumbledb::schema! {
        pub PowerWorld;

        relation Pool {
            id: u64 as PoolId,
            supply: u64,
        }
        relation Device {
            id: u64 as DeviceId,
            pool: u64 as PoolId,
            watts: u64,
        }

        // Declared id keys first (E-NO-RESERVE): the retired fresh
        // auto-keys are ordinary declared statements now, at the head so
        // the later declared statement ids keep their historical slots.
        Pool(id)   -> Pool;
        Device(id) -> Device;

        Device(pool) <= Pool(id);
        Pool(id) <=[watts]{0..supply} Device(pool);
    }
}

pub mod power_baseline {
    bumbledb::schema! {
        pub UnbudgetedWorld;

        relation Pool {
            id: u64 as PoolId,
            supply: u64,
        }
        relation Device {
            id: u64 as DeviceId,
            pool: u64 as PoolId,
            watts: u64,
        }

        Pool(id)   -> Pool;
        Device(id) -> Device;
    }
}

pub mod calendar {
    bumbledb::schema! {
        pub CalendarCapacityWorld;

        relation Room {
            id: u64 as RoomId,
            span: interval<u64>,
        }
        relation Booking {
            id: u64 as BookingId,
            room: u64 as RoomId,
            booked: interval<u64>,
        }

        Room(id)    -> Room;
        Booking(id) -> Booking;

        Booking(room) <= Room(id);
        Room(id) <=[Duration(booked)]{0..Duration(span)} Booking(room);
    }
}

/// The application-owned child-id mint base for the measured commit
/// families (E-NO-RESERVE): the corpus is dense from 0 (at most
/// `parents x children_per_parent` rows), so cursors seeded here can
/// never collide with a loaded row; each family owns one cursor that
/// persists across its timed window and any traced re-run.
pub const MINT_BASE: u64 = 1 << 32;

pub mod ids {
    use bumbledb::RelationId;

    pub const PARENT: RelationId = RelationId(0);
    pub const CHILD: RelationId = RelationId(1);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mass {
    pub parents: u64,

    pub children_per_parent: u64,
}

impl Mass {
    pub const BENCH: Self = Self {
        parents: 4_096,
        children_per_parent: 8,
    };

    #[must_use]
    pub const fn unit() -> Self {
        Self {
            parents: 16,
            children_per_parent: 4,
        }
    }
}

pub const PARENTS: u64 = Mass::BENCH.parents;

/// Every pool's supply — far above what the seed plus every timed sample can
/// accumulate, so every measured commit is legal and the measured cost is the
/// judge (bound read + weighted walk), never a refusal.
pub const SUPPLY: u64 = 1_000_000;

#[must_use]
pub fn seeded_watts(i: u64) -> u64 {
    i % 9
}

pub const SPAN: u64 = 1_000_000;

pub const BOOKED_LEN: u64 = 10;

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

/// # Panics
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

/// # Errors
pub fn load<S>(
    db: &Db<S>,
    mass: Mass,
    rows: fn(Mass, RelationId) -> Box<dyn Iterator<Item = Vec<Value>>>,
) -> Result<(), String> {
    for rel in [ids::PARENT, ids::CHILD] {
        db.write(|tx| {
            tx.insert_dyn(rel, rows(mass, rel))
                .map(bumbledb::MutationReport::changed)
        })
        .map_err(|e| format!("capacity load: {e:?}"))?
        .unwrap();
    }
    Ok(())
}

/// The protocol threads in
/// # Errors
/// # Panics
pub fn commit_capacity_sum(
    db: &Db<power::PowerWorld>,
    proto: Protocol,
    mint: &mut u64,
) -> Result<Measurement, String> {
    let mut rng = Rng::new(0x0CA9_0001);
    harness::measure(proto, || {
        let pool = power::PoolId(rng.range(PARENTS));
        let id = power::DeviceId(*mint);
        *mint += 1;
        db.write(|tx| tx.insert([&power::Device { id, pool, watts: 1 }]))
            .map(|_| 1)
            .map_err(|e| format!("commit_capacity_sum: {e:?}"))
    })
}

/// # Errors
/// # Panics
pub fn commit_capacity_baseline(
    db: &Db<power_baseline::UnbudgetedWorld>,
    proto: Protocol,
    mint: &mut u64,
) -> Result<Measurement, String> {
    let mut rng = Rng::new(0x0CA9_0001);
    harness::measure(proto, || {
        let pool = power_baseline::PoolId(rng.range(PARENTS));
        let id = power_baseline::DeviceId(*mint);
        *mint += 1;
        db.write(|tx| tx.insert([&power_baseline::Device { id, pool, watts: 1 }]))
            .map(|_| 1)
            .map_err(|e| format!("commit_capacity_baseline: {e:?}"))
    })
}

/// # Errors
/// # Panics
pub fn commit_capacity_duration(
    db: &Db<calendar::CalendarCapacityWorld>,
    proto: Protocol,
    mint: &mut u64,
) -> Result<Measurement, String> {
    let mut rng = Rng::new(0x0CA9_0002);
    let mut sample = 0u64;
    harness::measure(proto, || {
        let room = calendar::RoomId(rng.range(PARENTS));

        let start = SPAN / 2 + sample;
        sample += 1;
        let id = calendar::BookingId(*mint);
        *mint += 1;
        db.write(|tx| {
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

/// The POLARITY TABLE decides the trigger set per law: a non-negative-weight
/// ceiling is insert-violable on the weighed side (a new device) AND on the
/// bound-carrying side (a re-inserted pool lowering its own supply — the engine
/// models an complete set is the two BEFORE INSERT triggers; a floor law would
/// owe a BEFORE DELETE twin instead.
pub mod sqlite {

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

/// # Errors
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

    // shadow must not carry the judged rows' fsyncs).
    // One persistent mint per (family, store): the cursor survives the
    // traced re-run so a re-invocation keeps inserting NEW rows instead of
    // degrading into no-op duplicate commits.
    let mut baseline_mint = MINT_BASE;
    let mut sum_mint = MINT_BASE;
    let mut duration_mint = MINT_BASE;
    push("commit_capacity_baseline", &mut |proto| {
        commit_capacity_baseline(&unbudgeted, proto, &mut baseline_mint)
    })?;
    push("commit_capacity_sum", &mut |proto| {
        commit_capacity_sum(&budgeted, proto, &mut sum_mint)
    })?;
    push("commit_capacity_duration", &mut |proto| {
        commit_capacity_duration(&rooms, proto, &mut duration_mint)
    })?;
    Ok(out)
}
