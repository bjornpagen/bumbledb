//! Internal repository-lock Effect bridge (C8 / TS-014).
//!
//! Generation holds L11's kernel directory exclusion over a persistent
//! inode. The capability is minted with L12 `NativeKind::RepositoryLock`
//! (`Runtime::mint_repository_lock`) **before** the handle returns to JS.
//! `cap.kind` is the stamp — this is not a directory-owner twin. Release
//! is idempotent and joins drain. This is not a second public DB/file-lock API.

use std::sync::atomic::{AtomicBool, Ordering};

use bumbledb::work::WorkContext;
use bumbledb_log::store::fence::{RepositoryLock, acquire_repository_lock};
use napi::bindgen_prelude::{Env, External, Function};
use napi_derive::napi;

use crate::runtime::registry::{NativeKind, RegistryAdmission};
use crate::runtime::{Output, Runtime, RuntimeError};
use crate::runtime_wire::{
    CloseWire, OperationHandle, PolicyWire, RuntimeHandle, notification, operation_handle,
    owner as runtime_owner, reporter, take_output, thrown,
};

use super::{LogFail, MachineOutput};

/// Opaque capability: L12 `NativeKind::RepositoryLock` + L11 kernel lock.
pub struct RepositoryLockHandle {
    identity: usize,
    admission: RegistryAdmission,
    released: AtomicBool,
}

pub struct RepositoryLockOwned {
    pub(crate) lock: Option<RepositoryLock>,
    pub(crate) directory: String,
}

impl Drop for RepositoryLockOwned {
    fn drop(&mut self) {
        // Abandoned output: drop the kernel lock. Never unlink owner.lock.
        drop(self.lock.take());
    }
}

fn stamped_lock(admission: &RegistryAdmission) -> Result<crate::runtime::Capability, RuntimeError> {
    let cap = admission.cap();
    if cap.kind != NativeKind::RepositoryLock {
        return Err(RuntimeError::Internal);
    }
    Ok(cap)
}

/// Acquire the kernel repository lock on the worker. The stamped capability
/// is minted at take — never an unstamped directory-owner twin.
#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn log_repository_lock_acquire(
    env: Env,
    handle: &External<RuntimeHandle>,
    policy: PolicyWire,
    directory: String,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = runtime_owner(handle).map_err(|error| thrown(env, error))?;
    if directory.is_empty() {
        return Err(thrown(env, RuntimeError::InvalidPath));
    }
    let operation = runtime
        .submit(
            policy.parse().map_err(|error| thrown(env, error))?,
            notification(callback)?,
            move |context| {
                context.input(directory.len() as u64)?;
                Ok(Box::new(move |context: &WorkContext| {
                    context.checkpoint()?;
                    let held = match acquire_repository_lock(std::path::Path::new(&directory)) {
                        Ok(held) => held,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            return Ok(super::fail_output(LogFail::Core(
                                RuntimeError::DirectoryBusy,
                            )));
                        }
                        Err(error) => return Err(crate::runtime::owners::io_error(error)),
                    };
                    if context.checkpoint().is_err() {
                        drop(held);
                        return Err(RuntimeError::ClosedHandle);
                    }
                    Ok(Output::Machine(MachineOutput::RepositoryLock(
                        RepositoryLockOwned {
                            lock: Some(held),
                            directory,
                        },
                    )))
                }))
            },
        )
        .map_err(|error| thrown(env, error))?;
    Ok(operation_handle(runtime, operation))
}

/// Take a registered lock. Mint stamps `NativeKind::RepositoryLock` before
/// the handle returns to JS. Abandoned take drops the kernel lock.
#[napi]
pub fn log_repository_lock_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<External<RepositoryLockHandle>> {
    let runtime = crate::runtime_wire::operation_runtime(handle);
    match take_output(env, handle)? {
        Output::Machine(MachineOutput::RepositoryLock(mut owned)) => {
            let lock = owned
                .lock
                .take()
                .ok_or_else(|| thrown(env, RuntimeError::InvalidArgument))?;
            let admission = runtime
                .mint_repository_lock(lock)
                .map_err(|error| thrown(env, error))?;
            let cap = stamped_lock(&admission).map_err(|error| thrown(env, error))?;
            let _ = cap;
            Ok(External::new(RepositoryLockHandle {
                identity: crate::runtime_wire::addon_identity(),
                admission,
                released: AtomicBool::new(false),
            }))
        }
        Output::Machine(MachineOutput::Admin(super::AdminOwned::Failed { fail, .. })) => {
            Err(super::throw_frame(env, &fail))
        }
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}

/// Idempotent joined release. Repeated close joins one drain. A stale
/// token cannot unlock a successor (the kernel lock lives on the inode).
#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn log_repository_lock_release(
    env: Env,
    handle: &External<RepositoryLockHandle>,
    callback: Function<CloseWire, ()>,
) -> napi::Result<()> {
    if handle.identity != crate::runtime_wire::addon_identity() {
        return Err(thrown(env, RuntimeError::ForeignRuntime));
    }
    let report = reporter(callback)?;
    if handle.released.swap(true, Ordering::AcqRel) {
        report(crate::runtime::CloseReport::Closed);
        return Ok(());
    }
    let cap = stamped_lock(&handle.admission).map_err(|error| thrown(env, error))?;
    handle
        .admission
        .runtime
        .close_resource(cap, report)
        .map_err(|error| thrown(env, error))?;
    Ok(())
}

