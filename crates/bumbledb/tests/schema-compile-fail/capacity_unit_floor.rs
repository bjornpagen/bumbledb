//@ error: `{N..*}` on the unit instance

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64 }

    Parent(id) <={2..*} Task(parent);
}
