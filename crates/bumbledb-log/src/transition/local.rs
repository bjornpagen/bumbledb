//! Local transition ownership. Application batches reach only unpublished
//! storage; final admission and namespace installation consume that owner.

use std::path::{Path, PathBuf};

use bumbledb::integration::{AttachmentChange, HostChanges, HostRecordChange, Preparation};
use bumbledb::schema::SchemaDescriptor;
use bumbledb::work::ScratchRelation;
use bumbledb::{ChangeSet, Db, WorkContext};

use crate::history::authority::{
    Access, ActivateOutcome, Activation, ActivationCause, DeleteOutcome, DeletedReason,
    FreezeIntent, FreezeOutcome, HeadAuthority, Lifecycle, decode_control, encode_control,
};
use crate::history::command::Limits;
use crate::history::decision::{GenesisProvenance, GenesisRecord, genesis_stamp};
use crate::history::{DecisionDigest, FrameError};
use crate::recovery::{RecoveryError, StagedPopulation, begin_staged};
use crate::writer::{LocalHistory, LogError};

use super::namespace::{NamespaceError, NamespaceLock, TargetNamespace};
use super::{Captured, Contract, Installed};

#[derive(Debug)]
pub enum Error {
    Core(bumbledb::Error),
    Work(bumbledb::work::WorkError),
    Log(LogError),
    Recovery(RecoveryError),
    Namespace(NamespaceError),
    Frame(FrameError),
    ContractMismatch,
    UnsupportedArtifact,
    Aborted,
    ActivationWon,
    OutputMismatch,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "transition: {self:?}")
    }
}
impl std::error::Error for Error {}

macro_rules! from_error {
    ($variant:ident, $source:ty) => {
        impl From<$source> for Error {
            fn from(error: $source) -> Self {
                Self::$variant(error)
            }
        }
    };
}
from_error!(Core, bumbledb::Error);
from_error!(Log, LogError);
from_error!(Recovery, RecoveryError);
from_error!(Namespace, NamespaceError);
from_error!(Frame, FrameError);

impl From<bumbledb::integration::IntegrationError> for Error {
    fn from(error: bumbledb::integration::IntegrationError) -> Self {
        Self::Log(error.into())
    }
}

impl From<bumbledb::work::WorkError> for Error {
    fn from(error: bumbledb::work::WorkError) -> Self {
        Self::Work(error)
    }
}

fn authority_error(error: crate::history::authority::AuthorityError) -> Error {
    Error::Log(error.into())
}

pub(crate) fn evidence_key(contract: &Contract) -> [u8; 17] {
    let mut key = [b't'; 17];
    key[1..].copy_from_slice(contract.operation.as_core().as_bytes());
    key
}

/// Target evidence alone never asserts that the source is writable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    Uninstalled,
    Ready(Installed),
    Activated {
        contract: Contract,
        genesis: DecisionDigest,
    },
    Aborted,
}

#[expect(
    clippy::large_enum_variant,
    reason = "one bounded population owner; no allocation needed to return it"
)]
pub enum Begin {
    Population(Population),
    Resolved(Resolution),
}

/// The sole population owner. Native registry admission serializes calls;
/// consuming or dropping it releases the attempt only after its store.
pub struct Population {
    staged: Option<StagedPopulation>,
    private: PathBuf,
    namespace: TargetNamespace,
    captured: Captured,
    limits: Limits,
    _attempt: NamespaceLock,
}

impl Drop for Population {
    fn drop(&mut self) {
        drop(self.staged.take());
        let _ = std::fs::remove_dir_all(&self.private);
    }
}

/// The source freeze binds the full native contract.
/// # Errors
/// Native contract framing or allocation refusal.
pub fn intent(contract: &Contract) -> Result<FreezeIntent, Error> {
    Ok(FreezeIntent::Transition {
        contract_digest: contract.digest()?,
        target: contract.target.incarnation_id,
    })
}

pub(crate) fn cause(contract: &Contract) -> Result<ActivationCause, Error> {
    Ok(ActivationCause::Transition {
        contract_digest: contract.digest()?,
    })
}

pub(crate) fn aborted_reason(contract: &Contract) -> Result<DeletedReason, Error> {
    Ok(DeletedReason::TransitionAborted {
        source_database: contract.source.database_id,
        source_incarnation: contract.source.incarnation_id,
        contract_digest: contract.digest()?,
    })
}

