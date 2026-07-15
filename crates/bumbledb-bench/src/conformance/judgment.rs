//! The judgment conformance lane — the WRITE-side third oracle.
//!
//! The query lane compares answer sets against the executable
//! denotation; this lane compares COMMIT VERDICTS against the
//! executable judge: `lean/Bumbledb/Decide.lean: Txn.judgeB`, proved
//! to agree with the model's `Txn.judge` verdict and violation sets
//! phase for phase (`Txn.judgeB_agrees`). One JSON document per case:
//! the theory
//! (sealed relations, ground axioms, the MATERIALIZED statement list —
//! indices are the engine's statement ids), the committed pre-state,
//! one delta, and the verdict both Rust oracles agreed on — accept, or
//! the rejecting phase with its complete violation set as statement
//! ids. Format in `lean/conformance/README.md` § judgment cases.
//!
//! **The verdict is compared per phase.** A key (functionality)
//! violation preempts the statement phase on all three oracles
//! (`lean/Bumbledb/Txn.lean: judge_key_preempts`); the statement phase
//! cites containment and cardinality violations together. The
//! containment `Direction` is a Rust-side refinement below the Lean
//! altitude (`Txn.lean`'s violation sets are per-statement), so the
//! serialized set deduplicates a statement cited in both directions —
//! recorded in the README.
//!
//! Every fixture is hand-authored (no seeded arm: judgment cases are
//! theorem-shaped, not distribution-shaped), and every document is
//! written only after the engine and the naive model agreed on the
//! verdict — a disagreement panics as a trophy, exactly the query
//! lane's rule.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, LiteralSet, RelationDescriptor, RelationId, Row,
    SchemaDescriptor, Side, StatementDescriptor, ValueType,
};
use bumbledb::{Db, Interval, Value};

use crate::differential::{self, Verdict};
use crate::naive::{Delta, NaiveDb, Violation};

use super::ScratchDir;

type Facts = Vec<(RelationId, Vec<Value>)>;

/// One hand judgment fixture: a schema, a green base state, and the
/// judged delta. The verdict is never written down here — it is
/// COMPUTED through both Rust oracles and recorded only on agreement.
struct JudgmentFixture {
    name: &'static str,
    schema: SchemaDescriptor,
    base: Facts,
    deletes: Facts,
    inserts: Facts,
}

/// The verdict both Rust oracles agreed on, in the lane's shape.
enum JVerdict {
    Accept,
    Reject {
        key_phase: bool,
        violations: Vec<u16>,
    },
}

fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    }
}

fn u64_relation(name: &str, fields: &[&str]) -> RelationDescriptor {
    RelationDescriptor {
        extension: None,
        name: name.into(),
        fields: fields.iter().map(|f| field(f, ValueType::U64)).collect(),
    }
}

fn side(relation: RelationId, projection: &[u16]) -> Side {
    Side {
        relation,
        projection: projection.iter().map(|f| FieldId(*f)).collect(),
        selection: Box::new([]),
    }
}

fn side_where(relation: RelationId, projection: &[u16], selection: &[(u16, LiteralSet)]) -> Side {
    Side {
        relation,
        projection: projection.iter().map(|f| FieldId(*f)).collect(),
        selection: selection
            .iter()
            .map(|(f, set)| (FieldId(*f), set.clone()))
            .collect(),
    }
}

fn iv(start: u64, end: u64) -> Value {
    Value::IntervalU64(Interval::<u64>::new(start, end).expect("fixture intervals are nonempty"))
}

// ---------- the marks world: the window over set-selected children ----------

const HOLDER: RelationId = RelationId(0);
const ACCOUNT: RelationId = RelationId(1);
const ITEM: RelationId = RelationId(2);

/// Holder(id, tag; key id); Account(holder, kind, num) windowed
/// `Account(holder | kind ∈ {1, 2}) in 1..2 per Holder(id)` — a
/// SET selection, so the lane's σ compares disjunctively; Item is
/// unconstrained payload data riding the same commits.
fn marks_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            u64_relation("Holder", &["id", "tag"]),
            u64_relation("Account", &["holder", "kind", "num"]),
            u64_relation("Item", &["doc", "pos", "note"]),
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: HOLDER,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Cardinality {
                source: side_where(
                    ACCOUNT,
                    &[0],
                    &[(
                        1,
                        LiteralSet::Many(Box::new([Value::U64(1), Value::U64(2)])),
                    )],
                ),
                lo: 1,
                hi: Some(2),
                target: side(HOLDER, &[0]),
            },
        ],
    }
}

fn holder(id: u64) -> (RelationId, Vec<Value>) {
    (HOLDER, vec![Value::U64(id), Value::U64(0)])
}

fn holder_tagged(id: u64, tag: u64) -> (RelationId, Vec<Value>) {
    (HOLDER, vec![Value::U64(id), Value::U64(tag)])
}

fn account(holder: u64, kind: u64, num: u64) -> (RelationId, Vec<Value>) {
    (
        ACCOUNT,
        vec![Value::U64(holder), Value::U64(kind), Value::U64(num)],
    )
}

