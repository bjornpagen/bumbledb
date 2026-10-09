/// The deterministic `SplitMix64` stream every generator draws from.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
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
    use crate::oracle::querygen;
    use crate::worlds::corpus_gen::{GenConfig, Scale, corpus_digest, digest_hex};

    fn artifacts(seed: u64) -> String {
        let mut rng = Rng::new(seed);
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
    fn one_seed_generates_identically_and_seeds_steer_generation() {
        let first = artifacts(1);
        assert_eq!(first, artifacts(1), "same seed, same artifacts");
        assert_ne!(first, artifacts(2), "seeds steer generation");
    }

    #[test]
    fn the_stream_emits_full_width_words() {
        let mut rng = Rng::new(1);
        let mut acc = 0u64;
        for _ in 0..64 {
            acc |= rng.u64();
        }
        assert_eq!(acc, u64::MAX, "all 64 bit positions reachable");
    }
}
