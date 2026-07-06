//! The benchmark's ledger schema (docs/architecture/50-validation.md;
//! `docs/architecture/50-validation.md` owns the ledger decision). Nine
//! relations exercising every engine construct: all six value types,
//! serials, single and compound uniques, single and compound FKs
//! (including the FK-inheritance pattern), enums, interned strings and
//! bytes, bools.
//!
//! Rationale per relation:
//! - `Posting` is the fact table — the 10⁵–10⁷ scale axis.
//! - `AccountTag` carries the compound unique; `TagNote` carries the
//!   compound-FK inheritance pattern targeting it.
//! - `Transfer.extref` is the Bytes exerciser.
//! - `unique(code)` / `unique(label)` are interned-string unique guards.
//!
//! Changing this schema is a deliberate act: it re-baselines every stored
//! corpus digest and every published report (the golden fingerprint test
//! is the tripwire).

bumbledb::schema! {
    relation Currency {
        id: u64 as CurrencyId, serial,
        code: str,
        unique(code),
    }
    relation Holder {
        id: u64 as HolderId, serial,
        name: str,
        region: enum Region { Na, Eu, Apac, Latam },
    }
    relation Instrument {
        id: u64 as InstrumentId, serial,
        symbol: str,
        currency: u64 as CurrencyId, fk(Currency.id),
        kind: enum Kind { Cash, Equity, Bond, Fund },
    }
    relation Account {
        id: u64 as AccountId, serial,
        holder: u64 as HolderId, fk(Holder.id),
        currency: u64 as CurrencyId, fk(Currency.id),
        status: enum Status { Open, Frozen, Closed },
        opened_at: i64,
    }
    relation Transfer {
        id: u64 as TransferId, serial,
        at: i64,
        extref: bytes,
    }
    relation Posting {
        id: u64 as PostingId, serial,
        transfer: u64 as TransferId, fk(Transfer.id),
        account: u64 as AccountId, fk(Account.id),
        instrument: u64 as InstrumentId, fk(Instrument.id),
        amount: i64,
        at: i64,
        memo: str,
        reconciled: bool,
    }
    relation Tag {
        id: u64 as TagId, serial,
        label: str,
        unique(label),
    }
    relation AccountTag {
        account: u64 as AccountId, fk(Account.id),
        tag: u64 as TagId, fk(Tag.id),
        unique(account, tag),
    }
    relation TagNote {
        account: u64 as AccountId,
        tag: u64 as TagId,
        fk(account, tag -> AccountTag.account_tag),
        note: str,
    }
}

/// Relation and field ids by name — no magic numbers in family
/// definitions or the generator (declaration order is the id order).
pub mod ids {
    use bumbledb::{FieldId, RelationId};

    pub const CURRENCY: RelationId = RelationId(0);
    pub const HOLDER: RelationId = RelationId(1);
    pub const INSTRUMENT: RelationId = RelationId(2);
    pub const ACCOUNT: RelationId = RelationId(3);
    pub const TRANSFER: RelationId = RelationId(4);
    pub const POSTING: RelationId = RelationId(5);
    pub const TAG: RelationId = RelationId(6);
    pub const ACCOUNT_TAG: RelationId = RelationId(7);
    pub const TAG_NOTE: RelationId = RelationId(8);

    /// The number of relations — loaders iterate `0..RELATIONS`.
    pub const RELATIONS: u32 = 9;

