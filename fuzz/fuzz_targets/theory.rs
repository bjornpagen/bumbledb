//! The theory target: schema acceptance as the trust root — everything
//! else assumes accepted schemas are sound. Thin by charter; the runner
//! lives in the shared harness.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| bumbledb_fuzz::theory(data));
