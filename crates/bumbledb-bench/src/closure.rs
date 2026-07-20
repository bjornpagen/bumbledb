//! The recursion/closure lane — the roster extension's measurement
//! infrastructure for the landed recursion vocabulary
//! (`docs/architecture/20-query-ir.md` § engine recursion,
//! `docs/architecture/40-execution.md` § the fixpoint driver): a third
//! corpus world whose EDGE SHAPES are the point — one deep chain (the
//! depth axis: one new tuple per round, the round-overhead price) and
//! one wide tree (the fanout axis: frontier width, few rounds) — driven
//! through `Db::prepare` (`AtomSource::Idb`, the delta-variant
//! plans, the finished-image slot) against `SQLite`'s recursive CTE.
//!
//! Discipline mirrors the primary suite: seeded corpus regenerated per
//! run (never stored), verify-before-time (every family × draw is
//! row-identical across engines before a single timed sample — inline
//! here, since the recursion surface is translator-inexpressible and so
//! lives outside the stamped family registry), the exact warm protocol,
//! and the alloc window on request (the fixpoint's per-round transient
//! images are exactly the allocation-sensitive machinery —
//! `tests/alloc_gate.rs` pins the steady state; the bench lane reports
//! the measured window). Families are `Kind::Report`: measurement, not
//! gate claims.

use bumbledb::schema::ValidateDescriptor as _;
use std::path::Path;

use bumbledb::{
    Answers, Atom, Db, FindTerm, ParamId, PredId, PredicateDef, Program, RelationId, Rule, Term,
    Value, VarId,
};

use crate::corpus_gen::{GenConfig, Scale};
use crate::families::{Draw, Kind, param_args, scalar_draw};
use crate::harness::{self, Modes, Protocol, Rotation};
use crate::translate::{ParamSlot, Translated};
use crate::{clockproxy, compare, report, sqlite_run, sqlmap};

#[cfg(test)]
mod tests;

bumbledb::schema! {
    pub Reachability;

    relation Node {
        id: u64 as ClosNodeId, fresh,
    }
    relation Edge {
        src: u64 as ClosNodeId,
        dst: u64 as ClosNodeId,
    }

    Edge(src) <= Node(id);
    Edge(dst) <= Node(id);
}

/// Relation and field ids by declaration order.
pub mod ids {
    use bumbledb::{FieldId, RelationId};

    pub const NODE: RelationId = RelationId(0);
    pub const EDGE: RelationId = RelationId(1);

    pub mod edge {
        use super::FieldId;
        pub const SRC: FieldId = FieldId(0);
        pub const DST: FieldId = FieldId(1);
    }
}

/// The validated closure schema, memoized for the mirror's DDL.
///
/// # Panics
///
/// Never in practice: the declaration passes the acceptance gate.
pub fn schema() -> &'static bumbledb::Schema {
    use bumbledb::Theory as _;
    static SCHEMA: std::sync::OnceLock<bumbledb::Schema> = std::sync::OnceLock::new();
    SCHEMA.get_or_init(|| {
        Reachability
            .descriptor()
            .validate()
            .expect("the closure schema is valid")
    })
}

/// The closure corpus shape: one chain component and one complete
/// `fanout`-ary tree component, disjoint — depth and fanout as data,
/// no RNG anywhere (the shapes ARE the measurement).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClosSizes {
    /// Chain edges: `0 → 1 → … → chain` (closure from node 0 runs
    /// `chain` fixpoint rounds of one new tuple each).
    pub chain: u64,
    /// The tree's branching factor.
    pub fanout: u64,
    /// The tree's depth (root = depth 0).
    pub depth: u32,
}

impl ClosSizes {
    /// Two size points: `Tiny` for the naive/differential slice, the
    /// standard shape for every timed scale (the closure world prices
    /// the driver, not the ledger's mass — its identity is the shape).
    #[must_use]
    pub fn of(scale: Scale) -> Self {
        match scale {
            Scale::Tiny => Self {
                chain: 64,
                fanout: 4,
                depth: 3,
            },
            Scale::S | Scale::M | Scale::L => Self {
                chain: 4_096,
                fanout: 8,
                depth: 4,
            },
        }
    }

