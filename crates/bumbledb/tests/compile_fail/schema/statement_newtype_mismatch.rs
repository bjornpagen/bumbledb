//! Paired faces agree on their newtype.
//@ error: pairs `Attempt.kind` (`SheetId`) with `Kind.id` (`KindId`)
//@ line: 12

bumbledb::schema! {
    pub Grading;

    closed relation Kind as KindId = { DirectPass, Failed };

    relation Attempt { kind: u64 as SheetId }

    Attempt(kind) <= Kind(id);
}
