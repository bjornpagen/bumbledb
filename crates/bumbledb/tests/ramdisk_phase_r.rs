//! The ramdisk phase-R measurement harness — the executable record of
//! the numbers the ephemeral-store decision stands on. Ignored — it
//! attaches real ram disks and runs minutes of timed commits; run it
//! manually on the pinned M2 Max:
//!
//! ```sh
//! cargo test -p bumbledb --release --test ramdisk_phase_r -- --ignored --nocapture
//! ```
//!
//! Placement (a recorded decision): this lives as an engine integration
//! test, not a bench-crate bin, because R4 opens scratch heed
//! environments with the flags the engine forbids — and the bench
//! crate's dependency quarantine (`docs/architecture/00-product.md`) is
//! rusqlite and nothing else. `heed` as an engine dev-dependency adds no
//! node to the dependency graph.
//!
//! Teardown law: every ram disk this harness attaches is detached by a
//! drop guard, on the failure paths included (a panic unwinds through
//! [`RamDisk::drop`]). After a run, `hdiutil info` must show none of
//! ours.

#![cfg(target_os = "macos")]

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use bumbledb::Db;

bumbledb::schema! {
    pub Meter;

    relation Sample {
        id: u64 as SampleId, fresh,
        bucket: u64,
        amount: i64,
    }
}

// ---------------------------------------------------------------------
// Ram-disk lifecycle
// ---------------------------------------------------------------------

/// One attached ram disk, formatted and mounted. Dropping detaches it —
/// escalating to `-force` rather than leaking wired memory.
struct RamDisk {
    dev: String,
    mount: PathBuf,
}

/// 2 GiB in 512-byte sectors — `ram://4194304`.
const RAM_SECTORS: u64 = 4_194_304;

