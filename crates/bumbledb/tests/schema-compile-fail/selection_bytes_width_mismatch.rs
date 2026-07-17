//! The width is the type: a `bytes<N>` selection literal of any other
//! width must die at the token→`Value` seam (an expansion error), never
//! degrade to a `Db::create` error.
//@ error: does not fit the field's declared type

bumbledb::schema! {
    pub Review;

    relation Item { id: u64 as ItemId, fresh, mark: bytes<4> }

    Item(id | mark == b"toolong!") <= Item(id);
}
