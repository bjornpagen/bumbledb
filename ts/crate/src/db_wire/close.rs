//! Capability close: the JS wrapper is never the retention authority.
//! L12 owns the worker-table drain; L13 only requests the already-admitted
//! close obligation.

use std::sync::Arc;

use crate::runtime::registry::{Capability, RegistryAdmission};
use crate::runtime::{Report, Runtime};

/// Deterministic native close. Repeated close joins one drain. Heavy
/// destruction runs on the control lane when the resource is still live.
pub(crate) fn close_admitted(
    runtime: &Arc<Runtime>,
    cap: Capability,
    _admission: &RegistryAdmission,
    report: Report,
) {
    if let Err(error) = runtime.close_resource(cap, report) {
        let _ = error;
    }
}

/// Control-lane teardown used by L14 for abandoned owners. Not a
/// QueueFull-prone ordinary job.
pub(crate) fn spawn_teardown(
    runtime: &Arc<Runtime>,
    report: crate::runtime::Report,
    work: impl FnOnce() + Send + 'static,
) {
    let _ = runtime.submit_control(Box::new(work), Some(report));
}
