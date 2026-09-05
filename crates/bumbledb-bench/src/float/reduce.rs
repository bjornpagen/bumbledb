//! Deliberately slow independent oracle: base-two digits and binary search
//! between adjacent IEEE encodings. No production limb accumulator, rounding
//! helper, floating arithmetic, or ordered-key codec participates.
use bumbledb::F64;
use std::cmp::Ordering;

// 2098 input bits + 64 count bits + midpoint carry, with generous headroom.
const DIGITS: usize = 2200;
type Digits = Vec<u128>;

fn normalize(value: &mut [u128]) {
    for index in 0..value.len() - 1 {
        value[index + 1] += value[index] / 2;
        value[index] %= 2;
    }
    assert!(value[value.len() - 1] < 2, "oracle digit bound");
}

fn scaled(bits: u64, count: u64) -> Digits {
    let exponent = (bits >> 52) as usize;
    let fraction = bits & ((1 << 52) - 1);
    let mut value = vec![0; DIGITS];
    if exponent == 0x7ff {
        // The finite continuation above MAX used only for overflow rounding.
        value[2098] = u128::from(count);
    } else {
        let (mantissa, shift) = if exponent == 0 {
            (fraction, 0)
        } else {
            (fraction | (1 << 52), exponent - 1)
        };
        value[shift] = u128::from(mantissa) * u128::from(count);
    }
    normalize(&mut value);
    value
}

fn compare(left: &[u128], right: &[u128]) -> Ordering {
    left.iter().rev().cmp(right.iter().rev())
}

fn add(left: &mut [u128], right: &[u128]) {
    for (left, right) in left.iter_mut().zip(right) {
        *left += right;
    }
    normalize(left);
}

fn subtract(larger: &mut [u128], smaller: &[u128]) {
    let mut borrow = 0;
    for (left, right) in larger.iter_mut().zip(smaller) {
        let amount = right + borrow;
        if *left < amount {
            *left = *left + 2 - amount;
            borrow = 1;
        } else {
            *left -= amount;
            borrow = 0;
        }
    }
    assert_eq!(borrow, 0);
}

pub(crate) fn reduce(values: impl Iterator<Item = F64>, mean: bool) -> F64 {
    let mut positive = vec![0; DIGITS];
    let mut negative = vec![0; DIGITS];
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
    let doubled = total.clone();
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
    fn independent_digit_search_agrees_with_python_fraction_fixtures() {
        let mut checked = 0;
        for line in include_str!("../../../bumbledb/tests/fixtures/f64_reference.txt").lines() {
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
