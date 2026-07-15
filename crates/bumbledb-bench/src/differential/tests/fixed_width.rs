//! The fixed-width differential arms (`interval<E, w>` — the width is
//! the type): a seeded op stream whose lane draws walk the BOUNDARY
//! LADDER (equal / adjacent / nested / near-ceiling starts), and the
//! exact-partition subfamily (the playlist recipe,
//! `docs/architecture/30-dependencies.md` § Q1) judged through four
//! violating deltas. Both arms run engine-vs-naive through the one
//! [`run`] loop — verdicts, citations, and answers must agree.

use bumbledb::schema::{
    IntervalElement, RelationDescriptor, SchemaDescriptor, Side, StatementDescriptor, ValueType,
};
use bumbledb::{
    AllenMask, CmpOp, Comparison, ConditionTree, Db, FieldId, FindTerm, MaskTerm, Query,
    RelationId, Rule, Value, VarId,
};

use super::{Rng, pick};
use crate::differential::{Op, Summary, run};
use crate::fixture::{TempDir, atom, field, var};
use crate::naive::{Delta, NaiveDb};

const ZONE: RelationId = RelationId(0);
const SPAN: RelationId = RelationId(1);

/// Zone(group, lane: interval<u64, 5>) under the pointwise key
/// (group, lane) — width 5, so the ladder has room to nest a start —
/// and Span(group, extent: interval<u64>) under (group, extent): the
/// GENERAL side the mixed-width Allen query classifies against (Q1).
fn ladder_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Zone".into(),
                fields: vec![
                    field("group", ValueType::U64),
                    field(
                        "lane",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                            width: Some(5),
                        },
                    ),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Span".into(),
                fields: vec![
                    field("group", ValueType::U64),
                    field(
                        "extent",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                            width: None,
                        },
                    ),
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: ZONE,
                projection: Box::new([FieldId(0), FieldId(1)]),
            },
            StatementDescriptor::Functionality {
                relation: SPAN,
                projection: Box::new([FieldId(0), FieldId(1)]),
            },
        ],
    }
}

fn zone(group: u64, start: u64) -> Vec<Value> {
    // A ladder step off a near-ceiling base can walk past the Q2 bound;
    // such a VALUE is unconstructible (`Interval::fixed` is the bound's
    // discharge), so the draw saturates to the last legal start — which
    // turns the step into a ceiling COLLISION, still a live arm.
    let start = if bumbledb::Interval::<u64>::fixed(start, 5).is_some() {
        start
    } else {
        u64::MAX - 6
    };
    vec![
        Value::U64(group),
        Value::IntervalU64(bumbledb::Interval::<u64>::fixed(start, 5).expect("in-domain lane")),
    ]
}

fn span_row(group: u64, rng: &mut Rng) -> Vec<Value> {
    let start = rng.below(60);
    let end = start + 1 + rng.below(12);
    vec![
        Value::U64(group),
        Value::IntervalU64(bumbledb::Interval::<u64>::new(start, end).expect("nonempty")),
    ]
}

/// One ladder draw: the start relative to a picked existing lane —
/// EQUAL (the collision: same start, same derived bounds), ADJACENT
/// (`start + 5`: legal by half-openness), NESTED (`start + 2`: an
/// overlap the neighbor probe must convict from DERIVED ends), or
/// NEAR-CEILING (the largest legal starts: `u64::MAX - 6` is the last
/// one the Q2 bound admits for width 5, then adjacent predecessors).
fn ladder_start(rng: &mut Rng, existing: Option<u64>) -> u64 {
    let base = existing.unwrap_or_else(|| rng.below(40) * 3);
    match rng.below(4) {
        0 => base,                            // equal
        1 => base + 5,                        // adjacent
        2 => base + 2,                        // nested/overlapping
        _ => u64::MAX - 6 - rng.below(3) * 5, // near-ceiling
    }
}

