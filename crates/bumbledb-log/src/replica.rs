//! The local state the machine decides against and applies to. The LMDB
//! [`crate::cache::Cache`] is the product replica; the simulation tests fold
//! the same entries through an in-memory reference.

use std::path::{Path, PathBuf};

use bumbledb::schema::RelationDescriptor;
use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::schema::fingerprint::fingerprint;
use bumbledb::{ChangeSet, RelationId, Schema, SchemaDescriptor, SchemaError, SchemaFingerprint};

use crate::entry::MigrationId;
use crate::head::Head;
use crate::ids::{ImageDigest, RequestId, Seq};
use crate::receipt::{Delta, Evidence, Receipt};

/// The migrations this code ships, oldest first. The first one is the
/// initial schema a new database starts from.
#[derive(Debug, Clone)]
pub struct Bundle {
    steps: Box<[BundledMigration]>,
}

#[derive(Debug, Clone)]
pub struct BundledMigration {
    pub id: MigrationId,
    pub descriptor: SchemaDescriptor,
    pub schema: Schema,
    pub fingerprint: SchemaFingerprint,
}

#[derive(Debug)]
pub enum BundleError {
    Empty,
    Schema { index: usize, error: SchemaError },
}

impl Bundle {
    /// # Errors
    /// An empty bundle or a schema the engine refuses.
    pub fn new(steps: Vec<(MigrationId, SchemaDescriptor)>) -> Result<Self, BundleError> {
        if steps.is_empty() {
            return Err(BundleError::Empty);
        }
        let steps = steps
            .into_iter()
            .enumerate()
            .map(|(index, (id, descriptor))| {
                let schema = descriptor
                    .clone()
                    .validate()
                    .map_err(|error| BundleError::Schema { index, error })?;
                Ok(BundledMigration {
                    id,
                    fingerprint: fingerprint(&schema),
                    descriptor,
                    schema,
                })
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { steps })
    }

    #[must_use]
    pub fn steps(&self) -> &[BundledMigration] {
        &self.steps
    }

    #[must_use]
    pub fn initial(&self) -> &BundledMigration {
        &self.steps[0]
    }

    #[must_use]
    pub fn ids(&self) -> Vec<MigrationId> {
        self.steps.iter().map(|step| step.id.clone()).collect()
    }

    #[must_use]
    pub fn by_schema(&self, schema: SchemaFingerprint) -> Option<&BundledMigration> {
        self.steps.iter().find(|step| step.fingerprint == schema)
    }

    /// The `(new, old)` relations migration `index` carries over unchanged
    /// from the schema before it: same name, same fields, not closed.
    #[must_use]
    pub fn unchanged(&self, index: usize) -> Box<[(RelationId, RelationId)]> {
        let Some(previous) = index
            .checked_sub(1)
            .and_then(|before| self.steps.get(before))
        else {
            return Box::new([]);
        };
        let Some(step) = self.steps.get(index) else {
            return Box::new([]);
        };
        let old = writable(&previous.descriptor.relations);
        writable(&step.descriptor.relations)
            .into_iter()
            .filter_map(|(new, relation)| {
                old.iter()
                    .find(|(_, before)| {
                        before.name == relation.name && before.fields == relation.fields
                    })
                    .map(|(old, _)| (new, *old))
            })
            .collect()
    }
}

/// The relations a change set may write (not closed), with their ids.
fn writable(relations: &[RelationDescriptor]) -> Vec<(RelationId, &RelationDescriptor)> {
    relations
        .iter()
        .enumerate()
        .filter(|(_, relation)| relation.extension.is_none())
        .map(|(id, relation)| {
            let id = RelationId(u32::try_from(id).expect("relation ids fit u32"));
            (id, relation)
        })
        .collect()
}

/// How one change set fares against the state it would apply to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Judgment {
    Changed(Delta),
    Unchanged,
    Rejected(Evidence),
}

/// One entry's effect: the committed change sets in decision order with the
/// net delta each produced, the receipts, and the head after it.
pub struct Update<'a> {
    pub head: &'a Head,
    pub commits: &'a [(&'a ChangeSet, Delta)],
    pub receipts: &'a [Receipt],
}

/// A file image of a whole state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub path: PathBuf,
    pub digest: ImageDigest,
}

/// The state a migration produces from the state at `base`: each
/// `(new, old)` relation pair copied unchanged, plus `rows` at the new schema.
#[derive(Debug, Clone)]
pub struct Population {
    pub step: MigrationId,
    pub base: Seq,
    pub copy: Box<[(RelationId, RelationId)]>,
    pub rows: ChangeSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Migrated {
    Image(Image),
    /// The populated state breaks the new schema's laws.
    Rejected(Evidence),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheError {
    /// The engine or the filesystem failed; the message is for people.
    Local(String),
    /// Applying a decided entry produced different effects than it records.
    Diverged,
    /// An image whose bytes do not match its digest.
    Digest,
    /// No bundled schema has this fingerprint.
    UnknownSchema(SchemaFingerprint),
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "log cache: {self:?}")
    }
}

impl std::error::Error for CacheError {}

pub trait Replica {
    fn bundle(&self) -> &Bundle;

    /// The applied head; `None` until a Genesis or an image is installed.
    fn head(&self) -> Option<&Head>;

    /// The receipt of `request` if the head has decided it.
    /// # Errors
    /// Local failure.
    fn receipt(&self, request: RequestId) -> Result<Option<Receipt>, CacheError>;

    /// Judge `next` against the head state with `accepted` applied in order.
    /// # Errors
    /// Local failure.
    fn judge(&self, accepted: &[ChangeSet], next: &ChangeSet) -> Result<Judgment, CacheError>;

    /// Apply one entry's effect atomically.
    /// # Errors
    /// Local failure, or [`CacheError::Diverged`] when a change set's net
    /// delta differs from the recorded one.
    fn apply(&mut self, update: Update<'_>) -> Result<(), CacheError>;

    /// Start an empty state at `head`, whose schema is bundled.
    /// # Errors
    /// Local failure.
    fn create(&mut self, head: &Head) -> Result<(), CacheError>;

    /// Replace the state with the image at `path`, built at `schema`.
    /// # Errors
    /// Local failure, a digest mismatch or an unbundled schema.
    fn install(
        &mut self,
        path: &Path,
        digest: ImageDigest,
        schema: SchemaFingerprint,
    ) -> Result<(), CacheError>;

    /// Write an image of the head state.
    /// # Errors
    /// Local failure.
    fn image(&mut self) -> Result<Image, CacheError>;

    /// Where the machine downloads an image before installing it.
    fn download_path(&self) -> PathBuf;

    /// Build the state `population` produces, carrying receipts, at `head`.
    /// # Errors
    /// Local failure.
    fn migrate(&mut self, population: &Population, head: &Head) -> Result<Migrated, CacheError>;
}
