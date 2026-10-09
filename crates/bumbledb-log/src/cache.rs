//! The product replica: a disposable LMDB cache of the log's state. Facts,
//! receipts (host records `r‖request`) and the head (the attachment) commit
//! in one transaction. The live database is the generation named by
//! `CURRENT`; creation, images and migrations are built aside and swapped in
//! by renaming `CURRENT`, so a crash leaves the old state or the new one.

use std::collections::{BTreeMap, HashMap};
use std::convert::Infallible;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bumbledb::changes::ChangeKind;
use bumbledb::integration::{AttachmentChange, HostChanges, HostRecordChange, Preparation};
use bumbledb::schema::evidence;
use bumbledb::store::{
    CandidateJudge, CandidateState, Judgment as StoreJudgment, Prepared, StoreResult, UnindexedRows,
};
use bumbledb::{
    Admission, ChangeSet, Db, RelationId, Schema, SchemaDescriptor, SchemaFingerprint, WorkContext,
};

use crate::head::Head;
use crate::ids::{ImageDigest, RequestId};
use crate::receipt::{Delta, Evidence, Receipt};
use crate::replica::{
    Bundle, BundledMigration, CacheError, Image, Judgment, Migrated, Population, Replica, Update,
};

const CURRENT: &str = "CURRENT";
const RECEIPT: u8 = b'r';
/// The largest rejection evidence a log entry carries.
const EVIDENCE_BYTES: usize = 64 * 1024;

pub type CacheDb = Db<SchemaDescriptor>;

pub struct Cache {
    root: PathBuf,
    bundle: Bundle,
    live: Option<Live>,
}

struct Live {
    generation: u64,
    db: Arc<CacheDb>,
    head: Head,
}

fn local(error: impl Debug) -> CacheError {
    CacheError::Local(format!("{error:?}"))
}

impl Cache {
    /// Open the cache under `root`. Anything unusable there (no `CURRENT`, an
    /// unbundled schema, a database LMDB refuses, a missing head) is
    /// discarded: the log rebuilds it.
    /// # Errors
    /// The directory cannot be created or cleared.
    pub fn open(root: &Path, bundle: Bundle) -> Result<Self, CacheError> {
        std::fs::create_dir_all(root).map_err(local)?;
        let mut cache = Self {
            root: root.to_path_buf(),
            bundle,
            live: None,
        };
        cache.live = cache.load();
        cache.sweep()?;
        Ok(cache)
    }

    /// The database for reads; replaced whenever an image is installed.
    #[must_use]
    pub fn db(&self) -> Option<&Arc<CacheDb>> {
        self.live.as_ref().map(|live| &live.db)
    }

    fn load(&self) -> Option<Live> {
        let pointer = std::fs::read_to_string(self.root.join(CURRENT)).ok()?;
        let (generation, schema) = pointer.trim().split_once(' ')?;
        let generation = generation.parse::<u64>().ok()?;
        let step = self
            .bundle
            .steps()
            .iter()
            .find(|step| hex(&step.fingerprint.0) == schema)?;
        let db = open_db(&self.generation_path(generation), step).ok()?;
        let head = read_head(&db).ok()??;
        (head.schema == step.fingerprint).then(|| Live {
            generation,
            db: Arc::new(db),
            head,
        })
    }

    /// Remove every entry under the root but `CURRENT` and the live generation.
    fn sweep(&self) -> Result<(), CacheError> {
        let keep = self
            .live
            .as_ref()
            .map(|live| self.generation_path(live.generation));
        for entry in std::fs::read_dir(&self.root).map_err(local)? {
            let path = entry.map_err(local)?.path();
            if path.file_name().is_some_and(|name| name == CURRENT) || Some(&path) == keep.as_ref()
            {
                continue;
            }
            remove(&path)?;
        }
        if self.live.is_none() {
            remove(&self.root.join(CURRENT))?;
        }
        Ok(())
    }

    fn generation_path(&self, generation: u64) -> PathBuf {
        self.root.join(format!("{generation}.bdb"))
    }

    fn next_generation(&self) -> u64 {
        self.live.as_ref().map_or(1, |live| live.generation + 1)
    }

    fn step(&self, schema: SchemaFingerprint) -> Result<&BundledMigration, CacheError> {
        self.bundle
            .by_schema(schema)
            .ok_or(CacheError::UnknownSchema(schema))
    }

    fn live(&self) -> &Live {
        self.live
            .as_ref()
            .expect("the machine reads only a created cache")
    }

