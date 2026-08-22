fn answer(xs: &[u64]) -> u64 {
    // narration: sum the slice
    let msg = "not a // comment";
    let raw = r#"still not a /* comment */"#;
    xs.iter().copied().sum() /* trailing fold */
}

/// Public contract sentence that will be tightened.
pub fn name<'a>(s: &'a str) -> &'a str {
    s
}
