//! Write and cold benchmark runners (docs/architecture/60-validation.md): single-commit
//! latency (fsync-bound), batch commit, bulk throughput, and the cold
//! first-execution spike. All `Kind::Report` — described honestly, never
//! gated.
//!
//! Corpus discipline: these runners mutate the store they are handed, so
//! bench NEVER points them at a verified corpus in place — it loads (or
//! copies) its own scratch corpus per invocation, keeping the verify
//! stamp honest. Inserted posting ids are minted via `tx.alloc`, so
//! samples cannot collide with corpus ids.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::Path;

use bumbledb::{Answers, Db, RelationId};

use crate::corpus_gen::{self, GenConfig, Rng, Sizes};
use crate::families::{self, param_args};
use crate::harness::{self, Measurement, Protocol, Rotation};
use crate::schema::{AccountId, InstrumentId, JournalEntryId, Ledger, Posting, PostingId, ids};

/// The registered protocol for a write family (shared with the `SQLite`
/// mirror runners in `sqlite_run`).
///
/// # Panics
///
/// On an unregistered name (a programmer error).
pub(crate) fn write_protocol(name: &str) -> Protocol {
    families::write_families()
        .iter()
        .find(|f| f.name == name)
        .expect("registered write family")
        .protocol
}

/// One seeded posting body (everything but the id), referencing existing
/// corpus rows.
pub(crate) fn prepared_posting(rng: &mut Rng, sizes: &Sizes, id: PostingId) -> Posting {
    Posting {
        id,
        entry: JournalEntryId(rng.range(sizes.entries)),
        account: AccountId(rng.range(sizes.accounts)),
        instrument: InstrumentId(rng.range(sizes.instruments)),
        amount: i64::try_from(1 + rng.range(5_000_000)).expect("fits"),
        at: corpus_gen::AT_BASE + i64::try_from(rng.range(1 << 30)).expect("fits"),
    }
}

/// `commit_single` on bumbledb: one sample = one `db.write` allocating a
/// `PostingId` and inserting one seeded posting through the typed path.
///
/// # Errors
///
/// Engine errors, stringified.
pub fn commit_single_bumbledb(db: &Db<Ledger>, cfg: GenConfig) -> Result<Measurement, String> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0115_0001);
    harness::measure(write_protocol("commit_single"), || {
        db.write(|tx| {
            let id: PostingId = tx.alloc()?;
            tx.insert(&prepared_posting(&mut rng, &sizes, id))
        })
        .map(|_| 1)
        .map_err(|e| format!("commit_single: {e:?}"))
    })
}

/// `commit_witnessed` on bumbledb: one sample = one `Db::write_from`
/// under a fresh read snapshot as the witness, inserting one seeded
/// posting — `commit_single` plus the generation witness (the
/// `70-api.md` conditional write). Single-threaded, so the witness
/// never moves and every sample commits; the family prices the witness
/// mechanism (a snapshot generation read + one integer compare inside
/// the critical section), not contention.
///
/// # Errors
///
/// Engine errors, stringified.
pub fn commit_witnessed_bumbledb(db: &Db<Ledger>, cfg: GenConfig) -> Result<Measurement, String> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0115_0003);
    harness::measure(write_protocol("commit_witnessed"), || {
        db.read(|snap| {
            db.write_from(snap, |tx| {
                let id: PostingId = tx.alloc()?;
                tx.insert(&prepared_posting(&mut rng, &sizes, id))
            })
        })
        .map(|_| 1)
        .map_err(|e| format!("commit_witnessed: {e:?}"))
    })
}

/// `commit_batch` on bumbledb: one sample = one `db.write` inserting 512
/// seeded postings.
///
/// # Errors
///
/// Engine errors, stringified.
pub fn commit_batch_bumbledb(db: &Db<Ledger>, cfg: GenConfig) -> Result<Measurement, String> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0115_0002);
    harness::measure(write_protocol("commit_batch"), || {
        db.write(|tx| {
            for _ in 0..512 {
                let id: PostingId = tx.alloc()?;
                tx.insert(&prepared_posting(&mut rng, &sizes, id))?;
            }
            Ok(())
        })
        .map(|()| 512)
        .map_err(|e| format!("commit_batch: {e:?}"))
    })
}