    /// Make `generation` (already holding `db` at `head`) the live state.
    fn promote(&mut self, generation: u64, db: CacheDb, head: Head) -> Result<(), CacheError> {
        let pointer = self.root.join("CURRENT.tmp");
        std::fs::write(&pointer, format!("{generation} {}\n", hex(&head.schema.0)))
            .map_err(local)?;
        std::fs::rename(&pointer, self.root.join(CURRENT)).map_err(local)?;
        let previous = self.live.replace(Live {
            generation,
            db: Arc::new(db),
            head,
        });
        if let Some(previous) = previous {
            drop(previous.db);
            remove(&self.generation_path(previous.generation))?;
        }
        Ok(())
    }

    /// Build a new generation with `build`, then promote it; on failure the
    /// half-built generation is removed and the live state is untouched.
    fn replace(
        &mut self,
        build: impl FnOnce(&Path) -> Result<(CacheDb, Head), CacheError>,
    ) -> Result<(), CacheError> {
        let generation = self.next_generation();
        let path = self.generation_path(generation);
        remove(&path)?;
        match build(&path) {
            Ok((db, head)) => self.promote(generation, db, head),
            Err(error) => {
                remove(&path)?;
                Err(error)
            }
        }
    }
}

impl Replica for Cache {
    fn bundle(&self) -> &Bundle {
        &self.bundle
    }

    fn head(&self) -> Option<&Head> {
        self.live.as_ref().map(|live| &live.head)
    }

    fn receipt(&self, request: RequestId) -> Result<Option<Receipt>, CacheError> {
        let Some(live) = &self.live else {
            return Ok(None);
        };
        let key = receipt_key(request);
        let mut found = Ok(None);
        live.db
            .read(WorkContext::new(), |frame| {
                found = frame
                    .integration_host_record(&key)
                    .map(|bytes| bytes.map(<[u8]>::to_vec))
                    .map_err(local);
                Ok(())
            })
            .map_err(local)?;
        found?
            .map(|bytes| Receipt::decode(&bytes).map_err(local))
            .transpose()
    }

    fn judge(&self, accepted: &[ChangeSet], next: &ChangeSet) -> Result<Judgment, CacheError> {
        let live = self.live();
        let schema = live.db.schema();
        let mut sets: Vec<&ChangeSet> = accepted.iter().collect();
        sets.push(next);
        let (added, removed) = *deltas(&live.db, &sets)?.last().expect("next is judged");
        let candidate = sequence(schema, &sets)?;
        let work = WorkContext::new();
        let mut session = live.db.integration_writer(&work).map_err(local)?;
        match session.prepare(&candidate).map_err(local)? {
            Preparation::Accepted(prepared) => {
                prepared.abort();
                Ok(Delta::new(added, removed).map_or(Judgment::Unchanged, Judgment::Changed))
            }
            Preparation::Rejected { violations, .. } => {
                let bytes = evidence::encode_violations(schema, &violations, EVIDENCE_BYTES, &work)
                    .map_err(local)?;
                Ok(Judgment::Rejected(
                    Evidence::new(bytes.into()).ok_or(CacheError::Diverged)?,
                ))
            }
        }
    }

    fn apply(&mut self, update: Update<'_>) -> Result<(), CacheError> {
        let live = self
            .live
            .as_mut()
            .expect("the machine applies to a created cache");
        let sets: Vec<&ChangeSet> = update.commits.iter().map(|(changes, _)| *changes).collect();
        let applied = deltas(&live.db, &sets)?;
        for ((_, delta), (added, removed)) in update.commits.iter().zip(&applied) {
            if (delta.added(), delta.removed()) != (*added, *removed) {
                return Err(CacheError::Diverged);
            }
        }
        let changes = if sets.is_empty() {
            ChangeSet::builder(live.db.schema(), WorkContext::new())
                .finish()
                .map_err(local)?
        } else {
            sequence(live.db.schema(), &sets)?
        };
        let receipts: BTreeMap<Vec<u8>, Vec<u8>> = update
            .receipts
            .iter()
            .map(|receipt| (receipt_key(receipt.command.request), receipt.encode()))
            .collect();
        let records: Vec<HostRecordChange<'_>> = receipts
            .iter()
            .map(|(key, value)| HostRecordChange::Put { key, value })
            .collect();
        let head = update.head.encode();
        let work = WorkContext::new();
        let mut owner = live.db.integration_store().writer(&work).map_err(local)?;
        let prepared = match owner
            .prepare(&changes, &UnindexedRows, &Decided)
            .map_err(local)?
        {
            Prepared::Admitted(prepared) => prepared,
            Prepared::Rejected { rejection, .. } => match rejection {},
        };
        prepared
            .seal(HostChanges {
                records: &records,
                attachment: AttachmentChange::Put(&head),
            })
            .map_err(local)?
            .commit()
            .map_err(local)?;
        drop(owner);
        live.head = update.head.clone();
        Ok(())
    }

