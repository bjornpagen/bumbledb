//! The recursion/closure lane — the roster extension's measurement: a third
//! corpus world whose EDGE SHAPES are the point — one deep chain (the depth
//! axis: one new tuple per round, the round-overhead price) and one wide tree
//! (the fanout axis: frontier width, few rounds) — driven through `Db::prepare`
//! (`AtomSource::Interior`, the reach pipeline, row-identical across engines
//! before a single timed sample — inline lives outside the stamped family
//! registry), the exact warm protocol,

use bumbledb::schema::ValidateDescriptor as _;
use std::path::Path;

use bumbledb::{
    Answers, Atom, Db, FieldId, FindTerm, InteriorId, NonEmpty, ParamId, Query, Rec, RecRule,
    RecStep, RelationId, Rule, Term, Value, VarId,
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

/// # Panics
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClosSizes {
    pub chain: u64,

    pub fanout: u64,

    pub depth: u32,
}

impl ClosSizes {
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

    #[must_use]
    pub fn tree_nodes(&self) -> u64 {
        (self.fanout.pow(self.depth + 1) - 1) / (self.fanout - 1)
    }

    #[must_use]
    pub fn tree_base(&self) -> u64 {
        self.chain + 1
    }

    #[must_use]
    pub fn nodes(&self) -> u64 {
        self.tree_base() + self.tree_nodes()
    }

    #[must_use]
    pub fn edges(&self) -> u64 {
        self.chain + self.tree_nodes() - 1
    }
}

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

pub fn relation_rows(sizes: ClosSizes, rel: RelationId) -> Box<dyn Iterator<Item = Vec<Value>>> {
    match rel {
        ids::NODE => Box::new((0..sizes.nodes()).map(|i| vec![Value::U64(i)])),
        ids::EDGE => Box::new((0..sizes.edges()).map(move |i| edge_row(&sizes, i))),
        _ => unreachable!("two closure relations"),
    }
}

#[must_use]
pub fn closure_query() -> Query {
    use bumbledb::ir::{AtomSource, HeadTerm};
    let edge = |src: Term, dst: Term| Atom {
        source: AtomSource::Edb(ids::EDGE),
        bindings: vec![(ids::edge::SRC, src), (ids::edge::DST, dst)],
    };
    Query {
        interiors: vec![],
        rec: Some(Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0)],
                atoms: vec![edge(Term::Param(ParamId(0)), Term::Var(VarId(0)))],
                conditions: vec![],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(1)],
                self_bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
                atoms: vec![edge(Term::Var(VarId(0)), Term::Var(VarId(1)))],
                conditions: vec![],
            }),
        }),
        head: vec![HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: AtomSource::Interior(InteriorId(0)),
                bindings: vec![(bumbledb::FieldId(0), Term::Var(VarId(0)))],
            }],
            negated: vec![],
            conditions: vec![],
        }],
    }
}

pub const CLOSURE_SQL: &str = "WITH RECURSIVE reach(n) AS (SELECT \"dst\" FROM \"Edge\" WHERE \"src\" = ?1 UNION SELECT e.\"dst\" FROM \"Edge\" AS e, reach AS r WHERE e.\"src\" = r.n) SELECT n FROM reach";

pub struct ClosureFamily {
    pub name: &'static str,
    pub kind: Kind,
    pub query: fn() -> Query,
    pub params: fn(&GenConfig) -> Vec<Draw>,
    pub sql: &'static str,
    pub param_policy: &'static str,
}

fn depth_params(cfg: &GenConfig) -> Vec<Draw> {
    let sizes = ClosSizes::of(cfg.scale);
    vec![
        scalar_draw(vec![Value::U64(0)]),
        scalar_draw(vec![Value::U64(sizes.chain / 2)]),
        scalar_draw(vec![Value::U64(sizes.chain - 1)]),
        scalar_draw(vec![Value::U64(sizes.nodes() + 1_000_000)]),
    ]
}

fn fanout_params(cfg: &GenConfig) -> Vec<Draw> {
    let sizes = ClosSizes::of(cfg.scale);
    let base = sizes.tree_base();
    vec![
        scalar_draw(vec![Value::U64(base)]),
        scalar_draw(vec![Value::U64(base + 1)]),
        scalar_draw(vec![Value::U64(sizes.nodes() - 1)]),
        scalar_draw(vec![Value::U64(sizes.nodes() + 1_000_000)]),
    ]
}