fn run(cmd: &str, args: &[&str]) -> String {
    let out = Command::new(cmd)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn {cmd}: {e}"));
    assert!(
        out.status.success(),
        "{cmd} {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("utf-8 tool output")
}

impl RamDisk {
    /// Attaches a fresh 2 GiB ram device and formats it with the given
    /// diskutil personality (`"HFS+"` is non-journaled by name — the
    /// journaled personality is spelled `"Journaled HFS+"`; `"APFS"`
    /// creates a synthesized container). `diskutil erasevolume` mounts
    /// the volume at `/Volumes/<label>`.
    fn attach(personality: &str, label: &str) -> Self {
        Self::attach_sized(personality, label, RAM_SECTORS)
    }

    /// [`RamDisk::attach`] with an explicit sector count. R6 needs it:
    /// an EPHEMERAL store's `MDB_WRITEMAP` ftruncates the data file to
    /// the full 4 GiB ephemeral map (`MAP_SIZE_EPHEMERAL` — the
    /// per-kind split keeps the scratch kind's map small) at open, and
    /// HFS+ has no sparse files, so the disk must hold the whole map
    /// plus slack (a recorded consequence — `50-storage.md` § the
    /// ephemeral store kind).
    fn attach_sized(personality: &str, label: &str, sectors: u64) -> Self {
        let dev = run(
            "hdiutil",
            &["attach", "-nomount", &format!("ram://{sectors}")],
        )
        .trim()
        .to_owned();
        let disk = Self {
            dev,
            mount: PathBuf::from(format!("/Volumes/{label}")),
        };
        // A failed format still detaches: the guard is already armed.
        let status = Command::new("diskutil")
            .args(["erasevolume", personality, label, &disk.dev])
            .status()
            .expect("spawn diskutil");
        assert!(
            status.success(),
            "diskutil erasevolume {personality} failed"
        );
        disk
    }

    fn store_dir(&self, tag: &str) -> PathBuf {
        let dir = self.mount.join(tag);
        std::fs::create_dir_all(&dir).expect("create store dir on ram disk");
        dir
    }
}

impl Drop for RamDisk {
    fn drop(&mut self) {
        let plain = Command::new("hdiutil").args(["detach", &self.dev]).status();
        if !plain.is_ok_and(|s| s.success()) {
            let forced = Command::new("hdiutil")
                .args(["detach", "-force", &self.dev])
                .status();
            assert!(
                forced.is_ok_and(|s| s.success()),
                "ram disk {} could not be detached — detach it by hand",
                self.dev
            );
        }
    }
}

// ---------------------------------------------------------------------
// Timing scaffolding
// ---------------------------------------------------------------------

fn median(mut samples: Vec<Duration>) -> Duration {
    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn spread(samples: &[Duration]) -> (Duration, Duration) {
    let min = samples.iter().min().copied().expect("nonempty");
    let max = samples.iter().max().copied().expect("nonempty");
    (min, max)
}

fn ms(d: Duration) -> String {
    format!("{:.3}", d.as_secs_f64() * 1e3)
}

fn cell(samples: &[Duration]) -> String {
    let (min, max) = spread(samples);
    format!(
        "{} ms (min {}, max {}, n={})",
        ms(median(samples.to_vec())),
        ms(min),
        ms(max),
        samples.len()
    )
}

/// One timed committing write of `facts` inserts through the engine's
/// typed insert path; `bucket` persists across calls so interleaved
/// cells (R6) keep their own running payloads.
fn one_engine_commit(db: &Db<Meter>, facts: u32, bucket: &mut u64) -> Duration {
    let start = Instant::now();
    db.write(|tx| {
        for _ in 0..facts {
            let id: SampleId = tx.alloc()?;
            *bucket += 1;
            tx.insert(&Sample {
                id,
                bucket: *bucket,
                amount: 7,
            })?;
        }
        Ok(())
    })
    .expect("committing write succeeds");
    start.elapsed()
}

/// One timed committing write of `facts` inserts, repeated `commits`
/// times; returns per-commit wall times.
fn timed_commits(db: &Db<Meter>, commits: u32, facts: u32) -> Vec<Duration> {
    let mut samples = Vec::with_capacity(commits as usize);
    let mut bucket = 0u64;
    for _ in 0..commits {
        samples.push(one_engine_commit(db, facts, &mut bucket));
    }
    samples
}

/// The two write shapes the campaign prices: the small transaction and
/// the bulk-load chunk (`Db::bulk_load`'s 4096-fact commit, expressed
/// through the same write path so the shapes differ only in size).
const SMALL_FACTS: u32 = 16;
const SMALL_COMMITS: u32 = 64;
const BULK_FACTS: u32 = 4096;
const BULK_COMMITS: u32 = 8;

struct EngineCells {
    small: Vec<Duration>,
    bulk: Vec<Duration>,
}

fn engine_cells(dir: &Path) -> EngineCells {
    let db = Db::create(dir, Meter).expect("store creates");
    EngineCells {
        small: timed_commits(&db, SMALL_COMMITS, SMALL_FACTS),
        bulk: timed_commits(&db, BULK_COMMITS, BULK_FACTS),
    }
}

// ---------------------------------------------------------------------
// R4: the scratch heed environment (flag experiment — never the engine's)
// ---------------------------------------------------------------------

/// Opens a scratch LMDB environment with the given extra flags. This is
/// the bench-side experiment the plan names: the engine's environment
/// never carries these flags (its no-sync-mode law is
/// `docs/architecture/50-storage.md`); the scratch env exists to price
/// them on a ram disk and dies with the run.
#[expect(
    unsafe_code,
    reason = "heed marks flag-setting and open unsafe; the scratch environment is single-threaded, single-open, measurement-only"
)]
fn scratch_env(dir: &Path, flags: heed::EnvFlags) -> heed::Env {
    let mut options = heed::EnvOpenOptions::new();
    options.map_size(1 << 30).max_dbs(1);
    // SAFETY: single-threaded single-process scratch environment; NO_SYNC/
    // WRITE_MAP only trade durability, which the experiment exists to price.
    unsafe { options.flags(flags) };
    // SAFETY: each scratch environment gets a fresh directory, opened once.
    unsafe { options.open(dir).expect("scratch env opens") }
}

/// Creates the one named database of a scratch environment (its own
/// committing transaction, outside any timed window).
fn scratch_db(env: &heed::Env) -> heed::Database<heed::types::Bytes, heed::types::Bytes> {
    let mut wtxn = env.write_txn().expect("open write txn");
    let db = env
        .create_database(&mut wtxn, None)
        .expect("create scratch db");
    wtxn.commit().expect("commit db creation");
    db
}

/// One timed LMDB commit against a scratch environment: `facts` puts of
/// 24-byte keys and 16-byte values (the order of the engine's fact-row
/// shape). One commit per call so the R4 cells can interleave.
fn one_scratch_commit(
    env: &heed::Env,
    db: &heed::Database<heed::types::Bytes, heed::types::Bytes>,
    facts: u32,
    seq: &mut u64,
) -> Duration {
    let start = Instant::now();
    let mut wtxn = env.write_txn().expect("open write txn");
    for _ in 0..facts {
        *seq += 1;
        let mut key = [0u8; 24];
        key[0] = b'F';
        key[8..16].copy_from_slice(&seq.to_be_bytes());
        let value = seq.to_le_bytes();
        let mut payload = [0u8; 16];
        payload[..8].copy_from_slice(&value);
        db.put(&mut wtxn, &key, &payload).expect("put");
    }
    wtxn.commit().expect("commit");
    start.elapsed()
}

