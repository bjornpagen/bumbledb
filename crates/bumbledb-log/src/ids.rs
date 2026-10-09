//! Identifiers and coordinates shared by entries, receipts and the machine.

use std::num::NonZeroU64;

macro_rules! bytes_role {
    ($name:ident, $len:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub [u8; $len]);
    };
}

bytes_role!(
    DatabaseId,
    16,
    "One database's identity, minted by its Genesis entry."
);
bytes_role!(
    RequestId,
    16,
    "The caller's idempotency key: a request is decided at most once."
);
bytes_role!(
    Nonce,
    16,
    "Unique per written entry, so two writers never produce identical bytes."
);
bytes_role!(
    CommandDigest,
    32,
    "BLAKE3 binding of a command's request, precondition and changes."
);
bytes_role!(
    ImageDigest,
    32,
    "BLAKE3 of a checkpoint or migration image file."
);
bytes_role!(
    MigrationHash,
    32,
    "Content hash of one bundled migration, verified against the ledger."
);

/// A log position. `log/{seq}` holds the entry at `seq`; Genesis is 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Seq(NonZeroU64);

impl Seq {
    pub const GENESIS: Self = Self(NonZeroU64::MIN);

    #[must_use]
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    /// The following position; the log never reaches `u64::MAX` entries.
    #[must_use]
    pub const fn next(self) -> Self {
        match self.0.checked_add(1) {
            Some(next) => Self(next),
            None => panic!("log sequence exhausted"),
        }
    }
}

/// The number of committed state changes; a precondition names one exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Revision(pub u64);

impl Revision {
    #[must_use]
    pub const fn next(self) -> Self {
        match self.0.checked_add(1) {
            Some(next) => Self(next),
            None => panic!("revision exhausted"),
        }
    }
}

/// Milliseconds since the Unix epoch, always as reported by the object store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Millis(pub u64);