/// The relations a bulk sample's throwaway store is pre-seeded with: the
/// whole corpus minus the posting mass (the timed part is the posting
/// load; `PostingTag` rides with it — its containment targets postings,
/// so it cannot precede them).
pub(crate) fn non_posting_relations() -> impl Iterator<Item = RelationId> {
    (0..ids::RELATIONS)
        .map(RelationId)
        .filter(|rel| *rel != ids::POSTING && *rel != ids::POSTING_TAG)
}

/// `bulk` on bumbledb: one sample = `bulk_load` of the full posting stream
/// into a pre-seeded throwaway store under `scratch` (S-minus-postings,
/// built before any timing starts). Facts/sec derives from
/// `work / stats`.
///
/// # Errors
///
/// Engine errors, stringified.
///
/// # Panics
///
/// On scratch I/O failures.
pub fn bulk_bumbledb(
    cfg: GenConfig,
    scratch: &Path,
    mode: crate::storemode::StoreMode,
) -> Result<Measurement, String> {
    let proto = write_protocol("bulk");
    let mut pending = VecDeque::new();
    for sample in 0..proto.warmups + proto.samples {
        let dir = scratch.join(format!("bulk-bumbledb-{sample}"));
        let db = mode.create(&dir, Ledger)?;
        for rel in non_posting_relations() {
            db.bulk_load_dyn(rel, corpus_gen::relation_rows(cfg, rel))
                .map_err(|e| format!("seed: {e:?}"))?;
        }
        pending.push_back(db);
    }
    let pending = RefCell::new(pending);
    let done = RefCell::new(Vec::new());
    harness::measure(proto, || {
        let db = pending.borrow_mut().pop_front().expect("pre-seeded store");
        let mut facts = db
            .bulk_load_dyn(ids::POSTING, corpus_gen::relation_rows(cfg, ids::POSTING))
            .map_err(|e| format!("bulk: {e:?}"))?;
        facts += db
            .bulk_load_dyn(
                ids::POSTING_TAG,
                corpus_gen::relation_rows(cfg, ids::POSTING_TAG),
            )
            .map_err(|e| format!("bulk tags: {e:?}"))?;
        // Keep the store alive: its Drop must not land inside a sample.
        done.borrow_mut().push(db);
        Ok(facts)
    })
}

/// `cold_containment_walk`: `measure_cold` over the `containment_walk` family — every sample
/// pays a touch commit (generation bump, cache eviction), so the timed
/// execution carries the image-rebuild spike. The `SQLite` mirror runs
/// the identical protocol (`sqlite_run::cold_containment_walk`): it keeps no
/// derived cache, so its number is the honest post-commit query cost —
/// the comparison that prices our cold path instead of reporting it
/// absolute.
///
/// # Errors
///
/// Engine errors, stringified.
///
/// # Panics
///
/// Only on registry corruption (`containment_walk` missing).
pub fn cold_containment_walk(db: &Db<Ledger>, cfg: GenConfig) -> Result<Measurement, String> {
    let family = families::all()
        .iter()
        .find(|f| f.name == "containment_walk")
        .expect("containment_walk is registered");
    let query = (family.query)();
    let mut prepared = db.prepare(&query).map_err(|e| format!("prepare: {e:?}"))?;
    let mut rotation = Rotation::new((family.params)(&cfg));
    let mut buffer = Answers::new();
    harness::measure_cold(
        write_protocol("cold_containment_walk"),
        harness::org_touch(db),
        || {
            let args = param_args(rotation.next_set());
            db.read(|snap| snap.execute_args(&mut prepared, &args, &mut buffer))
                .map_err(|e| format!("cold execute: {e:?}"))?;
            Ok(buffer.len() as u64)
        },
    )
}

