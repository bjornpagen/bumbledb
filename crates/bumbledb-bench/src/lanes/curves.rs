//! The curves lane: four families as scale curves at S/M/L with inline
//! oracle gates, plus the cold/warm/memoized warmth panel —
//! REPORT-class ([`crate::lanes`] carries the charter).
//!
//! A scale curve is data, not a script: [`CURVE_FAMILIES`] is a
//! registry over the EXISTING family definitions — `triangle` and
//! `point` from [`crate::families`] (point is the crud/point-regime key
//! probe), `busy_scan` from [`crate::calendar::families`], and
//! `closure_fanout` over [`crate::closure::closure_program`] (the
//! rings-shaped recursive mass family — the transitive-closure world).
//! Zero new query semantics; only corpus scale is parameterized, the
//! closure world through the lane-owned [`curve_sizes`] ladder so the
//! existing closure lane's identity ([`ClosSizes::of`], constant across
//! S/M/L by design) is untouched.
//!
//! **The oracle law** is carried by control-flow shape borrowed from
//! `scenarios::run_query`: inside [`curve_point`] the gate call
//! DOMINATES the timing call — for every draw the engine's answers and
//! the `SQLite` twin's answers must be value-identical multisets
//! ([`crate::compare::multisets`]) before anything reaches a timer. The
//! ledger/calendar corpora come from the digest-keyed cache
//! ([`crate::driver::ensure_corpus`]) READ-ONLY, and the verify stamp
//! is NOT required: the law is satisfied by the INLINE per-draw
//! multiset gate — the scenarios/closure precedent (recursion and
//! scenario worlds live outside the stamped family registry and gate
//! inline; this lane says so here and does the same).
//!
//! **The DNF cap** ([`DnfCap`]) is a typed outcome, not an exception
//! path: a capped `SQLite` region produces `Ok(None)` and the point
//! carries a [`CapEvent`] naming where it fired (`"gate"`, `"timing"`,
//! `"hand"`) with a `None` stats field — "excluded" is representable
//! and countable instead of a dropped row. The deadline re-arms PER
//! REGION (one gate pass or one whole timing protocol block); a capped
//! TIMING block reports the whole block as capped, honestly
//! excluded-and-counted. A capped GATE means NEITHER side is timed —
//! never time what is not verified.
//!
//! **`SQLite` parity per lane**: ledger/calendar twins open through
//! [`crate::sqlite_run::open_for_bench`] (WAL, `synchronous=FULL`,
//! 256 MiB cache, 1 GiB mmap, checkpointed) with
//! [`crate::sqlite_run::FairnessCheck`] asserted before timing
//! (indexes + ANALYZE present); the closure twin loads through the
//! closure lane's own loader (configured session, statement-derived
//! indexes, ANALYZE). Statements are prepared once and reused. The
//! `SQLite` side is the CANONICAL translation everywhere — and where
//! the canonical rendering inflates SQL (the Allen 9-basic OR-chain of
//! `busy_scan`), the hand-tuned twin [`BUSY_SCAN_HAND`] runs beside it,
//! gated exactly like the canonical before it is timed. Both are
//! reported — we never flatter ourselves.
//!
//! **The warmth panel** ([`warmth_panel`], `--warmth`) makes the
//! (relation, generation) image cache and the resolved-filter view
//! slots (docs/architecture/50-storage.md) — the memo the warm suite
//! otherwise silently enjoys — an explicit three-point representation
//! {cold, warm, memoized}, measured symmetrically on both engines
//! (per-round reopen for cold/warm, a reused prepared statement for
//! memoized), so the flatterer becomes a chart instead of a default.
//! Honesty bound: reopen-cold is process-fresh but OS-page-cache-warm —
//! as close to cold as the harness allows without dropping kernel
//! caches. The panel runs at the FIRST scale in `--scales`
//! (deterministic and Tiny-testable), only on stores already gated at
//! that scale this run.
//!
//! **Quantum-floor rebatch is deliberately omitted** (the closure
//! lane's `measure_batched` re-run): this lane is report-class and its
//! curve scales put medians well above the 500 ns floor; a Tiny smoke
//! run may quantize, and Tiny is never published.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use bumbledb::schema::ValueType;
use bumbledb::{Answers, Db, ParamId, Program, RelationId, Value};

use crate::calendar::corpus_gen::CalSizes;
use crate::clockproxy;
use crate::closure::{self, ClosSizes};
use crate::compare;
use crate::corpus_gen::{GenConfig, Scale, Sizes};
use crate::families::{Draw, param_args, scalar_draw, set_bindings};
use crate::harness::{self, Protocol, Rotation, Stats};
use crate::report::{self, GhzReport, Provenance};
use crate::sqlite_run::{self, FairnessCheck, PreparedFamily, open_for_bench};
use crate::translate::{ParamSlot, Translated, translate};

/// The whole curves report, plain data.
#[derive(Debug, Clone, PartialEq)]
pub struct CurvesReport {
    pub provenance: Provenance,
    pub seed: u64,
    pub samples: u32,
    pub cap_ms: u64,
    pub families: Vec<FamilyCurve>,
}

/// One family's curve across the scale ladder.
#[derive(Debug, Clone, PartialEq)]
pub struct FamilyCurve {
    pub name: &'static str,
    pub world: &'static str,
    pub rows: Vec<CurvePoint>,
    pub warmth: Option<Warmth>,
}

/// One (family, scale) point. Absent stats mean the engine never
/// produced a timing for the point (a cap event says why). `ghz` is the
/// merged clock-proxy bracket over every timed block at the point — the
/// contamination discriminator every other timed lane carries (finding
/// 072): a co-tenant's slow-clock span is recorded, never invisible.
#[derive(Debug, Clone, PartialEq)]
pub struct CurvePoint {
    pub scale: &'static str,
    pub facts: u64,
    pub answers: u64,
    pub ours: Option<Stats>,
    pub theirs: Option<Stats>,
    pub theirs_hand: Option<Stats>,
    pub cap: Option<CapEvent>,
    pub ghz: Option<GhzReport>,
}

/// Where the DNF cap fired: `"gate"` (the oracle pass), `"timing"`
/// (the canonical twin), or `"hand"` (the hand-tuned twin).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapEvent {
    pub at: &'static str,
}

/// The cold/warm/memoized panel, both engines — one proxy bracket
/// around the whole panel (the reopen rounds are not idempotent, so
/// the stamp annotates, never re-runs).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Warmth {
    pub ours_cold: Stats,
    pub ours_warm: Stats,
    pub ours_memoized: Stats,
    pub theirs_cold: Stats,
    pub theirs_warm: Stats,
    pub theirs_memoized: Stats,
    pub ghz: Option<GhzReport>,
}

fn push_point(out: &mut String, point: &CurvePoint) {
    let _ = write!(
        out,
        "{{\"scale\":\"{}\",\"facts\":{},\"answers\":{},\"ours\":",
        point.scale, point.facts, point.answers
    );
    super::push_opt_stats(out, point.ours.as_ref());
    out.push_str(",\"theirs\":");
    super::push_opt_stats(out, point.theirs.as_ref());
    out.push_str(",\"theirs_hand\":");
    super::push_opt_stats(out, point.theirs_hand.as_ref());
    out.push_str(",\"cap\":");
    match point.cap {
        Some(cap) => {
            let _ = write!(out, "{{\"at\":\"{}\"}}", cap.at);
        }
        None => out.push_str("null"),
    }
    super::push_ghz(out, point.ghz);
    out.push('}');
}

