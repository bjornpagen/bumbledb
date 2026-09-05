#![cfg(not(feature = "alloc-counter"))]
#![allow(unsafe_code)] // GlobalAlloc is an unsafe trait; the census only counts and delegates.
#![allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
mod common;

use std::alloc::{GlobalAlloc, Layout, System};
use std::backtrace::Backtrace;
use std::cell::Cell;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use bumbledb::ir::{
    Atom, AtomSource, CmpOp, Comparison, FindTerm, FoldOp, HeadTerm, Interior, InteriorId, ParamId,
    Query, Rec, RecRule, RecStep, Rule, Term, Value, VarId,
};
use bumbledb::schema::{
    Bound, FieldDescriptor, FieldId, RelationDescriptor, RelationId, SchemaDescriptor, Side,
    StatementDescriptor, ValueType, Weight,
};
use bumbledb::{AllenMask, Answers, BindValue, ConditionTree, Db, NonEmpty, ProjectionRule};

static ALLOCS: AtomicU64 = AtomicU64::new(0);
static DEALLOCS: AtomicU64 = AtomicU64::new(0);
static ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static DEALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static ATTRIB: AtomicBool = AtomicBool::new(false);
static DROPPED: AtomicU64 = AtomicU64::new(0);

const EVENT_CAP: usize = 120_000;

struct Event {
    bytes: u64,
    realloc: bool,
    bt: Backtrace,
}

static EVENTS: Mutex<Vec<Event>> = Mutex::new(Vec::new());

thread_local! {
    static GUARD: Cell<bool> = const { Cell::new(false) };
}

fn record(bytes: u64, realloc: bool) {
    if !ATTRIB.load(Ordering::Relaxed) {
        return;
    }
    GUARD.with(|g| {
        if g.get() {
            return;
        }
        g.set(true);
        let bt = Backtrace::force_capture();
        let mut events = EVENTS.lock().expect("events lock");
        if events.len() < EVENT_CAP {
            events.push(Event { bytes, realloc, bt });
        } else {
            DROPPED.fetch_add(1, Ordering::Relaxed);
        }
        drop(events);
        g.set(false);
    });
}

struct CensusAllocator;

// SAFETY: delegates to `System`; the counters and event log are side
unsafe impl GlobalAlloc for CensusAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        ALLOC_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        record(layout.size() as u64, false);
        // SAFETY: forwarded contract.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOCS.fetch_add(1, Ordering::Relaxed);
        DEALLOC_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        // SAFETY: forwarded contract.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        ALLOC_BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
        DEALLOC_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        record(new_size as u64, true);
        // SAFETY: forwarded contract.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CensusAllocator = CensusAllocator;

#[derive(Clone, Copy, Default)]
struct Win {
    allocs: u64,
    deallocs: u64,
    alloc_bytes: u64,
    dealloc_bytes: u64,
}

fn reset() {
    ALLOCS.store(0, Ordering::Relaxed);
    DEALLOCS.store(0, Ordering::Relaxed);
    ALLOC_BYTES.store(0, Ordering::Relaxed);
    DEALLOC_BYTES.store(0, Ordering::Relaxed);
}

fn window() -> Win {
    Win {
        allocs: ALLOCS.load(Ordering::Relaxed),
        deallocs: DEALLOCS.load(Ordering::Relaxed),
        alloc_bytes: ALLOC_BYTES.load(Ordering::Relaxed),
        dealloc_bytes: DEALLOC_BYTES.load(Ordering::Relaxed),
    }
}

fn measured<R>(flow: &str, label: &str, attrib: bool, f: impl FnOnce() -> R) -> R {
    if attrib {
        EVENTS.lock().expect("events lock").clear();
        DROPPED.store(0, Ordering::Relaxed);
        ATTRIB.store(true, Ordering::Relaxed);
    }
    reset();
    let out = f();
    let w = window();
    ATTRIB.store(false, Ordering::Relaxed);
    println!(
        "CENSUS | {flow} | {label} | allocs={} deallocs={} alloc_bytes={} dealloc_bytes={}",
        w.allocs, w.deallocs, w.alloc_bytes, w.dealloc_bytes
    );
    if attrib {
        print_sites();
    }
    out
}

