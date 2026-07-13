//! The crash target: durability under torn commits — an ops prefix and
//! one victim commit replayed in a child process that aborts at a drawn
//! crashpoint; the parent proves all-or-nothing recovery. Thin by
//! charter; the runner (and the child path, env-var-steered into this
//! same binary) lives in the shared harness.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| bumbledb_fuzz::crash::run(data));
