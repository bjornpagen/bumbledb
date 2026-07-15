//! The calendar theory — the benchmark's **second** schema/corpus/family
//! world (docs/architecture/60-validation.md § the calendar benchmark):
//! ledger-adjacent scheduling from the workload census, the measured form
//! the algebra's vocabulary exists for. Same protocol as the ledger
//! (fully-indexed `SQLite`, fullfsync parity, warm medians, verify before
//! time), a second theory: accounts of persons, per-person calendars,
//! events with bounded and ray horizons, attendance with RSVP arms (the
//! discriminated-union shape whose exclusivity proof is visible in
//! introspection), per-person claims over intervals with
//! busy/OOO arms, rooms with pointwise-keyed bookings (the exclusion
//! theorem as data), and working-hour segments covering every busy claim
//! (the coverage walk as data).
//!
//! Statement completeness, per the design ruling:
//! - **room exclusion** — `Booking(room, span) -> Booking`, the pointwise
//!   key (no two bookings share a room and an instant);
//! - **claim ↔ attendance `==`** — `Attendance(id | rsvp == Accepted) ==
//!   Claim(source | arm == Busy)`: every accepted attendance owns exactly
//!   one busy claim (totality) and every busy claim's source is an
//!   accepted attendance (arm validity) — the DU pattern with the arm
//!   carried inside a shared child relation, selected by its own
//!   discriminant on both sides;
//! - **working-hours coverage** — `Claim(person, span | arm == Busy) <=
//!   WorkHours(person, hours)`: every point of every busy claim lies
//!   under the person's working-hour segments (exact-abutment chains, a
//!   ray tail reaching ∞ so ray claims are coverable).
//!
//! `Event.hash` is the `bytes<32>` content-hash column the PRD spine owes
//! this family (PRD 17 landed first, so the type exists); `created_at`
//! is the scalar-instant lane (every censused events table carries one),
//! the anchor the at-instant anti-probe binds through — exactly the
//! ledger's `Posting.at` role.
//!
//! **The roster-extension reshape (no prior reshape note existed — this
//! is the minimal reshape covering the order purge's fixed-width
//! interval types):** `Slot` is the fixed-width lane — a per-room grid
//! of `interval<i64, 7200>` bookable slots (the width IS the type; the
//! encoding stores the start word only) under the pointwise key
//! `Slot(room, span) -> Slot`. It exists so the benchmark prices the
//! 8-byte start-only encoding against the general 16-byte form on the
//! same corpus (`slot_scan`, `slot_booking_overlap`). Changing this
//! schema re-baselined every corpus digest and verify stamp — a
//! deliberate act.

pub mod corpus;
pub mod corpus_gen;
pub mod families;
#[cfg(test)]
mod tests;

bumbledb::schema! {
    pub Scheduling;

    relation Account {
        id: u64 as CalAccountId, fresh,
        name: str,
    }
    relation Person {
        id: u64 as CalPersonId, fresh,
        account: u64 as CalAccountId,
        name: str,
    }
    relation Calendar {
        id: u64 as CalendarId, fresh,
        owner: u64 as CalPersonId,
    }
    relation Event {
        id: u64 as CalEventId, fresh,
        calendar: u64 as CalendarId,
        span: interval<i64>,
        created_at: i64,
        hash: bytes<32>,
    }
    relation Attendance {
        id: u64 as AttendanceId, fresh,
        event: u64 as CalEventId,
        person: u64 as CalPersonId,
        rsvp: u64 as RsvpId,
    }
    relation Claim {
        source: u64,
        person: u64 as CalPersonId,
        arm: u64 as ClaimArmId,
        span: interval<i64>,
    }
    relation Room {
        id: u64 as RoomId, fresh,
        name: str,
    }
    relation Booking {
        room: u64 as RoomId,
        event: u64 as CalEventId,
        span: interval<i64>,
    }
    relation WorkHours {
        person: u64 as CalPersonId,
        hours: interval<i64>,
    }
    relation Slot {
        room: u64 as RoomId,
        span: interval<i64, 7200>,
    }

    closed relation Rsvp as RsvpId = { Accepted, Tentative, Declined };
    closed relation Arm as ClaimArmId = { Busy, Ooo };

    Person(account)     <= Account(id);
    Calendar(owner)     <= Person(id);
    Event(calendar)     <= Calendar(id);
    Attendance(event)   <= Event(id);
    Attendance(person)  <= Person(id);
    Attendance(rsvp)    <= Rsvp(id);
    Attendance(event, person) -> Attendance;
    Claim(person)       <= Person(id);
    Claim(arm)          <= Arm(id);
    Claim(source)       -> Claim;
    Claim(person, span) -> Claim;
    Attendance(id | rsvp == Accepted) == Claim(source | arm == Busy);
    Claim(person, span | arm == Busy) <= WorkHours(person, hours);
    Booking(room)       <= Room(id);
    Booking(event)      <= Event(id);
    Booking(room, span) -> Booking;
    WorkHours(person)   <= Person(id);
    WorkHours(person, hours) -> WorkHours;
    Slot(room)          <= Room(id);
    Slot(room, span)    -> Slot;
}

