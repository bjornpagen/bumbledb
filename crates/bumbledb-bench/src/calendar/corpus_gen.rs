//! The calendar corpus generator: seeded, streaming, **valid by
//! construction** under every declared statement, stratified over
//! persons × meeting density × ray fraction.
//!
//! - **Zipfian meeting density, hand-rolled** (the bench-crate dependency
//!   quarantine): person `p`'s chain length is
//!   `max(min_segments, max_segments >> ⌊log₂(p + 1)⌋)` — the count
//!   halves as the rank doubles, the 1/rank envelope in closed form, no
//!   crate, no floats. Rank buckets are the density strata.
//! - **Pointwise validity by construction**: each person's segments are
//!   sequential and non-overlapping, mixing **abutting** boundaries
//!   (every third — the neighbor-probe boundary as data) with gapped
//!   ones; bookings put one person per room, so per-room spans inherit
//!   the person's disjointness.
//! - **Ray fraction**: every fourth person's final segment is a ray
//!   (`end == MAX_END` = `[s, ∞)` — the recurrence horizon), always a
//!   busy segment, so ray events, ray claims, ray bookings, and the
//!   coverage-to-∞ case all exist structurally, never by chance.
//! - **The DU and the coverage hold by construction**: busy segment →
//!   one event + one accepted attendance (id `3e`) + one busy claim
//!   (`source = 3e`); OOO segments claim from a disjoint source range;
//!   every person's working hours are an exact-abutment 4-segment chain
//!   from [`CAL_BASE`] to ∞, covering every busy claim.

use bumbledb::{Interval, RelationId, Value};

use crate::calendar::ids;
use crate::corpus_gen::{GenConfig, Rng, Scale};

/// The corpus epoch (seconds-scale i64 instants).
pub const CAL_BASE: i64 = 1_700_000_000;

/// One hour, in corpus time units.
pub const HOUR: i64 = 3_600;

/// Every bounded segment ends strictly below this instant (the longest
/// chain tops out near `CAL_BASE + max_segments × 12 × HOUR` ≈
/// `CAL_BASE + 2.3 × 10⁷`) — the ray-filter literal the measure family
/// filters against: `Allen(span, [CAL_HORIZON, ∞), DISJOINT)` keeps
/// exactly the bounded claims.
pub const CAL_HORIZON: i64 = CAL_BASE + 100_000_000;

/// Working-hour segments per person (an exact-abutment chain, ray tail).
pub const WORK_SEGMENTS: usize = 4;

/// Attendances per event: the owner's accepted RSVP plus two invitees
/// (tentative/declined alternating) — the DU arms all populated.
pub const ATTENDANCE_PER_EVENT: u64 = 3;

/// The fixed slot width, in corpus time units — the `Slot.span` TYPE's
/// width (`interval<i64, 7200>`): every slot is exactly two hours, and
/// the encoding stores only the start word.
pub const SLOT_WIDTH: i64 = 2 * HOUR;

/// The calendar corpus shape: person count plus the density envelope —
/// everything else derives. [`CalSizes::of`] carries the scale points;
/// the naive lane shrinks every axis through [`CalSizes::unit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalSizes {
    pub persons: u64,
    /// The densest person's chain length (rank 0 of the Zipf envelope).
    pub max_segments: u64,
    /// The floor every tail person keeps.
    pub min_segments: u64,
    pub accounts: u64,
    pub rooms: u64,
    /// Fixed-width slots per room (the fixed-width interval lane's
    /// grid density — [`slot_span`] owns the layout).
    pub slots_per_room: u64,
    /// Derived totals (one O(persons) closed-form walk, no RNG).
    pub events: u64,
    pub claims: u64,
    pub attendances: u64,
    pub bookings: u64,
}

impl CalSizes {
    /// The standard scale points (the ledger's 10⁵–10⁷ fact band),
    /// plus `Tiny` — the fuzz-iteration point (32 persons, 16-segment
    /// max chains), mirroring the ledger ladder's table.
    #[must_use]
    pub fn of(scale: Scale) -> Self {
        let (persons, max_segments, min_segments) = match scale {
            Scale::Tiny => (32, 16, 2),
            Scale::S => (2_000, 512, 16),
            Scale::M => (20_000, 512, 16),
            Scale::L => (200_000, 512, 16),
        };
        Self::derive(persons, max_segments, min_segments)
    }