fn attribution_key(bt: &Backtrace) -> String {
    let text = format!("{bt}");
    let mut frames: Vec<(String, String)> = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("at ") {
            if let Some(sym) = current.take() {
                frames.push((sym, rest.to_owned()));
            }
        } else if let Some((_, sym)) = t.split_once(": ") {
            if let Some(sym_only) = current.take() {
                frames.push((sym_only, String::new()));
            }
            current = Some(sym.to_owned());
        }
    }
    if let Some(sym) = current.take() {
        frames.push((sym, String::new()));
    }
    let clean_sym = |s: &str| {
        let s = s.strip_suffix("::h").unwrap_or(s);
        match s.rfind("::h") {
            Some(i) if s[i + 3..].chars().all(|c| c.is_ascii_hexdigit()) => s[..i].to_owned(),
            _ => s.to_owned(),
        }
    };
    let clean_loc = |l: &str| {
        let l = match l.rfind("crates/") {
            Some(i) => &l[i..],
            None => l,
        };

        match l.rfind(':') {
            Some(i) if l[..i].contains(':') => l[..i].to_owned(),
            _ => l.to_owned(),
        }
    };

    let ours: Vec<String> = frames
        .iter()
        .filter(|(sym, loc)| {
            !loc.contains("rustlib")
                && (loc.starts_with("./") || loc.contains("crates/bumbledb"))
                && !loc.contains("alloc_census.rs")
                && !sym.contains("alloc_census")
        })
        .take(3)
        .map(|(sym, loc)| {
            if loc.is_empty() {
                clean_sym(sym)
            } else {
                format!("{} @ {}", clean_sym(sym), clean_loc(loc))
            }
        })
        .collect();
    if !ours.is_empty() {
        return ours.join(" <- ");
    }
    // Foreign allocation (heed/LMDB shim, std machinery): first two

    let foreign: Vec<String> = frames
        .iter()
        .filter(|(sym, _)| {
            !sym.contains("alloc::alloc")
                && !sym.contains("alloc_census")
                && !sym.contains("__rust")
                && !sym.contains("backtrace")
                && !sym.starts_with("alloc::raw_vec")
        })
        .take(2)
        .map(|(sym, loc)| {
            if loc.is_empty() {
                clean_sym(sym)
            } else {
                format!("{} @ {}", clean_sym(sym), clean_loc(loc))
            }
        })
        .collect();
    if foreign.is_empty() {
        "<unresolved>".to_owned()
    } else {
        foreign.join(" <- ")
    }
}

fn print_sites() {
    let events = std::mem::take(&mut *EVENTS.lock().expect("events lock"));
    let dropped = DROPPED.load(Ordering::Relaxed);
    let mut agg: HashMap<String, (u64, u64, u64)> = HashMap::new();
    for e in &events {
        let key = attribution_key(&e.bt);
        let entry = agg.entry(key).or_insert((0, 0, 0));
        entry.0 += 1;
        entry.1 += e.bytes;
        entry.2 += u64::from(e.realloc);
    }
    let mut rows: Vec<(String, (u64, u64, u64))> = agg.into_iter().collect();
    rows.sort_by_key(|a| std::cmp::Reverse((a.1.1, a.1.0)));
    for (key, (count, bytes, reallocs)) in rows.iter().take(28) {
        println!("  SITE {count:>6}x {bytes:>10}B (re={reallocs}) {key}");
    }
    if rows.len() > 28 {
        let (c, b): (u64, u64) = rows[28..]
            .iter()
            .fold((0, 0), |(c, b), r| (c + r.1.0, b + r.1.1));
        println!("  SITE {c:>6}x {b:>10}B (…{} more sites)", rows.len() - 28);
    }
    if dropped > 0 {
        println!("  SITE (event cap hit: {dropped} events untraced)");
    }
}

const POSTING: RelationId = RelationId(0);
const ACCOUNT: RelationId = RelationId(1);
const BUSY: RelationId = RelationId(2);
const ITEM: RelationId = RelationId(3);
const PROFILE: RelationId = RelationId(4);

fn u64_field(name: &str) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
    }
}

