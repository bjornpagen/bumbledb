//! One-shot publication-gap cancel (D12/D25). L16 declares
//! `runtimeArmPublicationCancel`; this is the native symbol it calls.
//!
//! The hook cancels the actual operation immediately before the same
//! `PublicationSink::accept` gate used by ordinary interruption.
//! No public scheduling debug API.

use napi::bindgen_prelude::{Env, External};
use napi_derive::napi;

use crate::runtime_wire::{RuntimeHandle, owner, thrown};

/// Arm the next `dispatch_payload_message` / `run_payload_publication`.
/// After `work()` returns a page and before `operation.output` is written,
/// the local owner is dropped and the job fails `Cancelled`. A page already
/// registered is kept. Predelivery `Err` still publishes nothing.
#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_arm_publication_cancel(
    env: Env,
    handle: &External<RuntimeHandle>,
) -> napi::Result<()> {
    let runtime = owner(handle).map_err(|error| thrown(env, error))?;
    runtime.arm_publication_cancel();
    Ok(())
}