/// One R4 flag configuration under measurement: its live scratch
/// environment and the samples it has accumulated so far.
struct FlagCell {
    name: &'static str,
    env: heed::Env,
    db: heed::Database<heed::types::Bytes, heed::types::Bytes>,
    seq: u64,
    small: Vec<Duration>,
    bulk: Vec<Duration>,
}

/// The quiet-machine band for an R4 bulk cell: max/min spread within
/// 2x. Quiet runs on the pinned machine measure ~1.1–1.3x; the one
/// recorded co-tenant-contaminated run measured 2.9x (and printed a
/// spurious 2.27x trigger ratio). A run outside the band is not
/// decision-grade.
const R4_SPREAD_BAND: f64 = 2.0;

/// Prints a warning per bulk cell whose spread exceeds the band;
/// returns whether any did.
fn r4_noise_guard(cells: &[FlagCell]) -> bool {
    let mut noisy = false;
    for c in cells {
        let (min, max) = spread(&c.bulk);
        let ratio = max.as_secs_f64() / min.as_secs_f64().max(f64::EPSILON);
        if ratio > R4_SPREAD_BAND {
            noisy = true;
            println!(
                "R4 WARNING: {} bulk spread {ratio:.2}x exceeds the quiet-machine band \
                 (max/min <= {R4_SPREAD_BAND}x) — co-tenant load suspected; the trigger \
                 ratios below are NOT decision-grade, re-run on a quiet machine",
                c.name
            );
        }
    }
    noisy
}

// ---------------------------------------------------------------------
// R5: memory sampling
// ---------------------------------------------------------------------

/// One `vm_stat` sample, in bytes (pages × the reported page size).
/// Rough by design — the whole machine moves under it; the phase-R
/// report records the method and the caveat.
struct VmSample {
    wired: i64,
    file_backed: i64,
    free: i64,
}

fn vm_row(out: &str, prefix: &str, page_size: u64) -> i64 {
    let pages: i64 = out
        .lines()
        .find(|l| l.starts_with(prefix))
        .and_then(|l| l.rsplit(':').next())
        .map(|n| n.trim().trim_end_matches('.'))
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("vm_stat row {prefix}"));
    pages * i64::try_from(page_size).expect("page size fits")
}

fn vm_sample() -> VmSample {
    let out = run("vm_stat", &[]);
    let page_size: u64 = out
        .lines()
        .next()
        .and_then(|l| l.split("page size of ").nth(1))
        .and_then(|l| l.split(' ').next())
        .and_then(|n| n.parse().ok())
        .expect("vm_stat page size");
    VmSample {
        wired: vm_row(&out, "Pages wired down", page_size),
        file_backed: vm_row(&out, "File-backed pages", page_size),
        free: vm_row(&out, "Pages free", page_size),
    }
}

fn du_bytes(dir: &Path) -> u64 {
    let out = run("du", &["-sk", dir.to_str().expect("utf-8 path")]);
    out.split_whitespace()
        .next()
        .and_then(|n| n.parse::<u64>().ok())
        .expect("du output")
        * 1024
}

// ---------------------------------------------------------------------
// The harness
// ---------------------------------------------------------------------

