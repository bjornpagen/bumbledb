//! Independent scalar comparison and lossless SQLite relational mirror.
//!
//! SQLite REAL turns NaN into NULL. All F64 values therefore use eight-byte
//! ordered BLOBs in the mirror, not a mixture of SQL storage classes. This
//! preserves set equality, order, keys and min/max; ordinary SQL sum/avg are
//! not a numerical oracle for this representation or for exact reductions.

use std::cmp::Ordering;

use bumbledb::F64;

mod reduce;
pub(crate) use reduce::reduce;

#[cfg(test)]
mod relational_tests;

pub(crate) fn compare(left: F64, right: F64) -> Ordering {
    let (left, right) = (left.to_f64(), right.to_f64());
    match (left.is_nan(), right.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => left.partial_cmp(&right).expect("neither operand is NaN"),
    }
}

pub(crate) fn sql_bytes(value: F64) -> [u8; 8] {
    let bits = value.to_bits();
    let ordered = if bits >> 63 == 1 {
        u64::MAX - bits
    } else {
        bits + (1 << 63)
    };
    ordered.to_be_bytes()
}

pub(crate) fn from_sql_bytes(bytes: &[u8]) -> Result<F64, String> {
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| "f64 SQL key must have eight bytes")?;
    let key = u64::from_be_bytes(bytes);
    let bits = if key < (1 << 63) {
        u64::MAX - key
    } else {
        key - (1 << 63)
    };
    let value = f64::from_bits(bits);
    if (value.is_nan() && bits != 0x7ff8_0000_0000_0000) || bits == 0x8000_0000_0000_0000 {
        return Err("noncanonical f64 SQL key".into());
    }
    Ok(F64::from_bits(bits))
}

pub(crate) fn sql_literal(value: F64) -> String {
    format!("X'{:016X}'", u64::from_be_bytes(sql_bytes(value)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relational_sql_key_order_agrees_with_numeric_oracle() {
        let mut values = vec![
            F64::NEG_INFINITY,
            F64::from(-f64::MAX),
            F64::from(-1.0),
            F64::from_bits(0x8000_0000_0000_0001),
            F64::ZERO,
            F64::from_bits(1),
            F64::from(1.0),
            F64::from(f64::MAX),
            F64::INFINITY,
            F64::NAN,
        ];
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        for _ in 0..4096 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            values.push(F64::from_bits(state));
        }
        values.sort_by(|a, b| compare(*a, *b));
        for pair in values.windows(2) {
            assert_eq!(
                compare(pair[0], pair[1]),
                sql_bytes(pair[0]).cmp(&sql_bytes(pair[1]))
            );
        }
        for value in values {
            assert_eq!(
                from_sql_bytes(&sql_bytes(value)).unwrap().to_bits(),
                value.to_bits()
            );
        }
    }

    #[test]
    fn relational_sql_key_rejects_noncanonical_holes_and_widths() {
        for bytes in [&[][..], &[0; 7][..], &[0; 9][..]] {
            assert!(from_sql_bytes(bytes).is_err());
        }
        for bits in [
            0x8000_0000_0000_0000_u64,
            0x7ff0_0000_0000_0001,
            0xfff8_0000_0000_0000,
        ] {
            let key = if bits >> 63 == 1 {
                u64::MAX - bits
            } else {
                bits + (1 << 63)
            };
            assert!(from_sql_bytes(&key.to_be_bytes()).is_err());
        }
        assert_eq!(sql_literal(F64::ZERO), "X'8000000000000000'");
        assert_eq!(sql_literal(F64::NAN), "X'FFF8000000000000'");
    }
}