fn item(doc: u64, pos: u64, note: u64) -> (RelationId, Vec<Value>) {
    (
        ITEM,
        vec![Value::U64(doc), Value::U64(pos), Value::U64(note)],
    )
}

// ---------- the ledger world: containment and cardinality together ----------

/// Holder(id, tag; key id); Account(holder, kind, num) under BOTH
/// statement forms at once: the reference containment
/// `Account(holder) <= Holder(id)` (statement 1) and the window
/// `Account(holder | kind == 1) in 1..2 per Holder(id)` (statement 2)
/// — the one roster schema whose statement phase can cite containment
/// and cardinality TOGETHER, so the ordered multi-citation verdict
/// surface (`lean/Main.lean: RVerdict`'s list `BEq`, ascending indices
/// via `verdictOf`) is actually exercised.
fn ledger_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            u64_relation("Holder", &["id", "tag"]),
            u64_relation("Account", &["holder", "kind", "num"]),
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: HOLDER,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Containment {
                source: side(ACCOUNT, &[0]),
                target: side(HOLDER, &[0]),
            },
            StatementDescriptor::Cardinality {
                source: side_where(ACCOUNT, &[0], &[(1, LiteralSet::One(Value::U64(1)))]),
                lo: 1,
                hi: Some(2),
                target: side(HOLDER, &[0]),
            },
        ],
    }
}

// ---------- the exactness world: n..n and 0..* windows ----------

/// Holder + Account under `in 2..2 per` (exactness) and a second
/// `in 0..*` window (the provably vacuous default posture,
/// `lean/Bumbledb/Cardinality.lean: zero_star_admits`).
fn exact_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            u64_relation("Holder", &["id", "tag"]),
            u64_relation("Account", &["holder", "kind", "num"]),
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: HOLDER,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Cardinality {
                source: side_where(ACCOUNT, &[0], &[(1, LiteralSet::One(Value::U64(1)))]),
                lo: 2,
                hi: Some(2),
                target: side(HOLDER, &[0]),
            },
            StatementDescriptor::Cardinality {
                source: side_where(ACCOUNT, &[0], &[(1, LiteralSet::One(Value::U64(9)))]),
                lo: 0,
                hi: None,
                target: side(HOLDER, &[0]),
            },
        ],
    }
}

// ---------- the permuted-interval world (the docketed lock) ----------

const SLOT: RelationId = RelationId(0);
const CLAIM: RelationId = RelationId(1);

/// Slot(id, span) under the pointwise key DECLARED `(id, span)`;
/// the containment statement WRITTEN interval-first on both sides —
/// `Claim(span, id) <= Slot(span, id)` — accepted through the
/// set-canonical key resolution (the `FieldSet` doctrine,
/// `lean/Bumbledb/Schema.lean: Header.intervalSplit`) and judged as
/// coverage.
fn permuted_schema() -> SchemaDescriptor {
    let interval = ValueType::Interval {
        element: bumbledb::schema::IntervalElement::U64,
        width: None,
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Slot".into(),
                fields: vec![field("id", ValueType::U64), field("span", interval.clone())],
            },
            RelationDescriptor {
                extension: None,
                name: "Claim".into(),
                fields: vec![field("span", interval), field("id", ValueType::U64)],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: SLOT,
                projection: Box::new([FieldId(0), FieldId(1)]),
            },
            StatementDescriptor::Containment {
                source: side(CLAIM, &[0, 1]),
                target: side(SLOT, &[1, 0]),
            },
        ],
    }
}

fn slot(id: u64, start: u64, end: u64) -> (RelationId, Vec<Value>) {
    (SLOT, vec![Value::U64(id), iv(start, end)])
}

fn claim(start: u64, end: u64, id: u64) -> (RelationId, Vec<Value>) {
    (CLAIM, vec![iv(start, end), Value::U64(id)])
}

// ---------- the fixed-width playlist world (Q1 + interval<E, w>) ----------

const PLAYLIST: RelationId = RelationId(0);
const FSLOT: RelationId = RelationId(1);

/// The playlist recipe, verbatim (the cookbook's ordering triple —
/// Q1's own replacement recipe): a general `interval<u64>` span
/// exact-partitioned (`==`, its two containments) by `interval<u64, 1>`
/// unit slots, both sides under pointwise keys. Element-domain typing
/// meets at the containments' interval position (widths free —
/// `lean/Bumbledb/Schema.lean: Value.points_one_tag_u64`).
fn playlist_schema() -> SchemaDescriptor {
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
                            element: bumbledb::schema::IntervalElement::U64,
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
                            element: bumbledb::schema::IntervalElement::U64,
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
                relation: FSLOT,
                projection: Box::new([FieldId(0), FieldId(1)]),
            },
            StatementDescriptor::Containment {
                source: side(FSLOT, &[0, 1]),
                target: side(PLAYLIST, &[0, 1]),
            },
            StatementDescriptor::Containment {
                source: side(PLAYLIST, &[0, 1]),
                target: side(FSLOT, &[0, 1]),
            },
        ],
    }
}

