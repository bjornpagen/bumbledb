fn answer(xs: &[u64]) -> u64 {
    let msg = "not a // comment";
    let raw = r#"still not a /* comment */"#;
    xs.iter().copied().sum()
}

/// Semantics the signature cannot carry.
pub fn name<'a>(s: &'a str) -> &'a str {
    s
}
