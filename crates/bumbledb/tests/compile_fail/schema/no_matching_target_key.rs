//! A containment's target projection is a declared key of the target.
//@ error: target relation Parent (0) projection {id (0)} matches no declared key
//@ line: 10

bumbledb::schema! {
    pub Ledger;
    relation Parent { id: u64 }
    relation Child  { parent: u64 }

    Child(parent) <= Parent(id);
}