fn playlist(id: u64, start: u64, end: u64) -> (RelationId, Vec<Value>) {
    (PLAYLIST, vec![Value::U64(id), iv(start, end)])
}

/// A unit slot: the fixed value spells `[at, at + 1)` — the serializer
/// re-derives the `[start, width]` spelling from the field's type.
fn unit_slot(playlist: u64, at: u64, track: u64) -> (RelationId, Vec<Value>) {
    (
        FSLOT,
        vec![Value::U64(playlist), iv(at, at + 1), Value::U64(track)],
    )
}

/// LaneU(id, lane interval<u64, 5>) + LaneI(id, lane interval<i64, 5>)
/// — both element domains, keyed pointwise; no containments (the
/// boundary-start fixture exercises the encodings, not coverage).
fn lanes_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "LaneU".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field(
                        "lane",
                        ValueType::Interval {
                            element: bumbledb::schema::IntervalElement::U64,
                            width: Some(5),
                        },
                    ),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "LaneI".into(),
                fields: vec![
                    field("id", ValueType::U64),
                    field(
                        "lane",
                        ValueType::Interval {
                            element: bumbledb::schema::IntervalElement::I64,
                            width: Some(5),
                        },
                    ),
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::new([FieldId(0), FieldId(1)]),
            },
            StatementDescriptor::Functionality {
                relation: RelationId(1),
                projection: Box::new([FieldId(0), FieldId(1)]),
            },
        ],
    }
}

fn lane_u(id: u64, start: u64) -> (RelationId, Vec<Value>) {
    (RelationId(0), vec![Value::U64(id), iv(start, start + 5)])
}

fn lane_i(id: u64, start: i64) -> (RelationId, Vec<Value>) {
    (
        RelationId(1),
        vec![
            Value::U64(id),
            Value::IntervalI64(
                Interval::<i64>::new(start, start + 5).expect("fixture lanes are in-domain"),
            ),
        ],
    )
}

// ---------- the closed-target world ----------

/// Currency closed (two handle-only axioms) referenced by
/// Account(currency) — the ground-axioms merge and the member-set
/// judgment through the lane.
fn closed_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: Some(Box::new([
                    Row {
                        handle: "Usd".into(),
                        values: Box::new([]),
                    },
                    Row {
                        handle: "Eur".into(),
                        values: Box::new([]),
                    },
                ])),
                name: "Currency".into(),
                fields: vec![],
            },
            u64_relation("Account", &["id", "currency"]),
        ],
        statements: vec![StatementDescriptor::Containment {
            source: side(RelationId(1), &[1]),
            target: side(RelationId(0), &[0]),
        }],
    }
}

fn cur_account(id: u64, currency: u64) -> (RelationId, Vec<Value>) {
    (RelationId(1), vec![Value::U64(id), Value::U64(currency)])
}