    /// The naive lane's unit corpus: small enough for brute-force
    /// nested loops, dense enough that every family query has witnesses.
    #[must_use]
    pub fn unit() -> Self {
        Self::derive(6, 8, 2)
    }

    fn derive(persons: u64, max_segments: u64, min_segments: u64) -> Self {
        let accounts = (persons / 8).max(1);
        let rooms = (persons / 16).max(1);
        // The fixed-width slot grid: dense enough for witnesses at unit
        // scale (16), a real scan mass at the standard points (256).
        let slots_per_room = if persons <= 32 { 16 } else { 256 };
        let mut events = 0u64;
        let mut claims = 0u64;
        let mut bookings = 0u64;
        let mut sizes = Self {
            persons,
            max_segments,
            min_segments,
            accounts,
            rooms,
            slots_per_room,
            events: 0,
            claims: 0,
            attendances: 0,
            bookings: 0,
        };
        for person in 0..persons {
            let n = sizes.segments_of(person);
            let busy = (0..n).filter(|k| !sizes.segment_is_ooo(person, *k)).count() as u64;
            events += busy;
            claims += n;
            if person < rooms {
                bookings += busy;
            }
        }
        sizes.events = events;
        sizes.claims = claims;
        sizes.attendances = events * ATTENDANCE_PER_EVENT;
        sizes.bookings = bookings;
        sizes
    }

    /// Person `p`'s chain length — the hand-rolled Zipf envelope.
    #[must_use]
    pub fn segments_of(&self, person: u64) -> u64 {
        (self.max_segments >> (person + 1).ilog2()).max(self.min_segments)
    }

    /// The ray stratum: every fourth person's chain ends in a ray.
    #[must_use]
    pub fn person_has_ray(&self, person: u64) -> bool {
        person.is_multiple_of(4)
    }

    /// Whether segment `k` of `person`'s chain is an OOO claim (every
    /// fifth segment), except a ray tail — the ray is always busy, so
    /// ray events and coverage-to-∞ exist structurally.
    #[must_use]
    pub fn segment_is_ooo(&self, person: u64, k: u64) -> bool {
        let n = self.segments_of(person);
        (k % 5 == 4) && !(self.person_has_ray(person) && k == n - 1)
    }

    /// Rows of one relation — loaders and the digest iterate the same
    /// counts the streams produce.
    #[must_use]
    pub fn rows(&self, rel: RelationId) -> u64 {
        match rel {
            ids::ACCOUNT => self.accounts,
            ids::PERSON | ids::CALENDAR => self.persons,
            ids::EVENT => self.events,
            ids::ATTENDANCE => self.attendances,
            ids::CLAIM => self.claims,
            ids::ROOM => self.rooms,
            ids::BOOKING => self.bookings,
            ids::WORK_HOURS => self.persons * WORK_SEGMENTS as u64,
            ids::SLOT => self.rooms * self.slots_per_room,
            _ => unreachable!("ten calendar relations"),
        }
    }

    /// The first OOO claim's `source` id: attendance ids occupy
    /// `0..3 × events`, OOO sources the disjoint range above — the
    /// `Claim(source) -> Claim` key holds by construction.
    #[must_use]
    pub fn ooo_source_base(&self) -> u64 {
        self.attendances
    }
}

/// One chain segment: the half-open window plus its arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalSegment {
    pub start: i64,
    pub end: i64,
    pub ooo: bool,
}

