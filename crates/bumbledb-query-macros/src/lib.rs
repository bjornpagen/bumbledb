//! `query!` and `params!`, compiled from `bumbledb-macros`' sources until
//! `bumbledb` re-exports them from there; then this crate is deleted.
#![allow(dead_code)]
#[path = "../../bumbledb-macros/src/lex.rs"]
mod lex;
#[path = "../../bumbledb-macros/src/query/mod.rs"]
mod query;

use proc_macro::TokenStream;

#[proc_macro]
pub fn query(input: TokenStream) -> TokenStream {
    query::expand(input.into())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

#[proc_macro]
pub fn params(input: TokenStream) -> TokenStream {
    query::expand_params(input.into())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}