    pub mod currency {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const CODE: FieldId = FieldId(1);
    }
    pub mod holder {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const NAME: FieldId = FieldId(1);
        pub const REGION: FieldId = FieldId(2);
    }
    pub mod instrument {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const SYMBOL: FieldId = FieldId(1);
        pub const CURRENCY: FieldId = FieldId(2);
        pub const KIND: FieldId = FieldId(3);
    }
    pub mod account {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const HOLDER: FieldId = FieldId(1);
        pub const CURRENCY: FieldId = FieldId(2);
        pub const STATUS: FieldId = FieldId(3);
        pub const OPENED_AT: FieldId = FieldId(4);
    }
    pub mod transfer {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const AT: FieldId = FieldId(1);
        pub const EXTREF: FieldId = FieldId(2);
    }
    pub mod posting {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const TRANSFER: FieldId = FieldId(1);
        pub const ACCOUNT: FieldId = FieldId(2);
        pub const INSTRUMENT: FieldId = FieldId(3);
        pub const AMOUNT: FieldId = FieldId(4);
        pub const AT: FieldId = FieldId(5);
        pub const MEMO: FieldId = FieldId(6);
        pub const RECONCILED: FieldId = FieldId(7);
    }
    pub mod tag {
        use super::FieldId;
        pub const ID: FieldId = FieldId(0);
        pub const LABEL: FieldId = FieldId(1);
    }
    pub mod account_tag {
        use super::FieldId;
        pub const ACCOUNT: FieldId = FieldId(0);
        pub const TAG: FieldId = FieldId(1);
    }
    pub mod tag_note {
        use super::FieldId;
        pub const ACCOUNT: FieldId = FieldId(0);
        pub const TAG: FieldId = FieldId(1);
        pub const NOTE: FieldId = FieldId(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumbledb::schema::{ConstraintDescriptor, ValueType};
    use bumbledb::Fact;

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
            hex, "0f1b316b5f4464e451b325bd7f12faf95b304a95f2731f9c5a7075f1fe9e4684",
            "the ledger schema changed — re-baseline corpora and reports deliberately"
        );
    }

    #[test]
    fn every_engine_construct_is_exercised() {
        let s = schema();
        let mut types = std::collections::HashSet::new();
        let mut serials = 0;
        let mut single_fks = 0;
        let mut compound_fks = 0;
        let mut compound_uniques = 0;
        for relation in s.relations() {
            for field in relation.fields() {
                types.insert(std::mem::discriminant(&field.value_type));
                if field.generation == bumbledb::schema::Generation::Serial {
                    serials += 1;
                }
            }
            for constraint in relation.constraints() {
                match constraint {
                    ConstraintDescriptor::ForeignKey { fields, .. } => {
                        if fields.len() == 1 {
                            single_fks += 1;
                        } else {
                            compound_fks += 1;
                        }
                    }
                    ConstraintDescriptor::Unique { fields, .. } => {
                        if fields.len() > 1 {
                            compound_uniques += 1;
                        }
                    }
                }
            }
        }
        // All six value types (discriminants of ValueType).
        assert_eq!(types.len(), 6, "all six value types present");
        assert!(serials >= 5, "serials: {serials}");
        assert!(single_fks >= 6, "single fks: {single_fks}");
        assert!(compound_fks >= 1, "compound fks: {compound_fks}");
        assert!(
            compound_uniques >= 1,
            "compound uniques: {compound_uniques}"
        );
        // Silence the unused-type warning path: the discriminant set must
        // include an Enum — probe one directly.
        let region = &s
            .relation(ids::HOLDER)
            .field(ids::holder::REGION)
            .value_type;
        assert!(matches!(region, ValueType::Enum { .. }));
    }

    #[test]
    fn tag_note_targets_the_compound_unique() {
        let s = schema();
        let tag_note = s.relation(TagNote::RELATION);
        let fk = tag_note
            .constraints()
            .iter()
            .find_map(|c| match c {
                ConstraintDescriptor::ForeignKey {
                    fields,
                    target_relation,
                    target_constraint,
                    ..
                } if fields.len() == 2 => Some((*target_relation, *target_constraint)),
                _ => None,
            })
            .expect("the compound fk");
        assert_eq!(fk.0, AccountTag::RELATION);
        // AccountTag's compound unique `account_tag` (no serial fields, so
        // it is constraint 0... after any auto-uniques — resolve by name).
        let target = s.relation(AccountTag::RELATION).constraint(fk.1);
        assert_eq!(target.name(), "account_tag");
        assert!(matches!(target, ConstraintDescriptor::Unique { .. }));
    }

    #[test]
    fn the_id_registry_matches_declaration_order() {
        let s = schema();
        for (idx, name) in [
            "Currency",
            "Holder",
            "Instrument",
            "Account",
            "Transfer",
            "Posting",
            "Tag",
            "AccountTag",
            "TagNote",
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
            s.relation(ids::POSTING).field(ids::posting::MEMO).name,
            "memo".into()
        );
    }
}