    fn create(&mut self, head: &Head) -> Result<(), CacheError> {
        let step = self.step(head.schema)?.clone();
        self.replace(|path| {
            let work = WorkContext::new();
            let Admission::Accepted(db) =
                CacheDb::create(path, step.descriptor.clone(), work.clone()).map_err(local)?
            else {
                return Err(CacheError::Diverged);
            };
            write_head(&db, &[], head)?;
            Ok((db, head.clone()))
        })
    }

    fn install(
        &mut self,
        path: &Path,
        digest: ImageDigest,
        schema: SchemaFingerprint,
    ) -> Result<(), CacheError> {
        if file_digest(path)? != digest {
            return Err(CacheError::Digest);
        }
        let step = self.step(schema)?.clone();
        self.replace(|dir| {
            std::fs::create_dir_all(dir).map_err(local)?;
            std::fs::rename(path, dir.join("data.mdb")).map_err(local)?;
            let db = open_db(dir, &step)?;
            let head = read_head(&db)?.ok_or(CacheError::Digest)?;
            if head.schema != schema {
                return Err(CacheError::Digest);
            }
            Ok((db, head))
        })
    }

    fn image(&mut self) -> Result<Image, CacheError> {
        let dir = self.root.join("outgoing.bdb");
        compact(&self.live().db, &dir)
    }

    fn download_path(&self) -> PathBuf {
        self.root.join("incoming.bdb")
    }

    fn migrate(&mut self, population: &Population, head: &Head) -> Result<Migrated, CacheError> {
        let step = self.step(head.schema)?.clone();
        let old = &self.live().db;
        let stage = self.root.join("stage.bdb");
        remove(&stage)?;
        let work = WorkContext::new();
        let Admission::Accepted(db) =
            CacheDb::create(&stage, step.descriptor.clone(), work.clone()).map_err(local)?
        else {
            return Err(CacheError::Diverged);
        };
        let copied = copy_relations(old, &step.schema, &population.copy)?;
        let rows = copied.compose(&population.rows, &work).map_err(local)?;
        let records = host_records(old)?;
        let puts: Vec<HostRecordChange<'_>> = records
            .iter()
            .map(|(key, value)| HostRecordChange::Put { key, value })
            .collect();
        let encoded = head.encode();
        let rejected = {
            let mut session = db.integration_writer(&work).map_err(local)?;
            match session.prepare(&rows).map_err(local)? {
                Preparation::Accepted(prepared) => {
                    prepared
                        .seal(HostChanges {
                            records: &puts,
                            attachment: AttachmentChange::Put(&encoded),
                        })
                        .map_err(local)?
                        .commit()
                        .map_err(local)?;
                    None
                }
                Preparation::Rejected { violations, .. } => Some(
                    evidence::encode_violations(&step.schema, &violations, EVIDENCE_BYTES, &work)
                        .map_err(local)?,
                ),
            }
        };
        let migrated = match rejected {
            Some(bytes) => Evidence::new(bytes.into())
                .map(Migrated::Rejected)
                .ok_or(CacheError::Diverged),
            None => compact(&db, &self.root.join("migration.bdb")).map(Migrated::Image),
        };
        drop(db);
        remove(&stage)?;
        migrated
    }
}

/// Admits every candidate: an applied entry was decided when it was written.
struct Decided;

impl CandidateJudge for Decided {
    type Rejection = Infallible;

    fn judge(
        &self,
        _: &CandidateState<'_, '_>,
        _: &WorkContext,
    ) -> StoreResult<StoreJudgment<Infallible>> {
        Ok(StoreJudgment::Admitted)
    }
}

fn open_db(path: &Path, step: &BundledMigration) -> Result<CacheDb, CacheError> {
    CacheDb::open(path, step.descriptor.clone(), WorkContext::new()).map_err(local)
}

fn read_head(db: &CacheDb) -> Result<Option<Head>, CacheError> {
    let mut bytes = None;
    db.read(WorkContext::new(), |frame| {
        bytes = frame.integration_host_attachment()?.map(<[u8]>::to_vec);
        Ok(())
    })
    .map_err(local)?;
    bytes
        .map(|bytes| Head::decode(&bytes).map_err(local))
        .transpose()
}

