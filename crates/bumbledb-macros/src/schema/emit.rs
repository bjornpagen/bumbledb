//! `schema!` output: the theory type and its descriptor, newtypes, closed
//! host enums, fact and key structs, and one `Theory::Relation` constant
//! per relation whose fields are the relation's `FieldId`s.
use std::collections::BTreeMap;

use bumbledb_theory::Value;
use bumbledb_theory::schema::{
    Bound, FixedIntervalElement, IntervalElement, LiteralSet, SchemaDescriptor, Side,
    StatementDescriptor, ValueType, Weight,
};
use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::{format_ident, quote};

use super::{Field, FieldTy, Relation, Schema, Statement};
use crate::lex::{Result, f64_tokens, fail, uuid_tokens, value_tokens};

pub(super) fn schema(schema: &Schema, descriptor: &SchemaDescriptor) -> Result<TokenStream> {
    let name = &schema.name;
    let descriptor_tokens = descriptor_tokens(descriptor);
    let doc = format!(
        "The `{name}` schema: the value `Db::create` and `Db::open` take and the \
         typestate `Db<{name}>` carries."
    );
    let newtypes = newtypes(schema)?;
    let closed: TokenStream = schema
        .relations
        .iter()
        .enumerate()
        .filter_map(|(index, relation)| {
            let extension = descriptor.relations[index].extension.as_deref()?;
            Some(host_enum(relation, extension))
        })
        .collect();
    let facts: TokenStream = schema
        .relations
        .iter()
        .enumerate()
        .filter(|(_, relation)| relation.closed.is_none())
        .map(|(index, relation)| fact_struct(name, index, relation))
        .collect();
    let keys = key_structs(schema);
    let relations = relation_consts(schema);
    Ok(quote! {
        #[doc = #doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct #name;
        impl ::bumbledb::Theory for #name {
            fn descriptor(self) -> ::bumbledb::schema::SchemaDescriptor {
                #descriptor_tokens
            }
        }
        #newtypes
        #closed
        #facts
        #keys
        #relations
    })
}

fn index(value: usize) -> Literal {
    Literal::usize_unsuffixed(value)
}

fn element_rust(element: IntervalElement) -> TokenStream {
    match element {
        IntervalElement::U64 => quote!(u64),
        IntervalElement::I64 => quote!(i64),
        IntervalElement::F64 => quote!(::bumbledb::F64),
    }
}

fn fixed_element(element: FixedIntervalElement) -> IntervalElement {
    match element {
        FixedIntervalElement::U64 => IntervalElement::U64,
        FixedIntervalElement::I64 => IntervalElement::I64,
    }
}

