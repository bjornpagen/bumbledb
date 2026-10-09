//! The product replica: a disposable LMDB cache of the log's state, opened
//! without syncing (the log is the authority). Facts, receipts (host records
//! `r‖request`) and the head commit in one transaction. The live database is
//! the generation named by `CURRENT`; creation, images and migrations are
//! built aside and swapped in by renaming `CURRENT`.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bumbledb::host::{self, HostChanges, HostRecord, Judged};
use bumbledb::schema::evidence;
use bumbledb::{
    Admission, ChangeSet, Db, Durability, Options, Schema, SchemaDescriptor, SchemaFingerprint,
    Violations, WorkContext,
};

use crate::head::Head;
use crate::ids::{DatabaseId, ImageDigest, RequestId};
use crate::io::hex;
use crate::receipt::{Delta, Evidence, Receipt};
use crate::replica::{
    Bundle, BundledMigration, CacheError, Image, Judgment, Migrated, Population, Replica, Update,
};

const CURRENT: &str = "CURRENT";
const RECEIPT: u8 = b'r';
const IMAGE_CONTEXT: &str = "bdb.image.v1 digest";
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
        let bytes = live
            .db
            .read(WorkContext::new(), |frame| {
                Ok(frame.host_record(&key)?.map(<[u8]>::to_vec))
            })
            .map_err(local)?;
        bytes
            .map(|bytes| Receipt::decode(&bytes).map_err(local))
            .transpose()
    }

    fn judge(&self, accepted: &[ChangeSet], next: &ChangeSet) -> Result<Judgment, CacheError> {
        let live = self.live();
        let work = WorkContext::new();
        let mut sets = accepted.to_vec();
        sets.push(next.clone());
        let judged = live
            .db
            .host_writer(&work)
            .and_then(|mut session| session.decide_all(&sets))
            .map_err(local)?;
        match judged.last().expect("the next set is judged") {
            Judged::Accepted(applied) => Ok(Delta::new(applied.added, applied.removed)
                .map_or(Judgment::Unchanged, Judgment::Changed)),
            Judged::Rejected(violations) => {
                evidence_of(live.db.schema(), violations).map(Judgment::Rejected)
            }
        }
    }

    fn apply(&mut self, update: Update<'_>) -> Result<(), CacheError> {
        let live = self
            .live
            .as_mut()
            .expect("the machine applies to a created cache");
        let sets: Vec<ChangeSet> = update
            .commits
            .iter()
            .map(|(changes, _)| (*changes).clone())
            .collect();
        let receipts: BTreeMap<Vec<u8>, Vec<u8>> = update
            .receipts
            .iter()
            .map(|receipt| (receipt_key(receipt.command.request), receipt.encode()))
            .collect();
        let records: Vec<HostRecord<'_>> = receipts
            .iter()
            .map(|(key, value)| HostRecord::Put { key, value })
            .collect();
        let head = update.head.encode();
        let work = WorkContext::new();
        let mut session = live.db.host_writer(&work).map_err(local)?;
        let prepared = session.apply_decided(&sets).map_err(local)?;
        let diverged =
            update
                .commits
                .iter()
                .zip(prepared.applied_each())
                .any(|((_, delta), applied)| {
                    (delta.added(), delta.removed()) != (applied.added, applied.removed)
                });
        if diverged {
            prepared.abort();
            return Err(CacheError::Diverged);
        }
        prepared
            .seal(HostChanges {
                records: &records,
                head: host::Head::Put(&head),
            })
            .and_then(host::Sealed::commit)
            .map_err(local)?;
        drop(session);
        live.head = update.head.clone();
        Ok(())
    }

    fn create(&mut self, head: &Head) -> Result<(), CacheError> {
        let step = self.step(head.schema)?.clone();
        self.replace(|path| {
            let work = WorkContext::new();
            let database = engine_id(head.database);
            let created = CacheDb::create_identified(
                path,
                step.descriptor.clone(),
                database,
                options(),
                work.clone(),
            )
            .map_err(local)?;
            let Admission::Accepted(db) = created else {
                return Err(CacheError::Diverged);
            };
            let encoded = head.encode();
            db.host_writer(&work)
                .and_then(|mut session| {
                    session
                        .unchanged()?
                        .seal(HostChanges {
                            records: &[],
                            head: host::Head::Put(&encoded),
                        })?
                        .commit()
                })
                .map_err(local)?;
            Ok((db, head.clone()))
        })
    }

    fn install(
        &mut self,
        path: &Path,
        digest: ImageDigest,
        schema: SchemaFingerprint,
    ) -> Result<(), CacheError> {
        let step = self.step(schema)?.clone();
        self.replace(|dir| {
            let db = CacheDb::install_image(
                path,
                dir,
                step.descriptor.clone(),
                options(),
                WorkContext::new(),
            )
            .map_err(local)?;
            if state_digest(&db)? != digest {
                return Err(CacheError::Digest);
            }
            let head = read_head(&db)?.ok_or(CacheError::Digest)?;
            if head.schema != schema || engine_id(head.database) != db.database_id() {
                return Err(CacheError::Digest);
            }
            Ok((db, head))
        })
    }

    fn image(&mut self) -> Result<Image, CacheError> {
        let dir = self.root.join("outgoing.bdb");
        image(&self.live().db, &dir)
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
        let mut populating = host::Population::begin(
            &stage,
            step.descriptor.clone(),
            engine_id(head.database),
            options(),
            work.clone(),
        )
        .map_err(local)?;
        old.read(work, |frame| {
            for &(new, from) in &population.copy {
                populating.copy_relation(frame, from, new)?;
            }
            Ok(())
        })
        .map_err(local)?;
        populating.apply(&population.rows).map_err(local)?;
        let records = host_records(old)?;
        let puts: Vec<HostRecord<'_>> = records
            .iter()
            .map(|(key, value)| HostRecord::Put { key, value })
            .collect();
        let encoded = head.encode();
        let admitted = populating
            .admit(HostChanges {
                records: &puts,
                head: host::Head::Put(&encoded),
            })
            .map_err(local)?;
        match admitted {
            Admission::Rejected(violations) => {
                remove(&stage)?;
                evidence_of(&step.schema, &violations).map(Migrated::Rejected)
            }
            Admission::Accepted(db) => {
                let migrated = image(&db, &self.root.join("migration.bdb"));
                drop(db);
                remove(&stage)?;
                migrated.map(Migrated::Image)
            }
        }
    }
}

