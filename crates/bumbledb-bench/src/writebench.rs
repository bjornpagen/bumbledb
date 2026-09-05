use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::Path;

use bumbledb::{Answers, Db, RelationId, Value};

use crate::corpus_gen::{self, GenConfig, Rng, Sizes};
use crate::families::{self, param_args};
use crate::harness::{self, Measurement, Protocol, Rotation};
use crate::schema::{AccountId, InstrumentId, JournalEntryId, Ledger, Posting, PostingId, ids};

/// Application-owned posting-id authority (E-NO-RESERVE): the successor
/// engine mints nothing — entity identity is application data — so each
/// write family derives its next id from the store itself, `MAX(id) + 1`
/// over the live postings: exactly the mint the SQLite twin has always
/// used (`lanes::writes::next_posting_id`). Probed once per family entry
/// (untimed setup — an O(n) decode at bench scales, never inside a timed
/// window), then advanced locally. A refused commit leaves a gap in the
/// id space, which is ordinary application data, not a burn.
pub(crate) struct PostingMint(u64);

impl PostingMint {
    pub(crate) fn probe(db: &Db<Ledger>) -> Result<Self, String> {
        let next = db
            .read(|snap| {
                let mut max: Option<u64> = None;
                for fact in snap.scan(ids::POSTING)? {
                    let row = fact?;
                    if let Some(Value::U64(id)) = row.first() {
                        max = Some(max.map_or(*id, |seen| seen.max(*id)));
                    }
                }
                Ok(max.map_or(0, |seen| seen + 1))
            })
            .map_err(|e| format!("posting mint probe: {e:?}"))?;
        Ok(Self(next))
    }

    pub(crate) fn next(&mut self) -> PostingId {
        let id = PostingId(self.0);
        self.0 += 1;
        id
    }
}

/// The registered protocol for a write family (shared with the `SQLite` mirror
/// runners in `sqlite_run`).
/// # Panics
pub(crate) fn write_protocol(name: &str) -> Protocol {
    families::write_families()
        .iter()
        .find(|f| f.name == name)
        .expect("registered write family")
        .protocol
}

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

/// # Errors
/// # Panics
pub fn commit_single_bumbledb(db: &Db<Ledger>, cfg: GenConfig) -> Result<Measurement, String> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0115_0001);
    let mut mint = PostingMint::probe(db)?;
    harness::measure(write_protocol("commit_single"), || {
        db.write(|tx| {
            let id = mint.next();
            tx.insert([&prepared_posting(&mut rng, &sizes, id)])
        })
        .map(|admission| {
            admission.unwrap();
            1
        })
        .map_err(|e| format!("commit_single: {e:?}"))
    })
}

/// # Errors
/// # Panics
pub fn commit_witnessed_bumbledb(db: &Db<Ledger>, cfg: GenConfig) -> Result<Measurement, String> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0115_0003);
    let mut mint = PostingMint::probe(db)?;
    harness::measure(write_protocol("commit_witnessed"), || {
        db.read(|instance| {
            db.write_from(&instance.witness()?, |tx| {
                let id = mint.next();
                tx.insert([&prepared_posting(&mut rng, &sizes, id)])
            })?
            .unwrap();
            Ok(1)
        })
        .map_err(|e| format!("commit_witnessed: {e:?}"))
    })
}

/// # Errors
/// # Panics
pub fn commit_batch_bumbledb(db: &Db<Ledger>, cfg: GenConfig) -> Result<Measurement, String> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0115_0002);
    let mut mint = PostingMint::probe(db)?;
    harness::measure(write_protocol("commit_batch"), || {
        db.write(|tx| {
            for _ in 0..512 {
                let id = mint.next();
                tx.insert([&prepared_posting(&mut rng, &sizes, id)])?;
            }
            Ok(())
        })
        .map(|admission| {
            admission.unwrap();
            512
        })
        .map_err(|e| format!("commit_batch: {e:?}"))
    })
}

pub(crate) fn non_posting_relations() -> impl Iterator<Item = RelationId> {
    (0..ids::RELATIONS)
        .map(RelationId)
        .filter(|rel| *rel != ids::POSTING && *rel != ids::POSTING_TAG)
}

