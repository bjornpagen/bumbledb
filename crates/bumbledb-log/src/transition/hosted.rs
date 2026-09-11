//! Imperative populations over the existing conditional hosted authority.
//! Ordinary native staging and complete judgment are shared with local
//! transitions. Only immutable evidence, checkpoints and HEAD CAS cross the
//! backend. No application code, row expressions or batch journal is stored.
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::local::{self, Resolution};
use super::namespace::{NamespaceError, NamespaceLock, TargetNamespace};
use super::{Captured, Contract, Installed};
use crate::checkpointer::{CheckpointError, CheckpointPolicy, upload_snapshot};
use crate::history::FrameError;
use crate::history::authority::{
    Access, ActivateOutcome, DeleteOutcome, FreezeOutcome, HeadAuthority, encode_control,
};
use crate::history::command::Limits;
use crate::history::decision::genesis_stamp;
use crate::manifest::{self, HeadRecord, RecoveryRoot};
use crate::recovery::{
    RecoveryError, StagedPopulation, begin_staged, import_stream, verified_chunks,
};
use crate::store::{
    BackendError, ConditionalOutcome, HeadVersion, ObjectError, ObservedError, ReceiveLimits,
    ReceivedHead, ReceivingStore, TransportContext, get_verified, head_key, read_head_bounded,
};
use crate::writer::{HostedHistory, LogError};
use bumbledb::{ChangeSet, Db, OwnedRead, SchemaDescriptor, WorkContext};

#[derive(Debug)]
pub enum Error {
    Native(local::Error),
    Object(ObjectError),
    Head(manifest::HeadError),
    Checkpoint(CheckpointError),
    /// A dispatched conditional operation could not be resolved. Keep the
    /// original contract and resolve; never infer failure or thaw from this.
    Unresolved,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "hosted transition: {self:?}")
    }
}
impl std::error::Error for Error {}
impl From<local::Error> for Error {
    fn from(error: local::Error) -> Self {
        Self::Native(error)
    }
}
impl From<FrameError> for Error {
    fn from(error: FrameError) -> Self {
        Self::Native(error.into())
    }
}
impl From<LogError> for Error {
    fn from(error: LogError) -> Self {
        Self::Native(error.into())
    }
}
impl From<RecoveryError> for Error {
    fn from(error: RecoveryError) -> Self {
        Self::Native(error.into())
    }
}
impl From<NamespaceError> for Error {
    fn from(error: NamespaceError) -> Self {
        Self::Native(error.into())
    }
}
impl From<bumbledb::Error> for Error {
    fn from(error: bumbledb::Error) -> Self {
        Self::Native(error.into())
    }
}
impl From<bumbledb::WorkError> for Error {
    fn from(error: bumbledb::WorkError) -> Self {
        Self::Native(error.into())
    }
}
impl From<ObjectError> for Error {
    fn from(error: ObjectError) -> Self {
        Self::Object(error)
    }
}
impl From<manifest::HeadError> for Error {
    fn from(error: manifest::HeadError) -> Self {
        Self::Head(error)
    }
}
impl From<CheckpointError> for Error {
    fn from(error: CheckpointError) -> Self {
        Self::Checkpoint(error)
    }
}

const ATTEMPTS: usize = 4;

/// Stable, trusted deployment coordinates. Use the same prefixes on every
/// retry. Scratch is private local storage; it is never a hosted authority.
pub struct Hosted<'a, B> {
    store: &'a B,
    source_prefix: &'a str,
    target_prefix: &'a str,
    target_epoch: u64,
    scratch: PathBuf,
    limits: Limits,
    policy: CheckpointPolicy,
}

#[expect(
    clippy::large_enum_variant,
    reason = "one bounded population owner; no allocation needed to return it"
)]
pub enum Begin<'a, S, B> {
    Populating {
        source: OwnedRead<S>,
        population: Population<'a, B>,
    },
    Resolved(Resolution),
}

/// One unpublished owner. Dropping releases scratch after admitted work;
/// only explicit abort can fence the target and thaw the source.
pub struct Population<'a, B> {
    hosted: Hosted<'a, B>,
    staged: Option<StagedPopulation>,
    descriptor: SchemaDescriptor,
    captured: Captured,
    private: PathBuf,
    _attempt: NamespaceLock,
}
impl<B> Drop for Population<'_, B> {
    fn drop(&mut self) {
        drop(self.staged.take());
        let _ = std::fs::remove_dir_all(&self.private);
    }
}

