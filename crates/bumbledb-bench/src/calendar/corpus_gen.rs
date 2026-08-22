use bumbledb::{Interval, RelationId, Value};

use crate::calendar::ids;
use crate::corpus_gen::{GenConfig, Rng, Scale};

pub const CAL_BASE: i64 = 1_700_000_000;

pub const HOUR: i64 = 3_600;

pub const CAL_HORIZON: i64 = CAL_BASE + 100_000_000;

pub const WORK_SEGMENTS: usize = 4;

pub const ATTENDANCE_PER_EVENT: u64 = 3;

pub const SLOT_WIDTH: i64 = 2 * HOUR;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalSizes {
    pub persons: u64,

    pub max_segments: u64,

    pub min_segments: u64,
    pub accounts: u64,
    pub rooms: u64,

    pub slots_per_room: u64,

    pub events: u64,
    pub claims: u64,
    pub attendances: u64,
    pub bookings: u64,
}

impl CalSizes {

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

    #[must_use]
    pub fn unit() -> Self {
        Self::derive(6, 8, 2)
    }

    fn derive(persons: u64, max_segments: u64, min_segments: u64) -> Self {
        let accounts = (persons / 8).max(1);
        let rooms = (persons / 16).max(1);

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

    #[must_use]
    pub fn segments_of(&self, person: u64) -> u64 {
        (self.max_segments >> (person + 1).ilog2()).max(self.min_segments)
    }

    #[must_use]
    pub fn person_has_ray(&self, person: u64) -> bool {
        person.is_multiple_of(4)
    }

    #[must_use]
    pub fn segment_is_ooo(&self, person: u64, k: u64) -> bool {
        let n = self.segments_of(person);
        (k % 5 == 4) && !(self.person_has_ray(person) && k == n - 1)
    }

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

    #[must_use]
    pub fn ooo_source_base(&self) -> u64 {
        self.attendances
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalSegment {
    pub start: i64,
    pub end: i64,
    pub ooo: bool,
}

/// # Panics
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

        let gap = if k % 3 == 2 {
            0
        } else {
            (1 + i64::try_from(rng.range(4)).expect("fits")) * HOUR
        };
        cursor = cursor + length + gap;
    }
    segments
}

/// # Panics
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

/// # Panics
#[must_use]
pub fn created_at(seed: u64, event: u64) -> i64 {
    let word = crate::corpus_gen::mix(seed, ids::EVENT, event);
    CAL_BASE + i64::try_from(word % 22_000_000).expect("fits")
}

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

#[derive(Debug, Clone, Copy)]
pub struct SegmentRow {
    pub person: u64,
    pub segment: CalSegment,

    pub event: Option<u64>,

    pub ooo_index: Option<u64>,
}

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
    vec![Value::U64(i), Value::String(format!("acct-{i:05}").into())]
}

fn person_row(sizes: &CalSizes, i: u64) -> Vec<Value> {
    vec![
        Value::U64(i),
        Value::U64(i / 8 % sizes.accounts.max(1)),
        Value::String(format!("person-{i:06}").into()),
    ]
}

fn calendar_row(i: u64) -> Vec<Value> {
    vec![Value::U64(i), Value::U64(i)]
}

fn room_row(i: u64) -> Vec<Value> {
    vec![Value::U64(i), Value::String(format!("room-{i:04}").into())]
}

fn event_row(seed: u64, row: &SegmentRow) -> Vec<Value> {
    let event = row.event.expect("busy segments carry the event id");
    vec![
        Value::U64(event),
        Value::U64(row.person), 
        Value::IntervalI64(
            bumbledb::Interval::<i64>::new(row.segment.start, row.segment.end)
                .expect("nonempty interval"),
        ),
        Value::I64(created_at(seed, event)),
        event_hash(seed, event),
    ]
}

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

/// # Panics
#[must_use]
pub fn slot_span(k: u64) -> (i64, i64) {
    let triple = i64::try_from(k / 3).expect("fits");
    let offset = match k % 3 {
        0 => 0,
        1 => 3 * HOUR,

        _ => 5 * HOUR,
    };
    let start = CAL_BASE + triple * 8 * HOUR + offset;
    (start, start + SLOT_WIDTH)
}

fn slot_row(sizes: &CalSizes, i: u64) -> Vec<Value> {
    let room = i / sizes.slots_per_room;
    let (start, end) = slot_span(i % sizes.slots_per_room);
    vec![
        Value::U64(room),
        Value::IntervalI64(Interval::<i64>::new(start, end).expect("nonempty fixed slot")),
    ]
}

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

pub fn relation_rows(cfg: GenConfig, rel: RelationId) -> Box<dyn Iterator<Item = Vec<Value>>> {
    let sizes = CalSizes::of(cfg.scale);
    relation_rows_sized(cfg, sizes, rel)
}

/// # Panics
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
        ids::SLOT => {
            Box::new((0..sizes.rooms * sizes.slots_per_room).map(move |i| slot_row(&sizes, i)))
        }
        _ => unreachable!("ten calendar relations"),
    }
}