/// One delete+reinsert swap commit on `Posting` — the cookbook's
/// canonical revision idiom (recipe 20's `delete(old)` + `insert(new)`;
/// primer's attemptText output swap), the majority write shape the
/// insert-only families never exercise. One `db.write`: delete the
/// previous revision, mint a fresh id (ids are never reissued), insert
/// the replacement. The replacement genuinely changes bytes (a fresh id
/// alone guarantees it — a same-bytes delete+insert would cancel in the
/// delta and commit nothing).
///
/// Delete-bearing **by contract**, not by hope: a no-op delete (the
/// previous revision absent) refuses INSIDE the write closure, so the
/// whole transaction aborts and a refused swap leaves the store
/// byte-identical — the lane can never drift into measuring the
/// insert-only fork, and a refusal never commits the replacement insert
/// it would otherwise have smuggled in. Containment-safe by
/// construction: the swapped posting is this runner's own (no
/// `PostingTag` references it), and the replacement's references are
/// drawn from committed corpus rows.
///
/// # Errors
///
/// Engine errors, stringified; a non-delete-bearing swap, named.
pub(crate) fn posting_swap(
    db: &Db<Ledger>,
    rng: &mut Rng,
    sizes: &Sizes,
    prev: &Posting,
) -> Result<Posting, String> {
    db.write(|tx| {
        if !tx.delete(prev)? {
            // The in-closure sentinel abort (the fuzz harness's
            // deliberate-abandon precedent): returning `Err` here drops
            // the delta whole, so nothing below ever reaches the store.
            return Err(bumbledb::Error::Io(std::io::Error::other(
                "the swap touch must be delete-bearing: the previous revision was absent",
            )));
        }
        let id: PostingId = tx.alloc()?;
        let next = prepared_posting(rng, sizes, id);
        tx.insert(&next)?;
        Ok(next)
    })
    .map_err(|e| format!("posting swap: {e:?}"))
}

/// The first swap target — one seeded posting committed before any
/// timing, so every touch (warmups included) has a revision to delete.
///
/// # Errors
///
/// Engine errors, stringified.
pub(crate) fn posting_swap_seed(
    db: &Db<Ledger>,
    rng: &mut Rng,
    sizes: &Sizes,
) -> Result<Posting, String> {
    db.write(|tx| {
        let id: PostingId = tx.alloc()?;
        let seed = prepared_posting(rng, sizes, id);
        tx.insert(&seed)?;
        Ok(seed)
    })
    .map_err(|e| format!("posting swap seed: {e:?}"))
}