fn write_head(
    db: &CacheDb,
    records: &[HostRecordChange<'_>],
    head: &Head,
) -> Result<(), CacheError> {
    let work = WorkContext::new();
    let empty = ChangeSet::builder(db.schema(), work.clone())
        .finish()
        .map_err(local)?;
    let encoded = head.encode();
    let mut session = db.integration_writer(&work).map_err(local)?;
    let Preparation::Accepted(prepared) = session.prepare(&empty).map_err(local)? else {
        return Err(CacheError::Diverged);
    };
    prepared
        .seal(HostChanges {
            records,
            attachment: AttachmentChange::Put(&encoded),
        })
        .map_err(local)?
        .commit()
        .map_err(local)?;
    Ok(())
}

/// Compact `db` into `dir` and name its data file as an image.
fn compact(db: &CacheDb, dir: &Path) -> Result<Image, CacheError> {
    remove(dir)?;
    db.compact(dir, WorkContext::new()).map_err(local)?;
    let path = dir.join("data.mdb");
    Ok(Image {
        digest: file_digest(&path)?,
        path,
    })
}

fn file_digest(path: &Path) -> Result<ImageDigest, CacheError> {
    let mut hasher = blake3::Hasher::new();
    let mut file = std::fs::File::open(path).map_err(local)?;
    std::io::copy(&mut file, &mut hasher).map_err(local)?;
    Ok(ImageDigest(*hasher.finalize().as_bytes()))
}

fn receipt_key(request: RequestId) -> Vec<u8> {
    let mut key = Vec::with_capacity(17);
    key.push(RECEIPT);
    key.extend_from_slice(&request.0);
    key
}

/// Host records by key, in the order a seal requires.
type HostRecords = BTreeMap<Vec<u8>, Vec<u8>>;

fn host_records(db: &CacheDb) -> Result<HostRecords, CacheError> {
    let mut records = HostRecords::new();
    let mut scanned = Ok(());
    db.read(WorkContext::new(), |frame| {
        scanned = frame
            .integration_host_scan(b"", &mut |key, value| {
                records.insert(key.to_vec(), value.to_vec());
                Ok(())
            })
            .map_err(local);
        Ok(())
    })
    .map_err(local)?;
    scanned.map(|()| records)
}

/// Every row of each `(new, old)` relation of `db`, as additions at `schema`.
fn copy_relations(
    db: &CacheDb,
    schema: &Schema,
    pairs: &[(RelationId, RelationId)],
) -> Result<ChangeSet, CacheError> {
    let work = WorkContext::new();
    let old = db.snapshot(&work).map_err(local)?;
    let mut builder = ChangeSet::builder(schema, work.clone());
    for &(new, from) in pairs {
        let fields = db
            .schema()
            .relation_checked(from)
            .ok_or_else(|| local(from))?
            .fields();
        for row in old.snapshot().rows(from).map_err(local)? {
            let (_, row) = row.map_err(local)?;
            let values = bumbledb::canonical::decode(fields, row, &work).map_err(local)?;
            builder.insert(new, values.values()).map_err(local)?;
        }
    }
    builder.finish().map_err(local)
}

/// The net `(added, removed)` each change set makes when the sets apply in
/// order to the committed state.
fn deltas(db: &CacheDb, sets: &[&ChangeSet]) -> Result<Vec<(u64, u64)>, CacheError> {
    let work = WorkContext::new();
    let read = db.snapshot(&work).map_err(local)?;
    let mut present: HashMap<(RelationId, &[u8]), bool> = HashMap::new();
    let mut out = Vec::with_capacity(sets.len());
    for set in sets {
        let (mut added, mut removed) = (0, 0);
        for record in set.records() {
            let key = (record.relation, record.row);
            let before = match present.get(&key) {
                Some(before) => *before,
                None => read
                    .snapshot()
                    .contains(record.relation, record.row, &work)
                    .map_err(local)?,
            };
            let after = record.kind == ChangeKind::Add;
            match (before, after) {
                (false, true) => added += 1,
                (true, false) => removed += 1,
                _ => {}
            }
            present.insert(key, after);
        }
        out.push((added, removed));
    }
    Ok(out)
}

/// One change set equal to applying `sets` in order: per row, the last
/// action wins.
fn sequence(schema: &Schema, sets: &[&ChangeSet]) -> Result<ChangeSet, CacheError> {
    if let [only] = sets {
        return Ok((*only).clone());
    }
    let work = WorkContext::new();
    let mut last: BTreeMap<(RelationId, &[u8]), ChangeKind> = BTreeMap::new();
    for set in sets {
        for record in set.records() {
            last.insert((record.relation, record.row), record.kind);
        }
    }
    let mut builder = ChangeSet::builder(schema, work.clone());
    for ((relation, row), kind) in last {
        let fields = schema.relation(relation).fields();
        let values = bumbledb::canonical::decode(fields, row, &work).map_err(local)?;
        match kind {
            ChangeKind::Add => builder.insert(relation, values.values()),
            ChangeKind::Remove => builder.delete(relation, values.values()),
        }
        .map_err(local)?;
    }
    builder.finish().map_err(local)
}

fn remove(path: &Path) -> Result<(), CacheError> {
    let removed = if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
    match removed {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(local(error)),
        _ => Ok(()),
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}
