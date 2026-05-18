//! Reusable query workloads.

/// Representative supported positive Datalog queries.
pub fn ledger_queries() -> Vec<&'static str> {
    vec![
        "find ?account where Account(id: ?account, holder: $holder)",
        r#"
        find ?account ?holder_name
        where
          Account(id: ?account, holder: ?holder)
          Holder(id: ?holder, name: ?holder_name)
        "#,
        r#"
        find ?posting ?account ?holder_name
        where
          Posting(id: ?posting, account: ?account, amount: ?amount, at: ?t)
          Account(id: ?account, holder: ?holder)
          Holder(id: ?holder, name: ?holder_name)
          ?t >= $start
          ?t < $end
        "#,
        r#"
        find ?account sum(?amount) count(?posting)
        where
          Posting(id: ?posting, account: ?account, amount: ?amount, at: ?t)
        "#,
    ]
}