/// The starter roster: both classical forms (scalar key, containment —
/// pointwise key and coverage through the permuted world), the window
/// form, the two-phase preemption
/// mix, set-selections, the exactness/vacuity/empty-parent window
/// boundaries, the delete-then-reinsert touched-group seam, and the
/// permuted-interval docketed lock.
#[expect(
    clippy::too_many_lines,
    reason = "one flat fixture roster, data not logic"
)]
fn fixtures() -> Vec<JudgmentFixture> {
    vec![
        JudgmentFixture {
            name: "judgment-accept-green",
            schema: marks_schema(),
            base: vec![],
            deletes: vec![],
            inserts: vec![holder(7), account(7, 1, 0), item(1, 1, 10), item(1, 2, 11)],
        },
        JudgmentFixture {
            name: "judgment-window-floor-childless",
            schema: marks_schema(),
            base: vec![holder(7), account(7, 1, 0)],
            deletes: vec![],
            inserts: vec![holder(8)],
        },
        JudgmentFixture {
            name: "judgment-window-floor-by-deletion",
            schema: marks_schema(),
            base: vec![holder(7), account(7, 1, 0)],
            deletes: vec![account(7, 1, 0)],
            inserts: vec![],
        },
        JudgmentFixture {
            name: "judgment-window-set-selection-ceiling",
            schema: marks_schema(),
            base: vec![holder(7), account(7, 1, 0), account(7, 2, 1)],
            deletes: vec![],
            inserts: vec![account(7, 2, 2)],
        },
        JudgmentFixture {
            name: "judgment-preemption-mix",
            schema: marks_schema(),
            base: vec![],
            deletes: vec![],
            inserts: vec![holder_tagged(9, 0), holder_tagged(9, 1), item(5, 2, 0)],
        },
        JudgmentFixture {
            name: "judgment-statement-mix",
            schema: marks_schema(),
            base: vec![holder(7), account(7, 1, 0)],
            deletes: vec![],
            inserts: vec![holder(8), item(6, 1, 0), item(6, 3, 1)],
        },
        JudgmentFixture {
            name: "judgment-delete-reinsert-touched-group",
            schema: marks_schema(),
            base: vec![holder(7), account(7, 1, 0), item(1, 1, 10), item(1, 2, 11)],
            deletes: vec![item(1, 2, 11), account(7, 1, 0)],
            inserts: vec![item(1, 2, 11), account(7, 1, 0)],
        },
        JudgmentFixture {
            name: "judgment-window-exact-pass",
            schema: exact_schema(),
            base: vec![],
            deletes: vec![],
            inserts: vec![holder(1), account(1, 1, 0), account(1, 1, 1)],
        },
        JudgmentFixture {
            name: "judgment-window-exact-under",
            schema: exact_schema(),
            base: vec![],
            deletes: vec![],
            inserts: vec![holder(2), account(2, 1, 0)],
        },
        JudgmentFixture {
            name: "judgment-window-vacuity",
            schema: exact_schema(),
            base: vec![holder(1), account(1, 1, 0), account(1, 1, 1)],
            deletes: vec![],
            inserts: vec![
                account(1, 9, 0),
                account(1, 9, 1),
                account(1, 9, 2),
                account(1, 9, 3),
            ],
        },
        JudgmentFixture {
            name: "judgment-window-empty-parent",
            schema: exact_schema(),
            base: vec![],
            deletes: vec![],
            inserts: vec![account(3, 1, 0)],
        },
        JudgmentFixture {
            name: "judgment-permuted-interval-covered",
            schema: permuted_schema(),
            base: vec![slot(5, 10, 20), slot(5, 20, 30)],
            deletes: vec![],
            inserts: vec![claim(12, 28, 5)],
        },
        JudgmentFixture {
            name: "judgment-permuted-interval-uncovered",
            schema: permuted_schema(),
            base: vec![slot(5, 10, 20), slot(5, 20, 30)],
            deletes: vec![],
            inserts: vec![claim(25, 35, 5)],
        },
        JudgmentFixture {
            name: "judgment-permuted-pointwise-key",
            schema: permuted_schema(),
            base: vec![slot(5, 10, 20)],
            deletes: vec![],
            inserts: vec![slot(5, 15, 25)],
        },
        JudgmentFixture {
            name: "judgment-closed-ref-valid",
            schema: closed_schema(),
            base: vec![],
            deletes: vec![],
            inserts: vec![cur_account(1, 0), cur_account(2, 1)],
        },
        JudgmentFixture {
            name: "judgment-closed-ref-invalid",
            schema: closed_schema(),
            base: vec![],
            deletes: vec![],
            inserts: vec![cur_account(3, 9)],
        },
        // The playlist recipe verbatim (Q1 + interval<u64, 1>): an
        // exact tiling COMMITS — unit slots partition the span, mixed
        // widths meeting at the containments' interval position.
        JudgmentFixture {
            name: "judgment-fixed-partition-tiling",
            schema: playlist_schema(),
            base: vec![],
            deletes: vec![],
            inserts: vec![
                playlist(1, 0, 3),
                unit_slot(1, 0, 100),
                unit_slot(1, 1, 200),
                unit_slot(1, 2, 300),
            ],
        },
        // A gap in the partition ABORTS: the span's point 1 has no
        // covering unit slot — the Playlist-side coverage containment
        // (statement 3) convicts.
        JudgmentFixture {
            name: "judgment-fixed-partition-gap",
            schema: playlist_schema(),
            base: vec![],
            deletes: vec![],
            inserts: vec![
                playlist(1, 0, 3),
                unit_slot(1, 0, 100),
                unit_slot(1, 2, 300),
            ],
        },
        // An overlapping unit slot ABORTS in the KEY phase: width 1
        // makes overlap collision, and the pointwise key's disjointness
        // (statement 1) preempts the statement phase
        // (`lean/Bumbledb/Txn.lean: judge_key_preempts`).
        JudgmentFixture {
            name: "judgment-fixed-partition-overlap",
            schema: playlist_schema(),
            base: vec![
                playlist(1, 0, 3),
                unit_slot(1, 0, 100),
                unit_slot(1, 1, 200),
                unit_slot(1, 2, 300),
            ],
            deletes: vec![],
            inserts: vec![unit_slot(1, 1, 999)],
        },
        // Boundary starts, both element domains: the largest legal
        // starts (`start + w = MAX_END − 1`) and the i64 floor, plus an
        // adjacent pair — all ACCEPT (the Q2 bound's positive edge; the
        // at-bound negatives are decoder convictions, not commit
        // verdicts: `Conformance.lean`'s ceiling `#guard`s and
        // `verify_store`'s at-rest fixture own them).
        JudgmentFixture {
            name: "judgment-fixed-boundary-starts",
            schema: lanes_schema(),
            base: vec![],
            deletes: vec![],
            inserts: vec![
                lane_u(1, u64::MAX - 11),
                lane_u(2, u64::MAX - 6),
                lane_i(1, i64::MIN),
                lane_i(2, i64::MAX - 6),
            ],
        },
        // The multi-citation statement phase: one delta violating the
        // containment (statement 1 — account 8 references no holder;
        // kind 5 sits outside the window's σ) AND the window floor
        // (statement 2 — holder 9 lands childless) with clean keys.
        // The recorded verdict is the ordered ASCENDING list [1, 2]
        // mixing citation kinds — the whole-list comparison surface
        // (`lean/Main.lean: RVerdict` list `BEq`, `verdictOf`'s indexed
        // filterMap) exercised beyond length 1 for the first time.
        JudgmentFixture {
            name: "judgment-statement-mixed-citations",
            schema: ledger_schema(),
            base: vec![holder(7), account(7, 1, 0)],
            deletes: vec![],
            inserts: vec![account(8, 5, 0), holder(9)],
        },
        // One containment cited in BOTH directions by one delta: the
        // deleted slot uncovers the held-before claim (target
        // direction) while the inserted claim lands uncovered (source
        // direction). The `Direction` refinement sits below the Lean
        // altitude, so the serialized set must collapse the double
        // citation to the single statement id — the `ids.dedup()` rule
        // the README records, previously covered by no case.
        JudgmentFixture {
            name: "judgment-containment-both-directions",
            schema: permuted_schema(),
            base: vec![slot(5, 10, 20), claim(12, 18, 5)],
            deletes: vec![slot(5, 10, 20)],
            inserts: vec![claim(40, 50, 5)],
        },
        // The multi-key rejection: overlapping playlist spans collide
        // on statement 0's pointwise key while overlapping unit slots
        // collide on statement 1's — the key phase's COMPLETE set is
        // the ascending pair [0, 1]
        // (`lean/Bumbledb/Txn.lean: judge_key_preempts` drops the
        // simultaneous containment convictions), where every prior
        // key-phase case cited exactly one statement.
        JudgmentFixture {
            name: "judgment-multi-key-collisions",
            schema: playlist_schema(),
            base: vec![],
            deletes: vec![],
            inserts: vec![
                playlist(1, 0, 3),
                playlist(1, 2, 5),
                unit_slot(1, 0, 100),
                unit_slot(1, 0, 999),
            ],
        },
    ]
}