pub(crate) fn check_target_schema(
    contract: &Contract,
    descriptor: &SchemaDescriptor,
) -> Result<(), Error> {
    if crate::schema_file::schema_id(descriptor)? != contract.target.schema_id
        || contract.source.incarnation_id == contract.target.incarnation_id
    {
        return Err(Error::ContractMismatch);
    }
    Ok(())
}

pub(crate) fn read_control<S>(
    db: &Db<S>,
    work: &WorkContext,
    cap: usize,
) -> Result<HeadAuthority, Error> {
    let mut bytes = None;
    db.read(work.clone(), |read| {
        bytes = read.integration_host_attachment()?.map(<[u8]>::to_vec);
        Ok(())
    })?;
    Ok(decode_control(
        &bytes.ok_or(Error::Log(LogError::NotInitialized))?,
        cap,
    )?)
}

fn commit_control<S>(
    db: &Db<S>,
    session: &mut bumbledb::integration::WriterSession<'_, S>,
    work: &WorkContext,
    authority: &HeadAuthority,
    records: &[HostRecordChange<'_>],
    limits: Limits,
) -> Result<(), Error> {
    let bytes = encode_control(authority, limits.envelope_bytes)?;
    let empty = ChangeSet::builder(db.schema(), work.clone())
        .finish()
        .map_err(|error| Error::Log(LogError::Core(error.into())))?;
    let prepared = match session.prepare(&empty)? {
        Preparation::Accepted(prepared) => prepared,
        Preparation::Rejected { .. } => return Err(Error::OutputMismatch),
    };
    prepared
        .seal(HostChanges {
            records,
            attachment: AttachmentChange::Put(&bytes),
        })?
        .commit()?;
    Ok(())
}

fn read_capture<S>(
    db: &Db<S>,
    contract: &Contract,
    cap: usize,
    work: &WorkContext,
) -> Result<Option<Captured>, Error> {
    let mut bytes = None;
    let mut failure = None;
    db.read(work.clone(), |read| {
        match read.integration_host_record(&evidence_key(contract)) {
            Ok(value) => bytes = value.map(<[u8]>::to_vec),
            Err(error) => failure = Some(error),
        }
        Ok(())
    })?;
    if let Some(error) = failure {
        return Err(Error::Log(error.into()));
    }
    bytes
        .map(|bytes| Captured::decode(&bytes, cap).map_err(Error::Frame))
        .transpose()
}

fn check_source_contract<S>(
    source: &LocalHistory<S>,
    contract: &Contract,
    limits: Limits,
    work: &WorkContext,
) -> Result<(), Error> {
    if source.identity() != contract.source
        || read_capture(source.db(), contract, limits.envelope_bytes, work)?
            .is_some_and(|captured| captured.contract != *contract)
    {
        return Err(Error::ContractMismatch);
    }
    Ok(())
}

fn freeze<S>(
    source: &LocalHistory<S>,
    contract: Contract,
    limits: Limits,
    work: &WorkContext,
) -> Result<Captured, Error> {
    if source.identity() != contract.source {
        return Err(Error::ContractMismatch);
    }
    let db = source.db();
    let mut session = db.integration_writer(work)?;
    let authority = read_control(db, work, limits.envelope_bytes)?;
    if authority.identity != contract.source {
        return Err(Error::ContractMismatch);
    }
    let mut previous = None;
    let key = evidence_key(&contract);
    let mut failure = None;
    db.read(work.clone(), |read| {
        match read.integration_host_record(&key) {
            Ok(bytes) => previous = bytes.map(<[u8]>::to_vec),
            Err(error) => {
                failure = Some(error);
                return Ok(());
            }
        }
        Ok(())
    })?;
    if let Some(error) = failure {
        return Err(Error::Log(error.into()));
    }
    crate::writer::local::refuse_retired(db, work)?;
    let live = authority.live().map_err(authority_error)?;
    let captured = Captured {
        contract,
        decision: live.decision,
        state: live.state,
    };
    if let Some(previous) = previous {
        if Captured::decode(&previous, limits.envelope_bytes)? != captured
            || live.access
                != (Access::Frozen {
                    operation: contract.operation,
                    intent: intent(&contract)?,
                })
        {
            return Err(Error::ContractMismatch);
        }
        return Ok(captured);
    }
    let next = match authority
        .freeze(contract.operation, intent(&contract)?)
        .map_err(authority_error)?
    {
        FreezeOutcome::Frozen(next) => next,
        FreezeOutcome::AlreadyFrozen { .. } => return Err(Error::UnsupportedArtifact),
    };
    let bytes = captured.encode(limits.envelope_bytes)?;
    commit_control(
        db,
        &mut session,
        work,
        &next,
        &[HostRecordChange::Put {
            key: &key,
            value: &bytes,
        }],
        limits,
    )?;
    Ok(captured)
}

pub(crate) fn activation_evidence(
    contract: &Contract,
    control: &HeadAuthority,
) -> Result<Option<Resolution>, Error> {
    if control.identity != contract.target {
        return Err(Error::ContractMismatch);
    }
    if let Activation::Activated {
        operation,
        target_genesis,
        cause: recorded,
    } = control.activation
    {
        if operation != contract.operation || recorded != cause(contract)? {
            return Err(Error::ContractMismatch);
        }
        return Ok(Some(Resolution::Activated {
            contract: *contract,
            genesis: target_genesis,
        }));
    }
    if let Lifecycle::Deleted { operation, reason } = control.lifecycle {
        if operation != contract.operation || reason != aborted_reason(contract)? {
            return Err(Error::ContractMismatch);
        }
        return Ok(Some(Resolution::Aborted));
    }
    Ok(None)
}

/// Canonical genesis binds the admitted facts and operation evidence.
/// # Errors
/// Native evidence framing or allocation refusal.
pub fn genesis(installed: &Installed, cap: usize) -> Result<GenesisRecord, Error> {
    let contract = installed.captured.contract;
    Ok(GenesisRecord {
        identity: contract.target,
        initial_application_digest: installed.application_digest,
        initial_system_digest: blake3::derive_key(
            "bumbledb.transition.v1/installed",
            &installed.encode(cap)?,
        ),
        provenance: GenesisProvenance::Transition {
            source_database: contract.source.database_id,
            source_incarnation: contract.source.incarnation_id,
            contract_digest: contract.digest()?,
        },
    })
}

fn resolve_locked(
    namespace: &TargetNamespace,
    lock: &NamespaceLock,
    contract: &Contract,
    descriptor: &SchemaDescriptor,
    limits: Limits,
    work: &WorkContext,
) -> Result<Resolution, Error> {
    if let Some(control) = namespace.read_tombstone(limits.envelope_bytes)? {
        return activation_evidence(contract, &control)?.ok_or(Error::OutputMismatch);
    }
    if let Some(control) = namespace.read_activation(limits.envelope_bytes)? {
        return activation_evidence(contract, &control)?.ok_or(Error::OutputMismatch);
    }
    if !namespace.target_exists() {
        return Ok(Resolution::Uninstalled);
    }
    let db = Db::open(&namespace.target_dir(), descriptor.clone(), work.clone())?;
    let control = read_control(&db, work, limits.envelope_bytes)?;
    if let Some(terminal) = activation_evidence(contract, &control)? {
        if matches!(terminal, Resolution::Activated { .. }) {
            namespace.record_activation(lock, &control, limits.envelope_bytes)?;
        }
        return Ok(terminal);
    }
    let live = control.live().map_err(authority_error)?;
    if live.access
        != (Access::Frozen {
            operation: contract.operation,
            intent: intent(contract)?,
        })
    {
        return Err(Error::ContractMismatch);
    }
    let mut bytes = None;
    let mut failure = None;
    db.read(work.clone(), |read| {
        match read.integration_host_record(&evidence_key(contract)) {
            Ok(value) => bytes = value.map(<[u8]>::to_vec),
            Err(error) => failure = Some(error),
        }
        Ok(())
    })?;
    if let Some(error) = failure {
        return Err(Error::Log(error.into()));
    }
    let installed = Installed::decode(&bytes.ok_or(Error::OutputMismatch)?, limits.envelope_bytes)?;
    if installed.captured.contract != *contract
        || live.decision
            != genesis_stamp(
                &genesis(&installed, limits.envelope_bytes)?,
                limits.envelope_bytes,
            )?
        || digest_snapshot(
            &db.integration_store()
                .snapshot(work)
                .map_err(|error| Error::Core(bumbledb::Error::Store(Box::new(error))))?,
            contract,
            work,
        )? != installed.application_digest
    {
        return Err(Error::OutputMismatch);
    }
    Ok(Resolution::Ready(installed))
}

/// Resolve installation before deciding to rerun an application transform.
/// # Errors
/// Contract mismatch, storage failures and corrupt persisted evidence.
pub fn resolve<S>(
    source: &LocalHistory<S>,
    root: &Path,
    contract: &Contract,
    descriptor: &SchemaDescriptor,
    limits: Limits,
    work: &WorkContext,
) -> Result<Resolution, Error> {
    work.checkpoint()?;
    check_target_schema(contract, descriptor)?;
    check_source_contract(source, contract, limits, work)?;
    let namespace = TargetNamespace::new(root, contract.target.incarnation_id)?;
    let lock = namespace.lock()?;
    resolve_locked(&namespace, &lock, contract, descriptor, limits, work)
}

impl Population {
    /// Resolve or capture, then start from empty private disk-backed storage.
    /// # Errors
    /// Refuses another live attempt, a changed contract or an invalid schema.
    pub fn begin<S>(
        source: &LocalHistory<S>,
        root: &Path,
        contract: Contract,
        descriptor: SchemaDescriptor,
        limits: Limits,
        work: &WorkContext,
    ) -> Result<Begin, Error> {
        work.checkpoint()?;
        check_target_schema(&contract, &descriptor)?;
        check_source_contract(source, &contract, limits, work)?;
        let namespace = TargetNamespace::new(root, contract.target.incarnation_id)?;
        let lock = namespace.lock()?;
        let resolved = resolve_locked(&namespace, &lock, &contract, &descriptor, limits, work)?;
        if resolved != Resolution::Uninstalled {
            return Ok(Begin::Resolved(resolved));
        }
        let attempt = namespace.population_lock()?;
        let captured = freeze(source, contract, limits, work)?;
        namespace.clean_abandoned(&attempt)?;
        let private = namespace.fresh_staging();
        if let Some(parent) = private.parent() {
            std::fs::create_dir_all(parent).map_err(NamespaceError::Io)?;
        }
        let staged = begin_staged(&private, descriptor, work)?;
        drop(lock);
        Ok(Begin::Population(Self {
            staged: Some(staged),
            private,
            namespace,
            captured,
            limits,
            _attempt: attempt,
        }))
    }

    #[must_use]
    pub const fn captured(&self) -> Captured {
        self.captured
    }

    #[must_use]
    pub fn deployment_dir(&self) -> PathBuf {
        self.namespace.deployment_dir()
    }

    /// Ordinary sequential changes. Only complete finalization judges laws.
    /// # Errors
    /// Wrong schema, terminal fencing or the exact staging failure.
    pub fn apply(&mut self, changes: &ChangeSet, work: &WorkContext) -> Result<(), Error> {
        work.checkpoint()?;
        if changes.schema() != self.captured.contract.target.schema_id {
            return Err(Error::ContractMismatch);
        }
        let _lock = self.namespace.lock()?;
        self.check_live()?;
        self.staged
            .as_ref()
            .ok_or(Error::Aborted)?
            .apply_unjudged(changes, work)?;
        Ok(())
    }

    fn check_live(&self) -> Result<(), Error> {
        if let Some(control) = self.namespace.read_tombstone(self.limits.envelope_bytes)? {
            activation_evidence(&self.captured.contract, &control)?;
            return Err(Error::Aborted);
        }
        if self.namespace.target_exists() {
            return Err(Error::OutputMismatch);
        }
        Ok(())
    }

    /// Consume all population authority, hash once, judge once and install.
    /// Failure yields no writable owner; uncertain installation is resolved
    /// through the retained contract before beginning another attempt.
    /// # Errors
    /// Final law rejection, fencing, or native resource/storage failures.
    pub fn finish(mut self, work: &WorkContext) -> Result<Installed, Error> {
        work.checkpoint()?;
        let lock = self.namespace.lock()?;
        self.check_live()?;
        let staged = self.staged.take().ok_or(Error::Aborted)?;
        let binding = crate::recovery::encode_binding(&crate::recovery::OriginBinding {
            origin: "local".into(),
            prefix: self
                .namespace
                .deployment_dir()
                .to_string_lossy()
                .as_ref()
                .into(),
            identity: self.captured.contract.target,
        })?;
        let installed = admit_target(staged, self.captured, Some(&binding), self.limits, work)?;
        self.namespace
            .install_target(&lock, &self.private, self.limits.envelope_bytes)?;
        Ok(installed)
    }
}

/// Shared native admission for local and hosted population. Only the
/// publication authority differs; facts, judgment and digests are identical.
pub(crate) fn admit_target(
    staged: StagedPopulation,
    captured: Captured,
    binding: Option<&[u8]>,
    limits: Limits,
    work: &WorkContext,
) -> Result<Installed, Error> {
    let installed = Installed {
        captured,
        application_digest: digest(&staged, &captured.contract, work)?,
    };
    let contract = installed.captured.contract;
    let stamp = genesis_stamp(
        &genesis(&installed, limits.envelope_bytes)?,
        limits.envelope_bytes,
    )?;
    let authority = HeadAuthority::genesis(contract.target, stamp, Activation::NotActivated)
        .map_err(authority_error)?;
    let frozen = match authority
        .freeze(contract.operation, intent(&contract)?)
        .map_err(authority_error)?
    {
        FreezeOutcome::Frozen(frozen) => frozen,
        FreezeOutcome::AlreadyFrozen { .. } => return Err(Error::OutputMismatch),
    };
    let control = encode_control(&frozen, limits.envelope_bytes)?;
    let record = installed.encode(limits.envelope_bytes)?;
    let key = evidence_key(&contract);
    let mut records = Vec::with_capacity(2);
    if let Some(binding) = binding {
        records.push(HostRecordChange::Put {
            key: crate::recovery::BINDING_KEY,
            value: binding,
        });
    }
    records.push(HostRecordChange::Put {
        key: &key,
        value: &record,
    });
    staged.write_host(&records, Some(&control), work)?;
    drop(staged.complete_install(work)?);
    Ok(installed)
}

/// Canonical ordering is needed only once, at finalization. The existing
/// native scratch relation keeps the sort on disk, independent of batch order.
fn digest(
    staged: &StagedPopulation,
    contract: &Contract,
    work: &WorkContext,
) -> Result<[u8; 32], Error> {
    staged.inspect(work, |read, _| {
        Ok(digest_snapshot(read.snapshot(), contract, work))
    })?
}

pub(crate) fn digest_snapshot(
    snapshot: &bumbledb::store::OwnedSnapshot,
    contract: &Contract,
    work: &WorkContext,
) -> Result<[u8; 32], Error> {
    let mut sorted = ScratchRelation::new(work);
    sorted.force_spill()?;
    let mut failure = None;
    let mut rows = 0u64;
    let exported = snapshot.export(work, &mut |relation, bytes| {
        let mut key = Vec::with_capacity(4 + bytes.len());
        key.extend_from_slice(&relation.0.to_be_bytes());
        key.extend_from_slice(bytes);
        if let Err(error) = sorted.put(&key, &[]) {
            failure = Some(error);
            return Err(bumbledb::store::StoreError::Work(
                bumbledb::work::WorkError::Cancelled,
            ));
        }
        rows += 1;
        Ok(())
    });
    if let Some(error) = failure {
        return Err(Error::Core(error));
    }
    exported.map_err(|error| Error::Core(bumbledb::Error::Store(Box::new(error))))?;
    let mut hash = blake3::Hasher::new_derive_key("bumbledb.transition.v1/application");
    hash.update(&contract.target.schema_id.0);
    hash.update(&rows.to_be_bytes());
    sorted.for_each(&mut |key, _| {
        hash.update(&(key.len() as u64).to_be_bytes());
        hash.update(key);
        Ok(true)
    })?;
    Ok(*hash.finalize().as_bytes())
}

/// Explicit activation of the exact immutable content inspected by the app.
/// # Errors
/// Wrong evidence, terminal cancellation or native storage failure.
pub fn activate<S>(
    source: &LocalHistory<S>,
    root: &Path,
    installed: &Installed,
    descriptor: &SchemaDescriptor,
    limits: Limits,
    work: &WorkContext,
) -> Result<Resolution, Error> {
    let contract = installed.captured.contract;
    check_target_schema(&contract, descriptor)?;
    check_source_contract(source, &contract, limits, work)?;
    let namespace = TargetNamespace::new(root, contract.target.incarnation_id)?;
    let lock = namespace.lock()?;
    match resolve_locked(&namespace, &lock, &contract, descriptor, limits, work)? {
        resolved @ Resolution::Activated {
            genesis: recorded, ..
        } => {
            if genesis_stamp(
                &genesis(installed, limits.envelope_bytes)?,
                limits.envelope_bytes,
            )?
            .hash
                != recorded
            {
                return Err(Error::OutputMismatch);
            }
            return Ok(resolved);
        }
        Resolution::Ready(recorded) if recorded == *installed => {}
        Resolution::Aborted => return Err(Error::Aborted),
        _ => return Err(Error::OutputMismatch),
    }
    let db = Db::open(&namespace.target_dir(), descriptor.clone(), work.clone())?;
    let mut session = db.integration_writer(work)?;
    let authority = read_control(&db, work, limits.envelope_bytes)?;
    let stamp = genesis_stamp(
        &genesis(installed, limits.envelope_bytes)?,
        limits.envelope_bytes,
    )?;
    let next = match authority
        .activate(contract.operation, stamp.hash, cause(&contract)?)
        .map_err(authority_error)?
    {
        ActivateOutcome::Activated(next) => next,
        ActivateOutcome::AlreadyActivated { .. } => authority,
    };
    commit_control(&db, &mut session, work, &next, &[], limits)?;
    namespace.record_activation(&lock, &next, limits.envelope_bytes)?;
    Ok(Resolution::Activated {
        contract,
        genesis: stamp.hash,
    })
}

/// Fence the target under the batch/install lock before thawing the source.
/// A later population call observes the durable fence before touching facts.
/// # Errors
/// A won activation, changed contract or native storage failure.
pub fn abort<S>(
    source: &LocalHistory<S>,
    root: &Path,
    contract: &Contract,
    descriptor: &SchemaDescriptor,
    limits: Limits,
    work: &WorkContext,
) -> Result<(), Error> {
    check_target_schema(contract, descriptor)?;
    if source.identity() != contract.source {
        return Err(Error::ContractMismatch);
    }
    let namespace = TargetNamespace::new(root, contract.target.incarnation_id)?;
    let lock = namespace.lock()?;
    let db = source.db();
    let mut session = db.integration_writer(work)?;
    let authority = read_control(db, work, limits.envelope_bytes)?;
    if authority.identity != contract.source {
        return Err(Error::ContractMismatch);
    }
    let previous = read_capture(db, contract, limits.envelope_bytes, work)?;
    if previous.is_some_and(|saved| saved.contract != *contract) {
        return Err(Error::ContractMismatch);
    }
    let live = authority.live().map_err(authority_error)?;
    if let Access::Frozen {
        operation,
        intent: held,
    } = live.access
    {
        if operation != contract.operation || held != intent(contract)? {
            return Err(Error::ContractMismatch);
        }
        if previous
            != Some(Captured {
                contract: *contract,
                decision: live.decision,
                state: live.state,
            })
        {
            return Err(Error::OutputMismatch);
        }
    }
    // Retain the complete contract even when abort arrives before capture.
    // This source operation cannot later name a different target or transform.
    if previous.is_none() {
        let record = Captured {
            contract: *contract,
            decision: live.decision,
            state: live.state,
        }
        .encode(limits.envelope_bytes)?;
        commit_control(
            db,
            &mut session,
            work,
            &authority,
            &[HostRecordChange::Put {
                key: &evidence_key(contract),
                value: &record,
            }],
            limits,
        )?;
    }
    match resolve_locked(&namespace, &lock, contract, descriptor, limits, work)? {
        Resolution::Activated { .. } => return Err(Error::ActivationWon),
        Resolution::Ready(_) => {
            let db = Db::open(&namespace.target_dir(), descriptor.clone(), work.clone())?;
            let mut target_session = db.integration_writer(work)?;
            let authority = read_control(&db, work, limits.envelope_bytes)?;
            let next = match authority
                .delete(contract.operation, aborted_reason(contract)?)
                .map_err(authority_error)?
            {
                DeleteOutcome::Deleted(next) => next,
                DeleteOutcome::AlreadyDeleted { .. } => authority,
            };
            commit_control(&db, &mut target_session, work, &next, &[], limits)?;
        }
        Resolution::Uninstalled => {
            let tombstone = HeadAuthority::cancelled_before_genesis(
                contract.target,
                contract.operation,
                aborted_reason(contract)?,
            );
            namespace.install_tombstone(&lock, &tombstone, limits.envelope_bytes)?;
        }
        Resolution::Aborted => {}
    }
    if let Access::Frozen {
        operation,
        intent: held,
    } = authority.live().map_err(authority_error)?.access
    {
        if operation != contract.operation || held != intent(contract)? {
            return Err(Error::ContractMismatch);
        }
        let next = authority
            .thaw(contract.operation)
            .map_err(authority_error)?;
        commit_control(db, &mut session, work, &next, &[], limits)?;
    }
    Ok(())
}