fn push_warmth(out: &mut String, warmth: Option<&Warmth>) {
    out.push_str(",\"warmth\":");
    let Some(w) = warmth else {
        out.push_str("null");
        return;
    };
    out.push_str("{\"ours_cold\":");
    super::push_stats(out, &w.ours_cold);
    out.push_str(",\"ours_warm\":");
    super::push_stats(out, &w.ours_warm);
    out.push_str(",\"ours_memoized\":");
    super::push_stats(out, &w.ours_memoized);
    out.push_str(",\"theirs_cold\":");
    super::push_stats(out, &w.theirs_cold);
    out.push_str(",\"theirs_warm\":");
    super::push_stats(out, &w.theirs_warm);
    out.push_str(",\"theirs_memoized\":");
    super::push_stats(out, &w.theirs_memoized);
    super::push_ghz(out, w.ghz);
    out.push('}');
}

/// The machine-consumable curves artifact — hand-rolled, like
/// `report/json_out.rs`.
#[must_use]
pub fn to_json(report: &CurvesReport) -> String {
    let mut out = String::new();
    out.push_str("{\"provenance\":");
    super::push_provenance(&mut out, &report.provenance);
    let _ = write!(
        out,
        ",\"seed\":{},\"samples\":{},\"cap_ms\":{},\"families\":[",
        report.seed, report.samples, report.cap_ms
    );
    for (index, family) in report.families.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            "{{\"name\":\"{}\",\"world\":\"{}\",\"rows\":[",
            family.name, family.world
        );
        for (row_index, point) in family.rows.iter().enumerate() {
            if row_index > 0 {
                out.push(',');
            }
            push_point(&mut out, point);
        }
        out.push(']');
        push_warmth(&mut out, family.warmth.as_ref());
        out.push('}');
    }
    out.push_str("]}");
    out
}

// ---------------------------------------------------------------------
// The registry: four curve families over the existing definitions.
// ---------------------------------------------------------------------

/// Which corpus world a curve family runs against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum World {
    Ledger,
    Calendar,
    Closure,
}

impl World {
    fn label(self) -> &'static str {
        match self {
            Self::Ledger => "ledger",
            Self::Calendar => "calendar",
            Self::Closure => "closure",
        }
    }
}

/// One curve family: a name into an existing registry plus its world.
struct CurveFamily {
    name: &'static str,
    world: World,
}

/// The lane roster: `triangle` (the cyclic self-join), `point` (the
/// crud/point-regime key probe), `busy_scan` (the Allen-mask scan whose
/// canonical SQL is the inflated 9-basic OR-chain — the hand-twin
/// case), and `closure_fanout` (the rings-shaped recursive mass family
/// — the transitive-closure world).
const CURVE_FAMILIES: [CurveFamily; 4] = [
    CurveFamily {
        name: "triangle",
        world: World::Ledger,
    },
    CurveFamily {
        name: "point",
        world: World::Ledger,
    },
    CurveFamily {
        name: "busy_scan",
        world: World::Calendar,
    },
    CurveFamily {
        name: "closure_fanout",
        world: World::Closure,
    },
];

/// Resolves `--families` against the roster; an unknown name is an
/// `Err` listing the four.
fn select(names: Option<&[String]>) -> Result<Vec<&'static CurveFamily>, String> {
    let Some(names) = names else {
        return Ok(CURVE_FAMILIES.iter().collect());
    };
    for name in names {
        if !CURVE_FAMILIES.iter().any(|family| family.name == name) {
            return Err(format!(
                "curves: unknown family {name} — the lane's four are \
                 triangle, point, busy_scan, closure_fanout"
            ));
        }
    }
    Ok(CURVE_FAMILIES
        .iter()
        .filter(|family| names.iter().any(|name| name == family.name))
        .collect())
}

// ---------------------------------------------------------------------
// The closure curve ladder — lane-local; `ClosSizes::of` untouched.
// ---------------------------------------------------------------------

/// The lane-owned closure scale ladder: ~10x edge growth per step
/// (S ≈ 8.8k edges, M ≈ 78k, L ≈ 709k; the test pins the ratios). The
/// `Tiny` arm matches `ClosSizes::of(Tiny)` so the smoke slice is the
/// closure lane's own fuzz point.
fn curve_sizes(scale: Scale) -> ClosSizes {
    match scale {
        Scale::Tiny => ClosSizes {
            chain: 64,
            fanout: 4,
            depth: 3,
        },
        Scale::S => ClosSizes {
            chain: 4_096,
            fanout: 8,
            depth: 4,
        },
        Scale::M => ClosSizes {
            chain: 40_960,
            fanout: 8,
            depth: 5,
        },
        Scale::L => ClosSizes {
            chain: 409_600,
            fanout: 8,
            depth: 6,
        },
    }
}

/// The fanout anchors, mirroring `closure::fanout_params`' four-draw
/// shape but computed from the lane's own sizes: the tree root, a
/// depth-1 subtree root, a leaf, and the miss.
fn closure_curve_params(sizes: &ClosSizes) -> Vec<Draw> {
    let base = sizes.tree_base();
    vec![
        scalar_draw(vec![Value::U64(base)]),
        scalar_draw(vec![Value::U64(base + 1)]),
        scalar_draw(vec![Value::U64(sizes.nodes() - 1)]),
        scalar_draw(vec![Value::U64(sizes.nodes() + 1_000_000)]),
    ]
}

// ---------------------------------------------------------------------
// The DNF cap (LAW 5): a typed outcome, re-armed per region.
// ---------------------------------------------------------------------

/// The number of `SQLite` VM ops between deadline checks: coarse enough
/// that the handler's `Instant::now()` read vanishes against any region
/// worth capping, fine enough that even a Tiny gate query crosses it.
const CAP_GRANULARITY_OPS: std::ffi::c_int = 4_096;

/// The per-region `SQLite` wall-clock bound.
#[derive(Debug, Clone, Copy)]
struct DnfCap {
    cap: Duration,
}

