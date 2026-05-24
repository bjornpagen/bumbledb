use crate::runner::{BenchError, BenchResult};

pub(crate) fn validate_select_distinct(sql: &str) -> BenchResult<()> {
    let normalized = sql.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = normalized.to_ascii_lowercase();
    if !lower.starts_with("select distinct ") {
        return Err(BenchError::new(
            "benchmark oracle SQL must start with SELECT DISTINCT",
        ));
    }
    for forbidden in [
        "count(",
        "group by",
        " left join ",
        " right join ",
        " full join ",
        " outer join ",
        " anti join ",
        " is null",
        " is not null",
        " union all ",
    ] {
        if lower.contains(forbidden) {
            return Err(BenchError::new(format!(
                "benchmark oracle SQL uses forbidden feature {forbidden}"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn parse_nullable_u64(raw: &str) -> BenchResult<u64> {
    if raw.eq_ignore_ascii_case("null") || raw.is_empty() {
        return Err(BenchError::new("open-data null values must be rejected"));
    }
    raw.parse()
        .map_err(|_| BenchError::new(format!("invalid u64 value {raw}")))
}

#[cfg(test)]
pub(crate) fn parse_scaled_i64(raw: &str, scale: i64) -> BenchResult<i64> {
    let Some((whole, frac)) = raw.split_once('.') else {
        return raw
            .parse::<i64>()
            .map(|value| value * scale)
            .map_err(|_| BenchError::new(format!("invalid fixed-point value {raw}")));
    };
    if frac.len() != 2 || scale != 100 {
        return Err(BenchError::new(
            "fixed-point decimals must be exactly representable at scale",
        ));
    }
    let whole = whole
        .parse::<i64>()
        .map_err(|_| BenchError::new(format!("invalid fixed-point value {raw}")))?;
    let frac = frac
        .parse::<i64>()
        .map_err(|_| BenchError::new(format!("invalid fixed-point value {raw}")))?;
    Ok(whole * scale + frac)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_lint_requires_select_distinct() {
        assert!(validate_select_distinct("SELECT DISTINCT x FROM r").is_ok());
        assert!(validate_select_distinct("SELECT x FROM r").is_err());
        assert!(validate_select_distinct("SELECT DISTINCT COUNT(*) FROM r").is_err());
        assert!(validate_select_distinct("SELECT DISTINCT x FROM r GROUP BY x").is_err());
        assert!(validate_select_distinct("SELECT DISTINCT x FROM r LEFT JOIN s").is_err());
        assert!(validate_select_distinct("SELECT DISTINCT x FROM r WHERE x IS NULL").is_err());
    }

    #[test]
    fn open_data_parsers_reject_null_and_preserve_decimal_exactness() -> BenchResult<()> {
        assert_eq!(parse_nullable_u64("42")?, 42);
        assert!(parse_nullable_u64("NULL").is_err());
        assert_eq!(parse_scaled_i64("12.34", 100)?, 1234);
        assert!(parse_scaled_i64("12.345", 100).is_err());
        Ok(())
    }
}
