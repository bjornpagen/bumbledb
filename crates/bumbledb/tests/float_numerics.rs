//! Checked-in expectations come from exact Python Fraction arithmetic and a
//! binary-search rounding oracle, never the production limb/guard routines.
use bumbledb::{F64, F64CastError, F64Math};

fn bits(text: &str) -> u64 {
    u64::from_str_radix(text, 16).expect("fixture bits")
}

#[test]
fn independent_rational_goldens_cover_arithmetic_reductions_and_casts() {
    let mut counts = [0; 7];
    for (line, fixture) in include_str!("fixtures/f64_reference.txt")
        .lines()
        .enumerate()
    {
        if fixture.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = fixture.split_whitespace().collect();
        let at = format!("fixture line {}: {fixture}", line + 1);
        match fields[0] {
            "add" | "sub" | "mul" | "div" => {
                let left = F64::from_canonical_bits(bits(fields[1])).unwrap();
                let right = F64::from_canonical_bits(bits(fields[2])).unwrap();
                let (index, actual) = match fields[0] {
                    "add" => (0, F64Math::add(left, right)),
                    "sub" => (1, F64Math::subtract(left, right)),
                    "mul" => (2, F64Math::multiply(left, right)),
                    "div" => (3, F64Math::divide(left, right)),
                    _ => unreachable!(),
                };
                counts[index] += 1;
                assert_eq!(actual.unwrap().to_bits(), bits(fields[3]), "{at}");
            }
            "reduce" => {
                counts[4] += 1;
                let values: Vec<_> = fields[3..]
                    .iter()
                    .map(|text| F64::from_canonical_bits(bits(text)).unwrap())
                    .collect();
                let (sum, mean) = F64Math::sum_and_mean(values.iter().copied())
                    .unwrap()
                    .unwrap();
                assert_eq!(sum.to_bits(), bits(fields[1]), "sum {at}");
                assert_eq!(mean.to_bits(), bits(fields[2]), "mean {at}");
                assert_eq!(
                    F64Math::sum(values.iter().rev().copied()).unwrap(),
                    Some(sum),
                    "{at}"
                );
                assert_eq!(
                    F64Math::mean(values.iter().rev().copied()).unwrap(),
                    Some(mean),
                    "{at}"
                );
            }
            "u64" => {
                counts[5] += 1;
                let value = fields[1].parse::<u64>().unwrap();
                assert_eq!(F64::from_u64(value).to_bits(), bits(fields[2]), "{at}");
                let exact = F64::from_u64_exact(value);
                if fields[3] == "1" {
                    assert_eq!(exact.unwrap().to_u64_exact(), Ok(value), "{at}");
                } else {
                    assert_eq!(exact, Err(F64CastError::Inexact), "{at}");
                }
            }
            "i64" => {
                counts[6] += 1;
                let value = fields[1].parse::<i64>().unwrap();
                assert_eq!(F64::from_i64(value).to_bits(), bits(fields[2]), "{at}");
                let exact = F64::from_i64_exact(value);
                if fields[3] == "1" {
                    assert_eq!(exact.unwrap().to_i64_exact(), Ok(value), "{at}");
                } else {
                    assert_eq!(exact, Err(F64CastError::Inexact), "{at}");
                }
            }
            other => panic!("unknown oracle case {other}"),
        }
    }
    assert_eq!(counts, [1536, 1536, 1536, 1536, 317, 265, 263]);
}

#[test]
fn casts_refuse_nonfinite_fractions_sign_and_exact_domain_boundaries() {
    for value in [F64::NAN, F64::INFINITY, F64::NEG_INFINITY] {
        assert_eq!(value.to_i64_exact(), Err(F64CastError::NonFinite));
        assert_eq!(value.to_u64_exact(), Err(F64CastError::NonFinite));
    }
    for bits in [1, 0x3fe0_0000_0000_0000, 0xbff8_0000_0000_0000] {
        let value = F64::from_bits(bits);
        assert_eq!(value.to_i64_exact(), Err(F64CastError::Fractional));
        assert_eq!(value.to_u64_exact(), Err(F64CastError::Fractional));
    }
    assert_eq!(
        F64::from_bits(0x43e0_0000_0000_0000).to_i64_exact(),
        Err(F64CastError::OutOfRange)
    );
    assert_eq!(
        F64::from_bits(0xc3e0_0000_0000_0000).to_i64_exact(),
        Ok(i64::MIN)
    );
    assert_eq!(
        F64::from_bits(0x43f0_0000_0000_0000).to_u64_exact(),
        Err(F64CastError::OutOfRange)
    );
    assert_eq!(
        F64::from_bits(0x43ef_ffff_ffff_ffff).to_u64_exact(),
        Ok(u64::MAX - 2047)
    );
    assert_eq!(
        F64::from_i64(-1).to_u64_exact(),
        Err(F64CastError::OutOfRange)
    );
    assert_eq!(F64::ZERO.to_i64_exact(), Ok(0));
    assert_eq!(F64::ZERO.to_u64_exact(), Ok(0));
}

#[test]
fn empty_reductions_create_no_group_and_equal_values_preserve_binding_count() {
    assert_eq!(F64Math::sum([]).unwrap(), None);
    assert_eq!(F64Math::mean([]).unwrap(), None);
    assert_eq!(F64Math::sum_and_mean([]).unwrap(), None);
    let one = F64::from_bits(0x3ff0_0000_0000_0000);
    assert_eq!(
        F64Math::sum([one, one]).unwrap(),
        Some(F64::from_bits(0x4000_0000_0000_0000))
    );
    assert_eq!(F64Math::mean([one, one]).unwrap(), Some(one));
}
