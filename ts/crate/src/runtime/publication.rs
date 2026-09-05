//! One-shot publication-gap cancel (D12/D25). L16 declares
//! `runtimeArmPublicationCancel`; this is the native symbol it calls.
//!
//! Armed: [`crate::db_wire::reject_publication`] then drop the local page.
//! Accept: live-ticket `PublicationSink::accept` or write plus
//! [`crate::db_wire::accept_publication`] as one locked transition.
//! No public scheduling debug API.

use napi::bindgen_prelude::{Env, External};
use napi_derive::napi;

use crate::runtime_wire::{owner, thrown, RuntimeHandle};

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
