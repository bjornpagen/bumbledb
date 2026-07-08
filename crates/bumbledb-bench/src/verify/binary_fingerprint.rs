/// The running binary's blake3 fingerprint, computed once per process.
/// One hash covers the engine, the translator, the comparator, the
/// generator, and every param policy at once — a stamp bound to it
/// vouches for the exact code that earned it. Consequences, accepted:
/// any rebuild re-keys the stamp (over-invalidation by embedded paths
/// included — re-verification is the honest default), and
/// [`stamp_matches`] fails for any binary other than the one that
/// earned the stamp, which is precisely the contract.
///
/// # Panics
///
/// On tool-level I/O failure reading the running executable.
#[must_use]
pub fn binary_fingerprint() -> [u8; 32] {
    static FINGERPRINT: std::sync::OnceLock<[u8; 32]> = std::sync::OnceLock::new();
    *FINGERPRINT.get_or_init(|| {
        let exe = std::env::current_exe().expect("current_exe");
        let bytes = std::fs::read(exe).expect("read the running binary");
        let mut digest = bumbledb::digest::Digest::new();
        digest.update(&bytes);
        digest.finalize()
    })
}
