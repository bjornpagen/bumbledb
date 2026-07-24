//! Separators are mandatory: `body := item (',' item)*` — a dropped
//! comma between body items is a parse error, never a silently accepted
//! respelling (the renderer emits commas, and the round-trip law holds
//! only if the pinned notation is the one writable spelling).
//@ error: expected `,` or `;`
//@ line: 20

bumbledb::schema! {
    pub Org;

    relation Parent {
        child: u64,
        parent: u64,
    }
}

pub fn q() -> bumbledb::Query {
    bumbledb_query::query!(Org {
        (c, p) | Parent(child: c, parent: p)
                 c < p;
    })
}