impl DnfCap {
    /// Runs one region (one gate pass or one whole timing protocol
    /// block) under the cap: installs the progress handler with a
    /// deadline captured at entry, ALWAYS clears it before returning,
    /// and folds an interrupt into `Ok(None)` — exceeded-cap as data.
    ///
    /// Cap detection matches the handler's own typed record of its
    /// trip, never the error text: the interrupt it induces arrives
    /// here stringified through the compare/sample seams (where the
    /// `rusqlite::ErrorCode::OperationInterrupted` code is folded into
    /// a message), so the handler stores the trip in a flag at the
    /// moment it returns `true` — the one place the code fires. A trip
    /// observed after the region completed keeps its finished result;
    /// an error without a trip propagates untouched. The zero cap is
    /// the degenerate budget: no region fits inside it, including its
    /// first op — excluded before entry.
    fn guarded<T>(
        self,
        conn: &rusqlite::Connection,
        f: impl FnOnce() -> Result<T, String>,
    ) -> Result<Option<T>, String> {
        if self.cap.is_zero() {
            return Ok(None);
        }
        let deadline = Instant::now() + self.cap;
        let tripped = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&tripped);
        conn.progress_handler(
            CAP_GRANULARITY_OPS,
            Some(move || {
                if Instant::now() >= deadline {
                    flag.store(true, Ordering::Relaxed);
                    true
                } else {
                    false
                }
            }),
        );
        let result = f();
        conn.progress_handler(0, None::<fn() -> bool>);
        match result {
            Ok(value) => Ok(Some(value)),
            Err(_) if tripped.load(Ordering::Relaxed) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

// ---------------------------------------------------------------------
// Family bundles: everything a (family, scale) point needs, as data.
// ---------------------------------------------------------------------

/// The hand-tuned `busy_scan` twin (LAW 4's inflated-canonical case):
/// `INTERSECTS` over half-open intervals is exactly
/// `start < window_end AND window_start < end`, beside the canonical
/// 9-basic OR-chain golden (`calendar::families::BUSY_SCAN`). Gated
/// exactly like the canonical before it is ever timed; both are
/// reported — we never flatter ourselves.
const BUSY_SCAN_HAND: &str = "SELECT DISTINCT t0.\"person\", t0.\"span_start\", t0.\"span_end\" FROM \"Claim\" AS t0 WHERE t0.\"arm\" = 0 AND t0.\"span_start\" < ?2 AND ?1 < t0.\"span_end\"";

/// [`BUSY_SCAN_HAND`]'s positional slots: the window's two halves.
const BUSY_SCAN_HAND_SLOTS: [ParamSlot; 2] =
    [ParamSlot::Start(ParamId(0)), ParamSlot::End(ParamId(0))];

/// The one param slot of [`closure::CLOSURE_SQL`].
fn closure_translated() -> Translated {
    Translated {
        sql: closure::CLOSURE_SQL.to_owned(),
        params: vec![ParamSlot::Whole(ParamId(0))],
    }
}

/// One (family, scale) unit of work, fully resolved: the engine program
/// (queries embed through the degenerate `From<Query> for Program`),
/// the family's existing draws, the canonical `SQLite` twin, the
/// optional hand twin, and the world's fact count. All four families
/// draw scalars only, so the canonical SQL is one statement per point,
/// prepared once and rebound per draw.
struct Bundle {
    program: Program,
    draws: Vec<Draw>,
    canonical: Translated,
    hand: Option<Translated>,
    facts: u64,
}

fn ledger_facts(scale: Scale) -> u64 {
    let sizes = Sizes::of(scale);
    (0..crate::schema::ids::RELATIONS)
        .map(|rel| sizes.rows(RelationId(rel)))
        .sum()
}

fn calendar_facts(scale: Scale) -> u64 {
    let sizes = CalSizes::of(scale);
    (0..crate::calendar::ids::RELATIONS)
        .map(|rel| sizes.rows(RelationId(rel)))
        .sum()
}

fn ledger_bundle(name: &str, cfg: &GenConfig) -> Result<Bundle, String> {
    let family = crate::families::all()
        .iter()
        .find(|family| family.name == name)
        .ok_or_else(|| format!("curves: {name} is not a ledger family"))?;
    let query = (family.query)();
    let draws = (family.params)(cfg);
    let canonical = translate(&query, crate::schema::schema(), &set_bindings(&draws[0]))
        .map_err(|e| format!("{name}: translate: {e}"))?;
    Ok(Bundle {
        program: Program::from(query),
        draws,
        canonical,
        hand: None,
        facts: ledger_facts(cfg.scale),
    })
}

fn calendar_bundle(name: &str, cfg: &GenConfig) -> Result<Bundle, String> {
    let family = crate::calendar::families::all()
        .iter()
        .find(|family| family.name == name)
        .ok_or_else(|| format!("curves: {name} is not a calendar family"))?;
    let query = (family.query)();
    let draws = (family.params)(cfg);
    let canonical = family.sql_for(&query, &draws[0])?;
    let hand = (name == "busy_scan").then(|| Translated {
        sql: BUSY_SCAN_HAND.to_owned(),
        params: BUSY_SCAN_HAND_SLOTS.to_vec(),
    });
    Ok(Bundle {
        program: Program::from(query),
        draws,
        canonical,
        hand,
        facts: calendar_facts(cfg.scale),
    })
}

fn closure_bundle(scale: Scale) -> Bundle {
    let sizes = curve_sizes(scale);
    Bundle {
        program: closure::closure_program(),
        draws: closure_curve_params(&sizes),
        canonical: closure_translated(),
        hand: None,
        facts: sizes.nodes() + sizes.edges(),
    }
}

fn bundle_for(family: &CurveFamily, cfg: &GenConfig) -> Result<Bundle, String> {
    match family.world {
        World::Ledger => ledger_bundle(family.name, cfg),
        World::Calendar => calendar_bundle(family.name, cfg),
        World::Closure => Ok(closure_bundle(cfg.scale)),
    }
}

// ---------------------------------------------------------------------
// Gate then time — the run_query shape, per (family, scale).
// ---------------------------------------------------------------------

/// The inline oracle gate for one `SQLite` lane: every draw's result
/// multiset must equal the engine's, or the point is an `Err` — nothing
/// downstream of a disagreement is ever timed.
fn gate_lane(
    conn: &rusqlite::Connection,
    label: &str,
    translated: &Translated,
    draws: &[Draw],
    ours: &[Vec<compare::Answer>],
    types: &[ValueType],
) -> Result<(), String> {
    let mut stmt = conn
        .prepare(&translated.sql)
        .map_err(|e| format!("{label}: oracle prepare: {e}"))?;
    for (index, draw) in draws.iter().enumerate() {
        let theirs = compare::from_sqlite(&mut stmt, &translated.params, draw, types)
            .map_err(|e| format!("{label}: oracle execute: {e}"))?;
        compare::multisets(ours[index].clone(), theirs).map_err(|mismatch| {
            format!(
                "{label} draw {index}: ENGINES DISAGREE — not timing a wrong answer\n{mismatch}"
            )
        })?;
    }
    Ok(())
}

/// Times one `SQLite` lane under the cap: the whole protocol block is
/// one capped region (statement prepared once, draws rotated); a trip
/// anywhere reports the block as capped — `Ok(None)`.
fn time_lane(
    conn: &rusqlite::Connection,
    cap: DnfCap,
    translated: &Translated,
    draws: &[Draw],
    types: &[ValueType],
    proto: Protocol,
) -> Result<Option<(Stats, clockproxy::GhzStamp)>, String> {
    cap.guarded(conn, || {
        let mut family = PreparedFamily::new(conn, translated, types.to_vec())?;
        let mut rotation = Rotation::new((0..draws.len()).collect::<Vec<_>>());
        let (measured, ghz) = clockproxy::stamped(|| {
            harness::measure(proto, || {
                let index = rotation.next_index();
                sqlite_run::sample_args(&mut family, &draws[index])
            })
        })?;
        Ok((measured.stats, ghz))
    })
}

/// One (family, scale) point: gate (capped) → time ours → time the
/// canonical twin (capped) → gate + time the hand twin (capped). The
/// gate call dominates the timing call — nothing reaches a timer
/// without a value-identical multiset agreement in this same function.
fn curve_point<S>(
    name: &str,
    scale_label: &'static str,
    db: &Db<S>,
    conn: &rusqlite::Connection,
    bundle: &Bundle,
    proto: Protocol,
    cap: DnfCap,
) -> Result<CurvePoint, String> {
    eprintln!("curves: {name} at {scale_label}");
    let mut prepared = db
        .prepare(&bundle.program)
        .map_err(|e| format!("{name}: prepare: {e:?}"))?;
    let types: Vec<ValueType> = prepared
        .predicate()
        .columns
        .iter()
        .map(|column| column.ty.clone())
        .collect();

    // The engine's answers for every draw — the gate's left side.
    let mut buffer = Answers::new();
    let mut ours_answers = Vec::with_capacity(bundle.draws.len());
    for draw in &bundle.draws {
        let args = param_args(draw);
        db.read(|snap| snap.execute_args(&mut prepared, &args, &mut buffer))
            .map_err(|e| format!("{name}: execute: {e:?}"))?;
        ours_answers.push(compare::from_answers(&buffer, &types));
    }

    // THE GATE: one capped region over every draw. Capped ⇒ neither
    // side is timed — never time what is not verified.
    let gate = cap.guarded(conn, || {
        gate_lane(
            conn,
            name,
            &bundle.canonical,
            &bundle.draws,
            &ours_answers,
            &types,
        )
    })?;
    if gate.is_none() {
        return Ok(CurvePoint {
            scale: scale_label,
            facts: bundle.facts,
            answers: 0,
            ours: None,
            theirs: None,
            theirs_hand: None,
            cap: Some(CapEvent { at: "gate" }),
            ghz: None,
        });
    }

    // Ours: the run_query timing shape — draws rotated, uncapped (the
    // engine answers for its own latency), bracketed by the retry-capable
    // proxy (fsync-free read timing is idempotent).
    let mut rotation = Rotation::new(bundle.draws.clone());
    let (ours, mut ghz) = clockproxy::frequency_checked(|| {
        harness::measure(proto, || {
            let args = param_args(rotation.next_set());
            db.read(|snap| snap.execute_args(&mut prepared, &args, &mut buffer))
                .map_err(|e| format!("execute: {e:?}"))?;
            Ok(buffer.len() as u64)
        })
    })?;
    let answers = ours.work / u64::from(proto.samples.max(1));

    // Theirs: the canonical twin under the cap; ours stats are kept
    // either way. Every timed block's stamp merges into the point's one
    // verdict — contamination of any block dirties the point.
    let theirs = time_lane(conn, cap, &bundle.canonical, &bundle.draws, &types, proto)?;
    let theirs = theirs.map(|(stats, stamp)| {
        ghz = ghz.merge(stamp);
        stats
    });
    let mut cap_event = if theirs.is_none() {
        Some(CapEvent { at: "timing" })
    } else {
        None
    };

    // The hand twin (busy_scan): gated exactly like the canonical
    // before it is timed; a cap here leaves the canonical results
    // intact (the first cap event wins the one report slot).
    let mut theirs_hand = None;
    if let Some(hand) = &bundle.hand {
        let hand_label = format!("{name}[hand]");
        let hand_gate = cap.guarded(conn, || {
            gate_lane(
                conn,
                &hand_label,
                hand,
                &bundle.draws,
                &ours_answers,
                &types,
            )
        })?;
        if hand_gate.is_some() {
            theirs_hand =
                time_lane(conn, cap, hand, &bundle.draws, &types, proto)?.map(|(stats, stamp)| {
                    ghz = ghz.merge(stamp);
                    stats
                });
        }
        if theirs_hand.is_none() {
            cap_event = cap_event.or(Some(CapEvent { at: "hand" }));
        }
    }

    Ok(CurvePoint {
        scale: scale_label,
        facts: bundle.facts,
        answers,
        ours: Some(ours.stats),
        theirs,
        theirs_hand,
        cap: cap_event,
        ghz: Some(ghz.into()),
    })
}

// ---------------------------------------------------------------------
// The warmth panel: {cold, warm, memoized}, both engines.
// ---------------------------------------------------------------------

/// Reopen-cold rounds discarded before recording begins.
const WARMTH_DISCARDED: usize = 2;
/// Reopen-cold rounds recorded.
const WARMTH_ROUNDS: usize = 16;
/// The memoized point's protocol.
const MEMO_PROTOCOL: Protocol = Protocol {
    warmups: 8,
    samples: 64,
};

fn elapsed_ns(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

/// Opens the engine store, absorbing the transient typed
/// `EnvironmentLocked` window — the engine's own drop-order test
/// (`dropping_the_handle_never_leaks_an_env_already_opened_window`)
/// documents the retry loop keyed on the typed error as the sanctioned
/// reopen pattern. The window is real under a parallel test suite:
/// dropping a handle releases the advisory flock synchronously, but a
/// concurrently forked child (the device-honesty probes spawn `mount`/
/// `hdiutil`) holds the inherited lock fd through its fork→exec
/// window. Bounded; every other error propagates. Retrying happens
/// strictly BEFORE any timed region — cold timing starts at the first
/// execute, never at open.
fn open_absorbing_lock_window<S: bumbledb::Theory + Copy>(
    path: &Path,
    theory: S,
) -> Result<Db<S>, String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match Db::open(path, theory) {
            Ok(db) => return Ok(db),
            Err(bumbledb::Error::EnvironmentLocked) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(e) => return Err(format!("open {}: {e:?}", path.display())),
        }
    }
}

/// The three-point warmth panel over one already-gated (family, scale)
/// store pair, both engines symmetric — nobody can call it one-sided.
/// Per reopen-cold round: drop and reopen the store, prepare (excluded
/// from the timed region — the timed-region law), time exactly the
/// first execution (`cold`), then the second execution of the same
/// prepared statement (`warm`); `memoized` is one open + one prepare
/// under [`MEMO_PROTOCOL`]. What the engine side prices is the
/// (relation, generation) image cache and the resolved-filter view
/// slots (docs/architecture/50-storage.md). Reopens commit nothing —
/// the corpus cache stays read-only.
fn warmth_panel<S: bumbledb::Theory + Copy>(
    theory: S,
    db_path: &Path,
    oracle_path: &Path,
    bundle: &Bundle,
) -> Result<Warmth, String> {
    let open_db =
        || open_absorbing_lock_window(db_path, theory).map_err(|e| format!("warmth reopen: {e}"));
    let types: Vec<ValueType> = {
        let db = open_db()?;
        let prepared = db
            .prepare(&bundle.program)
            .map_err(|e| format!("warmth prepare: {e:?}"))?;
        prepared
            .predicate()
            .columns
            .iter()
            .map(|column| column.ty.clone())
            .collect()
    };

    // Ours, reopen-cold rounds: exec1 = cold, exec2 = warm.
    let mut cold = Vec::with_capacity(WARMTH_ROUNDS);
    let mut warm = Vec::with_capacity(WARMTH_ROUNDS);
    for round in 0..(WARMTH_DISCARDED + WARMTH_ROUNDS) {
        let draw = &bundle.draws[round % bundle.draws.len()];
        let args = param_args(draw);
        let db = open_db()?;
        let mut prepared = db
            .prepare(&bundle.program)
            .map_err(|e| format!("warmth prepare: {e:?}"))?;
        let mut buffer = Answers::new();
        let start = Instant::now();
        db.read(|snap| snap.execute_args(&mut prepared, &args, &mut buffer))
            .map_err(|e| format!("warmth execute: {e:?}"))?;
        let first = elapsed_ns(start);
        std::hint::black_box(buffer.len());
        let start = Instant::now();
        db.read(|snap| snap.execute_args(&mut prepared, &args, &mut buffer))
            .map_err(|e| format!("warmth execute: {e:?}"))?;
        let second = elapsed_ns(start);
        std::hint::black_box(buffer.len());
        if round >= WARMTH_DISCARDED {
            cold.push(first);
            warm.push(second);
        }
    }
    let ours_cold = harness::stats(&mut cold);
    let ours_warm = harness::stats(&mut warm);

    // Ours, memoized: one open + one prepare, the measured protocol.
    let ours_memoized = {
        let db = open_db()?;
        let mut prepared = db
            .prepare(&bundle.program)
            .map_err(|e| format!("warmth prepare: {e:?}"))?;
        let mut rotation = Rotation::new(bundle.draws.clone());
        let mut buffer = Answers::new();
        let measured = harness::measure(MEMO_PROTOCOL, || {
            let args = param_args(rotation.next_set());
            db.read(|snap| snap.execute_args(&mut prepared, &args, &mut buffer))
                .map_err(|e| format!("execute: {e:?}"))?;
            Ok(buffer.len() as u64)
        })?;
        measured.stats
    };

    // Theirs, symmetric: per round drop the connection, reopen, prepare
    // the canonical SQL, time exec1/exec2 (full drain through the
    // PreparedFamily walk).
    let mut cold = Vec::with_capacity(WARMTH_ROUNDS);
    let mut warm = Vec::with_capacity(WARMTH_ROUNDS);
    for round in 0..(WARMTH_DISCARDED + WARMTH_ROUNDS) {
        let draw = &bundle.draws[round % bundle.draws.len()];
        let conn = open_for_bench(oracle_path).map_err(|e| format!("warmth oracle open: {e}"))?;
        let mut family = PreparedFamily::new(&conn, &bundle.canonical, types.clone())?;
        let start = Instant::now();
        sqlite_run::sample_args(&mut family, draw)?;
        let first = elapsed_ns(start);
        let start = Instant::now();
        sqlite_run::sample_args(&mut family, draw)?;
        let second = elapsed_ns(start);
        if round >= WARMTH_DISCARDED {
            cold.push(first);
            warm.push(second);
        }
    }
    let theirs_cold = harness::stats(&mut cold);
    let theirs_warm = harness::stats(&mut warm);

    // Theirs, memoized: one reused statement under the same protocol.
    let theirs_memoized = {
        let conn = open_for_bench(oracle_path).map_err(|e| format!("warmth oracle open: {e}"))?;
        let mut family = PreparedFamily::new(&conn, &bundle.canonical, types)?;
        let mut rotation = Rotation::new((0..bundle.draws.len()).collect::<Vec<_>>());
        let measured = harness::measure(MEMO_PROTOCOL, || {
            let index = rotation.next_index();
            sqlite_run::sample_args(&mut family, &bundle.draws[index])
        })?;
        measured.stats
    };

    Ok(Warmth {
        ours_cold,
        ours_warm,
        ours_memoized,
        theirs_cold,
        theirs_warm,
        theirs_memoized,
        ghz: None,
    })
}

/// [`warmth_panel`] under one non-retrying proxy bracket (reopen rounds
/// are not idempotent): the stamp annotates the whole panel.
fn warmth_panel_stamped<S: bumbledb::Theory + Copy>(
    theory: S,
    db_path: &Path,
    oracle_path: &Path,
    bundle: &Bundle,
) -> Result<Warmth, String> {
    let (mut warmth, ghz) =
        clockproxy::stamped(|| warmth_panel(theory, db_path, oracle_path, bundle))?;
    warmth.ghz = Some(ghz.into());
    Ok(warmth)
}

// ---------------------------------------------------------------------
// The lane driver.
// ---------------------------------------------------------------------

/// One world's open stores at one scale.
struct WorldStores<S> {
    db: Db<S>,
    conn: rusqlite::Connection,
    db_path: PathBuf,
    oracle_path: PathBuf,
}

impl<S> WorldStores<S> {
    /// Drops the live handles and keeps the paths — the warmth panel's
    /// reopen rounds need the store CLOSED first (one LMDB environment
    /// per store per process).
    fn into_paths(self) -> (PathBuf, PathBuf) {
        let Self {
            db,
            conn,
            db_path,
            oracle_path,
        } = self;
        drop(db);
        drop(conn);
        (db_path, oracle_path)
    }
}

/// The run-long context every scale pass shares.
struct LaneCtx<'a> {
    args: &'a crate::cli::CurvesArgs,
    selected: Vec<&'static CurveFamily>,
    proto: Protocol,
    cap: DnfCap,
    scratch: PathBuf,
}

