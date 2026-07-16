//! THE ALLOCATION DEEP CENSUS (perf/alloc-census): whole-flow allocation
//! accounting — counts, bytes, and call-path attribution — across the five
//! flow families (prepare, open, commit, execute, bulk/scan). The release
//! alloc gate (`tests/alloc_gate.rs`) proves steady-state zero on gated
//! scenarios; this harness measures EVERYTHING ELSE: the sanctioned cold
//! paths, the per-commit arena, the per-prepare pipeline, the fixpoint
//! driver's rounds — so each remaining site can be classified
//! JUSTIFIED / HOISTABLE / WASTEFUL.
//!
//! Run:
//!   `CARGO_PROFILE_RELEASE_DEBUG=2 cargo test --release --test alloc_census`
//!   `  -- --ignored --test-threads=1 --nocapture`
//!
//! The harness registers its own counting+tracing global allocator, so it
//! must be built WITHOUT the `alloc-counter` feature (which registers the
//! lib's). It is `#[ignore]`d: a measurement instrument, not a gate.

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
    AggOp, Atom, AtomSource, CmpOp, Comparison, FindTerm, HeadTerm, MaskTerm, ParamId, Query, Rule,
    Term, Value, VarId,
};
use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor, Side,
    StatementDescriptor, ValueType,
};
use bumbledb::{AllenMask, Answers, BindValue, ConditionTree, Db};

// =====================================================================
// The census allocator: counting always, backtrace capture when armed.
// =====================================================================

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
// effects with no aliasing (the event log is reentrancy-guarded).
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
        // The gate contract: one alloc event, both byte sides.
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

/// One measured window; `attrib` arms per-event backtrace capture and
/// prints the aggregated top sites afterward.
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

/// Renders one backtrace into an attribution key: the deepest ≤3
/// bumbledb-crate frames as `func @ file:line`, or a std/foreign fallback.
fn attribution_key(bt: &Backtrace) -> String {
    let text = format!("{bt}");
    let mut frames: Vec<(String, String)> = Vec::new(); // (symbol, loc)
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
        // Trim the trailing column.
        match l.rfind(':') {
            Some(i) if l[..i].contains(':') => l[..i].to_owned(),
            _ => l.to_owned(),
        }
    };
    // A repo frame: its resolved location is inside the workspace source
    // (a relative `./src/...` path or an explicit `crates/bumbledb...`),
    // never the rustlib sources — generic std instantiations like
    // `RawVec<bumbledb::...>` carry bumbledb in the SYMBOL but resolve to
    // rustlib, and they are plumbing, not sites.
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
    // frames past the raw allocation plumbing.
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
    let mut agg: HashMap<String, (u64, u64, u64)> = HashMap::new(); // count, bytes, reallocs
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

// =====================================================================
// The fixture: the gate's world + a determinant relation + a holder chain.
// =====================================================================

const POSTING: RelationId = RelationId(0);
const ACCOUNT: RelationId = RelationId(1);
const BUSY: RelationId = RelationId(2);
const ITEM: RelationId = RelationId(3);
const PROFILE: RelationId = RelationId(4);

