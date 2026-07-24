//! The DRAM-tier displaced lanes — the roster extension's measurement
//! infrastructure for the memory regime the S-scale roster cannot see
//! (`docs/reference/apple-silicon-performance.md`: residency is a
//! property of phase *interleaving*, not footprint —
//! `m2max.mem.residency-is-interleaving`; 24 MB of interleaved foreign
//! streaming degrades a nominally resident probe structure +53% —
//! `m2max.mem.co-tenant-displacement`). Two workload shapes, each with
//! a resident control and a displaced ladder:
//!
//! - `disp_probe*` — the join fold `Q(t, Sum(v)) :- Spoke(id, hub = h,
//!   val = v), Hub(id = h, tag = t)`: the executor iterates the HUB
//!   side (2^19 rows) and probes the forced SPOKE map keyed by hub
//!   value — ~2^19 scattered probes per pass. Working set, traced from
//!   the engine itself (the obs test in [`tests`] pins the engine's own
//!   `colt_force` event, not just the arithmetic): the forced map
//!   ingests all 2^20 spoke positions, lands
//!   [`FORCED_MAP_DISTINCT`] = 453,241 distinct hub keys, and sizes to
//!   2^18 buckets = 32 MiB bucket words + 2 MiB ctrl bytes ≈ **34 MiB**
//!   ([`forced_spoke_map_bytes`]). The force runs ONCE per prepared
//!   query (the view memo — every execute after the first shows
//!   `view_memo_hit` and zero `colt_force`/`image_build`, also pinned),
//!   so every timed pass is steady state: the 34 MiB map re-walked by
//!   2^19 hash-scattered probes, beside the 8 MiB hub image and the
//!   8 MiB spoke val column gathered through the map's position lists —
//!   ≈ 50 MiB touched per pass, past one P-cluster's 32 MiB L2 by
//!   construction.
//! - `disp_stream*` — the scan fold `Q(Sum(v)) :- Spoke(id, val = v)`:
//!   a 16 MiB two-column stream, the shape the ledger says pays 2.4–3×
//!   *less* under displacement than hit-heavy probes — the contrast
//!   control that shows the lanes distinguish shapes, not just bytes.
//!
//! First measured sessions (S, durable, mutex-held, clock-proxy clean;
//! 2026-07-16): the two shapes split exactly along the regime line. The
//! probe pass is already DRAM-tier *undisturbed* — its ≈ 50 MiB
//! steady-state working set exceeds one P-cluster's L2 on its own, so
//! the foreign mass adds no eviction the pass wasn't already paying and
//! is measured NEUTRAL on it (135.2/132.5 ms at d24/d96 vs 134.8 ms
//! resident; an apparent 1.22× d24 gap in the first session did not
//! reproduce and is recorded as cross-block ambient, not effect). An
//! earlier revision of this doc attributed the neutrality to "~170 MiB
//! of force writes per pass" self-displacing the row — REFUTED by the
//! engine's own trace: the force runs once per prepare, never in a
//! timed pass (the retraction rides commit history; the obs test now
//! pins the memoization). The stream pass is L2/SLC-resident
//! undisturbed and pays the displacement as predicted. Confirmed
//! post-review with `--proxy-per-rep` (three further mutex-held
//! sessions, DVFS-normalized p50 ratios vs the resident row): **96 MiB
//! = 1.18–1.20×** (clean clock brackets in two of three — the durable
//! fact, previously quoted 1.19–1.22× from raw cross-block p50s), while
//! **24 MiB wobbles 1.10–1.19×** across sessions — the first record's
//! 1.08–1.10× band was too narrow; quote the 24 MiB point only with its
//! spread. Read the rows accordingly: `disp_probe` is the roster's
//! standing DRAM-tier probe row (the >32 MiB working set itself);
//! `disp_stream_d*` are its standing displaced-residency rows, `d96`
//! the one to quote.
//!
//! The displaced variants stream a foreign buffer BETWEEN engine passes
//! (the in-situ shape, [`ForeignStream`]) through
//! [`harness::measure_interleaved`] — the foreign traffic is never
//! inside a timed span, so each sample prices the engine pass *given*
//! the displacement. The displacement mass is the row's parameter
//! ([`DisplacedFamily::displace_mib`]): 24 MiB (the co-tenant fact's
//! point, inside the 48 MB SLC) and 96 MiB (past the SLC — the full
//! DRAM tier). Both engines get the identical between-pass traffic —
//! the mirror is displaced exactly like the engine.
//!
//! Discipline mirrors the closure lane: seeded corpus regenerated per
//! run (never stored), verify-before-time inline at the lane's own
//! scale (every family × draw row-identical across engines before a
//! single timed sample), `SQLite` parity at the shrunk `Tiny` scale in
//! tests (the windowed family's unit-mass precedent — the brute oracle
//! is O(rows) per pass), the exact warm protocol shape with the lane's
//! own default sample count ([`PROTO`]: probe passes are ~130 ms, so 12
//! samples suffice and 256 would take minutes; `--samples` still
//! overrides), and `Kind::Report` rows: measurement, not gate claims.