// ---------- the serializer ----------

/// One value in the tagged compact form, AT ITS FIELD'S TYPE: a
/// fixed-width position renders `[start, width]` under the family's
/// own tag (the width is the type — `Value` spells bounds, so the
/// field type re-derives the spelling; `Conformance.lean: decodeValue`
/// re-checks the Q2 bound on the way back in). Judgment fixtures are
/// hand-authored and carry no strings or masks — the two tags that
/// would need a per-case context.
fn push_value(out: &mut String, value: &Value, ty: Option<&ValueType>) {
    if let Some(ValueType::Interval { width: Some(w), .. }) = ty {
        match value {
            Value::IntervalU64(iv) => {
                debug_assert_eq!(iv.end() - iv.start(), *w, "typed writes checked the width");
                let _ = write!(out, "{{\"interval_u64_fixed\":[{},{w}]}}", iv.start());
                return;
            }
            Value::IntervalI64(iv) => {
                let _ = write!(out, "{{\"interval_i64_fixed\":[{},{w}]}}", iv.start());
                return;
            }
            _ => {}
        }
    }
    match value {
        Value::Bool(v) => {
            let _ = write!(out, "{{\"bool\":{v}}}");
        }
        Value::U64(v) => {
            let _ = write!(out, "{{\"u64\":{v}}}");
        }
        Value::I64(v) => {
            let _ = write!(out, "{{\"i64\":{v}}}");
        }
        Value::FixedBytes(bytes) => {
            out.push_str("{\"bytes\":[");
            for (index, byte) in bytes.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                let _ = write!(out, "{byte}");
            }
            out.push_str("]}");
        }
        Value::IntervalU64(iv) => {
            let _ = write!(out, "{{\"interval_u64\":[{},{}]}}", iv.start(), iv.end());
        }
        Value::IntervalI64(iv) => {
            let _ = write!(out, "{{\"interval_i64\":[{},{}]}}", iv.start(), iv.end());
        }
        Value::String(_) | Value::AllenMask(_) => {
            unreachable!("judgment fixtures carry no strings or masks")
        }
    }
}

fn push_fact(out: &mut String, fact: &[Value], types: &[ValueType]) {
    out.push('[');
    for (index, value) in fact.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_value(out, value, types.get(index));
    }
    out.push(']');
}

/// One `{relation, facts}` block list from per-relation fact rows.
/// `id_prefixed` is the ground-axiom shape: facts open with the
/// synthetic row id, so positional types shift by one.
fn push_blocks(
    out: &mut String,
    blocks: &[(RelationId, Vec<Vec<Value>>)],
    schema: &SchemaDescriptor,
    id_prefixed: bool,
) {
    out.push('[');
    for (index, (relation, facts)) in blocks.iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }
        let mut types: Vec<ValueType> = if id_prefixed {
            vec![ValueType::U64]
        } else {
            vec![]
        };
        types.extend(
            schema.relations[relation.0 as usize]
                .fields
                .iter()
                .map(|field| field.value_type.clone()),
        );
        let _ = write!(out, "{{\"relation\":{},\"facts\":[", relation.0);
        for (position, fact) in facts.iter().enumerate() {
            if position > 0 {
                out.push_str(",\n");
            } else {
                out.push('\n');
            }
            push_fact(out, fact, &types);
        }
        if facts.is_empty() {
            out.push_str("]}");
        } else {
            out.push_str("\n]}");
        }
    }
    out.push(']');
}