fn schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Posting".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                    },
                    u64_field("account"),
                    FieldDescriptor {
                        name: "amount".into(),
                        value_type: ValueType::I64,
                    },
                    FieldDescriptor {
                        name: "memo".into(),
                        value_type: ValueType::String,
                    },
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Account".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                    },
                    u64_field("holder"),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Busy".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                    },
                    u64_field("person"),
                    FieldDescriptor {
                        name: "slot".into(),
                        value_type: ValueType::Interval {
                            element: bumbledb::schema::IntervalElement::U64,
                        },
                    },
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Item".into(),
                fields: vec![u64_field("doc"), u64_field("pos"), u64_field("note")],
            },
            RelationDescriptor {
                extension: None,
                name: "Profile".into(),
                fields: vec![u64_field("account"), u64_field("score")],
            },
        ],
        statements: vec![
            StatementDescriptor::Containment {
                source: Side {
                    relation: POSTING,
                    projection: Box::new([FieldId(1)]),
                    selection: Box::new([]),
                },
                target: Side {
                    relation: ACCOUNT,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
            },
            StatementDescriptor::Capacity {
                target: Side {
                    relation: ACCOUNT,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
                weight: Weight::Unit,
                lo: 1,
                hi: Some(Bound::Lit(4096)),
                source: Side {
                    relation: ITEM,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
            },
            StatementDescriptor::Functionality {
                relation: PROFILE,
                projection: Box::new([FieldId(0)]),
            },
        ],
    }
}

fn wide_schema(n: u16) -> SchemaDescriptor {
    SchemaDescriptor {
        relations: (0..n)
            .map(|i| RelationDescriptor {
                extension: None,
                name: format!("R{i}").into(),
                fields: vec![u64_field("a"), u64_field("b"), u64_field("c")],
            })
            .collect(),
        statements: (0..n)
            .map(|i| StatementDescriptor::Functionality {
                relation: RelationId(i.into()),
                projection: Box::new([FieldId(0)]),
            })
            .collect(),
    }
}

const CHAIN: u64 = 64;

fn populate(db: &Db<SchemaDescriptor>) {
    db.write(common::work(), |tx| {
        for account in 0..20u64 {
            tx.insert_dyn(ACCOUNT, [&[Value::U64(account), Value::U64(account % 5)]])?;
        }

        for id in 100..100 + CHAIN {
            tx.insert_dyn(ACCOUNT, [&[Value::U64(id), Value::U64(id + 1)]])?;
        }
        for id in 0..500u64 {
            tx.insert_dyn(
                POSTING,
                [&[
                    Value::U64(id),
                    Value::U64(id % 20),
                    Value::I64((id.cast_signed() % 100) - 50),
                    Value::String(format!("memo-{}", id % 4).into()),
                ]],
            )?;
        }
        for id in 0..120u64 {
            let person = id % 6;
            let start = (id * 7) % 40;
            let end = if id % 5 == 4 {
                u64::MAX
            } else {
                start + 1 + id % 9
            };
            tx.insert_dyn(
                BUSY,
                [&[
                    Value::U64(id),
                    Value::U64(person),
                    Value::IntervalU64(
                        bumbledb::Interval::<u64>::new(start, end).expect("nonempty interval"),
                    ),
                ]],
            )?;
        }

        for doc in (0..20u64).chain(100..100 + CHAIN) {
            for pos in 1..=8u64 {
                tx.insert_dyn(
                    ITEM,
                    [&[
                        Value::U64(doc),
                        Value::U64(pos),
                        Value::U64(doc * 10_000 + pos),
                    ]],
                )?;
            }
        }

        for account in 0..20u64 {
            tx.insert_dyn(PROFILE, [&[Value::U64(account), Value::U64(account * 3)]])?;
        }
        Ok(())
    })
    .expect("populate")
    .unwrap();
}

fn edb(rel: RelationId, bindings: Vec<(FieldId, Term)>) -> Atom {
    Atom {
        source: AtomSource::Edb(rel),
        bindings,
    }
}

fn chain_query(atoms: u16, conds: u16, rules: u16) -> Query {
    let rule = |seed: u64| Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(atoms))],
        atoms: (0..atoms)
            .map(|i| {
                edb(
                    ACCOUNT,
                    vec![
                        (FieldId(0), Term::Var(VarId(i))),
                        (FieldId(1), Term::Var(VarId(i + 1))),
                    ],
                )
            })
            .collect(),
        negated: vec![],
        conditions: (0..conds)
            .map(|j| {
                ConditionTree::Leaf(Comparison {
                    op: if j % 2 == 0 { CmpOp::Ge } else { CmpOp::Le },
                    lhs: Term::Var(VarId(j % (atoms + 1))),
                    rhs: Term::Literal(Value::U64(if j % 2 == 0 {
                        seed + u64::from(j)
                    } else {
                        1_000_000 + seed + u64::from(j)
                    })),
                })
            })
            .collect(),
    };
    Query {
        interiors: vec![],
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: (0..rules).map(|r| rule(u64::from(r))).collect(),
        rec: None,
    }
}