use bumbledb::schema::ValidateDescriptor as _;
use std::path::Path;

use bumbledb::{
    AggOp, Answers, Atom, AtomSource, Db, FindTerm, Query, RelationId, Rule, Term, Value, VarId,
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

/// Relation and field ids by declaration order.
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

/// The validated displaced schema, memoized for the mirror's DDL.
///
/// # Panics
///
/// Never in practice: the declaration passes the acceptance gate.
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

/// The displaced corpus shape. Like the closure world, the shape IS the
/// identity — one size for every timed scale (the lane prices a memory
/// regime, not the ledger's mass), `Tiny` for the parity slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DispSizes {
    /// Hub rows — the iterated probing side (2^19 probes per pass at
    /// the bench point) and the key space the spoke hubs scatter over.
    pub hubs: u64,
    /// Spoke rows — the forced-map side: all 2^20 positions ingest into
    /// the COLT keyed by hub value (≈ 453k distinct), sizing the probed
    /// structure to ≈ 34 MiB ([`forced_spoke_map_bytes`]), past the
    /// 32 MiB L2.
    pub spokes: u64,
    /// Distinct tag values (the fold's group count — small by design so
    /// the group map never competes with the probed structure).
    pub tags: u64,
}

impl DispSizes {
    /// Two size points, the closure precedent: `Tiny` for tests and the
    /// differential slice, the DRAM-tier shape for every timed scale.
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

    /// The hub image's byte mass: 2 word columns × 8 B per row
    /// (`bumbledb::image`: every u64 field decodes to one 8-byte word
    /// column).
    #[must_use]
    pub fn hub_image_bytes(&self) -> u64 {
        self.hubs * 2 * 8
    }

    /// The spoke image's byte mass: 3 word columns × 8 B per row.
    #[must_use]
    pub fn spoke_image_bytes(&self) -> u64 {
        self.spokes * 3 * 8
    }
}

/// The forced COLT map's byte mass over a singleton-arity node holding
/// `positions` positions with `distinct` distinct key values, computed
/// from the engine's own sizing rule (`bumbledb::exec::colt::force`):
/// initial buckets `next_pow2(clamp(positions/8, 16, 2·positions) ·
/// 5/16)` (the pre-pass guess is from the POSITION count — distinct
/// keys are unknown before the pass), then rehash-doubling per ingested
/// position while `(len + 1) · 5 > nbuckets · 16` with `len` the
/// distinct keys landed so far; ctrl is `nbuckets · 8` bytes, buckets
/// are `nbuckets · (8·arity + 8)` u64 words. The doubling loop here
/// uses `(distinct + 1)` — exact whenever any position follows the last
/// new key (this lane's shape: positions ≫ distinct, so appends trail
/// the final insert), conservative by at most one doubling at exact
/// boundary counts on an all-distinct node (the engine's own
/// check-before-probe over-size, `exec/colt/force.rs`).
///
/// The number this returns is the lane's ≥ 32 MiB claim — pinned by
/// [`tests`] against BOTH this arithmetic and the engine's own
/// `colt_force` trace at the bench shape, so a layout change in the
/// engine shows up as a failing assertion, not a silently mis-labeled
/// regime.
///
/// # Panics
///
/// Never in practice: 64-bit `usize` (the arithmetic never leaves the
/// lane's ≤ 2^20-position range).
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

