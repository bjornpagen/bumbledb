use bumbledb::RelationId;

use crate::gen::{Scale, Sizes};
use crate::schema::ids;

impl Sizes {
    #[must_use]
    pub fn of(scale: Scale) -> Self {
        let postings: u64 = match scale {
            Scale::S => 100_000,
            Scale::M => 1_000_000,
            Scale::L => 10_000_000,
        };
        let accounts = postings / 200;
        let account_tags = accounts * 2;
        Self {
            postings,
            transfers: postings / 2,
            accounts,
            holders: accounts / 4,
            instruments: 512,
            currencies: 16,
            tags: 256,
            account_tags,
            tag_notes: account_tags / 4,
        }
    }

    /// Rows for one relation.
    #[must_use]
    pub fn rows(&self, rel: RelationId) -> u64 {
        match rel {
            ids::CURRENCY => self.currencies,
            ids::HOLDER => self.holders,
            ids::INSTRUMENT => self.instruments,
            ids::ACCOUNT => self.accounts,
            ids::TRANSFER => self.transfers,
            ids::POSTING => self.postings,
            ids::TAG => self.tags,
            ids::ACCOUNT_TAG => self.account_tags,
            ids::TAG_NOTE => self.tag_notes,
            _ => unreachable!("nine ledger relations"),
        }
    }

    /// The hot-account set: the first `max(1, accounts/1000)` account ids
    /// receive [`HOT_SHARE_PCT`]% of postings.
    #[must_use]
    pub fn hot_accounts(&self) -> u64 {
        (self.accounts / 1000).max(1)
    }
}