fn dnf_query(k: u16) -> Query {
    let pair = |j: u64| {
        ConditionTree::And(vec![
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Ge,
                lhs: Term::Var(VarId(1)),
                rhs: Term::Literal(Value::U64(j)),
            }),
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Le,
                lhs: Term::Var(VarId(1)),
                rhs: Term::Literal(Value::U64(1_000 + j)),
            }),
        ])
    };
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![edb(
            ACCOUNT,
            vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Var(VarId(1))),
            ],
        )],
        negated: vec![],
        conditions: vec![ConditionTree::Or(
            (0..k).map(|j| pair(u64::from(j))).collect(),
        )],
    })
}

fn join_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            edb(
                POSTING,
                vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            ),
            edb(
                ACCOUNT,
                vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(0))),
                ],
            ),
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(0)),
        })],
    })
}

fn aggregate_query() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
            },
            FindTerm::Count,
        ],
        atoms: vec![
            edb(
                POSTING,
                vec![
                    (FieldId(0), Term::Var(VarId(3))),
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            ),
            edb(
                ACCOUNT,
                vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(0))),
                ],
            ),
        ],
        negated: vec![],
        conditions: vec![],
    })
}

fn string_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(3))],
        atoms: vec![
            edb(
                POSTING,
                vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(3), Term::Var(VarId(3))),
                ],
            ),
            edb(
                ACCOUNT,
                vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(0))),
                ],
            ),
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ne,
            lhs: Term::Var(VarId(3)),
            rhs: Term::Literal(Value::String(Box::from("memo-0"))),
        })],
    })
}

fn pack_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Pack { over: VarId(1) }],
        atoms: vec![edb(
            BUSY,
            vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        )],
        negated: vec![],
        conditions: vec![],
    })
}

fn calendar_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            edb(
                BUSY,
                vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Param(ParamId(0))),
                    (FieldId(2), Term::Var(VarId(2))),
                ],
            ),
            edb(
                BUSY,
                vec![
                    (FieldId(0), Term::Var(VarId(1))),
                    (FieldId(1), Term::Param(ParamId(0))),
                    (FieldId(2), Term::Var(VarId(3))),
                ],
            ),
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Allen {
                mask: AllenMask::INTERSECTS,
            },
            lhs: Term::Var(VarId(2)),
            rhs: Term::Var(VarId(3)),
        })],
    })
}

fn marks_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![edb(
            ITEM,
            vec![
                (FieldId(0), Term::Param(ParamId(0))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        )],
        negated: vec![],
        conditions: vec![],
    })
}

fn recursive_query() -> Query {
    let account = |a: u16, h: u16| {
        edb(
            ACCOUNT,
            vec![
                (FieldId(0), Term::Var(VarId(a))),
                (FieldId(1), Term::Var(VarId(h))),
            ],
        )
    };
    let cap = ConditionTree::Leaf(Comparison {
        op: CmpOp::Le,
        lhs: Term::Var(VarId(0)),
        rhs: Term::Param(ParamId(0)),
    });
    Query {
        interiors: vec![],
        rec: Some(Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0), VarId(1)],
                atoms: vec![account(0, 1)],
                conditions: vec![cap.clone()],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(0), VarId(2)],
                self_bindings: vec![
                    (FieldId(0), Term::Var(VarId(1))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
                atoms: vec![account(0, 1)],
                conditions: vec![cap],
            }),
        }),
        head: vec![HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: AtomSource::Interior(InteriorId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        }],
    }
}

fn interiors_only_query() -> Query {
    let join = Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            edb(
                POSTING,
                vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            ),
            edb(
                ACCOUNT,
                vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(0))),
                ],
            ),
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(0)),
        })],
    };
    Query {
        interiors: vec![Interior {
            rules: vec![
                ProjectionRule {
                    finds: vec![VarId(0), VarId(1)],
                    atoms: join.atoms,
                    negated: join.negated,
                    conditions: join.conditions,
                }
                .to_rule(),
            ],
        }],
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![Atom {
                source: AtomSource::Interior(InteriorId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        }],
        rec: None,
    }
}

bumbledb::schema! {
    pub CensusLedger;
    relation CItem {
        id: u64 as CItemId,
        memo: str,
    }
}

