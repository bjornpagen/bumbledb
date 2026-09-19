//! Exact partial truth expressions. Numerical comparisons build a sign test of
//! the exact difference; ordinary row comparisons never consume predicates.
use super::{
    Import, ImportKind, Name, Parse, Scope, Tokens, events, expect_ident, expect_punct, fail,
    numbers, parse_int, peek_punct, peek_span, take_paren_group,
};
use proc_macro::{Delimiter, TokenTree};

pub(super) enum Expression {
    Variable(Name),
    Region(Name, Name),
    Sign(numbers::Expression, u8),
    Imported(Name),
    Negate(Box<Self>),
    Binary(u8, Box<Self>, Box<Self>),
    OnDomain(Box<Self>, Name),
}
pub(super) fn parse(tokens: &mut Tokens) -> Parse<Expression> {
    let (mut args, _) = take_paren_group(tokens, "Predicate's expression")?;
    let value = expression(&mut args, 0, 0)?;
    end(&mut args)?;
    Ok(value)
}
pub(super) fn parse_test(tokens: &mut Tokens) -> Parse<(Expression, String)> {
    let (mut args, _) = take_paren_group(tokens, "PredicateTest's explicit quantifier")?;
    let name = expect_ident(&mut args, "Possibly, Always or IsTotal")?;
    if !matches!(name.text.as_str(), "Possibly" | "Always" | "IsTotal") {
        return fail(name.span, "query!: unknown predicate quantifier");
    }
    let value = parse(&mut args)?;
    end(&mut args)?;
    Ok((value, name.text))
}
fn end(tokens: &mut Tokens) -> Parse<()> {
    if let Some(extra) = tokens.next() {
        fail(
            extra.span(),
            "query!: unexpected token in predicate expression",
        )
    } else {
        Ok(())
    }
}
pub(super) fn expression(tokens: &mut Tokens, precedence: u8, depth: usize) -> Parse<Expression> {
    if precedence == 3 {
        return unary(tokens, depth);
    }
    let mut left = expression(tokens, precedence + 1, depth)?;
    let (token, op) = match precedence {
        0 => ('|', 14),
        1 => ('^', 6),
        _ => ('&', 8),
    };
    while peek_punct(tokens, token) {
        tokens.next();
        let right = expression(tokens, precedence + 1, depth)?;
        left = bounded(Expression::Binary(op, Box::new(left), Box::new(right)))?;
    }
    Ok(left)
}
fn comparison(tokens: &mut Tokens) -> Option<u8> {
    let TokenTree::Punct(first) = tokens.peek()? else {
        return None;
    };
    let first = first.as_char();
    if !matches!(first, '<' | '>' | '=' | '!') {
        return None;
    }
    let mut next = tokens.clone();
    next.next();
    let equal = peek_punct(&mut next, '=');
    if matches!(first, '=' | '!') && !equal {
        return None;
    }
    tokens.next();
    if equal {
        tokens.next();
    }
    Some(match (first, equal) {
        ('<', false) => 1,
        ('<', true) => 3,
        ('>', false) => 4,
        ('>', true) => 6,
        ('=', _) => 2,
        ('!', _) => 5,
        _ => unreachable!(),
    })
}
fn mask(tokens: &mut Tokens, bound: u8) -> Parse<u8> {
    let n = parse_int(tokens, "a truth/sign mask")?;
    let text = n
        .text
        .trim_end_matches("u64")
        .trim_end_matches("i64")
        .replace('_', "");
    let (radix, digits) = if let Some(n) = text.strip_prefix("0x") {
        (16, n)
    } else if let Some(n) = text.strip_prefix("0o") {
        (8, n)
    } else if let Some(n) = text.strip_prefix("0b") {
        (2, n)
    } else {
        (10, text.as_str())
    };
    if !n.negative
        && let Ok(value) = u8::from_str_radix(digits, radix)
        && value < bound
    {
        return Ok(value);
    }
    fail(
        proc_macro::Span::call_site(),
        "query!: predicate mask is out of range",
    )
}
fn unary(tokens: &mut Tokens, depth: usize) -> Parse<Expression> {
    if depth > 128 {
        return fail(
            peek_span(tokens),
            "query!: predicate expression is too deeply nested",
        );
    }
    if peek_punct(tokens, '!') {
        tokens.next();
        return bounded(Expression::Negate(Box::new(unary(tokens, depth + 1)?)));
    }
    // A parenthesized numerical left operand and a parenthesized predicate use
    // the same token shape. Only a complete numerical comparison commits here.
    let mut trial = tokens.clone();
    if let Ok(left) = numbers::expression(&mut trial, 0, depth + 1)
        && let Some(signs) = comparison(&mut trial)
    {
        let right = numbers::expression(&mut trial, 0, depth + 1)?;
        *tokens = trial;
        return bounded(Expression::Sign(
            numbers::Expression::Binary("Subtract", Box::new(left), Box::new(right)),
            signs,
        ));
    }
    if matches!(tokens.peek(),Some(TokenTree::Group(group)) if group.delimiter()==Delimiter::Parenthesis)
    {
        let (mut args, _) = take_paren_group(tokens, "predicate subexpression")?;
        let value = expression(&mut args, 0, depth + 1)?;
        end(&mut args)?;
        return Ok(value);
    }
    let name = expect_ident(tokens, "a predicate variable, comparison or operator")?;
    if !matches!(tokens.peek(), Some(TokenTree::Group(_))) {
        return Ok(Expression::Variable(name));
    }
    let (mut args, _) = take_paren_group(tokens, "predicate operands")?;
    let value = match name.text.as_str() {
        "Region" => {
            let domain = expect_ident(&mut args, "a declared number_domain import")?;
            expect_punct(&mut args, ',', "a comma")?;
            Expression::Region(
                domain,
                expect_ident(&mut args, "a declared parameter_region import")?,
            )
        }
        "Sign" => {
            let number = numbers::expression(&mut args, 0, depth + 1)?;
            expect_punct(&mut args, ',', "a comma")?;
            Expression::Sign(number, mask(&mut args, 8)?)
        }
        "Bool4" => {
            let op = mask(&mut args, 16)?;
            expect_punct(&mut args, ',', "a comma")?;
            let left = expression(&mut args, 0, depth + 1)?;
            expect_punct(&mut args, ',', "a comma")?;
            let right = expression(&mut args, 0, depth + 1)?;
            Expression::Binary(op, Box::new(left), Box::new(right))
        }
        "Imported" => Expression::Imported(expect_ident(&mut args, "a declared predicate import")?),
        "OnDomain" => {
            let value = expression(&mut args, 0, depth + 1)?;
            expect_punct(&mut args, ',', "a comma")?;
            Expression::OnDomain(
                Box::new(value),
                expect_ident(&mut args, "a declared number_domain import")?,
            )
        }
        _ => {
            return fail(
                name.span,
                "query!: unknown predicate operator or missing numerical comparison",
            );
        }
    };
    end(&mut args)?;
    bounded(value)
}
fn bounded(value: Expression) -> Parse<Expression> {
    check_shape(&value, 1, &mut 0)?;
    Ok(value)
}
pub(super) fn check_shape(value: &Expression, depth: usize, nodes: &mut usize) -> Parse<()> {
    let mut pending = vec![(value, depth)];
    while let Some((node, depth)) = pending.pop() {
        *nodes += 1;
        if depth > 128 || *nodes > 4096 {
            return fail(
                proc_macro::Span::call_site(),
                "query!: predicate expression exceeds shape budget",
            );
        }
        match node {
            Expression::Sign(number, _) => numbers::check_shape(number, depth + 1, nodes)?,
            Expression::Negate(value) | Expression::OnDomain(value, _) => {
                pending.push((value, depth + 1));
            }
            Expression::Binary(_, left, right) => {
                pending.push((right, depth + 1));
                pending.push((left, depth + 1));
            }
            _ => {}
        }
    }
    Ok(())
}
impl Expression {
    pub(super) fn emit(&self, scope: &Scope, imports: &[Import], depth: usize) -> Parse<String> {
        if depth > 128 {
            return fail(
                proc_macro::Span::call_site(),
                "query!: predicate expression is too deeply nested",
            );
        }
        let body = match self {
            Self::Variable(name) => format!("Var(::bumbledb::VarId({}))", scope.head_var(name)?),
            Self::Sign(number, signs) => format!(
                "Sign {{ number: {}, signs: ::bumbledb::event::PolynomialSigns::new({signs}u8).expect(\"checked mask\") }}",
                number.emit(scope, imports, depth + 1)?
            ),
            Self::Region(domain, region) => format!(
                "Region {{ domain: {}, region: {} }}",
                events::imported(domain, imports, ImportKind::NumberDomain)?,
                events::imported(region, imports, ImportKind::ParameterRegion)?
            ),
            Self::Imported(name) => format!(
                "Imported({})",
                events::imported(name, imports, ImportKind::Predicate)?
            ),
            Self::Negate(value) => format!(
                "Negate(::std::boxed::Box::new({}))",
                value.emit(scope, imports, depth + 1)?
            ),
            Self::Binary(op, left, right) => format!(
                "Binary {{ op: ::bumbledb::event::BoolOp4::new({op}u8).expect(\"checked mask\"), left: ::std::boxed::Box::new({}), right: ::std::boxed::Box::new({}) }}",
                left.emit(scope, imports, depth + 1)?,
                right.emit(scope, imports, depth + 1)?
            ),
            Self::OnDomain(value, domain) => format!(
                "OnDomain {{ value: ::std::boxed::Box::new({}), domain: {} }}",
                value.emit(scope, imports, depth + 1)?,
                events::imported(domain, imports, ImportKind::NumberDomain)?
            ),
        };
        Ok(format!("::bumbledb::PredicateExpr::{body}"))
    }
}
