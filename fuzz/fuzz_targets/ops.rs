//! The ops target: the flagship lifecycle interleaver — op sequences
//! against the live engine with the naive model in lockstep. Thin by
//! charter; the runner lives in the shared harness.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| bumbledb_fuzz::ops(data));