fn u64_field(name: &str) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::None,
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
                        generation: Generation::Fresh,
                    },
                    u64_field("account"),
                    FieldDescriptor {
                        name: "amount".into(),
                        value_type: ValueType::I64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "memo".into(),
                        value_type: ValueType::String,
                        generation: Generation::None,
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
                        generation: Generation::Fresh,
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
                        generation: Generation::Fresh,
                    },
                    u64_field("person"),
                    FieldDescriptor {
                        name: "slot".into(),
                        value_type: ValueType::Interval {
                            element: bumbledb::schema::IntervalElement::U64,
                            width: None,
                        },
                        generation: Generation::None,
                    },
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Item".into(),
                fields: vec![u64_field("doc"), u64_field("pos"), u64_field("note")],
            },
            // The determinant relation: Profile(account) -> Profile.
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
            // The cardinality window: Account(id) <={1..4096} Item(doc).
            StatementDescriptor::Cardinality {
                source: Side {
                    relation: ITEM,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([]),
                },
                lo: 1,
                hi: Some(4096),
                target: Side {
                    relation: ACCOUNT,
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

/// A generated schema of `n` relations (3 u64 fields each, one FD each)
/// for the validate-scaling axis of the open flow.
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

/// The holder chain length (accounts 100..100+CHAIN with holder = id+1):
/// the fixpoint driver's round count rides this.
const CHAIN: u64 = 64;

fn populate(db: &Db<SchemaDescriptor>) {
    db.write(|tx| {
        for account in 0..20u64 {
            tx.insert_dyn(ACCOUNT, &[Value::U64(account), Value::U64(account % 5)])?;
        }
        // The holder chain for recursion-round scaling.
        for id in 100..100 + CHAIN {
            tx.insert_dyn(ACCOUNT, &[Value::U64(id), Value::U64(id + 1)])?;
        }
        for id in 0..500u64 {
            tx.insert_dyn(
                POSTING,
                &[
                    Value::U64(id),
                    Value::U64(id % 20),
                    Value::I64((id.cast_signed() % 100) - 50),
                    Value::String(format!("memo-{}", id % 4).into_bytes().into()),
                ],
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
                &[
                    Value::U64(id),
                    Value::U64(person),
                    Value::IntervalU64(
                        bumbledb::Interval::<u64>::new(start, end).expect("nonempty interval"),
                    ),
                ],
            )?;
        }
        // The cardinality floor: every account parents an Item chain.
        for doc in (0..20u64).chain(100..100 + CHAIN) {
            for pos in 1..=8u64 {
                tx.insert_dyn(
                    ITEM,
                    &[
                        Value::U64(doc),
                        Value::U64(pos),
                        Value::U64(doc * 10_000 + pos),
                    ],
                )?;
            }
        }
        // The determinant rows.
        for account in 0..20u64 {
            tx.insert_dyn(PROFILE, &[Value::U64(account), Value::U64(account * 3)])?;
        }
        Ok(())
    })
    .expect("populate");
}

// =====================================================================
// Query shapes.
// =====================================================================

fn edb(rel: RelationId, bindings: Vec<(FieldId, Term)>) -> Atom {
    Atom {
        source: AtomSource::Edb(rel),
        bindings,
    }
}

/// The prepare-scaling family: a chain of `atoms` Account self-joins with
/// `conds` satisfiable range conditions, duplicated across `rules` rules.
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
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: (0..rules).map(|r| rule(u64::from(r))).collect(),
    }
}

/// The DNF axis: one rule whose condition is an Or of `k` conjunctions —
/// normalization multiplies rules.
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

/// Q(holder, amount) :- Posting ⋈ Account, amount >= ?0 — the join shape.
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

/// Q(holder, Sum(amount), Count) — the aggregate shape.
fn aggregate_query() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(1)),
            },
            FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            },
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

/// Q(holder, memo) with a Ne string literal — the string/byte-heap shape.
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
            rhs: Term::Literal(Value::String(Box::from(&b"memo-0"[..]))),
        })],
    })
}

/// Q(person, Pack(slot)) — the coalescing fold.
fn pack_query() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Pack,
                over: Some(VarId(1)),
            },
        ],
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

/// Q(a, b) :- Busy(a, s1), Busy(b, s2), s1 INTERSECTS s2, person = ?0 both
/// sides — the calendar/Allen interval-pair shape.
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
                mask: MaskTerm::Literal(AllenMask::INTERSECTS),
            },
            lhs: Term::Var(VarId(2)),
            rhs: Term::Var(VarId(3)),
        })],
    })
}

/// Q(pos, note) :- Item(doc = ?0, pos, note) — the windowed (marks) shape.
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

/// The recursive closure program over the Account holder graph, edge set
/// capped by `?0` (the gate's recursive family verbatim).
fn recursive_program() -> bumbledb::Program {
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
    bumbledb::Program {
        predicates: vec![
            bumbledb::PredicateDef {
                head: vec![HeadTerm::Var, HeadTerm::Var],
                rules: vec![
                    Rule {
                        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
                        atoms: vec![account(0, 1)],
                        negated: vec![],
                        conditions: vec![cap.clone()],
                    },
                    Rule {
                        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
                        atoms: vec![
                            account(0, 1),
                            Atom {
                                source: AtomSource::Idb(bumbledb::PredId(0)),
                                bindings: vec![
                                    (FieldId(0), Term::Var(VarId(1))),
                                    (FieldId(1), Term::Var(VarId(2))),
                                ],
                            },
                        ],
                        negated: vec![],
                        conditions: vec![cap],
                    },
                ],
            },
            bumbledb::PredicateDef {
                head: vec![HeadTerm::Var],
                rules: vec![Rule {
                    finds: vec![FindTerm::Var(VarId(0))],
                    atoms: vec![Atom {
                        source: AtomSource::Idb(bumbledb::PredId(0)),
                        bindings: vec![
                            (FieldId(0), Term::Var(VarId(0))),
                            (FieldId(1), Term::Var(VarId(1))),
                        ],
                    }],
                    negated: vec![],
                    conditions: vec![],
                }],
            },
        ],
        output: bumbledb::PredId(1),
    }
}

// =====================================================================
// The typed world for the typed bulk/scan lanes.
// =====================================================================

bumbledb::schema! {
    pub CensusLedger;
    relation CItem {
        id: u64 as CItemId, fresh,
        memo: str,
    }
}

