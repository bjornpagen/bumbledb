#![no_main]

use bumbledb_core::datalog::parse;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(source) = std::str::from_utf8(data) {
        let _ = parse(source);
    }
});
