//! `{1..*}` says only what the bare containment says — the ban table's
//! first line (the canonical-utterance law,
//! `docs/architecture/70-api.md`): drop the annotation, write `X <= Y`.
//@ error: `{1..*}` says only what the bare containment says
//@ error: Parent(…) <= Task(…)

bumbledb::schema! {
    pub Ledger;

    relation Parent { id: u64 }
    relation Task   { parent: u64 }

    Parent(id) <={1..*} Task(parent);
}