fn value_type_tokens(value_type: ValueType) -> TokenStream {
    let element_name = |element: IntervalElement| match element {
        IntervalElement::U64 => quote!(U64),
        IntervalElement::I64 => quote!(I64),
        IntervalElement::F64 => quote!(F64),
    };
    match value_type {
        ValueType::Bool => quote!(::bumbledb::schema::ValueType::Bool),
        ValueType::U64 => quote!(::bumbledb::schema::ValueType::U64),
        ValueType::I64 => quote!(::bumbledb::schema::ValueType::I64),
        ValueType::F64 => quote!(::bumbledb::schema::ValueType::F64),
        ValueType::Uuid => quote!(::bumbledb::schema::ValueType::Uuid),
        ValueType::String => quote!(::bumbledb::schema::ValueType::String),
        ValueType::FixedBytes { len } => {
            quote!(::bumbledb::schema::ValueType::FixedBytes { len: #len })
        }
        ValueType::Interval { element } => {
            let element = element_name(element);
            quote!(::bumbledb::schema::ValueType::Interval {
                element: ::bumbledb::schema::IntervalElement::#element
            })
        }
        ValueType::FixedInterval { element, width } => {
            let element = element_name(fixed_element(element));
            quote!(::bumbledb::schema::ValueType::FixedInterval {
                element: ::bumbledb::schema::FixedIntervalElement::#element,
                width: #width,
            })
        }
    }
}

fn literal_set_tokens(set: &LiteralSet) -> TokenStream {
    match set {
        LiteralSet::One(value) => {
            let value = value_tokens(value);
            quote!(::bumbledb::schema::LiteralSet::One(#value))
        }
        LiteralSet::Many(values) => {
            let values = values.iter().map(value_tokens);
            quote!(::bumbledb::schema::LiteralSet::Many(::std::boxed::Box::new([#(#values),*])))
        }
    }
}

fn field_ids(fields: &[bumbledb_theory::schema::FieldId]) -> TokenStream {
    let fields = fields.iter().map(|field| index(usize::from(field.0)));
    quote!(::std::boxed::Box::new([#(::bumbledb::schema::FieldId(#fields)),*]))
}

fn side_tokens(side: &Side) -> TokenStream {
    let relation = Literal::u32_unsuffixed(side.relation.0);
    let projection = field_ids(&side.projection);
    let selection = side.selection.iter().map(|(field, set)| {
        let field = index(usize::from(field.0));
        let set = literal_set_tokens(set);
        quote!((::bumbledb::schema::FieldId(#field), #set))
    });
    quote!(::bumbledb::schema::Side {
        relation: ::bumbledb::schema::RelationId(#relation),
        projection: #projection,
        selection: ::std::boxed::Box::new([#(#selection),*]),
    })
}

fn statement_tokens(statement: &StatementDescriptor) -> TokenStream {
    match statement {
        StatementDescriptor::Functionality {
            relation,
            projection,
        } => {
            let relation = Literal::u32_unsuffixed(relation.0);
            let projection = field_ids(projection);
            quote!(::bumbledb::schema::StatementDescriptor::Functionality {
                relation: ::bumbledb::schema::RelationId(#relation),
                projection: #projection,
            })
        }
        StatementDescriptor::Containment { source, target } => {
            let (source, target) = (side_tokens(source), side_tokens(target));
            quote!(::bumbledb::schema::StatementDescriptor::Containment {
                source: #source,
                target: #target,
            })
        }
        StatementDescriptor::Capacity {
            target,
            weight,
            lo,
            hi,
            source,
        } => {
            let weight = match weight {
                Weight::Unit => quote!(::bumbledb::schema::Weight::Unit),
                Weight::Field(field) => {
                    let field = index(usize::from(field.0));
                    quote!(::bumbledb::schema::Weight::Field(::bumbledb::schema::FieldId(#field)))
                }
                Weight::DurationOf(field) => {
                    let field = index(usize::from(field.0));
                    quote!(::bumbledb::schema::Weight::DurationOf(
                        ::bumbledb::schema::FieldId(#field)
                    ))
                }
            };
            let hi = match hi {
                None => quote!(::std::option::Option::None),
                Some(bound) => {
                    let bound = bound_tokens(*bound);
                    quote!(::std::option::Option::Some(#bound))
                }
            };
            let (target, source) = (side_tokens(target), side_tokens(source));
            quote!(::bumbledb::schema::StatementDescriptor::Capacity {
                target: #target,
                weight: #weight,
                lo: #lo,
                hi: #hi,
                source: #source,
            })
        }
    }
}

fn bound_tokens(bound: Bound) -> TokenStream {
    match bound {
        Bound::Lit(n) => quote!(::bumbledb::schema::Bound::Lit(#n)),
        Bound::TargetField(field) => {
            let field = index(usize::from(field.0));
            quote!(::bumbledb::schema::Bound::TargetField(::bumbledb::schema::FieldId(#field)))
        }
        Bound::TargetDuration(field) => {
            let field = index(usize::from(field.0));
            quote!(::bumbledb::schema::Bound::TargetDuration(::bumbledb::schema::FieldId(#field)))
        }
    }
}

fn descriptor_tokens(descriptor: &SchemaDescriptor) -> TokenStream {
    let relations = descriptor.relations.iter().map(|relation| {
        let name: &str = &relation.name;
        let fields = relation.fields.iter().map(|field| {
            let name: &str = &field.name;
            let value_type = value_type_tokens(field.value_type);
            quote!(::bumbledb::schema::FieldDescriptor {
                name: ::std::boxed::Box::from(#name),
                value_type: #value_type,
            })
        });
        let extension = match &relation.extension {
            None => quote!(::std::option::Option::None),
            Some(rows) => {
                let rows = rows.iter().map(|row| {
                    let handle: &str = &row.handle;
                    let values = row.values.iter().map(value_tokens);
                    quote!(::bumbledb::schema::Row {
                        handle: ::std::boxed::Box::from(#handle),
                        values: ::std::boxed::Box::new([#(#values),*]),
                    })
                });
                quote!(::std::option::Option::Some(::std::boxed::Box::new([#(#rows),*])))
            }
        };
        quote!(::bumbledb::schema::RelationDescriptor {
            name: ::std::boxed::Box::from(#name),
            fields: ::std::vec![#(#fields),*],
            extension: #extension,
        })
    });
    let statements = descriptor.statements.iter().map(statement_tokens);
    quote!(::bumbledb::schema::SchemaDescriptor {
        relations: ::std::vec![#(#relations),*],
        statements: ::std::vec![#(#statements),*],
    })
}

/// A field's host representation: `(encoding, inner type, ordered)`.
fn newtype_inner(ty: FieldTy) -> (String, TokenStream, bool) {
    match ty {
        FieldTy::U64 => ("u64".into(), quote!(u64), true),
        FieldTy::I64 => ("i64".into(), quote!(i64), true),
        FieldTy::F64 => ("f64".into(), quote!(::bumbledb::F64), true),
        FieldTy::Uuid => ("uuid".into(), quote!(::bumbledb::Uuid), false),
        FieldTy::FixedBytes(len) => {
            let len = index(usize::from(len));
            (format!("bytes<{len}>"), quote!([u8; #len]), false)
        }
        FieldTy::Interval(element) => {
            let rust = element_rust(element);
            (
                format!("interval<{element:?}>"),
                quote!(::bumbledb::Interval<#rust>),
                false,
            )
        }
        FieldTy::FixedInterval(element, width) => {
            let rust = element_rust(fixed_element(element));
            (
                format!("interval<{element:?}, {width}>"),
                quote!(::bumbledb::Interval<#rust>),
                false,
            )
        }
        FieldTy::Bool | FieldTy::Str => unreachable!("the parse refuses `as` on bool and str"),
    }
}

fn newtypes(schema: &Schema) -> Result<TokenStream> {
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    let mut out = TokenStream::new();
    let declared = schema.relations.iter().flat_map(|relation| {
        let handle = relation
            .closed
            .as_ref()
            .map(|closed| (&closed.newtype, FieldTy::U64));
        handle.into_iter().chain(
            relation
                .fields
                .iter()
                .filter_map(|field| Some((field.newtype.as_ref()?, field.ty))),
        )
    });
    for (name, ty) in declared {
        let (encoding, inner, ordered) = newtype_inner(ty);
        if let Some(existing) = seen.get(&name.to_string()) {
            if *existing != encoding {
                return fail(
                    name.span(),
                    format!(
                        "schema!: newtype `{name}` is declared over two encodings: \
                         {existing} and {encoding}"
                    ),
                );
            }
            continue;
        }
        seen.insert(name.to_string(), encoding);
        let order = ordered.then(|| quote!(, PartialOrd, Ord));
        out.extend(quote! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash #order)]
            pub struct #name(pub #inner);
        });
    }
    Ok(out)
}

fn rust_field_ty(field: &Field, lifetime: &TokenStream) -> TokenStream {
    if let Some(newtype) = &field.newtype {
        return quote!(#newtype);
    }
    match field.ty {
        FieldTy::Bool => quote!(bool),
        FieldTy::U64 => quote!(u64),
        FieldTy::I64 => quote!(i64),
        FieldTy::F64 => quote!(::bumbledb::F64),
        FieldTy::Uuid => quote!(::bumbledb::Uuid),
        FieldTy::Str => quote!(&#lifetime str),
        FieldTy::FixedBytes(len) => {
            let len = index(usize::from(len));
            quote!([u8; #len])
        }
        FieldTy::Interval(element) => {
            let rust = element_rust(element);
            quote!(::bumbledb::Interval<#rust>)
        }
        FieldTy::FixedInterval(element, _) => {
            let rust = element_rust(fixed_element(element));
            quote!(::bumbledb::Interval<#rust>)
        }
    }
}

fn const_value_tokens(value: &Value, field: &Field) -> TokenStream {
    let raw = match value {
        Value::Bool(v) => quote!(#v),
        Value::U64(v) => quote!(#v),
        Value::I64(v) => quote!(#v),
        Value::F64(v) => f64_tokens(*v),
        Value::String(text) => {
            let text: &str = text;
            quote!(#text)
        }
        Value::Uuid(id) => uuid_tokens(id.as_bytes()),
        Value::FixedBytes(bytes) => {
            let bytes = Literal::byte_string(bytes);
            quote!(*#bytes)
        }
        Value::IntervalU64(interval) => {
            let (start, end) = interval.bounds();
            quote!(::bumbledb::Interval::<u64>::const_new(#start, #end).expect("a nonempty interval"))
        }
        Value::IntervalI64(interval) => {
            let (start, end) = interval.bounds();
            quote!(::bumbledb::Interval::<i64>::const_new(#start, #end).expect("a nonempty interval"))
        }
        Value::IntervalF64(_) => unreachable!("the parse refuses interval<f64> closed columns"),
    };
    match &field.newtype {
        Some(newtype) => quote!(#newtype(#raw)),
        None => raw,
    }
}

fn host_enum(relation: &Relation, extension: &[bumbledb_theory::schema::Row]) -> TokenStream {
    let Some(closed) = &relation.closed else {
        return TokenStream::new();
    };
    let name = &relation.name;
    let newtype = &closed.newtype;
    let handles: Vec<&Ident> = closed.rows.iter().map(|row| &row.handle).collect();
    let ids: Vec<Literal> = (0..handles.len()).map(index).collect();
    let static_lifetime = quote!('static);
    let accessors = relation.fields.iter().enumerate().map(|(column, field)| {
        let column_name = &field.name;
        let ty = rust_field_ty(field, &static_lifetime);
        let values = extension
            .iter()
            .map(|row| const_value_tokens(&row.values[column], field));
        let doc = format!("The `{column_name}` column of each handle.");
        quote! {
            #[doc = #doc]
            #[must_use]
            pub const fn #column_name(self) -> #ty {
                match self { #(Self::#handles => #values),* }
            }
        }
    });
    let doc = format!(
        "The handles of the closed relation `{name}`, in declaration order; \
         [`{name}::id`] and [`{name}::from_id`] map them to row ids."
    );
    // Every handle is a stored row, so a variant the host never constructs
    // is not dead code.
    quote! {
        #[doc = #doc]
        #[allow(dead_code)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum #name { #(#handles),* }
        impl #name {
            /// The handle's declaration-order row id.
            #[must_use]
            pub const fn id(self) -> #newtype {
                match self { #(Self::#handles => #newtype(#ids)),* }
            }
            /// The handle a row id names; `None` past the extension.
            #[must_use]
            pub const fn from_id(id: #newtype) -> ::core::option::Option<Self> {
                match id.0 {
                    #(#ids => ::core::option::Option::Some(Self::#handles),)*
                    _ => ::core::option::Option::None,
                }
            }
            #(#accessors)*
        }
    }
}

/// One field's push onto `out` in sealed order.
fn append_value(field: &Field, index: usize, relation: &TokenStream, out: &Ident) -> TokenStream {
    let name = &field.name;
    let access = if field.newtype.is_some() {
        quote!(self.#name.0)
    } else {
        quote!(self.#name)
    };
    let value = match field.ty {
        FieldTy::Bool => quote!(::bumbledb::Value::Bool(#access)),
        FieldTy::U64 => quote!(::bumbledb::Value::U64(#access)),
        FieldTy::I64 => quote!(::bumbledb::Value::I64(#access)),
        FieldTy::F64 => quote!(::bumbledb::Value::F64(#access)),
        FieldTy::Uuid => quote!(::bumbledb::Value::Uuid(#access)),
        FieldTy::Str => quote!(::bumbledb::Value::String(::std::boxed::Box::from(#access))),
        FieldTy::FixedBytes(_) => {
            quote!(::bumbledb::Value::FixedBytes(::std::boxed::Box::from(&#access[..])))
        }
        FieldTy::Interval(IntervalElement::U64) => quote!(::bumbledb::Value::IntervalU64(#access)),
        FieldTy::Interval(IntervalElement::I64) => quote!(::bumbledb::Value::IntervalI64(#access)),
        FieldTy::Interval(IntervalElement::F64) => quote!(::bumbledb::Value::IntervalF64(#access)),
        FieldTy::FixedInterval(element, width) => {
            let check = match element {
                FixedIntervalElement::U64 => quote!(::bumbledb::__private::fixed_interval_u64),
                FixedIntervalElement::I64 => quote!(::bumbledb::__private::fixed_interval_i64),
            };
            let index = Literal::usize_unsuffixed(index);
            quote!(#check(#relation, ::bumbledb::schema::FieldId(#index), #access, #width)?)
        }
    };
    quote!(#out.push(#value);)
}

/// One field's `let` from the row reader, in sealed order.
fn decode_field(field: &Field, row: &Ident) -> TokenStream {
    let name = &field.name;
    let read = match field.ty {
        FieldTy::Bool => quote!(#row.next_bool()?),
        FieldTy::U64 => quote!(#row.next_u64()?),
        FieldTy::I64 => quote!(#row.next_i64()?),
        FieldTy::F64 => quote!(#row.next_f64()?),
        FieldTy::Uuid => quote!(#row.next_uuid()?),
        FieldTy::Str => quote!(#row.next_str()?),
        FieldTy::Interval(IntervalElement::U64)
        | FieldTy::FixedInterval(FixedIntervalElement::U64, _) => {
            quote!(#row.next_interval_u64()?)
        }
        FieldTy::Interval(IntervalElement::I64)
        | FieldTy::FixedInterval(FixedIntervalElement::I64, _) => {
            quote!(#row.next_interval_i64()?)
        }
        FieldTy::Interval(IntervalElement::F64) => quote!(#row.next_interval_f64()?),
        FieldTy::FixedBytes(len) => {
            let len = index(usize::from(len));
            quote!(#row.next_fixed_bytes::<#len>()?)
        }
    };
    let value = field
        .newtype
        .as_ref()
        .map_or_else(|| read.clone(), |newtype| quote!(#newtype(#read)));
    quote!(let #name = #value;)
}

fn borrows(fields: &[&Field]) -> bool {
    fields.iter().any(|field| matches!(field.ty, FieldTy::Str))
}

fn fact_struct(schema: &Ident, index: usize, relation: &Relation) -> TokenStream {
    let name = &relation.name;
    let fields: Vec<&Field> = relation.fields.iter().collect();
    let lifetime = quote!('a);
    let (generics, self_ty) = if borrows(&fields) {
        (quote!(<'a>), quote!(#name<'a>))
    } else {
        (TokenStream::new(), quote!(#name))
    };
    let names = fields.iter().map(|field| &field.name);
    let types = fields.iter().map(|field| rust_field_ty(field, &lifetime));
    let out = Ident::new(
        if fields.is_empty() { "_out" } else { "__out" },
        Span::call_site(),
    );
    let row = Ident::new("__row", Span::call_site());
    let relation_expr = quote!(<Self as ::bumbledb::Fact<'a>>::RELATION);
    let appends = fields
        .iter()
        .enumerate()
        .map(|(index, field)| append_value(field, index, &relation_expr, &out));
    let decodes = fields.iter().map(|field| decode_field(field, &row));
    let field_names = fields.iter().map(|field| &field.name);
    let index = self::index(index);
    quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct #name #generics { #(pub #names: #types),* }
        impl<'a> ::bumbledb::Fact<'a> for #self_ty {
            type Schema = #schema;
            const RELATION: ::bumbledb::schema::RelationId =
                ::bumbledb::schema::RelationId(#index);
            fn append_values(
                &self,
                #out: &mut ::std::vec::Vec<::bumbledb::Value>,
            ) -> ::bumbledb::Result<()> {
                #(#appends)*
                ::std::result::Result::Ok(())
            }
            fn decode(mut #row: ::bumbledb::RowReader<'a>) -> ::bumbledb::Result<Self> {
                #(#decodes)*
                #row.finish()?;
                ::std::result::Result::Ok(Self { #(#field_names),* })
            }
        }
    }
}

fn pascal(name: &str) -> String {
    name.split('_')
        .filter_map(|segment| {
            let mut chars = segment.chars();
            let first = chars.next()?;
            Some(first.to_ascii_uppercase().to_string() + chars.as_str())
        })
        .collect()
}

/// Statement ids: closed relations' handle keys first, in declaration
/// order, then declared statements, `==` counting as two.
fn key_structs(schema: &Schema) -> TokenStream {
    let mut next = schema
        .relations
        .iter()
        .filter(|relation| relation.closed.is_some())
        .count();
    let mut out = TokenStream::new();
    for statement in &schema.statements {
        let id = next;
        next += match statement {
            Statement::Containment {
                bidirectional: true,
                ..
            } => 2,
            _ => 1,
        };
        let Statement::Key {
            relation,
            projection,
        } = statement
        else {
            continue;
        };
        let Some((rel_idx, rel)) = schema
            .relations
            .iter()
            .enumerate()
            .find(|(_, r)| r.name == *relation)
        else {
            continue;
        };
        if rel.closed.is_none() {
            out.extend(key_struct(&schema.name, rel_idx, rel, projection, id));
        }
    }
    out
}

fn key_struct(
    schema: &Ident,
    rel_idx: usize,
    relation: &Relation,
    projection: &[Ident],
    statement: usize,
) -> TokenStream {
    let rel_name = &relation.name;
    let fields: Vec<(usize, &Field)> = projection
        .iter()
        .filter_map(|name| {
            relation
                .fields
                .iter()
                .enumerate()
                .find(|(_, field)| field.name == *name)
        })
        .collect();
    let key_name = format_ident!(
        "{rel_name}By{}",
        projection
            .iter()
            .map(|name| pascal(&name.to_string()))
            .collect::<String>()
    );
    let key_fields: Vec<&Field> = fields.iter().map(|(_, field)| *field).collect();
    let all_fields: Vec<&Field> = relation.fields.iter().collect();
    let lifetime = quote!('a);
    let (generics, impl_generics, impl_ty) = if borrows(&key_fields) {
        (quote!(<'a>), quote!(<'a, 'k>), quote!(#key_name<'k>))
    } else {
        (TokenStream::new(), quote!(<'a>), quote!(#key_name))
    };
    let fact_ty = if borrows(&all_fields) {
        quote!(#rel_name<'a>)
    } else {
        quote!(#rel_name)
    };
    let names = key_fields.iter().map(|field| &field.name);
    let types = key_fields
        .iter()
        .map(|field| rust_field_ty(field, &lifetime));
    let out = Ident::new(
        if fields.is_empty() { "_out" } else { "__out" },
        Span::call_site(),
    );
    let rel_index = index(rel_idx);
    let relation_expr = quote!(::bumbledb::schema::RelationId(#rel_index));
    let appends = fields
        .iter()
        .map(|(index, field)| append_value(field, *index, &relation_expr, &out));
    let spelling = format!(
        "The key of `{rel_name}({}) -> {rel_name}`: `get` with it returns `Option<{rel_name}>`.",
        projection
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    );
    let statement = index(statement);
    quote! {
        #[doc = #spelling]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct #key_name #generics { #(pub #names: #types),* }
        impl #impl_generics ::bumbledb::Key<'a> for #impl_ty {
            type Schema = #schema;
            type Fact = #fact_ty;
            const STATEMENT: ::bumbledb::schema::StatementId =
                ::bumbledb::schema::StatementId(#statement);
            fn append_key_values(
                &self,
                #out: &mut ::std::vec::Vec<::bumbledb::Value>,
            ) -> ::bumbledb::Result<()> {
                #(#appends)*
                ::std::result::Result::Ok(())
            }
        }
    }
}

/// `Theory::Relation`: one constant per relation, typed by a struct whose
/// fields are the relation's sealed `FieldId`s. `query!` resolves every
/// relation, field and handle through these constants. The structs live in
/// an anonymous block; their names extend the theory's, so none can shadow it.
fn relation_consts<'s>(schema: &'s Schema) -> TokenStream {
    let theory = &schema.name;
    let not_closed = format_ident!("{theory}NotAClosedReference");
    let extension = |relation: &'s Relation| {
        let closed = relation.closed.as_ref()?;
        let ty = format_ident!("{theory}{}Extension", relation.name);
        let handles: Vec<&Ident> = closed.rows.iter().map(|row| &row.handle).collect();
        let ids = (0..handles.len()).map(index);
        let value = quote!(#ty { #(#handles: #ids),* });
        let def = quote! {
            #[doc(hidden)]
            #[allow(non_snake_case)]
            #[derive(Clone, Copy)]
            pub struct #ty { #(pub #handles: u64),* }
        };
        Some((&closed.newtype, quote!(#ty), value, def))
    };
    let extensions: Vec<_> = schema.relations.iter().filter_map(extension).collect();
    let reference = |newtype: Option<&Ident>| {
        newtype
            .and_then(|newtype| extensions.iter().find(|(handle, ..)| *handle == newtype))
            .map_or_else(
                || (quote!(#not_closed), quote!(#not_closed)),
                |(_, ty, value, _)| (ty.clone(), value.clone()),
            )
    };
    let mut types: TokenStream = extensions.iter().map(|(.., def)| def.clone()).collect();
    let mut consts = TokenStream::new();
    for (rel_idx, relation) in schema.relations.iter().enumerate() {
        let name = &relation.name;
        let relation_ty = format_ident!("{theory}{name}Relation");
        let references_ty = format_ident!("{theory}{name}References");
        let synthetic = relation
            .closed
            .as_ref()
            .map(|closed| (Ident::new("id", name.span()), Some(&closed.newtype)));
        let sealed: Vec<(Ident, Option<&Ident>)> = synthetic
            .into_iter()
            .chain(
                relation
                    .fields
                    .iter()
                    .map(|field| (field.name.clone(), field.newtype.as_ref())),
            )
            .collect();
        let fields: Vec<&Ident> = sealed.iter().map(|(field, _)| field).collect();
        let ids: Vec<TokenStream> = (0..sealed.len())
            .map(|id| {
                let id = index(id);
                quote!(::bumbledb::schema::FieldId(#id))
            })
            .collect();
        let docs = sealed
            .iter()
            .map(|(field, _)| format!("The `FieldId` of `{name}.{field}`."));
        let (reference_types, reference_values): (Vec<_>, Vec<_>) = sealed
            .iter()
            .map(|(_, newtype)| reference(*newtype))
            .unzip();
        let (extension_ty, extension_value) =
            reference(relation.closed.as_ref().map(|closed| &closed.newtype));
        let rel_index = index(rel_idx);
        let doc = format!("The field ids of `{name}`; `.relation()` is its `RelationId`.");
        let const_doc = format!("The relation `{name}`.");
        types.extend(quote! {
            #[doc = #doc]
            #[derive(Clone, Copy)]
            pub struct #relation_ty {
                #(#[doc = #docs] pub #fields: ::bumbledb::schema::FieldId),*
            }
            #[doc(hidden)]
            #[derive(Clone, Copy)]
            pub struct #references_ty { #(pub #fields: #reference_types),* }
            impl #relation_ty {
                /// The relation's declaration-order id.
                #[must_use]
                pub const fn relation(self) -> ::bumbledb::schema::RelationId {
                    ::bumbledb::schema::RelationId(#rel_index)
                }
                #[doc(hidden)]
                #[must_use]
                pub const fn __references(self) -> #references_ty {
                    #references_ty { #(#fields: #reference_values),* }
                }
                #[doc(hidden)]
                #[must_use]
                pub const fn __extension(self) -> #extension_ty {
                    #extension_value
                }
            }
        });
        consts.extend(quote! {
            #[doc = #const_doc]
            pub const #name: #relation_ty = #relation_ty { #(#fields: #ids),* };
        });
    }
    quote! {
        const _: () = {
            #[doc(hidden)]
            #[derive(Clone, Copy)]
            pub struct #not_closed;
            #types
            #[allow(non_upper_case_globals)]
            impl #theory {
                #consts
            }
        };
    }
}