/// The traced shape of the forced map at the bench point, pinned by the
/// obs test in [`tests`] against the engine's own `colt_force` event:
/// the executor iterates the HUB side and probes SPOKE, so the forced
/// map ingests all 2^20 spoke positions keyed by hub value —
/// 453,241 distinct (2^19 keys scattered by 2^20 uniform draws,
/// `1 − e^-2` occupancy; seed-invariant for seeds < 2^20, where the
/// generator's `seed ^ row` merely permutes the row set).
pub const FORCED_MAP_POSITIONS: u64 = 1 << 20;
/// See [`FORCED_MAP_POSITIONS`].
pub const FORCED_MAP_DISTINCT: u64 = 453_241;

/// One relation's full row stream — a pure function of `(seed, sizes)`
/// via the corpus generator's per-row mix, so streams are restartable
/// and identical across engines. Spoke hubs scatter uniformly over the
/// hub key space, filling the forced spoke map to ~453k distinct hub
/// keys (`1 − e^-2` of 2^19); each pass's 2^19 hub-side probes then
/// hash-scatter across the full ≥ 32 MiB structure, far past any
/// predictor's capacity.
#[must_use]
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

/// probe — `Q(t, Sum(v)) :- Spoke(id, hub = h, val = v),
/// Hub(id = h, tag = t)`: the hub side iterates, probing the forced
/// spoke map keyed by hub value (the direction the engine actually
/// plans — pinned by the obs test in [`tests`]), folded by tag. The
/// fresh spoke id binding makes every binding distinct, so the
/// distinct-bindings elision engages (the balance-family precedent) and
/// no seen-set competes with the probed map.
#[must_use]
pub fn probe_query() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(1)),
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

