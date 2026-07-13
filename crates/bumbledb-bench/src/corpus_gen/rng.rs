//! The entropy seam: one `Rng`, two sources. Every generator draws
//! through [`Rng`]'s primitives and never sees the concrete source —
//! the seam between "seeded reproducible run" and "fuzzer-driven run"
//! is the entropy source and nothing else.

/// Where generation entropy comes from. `Seeded` is the
/// bench/differential arm and must remain byte-identical to the
/// pre-seam stream — the corpus digest pin arbitrates.
#[derive(Debug, Clone)]
pub enum Rng {
    /// The seeded generator, unchanged — today's deterministic stream.
    Seeded(XorShift),
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

impl Rng {
    /// The seeded arm — the bench/differential constructor.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self::Seeded(XorShift::new(seed))
    }

    /// The one raw draw — the single point where the variant matters.
    /// Every bounded draw reduces this word identically across arms, so
    /// the two sources map onto generation decisions the same way.
    pub fn u64(&mut self) -> u64 {
        match self {
            Self::Seeded(seeded) => seeded.u64(),
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
