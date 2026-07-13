//! The entropy seam: one `Rng`, two sources. Every generator draws
//! through [`Rng`]'s primitives and never sees the concrete source —
//! the seam between "seeded reproducible run" and "fuzzer-driven run"
//! is the entropy source and nothing else.

/// Where generation entropy comes from. `Seeded` is the
/// bench/differential arm and must remain byte-identical to the
/// pre-seam stream — the corpus digest pin arbitrates. `Bytes` is the
/// fuzzer arm: draws consume the fuzzer's data; exhaustion falls back
/// to a fixed deterministic tail (zeros), never a panic — libFuzzer
/// shrinks better when short inputs are legal.
#[derive(Debug, Clone)]
pub enum Rng {
    /// The seeded generator, unchanged — today's deterministic stream.
    Seeded(XorShift),
    /// A cursor over fuzzer-provided `&[u8]`.
    Bytes(ByteSource),
}

/// The house LCG (the engine's test constants): deterministic, fast,
/// and dependency-free — the seeded arm's concrete source. Generator
/// logic never touches this type; it draws through [`Rng`].
#[derive(Debug, Clone)]
pub struct XorShift {
    state: u64,
}

impl XorShift {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            // Scramble the seed so small seeds diverge immediately.
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    pub fn u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state >> 33
    }
}

/// The fuzzer arm's concrete source: a cursor consuming fuzzer bytes,
/// zero forever once exhausted — a corpus byte string maps stably onto
/// generation decisions (stability is what makes libFuzzer's mutations
/// meaningful), and short inputs complete instead of panicking.
#[derive(Debug, Clone)]
pub struct ByteSource {
    data: Box<[u8]>,
    cursor: usize,
}

impl ByteSource {
    #[must_use]
    pub fn new(data: &[u8]) -> Self {
        Self {
            data: data.into(),
            cursor: 0,
        }
    }

    /// The next word, little-endian; missing bytes read as zero (the
    /// deterministic tail).
    pub fn u64(&mut self) -> u64 {
        let mut word = [0u8; 8];
        let rest = &self.data[self.cursor..];
        let take = rest.len().min(8);
        word[..take].copy_from_slice(&rest[..take]);
        self.cursor += take;
        u64::from_le_bytes(word)
    }
}

impl Rng {
    /// The seeded arm — the bench/differential constructor.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self::Seeded(XorShift::new(seed))
    }

    /// The fuzzer arm — generation steered by a fuzzer's byte stream.
    #[must_use]
    pub fn from_bytes(data: &[u8]) -> Self {
        Self::Bytes(ByteSource::new(data))
    }

    /// The one raw draw — the single point where the variant matters.
    /// Every bounded draw reduces this word identically across arms, so
    /// the two sources map onto generation decisions the same way.
    pub fn u64(&mut self) -> u64 {
        match self {
            Self::Seeded(seeded) => seeded.u64(),
            Self::Bytes(bytes) => bytes.u64(),
        }
    }

    /// A value in `0..n` (`n > 0`).
    pub fn range(&mut self, n: u64) -> u64 {
        debug_assert!(n > 0);
        self.u64() % n
    }

    /// True with probability `num/den`.
    pub fn chance(&mut self, num: u64, den: u64) -> bool {
        self.range(den) < num
    }
}

#[cfg(test)]
mod tests {
    use super::Rng;
    use crate::corpus_gen::{GenConfig, Scale, corpus_digest, digest_hex};
    use crate::querygen;

    /// One full byte-driven generation pass at `Scale::Tiny`, rendered
    /// to a comparable string: the schema is the fixed target theory,
    /// the data identity is the corpus digest (every ledger and
    /// calendar relation streamed whole), and the ops are the query
    /// draws, their param draws, and the judgment write cases.
    fn artifacts(bytes: &[u8]) -> String {
        let mut rng = Rng::from_bytes(bytes);
        let cfg = GenConfig {
            seed: rng.u64(),
            scale: Scale::Tiny,
        };
        let data = digest_hex(&corpus_digest(cfg));
        let queries: Vec<_> = (0..8)
            .map(|_| querygen::random_query(&mut rng, cfg))
            .collect();
        let params: Vec<_> = queries
            .iter()
            .map(|query| querygen::params_for(query, &mut rng, cfg))
            .collect();
        let writes = querygen::writes::closed_write_cases(&mut rng, 12);
        format!("{data} {queries:?} {params:?} {writes:?}")
    }

    /// The fuzzer arm is deterministic in its own right: the same byte
    /// string drives the identical schema+data+ops generation twice —
    /// and a different byte string steers elsewhere.
    #[test]
    fn the_bytes_arm_generates_identically_from_the_same_bytes() {
        let bytes: Vec<u8> = (1..=512u64)
            .flat_map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15).to_le_bytes())
            .collect();
        let first = artifacts(&bytes);
        assert_eq!(first, artifacts(&bytes), "same bytes, same artifacts");
        let other: Vec<u8> = (1..=512u64)
            .flat_map(|i| i.wrapping_mul(0xC2B2_AE3D_27D4_EB4F).to_le_bytes())
            .collect();
        assert_ne!(first, artifacts(&other), "bytes steer generation");
    }

    /// Exhaustion is legal: a short (even empty) input completes the
    /// full generation on the deterministic zero tail, no panic.
    #[test]
    fn a_short_input_completes_on_the_zero_tail() {
        let mut rng = Rng::from_bytes(&[0xAB, 0xCD, 0xEF]);
        assert_eq!(rng.u64(), 0x00EF_CDAB, "partial word, zero-filled");
        assert_eq!(rng.u64(), 0, "exhausted: the zero tail");
        assert_eq!(rng.range(97), 0, "bounded draws reduce the tail");
        let short = artifacts(&[7, 7, 7]);
        assert_eq!(short, artifacts(&[7, 7, 7]), "the tail is deterministic");
        let empty = artifacts(&[]);
        assert_eq!(empty, artifacts(&[]), "zero-length input is legal");
    }
}
