//! The model's own goldens: judgment cases re-expressing the engine's
//! commit fixtures, and the query-semantics landmarks. The differential
//! streams live with the runner (`crate::differential`).

mod closed;
mod dnf;
mod judgment;
mod query;

/// The independence law, grep-enforced: the model shares the engine's
/// *types*, never its algorithms or compiled representations — no
/// `schema::Resolved` import (the enforcement-plan data is the engine's
/// alone) and no bitsets (the closed judgment is σ over the extension
/// rows by value comparison, never a 256-bit member set).
#[test]
fn the_model_imports_no_compiled_representation() {
    for (name, source) in [
        ("naive.rs", include_str!("../naive.rs")),
        ("naive/query.rs", include_str!("query.rs")),
        ("naive/tuple.rs", include_str!("tuple.rs")),
    ] {
        for banned in ["Resolved", "MemberSet", "[u64; 4]", "1 <<", "bitset"] {
            assert!(
                !source.contains(banned),
                "{name} mentions {banned:?} — the model must not share the \
                 engine's compiled representation"
            );
        }
    }
}