/// The query battery: the mixed-width Allen join (fixed lane against
/// general extent, one element domain — Q1's classification over
/// derived bounds) plus the plain lane scan.
fn ladder_queries() -> Vec<Query> {
    vec![
        Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![
                atom(ZONE, &[(0, var(0)), (1, var(2))]),
                atom(SPAN, &[(0, var(1)), (1, var(3))]),
            ],
            negated: vec![],
            conditions: vec![ConditionTree::Leaf(Comparison {
                op: CmpOp::Allen {
                    mask: MaskTerm::Literal(AllenMask::INTERSECTS),
                },
                lhs: var(2),
                rhs: var(3),
            })],
        }),
        Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![atom(ZONE, &[(0, var(0)), (1, var(1))])],
            negated: vec![],
            conditions: vec![],
        }),
    ]
}

/// The boundary-ladder stream: 160 seeded writes whose lane starts walk
/// equal/adjacent/nested/near-ceiling against the generator's own
/// mirror, punctuated by the mixed-width query battery — engine and
/// naive must agree on every verdict and every answer set.
#[test]
fn fixed_width_ladder_stream_agrees_with_the_engine() {
    let descriptor = ladder_schema();
    let dir = TempDir::new("differential-fixed-ladder");
    let db = Db::create(dir.path(), descriptor.clone()).expect("create engine store");
    let mut naive = NaiveDb::new(&descriptor);

    let mut mirror = NaiveDb::new(&descriptor);
    let mut rng = Rng(0x000F_15ED);
    let queries = ladder_queries();
    let mut ops = Vec::new();
    let mut commits_expected = 0u64;
    for index in 0..160u64 {
        let group = rng.below(3);
        let existing = pick(&mirror, ZONE, &mut rng).and_then(|fact| match &fact[1] {
            Value::IntervalU64(iv) => Some(iv.start()),
            _ => None,
        });
        let delta = if rng.below(8) == 0 {
            // A delete of a live lane (keyed relations only — no
            // containment to disestablish here).
            match pick(&mirror, ZONE, &mut rng) {
                Some(fact) => Delta {
                    deletes: vec![(ZONE, fact)],
                    inserts: vec![],
                },
                None => Delta {
                    deletes: vec![],
                    inserts: vec![(SPAN, span_row(group, &mut rng))],
                },
            }
        } else if rng.below(5) == 0 {
            Delta {
                deletes: vec![],
                inserts: vec![(SPAN, span_row(group, &mut rng))],
            }
        } else {
            Delta {
                deletes: vec![],
                inserts: vec![(ZONE, zone(group, ladder_start(&mut rng, existing)))],
            }
        };
        if mirror.apply(&delta).is_ok() {
            commits_expected += 1;
        }
        ops.push(Op::Write(delta));
        if (index + 1) % 8 == 0 {
            for query in &queries {
                ops.push(Op::Query {
                    query: query.clone(),
                    params: vec![],
                });
            }
        }
    }
    let summary: Summary = run(&db, &mut naive, &ops).unwrap_or_else(|divergence| {
        panic!("engine and model disagree on the fixed-width ladder: {divergence:#?}");
    });
    assert!(summary.commits >= 30, "commits: {summary:?}");
    assert!(summary.aborts >= 20, "aborts: {summary:?}");
    assert_eq!(summary.commits, commits_expected, "mirror agreement");
    assert!(
        !naive.relation(ZONE).is_empty(),
        "the stream should leave live lanes"
    );
}

// ---------- the exact-partition subfamily ----------

const PLAYLIST: RelationId = RelationId(0);
const SLOT: RelationId = RelationId(1);