/// `cold_containment_walk_delete` (PRD-I2): `cold_containment_walk`'s
/// sibling, identical in every respect except the touch commit — a
/// **delete-bearing** swap ([`posting_swap`]: delete one `Posting` +
/// reinsert a revision) instead of one Org insert. The delete lands on
/// a relation the timed walk reads, so the timed number carries the
/// rebuild a delete-bearing commit induces — the cost the majority
/// write shape (recipe-20/attemptText delete+reinsert) actually pays on
/// its next cold read, invisible to every other family by construction.
///
/// **The I1 interaction contract (the pair is the discriminator's
/// end-to-end witness):** under I1's append-only incremental images,
/// `cold_containment_walk` (insert touch) should collapse while this
/// lane must NOT improve — the walked relation carries a delete every
/// sample, so the append arm never fires for it. Append lane fast,
/// delete lane unmoved; if the delete lane moves under I1's twin, the
/// discriminator is wrong and the landing stops. Today (full rebuild on
/// every commit) the two lanes should read approximately equal.
///
/// Report-class, never gated — and structurally ungated: the ALL-WIN
/// gate (`report::RunReport::all_win`) iterates the READ families only;
/// write/cold rows never enter it. No README claim rides on this row;
/// it exists so the compact-vs-mask fork's trigger is a measurement,
/// not an argument (the mask PRD stays unwritten; see the decider twin
/// beside the kernel, `filter_mask_twin`). First honest numbers arrive
/// in the Measure phase under `scripts/measure.sh` — nothing is claimed
/// before that run.
///
/// # Errors
///
/// Engine errors, stringified.
///
/// # Panics
///
/// Only on registry corruption (`containment_walk` missing).
pub fn cold_containment_walk_delete(
    db: &Db<Ledger>,
    cfg: GenConfig,
) -> Result<Measurement, String> {
    let family = families::all()
        .iter()
        .find(|f| f.name == "containment_walk")
        .expect("containment_walk is registered");
    let query = (family.query)();
    let mut prepared = db.prepare(&query).map_err(|e| format!("prepare: {e:?}"))?;
    let mut rotation = Rotation::new((family.params)(&cfg));
    let mut buffer = Answers::new();
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0115_0004);
    let mut prev = posting_swap_seed(db, &mut rng, &sizes)?;
    harness::measure_cold(
        write_protocol("cold_containment_walk_delete"),
        || {
            prev = posting_swap(db, &mut rng, &sizes, &prev)?;
            Ok(())
        },
        || {
            let args = param_args(rotation.next_set());
            db.read(|snap| snap.execute_args(&mut prepared, &args, &mut buffer))
                .map_err(|e| format!("cold execute: {e:?}"))?;
            Ok(buffer.len() as u64)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus;
    use crate::corpus_gen::Scale;

    const CFG: GenConfig = GenConfig {
        seed: 1,
        scale: Scale::S,
    };

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("bumbledb-bench-write-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    /// A store holding every posting containment target (the commit families need
    /// referenced rows, not the posting mass).
    fn containment_target_db(dir: &Path) -> Db<Ledger> {
        let db = Db::create(dir, Ledger).expect("create");
        for rel in non_posting_relations() {
            db.bulk_load_dyn(rel, corpus_gen::relation_rows(CFG, rel))
                .expect("seed");
        }
        db
    }

    /// Both commit families run their full protocols on bumbledb, and the
    /// source corpus directory is never touched — the runs happen against
    /// a copy, whose generation grows while the original's stands still.
    #[test]
    fn commits_run_and_preserve_the_source_corpus() {
        let dir = scratch("commit");
        let source = dir.join("source");
        let db = containment_target_db(&source);
        let generation_before = db.generation().expect("generation");
        drop(db);

        // The scratch copy: bench never mutates a verified corpus in
        // place.
        let copy = dir.join("copy");
        std::fs::create_dir_all(&copy).expect("copy dir");
        for entry in std::fs::read_dir(&source).expect("read source") {
            let entry = entry.expect("entry");
            std::fs::copy(entry.path(), copy.join(entry.file_name())).expect("copy file");
        }

        let db = Db::open(&copy, Ledger).expect("open copy");
        let single = commit_single_bumbledb(&db, CFG).expect("commit_single");
        assert!(single.stats.min > 0);
        assert_eq!(single.work, 64, "one row per sample");
        let batch = commit_batch_bumbledb(&db, CFG).expect("commit_batch");
        assert!(batch.stats.min > 0);
        assert_eq!(batch.work, 512 * 32);
        // The witnessed twin: single-threaded, so the witness never
        // moves and every sample commits.
        let witnessed = commit_witnessed_bumbledb(&db, CFG).expect("commit_witnessed");
        assert!(witnessed.stats.min > 0);
        assert_eq!(witnessed.work, 64, "one row per sample");
        assert!(db.generation().expect("generation") > generation_before);
        drop(db);

        let db = Db::open(&source, Ledger).expect("reopen source");
        assert_eq!(
            db.generation().expect("generation"),
            generation_before,
            "the source corpus is untouched"
        );
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The bulk runner completes its protocol with positive
    /// throughput.
    #[test]
    fn bulk_reports_positive_throughput() {
        let dir = scratch("bulk");
        let ours =
            bulk_bumbledb(CFG, &dir, crate::storemode::StoreMode::Durable).expect("bulk bumbledb");
        let sizes = Sizes::of(CFG.scale);
        assert_eq!(
            ours.work,
            (sizes.postings + sizes.posting_tags) * 8,
            "full stream per sample"
        );
        assert!(ours.stats.min > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The cold protocol runs, and rebuild cost shows: cold p50 is at
    /// least warm p50 on the same corpus (a 1x-margin inequality only).
    #[test]
    fn cold_containment_walk_costs_at_least_warm() {
        let dir = scratch("cold");
        let db = Db::create(&dir, Ledger).expect("create");
        corpus::load_bumbledb(&db, CFG).expect("load");

        let cold = cold_containment_walk(&db, CFG).expect("cold");
        assert!(cold.stats.min > 0);

        let family = families::all()
            .iter()
            .find(|f| f.name == "containment_walk")
            .expect("registered");
        let query = (family.query)();
        let mut prepared = db.prepare(&query).expect("prepare");
        let mut rotation = Rotation::new((family.params)(&CFG));
        let mut buffer = Answers::new();
        let warm = harness::measure(Protocol::WARM, || {
            let args = param_args(rotation.next_set());
            db.read(|snap| snap.execute_args(&mut prepared, &args, &mut buffer))
                .map_err(|e| format!("warm execute: {e:?}"))?;
            Ok(buffer.len() as u64)
        })
        .expect("warm");
        assert!(
            cold.stats.p50 >= warm.stats.p50,
            "rebuild cost must show: cold p50 {} < warm p50 {}",
            cold.stats.p50,
            warm.stats.p50
        );
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The delete lane's protocol runs, and rebuild cost shows:
    /// delete-cold p50 is at least warm p50 on the same corpus (the
    /// same 1x-margin inequality the insert-touch lane pins).
    #[test]
    fn cold_containment_walk_delete_costs_at_least_warm() {
        let dir = scratch("cold-delete");
        let db = Db::create(&dir, Ledger).expect("create");
        corpus::load_bumbledb(&db, CFG).expect("load");

        let cold = cold_containment_walk_delete(&db, CFG).expect("delete cold");
        assert!(cold.stats.min > 0);

        let family = families::all()
            .iter()
            .find(|f| f.name == "containment_walk")
            .expect("registered");
        let query = (family.query)();
        let mut prepared = db.prepare(&query).expect("prepare");
        let mut rotation = Rotation::new((family.params)(&CFG));
        let mut buffer = Answers::new();
        let warm = harness::measure(Protocol::WARM, || {
            let args = param_args(rotation.next_set());
            db.read(|snap| snap.execute_args(&mut prepared, &args, &mut buffer))
                .map_err(|e| format!("warm execute: {e:?}"))?;
            Ok(buffer.len() as u64)
        })
        .expect("warm");
        assert!(
            cold.stats.p50 >= warm.stats.p50,
            "rebuild cost must show: delete-cold p50 {} < warm p50 {}",
            cold.stats.p50,
            warm.stats.p50
        );
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The touch-shape pin — the lane's reason to exist: every swap
    /// commit genuinely carries one Delete disposition for the walked
    /// relation. Enforced by contract in [`posting_swap`] (a no-op
    /// delete aborts the transaction whole, so a drift to insert-only
    /// cannot silently measure the wrong fork), and falsified here from
    /// both sides: a live previous revision swaps (delete `Ok(true)`
    /// inside, fresh id out, generation bumped), while a stale one —
    /// already deleted — REFUSES rather than degrading to an insert,
    /// and the refusal commits NOTHING (the generation does not move).
    #[test]
    fn posting_swap_touch_is_delete_bearing_by_contract() {
        let dir = scratch("swap-shape");
        let db = containment_target_db(&dir);
        let sizes = Sizes::of(CFG.scale);
        let mut rng = Rng::new(CFG.seed ^ 0x0115_0004);

        let seed = posting_swap_seed(&db, &mut rng, &sizes).expect("seed");
        let generation_before = db.generation().expect("generation");
        let next = posting_swap(&db, &mut rng, &sizes, &seed).expect("swap");
        assert!(next.id.0 > seed.id.0, "fresh ids mint forward");
        assert!(
            db.generation().expect("generation") > generation_before,
            "the swap is one state-changing commit"
        );
        // The stale side: `seed` is gone, so a swap against it must
        // refuse — the delete-bearing contract, not a silent insert.
        let generation_at_refusal = db.generation().expect("generation");
        let refusal = posting_swap(&db, &mut rng, &sizes, &seed);
        assert!(
            refusal.is_err(),
            "a swap whose delete is a no-op must refuse"
        );
        // The refusal aborts the transaction whole: no stray insert-only
        // commit rides along (the abort happens inside the closure, so
        // the replacement insert never reaches the store).
        assert_eq!(
            db.generation().expect("generation"),
            generation_at_refusal,
            "a refused swap must leave the store untouched"
        );
        // The live chain continues: the last revision swaps again.
        let after = posting_swap(&db, &mut rng, &sizes, &next).expect("swap chain");
        assert!(after.id.0 > next.id.0);
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The Wave-M cold-lineage A/B twin (PRD-I1's measurement;
    /// `docs/prds/incremental-images/prd-I1-copy-on-append.md`
    /// § Measurement): the copy-on-append win laid out beside its own
    /// absence in ONE process. Arm ON is the shipped path
    /// (`ImageCache::advance` — dirty relations evict, delete-free ones
    /// survive as append bases); arm OFF runs under
    /// `bumbledb::with_lineage_disabled` (every `advance` behaves as
    /// `evict_older_than` — the pre-copy-on-append eviction; the
    /// `ground-off` switch idiom, enabled here as a dev-dependency
    /// feature). Both cold lanes run per arm: `cold_containment_walk`
    /// is the append-path win's witness (the Org-insert touch leaves
    /// every walked relation delete-free, so the timed read appends or
    /// carries instead of rebuilding), and `cold_containment_walk_delete`
    /// is the discriminator's NEGATIVE witness (the swap touch dirties
    /// `Posting` every sample — the append arm never fires for the
    /// walked mass; if this lane moves materially between arms, the
    /// discriminator is wrong and the landing stops — PRD-I2 §4).
    ///
    /// Protocol: 3 reps × 2 arms, arm order rotating per rep (drift
    /// cancellation); fresh data per (rep, arm) — a fresh durable store,
    /// freshly corpus-loaded, so no cache, page, or store state crosses
    /// arms; each family runs its registered COLD protocol (2 warmups,
    /// 16 samples, touch commit + spin-settle before every timed read);
    /// the per-arm figure is the min of the 3 rep p50s. Each (rep, arm)
    /// block is clock-proxy bracketed with the non-retrying write-form
    /// stamp (`clockproxy::stamped` — fsync-parked cores read low; the
    /// run records contamination instead of hiding it). The test PRINTS
    /// its numbers and asserts only work sanity — never a timing (the
    /// landing bar: the verdict is read from the measured run).
    ///
    /// Invocation (the Measure phase owns execution — minutes of
    /// fsync-bound touch commits):
    /// `scripts/measure.sh cargo test --release -p bumbledb-bench cold_lineage_twin -- --ignored --nocapture`
    ///
    /// **Recorded (Wave M, 2026-07-19; Apple M2 Max, idle machine,
    /// release, `scripts/measure.sh`, scale S seed 1, durable stores):**
    /// `cold_containment_walk` ON p50s [1349.9, 1362.5, 1342.0] µs
    /// (min 1342.0) vs OFF [3536.5, 3555.8, 3405.2] µs (min 3405.2) —
    /// **OFF/ON = 2.54×** (the copy-on-append win, family-level; the
    /// PRD's ≥ 5× prediction was a prediction — the machine says 2.54×).
    /// `cold_containment_walk_delete` ON min 3578.8 µs vs OFF min
    /// 3547.0 µs — **OFF/ON = 0.99**: the delete lane is unmoved, the
    /// discriminator's negative witness holds. Every block's GHz stamp
    /// read 3.00–3.36 (post-fsync DVFS) — contamination recorded, not
    /// hidden; both arms are equally touched and interleaved, so the
    /// ratio is the claim, not the absolute p50s.
    #[test]
    #[ignore = "the Wave-M A/B twin: minutes of fsync-bound touches; scripts/measure.sh owns invocation"]
    #[expect(
        clippy::cast_precision_loss,
        reason = "nanosecond p50s sit far below 2^52"
    )]
    fn cold_lineage_twin() {
        const REPS: usize = 3;
        let dir = scratch("lineage-twin");
        // [arm][rep] p50 ns; arm 0 = lineage ON (shipped), 1 = OFF.
        let mut walk = [[0u64; REPS]; 2];
        let mut walk_delete = [[0u64; REPS]; 2];
        for rep in 0..REPS {
            let order: [usize; 2] = if rep % 2 == 0 { [0, 1] } else { [1, 0] };
            for arm in order {
                // Fresh data per (rep, arm): a fresh durable store,
                // freshly corpus-loaded — no cache, page, or store
                // state crosses arms.
                let store = dir.join(format!("rep{rep}-arm{arm}"));
                std::fs::create_dir_all(&store).expect("store dir");
                let db = Db::create(&store, Ledger).expect("create");
                corpus::load_bumbledb(&db, CFG).expect("load");
                let mut run = || -> Result<(Measurement, Measurement), String> {
                    let w = cold_containment_walk(&db, CFG)?;
                    let d = cold_containment_walk_delete(&db, CFG)?;
                    Ok((w, d))
                };
                let ((w, d), ghz) = if arm == 0 {
                    crate::clockproxy::stamped(&mut run).expect("arm ON")
                } else {
                    bumbledb::with_lineage_disabled(|| crate::clockproxy::stamped(&mut run))
                        .expect("arm OFF")
                };
                assert!(w.stats.min > 0, "walk work sanity");
                assert!(d.stats.min > 0, "delete-walk work sanity");
                walk[arm][rep] = w.stats.p50;
                walk_delete[arm][rep] = d.stats.p50;
                println!(
                    "rep {rep} arm {} — walk p50 {:.1} us, delete-walk p50 {:.1} us, ghz {:.2}/{:.2}{}",
                    if arm == 0 { "ON " } else { "OFF" },
                    w.stats.p50 as f64 / 1e3,
                    d.stats.p50 as f64 / 1e3,
                    ghz.pre,
                    ghz.post,
                    if ghz.contaminated() {
                        " (CONTAMINATED)"
                    } else {
                        ""
                    },
                );
                drop(db);
            }
        }
        let min = |xs: &[u64; REPS]| *xs.iter().min().expect("nonempty");
        let us = |ns: u64| ns as f64 / 1e3;
        println!(
            "cold_lineage_twin: scale {} seed {}, durable stores, \
             fresh store per (rep, arm), COLD protocol (2 warmups, 16 samples), \
             min-of-{REPS} p50s",
            CFG.scale.label(),
            CFG.seed,
        );
        for (name, arms) in [
            ("cold_containment_walk", &walk),
            ("cold_containment_walk_delete", &walk_delete),
        ] {
            let (on, off) = (min(&arms[0]), min(&arms[1]));
            println!(
                "{name}: ON p50s {:?} us (min {:.1}) | OFF p50s {:?} us (min {:.1}) | OFF/ON = {:.2}",
                arms[0].map(us),
                us(on),
                arms[1].map(us),
                us(off),
                us(off) / us(on).max(f64::EPSILON),
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