fn flow_open() {
    let dir = common::TempDir::new("census-open");
    let db = measured("open", "Db::create(fixture schema, common::work())", true, || {
        Db::create(dir.path(), schema(), common::work())
            .expect("create")
            .expect("accepted")
    });
    drop(db);
    let db = measured("open", "Db::open(existing, common::work())", true, || {
        Db::open(dir.path(), schema(), common::work()).expect("open")
    });
    drop(db);

    for n in [4u16, 16, 64] {
        let wdir = common::TempDir::new(&format!("census-wide-{n}"));
        let db = measured(
            "open",
            &format!("Db::create(wide schema, {n} relations, common::work())"),
            false,
            || {
                Db::create(wdir.path(), wide_schema(n), common::work())
                    .expect("create wide")
                    .expect("accepted")
            },
        );
        drop(db);
    }
}

fn flow_prepare(db: &Db<SchemaDescriptor>) {
    for (label, q) in [
        ("chain a=1 c=0 r=1", chain_query(1, 0, 1)),
        ("chain a=2 c=1 r=1", chain_query(2, 1, 1)),
        ("chain a=4 c=2 r=1", chain_query(4, 2, 1)),
        ("chain a=8 c=4 r=1", chain_query(8, 4, 1)),
        ("chain a=2 c=1 r=2", chain_query(2, 1, 2)),
        ("chain a=2 c=1 r=4", chain_query(2, 1, 4)),
        ("chain a=2 c=1 r=8", chain_query(2, 1, 8)),
        ("chain a=2 c=8 r=1", chain_query(2, 8, 1)),
        ("dnf k=2", dnf_query(2)),
        ("dnf k=4", dnf_query(4)),
        ("dnf k=8", dnf_query(8)),
    ] {
        for _ in 0..2 {
            drop(db.prepare(&q, common::work()).expect("prepare"));
        }
        measured("prepare", label, false, || {
            drop(db.prepare(&q, common::work()).expect("prepare"));
        });
    }

    let q = chain_query(4, 2, 2);
    measured("prepare", "chain a=4 c=2 r=2 (attributed)", true, || {
        drop(db.prepare(&q, common::work()).expect("prepare"));
    });
    let p = recursive_query();
    for _ in 0..2 {
        drop(db.prepare(&p, common::work()).expect("prepare"));
    }
    measured("prepare", "recursive query (attributed)", true, || {
        drop(db.prepare(&p, common::work()).expect("prepare"));
    });
    let q = join_query();
    measured("prepare", "join (2 atoms, 1 param cond)", false, || {
        drop(db.prepare(&q, common::work()).expect("prepare"));
    });
}

fn commit_shape(db: &Db<SchemaDescriptor>, label: &str, next_id: &mut u64, k: u64, attrib: bool) {
    for round in 0..3 {
        let base = *next_id;
        *next_id += k;
        let body = |tx: &mut bumbledb::WriteTx<'_, SchemaDescriptor>| {
            for id in base..base + k {
                tx.insert_dyn(
                    POSTING,
                    [&[
                        Value::U64(id),
                        Value::U64(id % 20),
                        Value::I64((id.cast_signed() % 100) - 50),
                        Value::String(format!("memo-{}", id % 4).into()),
                    ]],
                )?;
            }
            Ok(())
        };
        if round == 2 {
            measured("commit", label, attrib, || db.write(common::work(), body).expect("commit")).unwrap();
        } else {
            db.write(common::work(), body).expect("commit").unwrap();
        }
    }
}

