//! Exact numerical heads, separate from ordinary scalar Compute expressions.
use super::{
    Import, ImportKind, Int, Name, Parse, Scope, Tokens, events, expect_ident, expect_punct, fail,
    parse_int, peek_punct, peek_span, take_paren_group,
};
use proc_macro::{Delimiter, TokenTree};

pub(super) enum Expression {
    Variable(Name),
    Integer(Name),
    Component(&'static str, Name),
    Literal(Int),
    Imported(Name),
    Binary(&'static str, Box<Self>, Box<Self>),
    Unary(&'static str, Box<Self>),
    Pow(Box<Self>, String),
    OnDomain(Box<Self>, Name),
}

pub(super) fn parse(tokens: &mut Tokens) -> Parse<Expression> {
    let (mut args, _) = take_paren_group(tokens, "Number's expression")?;
    let value = expression(&mut args, 0, 0)?;
    end(&mut args)?;
    Ok(value)
}
fn end(tokens: &mut Tokens) -> Parse<()> {
    if let Some(extra) = tokens.next() {
        fail(
            extra.span(),
            "query!: unexpected token in numerical expression",
        )
    } else {
        Ok(())
    }
}
fn expression(tokens: &mut Tokens, precedence: u8, depth: usize) -> Parse<Expression> {
    if precedence == 2 {
        return unary(tokens, depth);
    }
    let mut left = expression(tokens, precedence + 1, depth)?;
    loop {
        let op = if precedence == 0 {
            if peek_punct(tokens, '+') {
                Some("Add")
            } else if peek_punct(tokens, '-') {
                Some("Subtract")
            } else {
                None
            }
        } else if peek_punct(tokens, '*') {
            Some("Multiply")
        } else if peek_punct(tokens, '/') {
            Some("Divide")
        } else {
            None
        };
        let Some(op) = op else {
            break;
        };
        tokens.next();
        let right = expression(tokens, precedence + 1, depth)?;
        left = bounded(Expression::Binary(op, Box::new(left), Box::new(right)))?;
    }
    Ok(left)
}
fn unary(tokens: &mut Tokens, depth: usize) -> Parse<Expression> {
    if depth > 128 {
        return fail(
            peek_span(tokens),
            "query!: numerical expression is too deeply nested",
        );
    }
    if peek_punct(tokens, '-') {
        let mut lookahead = tokens.clone();
        lookahead.next();
        if matches!(lookahead.peek(), Some(TokenTree::Literal(_))) {
            return Ok(Expression::Literal(parse_int(
                tokens,
                "an exact integer literal",
            )?));
        }
        tokens.next();
        return bounded(Expression::Unary(
            "Negate",
            Box::new(unary(tokens, depth + 1)?),
        ));
    }
    if matches!(tokens.peek(), Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis)
    {
        let (mut args, _) = take_paren_group(tokens, "numerical subexpression")?;
        let value = expression(&mut args, 0, depth + 1)?;
        end(&mut args)?;
        return Ok(value);
    }
    if matches!(tokens.peek(), Some(TokenTree::Literal(_))) {
        return Ok(Expression::Literal(parse_int(
            tokens,
            "an exact integer literal",
        )?));
    }
    let name = expect_ident(tokens, "a numerical variable or operator")?;
    if !matches!(tokens.peek(), Some(TokenTree::Group(_))) {
        return Ok(Expression::Variable(name));
    }
    let (mut args, _) = take_paren_group(tokens, "numerical operands")?;
    let value = match name.text.as_str() {
        "Value" | "Numerator" | "EvidenceMass" => Expression::Component(
            match name.text.as_str() {
                "Value" => "Value",
                "Numerator" => "Numerator",
                _ => "EvidenceMass",
            },
            expect_ident(&mut args, "a probability or expectation variable")?,
        ),
        "Integer" => Expression::Integer(expect_ident(&mut args, "an integer column")?),
        "Imported" => Expression::Imported(expect_ident(&mut args, "a declared number import")?),
        "Abs" => Expression::Unary("Abs", Box::new(expression(&mut args, 0, depth + 1)?)),
        "Min" | "Max" => {
            let left = expression(&mut args, 0, depth + 1)?;
            expect_punct(&mut args, ',', "a comma")?;
            let right = expression(&mut args, 0, depth + 1)?;
            Expression::Binary(
                if name.text == "Min" { "Min" } else { "Max" },
                Box::new(left),
                Box::new(right),
            )
        }
        "Pow" => {
            let value = expression(&mut args, 0, depth + 1)?;
            expect_punct(&mut args, ',', "a comma")?;
            let exponent = parse_int(&mut args, "a nonnegative u32 power")?;
            if exponent.negative {
                return fail(name.span, "query!: Pow requires a natural exponent");
            }
            let text = exponent
                .text
                .trim_end_matches("u64")
                .trim_end_matches("i64")
                .replace('_', "");
            let (radix, digits) = if let Some(s) = text.strip_prefix("0x") {
                (16, s)
            } else if let Some(s) = text.strip_prefix("0o") {
                (8, s)
            } else if let Some(s) = text.strip_prefix("0b") {
                (2, s)
            } else {
                (10, text.as_str())
            };
            let Ok(exponent) = u32::from_str_radix(digits, radix) else {
                return fail(name.span, "query!: power exceeds u32");
            };
            Expression::Pow(Box::new(value), exponent.to_string())
        }
        "OnDomain" => {
            let value = expression(&mut args, 0, depth + 1)?;
            expect_punct(&mut args, ',', "a comma")?;
            Expression::OnDomain(
                Box::new(value),
                expect_ident(&mut args, "a declared number_domain import")?,
            )
        }
        _ => return fail(name.span, "query!: unknown numerical operator"),
    };
    end(&mut args)?;
    bounded(value)
}

// Bound each constructed parent, not just recursive parser calls: a long
// left-associative operator chain otherwise builds an unbounded recursive AST
// before emission can refuse it (and then dropping that AST can overflow).
fn bounded(value: Expression) -> Parse<Expression> {
    let mut pending = vec![(&value, 1usize)];
    let mut nodes = 0usize;
    while let Some((node, depth)) = pending.pop() {
        nodes += 1;
        if depth > 128 || nodes > 4096 {
            return fail(
                proc_macro::Span::call_site(),
                "query!: numerical expression exceeds shape budget",
            );
        }
        match node {
            Expression::Binary(_, left, right) => {
                pending.push((left, depth + 1));
                pending.push((right, depth + 1));
            }
            Expression::Unary(_, value)
            | Expression::Pow(value, _)
            | Expression::OnDomain(value, _) => pending.push((value, depth + 1)),
            _ => {}
        }
    }
    Ok(value)
}
impl Expression {
    pub(super) fn emit(&self, scope: &Scope, imports: &[Import], depth: usize) -> Parse<String> {
        if depth > 128 {
            return fail(
                proc_macro::Span::call_site(),
                "query!: numerical expression is too deeply nested",
            );
        }
        let body = match self {
            Self::Variable(name) => format!("Var(::bumbledb::VarId({}))", scope.head_var(name)?),
            Self::Integer(name) => format!("Integer(::bumbledb::VarId({}))", scope.head_var(name)?),
            Self::Component(component, name) => format!(
                "Component {{ observation: ::bumbledb::VarId({}), component: ::bumbledb::ObservationComponent::{component} }}",
                scope.head_var(name)?
            ),
            Self::Literal(value) => format!(
                "Literal(::bumbledb::event::ExactRational::from({}{}{}))",
                if value.negative { "-" } else { "" },
                value.text.trim_end_matches("u64").trim_end_matches("i64"),
                if value.signed { "i64" } else { "u64" }
            ),
            Self::Imported(name) => format!(
                "Imported({})",
                events::imported(name, imports, ImportKind::Number)?
            ),
            Self::Binary(op, left, right) => format!(
                "Binary {{ op: ::bumbledb::event::NumberOp::{op}, left: ::std::boxed::Box::new({}), right: ::std::boxed::Box::new({}) }}",
                left.emit(scope, imports, depth + 1)?,
                right.emit(scope, imports, depth + 1)?
            ),
            Self::Unary(op, value) => format!(
                "{op}(::std::boxed::Box::new({}))",
                value.emit(scope, imports, depth + 1)?
            ),
            Self::Pow(value, exponent) => format!(
                "Pow {{ value: ::std::boxed::Box::new({}), exponent: {exponent}u32 }}",
                value.emit(scope, imports, depth + 1)?
            ),
            Self::OnDomain(value, domain) => format!(
                "OnDomain {{ value: ::std::boxed::Box::new({}), domain: {} }}",
                value.emit(scope, imports, depth + 1)?,
                events::imported(domain, imports, ImportKind::NumberDomain)?
            ),
        };
        Ok(format!("::bumbledb::NumberExpr::{body}"))
    }
}