/// The playlist recipe (the ordering triple's judgment core): a general
/// span exact-partitioned (`==`, its two containments) by
/// `interval<u64, 1>` unit slots, both sides under pointwise keys.
fn playlist_schema() -> SchemaDescriptor {
    let side = |relation: RelationId| Side {
        relation,
        projection: Box::new([FieldId(0), FieldId(1)]),
        selection: Box::new([]),
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Playlist".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field(
                        "span",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                            width: None,
                        },
                    ),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Slot".into(),
                fields: vec![
                    field("playlist", ValueType::U64),
                    field(
                        "slot",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                            width: Some(1),
                        },
                    ),
                    field("track", ValueType::U64),
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: PLAYLIST,
                projection: Box::new([FieldId(0), FieldId(1)]),
            },
            StatementDescriptor::Functionality {
                relation: SLOT,
                projection: Box::new([FieldId(0), FieldId(1)]),
            },
            StatementDescriptor::Containment {
                source: side(SLOT),
                target: side(PLAYLIST),
            },
            StatementDescriptor::Containment {
                source: side(PLAYLIST),
                target: side(SLOT),
            },
        ],
    }
}

fn playlist(id: u64, start: u64, end: u64) -> (RelationId, Vec<Value>) {
    (
        PLAYLIST,
        vec![
            Value::U64(id),
            Value::IntervalU64(bumbledb::Interval::<u64>::new(start, end).expect("nonempty")),
        ],
    )
}

fn unit_slot(playlist: u64, at: u64, track: u64) -> (RelationId, Vec<Value>) {
    (
        SLOT,
        vec![
            Value::U64(playlist),
            Value::IntervalU64(bumbledb::Interval::<u64>::fixed(at, 1).expect("in-domain slot")),
            Value::U64(track),
        ],
    )
}

/// The exact-partition subfamily: one green tiling, then the FOUR
/// violating deltas — a gap tiling from scratch, an overlapping unit
/// slot, a slot past the span, and a slot delete that tears a hole in a
/// committed tiling. Each judged identically by engine and naive
/// through the one [`run`] loop (aborts leave the green state intact,
/// so every later delta still sees the tiling).
#[test]
fn exact_partition_subfamily_judges_the_four_violating_deltas() {
    let descriptor = playlist_schema();
    let dir = TempDir::new("differential-exact-partition");
    let db = Db::create(dir.path(), descriptor.clone()).expect("create engine store");
    let mut naive = NaiveDb::new(&descriptor);

    let green = Delta {
        deletes: vec![],
        inserts: vec![
            playlist(1, 0, 3),
            unit_slot(1, 0, 100),
            unit_slot(1, 1, 200),
            unit_slot(1, 2, 300),
        ],
    };
    // Delta 1 — the gap: a second playlist whose tiling skips point 1.
    let gap = Delta {
        deletes: vec![],
        inserts: vec![
            playlist(2, 0, 3),
            unit_slot(2, 0, 100),
            unit_slot(2, 2, 300),
        ],
    };
    // Delta 2 — the overlap: a colliding unit slot (width 1 makes
    // overlap collision; the pointwise key convicts, key phase).
    let overlap = Delta {
        deletes: vec![],
        inserts: vec![unit_slot(1, 1, 999)],
    };
    // Delta 3 — past the span: a unit slot outside `[0, 3)` (the
    // slot-side coverage direction convicts).
    let past = Delta {
        deletes: vec![],
        inserts: vec![unit_slot(1, 3, 999)],
    };
    // Delta 4 — the torn tiling: deleting a middle slot re-judges the
    // surviving playlist's coverage and convicts the gap it left.
    let tear = Delta {
        deletes: vec![(SLOT, unit_slot(1, 1, 200).1)],
        inserts: vec![],
    };
    let ops = vec![
        Op::Write(green),
        Op::Write(gap),
        Op::Write(overlap),
        Op::Write(past),
        Op::Write(tear),
    ];
    let summary: Summary = run(&db, &mut naive, &ops).unwrap_or_else(|divergence| {
        panic!("engine and model disagree on the exact-partition subfamily: {divergence:#?}");
    });
    assert_eq!(summary.commits, 1, "only the green tiling commits");
    assert_eq!(summary.aborts, 4, "all four violating deltas abort");
}