/// One scale pass: open exactly the worlds the selected families need
/// (the ledger/calendar pair from the digest-keyed cache, read-only,
/// fairness asserted before timing; the closure world into scratch
/// through the sized loader), then gate-and-time every selected family,
/// then — at the first scale under `--warmth` — drop the live handles
/// and run the panel on the already-gated stores.
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)]
fn run_scale(
    ctx: &LaneCtx<'_>,
    curves: &mut [FamilyCurve],
    scale: Scale,
    first: bool,
) -> Result<(), String> {
    let cfg = GenConfig {
        seed: ctx.args.seed,
        scale,
    };
    let bundles: Vec<Bundle> = ctx
        .selected
        .iter()
        .map(|family| bundle_for(family, &cfg))
        .collect::<Result<_, _>>()?;

    let needs = |world: World| ctx.selected.iter().any(|family| family.world == world);
    let paths = if needs(World::Ledger) || needs(World::Calendar) {
        Some(crate::driver::ensure_corpus(&ctx.args.dir, cfg)?)
    } else {
        None
    };

    let ledger = if needs(World::Ledger) {
        let paths = paths.as_ref().expect("corpus ensured above");
        let db = open_absorbing_lock_window(&paths.db, crate::schema::Ledger)
            .map_err(|e| format!("open ledger store: {e}"))?;
        let conn = open_for_bench(&paths.oracle).map_err(|e| format!("open ledger oracle: {e}"))?;
        FairnessCheck::run(&conn)?;
        Some(WorldStores {
            db,
            conn,
            db_path: paths.db.clone(),
            oracle_path: paths.oracle.clone(),
        })
    } else {
        None
    };

    let calendar = if needs(World::Calendar) {
        let paths = paths.as_ref().expect("corpus ensured above");
        let db = open_absorbing_lock_window(&paths.cal_db, crate::calendar::Scheduling)
            .map_err(|e| format!("open calendar store: {e}"))?;
        let conn =
            open_for_bench(&paths.cal_oracle).map_err(|e| format!("open calendar oracle: {e}"))?;
        FairnessCheck::run_calendar(&conn)?;
        Some(WorldStores {
            db,
            conn,
            db_path: paths.cal_db.clone(),
            oracle_path: paths.cal_oracle.clone(),
        })
    } else {
        None
    };

    let closure_world = if needs(World::Closure) {
        let dir = ctx.scratch.join(format!("closure-{}", scale.label()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).map_err(|e| format!("closure scratch: {e}"))?;
        eprintln!("curves: loading the closure world at {}", scale.label());
        let (db, conn) = closure::load_stores_sized(
            &dir,
            curve_sizes(scale),
            crate::storemode::StoreMode::default(),
        )?;
        Some(WorldStores {
            db,
            conn,
            db_path: dir.join("db"),
            oracle_path: dir.join("oracle.sqlite"),
        })
    } else {
        None
    };

    // The DVFS ramp eater (the driver/bench.rs precedent): the world
    // loads above end in fsync-heavy commits that drop the core to its
    // DVFS floor — eat the ramp before the first timed block.
    clockproxy::warm_up(std::time::Duration::from_millis(200));

    for ((family, bundle), curve) in ctx.selected.iter().zip(&bundles).zip(curves.iter_mut()) {
        let point = match family.world {
            World::Ledger => {
                let world = ledger.as_ref().expect("ledger world open");
                curve_point(
                    family.name,
                    scale.label(),
                    &world.db,
                    &world.conn,
                    bundle,
                    ctx.proto,
                    ctx.cap,
                )?
            }
            World::Calendar => {
                let world = calendar.as_ref().expect("calendar world open");
                curve_point(
                    family.name,
                    scale.label(),
                    &world.db,
                    &world.conn,
                    bundle,
                    ctx.proto,
                    ctx.cap,
                )?
            }
            World::Closure => {
                let world = closure_world.as_ref().expect("closure world open");
                curve_point(
                    family.name,
                    scale.label(),
                    &world.db,
                    &world.conn,
                    bundle,
                    ctx.proto,
                    ctx.cap,
                )?
            }
        };
        curve.rows.push(point);
    }

    if first && ctx.args.warmth {
        let ledger = ledger.map(WorldStores::into_paths);
        let calendar = calendar.map(WorldStores::into_paths);
        let closure_paths = closure_world.map(WorldStores::into_paths);
        for ((family, bundle), curve) in ctx.selected.iter().zip(&bundles).zip(curves.iter_mut()) {
            // Only stores gated at this scale this run enter the panel:
            // a capped gate left `ours` empty, and an unverified store
            // is never measured.
            if curve.rows.last().is_none_or(|point| point.ours.is_none()) {
                continue;
            }
            let warmth = match family.world {
                World::Ledger => {
                    let (db_path, oracle_path) = ledger.as_ref().expect("ledger world open");
                    warmth_panel_stamped(crate::schema::Ledger, db_path, oracle_path, bundle)?
                }
                World::Calendar => {
                    let (db_path, oracle_path) = calendar.as_ref().expect("calendar world open");
                    warmth_panel_stamped(crate::calendar::Scheduling, db_path, oracle_path, bundle)?
                }
                World::Closure => {
                    let (db_path, oracle_path) =
                        closure_paths.as_ref().expect("closure world open");
                    warmth_panel_stamped(closure::Reachability, db_path, oracle_path, bundle)?
                }
            };
            curve.warmth = Some(warmth);
        }
    }
    Ok(())
}

fn opt_p50(stats: Option<&Stats>) -> String {
    stats.map_or_else(|| "—".to_owned(), |s| s.p50.to_string())
}

/// The human-readable table — hand-rolled, like `scenarios/render.rs`.
fn render(report: &CurvesReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Curves report\n");
    let _ = writeln!(
        out,
        "Scale curves, report-class. Every point is oracle-gated inline \
         (value-identical multiset agreement against `SQLite`) before either \
         engine is timed; a capped `SQLite` region is excluded-and-counted \
         (`cap` names where it fired). `busy_scan` carries the hand-tuned \
         twin beside the canonical OR-chain — both reported. p50 in ns; \
         seed {}, {} samples per point, cap {} ms per region.\n",
        report.seed, report.samples, report.cap_ms
    );
    let _ = writeln!(
        out,
        "| family | world | scale | facts | answers | ours p50 | sqlite p50 | hand p50 | cap |"
    );
    let _ = writeln!(out, "|---|---|---|---:|---:|---:|---:|---:|---|");
    let mut capped = 0usize;
    for family in &report.families {
        for point in &family.rows {
            if point.cap.is_some() {
                capped += 1;
            }
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                family.name,
                family.world,
                point.scale,
                point.facts,
                point.answers,
                opt_p50(point.ours.as_ref()),
                opt_p50(point.theirs.as_ref()),
                opt_p50(point.theirs_hand.as_ref()),
                point.cap.map_or("—", |cap| cap.at),
            );
        }
    }
    let _ = writeln!(out, "\ncapped points: {capped} (excluded-and-counted)");
    if report.families.iter().any(|family| family.warmth.is_some()) {
        let _ = writeln!(
            out,
            "\n## Warmth panel (cold/warm/memoized, p50 ns)\n\n\
             Reopen-cold is process-fresh but OS-page-cache-warm — as close \
             as the harness allows. The engine side prices the (relation, \
             generation) image cache and the resolved-filter view slots.\n"
        );
        let _ = writeln!(
            out,
            "| family | engine | cold | warm | memoized |\n|---|---|---:|---:|---:|"
        );
        for family in &report.families {
            if let Some(w) = &family.warmth {
                let _ = writeln!(
                    out,
                    "| {} | bumbledb | {} | {} | {} |",
                    family.name, w.ours_cold.p50, w.ours_warm.p50, w.ours_memoized.p50
                );
                let _ = writeln!(
                    out,
                    "| {} | sqlite | {} | {} | {} |",
                    family.name, w.theirs_cold.p50, w.theirs_warm.p50, w.theirs_memoized.p50
                );
            }
        }
    }
    out
}

