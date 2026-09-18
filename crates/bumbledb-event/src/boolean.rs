/// A binary truth function. Bit `(a << 1) | b` is its result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoolOp4(u8);

impl BoolOp4 {
    pub(crate) const CELLS: [Self; 4] = [Self(1), Self(2), Self(4), Self(8)];
    pub const FALSE: Self = Self(0);
    pub const DIFFERENCE: Self = Self(4);
    pub const XOR: Self = Self(6);
    pub const AND: Self = Self(8);
    pub const IMPLIES: Self = Self(11);
    pub const OR: Self = Self(14);
    pub const TRUE: Self = Self(15);

    #[must_use]
    pub const fn new(bits: u8) -> Option<Self> {
        if bits < 16 { Some(Self(bits)) } else { None }
    }

    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    #[must_use]
    pub const fn evaluate(self, a: bool, b: bool) -> bool {
        self.0 >> ((a as u8) * 2 + b as u8) & 1 != 0
    }

    #[must_use]
    pub const fn complement(self) -> Self {
        Self(self.0 ^ 15)
    }

    #[must_use]
    pub const fn converse(self) -> Self {
        Self((self.0 & 9) | ((self.0 & 2) << 1) | ((self.0 & 4) >> 1))
    }

    pub(crate) const fn word(self, a: u64, b: u64) -> u64 {
        let low = (if self.0 & 1 != 0 { !b } else { 0 }) | (if self.0 & 2 != 0 { b } else { 0 });
        let high = (if self.0 & 4 != 0 { !b } else { 0 }) | (if self.0 & 8 != 0 { b } else { 0 });
        (a & high) | (!a & low)
    }
}

/// Nonempty cells of a pair's Venn diagram in an inhabited Event space.
/// This is neither a truth function nor a probability-independence judgment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Signature(pub(crate) u8);

impl Signature {
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    #[must_use]
    pub const fn possible(self, operation: BoolOp4) -> bool {
        self.0 & operation.bits() != 0
    }

    #[must_use]
    pub const fn full(self, operation: BoolOp4) -> bool {
        self.0 & operation.complement().bits() == 0
    }

    #[must_use]
    pub const fn included(self) -> bool {
        self.0 & 4 == 0
    }

    #[must_use]
    pub const fn equal(self) -> bool {
        self.0 & 6 == 0
    }

    #[must_use]
    pub const fn disjoint(self) -> bool {
        self.0 & 8 == 0
    }
}
