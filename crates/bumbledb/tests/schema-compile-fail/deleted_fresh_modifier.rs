//! Identity generation belongs to the host; the old modifier is not an alias.
//@ error: the `fresh` modifier is deleted

bumbledb::schema! {
    pub Minted;

    relation Record { id: uuid as RecordId, fresh, name: str }
}