impl<'a, B: ReceivingStore> Hosted<'a, B>
where
    B::Error: BackendError + ObservedError,
{
    #[must_use]
    pub fn new(
        store: &'a B,
        source_prefix: &'a str,
        target_prefix: &'a str,
        target_epoch: u64,
        scratch: &Path,
        limits: Limits,
        policy: CheckpointPolicy,
    ) -> Self {
        Self {
            store,
            source_prefix,
            target_prefix,
            target_epoch,
            scratch: scratch.to_path_buf(),
            limits,
            policy,
        }
    }
    fn duplicate(&self) -> Self {
        Self::new(
            self.store,
            self.source_prefix,
            self.target_prefix,
            self.target_epoch,
            &self.scratch,
            self.limits,
            self.policy,
        )
    }
    fn head(
        &self,
        prefix: &str,
        work: &WorkContext,
    ) -> Result<Option<(HeadVersion, HeadRecord)>, Error> {
        Ok(self
            .bytes(&head_key(prefix), work)?
            .map(|(version, bytes)| {
                manifest::decode_head(&bytes, self.limits.envelope_bytes)
                    .map(|head| (version, head))
            })
            .transpose()?)
    }
    fn bytes(
        &self,
        key: &str,
        work: &WorkContext,
    ) -> Result<Option<(HeadVersion, Vec<u8>)>, Error> {
        work.checkpoint()?;
        Ok(
            match read_head_bounded(
                self.store,
                key,
                TransportContext::new(
                    work,
                    ReceiveLimits::capped(self.limits.envelope_bytes as u64),
                ),
            )? {
                ReceivedHead::Absent => None,
                ReceivedHead::Present { version, body } => Some((version, body)),
            },
        )
    }
    fn operation_key(&self, contract: &Contract, record: &str) -> String {
        format!(
            "{}/transitions/v1/{}/{record}",
            self.source_prefix,
            contract.operation.as_core()
        )
    }
    fn require_contract(&self, contract: &Contract, work: &WorkContext) -> Result<(), Error> {
        if self.source_prefix == self.target_prefix {
            return Err(local::Error::ContractMismatch.into());
        }
        if let Some((_, bytes)) = self.bytes(&self.operation_key(contract, "contract"), work)?
            && Contract::decode(&bytes, self.limits.envelope_bytes)? != *contract
        {
            return Err(local::Error::ContractMismatch.into());
        }
        Ok(())
    }
    // Immutable operation data uses conditional create, not content-addressed
    // object PUT: one operation identity cannot name two different contracts.
    fn retain(&self, key: &str, bytes: &[u8], work: &WorkContext) -> Result<(), Error> {
        work.checkpoint()?;
        let dispatched = self.store.create_head(key, bytes);
        if matches!(dispatched, Ok(ConditionalOutcome::Published { .. })) {
            return Ok(());
        }
        match self.bytes(key, work)? {
            Some((_, stored)) if stored == bytes => Ok(()),
            Some(_) => Err(local::Error::ContractMismatch.into()),
            None => Err(Error::Unresolved),
        }
    }
    fn source_head(
        &self,
        contract: &Contract,
        work: &WorkContext,
    ) -> Result<(HeadVersion, HeadRecord), Error> {
        let source = self
            .head(self.source_prefix, work)?
            .ok_or(LogError::NotInitialized)?;
        if source.1.control.identity != contract.source {
            return Err(local::Error::ContractMismatch.into());
        }
        Ok(source)
    }
    fn replace(
        &self,
        prefix: &str,
        version: &HeadVersion,
        parent: &HeadRecord,
        next: &HeadAuthority,
        work: &WorkContext,
    ) -> Result<bool, Error> {
        work.checkpoint()?;
        let bytes = manifest::encode_head(&parent.with_control(*next), self.limits.envelope_bytes)?;
        Ok(matches!(
            self.store.replace_head(&head_key(prefix), version, &bytes),
            Ok(ConditionalOutcome::Published { .. })
        ))
    }
    fn freeze(
        &self,
        contract: Contract,
        descriptor: &SchemaDescriptor,
        work: &WorkContext,
    ) -> Result<Option<Captured>, Error> {
        self.require_contract(&contract, work)?;
        // Verify the source before reserving any operation identity.
        self.source_head(&contract, work)?;
        self.retain(
            &self.operation_key(&contract, "contract"),
            &contract.encode(self.limits.envelope_bytes)?,
            work,
        )?;
        for _ in 0..ATTEMPTS {
            if self.settled_target(&contract, descriptor, work)? {
                return Ok(None);
            }
            let (version, parent) = self.source_head(&contract, work)?;
            let frozen = match parent
                .control
                .freeze(contract.operation, local::intent(&contract)?)
                .map_err(LogError::from)?
            {
                FreezeOutcome::Frozen(next) => {
                    if !self.replace(self.source_prefix, &version, &parent, &next, work)? {
                        continue;
                    }
                    next
                }
                FreezeOutcome::AlreadyFrozen { .. } => parent.control,
            };
            // Abort can finish while a source freeze CAS is in flight.
            // Settle that matching late freeze before recording a capture.
            if self.settled_target(&contract, descriptor, work)? {
                return Ok(None);
            }
            let position = frozen.position().ok_or(LogError::DatabaseDeleted)?;
            let captured = Captured {
                contract,
                decision: position.decision,
                state: position.state,
            };
            self.retain(
                &self.operation_key(&contract, "captured"),
                &captured.encode(self.limits.envelope_bytes)?,
                work,
            )?;
            return Ok(Some(captured));
        }
        Err(Error::Unresolved)
    }
    fn settled_target(
        &self,
        contract: &Contract,
        descriptor: &SchemaDescriptor,
        work: &WorkContext,
    ) -> Result<bool, Error> {
        let Some((_, target)) = self.head(self.target_prefix, work)? else {
            return Ok(false);
        };
        if local::activation_evidence(contract, &target.control)? == Some(Resolution::Aborted) {
            self.abort(contract, descriptor, work)?;
        }
        Ok(true)
    }
    /// Resolve remote evidence before deciding whether to run the transform.
    /// Ready contents are rehydrated and verified. Activated results return
    /// their original genesis even after subsequent target commands.
    /// # Errors
    /// Changed contracts, malformed evidence, native or transport failures.
    pub fn resolve(
        &self,
        contract: &Contract,
        descriptor: &SchemaDescriptor,
        work: &WorkContext,
    ) -> Result<Resolution, Error> {
        struct Remove(PathBuf);
        impl Drop for Remove {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        local::check_target_schema(contract, descriptor)?;
        self.require_contract(contract, work)?;
        let Some((_, target)) = self.head(self.target_prefix, work)? else {
            return Ok(Resolution::Uninstalled);
        };
        if let Some(terminal) = local::activation_evidence(contract, &target.control)? {
            return Ok(terminal);
        }
        let live = target.control.live().map_err(LogError::from)?;
        if live.access
            != (Access::Frozen {
                operation: contract.operation,
                intent: local::intent(contract)?,
            })
        {
            return Err(local::Error::ContractMismatch.into());
        }
        let recovery = target.recovery.ok_or(local::Error::OutputMismatch)?;
        if recovery.base != live.decision || recovery.tip != live.decision {
            return Err(local::Error::OutputMismatch.into());
        }
        let reference = recovery.checkpoint.ok_or(local::Error::OutputMismatch)?;
        let bytes = get_verified(
            self.store,
            self.target_prefix,
            &reference,
            TransportContext::new(work, ReceiveLimits::exact(reference.length)),
        )?;
        let checkpoint = crate::codec::decode_manifest(&bytes, self.policy.stream)?;
        if checkpoint.identity != contract.target
            || checkpoint.decision != live.decision
            || checkpoint.control_at_capture.lifecycle != target.control.lifecycle
        {
            return Err(local::Error::OutputMismatch.into());
        }
        let namespace = TargetNamespace::new(&self.scratch, contract.target.incarnation_id)?;
        let private = namespace.fresh_staging();
        if let Some(parent) = private.parent() {
            std::fs::create_dir_all(parent).map_err(NamespaceError::Io)?;
        }
        let _remove = Remove(private.clone());
        let staged = begin_staged(&private, descriptor.clone(), work)?;
        import_stream(
            &staged,
            &checkpoint,
            verified_chunks(self.store, self.target_prefix, &checkpoint.chunks, work),
            &mut |_, _| true,
            None,
            self.policy.stream,
            work,
        )?;
        let installed = staged.inspect(work, |read, _| {
            let result = (|| -> Result<Installed, Error> {
                let evidence = read
                    .snapshot()
                    .host_record(&local::evidence_key(contract))
                    .map_err(|error| local::Error::Core(bumbledb::Error::Store(Box::new(error))))?
                    .ok_or(local::Error::OutputMismatch)?;
                let installed = Installed::decode(evidence, self.limits.envelope_bytes)?;
                if installed.captured.contract != *contract
                    || local::digest_snapshot(read.snapshot(), contract, work)?
                        != installed.application_digest
                    || genesis_stamp(
                        &local::genesis(&installed, self.limits.envelope_bytes)?,
                        self.limits.envelope_bytes,
                    )? != live.decision
                {
                    return Err(local::Error::OutputMismatch.into());
                }
                Ok(installed)
            })();
            Ok(result)
        })??;
        staged.write_host(
            &[],
            Some(&encode_control(
                &target.control,
                self.limits.envelope_bytes,
            )?),
            work,
        )?;
        drop(staged.complete_install(work)?);
        Ok(Resolution::Ready(installed))
    }
    /// Capture the frozen source and start empty disk-backed population.
    /// # Errors
    /// Schema/contract refusal precedes freeze; ownership and transport fail
    /// explicitly. Dropping a returned population never thaws the source.
    pub fn begin<S>(
        &self,
        source: &Arc<Db<S>>,
        contract: Contract,
        descriptor: SchemaDescriptor,
        work: &WorkContext,
    ) -> Result<Begin<'a, S, B>, Error> {
        local::check_target_schema(&contract, &descriptor)?;
        if bumbledb::schema::fingerprint::fingerprint(source.schema()) != contract.source.schema_id
        {
            return Err(local::Error::ContractMismatch.into());
        }
        crate::writer::local::refuse_retired(source, work)?;
        let resolved = self.resolve(&contract, &descriptor, work)?;
        if resolved != Resolution::Uninstalled {
            return Ok(Begin::Resolved(resolved));
        }
        let namespace = TargetNamespace::new(&self.scratch, contract.target.incarnation_id)?;
        let attempt = namespace.population_lock()?;
        let Some(captured) = self.freeze(contract, &descriptor, work)? else {
            return Ok(Begin::Resolved(self.resolve(
                &contract,
                &descriptor,
                work,
            )?));
        };
        // An abort/activation may have won after the initial probe.
        let resolved = self.resolve(&contract, &descriptor, work)?;
        if resolved != Resolution::Uninstalled {
            if resolved == Resolution::Aborted {
                self.abort(&contract, &descriptor, work)?;
            }
            return Ok(Begin::Resolved(resolved));
        }
        let history = HostedHistory::open(
            Arc::clone(source),
            self.store,
            self.source_prefix.to_owned(),
            self.limits,
            work,
        )?;
        if history.catch_up(work)? != captured.decision {
            return Err(local::Error::OutputMismatch.into());
        }
        let reader = source.snapshot(work)?;
        let frame = reader.frame(work);
        let bytes = frame
            .integration_host_attachment()?
            .ok_or(LogError::NotInitialized)?;
        let control = crate::history::authority::decode_control(bytes, self.limits.envelope_bytes)?;
        if control.identity != contract.source
            || control
                .position()
                .is_none_or(|p| p.decision != captured.decision || p.state != captured.state)
            || control.live().map_err(LogError::from)?.access
                != (Access::Frozen {
                    operation: contract.operation,
                    intent: local::intent(&contract)?,
                })
        {
            return Err(local::Error::OutputMismatch.into());
        }
        namespace.clean_abandoned(&attempt)?;
        let private = namespace.fresh_staging();
        if let Some(parent) = private.parent() {
            std::fs::create_dir_all(parent).map_err(NamespaceError::Io)?;
        }
        let staged = begin_staged(&private, descriptor.clone(), work)?;
        Ok(Begin::Populating {
            source: reader,
            population: Population {
                hosted: self.duplicate(),
                staged: Some(staged),
                descriptor,
                captured,
                private,
                _attempt: attempt,
            },
        })
    }
    /// Activate precisely the inspected target. Lost responses are resolved
    /// from the target authority; complete retries do not inspect newer facts.
    /// # Errors
    /// Foreign evidence, cancellation, a won abort, or unresolved transport.
    pub fn activate(
        &self,
        installed: &Installed,
        descriptor: &SchemaDescriptor,
        work: &WorkContext,
    ) -> Result<Resolution, Error> {
        let contract = installed.captured.contract;
        let stamp = genesis_stamp(
            &local::genesis(installed, self.limits.envelope_bytes)?,
            self.limits.envelope_bytes,
        )?;
        match self.resolve(&contract, descriptor, work)? {
            Resolution::Ready(recorded) if recorded == *installed => {}
            complete @ Resolution::Activated { genesis, .. } if genesis == stamp.hash => {
                return Ok(complete);
            }
            Resolution::Aborted => return Err(local::Error::Aborted.into()),
            _ => return Err(local::Error::OutputMismatch.into()),
        }
        for _ in 0..ATTEMPTS {
            let (version, parent) = self
                .head(self.target_prefix, work)?
                .ok_or(local::Error::OutputMismatch)?;
            if let Some(terminal) = local::activation_evidence(&contract, &parent.control)? {
                return match terminal {
                    Resolution::Activated { genesis, .. } if genesis == stamp.hash => Ok(terminal),
                    _ => Err(local::Error::Aborted.into()),
                };
            }
            let next = match parent
                .control
                .activate(contract.operation, stamp.hash, local::cause(&contract)?)
                .map_err(LogError::from)?
            {
                ActivateOutcome::Activated(next) => next,
                ActivateOutcome::AlreadyActivated { .. } => parent.control,
            };
            if self.replace(self.target_prefix, &version, &parent, &next, work)? {
                return Ok(Resolution::Activated {
                    contract,
                    genesis: stamp.hash,
                });
            }
        }
        Err(Error::Unresolved)
    }
    /// Fence the target with the same create/replace race used by genesis
    /// and activation, then thaw only the matching source freeze.
    /// # Errors
    /// Activation winning forbids thaw. Uncertainty leaves source frozen.
    pub fn abort(
        &self,
        contract: &Contract,
        descriptor: &SchemaDescriptor,
        work: &WorkContext,
    ) -> Result<(), Error> {
        local::check_target_schema(contract, descriptor)?;
        self.require_contract(contract, work)?;
        let (_, source) = self.source_head(contract, work)?;
        if let Access::Frozen { operation, intent } =
            source.control.live().map_err(LogError::from)?.access
            && (operation != contract.operation || intent != local::intent(contract)?)
        {
            return Err(local::Error::ContractMismatch.into());
        }
        self.retain(
            &self.operation_key(contract, "contract"),
            &contract.encode(self.limits.envelope_bytes)?,
            work,
        )?;
        let mut fenced = false;
        for _ in 0..ATTEMPTS {
            match self.head(self.target_prefix, work)? {
                None => {
                    let tombstone = HeadRecord::cancelled_before_genesis(
                        HeadAuthority::cancelled_before_genesis(
                            contract.target,
                            contract.operation,
                            local::aborted_reason(contract)?,
                        ),
                        self.target_epoch,
                    );
                    let bytes = manifest::encode_head(&tombstone, self.limits.envelope_bytes)?;
                    work.checkpoint()?;
                    if matches!(
                        self.store
                            .create_head(&head_key(self.target_prefix), &bytes),
                        Ok(ConditionalOutcome::Published { .. })
                    ) {
                        fenced = true;
                        break;
                    }
                }
                Some((version, parent)) => {
                    match local::activation_evidence(contract, &parent.control)? {
                        Some(Resolution::Activated { .. }) => {
                            return Err(local::Error::ActivationWon.into());
                        }
                        Some(Resolution::Aborted) => {
                            fenced = true;
                            break;
                        }
                        _ => {}
                    }
                    let next = match parent
                        .control
                        .delete(contract.operation, local::aborted_reason(contract)?)
                        .map_err(LogError::from)?
                    {
                        DeleteOutcome::Deleted(next) => next,
                        DeleteOutcome::AlreadyDeleted { .. } => parent.control,
                    };
                    if self.replace(self.target_prefix, &version, &parent, &next, work)? {
                        fenced = true;
                        break;
                    }
                }
            }
        }
        if !fenced {
            return Err(Error::Unresolved);
        }
        for _ in 0..ATTEMPTS {
            let (version, parent) = self.source_head(contract, work)?;
            match parent.control.live().map_err(LogError::from)?.access {
                Access::Active => return Ok(()),
                Access::Frozen { operation, intent }
                    if operation == contract.operation && intent == local::intent(contract)? =>
                {
                    let next = parent
                        .control
                        .thaw(contract.operation)
                        .map_err(LogError::from)?;
                    if self.replace(self.source_prefix, &version, &parent, &next, work)? {
                        return Ok(());
                    }
                }
                Access::Frozen { .. } => return Err(local::Error::ContractMismatch.into()),
            }
        }
        Err(Error::Unresolved)
    }
}

