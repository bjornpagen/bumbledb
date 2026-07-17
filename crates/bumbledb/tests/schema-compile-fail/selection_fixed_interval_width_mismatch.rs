//! The width is the type: an `interval<E, w>` selection literal whose
//! spelled width is not `w` must die at the token→`Value` seam (an
//! expansion error), never degrade to a `Db::create` error.
//@ error: does not fit the field's declared type

bumbledb::schema! {
    pub Review;

    relation Item { id: u64 as ItemId, fresh, lease: interval<u64, 7> as Lease }

    Item(id | lease == 1..3) <= Item(id);
}
