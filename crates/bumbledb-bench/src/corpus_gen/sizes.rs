use bumbledb::RelationId;

use crate::corpus_gen::{MANDATE_SEGMENTS, Scale, Sizes};
use crate::schema::ids;

impl Sizes {
    /// The scale ladder's size table — postings, instruments, and orgs
    /// per point; everything else derives. `Tiny` is the fuzz-iteration
    /// point: a full build-store → ops → oracles pass in milliseconds.
    ///
    /// | scale | postings   | instruments | orgs |
    /// |-------|------------|-------------|------|
    /// | Tiny  | 1 024      | 32          | 8    |
    /// | S     | 100 000    | 512         | 64   |
    /// | M     | 1 000 000  | 512         | 64   |
    /// | L     | 10 000 000 | 512         | 64   |
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

    /// Rows for one relation.
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

    /// The hot-account set: the first `max(1, accounts/1000)` account ids
    /// receive [`crate::corpus_gen::HOT_SHARE_PCT`]% of postings.
    #[must_use]
    pub fn hot_accounts(&self) -> u64 {
        (self.accounts / 1000).max(1)
    }
}