    /// Tree node count: `(fanout^(depth+1) - 1) / (fanout - 1)`.
    #[must_use]
    pub fn tree_nodes(&self) -> u64 {
        (self.fanout.pow(self.depth + 1) - 1) / (self.fanout - 1)
    }

    /// The tree root's node id (the chain occupies `0..=chain`).
    #[must_use]
    pub fn tree_base(&self) -> u64 {
        self.chain + 1
    }

    /// Total node count.
    #[must_use]
    pub fn nodes(&self) -> u64 {
        self.tree_base() + self.tree_nodes()
    }

    /// Total edge count (`chain` chain edges + `tree_nodes - 1` tree
    /// edges).
    #[must_use]
    pub fn edges(&self) -> u64 {
        self.chain + self.tree_nodes() - 1
    }
}

/// One edge row by index: chain edges first (`i → i + 1`), then the
/// tree in heap layout (edge `t` connects `parent(t) → t` for
/// `t in 1..tree_nodes`, `parent(t) = (t - 1) / fanout`, both offset by
/// [`ClosSizes::tree_base`]).
#[must_use]
pub fn edge_row(sizes: &ClosSizes, i: u64) -> Vec<Value> {
    if i < sizes.chain {
        vec![Value::U64(i), Value::U64(i + 1)]
    } else {
        let t = i - sizes.chain + 1;
        let base = sizes.tree_base();
        vec![
            Value::U64(base + (t - 1) / sizes.fanout),
            Value::U64(base + t),
        ]
    }
}

/// One relation's full row stream — pure function of the sizes.
#[must_use]
pub fn relation_rows(sizes: ClosSizes, rel: RelationId) -> Box<dyn Iterator<Item = Vec<Value>>> {
    match rel {
        ids::NODE => Box::new((0..sizes.nodes()).map(|i| vec![Value::U64(i)])),
        ids::EDGE => Box::new((0..sizes.edges()).map(move |i| edge_row(&sizes, i))),
        _ => unreachable!("two closure relations"),
    }
}

/// The transitive-closure program from a param anchor:
/// `Reach(x) | Edge(src = ?0, dst = x);
///  Reach(y) | Reach(x), Edge(src = x, dst = y);
///  Q(x) | Reach(x)` — the recursive interior under a non-recursive
/// output reading the FINISHED closure (the finished-image slot), the
/// exact shape the alloc gate pins.
#[must_use]
pub fn closure_program() -> Program {
    use bumbledb::ir::{AtomSource, HeadTerm};
    let edge = |src: Term, dst: Term| Atom {
        source: AtomSource::Edb(ids::EDGE),
        bindings: vec![(ids::edge::SRC, src), (ids::edge::DST, dst)],
    };
    Program {
        predicates: vec![
            PredicateDef {
                head: vec![HeadTerm::Var],
                rules: vec![
                    Rule {
                        finds: vec![FindTerm::Var(VarId(0))],
                        atoms: vec![edge(Term::Param(ParamId(0)), Term::Var(VarId(0)))],
                        negated: vec![],
                        conditions: vec![],
                    },
                    Rule {
                        finds: vec![FindTerm::Var(VarId(1))],
                        atoms: vec![
                            Atom {
                                source: AtomSource::Idb(PredId(0)),
                                bindings: vec![(bumbledb::FieldId(0), Term::Var(VarId(0)))],
                            },
                            edge(Term::Var(VarId(0)), Term::Var(VarId(1))),
                        ],
                        negated: vec![],
                        conditions: vec![],
                    },
                ],
            },
            PredicateDef {
                head: vec![HeadTerm::Var],
                rules: vec![Rule {
                    finds: vec![FindTerm::Var(VarId(0))],
                    atoms: vec![Atom {
                        source: AtomSource::Idb(PredId(0)),
                        bindings: vec![(bumbledb::FieldId(0), Term::Var(VarId(0)))],
                    }],
                    negated: vec![],
                    conditions: vec![],
                }],
            },
        ],
        output: PredId(1),
    }
}

