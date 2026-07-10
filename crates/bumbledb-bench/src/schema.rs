//! The benchmark's ledger schema — a statement-for-statement
//! transcription of the primary-benchmark block in
//! `docs/architecture/60-validation.md`: nine relations, the nine
//! containments, and the pointwise key
//! `Mandate(account, active) -> Mandate` (one mandate per account per
//! instant). The doc's notation leaves enum variant lists open; this
//! transcription pins the ones the query generator's target module
//! already uses (`Currency`, `Source`, `Tag` — three variants each).
//!
//! Changing this schema is a deliberate act: it re-baselines every stored
//! corpus digest and every published report (the golden fingerprint test
//! is the tripwire).

bumbledb::schema! {
    pub Ledger;

    relation Holder {
        id: u64 as HolderId, fresh,
        name: str,
    }
    relation Account {
        id: u64 as AccountId, fresh,
        holder: u64 as HolderId,
        currency: enum Currency { Usd, Eur, Gbp },
    }
    relation Instrument {
        id: u64 as InstrumentId, fresh,
        symbol: str,
    }
    relation JournalEntry {
        id: u64 as JournalEntryId, fresh,
        source: enum Source { Manual, Import, System },
        created_at: i64,
    }
    relation Posting {
        id: u64 as PostingId, fresh,
        entry: u64 as JournalEntryId,
        account: u64 as AccountId,
        instrument: u64 as InstrumentId,
        amount: i64,
        at: i64,
    }
    relation PostingTag {
        posting: u64 as PostingId,
        tag: enum Tag { Fee, Rebate, Adjustment },
    }
    relation Org {
        id: u64 as OrgId, fresh,
        name: str,
    }
    relation OrgParent {
        child: u64 as OrgId,
        parent: u64 as OrgId,
    }
    relation Mandate {
        account: u64 as AccountId,
        org: u64 as OrgId,
        active: interval<i64>,
    }

    Account(holder)      <= Holder(id);
    Posting(entry)       <= JournalEntry(id);
    Posting(account)     <= Account(id);
    Posting(instrument)  <= Instrument(id);
    PostingTag(posting)  <= Posting(id);
    OrgParent(child)     <= Org(id);
    OrgParent(parent)    <= Org(id);
    Mandate(account)     <= Account(id);
    Mandate(org)         <= Org(id);
    Mandate(account, active) -> Mandate;
}

/// The validated ledger schema, memoized for the inspection surfaces
/// (DDL rendering, id lookups, query translation); the engine itself
/// takes [`Ledger`] — `Db::create(dir, Ledger)` — and validates there.
///
/// # Panics
///
/// Never in practice: the ledger declaration passes the acceptance gate
/// (asserted on first use).
pub fn schema() -> &'static bumbledb::Schema {
    use bumbledb::Theory as _;
    static SCHEMA: std::sync::OnceLock<bumbledb::Schema> = std::sync::OnceLock::new();
    SCHEMA.get_or_init(|| {
        Ledger
            .descriptor()
            .validate()
            .expect("the ledger schema is valid")
    })
}

/// Relation and field ids by name — no magic numbers in family
/// definitions or the generator (declaration order is the id order).
pub mod ids {
    use bumbledb::{FieldId, RelationId};

    pub const HOLDER: RelationId = RelationId(0);
    pub const ACCOUNT: RelationId = RelationId(1);
    pub const INSTRUMENT: RelationId = RelationId(2);
    pub const JOURNAL_ENTRY: RelationId = RelationId(3);
    pub const POSTING: RelationId = RelationId(4);
    pub const POSTING_TAG: RelationId = RelationId(5);
    pub const ORG: RelationId = RelationId(6);
    pub const ORG_PARENT: RelationId = RelationId(7);
    pub const MANDATE: RelationId = RelationId(8);

    /// The number of relations — loaders iterate `0..RELATIONS`.
    pub const RELATIONS: u32 = 9;

