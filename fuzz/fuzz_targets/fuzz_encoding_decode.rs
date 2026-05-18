#![no_main]

use bumbledb_core::encoding::{
    decode_bool, decode_decimal, decode_i64, decode_i128, decode_intern_id, decode_timestamp,
    decode_u64, decode_uuid,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = decode_bool(data);
    let _ = decode_u64(data);
    let _ = decode_i64(data);
    let _ = decode_i128(data);
    let _ = decode_decimal(data);
    let _ = decode_timestamp(data);
    let _ = decode_uuid(data);
    let _ = decode_intern_id(data);
});
