#[cfg(test)]
mod tests {
    use crate::families;
    use crate::schema::Ledger;
    use bumbledb::Db;

    #[test]
    fn aggregate_family_fold_regimes_are_pinned() {
        let dir = std::env::temp_dir().join("bumbledb-tripwires-elide");
        let _ = std::fs::remove_dir_all(&dir);
        let db = Db::create(&dir, Ledger, crate::harness::bench_work())
            .expect("create")
            .expect("accepted");
        let regime = |name: &str| {
            let family = families::all()
                .iter()
                .find(|f| f.name == name)
                .expect("registered");
            let prepared = db
                .prepare(&(family.query)(), crate::harness::bench_work())
                .expect("prepares");
            prepared.distinct_bindings()
        };
        assert!(regime("balance"), "balance elides the seen set");
        assert!(!regime("stats"), "stats' dedup is semantics");
        assert!(
            regime("latest_posting_per_account"),
            "the Arg family binds the posting's declared id key"
        );
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
