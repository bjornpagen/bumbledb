//! The DRAM-tier displaced lanes — the roster extension's measurement
//! infrastructure for the memory regime the S-scale roster cannot see
//! (`docs/reference/apple-silicon-performance.md`: residency is a property of
//! phase *interleaving*, not footprint — `m2max.mem.residency-is-interleaving`;
//! 24 MB of interleaved foreign streaming degrades a nominally resident probe
//! structure +53% — query (the view memo — every execute after the first shows
//! scale (every family × draw row-identical across engines before a is O(rows)
//! per pass), the exact warm protocol shape with the lane's

use bumbledb::schema::ValidateDescriptor as _;
use std::path::Path;

use bumbledb::{
    Answers, Atom, AtomSource, Db, FindTerm, FoldOp, Query, RelationId, Rule, Term, Value, VarId,
};

use crate::corpus_gen::{GenConfig, Scale, mix};
use crate::families::{Draw, Kind, param_args, scalar_draw};
use crate::harness::{self, Modes, Protocol, Rotation};
use crate::translate::translate;
use crate::{clockproxy, compare, report, sqlite_run, sqlmap};

#[cfg(test)]
mod tests;

bumbledb::schema! {
    pub DisplacedWorld;

    relation Hub {
        id: u64 as HubId, fresh,
        tag: u64,
    }
    relation Spoke {
        id: u64 as SpokeId, fresh,
        hub: u64 as HubId,
        val: u64,
    }

    Spoke(hub) <= Hub(id);
}

pub mod ids {
    use bumbledb::{FieldId, RelationId};

    pub const HUB: RelationId = RelationId(0);
    pub const SPOKE: RelationId = RelationId(1);

    pub mod hub {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const TAG: FieldId = FieldId(1);
    }
    pub mod spoke {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const HUB: FieldId = FieldId(1);
        pub const VAL: FieldId = FieldId(2);
    }
}