/// The validated calendar schema, memoized for the inspection surfaces
/// (DDL rendering, translation, id lookups); the engine takes
/// [`Scheduling`] — `Db::create(dir, Scheduling)` — and validates there.
///
/// # Panics
///
/// Never in practice: the calendar declaration passes the acceptance
/// gate (asserted on first use).
pub fn schema() -> &'static bumbledb::Schema {
    use bumbledb::Theory as _;
    static SCHEMA: std::sync::OnceLock<bumbledb::Schema> = std::sync::OnceLock::new();
    SCHEMA.get_or_init(|| {
        Scheduling
            .descriptor()
            .validate()
            .expect("the calendar schema is valid")
    })
}

/// Relation and field ids by name — no magic numbers in family
/// definitions or the generator (declaration order is the id order).
pub mod ids {
    use bumbledb::{FieldId, RelationId};

    pub const ACCOUNT: RelationId = RelationId(0);
    pub const PERSON: RelationId = RelationId(1);
    pub const CALENDAR: RelationId = RelationId(2);
    pub const EVENT: RelationId = RelationId(3);
    pub const ATTENDANCE: RelationId = RelationId(4);
    pub const CLAIM: RelationId = RelationId(5);
    pub const ROOM: RelationId = RelationId(6);
    pub const BOOKING: RelationId = RelationId(7);
    pub const WORK_HOURS: RelationId = RelationId(8);
    pub const SLOT: RelationId = RelationId(9);
    pub const RSVP: RelationId = RelationId(10);
    pub const CLAIM_ARM: RelationId = RelationId(11);

    /// The number of **writable** relations — loaders iterate
    /// `0..RELATIONS`. The closed relations (`Rsvp`/`ClaimArm`, ids
    /// 10..12) sit after every ordinary relation by declaration: they
    /// are unwritable ground axioms, so no loader touches them.
    pub const RELATIONS: u32 = 10;

    pub mod account {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const NAME: FieldId = FieldId(1);
    }
    pub mod person {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const ACCOUNT: FieldId = FieldId(1);
        pub const NAME: FieldId = FieldId(2);
    }
    pub mod calendar {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const OWNER: FieldId = FieldId(1);
    }
    pub mod event {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const CALENDAR: FieldId = FieldId(1);
        pub const SPAN: FieldId = FieldId(2);
        pub const CREATED_AT: FieldId = FieldId(3);
        pub const HASH: FieldId = FieldId(4);
    }
    pub mod attendance {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const EVENT: FieldId = FieldId(1);
        pub const PERSON: FieldId = FieldId(2);
        pub const RSVP: FieldId = FieldId(3);
    }
    pub mod claim {
        use super::FieldId;
        pub const SOURCE: FieldId = FieldId(0);
        pub const PERSON: FieldId = FieldId(1);
        pub const ARM: FieldId = FieldId(2);
        pub const SPAN: FieldId = FieldId(3);
    }
    pub mod room {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const NAME: FieldId = FieldId(1);
    }
    pub mod booking {
        use super::FieldId;
        pub const ROOM: FieldId = FieldId(0);
        pub const EVENT: FieldId = FieldId(1);
        pub const SPAN: FieldId = FieldId(2);
    }
    pub mod work_hours {
        use super::FieldId;
        pub const PERSON: FieldId = FieldId(0);
        pub const HOURS: FieldId = FieldId(1);
    }
    pub mod slot {
        use super::FieldId;
        pub const ROOM: FieldId = FieldId(0);
        pub const SPAN: FieldId = FieldId(1);
    }
}

/// `Rsvp` row ids (declaration order is the encoding — closed-relation
/// handles are ground row ids, plain `u64` values in facts and queries).
pub const RSVP_ACCEPTED: u64 = 0;
pub const RSVP_TENTATIVE: u64 = 1;
pub const RSVP_DECLINED: u64 = 2;

/// `ClaimArm` row ids.
pub const ARM_BUSY: u64 = 0;
pub const ARM_OOO: u64 = 1;
