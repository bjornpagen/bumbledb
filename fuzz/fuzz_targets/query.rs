//! The query target: three-way parity (engine / naive model / SQLite)
//! on the valid arm, validation totality on the hostile arm. Thin by
//! charter; the runner lives in the shared harness.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| bumbledb_fuzz::query::run(data));
