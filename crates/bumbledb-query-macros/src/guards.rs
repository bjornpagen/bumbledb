//! Captured and row-bound plans keep sources and exact transport explicit.
use super::{
    Import, ImportKind, Name, Parse, Scope, Tokens, events, expect_ident, expect_punct, fail,
    predicates, take_paren_group,
};

pub(super) struct Expression {
    predicate: predicates::Expression,
    companions: Vec<predicates::Expression>,
    sources: Vec<Name>,
    plan: Plan,
    operation: Name,
    input: Option<Name>,
}
enum Plan {
    Captured(Name),
    Bound {
        source: Name,
        identity: Option<Name>,
    },
}
fn plan(tokens: &mut Tokens) -> Parse<Plan> {
    let name = expect_ident(tokens, "a guard import, Existing or Refine")?;
    if matches!(name.text.as_str(), "Existing" | "Refine")
        && matches!(tokens.peek(), Some(proc_macro::TokenTree::Group(_)))
    {
        let (mut args, _) = take_paren_group(tokens, "the bound source plan")?;
        let source = expect_ident(&mut args, "a full Event variable")?;
        let identity = if name.text == "Refine" {
            expect_punct(
                &mut args,
                ',',
                "a comma before the bytes<32> identity variable",
            )?;
            Some(expect_ident(&mut args, "a bytes<32> identity variable")?)
        } else {
            None
        };
        end(&mut args)?;
        Ok(Plan::Bound { source, identity })
    } else {
        Ok(Plan::Captured(name))
    }
}
fn source_plan(tokens: &mut Tokens) -> Parse<(Plan, Vec<Name>)> {
    let mut trial = tokens.clone();
    let name = expect_ident(&mut trial, "a guard plan or CommonSources")?;
    if name.text != "CommonSources"
        || !matches!(trial.peek(), Some(proc_macro::TokenTree::Group(_)))
    {
        return Ok((plan(tokens)?, Vec::new()));
    }
    *tokens = trial;
    let (mut args, span) = take_paren_group(tokens, "CommonSources' plan and sources")?;
    let base = plan(&mut args)?;
    let mut sources = Vec::new();
    expect_punct(
        &mut args,
        ',',
        "a comma before CommonSources' nonempty source roster",
    )?;
    loop {
        sources.push(expect_ident(&mut args, "a full Event source variable")?);
        if sources.len() > 4094 {
            return fail(span, "query!: common source roster exceeds shape budget");
        }
        if args.peek().is_none() {
            break;
        }
        expect_punct(&mut args, ',', "a comma between source variables")?;
        if args.peek().is_none() {
            break;
        }
    }
    Ok((base, sources))
}
fn end(tokens: &mut Tokens) -> Parse<()> {
    if let Some(extra) = tokens.next() {
        fail(extra.span(), "query!: unexpected token in guard expression")
    } else {
        Ok(())
    }
}
pub(super) fn parse(tokens: &mut Tokens) -> Parse<Expression> {
    let (mut body, _) = take_paren_group(tokens, "Guard's operation")?;
    let operation = expect_ident(&mut body, "Holds, Fails, Undefined, Lift or Descend")?;
    if !matches!(
        operation.text.as_str(),
        "Holds" | "Fails" | "Undefined" | "Lift" | "Descend"
    ) {
        return fail(operation.span, "query!: unknown guard operation");
    }
    let (mut args, _) = take_paren_group(&mut body, "guard operands")?;
    // One outer Guard node shares the predicate/numerical depth allowance.
    let predicate = predicates::expression(&mut args, 0, 1)?;
    let mut nodes = 1;
    predicates::check_shape(&predicate, 2, &mut nodes)?;
    expect_punct(&mut args, ',', "a comma before the guard plan")?;
    let mut companions = Vec::new();
    let mut trial = args.clone();
    let name = expect_ident(&mut trial, "a guard plan or Common")?;
    let (plan, sources) =
        if name.text == "Common" && matches!(trial.peek(), Some(proc_macro::TokenTree::Group(_))) {
            args = trial;
            let (mut roster, _) = take_paren_group(&mut args, "Common's plan and predicates")?;
            let source = source_plan(&mut roster)?;
            expect_punct(
                &mut roster,
                ',',
                "a comma before Common's nonempty predicate roster",
            )?;
            loop {
                let companion = predicates::expression(&mut roster, 0, 1)?;
                predicates::check_shape(&companion, 2, &mut nodes)?;
                companions.push(companion);
                if roster.peek().is_none() {
                    break;
                }
                expect_punct(&mut roster, ',', "a comma between Common predicates")?;
                if roster.peek().is_none() {
                    break;
                }
            }
            source
        } else {
            source_plan(&mut args)?
        };
    if nodes + sources.len() > 4096 {
        return fail(
            operation.span,
            "query!: guard expression exceeds shape budget",
        );
    }
    let input = if matches!(operation.text.as_str(), "Lift" | "Descend") {
        expect_punct(&mut args, ',', "a comma before the Event input")?;
        Some(expect_ident(&mut args, "an Event variable")?)
    } else {
        None
    };
    end(&mut args)?;
    end(&mut body)?;
    Ok(Expression {
        predicate,
        companions,
        sources,
        plan,
        operation,
        input,
    })
}
impl Expression {
    pub(super) fn emit(&self, scope: &Scope, imports: &[Import]) -> Parse<String> {
        let plan = match &self.plan {
            Plan::Captured(name) => format!(
                "::bumbledb::GuardPlanExpr::Captured({})",
                events::imported(name, imports, ImportKind::Guard)?
            ),
            Plan::Bound { source, identity } => {
                let source = scope.head_var(source)?;
                if let Some(identity) = identity {
                    format!(
                        "::bumbledb::GuardPlanExpr::Refine {{ source: ::bumbledb::VarId({source}), identity: ::bumbledb::VarId({}) }}",
                        scope.head_var(identity)?
                    )
                } else {
                    format!("::bumbledb::GuardPlanExpr::Existing(::bumbledb::VarId({source}))")
                }
            }
        };
        let predicate = self.predicate.emit(scope, imports, 1)?;
        let companions = self
            .companions
            .iter()
            .map(|p| p.emit(scope, imports, 1))
            .collect::<Parse<Vec<_>>>()?
            .join(",");
        let sources = self
            .sources
            .iter()
            .map(|name| {
                scope
                    .head_var(name)
                    .map(|v| format!("::bumbledb::VarId({v})"))
            })
            .collect::<Parse<Vec<_>>>()?
            .join(",");
        let input = self.input.as_ref().map(|v| scope.head_var(v)).transpose()?;
        let operation = if let Some(v) = input {
            format!("{}(::bumbledb::VarId({v}))", self.operation.text)
        } else {
            self.operation.text.clone()
        };
        Ok(format!(
            "::bumbledb::GuardExpr {{ plan: {plan}, predicate: {predicate}, companions: ::std::vec![{companions}], sources: ::std::vec![{sources}], operation: ::bumbledb::GuardOp::{operation} }}"
        ))
    }
}