// =====================================================================
// Flows.
// =====================================================================

fn flow_open() {
    // create / open / ephemeral / ephemeral-reopen (the probe battery).
    let dir = common::TempDir::new("census-open");
    let db = measured("open", "Db::create(fixture schema)", true, || {
        Db::create(dir.path(), schema()).expect("create")
    });
    drop(db);
    let db = measured("open", "Db::open(existing)", true, || {
        Db::open(dir.path(), schema()).expect("open")
    });
    drop(db);

    let edir = common::TempDir::new("census-ephemeral");
    let db = measured("open", "Db::ephemeral(fresh)", true, || {
        Db::ephemeral(edir.path(), schema()).expect("ephemeral create")
    });
    drop(db);
    let db = measured(
        "open",
        "Db::ephemeral(reopen: probe battery)",
        false,
        || Db::ephemeral(edir.path(), schema()).expect("ephemeral reopen"),
    );
    drop(db);

    // Validate/open scaling with schema width.
    for n in [4u16, 16, 64] {
        let wdir = common::TempDir::new(&format!("census-wide-{n}"));
        let db = measured(
            "open",
            &format!("Db::create(wide schema, {n} relations)"),
            false,
            || Db::create(wdir.path(), wide_schema(n)).expect("create wide"),
        );
        drop(db);
    }
}

fn flow_prepare(db: &Db<SchemaDescriptor>) {
    // Scaling axes: atoms, conditions, rules — three prepares each, the
    // third measured (per-call steady cost, not first-call jitter).
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
            drop(db.prepare(&q).expect("prepare"));
        }
        measured("prepare", label, false, || {
            drop(db.prepare(&q).expect("prepare"));
        });
    }
    // Attributed representative shapes.
    let q = chain_query(4, 2, 2);
    measured("prepare", "chain a=4 c=2 r=2 (attributed)", true, || {
        drop(db.prepare(&q).expect("prepare"));
    });
    let p = recursive_program();
    for _ in 0..2 {
        drop(db.prepare(&p).expect("prepare"));
    }
    measured("prepare", "recursive program (attributed)", true, || {
        drop(db.prepare(&p).expect("prepare"));
    });
    let q = join_query();
    measured("prepare", "join (2 atoms, 1 param cond)", false, || {
        drop(db.prepare(&q).expect("prepare"));
    });
}

fn commit_shape(db: &Db<SchemaDescriptor>, label: &str, next_id: &mut u64, k: u64, attrib: bool) {
    // Warm twice, measure the third commit.
    for round in 0..3 {
        let base = *next_id;
        *next_id += k;
        let body = |tx: &mut bumbledb::WriteTx<'_, SchemaDescriptor>| {
            for id in base..base + k {
                tx.insert_dyn(
                    POSTING,
                    &[
                        Value::U64(id),
                        Value::U64(id % 20),
                        Value::I64((id.cast_signed() % 100) - 50),
                        Value::String(format!("memo-{}", id % 4).into_bytes().into()),
                    ],
                )?;
            }
            Ok(())
        };
        if round == 2 {
            measured("commit", label, attrib, || db.write(body).expect("commit"));
        } else {
            db.write(body).expect("commit");
        }
    }
}

