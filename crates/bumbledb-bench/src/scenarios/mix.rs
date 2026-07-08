/// Deterministic per-row seed (the same construction as the ledger
/// generator's: corpus content is a pure function of (seed, rel, row)).
#[must_use]
pub fn mix(seed: u64, rel: u32, row: u64) -> u64 {
    let mut h = seed ^ 0x9E37_79B9_7F4A_7C15;
    h ^= u64::from(rel).wrapping_mul(0xA24B_AED4_963E_E407);
    h ^= row.wrapping_mul(0x9FB2_1C65_1E98_DF25);
    h ^= h >> 28;
    h = h.wrapping_mul(0x2545_F491_4F6C_DD1D);
    h ^ (h >> 28)
}