/// A cache environment: never synced, rebuilt from the log after a crash.
fn options() -> Options {
    Options {
        durability: Durability::Cache,
        ..Options::default()
    }
}

fn engine_id(database: DatabaseId) -> host::DatabaseId {
    host::DatabaseId(database.0)
}

fn open_db(path: &Path, step: &BundledMigration) -> Result<CacheDb, CacheError> {
    CacheDb::open_with(path, step.descriptor.clone(), options(), WorkContext::new()).map_err(local)
}

fn read_head(db: &CacheDb) -> Result<Option<Head>, CacheError> {
    let bytes = db
        .read(WorkContext::new(), |frame| {
            Ok(frame.head()?.map(<[u8]>::to_vec))
        })
        .map_err(local)?;
    bytes
        .map(|bytes| Head::decode(&bytes).map_err(local))
        .transpose()
}

fn evidence_of(schema: &Schema, violations: &Violations) -> Result<Evidence, CacheError> {
    let bytes =
        evidence::encode_violations(schema, violations, EVIDENCE_BYTES, &WorkContext::new())
            .map_err(local)?;
    Evidence::new(bytes.into()).ok_or(CacheError::Diverged)
}

/// Compact `db` into `dir` and name the image by its state.
fn image(db: &CacheDb, dir: &Path) -> Result<Image, CacheError> {
    remove(dir)?;
    db.compact(dir, WorkContext::new()).map_err(local)?;
    Ok(Image {
        path: dir.join("data.mdb"),
        digest: state_digest(db)?,
    })
}

/// BLAKE3 over the rows' platform-independent content digest, the head and
/// every host record: two images with one digest hold one state.
fn state_digest(db: &CacheDb) -> Result<ImageDigest, CacheError> {
    fn part(hasher: &mut blake3::Hasher, bytes: &[u8]) {
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    let mut hasher = blake3::Hasher::new_derive_key(IMAGE_CONTEXT);
    db.read(WorkContext::new(), |frame| {
        hasher.update(&frame.content_digest()?);
        part(&mut hasher, frame.head()?.unwrap_or_default());
        frame.host_scan(b"", &mut |key, value| {
            part(&mut hasher, key);
            part(&mut hasher, value);
            Ok(())
        })
    })
    .map_err(local)?;
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
    db.read(WorkContext::new(), |frame| {
        frame.host_scan(b"", &mut |key, value| {
            records.insert(key.to_vec(), value.to_vec());
            Ok(())
        })
    })
    .map_err(local)?;
    Ok(records)
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