fn flow_commit(db: &Db<SchemaDescriptor>) {
    let mut next_id = 10_000u64;
    commit_shape(db, "insert 1 posting", &mut next_id, 1, true);
    commit_shape(db, "insert 16 postings", &mut next_id, 16, false);
    commit_shape(db, "insert 512 postings", &mut next_id, 512, true);

    for round in 0..3u64 {
        let attrib = round == 2;
        let run = |label: &str,
                   f: &dyn Fn(
            &mut bumbledb::WriteTx<'_, SchemaDescriptor>,
        ) -> Result<(), bumbledb::Error>,
                   attrib: bool| {
            if attrib {
                measured(
                    "commit",
                    label,
                    label.starts_with("windowed append"),
                    || {
                        db.write(common::work(), |tx| f(tx)).expect("windowed commit").unwrap();
                    },
                );
            } else {
                db.write(common::work(), |tx| f(tx)).expect("windowed commit").unwrap();
            }
        };
        let append = move |tx: &mut bumbledb::WriteTx<'_, SchemaDescriptor>| {
            for doc in 0..5u64 {
                tx.insert_dyn(ITEM, [&[Value::U64(doc), Value::U64(9), Value::U64(round)]])?;
                let head = [Value::U64(doc), Value::U64(1), Value::U64(doc * 10_000 + 1)];
                tx.delete_dyn(ITEM, [&head])?;
                tx.insert_dyn(ITEM, [&head])?;
            }
            Ok(())
        };
        let restore = move |tx: &mut bumbledb::WriteTx<'_, SchemaDescriptor>| {
            for doc in 0..5u64 {
                tx.delete_dyn(ITEM, [&[Value::U64(doc), Value::U64(9), Value::U64(round)]])?;
            }
            Ok(())
        };
        run("windowed append+churn (5 parents)", &append, attrib);
        run("windowed restore", &restore, attrib);
    }

    for round in 0..3u64 {
        let body = move |tx: &mut bumbledb::WriteTx<'_, SchemaDescriptor>| {
            for account in 0..8u64 {
                tx.delete_dyn(
                    PROFILE,
                    [&[Value::U64(account), Value::U64(account * 3 + round)]],
                )?;
                tx.insert_dyn(
                    PROFILE,
                    [&[Value::U64(account), Value::U64(account * 3 + round + 1)]],
                )?;
            }
            Ok(())
        };

        let seeded = move |tx: &mut bumbledb::WriteTx<'_, SchemaDescriptor>| {
            if round == 0 {
                for account in 0..8u64 {
                    tx.delete_dyn(PROFILE, [&[Value::U64(account), Value::U64(account * 3)]])?;
                    tx.insert_dyn(
                        PROFILE,
                        [&[Value::U64(account), Value::U64(account * 3 + 1)]],
                    )?;
                }
                Ok(())
            } else {
                body(tx)
            }
        };
        if round == 2 {
            measured("commit", "determinant overwrite (8 tuples)", true, || {
                db.write(common::work(), seeded).expect("fd overwrite").unwrap();
            });
        } else {
            db.write(common::work(), seeded).expect("fd overwrite").unwrap();
        }
    }
}

