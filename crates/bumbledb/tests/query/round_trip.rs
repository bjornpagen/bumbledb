//! Notation round trips over the ledger: each source lowers, validates and
//! renders to its normalized text, and the normalized text lowers to the
//! same query.
use bumbledb::query;

use crate::common::pin;
use crate::notation::ledger::Ledger;

/// Whitespace-free text, so a render compares against `stringify!` tokens.
fn strip(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

macro_rules! round_trip {
    ($name:ident, { $($source:tt)+ }, { $($normalized:tt)+ }) => {
        #[test]
        fn $name() {
            let lowered = query!(Ledger { $($source)+ });
            let reparsed = query!(Ledger { $($normalized)+ });
            assert_eq!(*reparsed, *lowered, "the normalized text lowers to the same query");
            let rendered = pin(stringify!($name), Ledger, &lowered);
            assert_eq!(strip(&rendered), strip(stringify!($($normalized)+)));
        }
    };
}

round_trip!(punning_and_field_vars,
    { (name) | Holder(id: h, name); },
    { (v1) | Holder(id: v0, name: v1); });

round_trip!(negative_literal_selection,
    { (id) | Posting(id, amount == -100); },
    { (v0) | Posting(id: v0, amount == -100); });

round_trip!(bare_handle_selection,
    { (id) | Account(id, currency == Usd); },
    { (v0) | Account(id: v0, currency == Usd); });

round_trip!(param_selection,
    { (id) | Posting(id, account == ?acct); },
    { (v0) | Posting(id: v0, account == ?0); });

round_trip!(point_membership,
    { (org) | Mandate(org, active), ?today in active; },
    { (v0) | Mandate(org: v0, active: v1), ?0 in v1; });

round_trip!(allen_literal_mask,
    { (org) | Mandate(org, active), Allen(active, INTERSECTS, ?window); },
    { (v0) | Mandate(org: v0, active: v1), Allen(v1, INTERSECTS, ?0); });

round_trip!(min_and_max,
    { (account, lo: Min(amount), hi: Max(amount)) | Posting(account, amount); },
    { (v0, Min(v1), Max(v1)) | Posting(account: v0, amount: v1); });

round_trip!(pack,
    { (org, busy: Pack(active)) | Mandate(org, active); },
    { (v0, Pack(v1)) | Mandate(org: v0, active: v1); });

round_trip!(rule_union,
    { (id) | Account(id, currency == Usd);
      (id) | Account(id, currency == Eur); },
    { (v0) | Account(id: v0, currency == Usd);
      (v0) | Account(id: v0, currency == Eur); });

round_trip!(rooted_rec,
    { rec reach(o) | Org(id: o), o == ?root;
      rec reach(p) | OrgParent(child: c, parent: p), reach(c);
      (p) | Org(id: p), reach(p); },
    { rec(v0) | Org(id: v0), v0 == ?0;
      rec(v1) | OrgParent(child: v0, parent: v1), interior 0(v0);
      (v0) | Org(id: v0), interior 0(v0); });

round_trip!(interior_aggregate,
    { interior totals(account, total: Sum(amount)) | Posting(account, amount);
      (a, t) | totals(a, t); },
    { interior 0(v0, Sum(v1)) | Posting(account: v0, amount: v1);
      (v0, v1) | interior 0(v0, v1); });