fn flow_commit(db: &Db<SchemaDescriptor>) {
    let mut next_id = 10_000u64;
    commit_shape(db, "insert 1 posting", &mut next_id, 1, true);
    commit_shape(db, "insert 16 postings", &mut next_id, 16, false);
    commit_shape(db, "insert 512 postings", &mut next_id, 512, true);

    // The window-touching commit (marks machinery live): tail append +
    // net-nothing head delete/reinsert across 5 window parents, then the
    // restoring commit.
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
                        db.write(|tx| f(tx)).expect("windowed commit");
                    },
                );
            } else {
                db.write(|tx| f(tx)).expect("windowed commit");
            }
        };
        let append = move |tx: &mut bumbledb::WriteTx<'_, SchemaDescriptor>| {
            for doc in 0..5u64 {
                tx.insert_dyn(ITEM, &[Value::U64(doc), Value::U64(9), Value::U64(round)])?;
                let head = [Value::U64(doc), Value::U64(1), Value::U64(doc * 10_000 + 1)];
                tx.delete_dyn(ITEM, &head)?;
                tx.insert_dyn(ITEM, &head)?;
            }
            Ok(())
        };
        let restore = move |tx: &mut bumbledb::WriteTx<'_, SchemaDescriptor>| {
            for doc in 0..5u64 {
                tx.delete_dyn(ITEM, &[Value::U64(doc), Value::U64(9), Value::U64(round)])?;
            }
            Ok(())
        };
        run("windowed append+churn (5 parents)", &append, attrib);
        run("windowed restore", &restore, attrib);
    }

    // The determinant overwrite: delete+reinsert the same determinant
    // tuple with a new dependent — the recently-fixed clone path.
    for round in 0..3u64 {
        let body = move |tx: &mut bumbledb::WriteTx<'_, SchemaDescriptor>| {
            for account in 0..8u64 {
                tx.delete_dyn(
                    PROFILE,
                    &[Value::U64(account), Value::U64(account * 3 + round)],
                )?;
                tx.insert_dyn(
                    PROFILE,
                    &[Value::U64(account), Value::U64(account * 3 + round + 1)],
                )?;
            }
            Ok(())
        };
        // Keep the dependent moving: seed round 0 from the populate values.
        let seeded = move |tx: &mut bumbledb::WriteTx<'_, SchemaDescriptor>| {
            if round == 0 {
                for account in 0..8u64 {
                    tx.delete_dyn(PROFILE, &[Value::U64(account), Value::U64(account * 3)])?;
                    tx.insert_dyn(PROFILE, &[Value::U64(account), Value::U64(account * 3 + 1)])?;
                }
                Ok(())
            } else {
                body(tx)
            }
        };
        if round == 2 {
            measured("commit", "determinant overwrite (8 tuples)", true, || {
                db.write(seeded).expect("fd overwrite");
            });
        } else {
            db.write(seeded).expect("fd overwrite");
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
    db.read(|snap| {
        let mut prepared = db.prepare(q)?;
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

    // The fixpoint driver: cold executions at increasing caps — rounds
    // ride the holder chain, so allocation-per-round is the slope.
    let program = recursive_program();
    for cap in [110u64, 120, 140, 164] {
        db.read(|snap| {
            let mut prepared = db.prepare(&program)?;
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

    // The post-commit rebuild window: warm a prepared query, commit, then
    // measure the first execution after the commit (image rebuild).
    let q = join_query();
    let mut prepared = db.prepare(&q).expect("prepare");
    let mut out = Answers::new();
    db.read(|snap| {
        for _ in 0..4 {
            snap.execute(&mut prepared, &[BindValue::I64(0)], &mut out)?;
        }
        Ok(())
    })
    .expect("warm");
    db.write(|tx| {
        tx.insert_dyn(
            POSTING,
            &[
                Value::U64(99_000),
                Value::U64(3),
                Value::I64(7),
                Value::String(b"memo-1".to_vec().into()),
            ],
        )?;
        Ok(())
    })
    .expect("commit");
    db.read(|snap| {
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

fn flow_bulk_and_scan() {
    use bumbledb::Fresh as _;
    // The dynamic bulk lane: 10_000 Item facts (3 chunks) into a fresh db.
    let dir = common::TempDir::new("census-bulk");
    let db = Db::create(dir.path(), schema()).expect("create");
    let rows: Vec<Vec<Value>> = (0..10_000u64)
        .map(|i| vec![Value::U64(i % 97), Value::U64(i / 97 + 1), Value::U64(i)])
        .collect();
    measured(
        "bulk",
        "bulk_load_dyn 10k Item rows (3 chunks)",
        true,
        || {
            let n = db.bulk_load_dyn(ITEM, rows.clone()).expect("bulk");
            assert_eq!(n, 10_000);
        },
    );

    // The dynamic scan/export lane over the loaded relation.
    db.read(|snap| {
        measured("scan", "Snapshot::scan 10k rows (dyn export)", true, || {
            let mut n = 0usize;
            for row in snap.scan(ITEM).expect("scan") {
                let row = row.expect("row");
                n += row.len();
            }
            assert_eq!(n, 30_000);
        });
        Ok(())
    })
    .expect("read");
    drop(db);

    // The typed lanes: bulk_load of str-bearing facts + scan_facts.
    let tdir = common::TempDir::new("census-bulk-typed");
    let tdb = Db::create(tdir.path(), CensusLedger).expect("create typed");
    let memos: Vec<String> = (0..64).map(|i| format!("memo-{i}")).collect();
    measured(
        "bulk",
        "typed bulk_load 10k str facts (3 chunks)",
        true,
        || {
            let n = tdb
                .bulk_load((0..10_000u64).map(|i| CItem {
                    id: CItemId::from_fresh(i),
                    memo: &memos[(i % 64) as usize],
                }))
                .expect("typed bulk");
            assert_eq!(n, 10_000);
        },
    );
    tdb.read(|snap| {
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
    let db = Db::create(dir.path(), schema()).expect("create");
    measured("commit", "populate (fixture world, one tx)", false, || {
        populate(&db);
    });

    flow_prepare(&db);
    flow_commit(&db);
    flow_execute(&db);
    drop(db);

    flow_bulk_and_scan();
}