/// One person's full segment chain, **valid under the pointwise claim
/// key by construction**: sequential non-overlapping half-open segments,
/// every third boundary abutting (`end == next.start`), the rest gapped;
/// the ray stratum's final segment is `[s, ∞)`.
///
/// # Panics
///
/// Never in practice: chain arithmetic stays far below [`CAL_HORIZON`].
#[must_use]
pub fn chain(seed: u64, sizes: &CalSizes, person: u64) -> Vec<CalSegment> {
    let n = sizes.segments_of(person);
    let mut rng = Rng::new(crate::corpus_gen::mix(seed, ids::CLAIM, person));
    let mut segments = Vec::with_capacity(usize::try_from(n).expect("fits"));
    let mut cursor = CAL_BASE + i64::try_from(rng.range(4 * HOUR as u64)).expect("fits");
    for k in 0..n {
        let length = (1 + i64::try_from(rng.range(8)).expect("fits")) * HOUR;
        let ray = sizes.person_has_ray(person) && k == n - 1;
        let end = if ray {
            Interval::<i64>::MAX_END
        } else {
            cursor + length
        };
        segments.push(CalSegment {
            start: cursor,
            end,
            ooo: sizes.segment_is_ooo(person, k),
        });
        // Every third boundary abuts; the rest leave a strictly
        // positive gap (a free instant exists between them).
        let gap = if k % 3 == 2 {
            0
        } else {
            (1 + i64::try_from(rng.range(4)).expect("fits")) * HOUR
        };
        cursor = cursor + length + gap;
    }
    segments
}

/// One person's working-hour chain: [`WORK_SEGMENTS`] exact-abutment
/// segments from [`CAL_BASE`] to ∞ — the coverage target for every busy
/// claim (the abutment boundaries stress the frontier walk; the ray tail
/// covers ray claims).
///
/// # Panics
///
/// Never in practice: cut arithmetic stays far inside `i64`.
#[must_use]
pub fn work_chain(seed: u64, person: u64) -> [(i64, i64); WORK_SEGMENTS] {
    let mut rng = Rng::new(crate::corpus_gen::mix(seed, ids::WORK_HOURS, person));
    let mut cut = |floor: i64| floor + 1 + i64::try_from(rng.range(10_000_000)).expect("fits");
    let c1 = cut(CAL_BASE);
    let c2 = cut(c1);
    let c3 = cut(c2);
    [
        (CAL_BASE, c1),
        (c1, c2),
        (c2, c3),
        (c3, Interval::<i64>::MAX_END),
    ]
}

/// One event's `created_at` instant: seeded pseudo-scatter over the
/// active span, so at-instant anti-probes split persons into free and
/// claimed rather than degenerating to all-free.
///
/// # Panics
///
/// Never in practice: the scatter stays far inside `i64`.
#[must_use]
pub fn created_at(seed: u64, event: u64) -> i64 {
    let word = crate::corpus_gen::mix(seed, ids::EVENT, event);
    CAL_BASE + i64::try_from(word % 22_000_000).expect("fits")
}

/// One event's `bytes<32>` content hash: four seeded LE words —
/// deterministic, unique with overwhelming probability, identity-shaped.
#[must_use]
pub fn event_hash(seed: u64, event: u64) -> Value {
    let mut raw = Vec::with_capacity(32);
    for lane in 0..4u64 {
        raw.extend_from_slice(
            &crate::corpus_gen::mix(seed ^ lane, ids::EVENT, event).to_le_bytes(),
        );
    }
    Value::FixedBytes(raw.into())
}

/// One chain segment in walk order, with its global indexes assigned:
/// the busy index is the event id, the OOO index offsets into the
/// disjoint claim-source range.
#[derive(Debug, Clone, Copy)]
pub struct SegmentRow {
    pub person: u64,
    pub segment: CalSegment,
    /// `Some(event id)` for a busy segment.
    pub event: Option<u64>,
    /// `Some(OOO ordinal)` for an OOO segment.
    pub ooo_index: Option<u64>,
}

/// The whole corpus's segment walk, person by person in chain order,
/// global counters assigned — the one stream events, attendances,
/// claims, and bookings all derive from (their cross-references agree
/// because they are the same walk).
pub fn segment_walk(cfg: GenConfig, sizes: CalSizes) -> impl Iterator<Item = SegmentRow> {
    let seed = cfg.seed;
    let mut next_event = 0u64;
    let mut next_ooo = 0u64;
    (0..sizes.persons).flat_map(move |person| {
        chain(seed, &sizes, person)
            .into_iter()
            .map(|segment| {
                if segment.ooo {
                    let row = SegmentRow {
                        person,
                        segment,
                        event: None,
                        ooo_index: Some(next_ooo),
                    };
                    next_ooo += 1;
                    row
                } else {
                    let row = SegmentRow {
                        person,
                        segment,
                        event: Some(next_event),
                        ooo_index: None,
                    };
                    next_event += 1;
                    row
                }
            })
            .collect::<Vec<_>>()
    })
}

