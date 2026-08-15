//! One walk over a Query: interiors, then rec (base + step), then main.
//! Empty prefixes are skipped — CQ `params_for` / coverage RNG and
//! anchors stay identical when interiors are empty.

use bumbledb::{Query, Rule};

/// Every rule-list in declaration order: interiors, rec.base, rec.rec, main.
pub fn rules(query: &Query) -> impl Iterator<Item = &Rule> {
    let mut out: Vec<&Rule> = Vec::new();
    match query {
        Query::Cq {
            interiors, rules, ..
        } => {
            for interior in interiors {
                out.extend(&interior.rules);
            }
            out.extend(rules);
        }
        Query::Reach {
            interiors,
            rec,
            rules,
            ..
        } => {
            for interior in interiors {
                out.extend(&interior.rules);
            }
            out.extend(&rec.base);
            out.extend(&rec.rec);
            out.extend(rules);
        }
    }
    out.into_iter()
}

/// Apply `f` to every rule in declaration order. Stops and returns
/// `false` on the first `f` that returns `false`.
pub fn every_rule_mut(query: &mut Query, mut f: impl FnMut(&mut Rule) -> bool) -> bool {
    match query {
        Query::Cq {
            interiors, rules, ..
        } => {
            for interior in interiors {
                if !interior.rules.iter_mut().all(&mut f) {
                    return false;
                }
            }
            rules.iter_mut().all(f)
        }
        Query::Reach {
            interiors,
            rec,
            rules,
            ..
        } => {
            for interior in interiors {
                if !interior.rules.iter_mut().all(&mut f) {
                    return false;
                }
            }
            if !rec.base.iter_mut().all(&mut f) {
                return false;
            }
            if !rec.rec.iter_mut().all(&mut f) {
                return false;
            }
            rules.iter_mut().all(f)
        }
    }
}