#[must_use]
pub fn all() -> &'static [ClosureFamily] {
    &[
        ClosureFamily {
            name: "closure_depth",
            kind: Kind::Report,
            query: closure_query,
            params: depth_params,
            sql: CLOSURE_SQL,
            param_policy: "The chain head (chain-length rounds), the midpoint, the tail, + 1 miss.",
        },
        ClosureFamily {
            name: "closure_fanout",
            kind: Kind::Report,
            query: closure_query,
            params: fanout_params,
            sql: CLOSURE_SQL,
            param_policy: "The tree root (fanout^depth frontier), a depth-1 subtree, a leaf, + 1 miss.",
        },
    ]
}

#[must_use]
pub fn ddl() -> Vec<String> {
    sqlmap::schema_ddl(schema())
}

/// mirror file — targets before sources, the loader law.
/// # Errors
pub fn load_stores(
    dir: &Path,
    cfg: GenConfig,
    mode: crate::storemode::StoreMode,
) -> Result<(Db<Reachability>, rusqlite::Connection), String> {
    load_stores_sized(dir, ClosSizes::of(cfg.scale), mode)
}

/// # Errors
pub fn load_stores_sized(
    dir: &Path,
    sizes: ClosSizes,
    mode: crate::storemode::StoreMode,
) -> Result<(Db<Reachability>, rusqlite::Connection), String> {
    let db = mode.create(&dir.join("db"), Reachability)?;
    for rel in [ids::NODE, ids::EDGE] {
        db.write(|tx| {
            tx.insert_dyn(rel, relation_rows(sizes, rel))
                .map(bumbledb::MutationReport::changed)
        })
        .map_err(|e| format!("load: {e:?}"))?
        .unwrap();
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

fn translated() -> Translated {
    Translated {
        sql: CLOSURE_SQL.to_owned(),
        params: vec![ParamSlot::Whole(ParamId(0))],
    }
}

/// # Errors
pub fn verify_family(
    db: &Db<Reachability>,
    conn: &rusqlite::Connection,
    family: &ClosureFamily,
    draws: &[Draw],
) -> Result<(), String> {
    let query = (family.query)();
    let mut prepared = db
        .prepare(&query)
        .map_err(|e| format!("{}: prepare: {e:?}", family.name))?;
    let types: Vec<bumbledb::schema::ValueType> = prepared
        .signature()
        .columns
        .iter()
        .map(|column| *column.ty())
        .collect();
    let mut stmt = conn
        .prepare(family.sql)
        .map_err(|e| format!("{}: mirror prepare: {e}", family.name))?;
    let slots = translated().params;
    let mut buffer = Answers::new();
    for draw in draws {
        let args = param_args(draw);
        db.read(|snap| snap.execute(&mut prepared, &args, &mut buffer))
            .map_err(|e| format!("{}: execute: {e:?}", family.name))?;
        let ours = compare::from_answers(&buffer, &types);
        let theirs = compare::from_sqlite(&mut stmt, &slots, draw, &types)
            .map_err(|e| format!("{}: mirror: {e}", family.name))?;
        compare::multisets(ours, theirs)
            .map_err(|m| format!("{}: draw {draw:?}: {m}", family.name))?;
    }
    Ok(())
}

/// The timed closure lane: build the scratch world, verify every family × draw,
/// then measure both engines under the exact warm protocol — report-only rows
/// beside the read families.
/// # Errors
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

        let query = (family.query)();
        let mut prepared = db
            .prepare(&query)
            .map_err(|e| format!("{}: prepare: {e:?}", family.name))?;
        let mut rotation = Rotation::new(draws.clone());
        let mut buffer = Answers::new();
        let mut run_ours = |prepared: &mut bumbledb::PreparedQuery<Reachability>| {
            let args = param_args(rotation.next_set());
            db.read(|snap| snap.execute(prepared, &args, &mut buffer))
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

        let exec = None;
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
            exec,
            ghz: Some(merged.into()),
            p50_norm: ours.p50_norm,
        });
    }
    Ok(out)
}