/// One segment's `==`-cluster rows: the busy segment's attendance rows
/// beside its claim (or the OOO claim alone). Attendance and Claim load
/// **jointly** through this stream ([`crate::calendar::corpus`]): the
/// `Attendance(id | rsvp == Accepted) == Claim(source | arm == Busy)`
/// statement holds in neither one-relation prefix, so a chunk boundary
/// may fall only between segments, never inside one.
pub fn du_cluster_rows(
    cfg: GenConfig,
    sizes: CalSizes,
) -> impl Iterator<Item = (Vec<Vec<Value>>, Vec<Value>)> {
    segment_walk(cfg, sizes).map(move |row| {
        let attendances = if row.event.is_some() {
            attendance_rows(&sizes, &row)
        } else {
            Vec::new()
        };
        (attendances, claim_row(&sizes, &row))
    })
}

fn account_row(i: u64) -> Vec<Value> {
    vec![
        Value::U64(i),
        Value::String(format!("acct-{i:05}").into_bytes().into()),
    ]
}

fn person_row(sizes: &CalSizes, i: u64) -> Vec<Value> {
    vec![
        Value::U64(i),
        Value::U64(i / 8 % sizes.accounts.max(1)),
        Value::String(format!("person-{i:06}").into_bytes().into()),
    ]
}

/// Calendars are 1:1 with persons (calendar id = owner id).
fn calendar_row(i: u64) -> Vec<Value> {
    vec![Value::U64(i), Value::U64(i)]
}

fn room_row(i: u64) -> Vec<Value> {
    vec![
        Value::U64(i),
        Value::String(format!("room-{i:04}").into_bytes().into()),
    ]
}

fn event_row(seed: u64, row: &SegmentRow) -> Vec<Value> {
    let event = row.event.expect("busy segments carry the event id");
    vec![
        Value::U64(event),
        Value::U64(row.person), // calendar id = owner id
        Value::IntervalI64(
            bumbledb::Interval::<i64>::new(row.segment.start, row.segment.end)
                .expect("nonempty interval"),
        ),
        Value::I64(created_at(seed, event)),
        event_hash(seed, event),
    ]
}

/// One event's three attendances: the owner's accepted RSVP (id `3e` —
/// the busy claim's `source`), then two invitees alternating
/// tentative/declined (the arm variety; never accepted, so the `==`
/// totality stays a per-owner fact).
fn attendance_rows(sizes: &CalSizes, row: &SegmentRow) -> Vec<Vec<Value>> {
    let event = row.event.expect("busy segments carry the event id");
    let mut rows = vec![vec![
        Value::U64(3 * event),
        Value::U64(event),
        Value::U64(row.person),
        Value::U64(crate::calendar::RSVP_ACCEPTED),
    ]];
    for j in 1..ATTENDANCE_PER_EVENT {
        let invitee = (row.person + j) % sizes.persons;
        let rsvp = if (event + j).is_multiple_of(2) {
            crate::calendar::RSVP_TENTATIVE
        } else {
            crate::calendar::RSVP_DECLINED
        };
        rows.push(vec![
            Value::U64(3 * event + j),
            Value::U64(event),
            Value::U64(invitee),
            Value::U64(rsvp),
        ]);
    }
    rows
}

fn claim_row(sizes: &CalSizes, row: &SegmentRow) -> Vec<Value> {
    let (source, arm) = match (row.event, row.ooo_index) {
        (Some(event), None) => (3 * event, crate::calendar::ARM_BUSY),
        (None, Some(ooo)) => (sizes.ooo_source_base() + ooo, crate::calendar::ARM_OOO),
        _ => unreachable!("a segment is exactly one arm"),
    };
    vec![
        Value::U64(source),
        Value::U64(row.person),
        Value::U64(arm),
        Value::IntervalI64(
            bumbledb::Interval::<i64>::new(row.segment.start, row.segment.end)
                .expect("nonempty interval"),
        ),
    ]
}