impl<B: ReceivingStore> Population<'_, B>
where
    B::Error: BackendError + ObservedError,
{
    #[must_use]
    pub const fn captured(&self) -> Captured {
        self.captured
    }
    /// Sequential ordinary changes; no whole-target work per batch.
    /// # Errors
    /// Wrong schemas, terminal target fences, and native staging failures.
    pub fn apply(&mut self, changes: &ChangeSet, work: &WorkContext) -> Result<(), Error> {
        if changes.schema() != self.captured.contract.target.schema_id {
            return Err(local::Error::ContractMismatch.into());
        }
        work.checkpoint()?;
        self.require_uninstalled(work)?;
        self.staged
            .as_ref()
            .ok_or(local::Error::Aborted)?
            .apply_unjudged(changes, work)?;
        Ok(())
    }
    fn require_uninstalled(&self, work: &WorkContext) -> Result<(), Error> {
        let contract = &self.captured.contract;
        if let Some((_, target)) = self.hosted.head(self.hosted.target_prefix, work)? {
            local::activation_evidence(contract, &target.control)?;
            return Err(local::Error::Aborted.into());
        }
        Ok(())
    }
    /// Consume population, judge/hash once, upload one complete checkpoint,
    /// and conditionally publish genesis. Resolve after any uncertain error.
    /// # Errors
    /// Law rejection, fencing, conflicting output, or native/transport failure.
    pub fn finish(mut self, work: &WorkContext) -> Result<Resolution, Error> {
        self.require_uninstalled(work)?;
        let staged = self.staged.take().ok_or(local::Error::Aborted)?;
        let installed = local::admit_target(staged, self.captured, None, self.hosted.limits, work)?;
        let db = Db::open(&self.private, self.descriptor.clone(), work.clone())?;
        let (checkpoint, reference) = upload_snapshot(
            &db,
            self.hosted.store,
            self.hosted.target_prefix,
            self.hosted.target_epoch,
            0,
            &self.hosted.policy,
            work,
        )?;
        drop(db);
        let mut record =
            HeadRecord::genesis(checkpoint.control_at_capture, self.hosted.target_epoch)?;
        record.recovery = Some(RecoveryRoot::checkpoint_only(
            Some(reference),
            checkpoint.decision,
            0,
            self.hosted.target_epoch,
        ));
        let bytes = manifest::encode_head(&record, self.hosted.limits.envelope_bytes)?;
        work.checkpoint()?;
        if matches!(
            self.hosted
                .store
                .create_head(&head_key(self.hosted.target_prefix), &bytes),
            Ok(ConditionalOutcome::Published { .. })
        ) {
            return Ok(Resolution::Ready(installed));
        }
        match self
            .hosted
            .resolve(&self.captured.contract, &self.descriptor, work)?
        {
            ready @ Resolution::Ready(recorded) if recorded == installed => Ok(ready),
            complete @ Resolution::Activated { genesis, .. }
                if genesis == checkpoint.decision.hash =>
            {
                Ok(complete)
            }
            Resolution::Uninstalled => Err(Error::Unresolved),
            Resolution::Aborted => Err(local::Error::Aborted.into()),
            _ => Err(local::Error::OutputMismatch.into()),
        }
    }
}
