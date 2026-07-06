//! A stable streaming blake3 wrapper (reader: the bench crate's corpus
//! identity — `docs/architecture/50-validation.md`). The dependency
//! quarantine keeps `blake3` out of `bumbledb-bench`; this thin surface
//! lends the hash without leaking the dependency's types.

/// An incremental 256-bit digest.
#[derive(Debug, Default)]
pub struct Digest(blake3::Hasher);

impl Digest {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends bytes.
    pub fn update(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }

    /// Finishes into the 32-byte digest.
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