/// # Panics
pub fn schema() -> &'static bumbledb::Schema {
    use bumbledb::Theory as _;
    static SCHEMA: std::sync::OnceLock<bumbledb::Schema> = std::sync::OnceLock::new();
    SCHEMA.get_or_init(|| {
        DisplacedWorld
            .descriptor()
            .validate()
            .expect("the displaced schema is valid")
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DispSizes {

    pub hubs: u64,

    pub spokes: u64,

    pub tags: u64,
}

impl DispSizes {

    #[must_use]
    pub fn of(scale: Scale) -> Self {
        match scale {
            Scale::Tiny => Self {
                hubs: 2_048,
                spokes: 1_024,
                tags: 16,
            },
            Scale::S | Scale::M | Scale::L => Self {
                hubs: 1 << 19,
                spokes: 1 << 20,
                tags: 1_024,
            },
        }
    }

    #[must_use]
    pub fn hub_image_bytes(&self) -> u64 {
        self.hubs * 2 * 8
    }

    #[must_use]
    pub fn spoke_image_bytes(&self) -> u64 {
        self.spokes * 3 * 8
    }
}

/// The doubling loop here uses `(distinct + 1)` — exact whenever any position
/// follows the last keys are unknown before the pass), then rehash-doubling per
/// ingested
/// # Panics
#[must_use]
pub fn forced_spoke_map_bytes(positions: u64, distinct: u64) -> u64 {
    let count = usize::try_from(positions).expect("64-bit usize");
    let landed = usize::try_from(distinct).expect("64-bit usize");
    let guess = (count / 8).max(16).min(count.max(1) * 2);
    let mut nbuckets = (guess * 5 / 16).max(1).next_power_of_two();
    while (landed + 1) * 5 > nbuckets * 16 {
        nbuckets *= 2;
    }
    let ctrl = nbuckets * 8;
    let buckets = nbuckets * (8 + 8) * 8;
    u64::try_from(ctrl + buckets).expect("fits u64")
}

/// `1 − e^-2` occupancy; seed-invariant for seeds < 2^20, where the
pub const FORCED_MAP_POSITIONS: u64 = 1 << 20;

pub const FORCED_MAP_DISTINCT: u64 = 453_241;

pub fn relation_rows(
    sizes: DispSizes,
    seed: u64,
    rel: RelationId,
) -> Box<dyn Iterator<Item = Vec<Value>>> {
    match rel {
        ids::HUB => Box::new((0..sizes.hubs).map(move |i| {
            vec![
                Value::U64(i),
                Value::U64(mix(seed, ids::HUB, i) % sizes.tags),
            ]
        })),
        ids::SPOKE => Box::new((0..sizes.spokes).map(move |i| {
            let m = mix(seed, ids::SPOKE, i);
            vec![
                Value::U64(i),
                Value::U64(m % sizes.hubs),
                Value::U64((m >> 32) % 997),
            ]
        })),
        _ => unreachable!("two displaced relations"),
    }
}

fn var(id: u16) -> Term {
    Term::Var(VarId(id))
}

#[must_use]
pub fn probe_query() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
            },
        ],
        atoms: vec![
            Atom {
                source: AtomSource::Edb(ids::SPOKE),
                bindings: vec![
                    (ids::spoke::ID, var(2)),
                    (ids::spoke::HUB, var(3)),
                    (ids::spoke::VAL, var(1)),
                ],
            },
            Atom {
                source: AtomSource::Edb(ids::HUB),
                bindings: vec![(ids::hub::ID, var(3)), (ids::hub::TAG, var(0))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

#[must_use]
pub fn stream_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Aggregate {
            op: FoldOp::Sum,
            over: VarId(1),
        }],
        atoms: vec![Atom {
            source: AtomSource::Edb(ids::SPOKE),
            bindings: vec![(ids::spoke::ID, var(0)), (ids::spoke::VAL, var(1))],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

pub struct ForeignStream {
    buf: Vec<u8>,
}

impl ForeignStream {

    /// # Panics

    #[must_use]
    pub fn new(mib: u64) -> Self {
        Self {
            buf: vec![0u8; usize::try_from(mib << 20).expect("64-bit usize")],
        }
    }

    #[inline(never)]
    pub fn stream(&mut self) {
        let (lines, _) = self.buf.as_chunks_mut::<64>();
        for line in lines {
            line[0] = line[0].wrapping_add(1);
        }
        std::hint::black_box(self.buf.as_mut_ptr());
    }
}

pub struct DisplacedFamily {
    pub name: &'static str,
    pub kind: Kind,
    pub query: fn() -> Query,

    pub displace_mib: u64,

    pub about: &'static str,
}

#[must_use]
pub fn all() -> &'static [DisplacedFamily] {
    &[
        DisplacedFamily {
            name: "disp_probe",
            kind: Kind::Report,
            query: probe_query,
            displace_mib: 0,
            about: "2^19 hub-side probes/pass into the ~34 MiB forced spoke map — undisplaced control (itself DRAM-tier)",
        },
        DisplacedFamily {
            name: "disp_probe_d24",
            kind: Kind::Report,
            query: probe_query,
            displace_mib: 24,
            about: "the probe pass with 24 MiB foreign streaming between passes (SLC-tier displaced)",
        },
        DisplacedFamily {
            name: "disp_probe_d96",
            kind: Kind::Report,
            query: probe_query,
            displace_mib: 96,
            about: "the probe pass with 96 MiB foreign streaming between passes (DRAM-tier displaced)",
        },
        DisplacedFamily {
            name: "disp_stream",
            kind: Kind::Report,
            query: stream_query,
            displace_mib: 0,
            about: "the 16 MiB two-column scan fold — stream-shaped resident control",
        },
        DisplacedFamily {
            name: "disp_stream_d24",
            kind: Kind::Report,
            query: stream_query,
            displace_mib: 24,
            about: "the scan pass with 24 MiB foreign streaming between passes",
        },
        DisplacedFamily {
            name: "disp_stream_d96",
            kind: Kind::Report,
            query: stream_query,
            displace_mib: 96,
            about: "the scan pass with 96 MiB foreign streaming between passes",
        },
    ]
}

/// The lane's default protocol (a `--samples` override still applies — the
/// percentile machinery's needs (the cold protocol's 16-sample
pub const PROTO: Protocol = Protocol {
    warmups: 3,
    samples: 12,
};

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
) -> Result<(Db<DisplacedWorld>, rusqlite::Connection), String> {
    let sizes = DispSizes::of(cfg.scale);
    let db = mode.create(&dir.join("db"), DisplacedWorld)?;
    for rel in [ids::HUB, ids::SPOKE] {
        db.write(|tx| {
            tx.insert_dyn(rel, relation_rows(sizes, cfg.seed, rel))
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
    for rel in [ids::HUB, ids::SPOKE] {
        crate::corpus::insert_rows(
            &conn,
            schema().relation(rel),
            relation_rows(sizes, cfg.seed, rel),
        )
        .map_err(|e| format!("insert: {e}"))?;
    }
    conn.execute_batch("ANALYZE")
        .map_err(|e| format!("analyze: {e}"))?;
    Ok((db, conn))
}

#[must_use]
pub fn draws() -> Vec<Draw> {
    vec![scalar_draw(vec![])]
}

/// # Errors
pub fn verify_family(
    db: &Db<DisplacedWorld>,
    conn: &rusqlite::Connection,
    family: &DisplacedFamily,
) -> Result<(), String> {
    let query = (family.query)();
    let translated =
        translate(&query, schema(), &[]).map_err(|e| format!("{}: translate: {e}", family.name))?;
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
        .prepare(&translated.sql)
        .map_err(|e| format!("{}: mirror prepare: {e}", family.name))?;
    let mut buffer = Answers::new();
    for draw in draws() {
        let args = param_args(&draw);
        db.read(|snap| snap.execute(&mut prepared, &args, &mut buffer))
            .map_err(|e| format!("{}: execute: {e:?}", family.name))?;
        let ours = compare::from_answers(&buffer, &types);
        let theirs = compare::from_sqlite(&mut stmt, &translated.params, &draw, &types)
            .map_err(|e| format!("{}: mirror: {e}", family.name))?;
        compare::multisets(ours, theirs)
            .map_err(|m| format!("{}: draw {draw:?}: {m}", family.name))?;
    }
    Ok(())
}

/// The timed displaced lane: build the scratch world, verify every family, then
/// measure both engines under the interleaved protocol — the foreign stream
/// runs between passes on BOTH arms (the mirror is displaced exactly like the
/// engine), report-only rows beside the read families.
/// # Errors
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // one lane's full protocol, linear
pub fn bench_families(
    cfg: GenConfig,
    scratch: &Path,
    selected: &dyn Fn(&str) -> bool,
    samples: Option<u32>,
    alloc: bool,
    proxy_per_rep: bool,
    mode: crate::storemode::StoreMode,
) -> Result<Vec<report::ReadFamilyReport>, String> {
    if !all().iter().any(|family| selected(family.name)) {
        return Ok(Vec::new());
    }

    let proto = Protocol {
        warmups: PROTO.warmups,
        samples: samples.unwrap_or(PROTO.samples),
    };

    crate::devhonesty::assert_disk_backed(scratch, "the timed displaced families")
        .map_err(|refusal| refusal.to_string())?;
    let dir = scratch.join("displaced");
    std::fs::create_dir_all(&dir).map_err(|e| format!("displaced scratch: {e}"))?;
    eprintln!("bench: loading the displaced corpus");
    let (db, conn) = load_stores(&dir, cfg, mode)?;

    let mut out = Vec::new();
    for family in all() {
        if !selected(family.name) {
            continue;
        }
        eprintln!(
            "bench: displaced family {} ({} MiB between passes)",
            family.name, family.displace_mib
        );
        // Verify before time — row-identical or refuse to measure.
        verify_family(&db, &conn, family)?;

        let query = (family.query)();
        let mut prepared = db
            .prepare(&query)
            .map_err(|e| format!("{}: prepare: {e:?}", family.name))?;
        let sets = draws();
        let mut rotation = Rotation::new(sets.clone());
        let mut buffer = Answers::new();
        let mut run_ours = |prepared: &mut bumbledb::PreparedQuery<DisplacedWorld>| {
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
        let mut foreign = ForeignStream::new(family.displace_mib);
        let (ours, ghz_ours) = clockproxy::frequency_checked(|| {
            harness::measure_interleaved(
                proto,
                modes,
                1,
                || foreign.stream(),
                || run_ours(&mut prepared),
            )
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
                harness::measure_interleaved(
                    proto,
                    modes,
                    batch,
                    || foreign.stream(),
                    || run_ours(&mut prepared),
                )
            })?
        } else {
            (ours, ghz_ours)
        };

        let translated = translate(&query, schema(), &[])
            .map_err(|e| format!("{}: translate: {e}", family.name))?;
        let types: Vec<bumbledb::schema::ValueType> = prepared
            .signature()
            .columns
            .iter()
            .map(|column| *column.ty())
            .collect();
        let mut mirror = sqlite_run::PreparedFamily::new(&conn, &translated, types)?;
        let mut cursor = 0usize;
        let (theirs, ghz_theirs) = clockproxy::frequency_checked(|| {
            harness::measure_interleaved(
                proto,
                Modes::default(),
                batch,
                || foreign.stream(),
                || {
                    let index = cursor;
                    cursor = (cursor + 1) % sets.len();
                    sqlite_run::sample_args(&mut mirror, &sets[index])
                },
            )
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
            exec: None, 
            ghz: Some(merged.into()),
            p50_norm: ours.p50_norm,
        });
    }
    Ok(out)
}