/// The curves lane entry point: per scale in order, gate-and-time every
/// selected family (the gate dominates the timing — see the module
/// doc), assemble the report, write `curves-report.json` +
/// `curves-report.md`, print the markdown, and remove the scratch.
///
/// # Errors
///
/// Refusals (RAM-backed `--dir`, unknown `--families` names), corpus or
/// world setup failures, an oracle disagreement (`ENGINES DISAGREE` —
/// nothing is timed), and engine or `SQLite` errors — all as messages.
pub fn run(args: &crate::cli::CurvesArgs) -> Result<i32, String> {
    let selected = select(args.families.as_deref())?;
    if args.scales.is_empty() {
        return Err("curves: --scales named no scale".to_owned());
    }
    crate::devhonesty::assert_disk_backed(&args.dir, "the timed curve families")
        .map_err(|refusal| refusal.to_string())?;
    let proto = Protocol {
        warmups: 8,
        samples: args.samples.unwrap_or(64),
    };
    let out_dir = args.out.clone().unwrap_or_else(|| {
        PathBuf::from("bench-out").join(format!(
            "{}-curves",
            report::timestamp_iso8601().replace(':', "-")
        ))
    });
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("out {}: {e}", out_dir.display()))?;
    let ctx = LaneCtx {
        args,
        selected,
        proto,
        cap: DnfCap {
            cap: Duration::from_millis(args.cap_ms),
        },
        scratch: out_dir.join("scratch"),
    };

    let mut curves: Vec<FamilyCurve> = ctx
        .selected
        .iter()
        .map(|family| FamilyCurve {
            name: family.name,
            world: family.world.label(),
            rows: Vec::new(),
            warmth: None,
        })
        .collect();

    for (index, scale) in args.scales.iter().enumerate() {
        run_scale(&ctx, &mut curves, *scale, index == 0)?;
    }

    let report = CurvesReport {
        provenance: report::provenance(Path::new(".")),
        seed: args.seed,
        samples: proto.samples,
        cap_ms: args.cap_ms,
        families: curves,
    };
    std::fs::write(out_dir.join("curves-report.json"), to_json(&report))
        .map_err(|e| format!("artifact: {e}"))?;
    let markdown = render(&report);
    std::fs::write(out_dir.join("curves-report.md"), &markdown)
        .map_err(|e| format!("artifact: {e}"))?;
    print!("{markdown}");
    println!("artifacts: {}", out_dir.display());
    if ctx.scratch.exists() {
        std::fs::remove_dir_all(&ctx.scratch).map_err(|e| format!("scratch cleanup: {e}"))?;
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::Value as Json;

    fn provenance() -> Provenance {
        Provenance {
            crate_version: "0.0.0-test".to_owned(),
            git_rev: "deadbeef".to_owned(),
            timestamp: "2026-07-19T00:00:00Z".to_owned(),
            host: "test-host".to_owned(),
            shared: None,
        }
    }

    fn stats(base: u64) -> Stats {
        Stats {
            min: base,
            p50: base + 1,
            p90: base + 2,
            p95: base + 3,
            p99: base + 4,
            max: base + 5,
            mean_ns: base + 2,
        }
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bumbledb-bench-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    fn tiny_args(dir: &Path, out: &Path) -> crate::cli::CurvesArgs {
        crate::cli::CurvesArgs {
            scales: vec![Scale::Tiny],
            families: None,
            seed: 1,
            dir: dir.to_path_buf(),
            samples: Some(4),
            cap_ms: 30_000,
            warmth: false,
            out: Some(out.to_path_buf()),
        }
    }

    #[test]
    fn report_json_shape_is_pinned() {
        let report = CurvesReport {
            provenance: provenance(),
            seed: 3,
            samples: 16,
            cap_ms: 5000,
            families: vec![FamilyCurve {
                name: "triangle",
                world: "graph",
                rows: vec![
                    CurvePoint {
                        scale: "S",
                        facts: 100_000,
                        answers: 42,
                        ours: Some(stats(100)),
                        theirs: Some(stats(200)),
                        theirs_hand: None,
                        cap: None,
                        ghz: Some(GhzReport {
                            pre: 3.5,
                            post: 3.4,
                            retried: false,
                            contaminated: false,
                        }),
                    },
                    CurvePoint {
                        scale: "M",
                        facts: 1_000_000,
                        answers: 420,
                        ours: Some(stats(300)),
                        theirs: None,
                        theirs_hand: None,
                        cap: Some(CapEvent { at: "timing" }),
                        ghz: None,
                    },
                ],
                warmth: Some(Warmth {
                    ours_cold: stats(10),
                    ours_warm: stats(20),
                    ours_memoized: stats(30),
                    theirs_cold: stats(40),
                    theirs_warm: stats(50),
                    theirs_memoized: stats(60),
                    ghz: None,
                }),
            }],
        };
        let parsed = crate::json::parse(&to_json(&report)).expect("valid JSON");
        let provenance = parsed.get("provenance").expect("provenance");
        assert_eq!(
            provenance.get("timestamp").and_then(Json::as_str),
            Some("2026-07-19T00:00:00Z")
        );
        // Boost-off keeps the pre-boost provenance shape.
        assert!(provenance.get("shared_machine").is_none());
        assert_eq!(parsed.get("seed").and_then(Json::as_f64), Some(3.0));
        assert_eq!(parsed.get("samples").and_then(Json::as_f64), Some(16.0));
        assert_eq!(parsed.get("cap_ms").and_then(Json::as_f64), Some(5000.0));
        let families = parsed
            .get("families")
            .and_then(Json::as_arr)
            .expect("families");
        assert_eq!(families.len(), 1);
        assert_eq!(
            families[0].get("name").and_then(Json::as_str),
            Some("triangle")
        );
        assert_eq!(
            families[0].get("world").and_then(Json::as_str),
            Some("graph")
        );
        let rows = families[0]
            .get("rows")
            .and_then(Json::as_arr)
            .expect("rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get("scale").and_then(Json::as_str), Some("S"));
        assert_eq!(rows[0].get("facts").and_then(Json::as_f64), Some(100_000.0));
        assert_eq!(rows[0].get("answers").and_then(Json::as_f64), Some(42.0));
        let ours = rows[0].get("ours").expect("ours");
        assert_eq!(ours.get("p50").and_then(Json::as_f64), Some(101.0));
        let theirs = rows[0].get("theirs").expect("theirs");
        assert_eq!(theirs.get("max").and_then(Json::as_f64), Some(205.0));
        assert_eq!(rows[0].get("theirs_hand"), Some(&Json::Null));
        assert_eq!(rows[0].get("cap"), Some(&Json::Null));
        // The contamination discriminator rides every point (finding 072).
        let ghz = rows[0].get("ghz").expect("ghz");
        assert_eq!(ghz.get("pre").and_then(Json::as_f64), Some(3.5));
        assert_eq!(ghz.get("contaminated").and_then(Json::as_bool), Some(false));
        assert_eq!(rows[1].get("ghz"), Some(&Json::Null));
        // The capped point: theirs is null and the cap event says where.
        assert_eq!(rows[1].get("theirs"), Some(&Json::Null));
        assert_eq!(
            rows[1]
                .get("cap")
                .and_then(|c| c.get("at"))
                .and_then(Json::as_str),
            Some("timing")
        );
        let warmth = families[0].get("warmth").expect("warmth");
        assert_eq!(
            warmth
                .get("ours_cold")
                .and_then(|s| s.get("min"))
                .and_then(Json::as_f64),
            Some(10.0)
        );
        assert_eq!(
            warmth
                .get("theirs_memoized")
                .and_then(|s| s.get("mean_ns"))
                .and_then(Json::as_f64),
            Some(62.0)
        );
    }

    /// The whole lane at Tiny: all four families gated then timed, one
    /// point each, no cap events, the hand twin timed for `busy_scan`.
    #[test]
    fn tiny_scale_gates_then_reports() {
        let dir = scratch("curves-tiny-e2e");
        let out = dir.join("out");
        let code = run(&tiny_args(&dir, &out)).expect("the lane runs");
        assert_eq!(code, 0);
        let text = std::fs::read_to_string(out.join("curves-report.json")).expect("json artifact");
        let parsed = crate::json::parse(&text).expect("valid JSON");
        let families = parsed
            .get("families")
            .and_then(Json::as_arr)
            .expect("families");
        assert_eq!(families.len(), 4, "the full roster");
        for family in families {
            let name = family.get("name").and_then(Json::as_str).expect("name");
            let rows = family.get("rows").and_then(Json::as_arr).expect("rows");
            assert_eq!(rows.len(), 1, "{name}: one scale, one point");
            let row = &rows[0];
            assert_eq!(row.get("scale").and_then(Json::as_str), Some("Tiny"));
            assert!(
                row.get("ours")
                    .and_then(|s| s.get("p50"))
                    .and_then(Json::as_f64)
                    .is_some(),
                "{name}: ours timed"
            );
            assert!(
                row.get("theirs")
                    .and_then(|s| s.get("p50"))
                    .and_then(Json::as_f64)
                    .is_some(),
                "{name}: theirs timed"
            );
            assert_eq!(row.get("cap"), Some(&Json::Null), "{name}: no cap event");
            if name == "busy_scan" {
                assert!(
                    row.get("theirs_hand")
                        .and_then(|s| s.get("p50"))
                        .and_then(Json::as_f64)
                        .is_some(),
                    "the hand twin gated and timed at Tiny"
                );
            }
        }
        assert!(!out.join("scratch").exists(), "scratch removed");
        assert!(out.join("curves-report.md").exists(), "markdown artifact");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A zero cap is the degenerate budget: the gate region is
    /// excluded before entry, and an unverified point is never timed
    /// on either side.
    #[test]
    fn zero_cap_reports_exceeded_and_skips_timing() {
        let dir = scratch("curves-zero-cap");
        let out = dir.join("out");
        let mut args = tiny_args(&dir, &out);
        args.families = Some(vec!["busy_scan".to_owned()]);
        args.cap_ms = 0;
        let code = run(&args).expect("the lane runs");
        assert_eq!(code, 0);
        let text = std::fs::read_to_string(out.join("curves-report.json")).expect("json artifact");
        let parsed = crate::json::parse(&text).expect("valid JSON");
        let families = parsed
            .get("families")
            .and_then(Json::as_arr)
            .expect("families");
        assert_eq!(families.len(), 1);
        let row = &families[0]
            .get("rows")
            .and_then(Json::as_arr)
            .expect("rows")[0];
        assert_eq!(
            row.get("cap")
                .and_then(|c| c.get("at"))
                .and_then(Json::as_str),
            Some("gate"),
            "the cap fired at the gate"
        );
        assert_eq!(row.get("ours"), Some(&Json::Null), "never timed unverified");
        assert_eq!(
            row.get("theirs"),
            Some(&Json::Null),
            "never timed unverified"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The warmth panel lands all six stats, both engines, at the
    /// first (and only) scale.
    #[test]
    fn warmth_panel_reports_cold_warm_memoized() {
        let dir = scratch("curves-warmth");
        let out = dir.join("out");
        let mut args = tiny_args(&dir, &out);
        args.families = Some(vec!["point".to_owned()]);
        args.warmth = true;
        let code = run(&args).expect("the lane runs");
        assert_eq!(code, 0);
        let text = std::fs::read_to_string(out.join("curves-report.json")).expect("json artifact");
        let parsed = crate::json::parse(&text).expect("valid JSON");
        let families = parsed
            .get("families")
            .and_then(Json::as_arr)
            .expect("families");
        assert_eq!(families.len(), 1);
        let warmth = families[0].get("warmth").expect("warmth present");
        assert_ne!(warmth, &Json::Null, "warmth object present");
        for field in [
            "ours_cold",
            "ours_warm",
            "ours_memoized",
            "theirs_cold",
            "theirs_warm",
            "theirs_memoized",
        ] {
            let min = warmth
                .get(field)
                .and_then(|s| s.get("min"))
                .and_then(Json::as_f64)
                .expect(field);
            assert!(min > 0.0, "{field}: min must be positive, got {min}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The lane-local ladder pin: ~10x edge growth per step, and the
    /// Tiny arm equals the closure lane's own fuzz point.
    #[test]
    fn closure_curve_sizes_grow_tenfold_ish() {
        assert_eq!(curve_sizes(Scale::Tiny), ClosSizes::of(Scale::Tiny));
        let s = curve_sizes(Scale::S).edges();
        let m = curve_sizes(Scale::M).edges();
        let l = curve_sizes(Scale::L).edges();
        assert!(m / s >= 8, "S→M edges grew {m}/{s}");
        assert!(l / m >= 8, "M→L edges grew {l}/{m}");
    }

    /// The gate path really compares — by construction: a deliberately
    /// WRONG hand SQL (the unfiltered `Claim` scan, which agrees with
    /// the engine only over an empty corpus) must land the disagree
    /// refusal, proving the hand twin cannot reach a timer unverified.
    #[test]
    fn hand_twin_is_gated_before_timing() {
        let dir = scratch("curves-hand-gate");
        let cfg = GenConfig {
            seed: 1,
            scale: Scale::Tiny,
        };
        let paths = crate::driver::ensure_corpus(&dir, cfg).expect("corpus");
        let db = open_absorbing_lock_window(&paths.cal_db, crate::calendar::Scheduling)
            .expect("open cal store");
        let conn = open_for_bench(&paths.cal_oracle).expect("open cal oracle");
        let bundle = calendar_bundle("busy_scan", &cfg).expect("bundle");
        let mut prepared = db.prepare(&bundle.program).expect("prepare");
        let types: Vec<ValueType> = prepared
            .predicate()
            .columns
            .iter()
            .map(|column| column.ty.clone())
            .collect();
        let mut buffer = Answers::new();
        let mut ours = Vec::new();
        for draw in &bundle.draws {
            let args = param_args(draw);
            db.read(|snap| snap.execute_args(&mut prepared, &args, &mut buffer))
                .map_err(|e| format!("{e:?}"))
                .expect("execute");
            ours.push(compare::from_answers(&buffer, &types));
        }
        let wrong = Translated {
            sql: "SELECT DISTINCT t0.\"person\", t0.\"span_start\", t0.\"span_end\" \
                  FROM \"Claim\" AS t0 WHERE ?1 = ?1 AND ?2 = ?2"
                .to_owned(),
            params: BUSY_SCAN_HAND_SLOTS.to_vec(),
        };
        let err = gate_lane(
            &conn,
            "busy_scan[hand]",
            &wrong,
            &bundle.draws,
            &ours,
            &types,
        )
        .expect_err("the wrong twin must be refused");
        assert!(err.contains("ENGINES DISAGREE"), "{err}");
        // The genuine hand twin passes the same gate.
        let hand = bundle.hand.as_ref().expect("busy_scan carries the twin");
        gate_lane(&conn, "busy_scan[hand]", hand, &bundle.draws, &ours, &types)
            .expect("the real twin agrees");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
