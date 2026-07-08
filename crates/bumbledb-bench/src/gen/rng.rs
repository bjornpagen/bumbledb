use crate::gen::Rng;

impl Rng {
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
