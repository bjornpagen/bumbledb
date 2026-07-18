//! The coherence check (`docs/architecture/30-dependencies.md` § the
//! taxonomy is checked): the newtypes on a statement's paired faces must
//! agree, positionwise — two labels disagreeing is a spanned teaching
//! error at both faces, raised by the ONE shared lowering (a closed
//! relation's synthetic `id` carries the handle newtype).
//@ error: the containment pairs `Attempt.kind` (`SheetId`) with `Kind.id` (`KindId`)
//@ error: the faces of a dependency agree on their newtype, or neither carries one

bumbledb::schema! {
    pub Grading;

    closed relation Kind as KindId = { DirectPass, Failed };

    relation Attempt { kind: u64 as SheetId }

    Attempt(kind) <= Kind(id);
}
