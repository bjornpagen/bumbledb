//! The rewrites target: the dual-pipeline differential — the same query
//! through the rewritten (grounding + fold) and rewrite-free pipelines must
//! produce identical result sets. Thin by charter; the runner lives in
//! the shared harness.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| bumbledb_fuzz::rewrites::run(data));
