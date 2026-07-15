//! `{..hi}` is an open shorthand — never admitted; bounds are always
//! explicit: a ceiling is written `{0..hi}`.
//@ error: `{..hi}` never parses
//@ error: a ceiling is written `{0..hi}`

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64 }

    Parent(id) <={..5} Task(parent);
}