/// The sealed positional type spelling (a closed relation's list opens
/// with the synthetic id — the naive model's own sealed field space).
fn push_relations(out: &mut String, schema: &SchemaDescriptor) {
    out.push('[');
    for (index, relation) in schema.relations.iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }
        let _ = write!(
            out,
            "{{\"id\":{index},\"name\":\"{}\",\"closed\":{},\"fields\":[",
            relation.name,
            relation.extension.is_some()
        );
        let mut first = true;
        if relation.extension.is_some() {
            out.push_str("\"u64\"");
            first = false;
        }
        for field in &relation.fields {
            if !first {
                out.push(',');
            }
            first = false;
            let _ = write!(out, "\"{}\"", super::type_name(&field.value_type));
        }
        out.push_str("]}");
    }
    out.push(']');
}

fn push_field_ids(out: &mut String, fields: &[FieldId]) {
    out.push('[');
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let _ = write!(out, "{}", field.0);
    }
    out.push(']');
}

fn push_side(out: &mut String, side: &Side) {
    let _ = write!(out, "{{\"relation\":{},\"projection\":", side.relation.0);
    push_field_ids(out, &side.projection);
    out.push_str(",\"selection\":[");
    for (index, (field, literals)) in side.selection.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let _ = write!(out, "[{},[", field.0);
        for (position, literal) in literals.literals().iter().enumerate() {
            if position > 0 {
                out.push(',');
            }
            // Selection literals spell their own type (a σ over a
            // fixed-width field is not a fixture shape this lane
            // carries).
            push_value(out, literal, None);
        }
        out.push_str("]]");
    }
    out.push_str("]}");
}

/// The MATERIALIZED statement list — indices are the engine's
/// statement ids (the naive model's own materialization rule).
fn push_statements(out: &mut String, schema: &SchemaDescriptor) {
    out.push('[');
    for (index, statement) in schema.materialized_statements().iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }
        match statement {
            StatementDescriptor::Functionality {
                relation,
                projection,
            } => {
                let _ = write!(
                    out,
                    "{{\"functionality\":{{\"relation\":{},\"projection\":",
                    relation.0
                );
                push_field_ids(out, projection);
                out.push_str("}}");
            }
            StatementDescriptor::Containment { source, target } => {
                out.push_str("{\"containment\":{\"source\":");
                push_side(out, source);
                out.push_str(",\"target\":");
                push_side(out, target);
                out.push_str("}}");
            }
            StatementDescriptor::Cardinality {
                source,
                lo,
                hi,
                target,
            } => {
                out.push_str("{\"cardinality\":{\"source\":");
                push_side(out, source);
                let _ = write!(out, ",\"window\":{{\"lo\":{lo}");
                if let Some(hi) = hi {
                    let _ = write!(out, ",\"hi\":{hi}");
                }
                out.push_str("},\"target\":");
                push_side(out, target);
                out.push_str("}}");
            }
        }
    }
    out.push(']');
}

fn push_verdict(out: &mut String, verdict: &JVerdict) {
    match verdict {
        JVerdict::Accept => out.push_str("\"accept\""),
        JVerdict::Reject {
            key_phase,
            violations,
        } => {
            let _ = write!(
                out,
                "{{\"reject\":{{\"phase\":\"{}\",\"violations\":[",
                if *key_phase { "key" } else { "statement" }
            );
            for (index, id) in violations.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                let _ = write!(out, "{id}");
            }
            out.push_str("]}}");
        }
    }
}

/// Groups a flat fact list into per-relation blocks, relation-id
/// ascending, facts in listed order.
fn grouped(facts: &Facts) -> Vec<(RelationId, Vec<Vec<Value>>)> {
    let mut map: BTreeMap<RelationId, Vec<Vec<Value>>> = BTreeMap::new();
    for (relation, fact) in facts {
        map.entry(*relation).or_default().push(fact.clone());
    }
    map.into_iter().collect()
}

/// The agreed verdict in the lane's shape: the phase is read off the
/// citation kinds (a rejection is one phase's complete set — keys
/// preempt, `lean/Bumbledb/Txn.lean: judge_key_preempts`), and the
/// statement-id set deduplicates a containment cited in both
/// directions (the `Direction` refinement sits below the Lean
/// altitude).
fn lane_verdict(name: &str, verdict: &Verdict) -> JVerdict {
    match verdict {
        Verdict::Committed => JVerdict::Accept,
        Verdict::Aborted(violations) => {
            let key_phase = violations
                .iter()
                .all(|violation| matches!(violation, Violation::Functionality { .. }));
            let mut ids: Vec<u16> = violations
                .iter()
                .map(|violation| match violation {
                    Violation::Functionality { statement }
                    | Violation::Containment { statement, .. }
                    | Violation::Cardinality { statement } => statement.0,
                    Violation::ClosedRelationWrite { .. } => {
                        panic!("judgment fixture {name} wrote a closed relation")
                    }
                })
                .collect();
            ids.dedup();
            JVerdict::Reject {
                key_phase,
                violations: ids,
            }
        }
    }
}