#[test]
#[ignore = "the phase-R measurement harness: attaches real ram disks and runs timed commits; run manually in release with --ignored --nocapture"]
#[expect(
    clippy::too_many_lines,
    reason = "a linear measurement script: the R1..R5 cells run in one deliberate order (bulk fsync last per cell), and splitting them would scatter the order"
)]
fn ramdisk_phase_r() {
    let pid = std::process::id();
    println!("== phase R: ramdisk measurements (pid {pid}) ==");
    let sw_vers = run("sw_vers", &["-buildVersion"]);
    println!("macOS build: {}", sw_vers.trim());

    // --- SSD baseline (the system temp dir: the internal SSD, APFS) ---
    let ssd_dir = common::TempDir::new("ramdisk-phase-r-ssd");
    let ssd = engine_cells(&ssd_dir.path().join("db"));
    println!("R2 ssd small: {}", cell(&ssd.small));
    println!("R2 ssd bulk:  {}", cell(&ssd.bulk));

    // --- R3 baseline: back-to-back single-fact commits on the SSD ---
    let r3_commits: u32 = 128;
    let r3_ssd_db = Db::create(&ssd_dir.path().join("r3"), Meter).expect("store creates");
    let r3_ssd_start = Instant::now();
    let _ = timed_commits(&r3_ssd_db, r3_commits, 1);
    let r3_ssd = r3_ssd_start.elapsed();
    println!("R3 ssd {r3_commits} commits: {} ms", ms(r3_ssd));

    // --- HFS+ (non-journaled) ram disk ---
    let hfs_label = format!("bumbleR-hfs-{pid}");
    {
        let disk = RamDisk::attach("HFS+", &hfs_label);
        println!("attached {} at {}", disk.dev, disk.mount.display());

        // R1: the fullfsync smoke test — a committing write must not
        // surface CommitSync (LMDB's data sync on Darwin is
        // fcntl(F_FULLFSYNC), no fallback; a refusing device errors here).
        let smoke = Db::create(&disk.store_dir("smoke"), Meter).expect("store creates on HFS+");
        smoke
            .write(|tx| {
                let id: SampleId = tx.alloc()?;
                tx.insert(&Sample {
                    id,
                    bucket: 1,
                    amount: 1,
                })
            })
            .expect("R1 FAILED on HFS+: commit (fullfsync) refused on the ram device");
        println!("R1 hfs+ fullfsync smoke: PASS");

        // R2 cells.
        let cells = engine_cells(&disk.store_dir("r2"));
        println!("R2 hfs+ small: {}", cell(&cells.small));
        println!("R2 hfs+ bulk:  {}", cell(&cells.bulk));

        // R3: the DVFS dividend.
        let r3_db = Db::create(&disk.store_dir("r3"), Meter).expect("store creates");
        let start = Instant::now();
        let _ = timed_commits(&r3_db, r3_commits, 1);
        let r3_ram = start.elapsed();
        println!(
            "R3 hfs+ {r3_commits} commits: {} ms (ssd/ram ratio {:.1}x)",
            ms(r3_ram),
            r3_ssd.as_secs_f64() / r3_ram.as_secs_f64().max(f64::EPSILON)
        );

        // R4: the LMDB flag deltas, on this ram disk. The cells are
        // INTERLEAVED per repetition (the fact ledger's co-tenancy
        // remedy: interleaved same-session A/B stays valid under
        // ambient load) — sequential per-config blocks absorb a
        // co-tenant load spike asymmetrically, and one contaminated
        // sequential rerun printed a spurious 2.27x trigger against a
        // quiet-machine ~1.1x (the fixit record).
        let r4_root = disk.store_dir("r4");
        let configs = [
            ("default", heed::EnvFlags::empty()),
            ("NO_SYNC", heed::EnvFlags::NO_SYNC),
            (
                "WRITE_MAP|NO_SYNC",
                heed::EnvFlags::WRITE_MAP.union(heed::EnvFlags::NO_SYNC),
            ),
        ];
        let mut cells: Vec<FlagCell> = configs
            .into_iter()
            .map(|(name, flags)| {
                let dir = r4_root.join(name.replace('|', "-"));
                std::fs::create_dir_all(&dir).expect("create scratch env dir");
                let env = scratch_env(&dir, flags);
                let db = scratch_db(&env);
                FlagCell {
                    name,
                    env,
                    db,
                    seq: 0,
                    small: Vec::new(),
                    bulk: Vec::new(),
                }
            })
            .collect();
        for _ in 0..SMALL_COMMITS {
            for c in &mut cells {
                let sample = one_scratch_commit(&c.env, &c.db, SMALL_FACTS, &mut c.seq);
                c.small.push(sample);
            }
        }
        for _ in 0..BULK_COMMITS {
            for c in &mut cells {
                let sample = one_scratch_commit(&c.env, &c.db, BULK_FACTS, &mut c.seq);
                c.bulk.push(sample);
            }
        }
        for c in &cells {
            println!("R4 hfs+ {} small: {}", c.name, cell(&c.small));
            println!("R4 hfs+ {} bulk:  {}", c.name, cell(&c.bulk));
        }
        // The quiet-machine guard: a warned run's ratios must not
        // reopen (or re-close) the Phase-2 decision.
        let noisy = r4_noise_guard(&cells);
        let noise_tag = if noisy {
            " [NOISY — not decision-grade]"
        } else {
            ""
        };
        let default_bulk = median(cells[0].bulk.clone()).as_secs_f64();
        for c in &cells[1..] {
            println!(
                "R4 trigger ratio ({} vs default, bulk shape){noise_tag}: {:.2}x",
                c.name,
                default_bulk / median(c.bulk.clone()).as_secs_f64().max(f64::EPSILON)
            );
        }

        // R5: memory growth against store growth (vm_stat deltas —
        // signed: the whole machine moves under the sample). Only the
        // WIRED delta is decision-grade here: it reproduces at ~0
        // (ram-disk pages are not wired). The file-backed delta is
        // ambient-noise-dominated — recorded runs measured +172 MiB,
        // +708 MiB, and NEGATIVE for the same ~190 MiB store (the
        // fixit record) — so no budgeting rule may stand on it; the
        // worst-case RAM bound is the attach size.
        let before = vm_sample();
        let r5_dir = disk.store_dir("r5");
        let r5_db = Db::create(&r5_dir, Meter).expect("store creates");
        // ~190 MiB of facts through the bulk shape (128 commits x 4096).
        let _ = timed_commits(&r5_db, 128, BULK_FACTS);
        let after = vm_sample();
        let store = du_bytes(&r5_dir);
        let kib = |delta: i64| delta / 1024;
        println!(
            "R5 hfs+: store {} KiB on disk; vm_stat deltas: wired {:+} KiB, file-backed {:+} KiB, free {:+} KiB",
            store / 1024,
            kib(after.wired - before.wired),
            kib(after.file_backed - before.file_backed),
            kib(after.free - before.free)
        );
        println!(
            "R5 note: only the wired delta is decision-grade (~0: ram-disk pages are not \
             wired); the file-backed delta is ambient-dominated and carries no budgeting \
             rule — worst-case RAM is the attach size"
        );
    }
    println!("hfs+ ram disk detached");

    // --- APFS ram disk (sequential, halving peak wired memory) ---
    let apfs_label = format!("bumbleR-apfs-{pid}");
    {
        let disk = RamDisk::attach("APFS", &apfs_label);
        println!("attached {} at {}", disk.dev, disk.mount.display());

        let smoke = Db::create(&disk.store_dir("smoke"), Meter).expect("store creates on APFS");
        smoke
            .write(|tx| {
                let id: SampleId = tx.alloc()?;
                tx.insert(&Sample {
                    id,
                    bucket: 1,
                    amount: 1,
                })
            })
            .expect("R1 FAILED on APFS: commit (fullfsync) refused on the ram device");
        println!("R1 apfs fullfsync smoke: PASS");

        let cells = engine_cells(&disk.store_dir("r2"));
        println!("R2 apfs small: {}", cell(&cells.small));
        println!("R2 apfs bulk:  {}", cell(&cells.bulk));
    }
    println!("apfs ram disk detached");

    println!("== phase R complete — verify with: hdiutil info ==");
}