fn cold_and_warm(
    db: &Db<SchemaDescriptor>,
    label: &str,
    q: &Query,
    params: &[BindValue<'_>],
    attrib_cold: bool,
) {
    db.read(common::work(), |snap| {
        let mut prepared = db.prepare(q, common::work())?;
        let mut out = Answers::new();
        measured(
            "execute",
            &format!("{label} COLD (first execution)"),
            attrib_cold,
            || {
                snap.execute(&mut prepared, params, &mut out).expect(label);
            },
        );
        for _ in 0..3 {
            snap.execute(&mut prepared, params, &mut out)?;
        }
        measured("execute", &format!("{label} WARM"), false, || {
            snap.execute(&mut prepared, params, &mut out).expect(label);
        });
        Ok(())
    })
    .expect("read");
}

fn flow_execute(db: &Db<SchemaDescriptor>) {
    cold_and_warm(db, "join", &join_query(), &[BindValue::I64(0)], true);
    cold_and_warm(
        db,
        "interiors-only",
        &interiors_only_query(),
        &[BindValue::I64(0)],
        false,
    );
    cold_and_warm(db, "aggregate sum/count", &aggregate_query(), &[], false);
    cold_and_warm(db, "string/Ne", &string_query(), &[], false);
    cold_and_warm(db, "pack", &pack_query(), &[], true);
    cold_and_warm(
        db,
        "calendar/allen",
        &calendar_query(),
        &[BindValue::U64(2)],
        true,
    );
    cold_and_warm(
        db,
        "windowed/marks",
        &marks_query(),
        &[BindValue::U64(3)],
        false,
    );

    let query = recursive_query();
    for cap in [110u64, 120, 140, 164] {
        db.read(common::work(), |snap| {
            let mut prepared = db.prepare(&query, common::work())?;
            let mut out = Answers::new();
            let rounds = cap.saturating_sub(100).max(1);
            measured(
                "execute",
                &format!("recursive COLD cap={cap} (~{rounds} rounds)"),
                cap == 164,
                || {
                    snap.execute(&mut prepared, &[BindValue::U64(cap)], &mut out)
                        .expect("recursive");
                },
            );
            for _ in 0..3 {
                snap.execute(&mut prepared, &[BindValue::U64(cap)], &mut out)?;
            }
            measured(
                "execute",
                &format!("recursive WARM cap={cap}"),
                false,
                || {
                    snap.execute(&mut prepared, &[BindValue::U64(cap)], &mut out)
                        .expect("recursive");
                },
            );
            Ok(())
        })
        .expect("read");
    }

    // measure the first execution after the commit (image rebuild).
    let q = join_query();
    let mut prepared = db.prepare(&q, common::work()).expect("prepare");
    let mut out = Answers::new();
    db.read(common::work(), |snap| {
        for _ in 0..4 {
            snap.execute(&mut prepared, &[BindValue::I64(0)], &mut out)?;
        }
        Ok(())
    })
    .expect("warm");
    db.write(common::work(), |tx| {
        tx.insert_dyn(
            POSTING,
            [&[
                Value::U64(99_000),
                Value::U64(3),
                Value::I64(7),
                Value::String("memo-1".into()),
            ]],
        )?;
        Ok(())
    })
    .expect("commit")
    .unwrap();
    db.read(common::work(), |snap| {
        measured(
            "execute",
            "join REBUILD (first execution post-commit)",
            true,
            || {
                snap.execute(&mut prepared, &[BindValue::I64(0)], &mut out)
                    .expect("rebuild");
            },
        );
        measured("execute", "join post-rebuild WARM", false, || {
            snap.execute(&mut prepared, &[BindValue::I64(0)], &mut out)
                .expect("warm");
        });
        Ok(())
    })
    .expect("read");
}

fn flow_insert_and_scan() {
    let dir = common::TempDir::new("census-insert");
    let db = Db::create(dir.path(), schema(), common::work())
        .expect("create")
        .expect("accepted");
    let rows: Vec<Vec<Value>> = (0..10_000u64)
        .map(|i| vec![Value::U64(i % 97), Value::U64(i / 97 + 1), Value::U64(i)])
        .collect();
    measured("insert", "insert_dyn 10k Item rows", true, || {
        let n = db
            .write(common::work(), |tx| {
                tx.insert_dyn(ITEM, rows.clone())
                    .map(bumbledb::MutationReport::changed)
            })
            .expect("insert")
            .unwrap()
            .value;
        assert_eq!(n, 10_000);
    });

    db.read(common::work(), |snap| {
        measured(
            "scan",
            "ReadInstance::scan 10k rows (dyn export)",
            true,
            || {
                let mut n = 0usize;
                for row in snap.scan(ITEM).expect("scan") {
                    let row = row.expect("row");
                    n += row.len();
                }
                assert_eq!(n, 30_000);
            },
        );
        Ok(())
    })
    .expect("read");
    drop(db);

    let tdir = common::TempDir::new("census-insert-typed");
    let tdb = Db::create(tdir.path(), CensusLedger, common::work())
        .expect("create typed")
        .expect("accepted");
    let memos: Vec<String> = (0..64).map(|i| format!("memo-{i}")).collect();
    measured("insert", "typed insert 10k str facts", true, || {
        let n = tdb
            .write(common::work(), |tx| {
                let rows: Vec<_> = (0..10_000u64)
                    .map(|i| CItem {
                        id: CItemId(i),
                        memo: &memos[(i % 64) as usize],
                    })
                    .collect();
                Ok(tx.insert(&rows)?.changed())
            })
            .expect("typed insert")
            .unwrap()
            .value;
        assert_eq!(n, 10_000);
    });
    tdb.read(common::work(), |snap| {
        measured("scan", "scan_facts 10k typed str facts", true, || {
            let n = snap.scan_facts::<CItem>().expect("scan").count();
            assert_eq!(n, 10_000);
        });
        Ok(())
    })
    .expect("typed read");
}

#[test]
#[ignore = "the census harness — a measurement instrument, run explicitly"]
fn allocation_deep_census() {
    println!("== THE ALLOCATION DEEP CENSUS ==");
    flow_open();

    let dir = common::TempDir::new("census-main");
    let db = Db::create(dir.path(), schema(), common::work())
        .expect("create")
        .expect("accepted");
    measured("commit", "populate (fixture world, one tx)", false, || {
        populate(&db);
    });

    flow_prepare(&db);
    flow_commit(&db);
    flow_execute(&db);
    drop(db);

    flow_insert_and_scan();
}
