//! The inline `enum` field type is deleted vocabulary — a vocabulary is
//! a closed relation — and the word diagnoses its own replacement at
//! expansion.
//@ error: the enum type is deleted

bumbledb::schema! {
    pub Ledger;

    relation Account { kind: enum Kind { Checking, Savings } }
}
