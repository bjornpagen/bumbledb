/// # Panics
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
