use bumbledb::schema::ValidateDescriptor as _;
bumbledb::schema! {
    pub Ledger;

    relation Holder {
        id: u64 as HolderId, fresh,
        name: str,
    }
    relation Account {
        id: u64 as AccountId, fresh,
        holder: u64 as HolderId,
        currency: u64 as CurrencyId,
    }
    relation Instrument {
        id: u64 as InstrumentId, fresh,
        symbol: str,
    }
    relation JournalEntry {
        id: u64 as JournalEntryId, fresh,
        source: u64 as SourceId,
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
        tag: u64 as TagId,
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

    closed relation Currency as CurrencyId = { Usd, Eur, Gbp };
    closed relation Source as SourceId = { Manual, Import, System };
    closed relation Tag as TagId = { Fee, Rebate, Adjustment };

    Account(holder)      <= Holder(id);
    Account(currency)    <= Currency(id);
    JournalEntry(source) <= Source(id);
    Posting(entry)       <= JournalEntry(id);
    Posting(account)     <= Account(id);
    Posting(instrument)  <= Instrument(id);
    PostingTag(posting)  <= Posting(id);
    PostingTag(tag)      <= Tag(id);
    OrgParent(child)     <= Org(id);
    OrgParent(parent)    <= Org(id);
    Mandate(account)     <= Account(id);
    Mandate(org)         <= Org(id);
    Mandate(account, active) -> Mandate;
}

/// # Panics
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
    pub const CURRENCY: RelationId = RelationId(9);
    pub const SOURCE: RelationId = RelationId(10);
    pub const TAG: RelationId = RelationId(11);

    /// ids 9..12) sit after every ordinary relation by declaration:
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
    use bumbledb::schema::ValueType;

    const PRE_FUNERAL_FINGERPRINT: &str =
        "c64e3142a655bc9c60d0f0488540aa559210c650dd5ceb3e06761c7f3088cee8";

    fn fingerprint_hex() -> String {
        let fp = bumbledb::schema::fingerprint::fingerprint(schema());
        fp.0.iter().fold(String::new(), |mut acc, b| {
            use std::fmt::Write as _;
            let _ = write!(acc, "{b:02x}");
            acc
        })
    }

    #[test]
    fn the_fingerprint_is_pinned() {
        assert_eq!(
            fingerprint_hex(),
            "63fcf387c3497c42530d91790cd16ec46f843c2bfc7efac2a57443fb2d1cf635",
            "the ledger schema changed — re-baseline corpora and reports deliberately"
        );
    }

    #[test]
    fn the_funeral_moved_the_fingerprint() {
        assert_ne!(
            fingerprint_hex(),
            PRE_FUNERAL_FINGERPRINT,
            "the closed-relation ledger must not fingerprint like the enum ledger"
        );
    }

    #[test]
    fn the_statement_roster_matches_the_doc() {
        let schema = schema();
        assert_eq!(
            schema.keys().len() + schema.containments().len(),
            6 + 3 + 12 + 1
        );
        let mut autos = 0;
        let mut containments = Vec::new();
        let mut pointwise = 0;
        for statement in schema.keys() {
            if statement.form().is_pointwise() {
                pointwise += 1;
                assert_eq!(statement.relation, ids::MANDATE);
            } else {
                autos += 1;
            }
        }
        for statement in schema.containments() {
            containments.push((statement.source.relation, statement.target.relation));
        }
        assert_eq!(
            autos, 9,
            "Holder/Account/Instrument/JournalEntry/Posting/Org fresh ids \
             plus the Currency/Source/Tag closed auto-keys"
        );
        assert_eq!(pointwise, 1, "the pointwise Mandate key");
        assert_eq!(
            containments,
            vec![
                (ids::ACCOUNT, ids::HOLDER),
                (ids::ACCOUNT, ids::CURRENCY),
                (ids::JOURNAL_ENTRY, ids::SOURCE),
                (ids::POSTING, ids::JOURNAL_ENTRY),
                (ids::POSTING, ids::ACCOUNT),
                (ids::POSTING, ids::INSTRUMENT),
                (ids::POSTING_TAG, ids::POSTING),
                (ids::POSTING_TAG, ids::TAG),
                (ids::ORG_PARENT, ids::ORG),
                (ids::ORG_PARENT, ids::ORG),
                (ids::MANDATE, ids::ACCOUNT),
                (ids::MANDATE, ids::ORG),
            ],
            "the nine doc containments plus the three vocabulary containments, in source order"
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
            "Currency",
            "Source",
            "Tag",
        ]
        .iter()
        .enumerate()
        {
            let rel = bumbledb::RelationId(u32::try_from(idx).expect("small"));
            assert_eq!(s.relation(rel).name(), *name);
        }
        assert_eq!(s.relations().len(), 12);
        for rel in 0..ids::RELATIONS {
            assert!(
                s.relation(bumbledb::RelationId(rel))
                    .body()
                    .closed_rows()
                    .is_none(),
                "every writable relation precedes the closed vocabulary"
            );
        }
        for rel in [ids::CURRENCY, ids::SOURCE, ids::TAG] {
            assert!(s.relation(rel).body().closed_rows().is_some());
        }
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
