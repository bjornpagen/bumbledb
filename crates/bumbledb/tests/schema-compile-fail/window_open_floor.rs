//! `{lo..}` is an open shorthand — never admitted; bounds are always
//! explicit: a floor is written `{lo..*}`.
//@ error: `{lo..}` never parses
//@ error: a floor is written `{lo..*}`

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64 }

    Parent(id) <={2..} Task(parent);
}
