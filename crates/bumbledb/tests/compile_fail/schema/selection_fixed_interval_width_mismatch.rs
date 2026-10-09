//! An `interval<E, w>` literal has width w.
//@ error: the literal does not fit `Item.lease`'s declared type
//@ line: 10

bumbledb::schema! {
    pub Review;

    relation Item { id: u64 as ItemId, lease: interval<u64, 7> as Lease }

    Item(id | lease == 1..3) <= Item(id);
}
