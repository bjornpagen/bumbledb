//! Exact relational values and sealed host imports share one payoff grammar.
use super::{
    Import, ImportKind, Name, Parse, Scope, TokenTree, Tokens, events, expect_ident, expect_punct,
    fail, take_paren_group,
};

pub(super) enum Payoff {
    Integer(Name),
    Ratio { numerator: Name, denominator: Name },
    Imported(Name),
}
pub(super) fn parse(tokens: &mut Tokens) -> Parse<Payoff> {
    let name = expect_ident(
        tokens,
        "a payoff variable, Ratio(numerator, denominator), or Payoff(import)",
    )?;
    if !matches!(tokens.peek(), Some(TokenTree::Group(_))) {
        return Ok(Payoff::Integer(name));
    }
    match name.text.as_str() {
        "Ratio" => {
            let (mut args, _) = take_paren_group(tokens, "Ratio's numerator and denominator")?;
            let numerator = expect_ident(&mut args, "a numerator variable")?;
            expect_punct(&mut args, ',', "a comma")?;
            let denominator = expect_ident(&mut args, "a denominator variable")?;
            if let Some(extra) = args.next() {
                return fail(
                    extra.span(),
                    "query!: Ratio takes two body-bound integer variables",
                );
            }
            Ok(Payoff::Ratio {
                numerator,
                denominator,
            })
        }
        "Payoff" => {
            let (mut args, _) = take_paren_group(tokens, "Payoff's imported value")?;
            let name = expect_ident(&mut args, "a `use payoff` import")?;
            if let Some(extra) = args.next() {
                return fail(extra.span(), "query!: Payoff takes one declared import");
            }
            Ok(Payoff::Imported(name))
        }
        _ => fail(name.span, "query!: expected Ratio or Payoff"),
    }
}
impl Payoff {
    pub(super) fn emit(&self, scope: &Scope, imports: &[Import]) -> Parse<String> {
        Ok(match self {
            Self::Integer(name) => format!(
                "::bumbledb::PayoffExpr::Integer(::bumbledb::VarId({}))",
                scope.head_var(name)?
            ),
            Self::Ratio {
                numerator,
                denominator,
            } => format!(
                "::bumbledb::PayoffExpr::Ratio {{ numerator: ::bumbledb::VarId({}), denominator: ::bumbledb::VarId({}) }}",
                scope.head_var(numerator)?,
                scope.head_var(denominator)?
            ),
            Self::Imported(name) => format!(
                "::bumbledb::PayoffExpr::Imported({})",
                events::imported(name, imports, ImportKind::Payoff)?
            ),
        })
    }
}
