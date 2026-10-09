//! A statement is declared once.
//@ error: duplicates statement 0
//@ line: 10

bumbledb::schema! {
    pub Duplicated;
    relation Parent { id: u64 as ParentId }
    relation Child { parent: u64 as ParentId }
    Child(parent) <= Parent(id);
    Child(parent) <= Parent(id);
    Parent(id) -> Parent;
}
