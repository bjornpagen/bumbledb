//! Captured source plans keep guard construction and exact transport explicit.
use super::{
    Import, ImportKind, Name, Parse, Scope, Tokens, events, expect_ident, expect_punct, fail,
    peek_punct, predicates, take_paren_group,
};

pub(super) struct Expression {
    predicate: predicates::Expression,
    companions: Vec<predicates::Expression>,
    plan: Name,
    operation: Name,
    input: Option<Name>,
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
    let mut plan = expect_ident(&mut args, "a declared guard plan or Common")?;
    let mut companions = Vec::new();
    if plan.text == "Common" && !peek_punct(&mut args, ',') && args.peek().is_some() {
        let (mut roster, _) = take_paren_group(&mut args, "Common's plan and predicates")?;
        plan = expect_ident(&mut roster, "a declared guard plan")?;
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
        plan,
        operation,
        input,
    })
}
impl Expression {
    pub(super) fn emit(&self, scope: &Scope, imports: &[Import]) -> Parse<String> {
        let plan = events::imported(&self.plan, imports, ImportKind::Guard)?;
        let predicate = self.predicate.emit(scope, imports, 1)?;
        let companions = self
            .companions
            .iter()
            .map(|p| p.emit(scope, imports, 1))
            .collect::<Parse<Vec<_>>>()?
            .join(",");
        let input = self.input.as_ref().map(|v| scope.head_var(v)).transpose()?;
        let operation = if let Some(v) = input {
            format!("{}(::bumbledb::VarId({v}))", self.operation.text)
        } else {
            self.operation.text.clone()
        };
        Ok(format!(
            "::bumbledb::GuardExpr {{ plan: {plan}, predicate: {predicate}, companions: ::std::vec![{companions}], operation: ::bumbledb::GuardOp::{operation} }}"
        ))
    }
}
