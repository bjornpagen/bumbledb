//! Copy bounded captured source plans and combined guard/predicate programs.
use super::{
    Array, EventBudget, Object, err, exact_fields, numbers, predicates, req, req_at, req_text,
    var_in,
};
use crate::ingress::query::{GuardExpr, GuardPlan};
use bumbledb::{GuardOp, event::SpaceId};

pub(super) fn parse(obj: &Object, budget: &mut EventBudget<'_, '_>) -> napi::Result<GuardExpr> {
    budget.copy.checkpoint()?;
    if budget.nodes == 0 {
        return Err(err("guard expression exceeds shape budget".into()));
    }
    budget.nodes -= 1;
    let kind = req_text(obj, "kind", "guard operation")?;
    let operation = match kind.as_str() {
        "holds" => GuardOp::Holds,
        "fails" => GuardOp::Fails,
        "undefined" => GuardOp::Undefined,
        "lift" => GuardOp::Lift(var_in(obj, "input", "guard transport input")?),
        "descend" => GuardOp::Descend(var_in(obj, "input", "guard transport input")?),
        _ => return Err(err("unknown guard operation".into())),
    };
    let common = Object::keys(obj)?.iter().any(|key| key == "resolve");
    let mut fields = vec!["kind", "predicate", "plan"];
    if matches!(operation, GuardOp::Lift(_) | GuardOp::Descend(_)) {
        fields.push("input");
    }
    if common {
        fields.push("resolve");
    }
    exact_fields(obj, &fields)?;
    let plan = req::<Object>(obj, "plan", "guard plan")?;
    let mode = req_text(&plan, "kind", "guard plan mode")?;
    let plan = match mode.as_str() {
        "existing" | "refine" => {
            let refinement = if mode == "refine" {
                exact_fields(&plan, &["kind", "source", "identity"])?;
                let identity = numbers::bytes(&plan, "identity", budget)?;
                Some(SpaceId(identity.try_into().map_err(|_| {
                    err("guard refinement identity must have 32 bytes".into())
                })?))
            } else {
                exact_fields(&plan, &["kind", "source"])?;
                None
            };
            GuardPlan::Captured {
                source: numbers::bytes(&plan, "source", budget)?,
                refinement,
            }
        }
        "boundExisting" | "boundRefine" => {
            let refinement = if mode == "boundRefine" {
                exact_fields(&plan, &["kind", "source", "identity"])?;
                Some(var_in(&plan, "identity", "guard identity variable")?)
            } else {
                exact_fields(&plan, &["kind", "source"])?;
                None
            };
            GuardPlan::Bound {
                source: var_in(&plan, "source", "guard source variable")?,
                refinement,
            }
        }
        _ => return Err(err("unknown guard plan mode".into())),
    };
    let predicate = predicates::parse(
        &req::<Object>(obj, "predicate", "guard predicate")?,
        2,
        budget,
    )?;
    let mut companions = Vec::new();
    if common {
        let roster: Array = req(obj, "resolve", "common guard predicates")?;
        let len = roster.len() as usize;
        if len == 0 || len > budget.nodes {
            return Err(err("invalid common guard roster or shape budget".into()));
        }
        companions
            .try_reserve_exact(len)
            .map_err(|_| err("guard roster allocation failed".into()))?;
        for i in 0..roster.len() {
            companions.push(predicates::parse(
                &req_at(&roster, i, "common guard predicate")?,
                2,
                budget,
            )?);
        }
    }
    Ok(GuardExpr {
        plan,
        predicate,
        companions,
        operation,
    })
}