/// The recursive CTE the mirror runs — `UNION` (not `UNION ALL`) is the
/// set-semantics twin of the fixpoint's seen-set.
pub const CLOSURE_SQL: &str = "WITH RECURSIVE reach(n) AS (SELECT \"dst\" FROM \"Edge\" WHERE \"src\" = ?1 UNION SELECT e.\"dst\" FROM \"Edge\" AS e, reach AS r WHERE e.\"src\" = r.n) SELECT n FROM reach";

/// One closure family: a program (not a query — the recursion surface),
/// its seeded anchors, and the hand-written recursive-CTE mirror (the
/// translator cannot express `Idb` atoms; the hand SQL is the
/// `free_busy` precedent, verified row-identical before any timing).
pub struct ClosureFamily {
    pub name: &'static str,
    pub kind: Kind,
    pub program: fn() -> Program,
    pub params: fn(&GenConfig) -> Vec<Draw>,
    pub sql: &'static str,
    pub param_policy: &'static str,
}

fn depth_params(cfg: &GenConfig) -> Vec<Draw> {
    let sizes = ClosSizes::of(cfg.scale);
    vec![
        // The chain head: `chain` rounds of one new tuple — the pure
        // depth shape (per-round overhead dominates).
        scalar_draw(vec![Value::U64(0)]),
        // The midpoint: half the rounds.
        scalar_draw(vec![Value::U64(sizes.chain / 2)]),
        // The chain tail: the one-round closure.
        scalar_draw(vec![Value::U64(sizes.chain - 1)]),
        // The miss: no edges, the empty fixpoint.
        scalar_draw(vec![Value::U64(sizes.nodes() + 1_000_000)]),
    ]
}

fn fanout_params(cfg: &GenConfig) -> Vec<Draw> {
    let sizes = ClosSizes::of(cfg.scale);
    let base = sizes.tree_base();
    vec![
        // The root: `depth` rounds of exponentially widening frontiers
        // — the pure fanout shape (per-tuple cost dominates).
        scalar_draw(vec![Value::U64(base)]),
        // A depth-1 subtree root: one fanout-narrower closure.
        scalar_draw(vec![Value::U64(base + 1)]),
        // A leaf: the empty closure.
        scalar_draw(vec![Value::U64(sizes.nodes() - 1)]),
        // The miss.
        scalar_draw(vec![Value::U64(sizes.nodes() + 1_000_000)]),
    ]
}

/// The closure registry: two families, one program, two corpus shapes
/// selected by anchor — depth against fanout on the same driver.
#[must_use]
pub fn all() -> &'static [ClosureFamily] {
    &[
        ClosureFamily {
            name: "closure_depth",
            kind: Kind::Report,
            program: closure_program,
            params: depth_params,
            sql: CLOSURE_SQL,
            param_policy: "The chain head (chain-length rounds), the midpoint, the tail, + 1 miss.",
        },
        ClosureFamily {
            name: "closure_fanout",
            kind: Kind::Report,
            program: closure_program,
            params: fanout_params,
            sql: CLOSURE_SQL,
            param_policy: "The tree root (fanout^depth frontier), a depth-1 subtree, a leaf, + 1 miss.",
        },
    ]
}

/// The mirror's DDL: mapped tables plus the statement-derived indexes
/// (the two `Edge` containments give the honest opponent its `src` and
/// `dst` indexes).
#[must_use]
pub fn ddl() -> Vec<String> {
    sqlmap::schema_ddl(schema())
}

/// Loads the closure corpus into a fresh engine store and a `SQLite`
/// mirror file — targets before sources, the loader law.
///
/// # Errors
///
/// Engine and `SQLite` errors, stringified.
pub fn load_stores(
    dir: &Path,
    cfg: GenConfig,
    mode: crate::storemode::StoreMode,
) -> Result<(Db<Reachability>, rusqlite::Connection), String> {
    load_stores_sized(dir, ClosSizes::of(cfg.scale), mode)
}

