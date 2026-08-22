#[derive(Debug, Clone)]
pub enum Rng {
    Seeded(SplitMix),

    Bytes(ByteSource),
}

#[derive(Debug, Clone)]
pub struct SplitMix {
    state: u64,
}

impl SplitMix {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut word = self.state;
        word = (word ^ (word >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        word = (word ^ (word >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        word ^ (word >> 31)
    }
}

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
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self::Seeded(SplitMix::new(seed))
    }

    #[must_use]
    pub fn from_bytes(data: &[u8]) -> Self {
        Self::Bytes(ByteSource::new(data))
    }

    pub fn u64(&mut self) -> u64 {
        match self {
            Self::Seeded(seeded) => seeded.u64(),
            Self::Bytes(bytes) => bytes.u64(),
        }
    }

    pub fn range(&mut self, n: u64) -> u64 {
        debug_assert!(n > 0);
        self.u64() % n
    }

    pub fn chance(&mut self, num: u64, den: u64) -> bool {
        self.range(den) < num
    }
}

#[cfg(test)]
mod tests {
    use super::Rng;
    use crate::corpus_gen::{GenConfig, Scale, corpus_digest, digest_hex};
    use crate::querygen;

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

    /// `range(n)` is sound for any bound (ruled 2026-07-23, R20).
    #[test]
    fn the_seeded_arm_emits_full_width_words() {
        let mut rng = Rng::new(1);
        let mut acc = 0u64;
        for _ in 0..64 {
            acc |= rng.u64();
        }
        assert_eq!(acc, u64::MAX, "all 64 bit positions reachable");
    }

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
