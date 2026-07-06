//! Write and cold benchmark runners (docs/architecture/50-validation.md): single-commit
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

use bumbledb::{Db, RelationId, ResultBuffer};

use crate::families;
use crate::gen::{self, GenConfig, Rng, Sizes};
use crate::harness::{self, Measurement, Protocol, Rotation};
use crate::schema::{ids, schema, AccountId, InstrumentId, Posting, PostingId, TransferId};

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
pub(crate) fn seeded_posting(rng: &mut Rng, sizes: &Sizes, id: PostingId) -> Posting {
    Posting {
        id,
        transfer: TransferId(rng.range(sizes.transfers)),
        account: AccountId(rng.range(sizes.accounts)),
        instrument: InstrumentId(rng.range(sizes.instruments)),
        amount: i64::try_from(1 + rng.range(5_000_000)).expect("fits"),
        at: gen::AT_BASE + i64::try_from(rng.range(1 << 30)).expect("fits"),
        memo: format!("m{}", rng.range(gen::MEMO_VOCAB)),
        reconciled: rng.chance(3, 4),
    }
}

/// `commit_single` on bumbledb: one sample = one `db.write` allocating a
/// `PostingId` and inserting one seeded posting through the typed path.
///
/// # Errors
///
/// Engine errors, stringified.
pub fn commit_single_bumbledb(db: &Db<'_>, cfg: GenConfig) -> Result<Measurement, String> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0115_0001);
    harness::measure(write_protocol("commit_single"), || {
        db.write(|tx| {
            let id: PostingId = tx.alloc()?;
            tx.insert(&seeded_posting(&mut rng, &sizes, id))
        })
        .map(|_| 1)
        .map_err(|e| format!("commit_single: {e:?}"))
    })
}

/// `commit_batch` on bumbledb: one sample = one `db.write` inserting 512
/// seeded postings.
///
/// # Errors
///
/// Engine errors, stringified.
pub fn commit_batch_bumbledb(db: &Db<'_>, cfg: GenConfig) -> Result<Measurement, String> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0115_0002);
    harness::measure(write_protocol("commit_batch"), || {
        db.write(|tx| {
            for _ in 0..512 {
                let id: PostingId = tx.alloc()?;
                tx.insert(&seeded_posting(&mut rng, &sizes, id))?;
            }
            Ok(())
        })
        .map(|()| 512)
        .map_err(|e| format!("commit_batch: {e:?}"))
    })
}

/// The relations a bulk sample's throwaway store is pre-seeded with: the
/// whole corpus minus postings (the timed part is the posting load only).
pub(crate) fn non_posting_relations() -> impl Iterator<Item = RelationId> {
    (0..ids::RELATIONS)
        .map(RelationId)
        .filter(|rel| *rel != ids::POSTING)
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
pub fn bulk_bumbledb(cfg: GenConfig, scratch: &Path) -> Result<Measurement, String> {
    let proto = write_protocol("bulk");
    let mut pending = VecDeque::new();
    for sample in 0..proto.warmups + proto.samples {
        let dir = scratch.join(format!("bulk-bumbledb-{sample}"));
        let db = Db::create(&dir, schema()).map_err(|e| format!("create: {e:?}"))?;
        for rel in non_posting_relations() {
            db.bulk_load(rel, gen::relation_rows(cfg, rel))
                .map_err(|e| format!("seed: {e:?}"))?;
        }
        pending.push_back(db);
    }
    let pending = RefCell::new(pending);
    let done = RefCell::new(Vec::new());
    harness::measure(proto, || {
        let db = pending.borrow_mut().pop_front().expect("pre-seeded store");
        let facts = db
            .bulk_load(ids::POSTING, gen::relation_rows(cfg, ids::POSTING))
            .map_err(|e| format!("bulk: {e:?}"))?;
        // Keep the store alive: its Drop must not land inside a sample.
        done.borrow_mut().push(db);
        Ok(facts)
    })
}

/// `cold_fk_walk`: `measure_cold` over the `fk_walk` family — every sample
/// pays a touch commit (generation bump, cache eviction), so the timed
/// execution carries the image-rebuild spike. The `SQLite` mirror runs
/// the identical protocol (`sqlite_run::cold_fk_walk`): it keeps no
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
/// Only on registry corruption (`fk_walk` missing).
pub fn cold_fk_walk(db: &Db<'_>, cfg: GenConfig) -> Result<Measurement, String> {
    let family = families::all()
        .iter()
        .find(|f| f.name == "fk_walk")
        .expect("fk_walk is registered");
    let query = (family.query)();
    let mut prepared = db.prepare(&query).map_err(|e| format!("prepare: {e:?}"))?;
    let mut rotation = Rotation::new((family.params)(&cfg));
    let mut buffer = ResultBuffer::new();
    harness::measure_cold(
        write_protocol("cold_fk_walk"),
        harness::tag_touch(db),
        || {
            let params = rotation.next_set().to_vec();
            db.read(|snap| snap.execute(&mut prepared, &params, &mut buffer))
                .map_err(|e| format!("cold execute: {e:?}"))?;
            Ok(buffer.len() as u64)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus;
    use crate::gen::Scale;

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

    /// A store holding every posting FK target (the commit families need
    /// referenced rows, not the posting mass).
    fn fk_target_db(dir: &Path) -> Db<'static> {
        let db = Db::create(dir, schema()).expect("create");
        for rel in non_posting_relations() {
            db.bulk_load(rel, gen::relation_rows(CFG, rel))
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
        let db = fk_target_db(&source);
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

        let db = Db::open(&copy, schema()).expect("open copy");
        let single = commit_single_bumbledb(&db, CFG).expect("commit_single");
        assert!(single.stats.min > 0);
        assert_eq!(single.work, 64, "one row per sample");
        let batch = commit_batch_bumbledb(&db, CFG).expect("commit_batch");
        assert!(batch.stats.min > 0);
        assert_eq!(batch.work, 512 * 32);
        assert!(db.generation().expect("generation") > generation_before);
        drop(db);

        let db = Db::open(&source, schema()).expect("reopen source");
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
        let ours = bulk_bumbledb(CFG, &dir).expect("bulk bumbledb");
        let sizes = Sizes::of(CFG.scale);
        assert_eq!(ours.work, sizes.postings * 8, "full stream per sample");
        assert!(ours.stats.min > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The cold protocol runs, and rebuild cost shows: cold p50 is at
    /// least warm p50 on the same corpus (a 1x-margin inequality only).
    #[test]
    fn cold_fk_walk_costs_at_least_warm() {
        let dir = scratch("cold");
        let db = Db::create(&dir, schema()).expect("create");
        corpus::load_bumbledb(&db, CFG).expect("load");

        let cold = cold_fk_walk(&db, CFG).expect("cold");
        assert!(cold.stats.min > 0);

        let family = families::all()
            .iter()
            .find(|f| f.name == "fk_walk")
            .expect("registered");
        let query = (family.query)();
        let mut prepared = db.prepare(&query).expect("prepare");
        let mut rotation = Rotation::new((family.params)(&CFG));
        let mut buffer = ResultBuffer::new();
        let warm = harness::measure(Protocol::WARM, || {
            let params = rotation.next_set().to_vec();
            db.read(|snap| snap.execute(&mut prepared, &params, &mut buffer))
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
}