// ---------------------------------------------------------------------
// R6: the ephemeral constructor, priced through the REAL surface
// ---------------------------------------------------------------------

/// One R6 cell: a live engine store (durable or ephemeral, SSD or
/// ramdisk) accumulating interleaved samples of both commit shapes.
struct EngineFlagCell {
    name: &'static str,
    db: Db<Meter>,
    bucket: u64,
    small: Vec<Duration>,
    bulk: Vec<Duration>,
}

/// R6 (the ephemeral admission's number): the small-commit shape
/// through the REAL constructor — `Db::ephemeral` vs `Db::create` on
/// the same HFS+ ramdisk, plus ephemeral-on-SSD (the kind is
/// device-independent) and create-on-SSD as the fullfsync anchor. The
/// four cells are INTERLEAVED per repetition (the R4 co-tenancy
/// remedy), medians with min–max spreads, under the quiet-machine
/// guard — which hangs on the bulk shape exactly as R4's does (the
/// sub-100 µs small cells absorb single-commit outliers that make a
/// max/min band meaningless there; the bulk cells are the steady
/// co-tenancy witness). Run manually in release:
///
/// ```sh
/// cargo test -p bumbledb --release --test ramdisk_phase_r -- --ignored --nocapture ramdisk_phase_r_ephemeral
/// ```
#[test]
#[ignore = "the R6 measurement harness: attaches a real ram disk and runs timed commits; run manually in release with --ignored --nocapture"]
fn ramdisk_phase_r_ephemeral() {
    let pid = std::process::id();
    println!("== phase R6: the ephemeral constructor (pid {pid}) ==");
    let sw_vers = run("sw_vers", &["-buildVersion"]);
    println!("macOS build: {}", sw_vers.trim());

    let ssd_dir = common::TempDir::new("ramdisk-phase-r6-ssd");
    let hfs_label = format!("bumbleR6-hfs-{pid}");
    // 6 GiB, not the default 2: WRITEMAP ftruncates the ephemeral
    // store's data file to the full 4 GiB ephemeral map at open
    // (MAP_SIZE_EPHEMERAL), and HFS+ has no sparse files (the SSD cells
    // sit on APFS, where the ftruncate is free but open preallocates the
    // blocks explicitly — the capacity contract, storage/env/open_env.rs).
    // Ephemeral-on-HFS+ needs map size + slack.
    let disk = RamDisk::attach_sized("HFS+", &hfs_label, 12_582_912);
    println!("attached {} at {}", disk.dev, disk.mount.display());

    // Cells declared AFTER the disk so their environments close before
    // the drop guard detaches it.
    let mut cells = [
        EngineFlagCell {
            name: "create @ ssd",
            db: Db::create(&ssd_dir.path().join("create"), Meter).expect("store creates"),
            bucket: 0,
            small: Vec::new(),
            bulk: Vec::new(),
        },
        EngineFlagCell {
            name: "ephemeral @ ssd",
            db: Db::ephemeral(&ssd_dir.path().join("ephemeral"), Meter).expect("store creates"),
            bucket: 0,
            small: Vec::new(),
            bulk: Vec::new(),
        },
        EngineFlagCell {
            name: "create @ hfs+ ramdisk",
            db: Db::create(&disk.store_dir("create"), Meter).expect("store creates"),
            bucket: 0,
            small: Vec::new(),
            bulk: Vec::new(),
        },
        EngineFlagCell {
            name: "ephemeral @ hfs+ ramdisk",
            db: Db::ephemeral(&disk.store_dir("ephemeral"), Meter).expect("store creates"),
            bucket: 0,
            small: Vec::new(),
            bulk: Vec::new(),
        },
    ];
    for _ in 0..SMALL_COMMITS {
        for c in &mut cells {
            let sample = one_engine_commit(&c.db, SMALL_FACTS, &mut c.bucket);
            c.small.push(sample);
        }
    }
    for _ in 0..BULK_COMMITS {
        for c in &mut cells {
            let sample = one_engine_commit(&c.db, BULK_FACTS, &mut c.bucket);
            c.bulk.push(sample);
        }
    }
    for c in &cells {
        println!("R6 {} small: {}", c.name, cell(&c.small));
        println!("R6 {} bulk:  {}", c.name, cell(&c.bulk));
    }

    // The quiet-machine guard, R4's discipline verbatim: the bulk
    // cells' max/min spread against the 2x band.
    let mut noisy = false;
    for c in &cells {
        let (min, max) = spread(&c.bulk);
        let ratio = max.as_secs_f64() / min.as_secs_f64().max(f64::EPSILON);
        if ratio > R4_SPREAD_BAND {
            noisy = true;
            println!(
                "R6 WARNING: {} bulk spread {ratio:.2}x exceeds the quiet-machine band \
                 (max/min <= {R4_SPREAD_BAND}x) — co-tenant load suspected; the ratios \
                 below are NOT decision-grade, re-run on a quiet machine",
                c.name
            );
        }
    }
    let noise_tag = if noisy {
        " [NOISY — not decision-grade]"
    } else {
        ""
    };
    let med = |index: usize| median(cells[index].small.clone()).as_secs_f64();
    let ratio = |num: usize, den: usize| med(num) / med(den).max(f64::EPSILON);
    println!(
        "R6 flags dividend, ramdisk (create/ephemeral medians){noise_tag}: {:.1}x",
        ratio(2, 3)
    );
    println!(
        "R6 flags dividend, ssd (create/ephemeral medians){noise_tag}: {:.1}x",
        ratio(0, 1)
    );
    println!(
        "R6 staging win, create@ssd / ephemeral@ramdisk{noise_tag}: {:.1}x",
        ratio(0, 3)
    );
    println!(
        "R6 device tax on ephemeral, ssd/ramdisk medians{noise_tag}: {:.1}x",
        ratio(1, 3)
    );

    drop(cells);
    println!("== phase R6 complete — verify with: hdiutil info ==");
}
