//! A test hook for the publication gap: arm it, and the next payload
//! publication is cancelled after its page is produced and before its
//! delivery is accepted, through the same gate ordinary interruption uses.

use napi::bindgen_prelude::External;
use napi_derive::napi;

use crate::runtime_wire::{RuntimeHandle, owner};

/// Cancel the next payload publication between producing a page and
/// accepting its delivery. A page already accepted is kept.
#[napi]
pub fn runtime_arm_publication_cancel(handle: &External<RuntimeHandle>) {
    owner(handle).arm_publication_cancel();
}
