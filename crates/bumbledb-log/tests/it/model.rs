//! The reference replica: facts as an ordered set of canonical rows, judged
//! by the fixture schemas' one law (each relation's first field is a key),
//! imaged as a plain file. Simulations run on it; the LMDB cache is checked
//! against it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use bumbledb::changes::ChangeKind;
use bumbledb::{ChangeSet, RelationId, Schema, SchemaFingerprint, Value, WorkContext};
use bumbledb_log::{
    Bundle, CacheError, Delta, Evidence, Head, Image, ImageDigest, Judgment, Migrated, Population,
    Receipt, Replica, RequestId, Update,
};

pub type Rows = BTreeSet<(u32, Vec<u8>)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub head: Head,
    pub rows: Rows,
    pub receipts: BTreeMap<RequestId, Receipt>,
}

pub struct Model {
    bundle: Bundle,
    dir: PathBuf,
    state: Option<State>,
}

impl Model {
    pub fn new(bundle: Bundle, dir: &Path) -> Self {
        std::fs::create_dir_all(dir).expect("model directory");
        Self {
            bundle,
            dir: dir.to_path_buf(),
            state: None,
        }
    }

    pub fn state(&self) -> Option<&State> {
        self.state.as_ref()
    }

    /// Forget everything, as a lost cache directory.
    pub fn wipe(&mut self) {
        self.state = None;
    }

    fn schema(&self) -> &Schema {
        let head = &self.state.as_ref().expect("a judged state").head;
        &self.bundle.by_schema(head.schema).expect("bundled").schema
    }
}

/// Apply change sets in order to `rows`; the net delta of each.
pub fn apply_all(rows: &mut Rows, changes: &[&ChangeSet]) -> Vec<(u64, u64)> {
    changes
        .iter()
        .map(|changes| {
            let (mut added, mut removed) = (0, 0);
            for record in changes.records() {
                let row = (record.relation.0, record.row.to_vec());
                match record.kind {
                    ChangeKind::Add => added += u64::from(rows.insert(row)),
                    ChangeKind::Remove => removed += u64::from(rows.remove(&row)),
                }
            }
            (added, removed)
        })
        .collect()
}

/// The fixture law: within a relation, no two rows share their first field.
pub fn violations(schema: &Schema, rows: &Rows) -> Vec<(u32, u64)> {
    let mut seen = BTreeSet::new();
    let mut broken = Vec::new();
    for (relation, row) in rows {
        let fields = schema.relation(RelationId(*relation)).fields();
        let decoded =
            bumbledb::canonical::decode(fields, row, &WorkContext::new()).expect("canonical row");
        let Value::U64(key) = decoded.values()[0] else {
            panic!("fixture keys are u64");
        };
        if !seen.insert((*relation, key)) {
            broken.push((*relation, key));
        }
    }
    broken
}

fn evidence(broken: &[(u32, u64)]) -> Evidence {
    let bytes: Vec<u8> = broken
        .iter()
        .flat_map(|(relation, key)| relation.to_be_bytes().into_iter().chain(key.to_be_bytes()))
        .collect();
    Evidence::new(bytes.into()).expect("a violation has evidence")
}

impl Replica for Model {
    fn bundle(&self) -> &Bundle {
        &self.bundle
    }

    fn head(&self) -> Option<&Head> {
        self.state.as_ref().map(|state| &state.head)
    }

    fn receipt(&self, request: RequestId) -> Result<Option<Receipt>, CacheError> {
        Ok(self
            .state
            .as_ref()
            .and_then(|state| state.receipts.get(&request).cloned()))
    }

    fn judge(&self, accepted: &[ChangeSet], next: &ChangeSet) -> Result<Judgment, CacheError> {
        let state = self.state.as_ref().expect("a judged state");
        let mut rows = state.rows.clone();
        apply_all(&mut rows, &accepted.iter().collect::<Vec<_>>());
        let (added, removed) = apply_all(&mut rows, &[next])[0];
        let broken = violations(self.schema(), &rows);
        Ok(if broken.is_empty() {
            Delta::new(added, removed).map_or(Judgment::Unchanged, Judgment::Changed)
        } else {
            Judgment::Rejected(evidence(&broken))
        })
    }

    fn apply(&mut self, update: Update<'_>) -> Result<(), CacheError> {
        let state = self.state.as_mut().expect("an applied state");
        let changes: Vec<&ChangeSet> = update.commits.iter().map(|(changes, _)| *changes).collect();
        let deltas = apply_all(&mut state.rows, &changes);
        for ((_, delta), (added, removed)) in update.commits.iter().zip(deltas) {
            if (delta.added(), delta.removed()) != (added, removed) {
                return Err(CacheError::Diverged);
            }
        }
        for receipt in update.receipts {
            state
                .receipts
                .insert(receipt.command.request, receipt.clone());
        }
        state.head = update.head.clone();
        Ok(())
    }

