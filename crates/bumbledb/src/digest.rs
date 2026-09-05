//! A stable streaming blake3 wrapper. The dependency
//! quarantine keeps `blake3` out of `bumbledb-bench`; this thin surface
//! lends the hash without leaking the dependency's types.
//!
//! Role separation (chapter 41): this full 32-byte digest is the
//! **authoritative content-identity** width — schema fingerprints,
//! command/decision commitments and remote objects. It is never truncated
//! by a generic helper. The 16-byte exact-checked **local fact
//! fingerprint** is a different role with its own domain-separated
//! constructor (`crate::encoding::fact_fingerprint`); local tuple equality
//! is decided by full canonical bytes, not by either hash.
//! An incremental 256-bit digest.
#[derive(Debug, Default)]
pub struct Digest(blake3::Hasher);

impl Digest {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }

    #[must_use]
    pub fn finalize(&self) -> [u8; 32] {
        *self.0.finalize().as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_equals_one_shot() {
        let mut a = Digest::new();
        a.update(b"hello ");
        a.update(b"world");
        let mut b = Digest::new();
        b.update(b"hello world");
        assert_eq!(a.finalize(), b.finalize());
        assert_ne!(a.finalize(), Digest::new().finalize());
    }
}