/// stream — `Q(Sum(v)) :- Spoke(id, val = v)`: the pure two-column scan
/// fold (16 MiB per pass at the bench shape) — the stream-shaped
/// contrast row.
#[must_use]
pub fn stream_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::Sum,
            over: Some(VarId(1)),
        }],
        atoms: vec![Atom {
            source: AtomSource::Edb(ids::SPOKE),
            bindings: vec![(ids::spoke::ID, var(0)), (ids::spoke::VAL, var(1))],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

/// The foreign co-tenant: `mass` MiB streamed read-modify-write at
/// cache-line stride between engine passes — one byte touched per 64 B
/// line allocates and dirties the full line, the cheapest full-mass
/// eviction pressure (a 96 MiB pass is a fraction of a millisecond,
/// bandwidth-bound). Mass 0 is the resident control: the buffer is
/// empty and [`ForeignStream::stream`] is a no-op through the same code
/// path.
pub struct ForeignStream {
    buf: Vec<u8>,
}

impl ForeignStream {
    /// A zeroed buffer of `mib` MiB.
    ///
    /// # Panics
    ///
    /// Never in practice: 64-bit `usize` (the ladder tops out at
    /// 96 MiB).
    #[must_use]
    pub fn new(mib: u64) -> Self {
        Self {
            buf: vec![0u8; usize::try_from(mib << 20).expect("64-bit usize")],
        }
    }

    /// One full read-modify-write pass over the buffer.
    /// `inline(never)`: the foreign pass stays one disassembly-gated
    /// symbol (the RMW loop must exist as claimed), and its cost never
    /// smears into the callers' codegen.
    #[inline(never)]
    pub fn stream(&mut self) {
        let (lines, _) = self.buf.as_chunks_mut::<64>();
        for line in lines {
            line[0] = line[0].wrapping_add(1);
        }
        std::hint::black_box(self.buf.as_mut_ptr());
    }
}

/// One displaced family: a query shape and its between-pass foreign
/// mass — the displacement mass IS the row's parameter.
pub struct DisplacedFamily {
    pub name: &'static str,
    pub kind: Kind,
    pub query: fn() -> Query,
    /// Foreign-stream mass between passes, in MiB (0 = the resident
    /// control).
    pub displace_mib: u64,
    /// The regime the row instruments (rendered nowhere yet — the
    /// registry documents itself).
    pub about: &'static str,
}

/// The displaced registry: each shape's resident control, the co-tenant
/// fact's 24 MiB point (inside the SLC), and the 96 MiB
/// past-the-SLC point.
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

/// The lane's default protocol (a `--samples` override still applies —
/// the driver passes it through like every other read lane): passes run
/// ~130 µs (the stream shape) to ~130 ms (the DRAM-tier probe shape),
/// so the read default's 256 samples would spend minutes on the probe
/// ladder buying nothing — 12 measured samples sit far above the timer
/// quantum even at the stream shape (≈ 260× the floor) and well inside
/// the percentile machinery's needs (the cold protocol's 16-sample
/// precedent).
pub const PROTO: Protocol = Protocol {
    warmups: 3,
    samples: 12,
};

/// The mirror's DDL: mapped tables plus the statement-derived indexes
/// (the `Spoke(hub) <= Hub(id)` containment gives the mirror its hub
/// probe index).
#[must_use]
pub fn ddl() -> Vec<String> {
    sqlmap::schema_ddl(schema())
}

/// Loads the displaced corpus into a fresh engine store and a `SQLite`
/// mirror file — targets before sources, the loader law.
///
/// # Errors
///
/// Engine and `SQLite` errors, stringified.
pub fn load_stores(
    dir: &Path,
    cfg: GenConfig,
    mode: crate::storemode::StoreMode,
) -> Result<(Db<DisplacedWorld>, rusqlite::Connection), String> {
    let sizes = DispSizes::of(cfg.scale);
    let db = mode.create(&dir.join("db"), DisplacedWorld)?;
    for rel in [ids::HUB, ids::SPOKE] {
        db.bulk_load_dyn(rel, relation_rows(sizes, cfg.seed, rel))
            .map_err(|e| format!("load: {e:?}"))?;
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

/// The lane's draws: both shapes are param-less full folds (the
/// stats-family precedent) — one empty draw. A 2^20-probe scattered
/// sequence is its own fresh data; no rotation is needed to defeat the
/// predictor.
#[must_use]
pub fn draws() -> Vec<Draw> {
    vec![scalar_draw(vec![])]
}

/// Verify-before-time, inline at the lane's own scale (the closure
/// precedent): every family × draw row-identical across engines.
///
/// # Errors
///
/// The first mismatch, rendered — or either engine's error.
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
        .predicate()
        .columns
        .iter()
        .map(|column| column.ty.clone())
        .collect();
    let mut stmt = conn
        .prepare(&translated.sql)
        .map_err(|e| format!("{}: mirror prepare: {e}", family.name))?;
    let mut buffer = Answers::new();
    for draw in draws() {
        let args = param_args(&draw);
        db.read(|snap| snap.execute_args(&mut prepared, &args, &mut buffer))
            .map_err(|e| format!("{}: execute: {e:?}", family.name))?;
        let ours = compare::from_answers(&buffer, &types);
        let theirs = compare::from_sqlite(&mut stmt, &translated.params, &draw, &types)
            .map_err(|e| format!("{}: mirror: {e}", family.name))?;
        compare::multisets(ours, theirs)
            .map_err(|m| format!("{}: draw {draw:?}: {m}", family.name))?;
    }
    Ok(())
}

/// The timed displaced lane: build the scratch world, verify every
/// family, then measure both engines under the interleaved protocol —
/// the foreign stream runs between passes on BOTH arms (the mirror is
/// displaced exactly like the engine), report-only rows beside the read
/// families.
///
/// # Errors
///
/// Refusals (RAM-backed scratch), verify mismatches, and engine errors
/// — each message names the family.
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
    // The lane's own default sample count, but the user's --samples
    // request applies here exactly like every other read lane.
    let proto = Protocol {
        warmups: PROTO.warmups,
        samples: samples.unwrap_or(PROTO.samples),
    };
    // The device-honesty rule is symmetric: this lane times reads
    // against its scratch world, so the scratch is checked exactly like
    // the write families'.
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
        let mut run_ours = |prepared: &mut bumbledb::PreparedQuery<'_, DisplacedWorld>| {
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
            .predicate()
            .columns
            .iter()
            .map(|column| column.ty.clone())
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
            exec: None, // the profile pass would time nothing new; the plan digest is the tests' job
            ghz: Some(merged.into()),
            p50_norm: ours.p50_norm,
        });
    }
    Ok(out)
}