/// One fixture through BOTH Rust oracles: base committed green on
/// each, the delta judged on each, verdicts asserted equal (a
/// disagreement is a TROPHY — this builder refuses to check in a
/// disputed case), then the serialized document.
///
/// # Panics
///
/// On an engine-vs-naive disagreement, a refused base commit, or a
/// closed-relation write in a fixture.
#[expect(
    clippy::too_many_lines,
    reason = "one flat document assembly, data not logic"
)]
fn render_fixture(fixture: &JudgmentFixture) -> String {
    let dir = ScratchDir::new(&format!("judgment-{}", fixture.name));
    let db = Db::create(&dir.0, fixture.schema.clone()).expect("create judgment fixture store");
    let mut naive = NaiveDb::new(&fixture.schema);
    let base = Delta {
        deletes: vec![],
        inserts: fixture.base.clone(),
    };
    if !fixture.base.is_empty() {
        assert!(
            matches!(differential::engine_write(&db, &base), Verdict::Committed),
            "judgment fixture {}: the engine refused the base state",
            fixture.name
        );
        naive.apply(&base).unwrap_or_else(|violations| {
            panic!(
                "judgment fixture {}: the model refused the base state: {violations:?}",
                fixture.name
            )
        });
    }

    // The committed pre-state, from the model (canonical order), every
    // ordinary relation listed — an empty relation is an empty block.
    let instance: Vec<(RelationId, Vec<Vec<Value>>)> = fixture
        .schema
        .relations
        .iter()
        .enumerate()
        .filter(|(_, relation)| relation.extension.is_none())
        .map(|(index, _)| {
            let relation = RelationId(u32::try_from(index).expect("relation count fits u32"));
            (
                relation,
                naive
                    .relation(relation)
                    .iter()
                    .map(|tuple| tuple.0.clone())
                    .collect(),
            )
        })
        .collect();
    // The ground axioms, id-prefixed — the naive model's own seeding.
    let axioms: Vec<(RelationId, Vec<Vec<Value>>)> = fixture
        .schema
        .relations
        .iter()
        .enumerate()
        .filter_map(|(index, relation)| {
            let extension = relation.extension.as_ref()?;
            let relation = RelationId(u32::try_from(index).expect("relation count fits u32"));
            Some((
                relation,
                extension
                    .iter()
                    .enumerate()
                    .map(|(row, axiom)| {
                        let mut fact = vec![Value::U64(
                            u64::try_from(row).expect("extension rows fit u64"),
                        )];
                        fact.extend(axiom.values.iter().cloned());
                        fact
                    })
                    .collect(),
            ))
        })
        .collect();

    let delta = Delta {
        deletes: fixture.deletes.clone(),
        inserts: fixture.inserts.clone(),
    };
    let engine = differential::engine_write(&db, &delta);
    let model = match naive.apply(&delta) {
        Ok(()) => Verdict::Committed,
        Err(violations) => Verdict::Aborted(violations),
    };
    assert_eq!(
        engine, model,
        "TROPHY (engine vs naive) on judgment case {}: triage per the fuzzing charter",
        fixture.name
    );
    let verdict = lane_verdict(fixture.name, &engine);

    let mut relations_block = String::new();
    push_relations(&mut relations_block, &fixture.schema);
    let mut statements_block = String::new();
    push_statements(&mut statements_block, &fixture.schema);
    let mut axioms_block = String::new();
    push_blocks(&mut axioms_block, &axioms, &fixture.schema, true);
    let mut instance_block = String::new();
    push_blocks(&mut instance_block, &instance, &fixture.schema, false);
    let mut deletes_block = String::new();
    push_blocks(
        &mut deletes_block,
        &grouped(&fixture.deletes),
        &fixture.schema,
        false,
    );
    let mut inserts_block = String::new();
    push_blocks(
        &mut inserts_block,
        &grouped(&fixture.inserts),
        &fixture.schema,
        false,
    );
    let mut verdict_block = String::new();
    push_verdict(&mut verdict_block, &verdict);

    format!(
        "{{\n\"case\":\"{name}\",\n\"kind\":\"judgment\",\n\
         \"provenance\":{{\"hand\":\"{name}\"}},\n\
         \"theory\":{{\"relations\":{relations_block},\n\
         \"ground_axioms\":{axioms_block},\n\
         \"statements\":{statements_block}}},\n\
         \"instance\":{instance_block},\n\
         \"delta\":{{\"deletes\":{deletes_block},\n\"inserts\":{inserts_block}}},\n\
         \"verdict\":{verdict_block}\n}}\n",
        name = fixture.name
    )
}

