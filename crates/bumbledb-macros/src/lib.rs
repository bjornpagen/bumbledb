//! bumbledb's proc macros: `schema!` declares a theory, `query!` writes a
//! query against one, `params!` binds a template's named params. Each parses
//! the raw token stream (the grammars are not Rust syntax) and reports every
//! error as a `compile_error!` at the offending token.
mod lex;
mod query;
mod schema;

use proc_macro::TokenStream;

/// Declares a theory: `pub Name;`, then relations and dependency statements.
///
/// ```text
/// schema! {
///     pub Ledger;
///     closed relation Kind as KindId = { Checking, Savings };
///     relation Holder  { id: uuid as HolderId, name: str }
///     relation Account { id: uuid as AccountId, holder: uuid as HolderId, kind: u64 as KindId }
///     Holder(id) -> Holder;
///     Account(id) -> Account;
///     Account(holder) <= Holder(id);
///     Account(kind) <= Kind(id);
/// }
/// ```
/// Emits the theory unit struct (implementing `Theory`), the newtypes, a
/// host enum per closed relation, a fact struct per ordinary relation, a
/// key struct per declared key, and `Ledger::Account`-style constants whose
/// fields are the relation's `FieldId`s (`Ledger::Account.relation()` is its
/// `RelationId`).
#[proc_macro]
pub fn schema(input: TokenStream) -> TokenStream {
    schema::expand(input.into())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// Lowers the query notation to a typed template over `bumbledb::Query`.
///
/// ```text
/// let reachable = query!(Ledger {
///     rec reach(c, a) | OrgParent(child: c, parent: a);
///     rec reach(c, a) | OrgParent(child: c, parent: m), reach(m, a);
///     (c, a) | reach(c, a);
/// });
/// ```
/// Relations, fields and handles resolve through the theory's constants;
/// semantic checks beyond names happen at `Db::prepare`.
#[proc_macro]
pub fn query(input: TokenStream) -> TokenStream {
    query::expand(input.into())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// Named param values for a template's `bind`:
/// `template.bind(params! { student: id })`.
#[proc_macro]
pub fn params(input: TokenStream) -> TokenStream {
    query::expand_params(input.into())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}
