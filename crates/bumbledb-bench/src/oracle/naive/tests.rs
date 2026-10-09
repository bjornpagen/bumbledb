mod closed;
mod dnf;
mod judgment;
mod query;
mod reach;

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