/// The whole judgment corpus, deterministically: `(file name,
/// document)` pairs in roster order.
///
/// # Panics
///
/// On an engine-vs-naive disagreement (a trophy — see
/// [`render_fixture`]).
#[must_use]
pub fn generate_judgment_corpus() -> Vec<(String, String)> {
    fixtures()
        .iter()
        .map(|fixture| (format!("{}.json", fixture.name), render_fixture(fixture)))
        .collect()
}

/// One checked-in judgment case, fresh from its named fixture — the
/// replay half (`conformance::replay_checked_in_corpus` dispatches
/// `judgment-*.json` here).
///
/// # Panics
///
/// On an unknown fixture name (a stale corpus) or a trophy.
#[must_use]
pub fn replay_judgment_case(name: &str) -> String {
    let fixture = fixtures()
        .into_iter()
        .find(|fixture| fixture.name == name)
        .unwrap_or_else(|| panic!("unknown judgment fixture {name}: stale corpus"));
    render_fixture(&fixture)
}

#[cfg(test)]
mod tests {
    use bumbledb::{Direction, StatementId};

    use super::*;

    /// The serialized violation set is a WHOLE ordered list
    /// (`lean/Main.lean: RVerdict` derives `BEq` — order-sensitive), and
    /// `lane_verdict` owes it two invariants the corpus now also
    /// exercises: ascending statement order survives mixed citation
    /// kinds, and a containment cited in both directions collapses to
    /// ONE id (the `Direction` refinement sits below the Lean
    /// altitude). The dedup is adjacency-dependent, so this pin catches
    /// a reorder-before-dedup regression directly.
    #[test]
    fn lane_verdict_orders_and_dedups_the_citation_list() {
        let both_directions = Verdict::Aborted(vec![
            Violation::Containment {
                statement: StatementId(1),
                direction: Direction::SourceUnsatisfied,
            },
            Violation::Containment {
                statement: StatementId(1),
                direction: Direction::TargetRequired,
            },
        ]);
        match lane_verdict("pin-both-directions", &both_directions) {
            JVerdict::Reject {
                key_phase,
                violations,
            } => {
                assert!(!key_phase, "containment is the statement phase");
                assert_eq!(violations, vec![1], "both directions are one citation");
            }
            JVerdict::Accept => panic!("an aborted verdict never reads accept"),
        }

        let mixed = Verdict::Aborted(vec![
            Violation::Containment {
                statement: StatementId(1),
                direction: Direction::SourceUnsatisfied,
            },
            Violation::Cardinality {
                statement: StatementId(2),
            },
        ]);
        match lane_verdict("pin-mixed-kinds", &mixed) {
            JVerdict::Reject {
                key_phase,
                violations,
            } => {
                assert!(!key_phase);
                assert_eq!(violations, vec![1, 2], "ascending across citation kinds");
            }
            JVerdict::Accept => panic!("an aborted verdict never reads accept"),
        }

        let multi_key = Verdict::Aborted(vec![
            Violation::Functionality {
                statement: StatementId(0),
            },
            Violation::Functionality {
                statement: StatementId(1),
            },
        ]);
        match lane_verdict("pin-multi-key", &multi_key) {
            JVerdict::Reject {
                key_phase,
                violations,
            } => {
                assert!(key_phase, "an all-functionality set is the key phase");
                assert_eq!(violations, vec![0, 1], "the complete ascending key set");
            }
            JVerdict::Accept => panic!("an aborted verdict never reads accept"),
        }
    }

    /// The both-directions fixture really cites its containment TWICE
    /// pre-dedup — one source-direction conviction for the inserted
    /// uncovered claim, one target-direction conviction for the claim
    /// the deleted slot uncovers — so the corpus case exercises the
    /// dedup path, not a single-citation lookalike.
    #[test]
    fn the_both_directions_fixture_cites_two_directions() {
        let fixture = fixtures()
            .into_iter()
            .find(|fixture| fixture.name == "judgment-containment-both-directions")
            .expect("the fixture is on the roster");
        let dir = ScratchDir::new("judgment-both-directions-pin");
        let db = Db::create(&dir.0, fixture.schema.clone()).expect("create the pin store");
        let base = Delta {
            deletes: vec![],
            inserts: fixture.base.clone(),
        };
        assert!(
            matches!(differential::engine_write(&db, &base), Verdict::Committed),
            "the base state is green"
        );
        let delta = Delta {
            deletes: fixture.deletes.clone(),
            inserts: fixture.inserts.clone(),
        };
        match differential::engine_write(&db, &delta) {
            Verdict::Aborted(violations) => {
                let directions: Vec<Direction> = violations
                    .iter()
                    .map(|violation| match violation {
                        Violation::Containment {
                            statement,
                            direction,
                        } => {
                            assert_eq!(statement.0, 1, "the one containment statement");
                            *direction
                        }
                        other => panic!("only containment citations expected, got {other:?}"),
                    })
                    .collect();
                assert_eq!(
                    directions.len(),
                    2,
                    "both directions cited before the dedup"
                );
                assert_ne!(
                    directions[0], directions[1],
                    "one citation per direction, not a doubled one"
                );
            }
            Verdict::Committed => panic!("the fixture must reject"),
        }
    }
}
