/// Hex rendering of a digest (directory names, stamps, goldens).
#[must_use]
pub fn digest_hex(digest: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    digest.iter().fold(String::new(), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    })
}