/// [`load_stores`] with the sizes given directly — one loader, sized.
/// This lane's identity stays [`ClosSizes::of`]; the curves lane
/// (`crate::lanes::curves`) passes its lane-local `curve_sizes(scale)`
/// ladder through here without touching that identity.
///
/// # Errors
///
/// Engine and `SQLite` errors, stringified.
pub fn load_stores_sized(
    dir: &Path,
    sizes: ClosSizes,
    mode: crate::storemode::StoreMode,
) -> Result<(Db<Reachability>, rusqlite::Connection), String> {
    let db = mode.create(&dir.join("db"), Reachability)?;
    for rel in [ids::NODE, ids::EDGE] {
        db.bulk_load_dyn(rel, relation_rows(sizes, rel))
            .map_err(|e| format!("load: {e:?}"))?;
    }
    let conn = rusqlite::Connection::open(dir.join("oracle.sqlite"))
        .map_err(|e| format!("oracle: {e}"))?;
    crate::corpus::configure_sqlite(&conn).map_err(|e| format!("configure: {e}"))?;
    for statement in ddl() {
        conn.execute(&statement, [])
            .map_err(|e| format!("ddl: {e}"))?;
    }
    for rel in [ids::NODE, ids::EDGE] {
        crate::corpus::insert_rows(&conn, schema().relation(rel), relation_rows(sizes, rel))
            .map_err(|e| format!("insert: {e}"))?;
    }
    conn.execute_batch("ANALYZE")
        .map_err(|e| format!("analyze: {e}"))?;
    Ok((db, conn))
}

/// The one param slot of [`CLOSURE_SQL`].
fn translated() -> Translated {
    Translated {
        sql: CLOSURE_SQL.to_owned(),
        params: vec![ParamSlot::Whole(ParamId(0))],
    }
}

/// Verify-before-time, inline: every draw row-identical across engines
/// (the recursion surface lives outside the stamped registry, so the
/// law is enforced at the lane's own gate).
///
/// # Errors
///
/// The first mismatch, rendered — or either engine's error.
pub fn verify_family(
    db: &Db<Reachability>,
    conn: &rusqlite::Connection,
    family: &ClosureFamily,
    draws: &[Draw],
) -> Result<(), String> {
    let program = (family.program)();
    let mut prepared = db
        .prepare(&program)
        .map_err(|e| format!("{}: prepare: {e:?}", family.name))?;
    let types: Vec<bumbledb::schema::ValueType> = prepared
        .predicate()
        .columns
        .iter()
        .map(|column| column.ty.clone())
        .collect();
    let mut stmt = conn
        .prepare(family.sql)
        .map_err(|e| format!("{}: mirror prepare: {e}", family.name))?;
    let slots = translated().params;
    let mut buffer = Answers::new();
    for draw in draws {
        let args = param_args(draw);
        db.read(|snap| snap.execute_args(&mut prepared, &args, &mut buffer))
            .map_err(|e| format!("{}: execute: {e:?}", family.name))?;
        let ours = compare::from_answers(&buffer, &types);
        let theirs = compare::from_sqlite(&mut stmt, &slots, draw, &types)
            .map_err(|e| format!("{}: mirror: {e}", family.name))?;
        compare::multisets(ours, theirs)
            .map_err(|m| format!("{}: draw {draw:?}: {m}", family.name))?;
    }
    Ok(())
}