/// under `scratch` (S-minus-postings, built before any timing starts).
/// # Errors
/// # Panics
pub fn insert_stream_bumbledb(
    cfg: GenConfig,
    scratch: &Path,
    mode: crate::storemode::StoreMode,
) -> Result<Measurement, String> {
    let proto = write_protocol("insert_stream");
    let mut pending = VecDeque::new();
    for sample in 0..proto.warmups + proto.samples {
        let dir = scratch.join(format!("insert-stream-bumbledb-{sample}"));
        let db = mode.create(&dir, Ledger)?;
        for rel in non_posting_relations() {
            db.write(|tx| {
                tx.insert_dyn(rel, corpus_gen::relation_rows(cfg, rel))
                    .map(bumbledb::MutationReport::changed)
            })
            .map_err(|e| format!("seed: {e:?}"))?
            .unwrap();
        }
        pending.push_back(db);
    }
    let pending = RefCell::new(pending);
    let done = RefCell::new(Vec::new());
    harness::measure(proto, || {
        let db = pending.borrow_mut().pop_front().expect("pre-seeded store");
        let facts = db
            .write(|tx| {
                let postings = tx
                    .insert_dyn(ids::POSTING, corpus_gen::relation_rows(cfg, ids::POSTING))?
                    .changed();
                let tags = tx
                    .insert_dyn(
                        ids::POSTING_TAG,
                        corpus_gen::relation_rows(cfg, ids::POSTING_TAG),
                    )?
                    .changed();
                Ok(postings + tags)
            })
            .map_err(|e| format!("insert_stream: {e:?}"))?
            .unwrap()
            .value;
        // Keep the store alive: its Drop must not land inside a sample.
        done.borrow_mut().push(db);
        Ok(facts)
    })
}

/// the identical protocol (`sqlite_run::cold_containment_walk`): it keeps no
/// # Errors
/// # Panics
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
            db.read(|snap| snap.execute(&mut prepared, &args, &mut buffer))
                .map_err(|e| format!("cold execute: {e:?}"))?;
            Ok(buffer.len() as u64)
        },
    )
}

/// Delete-bearing **by contract**, not by hope: a no-op delete (the previous
/// revision absent) refuses INSIDE the write closure, so the whole transaction
/// aborts and a refused swap leaves the store byte-identical — the lane can
/// never drift into measuring the insert-only fork, and a refusal never commits
/// the replacement insert it would otherwise have smuggled in.
/// # Errors
/// # Panics
pub(crate) fn posting_swap(
    db: &Db<Ledger>,
    rng: &mut Rng,
    sizes: &Sizes,
    mint: &mut PostingMint,
    prev: &Posting,
) -> Result<Posting, String> {
    let id = mint.next();
    db.write(|tx| {
        if tx.delete([prev])?.changed() == 0 {
            return Err(bumbledb::Error::from(std::io::Error::other(
                "the swap touch must be delete-bearing: the previous revision was absent",
            )));
        }
        let next = prepared_posting(rng, sizes, id);
        tx.insert([&next])?;
        Ok(next)
    })
    .map_err(|e| format!("posting swap: {e:?}"))
    .map(|admission| admission.unwrap().value)
}

/// The first swap target — one seeded posting committed before any timing, so
/// every touch (warmups included) has a revision to delete.
/// # Errors
/// # Panics
pub(crate) fn posting_swap_seed(
    db: &Db<Ledger>,
    rng: &mut Rng,
    sizes: &Sizes,
    mint: &mut PostingMint,
) -> Result<Posting, String> {
    let id = mint.next();
    db.write(|tx| {
        let seed = prepared_posting(rng, sizes, id);
        tx.insert([&seed])?;
        Ok(seed)
    })
    .map_err(|e| format!("posting swap seed: {e:?}"))
    .map(|admission| admission.unwrap().value)
}

/// before that run.
/// # Errors
/// # Panics
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
    let mut mint = PostingMint::probe(db)?;
    let mut prev = posting_swap_seed(db, &mut rng, &sizes, &mut mint)?;
    harness::measure_cold(
        write_protocol("cold_containment_walk_delete"),
        || {
            prev = posting_swap(db, &mut rng, &sizes, &mut mint, &prev)?;
            Ok(())
        },
        || {
            let args = param_args(rotation.next_set());
            db.read(|snap| snap.execute(&mut prepared, &args, &mut buffer))
                .map_err(|e| format!("cold execute: {e:?}"))?;
            Ok(buffer.len() as u64)
        },
    )
}

