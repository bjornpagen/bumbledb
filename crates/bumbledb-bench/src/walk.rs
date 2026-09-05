use bumbledb::{InteriorId, Query, RecRule, RecStep, Rule};

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
                out.extend(interior.rules.iter().cloned());
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
                out.extend(interior.rules.iter().cloned());
            }
            out.extend(rec.base.iter().map(RecRule::to_rule));
            out.extend(rec.rec.iter().map(|step| RecStep::to_rule(step, rec_id)));
            out.extend(rules.iter().cloned());
        }
    }
    out.into_iter()
}

pub fn every_rule_mut(query: &mut Query, mut f: impl FnMut(&mut Rule) -> bool) -> bool {
    match query {
        Query {
            interiors,
            rules,
            rec: None,
            ..
        } => {
            for interior in interiors {
                for rule in &mut interior.rules {
                    if !f(rule) {
                        return false;
                    }
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
                for rule in &mut interior.rules {
                    if !f(rule) {
                        return false;
                    }
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
