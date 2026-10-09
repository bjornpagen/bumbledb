//! Capability close: the JS wrapper is never the retention authority.

use std::sync::Arc;

use crate::runtime::registry::Capability;
use crate::runtime::{Report, Runtime};

/// Deterministic native close. Repeated close joins one drain.
pub(crate) fn close_admitted(runtime: &Arc<Runtime>, cap: Capability, report: Report) {
    if let Err(error) = runtime.close_resource(cap, report) {
        let _ = error;
    }
}