    fn create(&mut self, head: &Head) -> Result<(), CacheError> {
        self.state = Some(State {
            head: head.clone(),
            rows: Rows::new(),
            receipts: BTreeMap::new(),
        });
        Ok(())
    }

    fn install(
        &mut self,
        path: &Path,
        digest: ImageDigest,
        schema: SchemaFingerprint,
    ) -> Result<(), CacheError> {
        let bytes = std::fs::read(path).map_err(|error| CacheError::Local(error.to_string()))?;
        if *blake3::hash(&bytes).as_bytes() != digest.0 {
            return Err(CacheError::Digest);
        }
        if self.bundle.by_schema(schema).is_none() {
            return Err(CacheError::UnknownSchema(schema));
        }
        let state = decode_image(&bytes).ok_or(CacheError::Digest)?;
        if state.head.schema != schema {
            return Err(CacheError::Digest);
        }
        self.state = Some(state);
        Ok(())
    }

    fn image(&mut self) -> Result<Image, CacheError> {
        let state = self.state.as_ref().expect("an imaged state");
        write_image(&self.dir.join("outgoing.img"), state)
    }

    fn download_path(&self) -> PathBuf {
        self.dir.join("incoming.img")
    }

    fn migrate(&mut self, population: &Population, head: &Head) -> Result<Migrated, CacheError> {
        let state = self.state.as_ref().expect("a migrated state");
        let mut rows = Rows::new();
        for (new, old) in &population.copy {
            rows.extend(
                state
                    .rows
                    .iter()
                    .filter(|(relation, _)| *relation == old.0)
                    .map(|(_, row)| (new.0, row.clone())),
            );
        }
        apply_all(&mut rows, &[&population.rows]);
        let schema = &self
            .bundle
            .by_schema(head.schema)
            .ok_or(CacheError::UnknownSchema(head.schema))?
            .schema;
        let broken = violations(schema, &rows);
        if !broken.is_empty() {
            return Ok(Migrated::Rejected(evidence(&broken)));
        }
        let migrated = State {
            head: head.clone(),
            rows,
            receipts: state.receipts.clone(),
        };
        write_image(&self.dir.join("migration.img"), &migrated).map(Migrated::Image)
    }
}

fn write_image(path: &Path, state: &State) -> Result<Image, CacheError> {
    fn blob(bytes: &mut Vec<u8>, part: &[u8]) {
        bytes.extend_from_slice(&(part.len() as u64).to_be_bytes());
        bytes.extend_from_slice(part);
    }
    let mut bytes = Vec::new();
    blob(&mut bytes, &state.head.encode());
    blob(&mut bytes, &(state.rows.len() as u64).to_be_bytes());
    for (relation, row) in &state.rows {
        blob(&mut bytes, &relation.to_be_bytes());
        blob(&mut bytes, row);
    }
    blob(&mut bytes, &(state.receipts.len() as u64).to_be_bytes());
    for receipt in state.receipts.values() {
        blob(&mut bytes, &receipt.encode());
    }
    std::fs::write(path, &bytes).map_err(|error| CacheError::Local(error.to_string()))?;
    Ok(Image {
        path: path.to_path_buf(),
        digest: ImageDigest(*blake3::hash(&bytes).as_bytes()),
    })
}

fn decode_image(mut bytes: &[u8]) -> Option<State> {
    let mut blob = || -> Option<Vec<u8>> {
        let (len, rest) = bytes.split_first_chunk::<8>()?;
        let len = usize::try_from(u64::from_be_bytes(*len)).ok()?;
        let (part, rest) = rest.split_at_checked(len)?;
        bytes = rest;
        Some(part.to_vec())
    };
    let count = |part: Vec<u8>| -> Option<u64> { Some(u64::from_be_bytes(part.try_into().ok()?)) };
    let head = Head::decode(&blob()?).ok()?;
    let mut rows = Rows::new();
    for _ in 0..count(blob()?)? {
        let relation = u32::from_be_bytes(blob()?.try_into().ok()?);
        rows.insert((relation, blob()?));
    }
    let mut receipts = BTreeMap::new();
    for _ in 0..count(blob()?)? {
        let receipt = Receipt::decode(&blob()?).ok()?;
        receipts.insert(receipt.command.request, receipt);
    }
    Some(State {
        head,
        rows,
        receipts,
    })
}
