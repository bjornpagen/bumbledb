//! Independent exact float reduction: every finite input becomes an exact
//! integer in units of 2^-1074, accumulated in plain base-2^64 limbs, and the
//! result is found by binary search between adjacent IEEE encodings. No
//! production accumulator, rounding helper, floating arithmetic or ordered-key
//! codec participates.
use bumbledb::F64;
use std::cmp::Ordering;

/// 2098 value bits + 64 count bits + the midpoint carry, with headroom.
const LIMBS: usize = 36;
type Big = [u64; LIMBS];

/// The low 64 bits of a limb sum.
fn low_word(word: u128) -> u64 {
    u64::try_from(word & u128::from(u64::MAX)).expect("masked to 64 bits")
}

/// Adds `value << shift` into `acc`.
fn add_shifted(acc: &mut Big, value: u128, shift: usize) {
    let (limb, bit) = (shift / 64, shift % 64);
    let shifted = value << bit;
    let high = if bit == 0 { 0 } else { value >> (128 - bit) };
    let words = [low_word(shifted), low_word(shifted >> 64), low_word(high)];
    let mut carry = 0u128;
    let mut index = limb;
    for word in words {
        let sum = u128::from(acc[index]) + u128::from(word) + carry;
        acc[index] = low_word(sum);
        carry = sum >> 64;
        index += 1;
    }
    while carry != 0 {
        let sum = u128::from(acc[index]) + carry;
        acc[index] = low_word(sum);
        carry = sum >> 64;
        index += 1;
    }
    assert_eq!(acc[LIMBS - 1], 0, "oracle limb bound");
}

/// `count` copies of the encoding `bits` (sign ignored) in units of 2^-1074.
/// The infinity encoding stands for 2^1024, the finite continuation above
/// MAX used only for overflow rounding.
fn scaled(bits: u64, count: u64) -> Big {
    let exponent = (bits >> 52) as usize;
    let fraction = bits & ((1 << 52) - 1);
    let mut value = [0; LIMBS];
    if exponent == 0x7ff {
        add_shifted(&mut value, u128::from(count), 2098);
    } else {
        let (mantissa, shift) = if exponent == 0 {
            (fraction, 0)
        } else {
            (fraction | (1 << 52), exponent - 1)
        };
        add_shifted(&mut value, u128::from(mantissa) * u128::from(count), shift);
    }
    value
}

fn compare(left: &Big, right: &Big) -> Ordering {
    left.iter().rev().cmp(right.iter().rev())
}

fn add(left: &mut Big, right: &Big) {
    let mut carry = 0u128;
    for (left, right) in left.iter_mut().zip(right) {
        let sum = u128::from(*left) + u128::from(*right) + carry;
        *left = low_word(sum);
        carry = sum >> 64;
    }
    assert_eq!(carry, 0, "oracle limb bound");
}

fn subtract(larger: &mut Big, smaller: &Big) {
    let mut borrow = false;
    for (left, right) in larger.iter_mut().zip(smaller) {
        let (step, first) = left.overflowing_sub(*right);
        let (step, second) = step.overflowing_sub(u64::from(borrow));
        *left = step;
        borrow = first || second;
    }
    assert!(!borrow, "subtract the smaller magnitude");
}

pub(crate) fn reduce(values: impl Iterator<Item = F64>, mean: bool) -> F64 {
    let mut positive: Big = [0; LIMBS];
    let mut negative: Big = [0; LIMBS];
    let mut count = 0u64;
    let (mut nan, mut plus_inf, mut minus_inf) = (false, false, false);
    for value in values {
        count = count.checked_add(1).expect("oracle fixture cardinality");
        let bits = value.to_bits();
        let magnitude = bits & !(1 << 63);
        match magnitude.cmp(&0x7ff0_0000_0000_0000) {
            Ordering::Greater => nan = true,
            Ordering::Equal => {
                if bits >> 63 == 0 {
                    plus_inf = true;
                } else {
                    minus_inf = true;
                }
            }
            Ordering::Less => {
                let destination = if bits >> 63 == 0 {
                    &mut positive
                } else {
                    &mut negative
                };
                add(destination, &scaled(magnitude, 1));
            }
        }
    }
    assert_ne!(count, 0, "no aggregate output for an empty group");
    if nan || (plus_inf && minus_inf) {
        return F64::NAN;
    }
    if plus_inf {
        return F64::INFINITY;
    }
    if minus_inf {
        return F64::NEG_INFINITY;
    }
    let negative_result = compare(&positive, &negative) == Ordering::Less;
    let (mut total, smaller) = if negative_result {
        (negative, positive)
    } else {
        (positive, negative)
    };
    subtract(&mut total, &smaller);
    let divisor = if mean { count } else { 1 };
    let (mut low, mut high) = (0u64, 0x7ff0_0000_0000_0000u64);
    while low + 1 < high {
        let middle = low + (high - low) / 2;
        if compare(&scaled(middle, divisor), &total) == Ordering::Greater {
            high = middle;
        } else {
            low = middle;
        }
    }
    let mut midpoint = scaled(low, divisor);
    add(&mut midpoint, &scaled(high, divisor));
    let doubled = total;
    add(&mut total, &doubled);
    let rounded = match compare(&total, &midpoint) {
        Ordering::Less => low,
        Ordering::Equal if low & 1 == 0 => low,
        Ordering::Equal | Ordering::Greater => high,
    };
    F64::from_bits(rounded | (u64::from(negative_result) << 63))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_reduction_agrees_with_python_fraction_fixtures() {
        let mut checked = 0;
        for line in include_str!("../../../../bumbledb/tests/fixtures/f64_reference.txt").lines() {
            let words: Vec<_> = line.split_whitespace().collect();
            if words.first() != Some(&"reduce") {
                continue;
            }
            let bits = |word: &str| u64::from_str_radix(word, 16).unwrap();
            let values = || words[3..].iter().map(|word| F64::from_bits(bits(word)));
            assert_eq!(
                reduce(values(), false).to_bits(),
                bits(words[1]),
                "sum: {line}"
            );
            assert_eq!(
                reduce(values(), true).to_bits(),
                bits(words[2]),
                "mean: {line}"
            );
            checked += 1;
        }
        assert_eq!(checked, 317);
    }
}
