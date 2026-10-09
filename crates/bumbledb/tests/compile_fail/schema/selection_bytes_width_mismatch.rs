//! A `bytes<N>` literal has width N.
//@ error: the literal does not fit `Item.mark`'s declared type
//@ line: 10

bumbledb::schema! {
    pub Review;

    relation Item { id: u64 as ItemId, mark: bytes<4> }

    Item(id | mark == b"toolong!") <= Item(id);
}