/// # Errors
/// # Panics
pub fn trace_cold_containment_walk_delete(
    db: &Db<Ledger>,
    cfg: GenConfig,
    dir: Option<&Path>,
) -> Result<Option<String>, String> {
    let family = families::all()
        .iter()
        .find(|f| f.name == "containment_walk")
        .expect("containment_walk is registered");
    let query = (family.query)();
    let mut prepared = db.prepare(&query).map_err(|e| format!("prepare: {e:?}"))?;
    let mut rotation = Rotation::new((family.params)(&cfg));
    let mut buffer = Answers::new();
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0115_0005);
    let mut mint = PostingMint::probe(db)?;
    let mut prev = posting_swap_seed(db, &mut rng, &sizes, &mut mint)?;
    crate::trace_out::traced_cold_solo(
        dir,
        "cold_containment_walk_delete",
        &mut || {
            prev = posting_swap(db, &mut rng, &sizes, &mut mint, &prev)?;
            Ok(())
        },
        &mut || {
            let args = param_args(rotation.next_set());
            db.read(|snap| snap.execute(&mut prepared, &args, &mut buffer))
                .map_err(|e| format!("cold execute: {e:?}"))?;
            Ok(buffer.len() as u64)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "obs")]
    use crate::corpus;
    use crate::corpus_gen::Scale;

    const CFG: GenConfig = GenConfig {
        seed: 1,
        scale: Scale::Tiny,
    };

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("bumbledb-bench-write-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn containment_target_db(dir: &Path) -> Db<Ledger> {
        let db = Db::create(dir, Ledger).expect("create").expect("accepted");
        for rel in non_posting_relations() {
            db.write(|tx| {
                tx.insert_dyn(rel, corpus_gen::relation_rows(CFG, rel))
                    .map(bumbledb::MutationReport::changed)
            })
            .expect("seed")
            .unwrap();
        }
        db
    }

    #[cfg(feature = "obs")]
    #[test]
    fn cold_containment_walk_delete_traced_twin_lands() {
        let dir = scratch("cold-delete-trace");
        let db = Db::create(&dir, Ledger).expect("create").expect("accepted");
        corpus::load_bumbledb(&db, CFG).expect("load");
        let trace_dir = dir.join("trace");
        let table = trace_cold_containment_walk_delete(&db, CFG, Some(&trace_dir))
            .expect("the traced twin runs")
            .expect("Some dir emits a table");
        assert!(!table.is_empty(), "the flame embed is non-empty");
        let json_path = trace_dir.join("cold_containment_walk_delete.json");
        let text = std::fs::read_to_string(&json_path)
            .unwrap_or_else(|e| panic!("{}: {e}", json_path.display()));
        assert!(
            text.starts_with("[\n") && text.ends_with("\n]\n"),
            "{} parses as a Chrome array",
            json_path.display()
        );
        assert!(
            text.contains(bumbledb::obs::names::APPLY_DELETES.label())
                || text.contains(bumbledb::obs::names::LMDB_COMMIT.label()),
            "the delete-bearing commit reaches the artifact"
        );
        let folded = std::fs::read_to_string(trace_dir.join("cold_containment_walk_delete.folded"))
            .expect("the folded twin lands beside the json");
        assert!(!folded.is_empty(), "a non-degenerate fold");
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// and the refusal commits NOTHING (the generation does not move).
    #[test]
    fn posting_swap_touch_is_delete_bearing_by_contract() {
        let dir = scratch("swap-shape");
        let db = containment_target_db(&dir);
        let sizes = Sizes::of(CFG.scale);
        let mut rng = Rng::new(CFG.seed ^ 0x0115_0004);
        let mut mint = PostingMint::probe(&db).expect("probe");

        let seed = posting_swap_seed(&db, &mut rng, &sizes, &mut mint).expect("seed");
        let generation_before = db.generation().expect("generation");
        let next = posting_swap(&db, &mut rng, &sizes, &mut mint, &seed).expect("swap");
        assert!(
            next.id.0 > seed.id.0,
            "the application-owned cursor mints forward"
        );
        assert!(
            db.generation().expect("generation") > generation_before,
            "the swap is one state-changing commit"
        );

        let generation_at_refusal = db.generation().expect("generation");
        let refusal = posting_swap(&db, &mut rng, &sizes, &mut mint, &seed);
        assert!(
            refusal.is_err(),
            "a swap whose delete is a no-op must refuse"
        );
        // The refusal aborts the transaction whole: no stray insert-only

        assert_eq!(
            db.generation().expect("generation"),
            generation_at_refusal,
            "a refused swap must leave the store untouched"
        );

        let after = posting_swap(&db, &mut rng, &sizes, &mut mint, &next).expect("swap chain");
        assert!(after.id.0 > next.id.0);

        // The mint re-derives from the live store: a fresh probe never
        // collides with anything committed (MAX + 1 over live rows).
        let reprobed = PostingMint::probe(&db).expect("reprobe");
        assert_eq!(reprobed.0, after.id.0 + 1, "MAX(id) + 1 over live rows");
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// REVIEW-001: rejected admission must not produce a successful measurement.
    #[test]
    fn commit_single_rejected_admission_is_not_a_measured_success() {
        let dir = scratch("commit-single-refusal");
        let db = Db::create(&dir, Ledger)
            .expect("create")
            .expect("accepted");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            commit_single_bumbledb(&db, CFG)
        }));
        assert!(
            result.is_err(),
            "reference violations reject; the old `.map(|_| 1)` path wrongly counted that as work"
        );
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
