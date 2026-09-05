//! HASH-03: reproducible fingerprint-width sizing math.
//!
//! Chapter 41's tables are not folklore; they are these functions. The tests
//! reproduce every published cell so a future width change has to re-derive
//! its numbers instead of copying them. Three distinct probability models are
//! kept apart on purpose:
//!
//! 1. **Birthday collision** (any pair among `n` distinct inputs collides):
//!    `lambda = n(n-1)/2^(b+1)`, `p ~= 1 - exp(-lambda)`.
//! 2. **Random corruption miss** (one already-chosen message against its
//!    expected checksum): `2^-b` per independent corruption. Hash chains and
//!    content addressing compare across objects, so they cannot universally
//!    claim this easier model.
//! 3. **Deliberate collision search** against a `b`-bit cryptographic hash:
//!    roughly `b/2` bits of generic resistance. A 16-byte commitment offers
//!    only ~64-bit generic collision resistance — which is exactly why local
//!    fingerprints are exact-checked and authoritative digests stay 32 bytes.
//!
//! No finite hash is collision-free over unbounded input; exact canonical-byte
//! comparison is what keeps a collision from changing the database's meaning.

/// `UUIDv4` stores 128 bits but has 122 random bits after version/variant
/// fields. Never feed the 128-bit column of the sizing table to a `UUIDv4`
/// generator population.
pub const UUID_V4_RANDOM_BITS: u32 = 122;

/// Full-random 16-byte application IDs (the recommended helper) do carry the
/// whole 128 bits.
pub const ID128_RANDOM_BITS: u32 = 128;

#[expect(
    clippy::cast_precision_loss,
    reason = "sizing math is approximate by construction; ulp loss on n is irrelevant next to the model error"
)]
fn pair_count(n: u128) -> f64 {
    // n(n-1) as f64; exact for every population the table uses (<= 1e12,
    // whose square is far below f64's 1e308 range — precision loss is fine).
    (n as f64) * ((n.saturating_sub(1)) as f64)
}

/// `lambda = n(n-1) / 2^(b+1)` — the expected number of colliding pairs among
/// `n` distinct inputs under an ideal uniform `b`-bit hash.
///
/// # Panics
/// On a hash width that does not fit `i32` (never for real widths).
#[must_use]
pub fn birthday_lambda(n: u128, bits: u32) -> f64 {
    pair_count(n) / 2f64.powi(i32::try_from(bits).expect("hash widths are small") + 1)
}

/// `p ~= 1 - exp(-lambda)`, computed as `-expm1(-lambda)` so tiny lambdas do
/// not underflow to zero.
#[must_use]
pub fn birthday_probability(n: u128, bits: u32) -> f64 {
    -f64::exp_m1(-birthday_lambda(n, bits))
}

/// Minimum whole bits for accidental-collision probability below `epsilon`:
/// `b >= ceil(log2(n(n-1) / (2 epsilon)))`. Returns `None` for degenerate
/// populations (`n < 2`) where no pair exists.
#[must_use]
pub fn required_bits(n: u128, epsilon: f64) -> Option<u32> {
    if n < 2 || epsilon.is_nan() || epsilon <= 0.0 {
        return None;
    }
    let ratio = pair_count(n) / (2.0 * epsilon);
    let bits = ratio.log2().ceil();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "ceil of a log2 of a finite positive ratio; the domain keeps it in u32 range"
    )]
    Some(bits.max(0.0) as u32)
}

/// Whole bytes for [`required_bits`].
#[must_use]
pub fn required_bytes(n: u128, epsilon: f64) -> Option<u32> {
    required_bits(n, epsilon).map(|bits| bits.div_ceil(8))
}

/// Fleet risk sums per-namespace probabilities. `domains` independent lookup
/// domains with per-domain collision probability `p` give approximately
/// `domains * p` (for small `p`), **not** the probability of one shared
/// domain holding the union of all rows. Saturates at 1.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "sizing math; domain counts far below 2^53 in every intended use"
)]
pub fn fleet_probability(domains: u64, per_domain: f64) -> f64 {
    (domains as f64 * per_domain).min(1.0)
}

/// Random corruption of one chosen message against its expected checksum:
/// ideal miss probability `2^-b` per independent corruption. Distinct from
/// any-pair birthday collisions; do not interchange them.
///
/// # Panics
/// On a hash width that does not fit `i32` (never for real widths).
#[must_use]
pub fn corruption_miss_probability(bits: u32) -> f64 {
    2f64.powi(-i32::try_from(bits).expect("hash widths are small"))
}

/// Generic collision-search resistance of an ideal `b`-bit cryptographic
/// hash, in bits (~`b/2`). Random accidental probabilities must never be
/// substituted for this adversarial figure.
#[must_use]
pub const fn generic_collision_resistance_bits(bits: u32) -> u32 {
    bits / 2
}
