//! A field is `name: type`; there are no field modifiers.
//@ error: unknown field modifier `autoincrement`
//@ line: 8

bumbledb::schema! {
    pub Minted;

    relation Record { id: uuid as RecordId, autoincrement, name: str }
}
