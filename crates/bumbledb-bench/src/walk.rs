//! One walk over a Query: interiors, then rec (base + step), then main.
//! Empty prefixes are skipped — CQ `params_for` / coverage RNG and
//! anchors stay identical when interiors are empty.

use bumbledb::{InteriorId, ProjectionRule, Query, RecRule, RecStep, Rule};

/// Every rule-list in declaration order: interiors, rec.base, rec.rec, main.
/// Derived arms lower through `to_rule` so callers see one [`Rule`] shape.
pub fn rules(query: &Query) -> impl Iterator<Item = Rule> {
    let mut out: Vec<Rule> = Vec::new();
    match query {
        Query {
            interiors,
            rules,
            rec: None,
            ..
        } => {
            for interior in interiors {
                out.extend(interior.rules.iter().map(ProjectionRule::to_rule));
            }
            out.extend(rules.iter().cloned());
        }
        Query {
            interiors,
            rec: Some(rec),
            rules,
            ..
        } => {
            let rec_id = InteriorId(u32::try_from(interiors.len()).expect("interior id fits u32"));
            for interior in interiors {
                out.extend(interior.rules.iter().map(ProjectionRule::to_rule));
            }
            out.extend(rec.base.iter().map(RecRule::to_rule));
            out.extend(rec.rec.iter().map(|step| RecStep::to_rule(step, rec_id)));
            out.extend(rules.iter().cloned());
        }
    }
    out.into_iter()
}

/// Apply `f` to every rule in declaration order. Stops and returns
/// `false` on the first `f` that returns `false`. Derived arms are
/// lowered, mutated, and written back through their condition lists
/// (the contradiction plant and converse twin only touch conditions).
pub fn every_rule_mut(query: &mut Query, mut f: impl FnMut(&mut Rule) -> bool) -> bool {
    match query {
        Query {
            interiors,
            rules,
            rec: None,
            ..
        } => {
            for interior in interiors {
                for proj in &mut interior.rules {
                    let mut rule = proj.to_rule();
                    if !f(&mut rule) {
                        return false;
                    }
                    proj.conditions = rule.conditions;
                    proj.atoms = rule.atoms;
                    proj.negated = rule.negated;
                }
            }
            rules.iter_mut().all(f)
        }
        Query {
            interiors,
            rec: Some(rec),
            rules,
            ..
        } => {
            let rec_id = InteriorId(u32::try_from(interiors.len()).expect("interior id fits u32"));
            for interior in interiors {
                for proj in &mut interior.rules {
                    let mut rule = proj.to_rule();
                    if !f(&mut rule) {
                        return false;
                    }
                    proj.conditions = rule.conditions;
                    proj.atoms = rule.atoms;
                    proj.negated = rule.negated;
                }
            }
            for base in rec.base.iter_mut() {
                let mut rule = base.to_rule();
                if !f(&mut rule) {
                    return false;
                }
                base.conditions = rule.conditions;
                base.atoms = rule.atoms;
            }
            for step in rec.rec.iter_mut() {
                let mut rule = step.to_rule(rec_id);
                if !f(&mut rule) {
                    return false;
                }
                step.conditions = rule.conditions;
            }
            rules.iter_mut().all(f)
        }
    }
}