/// The timed closure lane: build the scratch world, verify every
/// family × draw, then measure both engines under the exact warm
/// protocol — report-only rows beside the read families.
///
/// # Errors
///
/// Refusals (RAM-backed scratch), verify mismatches, and engine errors
/// — each message names the family.
pub fn bench_families(
    cfg: GenConfig,
    scratch: &Path,
    selected: &dyn Fn(&str) -> bool,
    proto: Protocol,
    alloc: bool,
    proxy_per_rep: bool,
    mode: crate::storemode::StoreMode,
) -> Result<Vec<report::ReadFamilyReport>, String> {
    if !all().iter().any(|family| selected(family.name)) {
        return Ok(Vec::new());
    }
    // The device-honesty rule is symmetric: this lane times reads
    // against its scratch world, so the scratch is checked exactly like
    // the write families'.
    crate::devhonesty::assert_disk_backed(scratch, "the timed closure families")
        .map_err(|refusal| refusal.to_string())?;
    let dir = scratch.join("closure");
    std::fs::create_dir_all(&dir).map_err(|e| format!("closure scratch: {e}"))?;
    eprintln!("bench: loading the closure corpus");
    let (db, conn) = load_stores(&dir, cfg, mode)?;

    let mut out = Vec::new();
    for family in all() {
        if !selected(family.name) {
            continue;
        }
        eprintln!("bench: closure family {}", family.name);
        let draws = (family.params)(&cfg);
        // Verify before time — row-identical or refuse to measure.
        verify_family(&db, &conn, family, &draws)?;

        let program = (family.program)();
        let mut prepared = db
            .prepare(&program)
            .map_err(|e| format!("{}: prepare: {e:?}", family.name))?;
        let mut rotation = Rotation::new(draws.clone());
        let mut buffer = Answers::new();
        let mut run_ours = |prepared: &mut bumbledb::PreparedQuery<'_, Reachability>| {
            let args = param_args(rotation.next_set());
            db.read(|snap| snap.execute_args(prepared, &args, &mut buffer))
                .map_err(|e| format!("execute: {e:?}"))?;
            Ok(buffer.len() as u64)
        };
        let modes = Modes {
            alloc_window: alloc,
            trace: false,
            proxy_per_rep,
        };
        let (ours, ghz_ours) = clockproxy::frequency_checked(|| {
            harness::measure_batched(proto, modes, 1, || run_ours(&mut prepared))
        })?;
        let batch = if ours.stats.p50 < harness::QUANTUM_FLOOR_NS {
            16
        } else {
            1
        };
        let (ours, ghz_ours) = if batch > 1 {
            eprintln!(
                "bench: {} p50 under the {} ns quantum floor — re-measuring at batch {batch}",
                family.name,
                harness::QUANTUM_FLOOR_NS
            );
            clockproxy::frequency_checked(|| {
                harness::measure_batched(proto, modes, batch, || run_ours(&mut prepared))
            })?
        } else {
            (ours, ghz_ours)
        };

        let mut mirror = sqlite_run::PreparedFamily::new(
            &conn,
            &translated(),
            vec![bumbledb::schema::ValueType::U64],
        )?;
        let mut cursor = 0usize;
        let sets = draws;
        let (theirs, ghz_theirs) = clockproxy::frequency_checked(|| {
            harness::measure_batched(proto, Modes::default(), batch, || {
                let index = cursor;
                cursor = (cursor + 1) % sets.len();
                sqlite_run::sample_args(&mut mirror, &sets[index])
            })
        })?;

        #[expect(
            clippy::cast_precision_loss,
            reason = "reporting accepts lossy integer-to-float conversion"
        )]
        let ratio_p50 = ours.stats.p50 as f64 / theirs.stats.p50.max(1) as f64;
        let alloc_report = ours.alloc.map(report::AllocReport::from);
        let merged = ghz_ours.merge(ghz_theirs);
        out.push(report::ReadFamilyReport {
            name: family.name.to_owned(),
            verdict: report::verdict(family.kind, ours.stats.p50, theirs.stats.p50),
            p99_within_budget: report::within_budget(ours.stats.p99),
            ours: ours.stats,
            theirs: theirs.stats,
            ratio_p50,
            alloc: alloc_report,
            exec: None, // the profile path is query-shaped; programs skip it
            ghz: Some(report::GhzReport {
                pre: merged.pre,
                post: merged.post,
                retried: merged.retried,
                contaminated: merged.contaminated(),
            }),
            p50_norm: ours.p50_norm,
        });
    }
    Ok(out)
}