/// Slot `k` of any room's fixed-width grid: triples of two-hour slots —
/// gapped, gapped, **abutting** (the neighbor-probe boundary as data,
/// the claim chain's every-third-abuts discipline) — one triple per
/// `8 × HOUR`, disjoint per room by construction (the pointwise
/// `Slot(room, span) -> Slot` key holds without an RNG). Fixed-width
/// values are never rays and always exactly [`SLOT_WIDTH`] wide — the
/// type admits nothing else.
///
/// # Panics
///
/// Never in practice: grid arithmetic stays far below [`CAL_HORIZON`].
#[must_use]
pub fn slot_span(k: u64) -> (i64, i64) {
    let triple = i64::try_from(k / 3).expect("fits");
    let offset = match k % 3 {
        0 => 0,
        1 => 3 * HOUR,
        // Abuts slot 1's end (3h + 2h = 5h into the triple).
        _ => 5 * HOUR,
    };
    let start = CAL_BASE + triple * 8 * HOUR + offset;
    (start, start + SLOT_WIDTH)
}

/// One slot row: room-major grid order (`i = room × slots_per_room + k`).
fn slot_row(sizes: &CalSizes, i: u64) -> Vec<Value> {
    let room = i / sizes.slots_per_room;
    let (start, end) = slot_span(i % sizes.slots_per_room);
    vec![
        Value::U64(room),
        Value::IntervalI64(Interval::<i64>::new(start, end).expect("nonempty fixed slot")),
    ]
}

/// One person's working-hour rows.
fn work_rows(seed: u64, person: u64) -> Vec<Vec<Value>> {
    work_chain(seed, person)
        .into_iter()
        .map(|(start, end)| {
            vec![
                Value::U64(person),
                Value::IntervalI64(
                    bumbledb::Interval::<i64>::new(start, end).expect("nonempty interval"),
                ),
            ]
        })
        .collect()
}

/// One relation's full row stream — O(one person's chain) memory,
/// restartable from scratch (streams are pure functions of the config).
///
/// Bookings are the events of persons `0..rooms` — one dedicated room
/// per head person (the Zipf head, so rooms are busy), per-room
/// disjointness inherited from the person's own chain.
#[must_use]
pub fn relation_rows(cfg: GenConfig, rel: RelationId) -> Box<dyn Iterator<Item = Vec<Value>>> {
    let sizes = CalSizes::of(cfg.scale);
    relation_rows_sized(cfg, sizes, rel)
}

/// [`relation_rows`] with explicit sizes — the naive lane's unit-corpus
/// seam.
///
/// # Panics
///
/// Never in practice: row arithmetic stays inside the generated
/// domains.
#[must_use]
pub fn relation_rows_sized(
    cfg: GenConfig,
    sizes: CalSizes,
    rel: RelationId,
) -> Box<dyn Iterator<Item = Vec<Value>>> {
    let seed = cfg.seed;
    match rel {
        ids::ACCOUNT => Box::new((0..sizes.accounts).map(account_row)),
        ids::PERSON => Box::new((0..sizes.persons).map(move |i| person_row(&sizes, i))),
        ids::CALENDAR => Box::new((0..sizes.persons).map(calendar_row)),
        ids::EVENT => Box::new(
            segment_walk(cfg, sizes)
                .filter(|row| row.event.is_some())
                .map(move |row| event_row(seed, &row)),
        ),
        ids::ATTENDANCE => Box::new(
            segment_walk(cfg, sizes)
                .filter(|row| row.event.is_some())
                .flat_map(move |row| attendance_rows(&sizes, &row)),
        ),
        ids::CLAIM => Box::new(segment_walk(cfg, sizes).map(move |row| claim_row(&sizes, &row))),
        ids::ROOM => Box::new((0..sizes.rooms).map(room_row)),
        ids::BOOKING => Box::new(
            segment_walk(cfg, sizes)
                .take_while(move |row| row.person < sizes.rooms)
                .filter(|row| row.event.is_some())
                .map(|row| {
                    vec![
                        Value::U64(row.person),
                        Value::U64(row.event.expect("busy")),
                        Value::IntervalI64(
                            bumbledb::Interval::<i64>::new(row.segment.start, row.segment.end)
                                .expect("nonempty interval"),
                        ),
                    ]
                }),
        ),
        ids::WORK_HOURS => Box::new((0..sizes.persons).flat_map(move |p| work_rows(seed, p))),
        ids::SLOT => Box::new(
            (0..sizes.rooms * sizes.slots_per_room).map(move |i| slot_row(&sizes, i)),
        ),
        _ => unreachable!("ten calendar relations"),
    }
}