/// Same-process / process-death exclusion over L11's persistent inode.
/// Minted handles carry `NativeKind::RepositoryLock`.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{CloseReport, Options};
    use std::time::Duration;

    fn options() -> Options {
        Options {
            workers: 2,
            queue_capacity: 8,
            cleanup_capacity: 8,
            owner_capacity: 8,
            native_handle_capacity: 16,
            aggregate_bytes: [64 << 20; 4],
            chunk_bytes: 1 << 20,
            cleanup_timeout: Duration::from_millis(500),
        }
    }

    fn unique_dir(tag: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "bumbledb-l14-lock-{tag}-{}-{seq}",
            std::process::id()
        ))
    }

    #[test]
    fn d28_same_process_duplicate_refuses_and_release_is_idempotent() {
        let runtime = Runtime::start(options()).unwrap();
        let dir = unique_dir("same-process");
        std::fs::create_dir_all(&dir).unwrap();
        let first = acquire_repository_lock(&dir).expect("first owner");
        #[cfg(unix)]
        let inode = first.lock_inode().expect("persistent inode");
        let second = acquire_repository_lock(&dir);
        assert_eq!(
            second.as_ref().map(|_| ()).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock,
            "a live owner is exclusive; lock body is irrelevant"
        );
        drop(first);
        let successor = acquire_repository_lock(&dir).expect("process-death/release reacquires");
        #[cfg(unix)]
        {
            let again = successor.lock_inode().expect("same inode");
            assert_eq!(inode, again, "the lock file inode is persistent");
        }
        drop(successor);
        let (tx, rx) = std::sync::mpsc::channel();
        runtime.drain(
            None,
            Box::new(move |report| {
                tx.send(report).unwrap();
            }),
        );
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(10)).expect("drain"),
            CloseReport::Closed
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn d18_refused_acquire_does_not_mint_a_lock_capability() {
        let runtime = Runtime::start(options()).unwrap();
        let dir = unique_dir("abandon");
        std::fs::create_dir_all(&dir).unwrap();
        let baseline = runtime.inspect().natives;
        let held = acquire_repository_lock(&dir).expect("occupies the inode");
        let busy = acquire_repository_lock(&dir);
        assert_eq!(
            busy.as_ref().map(|_| ()).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        assert_eq!(
            runtime.inspect().natives, baseline,
            "a refused acquire must not mint NativeKind::RepositoryLock"
        );
        drop(held);
        let recovered = acquire_repository_lock(&dir).expect("refused acquire leaked no fence");
        drop(recovered);
        let (tx, rx) = std::sync::mpsc::channel();
        runtime.drain(
            None,
            Box::new(move |report| {
                tx.send(report).unwrap();
            }),
        );
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(10)).expect("drain"),
            CloseReport::Closed
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn d18_minted_lock_capability_kind_is_repository_lock() {
        let runtime = Runtime::start(options()).unwrap();
        let dir = unique_dir("stamp");
        std::fs::create_dir_all(&dir).unwrap();
        let held = acquire_repository_lock(&dir).expect("kernel lock");
        let admission = runtime
            .mint_repository_lock(held)
            .expect("L12 stamps RepositoryLock");
        assert_eq!(
            admission.cap().kind,
            NativeKind::RepositoryLock,
            "cap.kind is the lock stamp; not a directory-owner twin"
        );
        let (tx, rx) = std::sync::mpsc::channel();
        runtime
            .close_resource(
                admission.cap(),
                Box::new(move |report| {
                    tx.send(report).unwrap();
                }),
            )
            .expect("stamped lock drains");
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(10)).expect("drain"),
            CloseReport::Closed
        );
        let (tx, rx) = std::sync::mpsc::channel();
        runtime.drain(
            None,
            Box::new(move |report| {
                tx.send(report).unwrap();
            }),
        );
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(10)).expect("runtime"),
            CloseReport::Closed
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
