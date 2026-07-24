//! Every head variable must be bound in the rule body — projection has
//! nothing to project otherwise. The refusal is spanned at the head
//! occurrence of the unbound name.
//@ error: head variable `q` is not bound in the rule body
//@ line: 18

bumbledb::schema! {
    pub Org;

    relation Parent {
        child: u64,
        parent: u64,
    }
}

pub fn q() -> bumbledb::Query {
    bumbledb_query::query!(Org {
        (c, q) |
            Parent(child: c, parent: p);
    })
}
