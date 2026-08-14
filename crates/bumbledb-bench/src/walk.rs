//! One walk over a Query: interiors, then rec (base + step), then main.
//! Empty prefixes are skipped — CQ `params_for` / coverage RNG and
//! anchors stay identical when interiors and rec are empty.

use bumbledb::{Query, Rule};

/// Every rule-list in declaration order: interiors, rec.base, rec.rec, main.
pub fn rules(query: &Query) -> impl Iterator<Item = &Rule> {
    query
        .interiors
        .iter()
        .flat_map(|interior| interior.rules.iter())
        .chain(
            query
                .rec
                .iter()
                .flat_map(|rec| rec.base.iter().chain(&rec.rec)),
        )
        .chain(query.rules.iter())
}

/// Apply `f` to every rule in declaration order. Stops and returns
/// `false` on the first `f` that returns `false`.
pub fn every_rule_mut(query: &mut Query, mut f: impl FnMut(&mut Rule) -> bool) -> bool {
    for interior in &mut query.interiors {
        if !interior.rules.iter_mut().all(&mut f) {
            return false;
        }
    }
    if let Some(rec) = &mut query.rec {
        if !rec.base.iter_mut().all(&mut f) {
            return false;
        }
        if !rec.rec.iter_mut().all(&mut f) {
            return false;
        }
    }
    query.rules.iter_mut().all(f)
}