    pub mod holder {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const NAME: FieldId = FieldId(1);
    }
    pub mod account {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const HOLDER: FieldId = FieldId(1);
        pub const CURRENCY: FieldId = FieldId(2);
    }
    pub mod instrument {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const SYMBOL: FieldId = FieldId(1);
    }
    pub mod journal_entry {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const SOURCE: FieldId = FieldId(1);
        pub const CREATED_AT: FieldId = FieldId(2);
    }
    pub mod posting {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const ENTRY: FieldId = FieldId(1);
        pub const ACCOUNT: FieldId = FieldId(2);
        pub const INSTRUMENT: FieldId = FieldId(3);
        pub const AMOUNT: FieldId = FieldId(4);
        pub const AT: FieldId = FieldId(5);
    }
    pub mod posting_tag {
        use super::FieldId;
        pub const POSTING: FieldId = FieldId(0);
        pub const TAG: FieldId = FieldId(1);
    }
    pub mod org {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const NAME: FieldId = FieldId(1);
    }
    pub mod org_parent {
        use super::FieldId;
        pub const CHILD: FieldId = FieldId(0);
        pub const PARENT: FieldId = FieldId(1);
    }
    pub mod mandate {
        use super::FieldId;
        pub const ACCOUNT: FieldId = FieldId(0);
        pub const ORG: FieldId = FieldId(1);
        pub const ACTIVE: FieldId = FieldId(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumbledb::schema::{Resolved, StatementDescriptor, ValueType};

    /// The golden fingerprint: changing the schema re-baselines every
    /// corpus digest and report — this test makes that a deliberate act,
    /// never an accident. Update the constant ONLY alongside a conscious
    /// schema change.
    #[test]
    fn the_fingerprint_is_pinned() {
        let fp = bumbledb::schema::fingerprint::fingerprint(schema());
        let hex = fp.0.iter().fold(String::new(), |mut acc, b| {
            use std::fmt::Write as _;
            let _ = write!(acc, "{b:02x}");
            acc
        });
        assert_eq!(
            hex, "2de5c610582eaa84fab6530ba997698bb6981197979e589d7f2a60467781fa81",
            "the ledger schema changed — re-baseline corpora and reports deliberately"
        );
    }

    /// The doc's statement roster, verbatim: six fresh auto-keys first
    /// (declaration order), then the eight containments in source order,
    /// then the pointwise key.
    #[test]
    fn the_statement_roster_matches_the_doc() {
        let statements = schema().statements();
        assert_eq!(statements.len(), 6 + 9 + 1);
        let mut autos = 0;
        let mut containments = Vec::new();
        let mut pointwise = 0;
        for statement in statements {
            match &statement.descriptor {
                StatementDescriptor::Functionality { relation, .. } => match statement.resolved {
                    Resolved::Functionality {
                        interval_position: Some(_),
                    } => {
                        pointwise += 1;
                        assert_eq!(*relation, ids::MANDATE);
                    }
                    _ => autos += 1,
                },
                StatementDescriptor::Containment { source, target } => {
                    containments.push((source.relation, target.relation));
                }
            }
        }
        assert_eq!(
            autos, 6,
            "Holder/Account/Instrument/JournalEntry/Posting/Org fresh ids"
        );
        assert_eq!(pointwise, 1, "the pointwise Mandate key");
        assert_eq!(
            containments,
            vec![
                (ids::ACCOUNT, ids::HOLDER),
                (ids::POSTING, ids::JOURNAL_ENTRY),
                (ids::POSTING, ids::ACCOUNT),
                (ids::POSTING, ids::INSTRUMENT),
                (ids::POSTING_TAG, ids::POSTING),
                (ids::ORG_PARENT, ids::ORG),
                (ids::ORG_PARENT, ids::ORG),
                (ids::MANDATE, ids::ACCOUNT),
                (ids::MANDATE, ids::ORG),
            ],
            "the doc block's nine containment statements, in source order"
        );
    }

    #[test]
    fn the_id_registry_matches_declaration_order() {
        let s = schema();
        for (idx, name) in [
            "Holder",
            "Account",
            "Instrument",
            "JournalEntry",
            "Posting",
            "PostingTag",
            "Org",
            "OrgParent",
            "Mandate",
        ]
        .iter()
        .enumerate()
        {
            let rel = bumbledb::RelationId(u32::try_from(idx).expect("small"));
            assert_eq!(s.relation(rel).name(), *name);
        }
        assert_eq!(
            u32::try_from(s.relations().len()).expect("small"),
            ids::RELATIONS
        );
        assert_eq!(
            s.relation(ids::POSTING).field(ids::posting::AT).name,
            "at".into()
        );
        assert!(matches!(
            s.relation(ids::MANDATE)
                .field(ids::mandate::ACTIVE)
                .value_type,
            ValueType::Interval { .. }
        ));
    }
}
