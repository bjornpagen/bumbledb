use bumbledb::RelationId;

use crate::corpus_gen::{MANDATE_SEGMENTS, Scale, Sizes};
use crate::schema::ids;

impl Sizes {
    #[must_use]
    pub fn of(scale: Scale) -> Self {
        let (postings, instruments, orgs): (u64, u64, u64) = match scale {
            Scale::Tiny => (1_024, 32, 8),
            Scale::S => (100_000, 512, 64),
            Scale::M => (1_000_000, 512, 64),
            Scale::L => (10_000_000, 512, 64),
        };
        let accounts = postings / 200;
        Self {
            postings,
            entries: postings / 2,
            accounts,
            holders: (accounts / 4).max(1),
            instruments,
            orgs,
            org_parents: orgs - 1,
            posting_tags: postings,
            mandates: accounts * MANDATE_SEGMENTS,
        }
    }

    #[must_use]
    pub fn rows(&self, rel: RelationId) -> u64 {
        match rel {
            ids::HOLDER => self.holders,
            ids::ACCOUNT => self.accounts,
            ids::INSTRUMENT => self.instruments,
            ids::JOURNAL_ENTRY => self.entries,
            ids::POSTING => self.postings,
            ids::POSTING_TAG => self.posting_tags,
            ids::ORG => self.orgs,
            ids::ORG_PARENT => self.org_parents,
            ids::MANDATE => self.mandates,
            _ => unreachable!("nine ledger relations"),
        }
    }

    #[must_use]
    pub fn hot_accounts(&self) -> u64 {
        (self.accounts / 1000).max(1)
    }
}
