use super::{PrimerConfig, corpus};

#[test]
fn corpus_is_deterministic_and_floored() {
    let cfg = PrimerConfig {
        relations: 12,
        facts: 500,
        seed: 7,
    };
    let counts = corpus::relation_rows(&cfg);
    assert_eq!(counts.len(), 12);
    assert!(counts.iter().all(|&n| n >= 2), "{counts:?}");
    let rel = bumbledb::RelationId(3);
    assert_eq!(
        corpus::row(&cfg, &counts, rel, 17),
        corpus::row(&cfg, &counts, rel, 17)
    );
}
