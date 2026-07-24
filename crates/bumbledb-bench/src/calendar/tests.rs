use bumbledb::{Answers, Db, Interval};

use crate::calendar::corpus_gen::{CAL_BASE, CAL_HORIZON, CalSizes, chain, work_chain};
use crate::calendar::{Scheduling, corpus, families, ids, schema};
use crate::corpus_gen::{GenConfig, Scale};
use crate::families::{param_args, set_bindings};
use crate::translate::translate;

const CFG: GenConfig = GenConfig {
    seed: 1,
    scale: Scale::S,
};

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("bumbledb-calendar-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// The id registry matches declaration order, and the statement roster
/// is complete: ten ordinary relations plus the two closed
/// vocabularies (`Rsvp`/`Arm`), six fresh auto-keys, the two closed
/// auto-keys, the thirteen declared containments (working-hours
/// coverage, the two vocabulary containments, and the fixed-width
/// `Slot(room)` among them), the declared keys
/// (`Attendance(event, person)`, `Claim(source)`, and the four
/// pointwise keys — `Slot(room, span)` is the fixed-width one), and
/// the `==` pair lowered to its two directions.
#[test]
fn the_schema_is_statement_complete() {
    let s = schema();
    for (idx, name) in [
        "Account",
        "Person",
        "Calendar",
        "Event",
        "Attendance",
        "Claim",
        "Room",
        "Booking",
        "WorkHours",
        "Slot",
        "Rsvp",
        "Arm",
    ]
    .iter()
    .enumerate()
    {
        let rel = bumbledb::RelationId(u32::try_from(idx).expect("small"));
        assert_eq!(s.relation(rel).name(), *name);
    }
    assert_eq!(s.relations().len(), 12);
    for rel in 0..ids::RELATIONS {
        assert!(
            !s.relation(bumbledb::RelationId(rel)).is_closed(),
            "every writable relation precedes the closed vocabulary"
        );
    }
    for rel in [ids::RSVP, ids::CLAIM_ARM] {
        assert!(s.relation(rel).is_closed());
    }

    let mut autos = 0;
    let mut closed_keys = 0;
    let mut scalar_keys = 0;
    let mut pointwise = Vec::new();
    let mut containments = Vec::new();
    for statement in s.keys() {
        if statement.pointwise() {
            pointwise.push(statement.relation);
        } else if s.relation(statement.relation).is_closed() {
            closed_keys += 1;
        } else {
            // The fresh auto-keys lead; the declared scalar keys are
            // Attendance(event, person) and Claim(source).
            if autos < 6 && scalar_keys == 0 {
                autos += 1;
            } else {
                scalar_keys += 1;
            }
        }
    }
    for statement in s.containments() {
        containments.push((statement.source.relation, statement.target.relation));
    }
    assert_eq!(autos, 6, "Account/Person/Calendar/Event/Attendance/Room");
    assert_eq!(closed_keys, 2, "the Rsvp and Arm closed auto-keys");
    assert_eq!(
        scalar_keys, 2,
        "Attendance(event, person) and Claim(source)"
    );
    assert_eq!(
        pointwise,
        vec![ids::CLAIM, ids::BOOKING, ids::WORK_HOURS, ids::SLOT],
        "the pointwise keys: per-person claims, room exclusion, per-person hours, \
         and the fixed-width slot grid"
    );
    // The `==` lowers to two containments; with the thirteen declared
    // ones (incl. the working-hours coverage, the two vocabulary
    // containments, and Slot(room)) that is fifteen total.
    assert_eq!(containments.len(), 15, "thirteen declared + the == pair");
    assert!(
        containments.contains(&(ids::SLOT, ids::ROOM)),
        "the fixed-width slot grid's room containment"
    );
    assert!(
        containments.contains(&(ids::ATTENDANCE, ids::RSVP))
            && containments.contains(&(ids::CLAIM, ids::CLAIM_ARM)),
        "the vocabulary containments: rsvp and arm are closed row ids"
    );
    assert!(
        containments.contains(&(ids::CLAIM, ids::WORK_HOURS)),
        "working-hours coverage: every busy claim under the person's hours"
    );
    assert!(
        containments.contains(&(ids::ATTENDANCE, ids::CLAIM))
            && containments.contains(&(ids::CLAIM, ids::ATTENDANCE)),
        "the DU ==: accepted attendance <-> busy claim, both directions"
    );
}

/// Chain validity by construction: sequential, non-overlapping,
/// abutting every third boundary, the ray stratum's tail unbounded.
#[test]
fn chains_are_valid_under_the_pointwise_key() {
    let sizes = CalSizes::of(Scale::S);
    let mut rays = 0;
    let mut abutments = 0;
    for person in 0..64 {
        let segments = chain(1, &sizes, person);
        for pair in segments.windows(2) {
            assert!(pair[0].end <= pair[1].start, "segments never overlap");
            abutments += usize::from(pair[0].end == pair[1].start);
        }
        let last = segments.last().expect("nonempty chain");
        if sizes.person_has_ray(person) {
            assert_eq!(last.end, Interval::<i64>::MAX_END, "the ray stratum");
            assert!(!last.ooo, "the ray is always busy");
            rays += 1;
        } else {
            assert!(
                last.end < CAL_HORIZON,
                "bounded ends stay below the horizon"
            );
        }
        let hours = work_chain(1, person);
        assert_eq!(hours[0].0, CAL_BASE);
        assert_eq!(hours[3].1, Interval::<i64>::MAX_END);
        for pair in hours.windows(2) {
            assert_eq!(pair[0].1, pair[1].0, "exact abutment");
        }
    }
    assert_eq!(rays, 16, "every fourth person");
    assert!(abutments > 0, "the neighbor-probe boundary exists as data");
}

/// Both stores load the same corpus at S scale, and the joint `==`
/// cluster load commits clean — the statements hold at every chunk.
#[test]
fn both_stores_load_the_same_corpus() {
    let dir = scratch("corpus-load");
    let db = Db::create(&dir.join("db"), Scheduling).expect("create");
    let ours = corpus::load_bumbledb(&db, CFG).expect("bumbledb load");
    let (conn, theirs) = corpus::load_sqlite(&dir.join("oracle.sqlite"), CFG).expect("sqlite load");
    assert_eq!(ours.facts, theirs.facts);
    corpus::assert_loaded_equal(&db, &conn, CFG);
    drop((db, conn));
    let _ = std::fs::remove_dir_all(&dir);
}

/// The translator-paired goldens are pinned: `translate` output equals
/// the hand-written SQL byte-for-byte (the arbitration anchor), and the
/// one unpaired family is exactly `free_busy` — reported, never dropped.
#[test]
fn goldens_pin_the_translator() {
    for family in families::all() {
        if family.hand_param_slots.is_some() {
            continue;
        }
        let query = (family.query)();
        let translated = translate(&query, schema(), &[]).expect("translates");
        assert_eq!(
            translated.sql, family.golden_sql,
            "{}: translator output diverged from the hand-written golden",
            family.name
        );
    }
    assert_eq!(
        families::translator_unpaired(),
        vec!["free_busy"],
        "the enumerated unpaired set"
    );
}

/// Every family produces witnesses on the unit corpus — the joins are
/// real, not vacuously empty (the S-scale rotations include misses by
/// policy; the unit draw is the guaranteed hit).
#[test]
fn every_family_has_witnesses_on_the_unit_corpus() {
    let dir = scratch("unit-witnesses");
    let sizes = CalSizes::unit();
    let db = Db::create(&dir, Scheduling).expect("create");
    corpus::load_bumbledb_sized(&db, CFG, sizes).expect("unit load");
    for family in families::all() {
        let query = (family.query)();
        let mut prepared = db.prepare(&query).expect("prepare");
        let draw = families::unit_draw(family.name, CFG.seed, &sizes);
        let args = param_args(&draw);
        let mut buffer = Answers::new();
        db.read(|snap| snap.execute_args(&mut prepared, &args, &mut buffer))
            .expect("execute");
        assert!(
            !buffer.is_empty(),
            "{}: the unit draw must produce witnesses",
            family.name
        );
    }
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The hand-written `free_busy` coalesce is row-identical to the
/// engine's `Pack` on the unit corpus — `SQLite`'s honest best shot,
/// verified before it is ever timed.
#[test]
fn the_hand_coalesce_matches_pack() {
    let dir = scratch("coalesce");
    let sizes = CalSizes::unit();
    let db = Db::create(&dir.join("db"), Scheduling).expect("create");
    corpus::load_bumbledb_sized(&db, CFG, sizes).expect("unit load");
    let conn = rusqlite::Connection::open_in_memory().expect("oracle");
    corpus::load_sqlite_into(&conn, CFG, sizes).expect("oracle load");

    let family = families::all()
        .iter()
        .find(|f| f.name == "free_busy")
        .expect("registered");
    let query = (family.query)();
    let mut prepared = db.prepare(&query).expect("prepare");
    let types: Vec<bumbledb::schema::ValueType> = prepared
        .predicate()
        .columns
        .iter()
        .map(|column| column.ty.clone())
        .collect();
    let draw = families::unit_draw("free_busy", CFG.seed, &sizes);
    let args = param_args(&draw);
    let mut buffer = Answers::new();
    db.read(|snap| snap.execute_args(&mut prepared, &args, &mut buffer))
        .expect("execute");
    let ours = crate::compare::from_answers(&buffer, &types);

    let translated = family.sql_for(&query, &draw).expect("hand SQL");
    let mut stmt = conn.prepare(&translated.sql).expect("prepare oracle");
    let theirs = crate::compare::from_sqlite(&mut stmt, &translated.params, &draw, &types)
        .expect("oracle rows");
    assert!(!ours.is_empty(), "a real coalesce");
    crate::compare::multisets(ours, theirs).expect("Pack == hand coalesce");
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// One SQL golden shape check on a loaded store: every family's SQL for
/// its first S draw prepares and executes on the mirror (window
/// functions included), and translation errors never reach a bench run.
#[test]
fn every_family_sql_prepares_on_the_mirror() {
    let dir = scratch("sql-prepares");
    let sizes = CalSizes::unit();
    let conn = rusqlite::Connection::open_in_memory().expect("oracle");
    corpus::load_sqlite_into(&conn, CFG, sizes).expect("oracle load");
    for family in families::all() {
        let query = (family.query)();
        for draw in (family.params)(&CFG) {
            let translated = family.sql_for(&query, &draw).expect("sql");
            let mut stmt = conn.prepare(&translated.sql).expect("prepares");
            let params = crate::sqlite_run::bind_args(&translated.params, &draw);
            let count = stmt
                .query_map(rusqlite::params_from_iter(params), |_| Ok(()))
                .expect("executes")
                .count();
            let _ = count;
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A dedicated golden for `set_bindings` parity: the calendar families
/// are scalar-only, so translator re-rendering never engages.
#[test]
fn calendar_families_are_scalar_only() {
    for family in families::all() {
        for draw in (family.params)(&CFG) {
            assert!(
                set_bindings(&draw).is_empty(),
                "{}: calendar draws are scalar-only",
                family.name
            );
        }
    }
}
