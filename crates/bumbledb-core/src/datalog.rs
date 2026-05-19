//! Named-field Datalog parser and schema typechecker.

use std::collections::BTreeMap;

pub use crate::query_ir::{
    AggregateFunction, ComparisonOperator, Literal, TypedClause, TypedComparison,
    TypedFieldBinding, TypedFindTerm, TypedInput, TypedLiteral, TypedOperand, TypedQuery,
    TypedRelationAtom, TypedTerm, TypedVariable,
};
use crate::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};

/// Result type for Datalog frontend operations.
pub type Result<T> = std::result::Result<T, DatalogError>;

/// Source span in byte offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    /// Inclusive start byte offset.
    pub start: usize,
    /// Exclusive end byte offset.
    pub end: usize,
}

impl Span {
    fn join(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// Datalog parse/typecheck error.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DatalogError {
    /// Syntax error.
    #[error("parse error at {span:?}: {message}")]
    Parse { span: Span, message: String },

    /// Explicitly unsupported syntax or semantic feature.
    #[error("unsupported feature at {span:?}: {feature}")]
    UnsupportedFeature { span: Span, feature: String },

    /// Unknown relation name.
    #[error("unknown relation {relation} at {span:?}")]
    UnknownRelation { span: Span, relation: String },

    /// Unknown field name.
    #[error("unknown field {relation}.{field} at {span:?}")]
    UnknownField {
        span: Span,
        relation: String,
        field: String,
    },

    /// Variable type conflict.
    #[error("variable {variable} has incompatible types {existing} and {incoming} at {span:?}")]
    VariableTypeConflict {
        span: Span,
        variable: String,
        existing: String,
        incoming: String,
    },

    /// Input parameter type conflict.
    #[error("input {input} has incompatible types {existing} and {incoming} at {span:?}")]
    InputTypeConflict {
        span: Span,
        input: String,
        existing: String,
        incoming: String,
    },

    /// Literal cannot be used where a type is expected.
    #[error("literal at {span:?} is incompatible with expected type {expected}")]
    LiteralTypeMismatch { span: Span, expected: String },

    /// Projection references an unbound variable.
    #[error("projection variable {variable} is unbound at {span:?}")]
    UnboundProjectionVariable { span: Span, variable: String },

    /// Comparison references an unbound operand.
    #[error("comparison operand is unbound at {span:?}")]
    UnboundComparisonOperand { span: Span },

    /// Aggregate argument is invalid.
    #[error("aggregate {function} cannot be applied to type {value_type} at {span:?}")]
    InvalidAggregateType {
        span: Span,
        function: AggregateFunction,
        value_type: String,
    },

    /// Aggregate references an unbound variable.
    #[error("aggregate variable {variable} is unbound at {span:?}")]
    UnboundAggregateVariable { span: Span, variable: String },
}

/// Parses and typechecks a Datalog query against `schema`.
pub fn parse_and_typecheck(schema: &SchemaDescriptor, source: &str) -> Result<TypedQuery> {
    let parsed = parse(source)?;
    typecheck(schema, parsed)
}

/// Parses a Datalog query into an untyped AST.
pub fn parse(source: &str) -> Result<ParsedQuery> {
    reject_source_level_unsupported(source)?;
    Parser::new(source)?.parse_query()
}

/// Typechecks an untyped query against a schema descriptor.
pub fn typecheck(schema: &SchemaDescriptor, query: ParsedQuery) -> Result<TypedQuery> {
    Typechecker::new(schema, query).finish()
}

/// Untyped parsed query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedQuery {
    /// Projection terms from `find`.
    pub find: Vec<ParsedFindTerm>,
    /// Clauses from `where`.
    pub clauses: Vec<ParsedClause>,
    /// Whole-query span.
    pub span: Span,
}

/// Untyped projection term.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParsedFindTerm {
    /// Variable projection.
    Variable { name: String, span: Span },
    /// Aggregate projection.
    Aggregate {
        function: AggregateFunction,
        variable: String,
        span: Span,
    },
}

/// Untyped where clause.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParsedClause {
    /// Relation atom.
    Relation(ParsedRelationAtom),
    /// Comparison predicate.
    Comparison(ParsedComparison),
}

/// Untyped relation atom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedRelationAtom {
    /// Relation name.
    pub relation: String,
    /// Named field bindings.
    pub fields: Vec<ParsedFieldBinding>,
    /// Source span.
    pub span: Span,
}

/// Untyped field binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedFieldBinding {
    /// Field name.
    pub field: String,
    /// Bound term.
    pub term: ParsedTerm,
    /// Source span.
    pub span: Span,
}

/// Untyped comparison.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedComparison {
    /// Left operand.
    pub left: ParsedOperand,
    /// Operator.
    pub operator: ComparisonOperator,
    /// Right operand.
    pub right: ParsedOperand,
    /// Source span.
    pub span: Span,
}

/// Relation atom term.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParsedTerm {
    /// Variable.
    Variable { name: String, span: Span },
    /// Input parameter.
    Input { name: String, span: Span },
    /// Wildcard.
    Wildcard { span: Span },
    /// Literal.
    Literal { literal: Literal, span: Span },
}

impl ParsedTerm {
    fn span(&self) -> Span {
        match self {
            ParsedTerm::Variable { span, .. }
            | ParsedTerm::Input { span, .. }
            | ParsedTerm::Wildcard { span }
            | ParsedTerm::Literal { span, .. } => *span,
        }
    }
}

/// Comparison operand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParsedOperand {
    /// Variable.
    Variable { name: String, span: Span },
    /// Input parameter.
    Input { name: String, span: Span },
    /// Literal.
    Literal { literal: Literal, span: Span },
}

impl ParsedOperand {
    fn span(&self) -> Span {
        match self {
            ParsedOperand::Variable { span, .. }
            | ParsedOperand::Input { span, .. }
            | ParsedOperand::Literal { span, .. } => *span,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TokenKind {
    Ident(String),
    Variable(String),
    Input(String),
    Number(i128),
    String(String),
    LParen,
    RParen,
    Colon,
    Comma,
    Eq,
    NotEq,
    Lt,
    Lte,
    Gt,
    Gte,
    Eof,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Token {
    kind: TokenKind,
    span: Span,
}

struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
        }
    }

    fn tokenize(mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            let done = matches!(token.kind, TokenKind::Eof);
            tokens.push(token);
            if done {
                return Ok(tokens);
            }
        }
    }

    fn next_token(&mut self) -> Result<Token> {
        self.skip_ws();
        let start = self.pos;
        let Some(byte) = self.peek() else {
            return Ok(Token {
                kind: TokenKind::Eof,
                span: Span { start, end: start },
            });
        };

        match byte {
            b'(' => {
                self.pos += 1;
                Ok(self.token(TokenKind::LParen, start))
            }
            b')' => {
                self.pos += 1;
                Ok(self.token(TokenKind::RParen, start))
            }
            b':' => {
                self.pos += 1;
                Ok(self.token(TokenKind::Colon, start))
            }
            b',' => {
                self.pos += 1;
                Ok(self.token(TokenKind::Comma, start))
            }
            b'=' => {
                self.pos += 1;
                Ok(self.token(TokenKind::Eq, start))
            }
            b'!' if self.peek_next() == Some(b'=') => {
                self.pos += 2;
                Ok(self.token(TokenKind::NotEq, start))
            }
            b'<' if self.peek_next() == Some(b'=') => {
                self.pos += 2;
                Ok(self.token(TokenKind::Lte, start))
            }
            b'>' if self.peek_next() == Some(b'=') => {
                self.pos += 2;
                Ok(self.token(TokenKind::Gte, start))
            }
            b'<' => {
                self.pos += 1;
                Ok(self.token(TokenKind::Lt, start))
            }
            b'>' => {
                self.pos += 1;
                Ok(self.token(TokenKind::Gt, start))
            }
            b'?' => self.variable(start),
            b'$' => self.input(start),
            b'"' => self.string(start),
            b'-' | b'0'..=b'9' => self.number(start),
            _ if is_ident_start(byte) => self.ident(start),
            _ => Err(DatalogError::Parse {
                span: Span {
                    start,
                    end: start + 1,
                },
                message: format!("unexpected character {:?}", byte as char),
            }),
        }
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn variable(&mut self, start: usize) -> Result<Token> {
        self.pos += 1;
        let name_start = self.pos;
        self.consume_ident_tail();
        if self.pos == name_start {
            return Err(DatalogError::Parse {
                span: Span {
                    start,
                    end: self.pos,
                },
                message: "expected variable name after ?".to_owned(),
            });
        }
        Ok(self.token(
            TokenKind::Variable(self.source[name_start..self.pos].to_owned()),
            start,
        ))
    }

    fn input(&mut self, start: usize) -> Result<Token> {
        self.pos += 1;
        let name_start = self.pos;
        self.consume_ident_tail();
        if self.pos == name_start {
            return Err(DatalogError::Parse {
                span: Span {
                    start,
                    end: self.pos,
                },
                message: "expected input name after $".to_owned(),
            });
        }
        Ok(self.token(
            TokenKind::Input(self.source[name_start..self.pos].to_owned()),
            start,
        ))
    }

    fn ident(&mut self, start: usize) -> Result<Token> {
        self.pos += 1;
        self.consume_ident_tail();
        Ok(self.token(
            TokenKind::Ident(self.source[start..self.pos].to_owned()),
            start,
        ))
    }

    fn number(&mut self, start: usize) -> Result<Token> {
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        let digit_start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        if self.pos == digit_start {
            return Err(DatalogError::Parse {
                span: Span {
                    start,
                    end: self.pos,
                },
                message: "expected digits after -".to_owned(),
            });
        }
        let text = &self.source[start..self.pos];
        let value = text.parse::<i128>().map_err(|_| DatalogError::Parse {
            span: Span {
                start,
                end: self.pos,
            },
            message: "integer literal is out of range".to_owned(),
        })?;
        Ok(self.token(TokenKind::Number(value), start))
    }

    fn string(&mut self, start: usize) -> Result<Token> {
        self.pos += 1;
        let mut out = String::new();
        while let Some(byte) = self.peek() {
            match byte {
                b'"' => {
                    self.pos += 1;
                    return Ok(self.token(TokenKind::String(out), start));
                }
                b'\\' => {
                    self.pos += 1;
                    let Some(escaped) = self.source[self.pos..].chars().next() else {
                        break;
                    };
                    self.pos += escaped.len_utf8();
                    match escaped {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        other => out.push(other),
                    }
                }
                _ => {
                    let Some(ch) = self.source[self.pos..].chars().next() else {
                        break;
                    };
                    out.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
        Err(DatalogError::Parse {
            span: Span {
                start,
                end: self.pos,
            },
            message: "unterminated string literal".to_owned(),
        })
    }

    fn consume_ident_tail(&mut self) {
        while matches!(self.peek(), Some(byte) if is_ident_tail(byte)) {
            self.pos += 1;
        }
    }

    fn token(&self, kind: TokenKind, start: usize) -> Token {
        Token {
            kind,
            span: Span {
                start,
                end: self.pos,
            },
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<u8> {
        self.bytes.get(self.pos + 1).copied()
    }
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_tail(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'/' | b'-')
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(source: &str) -> Result<Self> {
        Ok(Self {
            tokens: Lexer::new(source).tokenize()?,
            pos: 0,
        })
    }

    fn parse_query(mut self) -> Result<ParsedQuery> {
        let start = self.expect_keyword("find")?.span;
        let mut find = Vec::new();
        while !self.check_keyword("where") {
            if self.is_eof() {
                return Err(self.parse_error("expected where clause"));
            }
            find.push(self.parse_find_term()?);
        }
        self.expect_keyword("where")?;

        let mut clauses = Vec::new();
        while !self.is_eof() {
            clauses.push(self.parse_clause()?);
        }

        let end = self.previous_span().unwrap_or(start);
        Ok(ParsedQuery {
            find,
            clauses,
            span: start.join(end),
        })
    }

    fn parse_find_term(&mut self) -> Result<ParsedFindTerm> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Variable(name) => Ok(ParsedFindTerm::Variable {
                name,
                span: token.span,
            }),
            TokenKind::Ident(name) => {
                let Some(function) = parse_aggregate_function(&name) else {
                    return Err(DatalogError::Parse {
                        span: token.span,
                        message: "expected projection variable or aggregate".to_owned(),
                    });
                };
                self.expect_kind(TokenShape::LParen)?;
                let variable = self.expect_variable()?;
                let end = self.expect_kind(TokenShape::RParen)?.span;
                let span = token.span.join(end);
                Ok(ParsedFindTerm::Aggregate {
                    function,
                    variable: variable.0,
                    span,
                })
            }
            _ => Err(DatalogError::Parse {
                span: token.span,
                message: "expected projection variable or aggregate".to_owned(),
            }),
        }
    }

    fn parse_clause(&mut self) -> Result<ParsedClause> {
        self.reject_clause_unsupported()?;
        match self.peek_kind() {
            TokenKind::Ident(_) => Ok(ParsedClause::Relation(self.parse_relation_atom()?)),
            TokenKind::Variable(_)
            | TokenKind::Input(_)
            | TokenKind::Number(_)
            | TokenKind::String(_) => Ok(ParsedClause::Comparison(self.parse_comparison()?)),
            _ => Err(self.parse_error("expected relation atom or comparison")),
        }
    }

    fn parse_relation_atom(&mut self) -> Result<ParsedRelationAtom> {
        let relation = self.expect_ident()?;
        let start = relation.1;
        self.expect_kind(TokenShape::LParen)?;
        let mut fields = Vec::new();
        if self.check_shape(TokenShape::RParen) {
            let end = self.advance().span;
            return Ok(ParsedRelationAtom {
                relation: relation.0,
                fields,
                span: start.join(end),
            });
        }

        loop {
            if !matches!(self.peek_kind(), TokenKind::Ident(_)) {
                return Err(DatalogError::UnsupportedFeature {
                    span: self.peek().span,
                    feature: "positional relation syntax".to_owned(),
                });
            }
            let field = self.expect_ident()?;
            self.expect_kind(TokenShape::Colon)?;
            let term = self.parse_term()?;
            let field_span = field.1.join(term.span());
            fields.push(ParsedFieldBinding {
                field: field.0,
                term,
                span: field_span,
            });

            if self.check_shape(TokenShape::Comma) {
                self.advance();
                continue;
            }
            let end = self.expect_kind(TokenShape::RParen)?.span;
            return Ok(ParsedRelationAtom {
                relation: relation.0,
                fields,
                span: start.join(end),
            });
        }
    }

    fn parse_comparison(&mut self) -> Result<ParsedComparison> {
        let left = self.parse_operand()?;
        let operator = self.parse_comparison_operator()?;
        let right = self.parse_operand()?;
        let span = left.span().join(right.span());
        Ok(ParsedComparison {
            left,
            operator,
            right,
            span,
        })
    }

    fn parse_term(&mut self) -> Result<ParsedTerm> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Variable(name) => Ok(ParsedTerm::Variable {
                name,
                span: token.span,
            }),
            TokenKind::Input(name) => Ok(ParsedTerm::Input {
                name,
                span: token.span,
            }),
            TokenKind::Ident(name) if name == "_" => Ok(ParsedTerm::Wildcard { span: token.span }),
            TokenKind::Ident(name) if name == "true" => Ok(ParsedTerm::Literal {
                literal: Literal::Bool(true),
                span: token.span,
            }),
            TokenKind::Ident(name) if name == "false" => Ok(ParsedTerm::Literal {
                literal: Literal::Bool(false),
                span: token.span,
            }),
            TokenKind::Number(value) => Ok(ParsedTerm::Literal {
                literal: Literal::Integer(value),
                span: token.span,
            }),
            TokenKind::String(value) => Ok(ParsedTerm::Literal {
                literal: Literal::String(value),
                span: token.span,
            }),
            _ => Err(DatalogError::Parse {
                span: token.span,
                message: "expected variable, input, wildcard, or literal".to_owned(),
            }),
        }
    }

    fn parse_operand(&mut self) -> Result<ParsedOperand> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Variable(name) => Ok(ParsedOperand::Variable {
                name,
                span: token.span,
            }),
            TokenKind::Input(name) => Ok(ParsedOperand::Input {
                name,
                span: token.span,
            }),
            TokenKind::Ident(name) if name == "true" => Ok(ParsedOperand::Literal {
                literal: Literal::Bool(true),
                span: token.span,
            }),
            TokenKind::Ident(name) if name == "false" => Ok(ParsedOperand::Literal {
                literal: Literal::Bool(false),
                span: token.span,
            }),
            TokenKind::Number(value) => Ok(ParsedOperand::Literal {
                literal: Literal::Integer(value),
                span: token.span,
            }),
            TokenKind::String(value) => Ok(ParsedOperand::Literal {
                literal: Literal::String(value),
                span: token.span,
            }),
            _ => Err(DatalogError::Parse {
                span: token.span,
                message: "expected comparison operand".to_owned(),
            }),
        }
    }

    fn parse_comparison_operator(&mut self) -> Result<ComparisonOperator> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Eq => Ok(ComparisonOperator::Eq),
            TokenKind::NotEq => Ok(ComparisonOperator::NotEq),
            TokenKind::Lt => Ok(ComparisonOperator::Lt),
            TokenKind::Lte => Ok(ComparisonOperator::Lte),
            TokenKind::Gt => Ok(ComparisonOperator::Gt),
            TokenKind::Gte => Ok(ComparisonOperator::Gte),
            _ => Err(DatalogError::Parse {
                span: token.span,
                message: "expected comparison operator".to_owned(),
            }),
        }
    }

    fn reject_clause_unsupported(&self) -> Result<()> {
        if let TokenKind::Ident(name) = self.peek_kind() {
            let feature = match name.as_str() {
                "not" => Some("stratified negation"),
                "or" => Some("disjunction"),
                "order" | "order_by" => Some("ordered output"),
                "limit" => Some("limit"),
                "as_of" => Some("as-of queries"),
                _ => None,
            };
            if let Some(feature) = feature {
                return Err(DatalogError::UnsupportedFeature {
                    span: self.peek().span,
                    feature: feature.to_owned(),
                });
            }
        }
        Ok(())
    }

    fn expect_keyword(&mut self, keyword: &str) -> Result<Token> {
        let token = self.advance().clone();
        match &token.kind {
            TokenKind::Ident(name) if name == keyword => Ok(token),
            _ => Err(DatalogError::Parse {
                span: token.span,
                message: format!("expected keyword {keyword}"),
            }),
        }
    }

    fn expect_ident(&mut self) -> Result<(String, Span)> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Ident(name) => Ok((name, token.span)),
            _ => Err(DatalogError::Parse {
                span: token.span,
                message: "expected identifier".to_owned(),
            }),
        }
    }

    fn expect_variable(&mut self) -> Result<(String, Span)> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Variable(name) => Ok((name, token.span)),
            _ => Err(DatalogError::Parse {
                span: token.span,
                message: "expected variable".to_owned(),
            }),
        }
    }

    fn expect_kind(&mut self, shape: TokenShape) -> Result<Token> {
        let token = self.advance().clone();
        if shape.matches(&token.kind) {
            Ok(token)
        } else {
            Err(DatalogError::Parse {
                span: token.span,
                message: format!("expected {}", shape.name()),
            })
        }
    }

    fn check_keyword(&self, keyword: &str) -> bool {
        matches!(self.peek_kind(), TokenKind::Ident(name) if name == keyword)
    }

    fn check_shape(&self, shape: TokenShape) -> bool {
        shape.matches(self.peek_kind())
    }

    fn advance(&mut self) -> &Token {
        let index = self.pos;
        if !self.is_eof() {
            self.pos += 1;
        }
        &self.tokens[index]
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn is_eof(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }

    fn previous_span(&self) -> Option<Span> {
        self.pos.checked_sub(1).map(|index| self.tokens[index].span)
    }

    fn parse_error(&self, message: impl Into<String>) -> DatalogError {
        DatalogError::Parse {
            span: self.peek().span,
            message: message.into(),
        }
    }
}

#[derive(Clone, Copy)]
enum TokenShape {
    LParen,
    RParen,
    Colon,
    Comma,
}

impl TokenShape {
    fn matches(self, kind: &TokenKind) -> bool {
        matches!(
            (self, kind),
            (TokenShape::LParen, TokenKind::LParen)
                | (TokenShape::RParen, TokenKind::RParen)
                | (TokenShape::Colon, TokenKind::Colon)
                | (TokenShape::Comma, TokenKind::Comma)
        )
    }

    fn name(self) -> &'static str {
        match self {
            TokenShape::LParen => "(",
            TokenShape::RParen => ")",
            TokenShape::Colon => ":",
            TokenShape::Comma => ",",
        }
    }
}

fn parse_aggregate_function(name: &str) -> Option<AggregateFunction> {
    match name {
        "count" => Some(AggregateFunction::Count),
        "sum" => Some(AggregateFunction::Sum),
        "min" => Some(AggregateFunction::Min),
        "max" => Some(AggregateFunction::Max),
        _ => None,
    }
}

fn reject_source_level_unsupported(source: &str) -> Result<()> {
    for marker in [":-", "<-"] {
        if let Some(start) = source.find(marker) {
            return Err(DatalogError::UnsupportedFeature {
                span: Span {
                    start,
                    end: start + marker.len(),
                },
                feature: "rules and recursion".to_owned(),
            });
        }
    }
    Ok(())
}

struct Typechecker<'schema> {
    schema: &'schema SchemaDescriptor,
    query: ParsedQuery,
    vars: Vec<TypedVariable>,
    var_ids: BTreeMap<String, usize>,
    inputs: Vec<TypedInput>,
    input_ids: BTreeMap<String, usize>,
    clauses: Vec<TypedClause>,
}

impl<'schema> Typechecker<'schema> {
    fn new(schema: &'schema SchemaDescriptor, query: ParsedQuery) -> Self {
        Self {
            schema,
            query,
            vars: Vec::new(),
            var_ids: BTreeMap::new(),
            inputs: Vec::new(),
            input_ids: BTreeMap::new(),
            clauses: Vec::new(),
        }
    }

    fn finish(mut self) -> Result<TypedQuery> {
        let clauses = self.query.clauses.clone();
        for clause in clauses {
            if let ParsedClause::Relation(atom) = clause {
                let typed = self.type_relation(atom)?;
                self.clauses.push(TypedClause::Relation(typed));
            }
        }

        let clauses = self.query.clauses.clone();
        for clause in clauses {
            if let ParsedClause::Comparison(comparison) = clause {
                let typed = self.type_comparison(comparison)?;
                self.clauses.push(TypedClause::Comparison(typed));
            }
        }

        let find = self
            .query
            .find
            .clone()
            .into_iter()
            .map(|term| self.type_find_term(term))
            .collect::<Result<Vec<_>>>()?;

        Ok(TypedQuery {
            variables: self.vars,
            inputs: self.inputs,
            find,
            clauses: self.clauses,
        })
    }

    fn type_relation(&mut self, atom: ParsedRelationAtom) -> Result<TypedRelationAtom> {
        let (relation_id, relation) = self.relation(&atom.relation, atom.span)?;
        let mut fields = Vec::new();

        for binding in atom.fields {
            let (field_id, field) = self.field(relation, &binding.field, binding.span)?;
            let term = self.type_term(binding.term, &field.value_type)?;
            fields.push(TypedFieldBinding {
                field_id,
                field: field.name.clone(),
                value_type: field.value_type.clone(),
                term,
            });
        }

        Ok(TypedRelationAtom {
            relation_id,
            relation: relation.name.clone(),
            fields,
        })
    }

    fn type_term(&mut self, term: ParsedTerm, expected: &ValueType) -> Result<TypedTerm> {
        match term {
            ParsedTerm::Variable { name, span } => Ok(TypedTerm::Variable(self.bind_variable(
                name,
                expected.clone(),
                span,
            )?)),
            ParsedTerm::Input { name, span } => Ok(TypedTerm::Input(self.bind_input(
                name,
                expected.clone(),
                span,
            )?)),
            ParsedTerm::Wildcard { .. } => Ok(TypedTerm::Wildcard),
            ParsedTerm::Literal { literal, span } => Ok(TypedTerm::Literal(
                self.type_literal(literal, expected, span)?,
            )),
        }
    }

    fn type_comparison(&mut self, comparison: ParsedComparison) -> Result<TypedComparison> {
        let left_type = self.operand_type(&comparison.left);
        let right_type = self.operand_type(&comparison.right);
        let comparison_type = match (left_type, right_type) {
            (Some(left), Some(right)) => {
                merge_types(&left, &right).ok_or_else(|| DatalogError::VariableTypeConflict {
                    span: comparison.span,
                    variable: "comparison".to_owned(),
                    existing: type_name(&left),
                    incoming: type_name(&right),
                })?
            }
            (Some(value_type), None) | (None, Some(value_type)) => value_type,
            (None, None) => {
                return Err(DatalogError::UnboundComparisonOperand {
                    span: comparison.span,
                });
            }
        };

        if comparison.operator != ComparisonOperator::Eq && !is_orderable(&comparison_type) {
            return Err(DatalogError::LiteralTypeMismatch {
                span: comparison.span,
                expected: format!("orderable type, got {}", type_name(&comparison_type)),
            });
        }

        let left = self.type_operand(comparison.left, &comparison_type)?;
        let right = self.type_operand(comparison.right, &comparison_type)?;
        Ok(TypedComparison {
            left,
            operator: comparison.operator,
            right,
            value_type: comparison_type,
        })
    }

    fn operand_type(&self, operand: &ParsedOperand) -> Option<ValueType> {
        match operand {
            ParsedOperand::Variable { name, .. } => self
                .var_ids
                .get(name)
                .map(|id| self.vars[*id].value_type.clone()),
            ParsedOperand::Input { name, .. } => self
                .input_ids
                .get(name)
                .map(|id| self.inputs[*id].value_type.clone()),
            ParsedOperand::Literal { .. } => None,
        }
    }

    fn type_operand(
        &mut self,
        operand: ParsedOperand,
        expected: &ValueType,
    ) -> Result<TypedOperand> {
        match operand {
            ParsedOperand::Variable { name, span } => Ok(TypedOperand::Variable(
                self.bind_variable(name, expected.clone(), span)?,
            )),
            ParsedOperand::Input { name, span } => Ok(TypedOperand::Input(self.bind_input(
                name,
                expected.clone(),
                span,
            )?)),
            ParsedOperand::Literal { literal, span } => Ok(TypedOperand::Literal(
                self.type_literal(literal, expected, span)?,
            )),
        }
    }

    fn type_find_term(&mut self, term: ParsedFindTerm) -> Result<TypedFindTerm> {
        match term {
            ParsedFindTerm::Variable { name, span } => {
                let Some(id) = self.var_ids.get(&name).copied() else {
                    return Err(DatalogError::UnboundProjectionVariable {
                        span,
                        variable: name,
                    });
                };
                Ok(TypedFindTerm::Variable { variable: id })
            }
            ParsedFindTerm::Aggregate {
                function,
                variable,
                span,
            } => {
                let Some(id) = self.var_ids.get(&variable).copied() else {
                    return Err(DatalogError::UnboundAggregateVariable { span, variable });
                };
                let value_type = self.vars[id].value_type.clone();
                if !aggregate_supports(function, &value_type) {
                    return Err(DatalogError::InvalidAggregateType {
                        span,
                        function,
                        value_type: type_name(&value_type),
                    });
                }
                Ok(TypedFindTerm::Aggregate {
                    function,
                    variable: id,
                    value_type,
                })
            }
        }
    }

    fn type_literal(
        &self,
        literal: Literal,
        expected: &ValueType,
        span: Span,
    ) -> Result<TypedLiteral> {
        if literal_fits_type(self.schema, &literal, expected) {
            Ok(TypedLiteral {
                literal,
                value_type: expected.clone(),
            })
        } else {
            Err(DatalogError::LiteralTypeMismatch {
                span,
                expected: type_name(expected),
            })
        }
    }

    fn bind_variable(&mut self, name: String, incoming: ValueType, span: Span) -> Result<usize> {
        if let Some(id) = self.var_ids.get(&name).copied() {
            let existing = self.vars[id].value_type.clone();
            let Some(merged) = merge_types(&existing, &incoming) else {
                return Err(DatalogError::VariableTypeConflict {
                    span,
                    variable: name,
                    existing: type_name(&existing),
                    incoming: type_name(&incoming),
                });
            };
            self.vars[id].value_type = merged;
            Ok(id)
        } else {
            let id = self.vars.len();
            self.var_ids.insert(name.clone(), id);
            self.vars.push(TypedVariable {
                id,
                name,
                value_type: incoming,
            });
            Ok(id)
        }
    }

    fn bind_input(&mut self, name: String, incoming: ValueType, span: Span) -> Result<usize> {
        if let Some(id) = self.input_ids.get(&name).copied() {
            let existing = self.inputs[id].value_type.clone();
            let Some(merged) = merge_types(&existing, &incoming) else {
                return Err(DatalogError::InputTypeConflict {
                    span,
                    input: name,
                    existing: type_name(&existing),
                    incoming: type_name(&incoming),
                });
            };
            self.inputs[id].value_type = merged;
            Ok(id)
        } else {
            let id = self.inputs.len();
            self.input_ids.insert(name.clone(), id);
            self.inputs.push(TypedInput {
                id,
                name,
                value_type: incoming,
            });
            Ok(id)
        }
    }

    fn relation(&self, name: &str, span: Span) -> Result<(usize, &'schema RelationDescriptor)> {
        self.schema
            .relations
            .iter()
            .enumerate()
            .find(|(_, relation)| relation.name == name)
            .ok_or_else(|| {
                if looks_like_function(name) {
                    DatalogError::UnsupportedFeature {
                        span,
                        feature: "user-defined functions".to_owned(),
                    }
                } else {
                    DatalogError::UnknownRelation {
                        span,
                        relation: name.to_owned(),
                    }
                }
            })
    }

    fn field(
        &self,
        relation: &'schema RelationDescriptor,
        name: &str,
        span: Span,
    ) -> Result<(usize, &'schema FieldDescriptor)> {
        relation
            .fields
            .iter()
            .enumerate()
            .find(|(_, field)| field.name == name)
            .ok_or_else(|| DatalogError::UnknownField {
                span,
                relation: relation.name.clone(),
                field: name.to_owned(),
            })
    }
}

fn merge_types(existing: &ValueType, incoming: &ValueType) -> Option<ValueType> {
    if existing == incoming {
        return Some(existing.clone());
    }

    match (existing, incoming) {
        (
            ValueType::Id {
                name: id_name,
                relation,
            },
            ValueType::Ref {
                name: ref_name,
                target_relation,
            },
        )
        | (
            ValueType::Ref {
                name: ref_name,
                target_relation,
            },
            ValueType::Id {
                name: id_name,
                relation,
            },
        ) if id_name == ref_name && relation == target_relation => Some(ValueType::Id {
            name: id_name.clone(),
            relation: relation.clone(),
        }),
        _ => None,
    }
}

fn literal_fits_type(schema: &SchemaDescriptor, literal: &Literal, expected: &ValueType) -> bool {
    match (literal, expected) {
        (Literal::Bool(_), ValueType::Bool) => true,
        (Literal::String(_), ValueType::String) => true,
        (Literal::Integer(value), ValueType::Enum { name }) => {
            *value >= 0
                && *value <= u64::MAX as i128
                && schema.enum_contains_code(name, *value as u64)
        }
        (
            Literal::Integer(value),
            ValueType::U64 | ValueType::Id { .. } | ValueType::Ref { .. } | ValueType::Code { .. },
        ) => *value >= 0 && *value <= u64::MAX as i128,
        (Literal::Integer(value), ValueType::I64 | ValueType::TimestampMicros) => {
            *value >= i64::MIN as i128 && *value <= i64::MAX as i128
        }
        (Literal::Integer(_), ValueType::Decimal { .. }) => true,
        _ => false,
    }
}

fn aggregate_supports(function: AggregateFunction, value_type: &ValueType) -> bool {
    match function {
        AggregateFunction::Count => true,
        AggregateFunction::Sum => matches!(
            value_type,
            ValueType::U64 | ValueType::I64 | ValueType::Decimal { .. }
        ),
        AggregateFunction::Min | AggregateFunction::Max => is_orderable(value_type),
    }
}

fn is_orderable(value_type: &ValueType) -> bool {
    matches!(
        value_type,
        ValueType::U64
            | ValueType::I64
            | ValueType::Id { .. }
            | ValueType::Ref { .. }
            | ValueType::TimestampMicros
            | ValueType::Decimal { .. }
            | ValueType::Code { .. }
    )
}

fn type_name(value_type: &ValueType) -> String {
    match value_type {
        ValueType::Bool => "bool".to_owned(),
        ValueType::U64 => "u64".to_owned(),
        ValueType::I64 => "i64".to_owned(),
        ValueType::Id { name, relation } => format!("{name}@{relation}"),
        ValueType::Ref {
            name,
            target_relation,
        } => format!("{name}->{target_relation}"),
        ValueType::TimestampMicros => "timestamp".to_owned(),
        ValueType::Decimal { scale } => format!("decimal(scale={scale})"),
        ValueType::Uuid => "uuid".to_owned(),
        ValueType::Enum { name } | ValueType::Code { name } => name.clone(),
        ValueType::String => "string".to_owned(),
        ValueType::Bytes => "bytes".to_owned(),
    }
}

fn looks_like_function(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_lowercase)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{
        ConstraintDescriptor, FieldDescriptor, GeneratedIdDescriptor, PrimaryKeyDescriptor,
        RelationDescriptor, RelationKind,
    };

    #[test]
    fn parses_and_typechecks_single_relation_query() -> Result<()> {
        let typed = parse_and_typecheck(
            &schema(),
            r#"
            find ?account ?currency
            where
              Account(id: ?account, holder: $holder, currency: ?currency)
            "#,
        )?;

        assert_eq!(typed.variables.len(), 2);
        assert_eq!(typed.inputs.len(), 1);
        assert_eq!(typed.find.len(), 2);
        assert_eq!(typed.clauses.len(), 1);
        Ok(())
    }

    #[test]
    fn parses_and_typechecks_multi_relation_join() -> Result<()> {
        let typed = parse_and_typecheck(
            &schema(),
            r#"
            find ?account ?holder_name
            where
              Account(id: ?account, holder: ?holder, currency: ?currency)
              Holder(id: ?holder, name: ?holder_name)
            "#,
        )?;

        let holder = typed
            .variables
            .iter()
            .find(|var| var.name == "holder")
            .ok_or_else(|| DatalogError::Parse {
                span: Span { start: 0, end: 0 },
                message: "missing holder var".to_owned(),
            })?;
        assert_eq!(holder.value_type, holder_id_type());
        assert_eq!(typed.clauses.len(), 2);
        Ok(())
    }

    #[test]
    fn parses_comparisons_ranges_and_inputs() -> Result<()> {
        let typed = parse_and_typecheck(
            &schema(),
            r#"
            find ?posting ?amount
            where
              Posting(id: ?posting, account: $account, amount: ?amount, at: ?t)
              ?t >= $start
              ?t < $end
              ?amount != 0
            "#,
        )?;

        assert!(typed.inputs.iter().any(|input| input.name == "start"));
        assert!(typed.inputs.iter().any(|input| input.name == "end"));
        assert_eq!(
            typed
                .clauses
                .iter()
                .filter(|clause| matches!(clause, TypedClause::Comparison(_)))
                .count(),
            3
        );
        Ok(())
    }

    #[test]
    fn parses_aggregate_projection() -> Result<()> {
        let typed = parse_and_typecheck(
            &schema(),
            r#"
            find ?account sum(?amount) count(?posting)
            where
              Posting(id: ?posting, account: ?account, amount: ?amount, at: ?t)
            "#,
        )?;

        assert!(matches!(
            typed.find[1],
            TypedFindTerm::Aggregate {
                function: AggregateFunction::Sum,
                ..
            }
        ));
        assert!(matches!(
            typed.find[2],
            TypedFindTerm::Aggregate {
                function: AggregateFunction::Count,
                ..
            }
        ));
        Ok(())
    }

    #[test]
    fn unknown_relation_is_clear() {
        assert!(
            matches!(parse_and_typecheck(&schema(), "find ?x where Missing(id: ?x)"), Err(DatalogError::UnknownRelation { relation, .. }) if relation == "Missing")
        );
    }

    #[test]
    fn unknown_field_is_clear() {
        assert!(matches!(
            parse_and_typecheck(&schema(), "find ?x where Account(nope: ?x)"),
            Err(DatalogError::UnknownField { field, .. }) if field == "nope"
        ));
    }

    #[test]
    fn incompatible_variable_unification_is_rejected() {
        let result = parse_and_typecheck(
            &schema(),
            r#"
            find ?x
            where
              Account(id: ?x)
              Holder(name: ?x)
            "#,
        );
        assert!(
            matches!(result, Err(DatalogError::VariableTypeConflict { variable, .. }) if variable == "x")
        );
    }

    #[test]
    fn incompatible_input_reuse_is_rejected() {
        let result = parse_and_typecheck(
            &schema(),
            r#"
            find ?account
            where
              Account(id: ?account, holder: $x)
              Holder(name: $x)
            "#,
        );
        assert!(
            matches!(result, Err(DatalogError::InputTypeConflict { input, .. }) if input == "x")
        );
    }

    #[test]
    fn unbound_projection_is_rejected() {
        assert!(
            matches!(parse_and_typecheck(&schema(), "find ?x where Holder(id: ?holder)"), Err(DatalogError::UnboundProjectionVariable { variable, .. }) if variable == "x")
        );
    }

    #[test]
    fn unsupported_features_are_intentional() {
        for query in [
            "find ?x where not Holder(id: ?x)",
            "find ?x where or Holder(id: ?x)",
            "find ?x where Holder(id: ?x) limit 1",
            "find ?x where Holder(id: ?x) order ?x",
            "find ?x where as_of 1 Holder(id: ?x)",
            "find ?x where rule(?x) :- Holder(id: ?x)",
            "find ?x where Holder(?x)",
            "find ?x where lower(?x)",
        ] {
            let result = parse_and_typecheck(&schema(), query);
            assert!(
                matches!(result, Err(DatalogError::UnsupportedFeature { .. })),
                "{query}: {result:?}"
            );
        }
    }

    #[test]
    fn aggregate_type_validation_is_enforced() {
        assert!(matches!(
            parse_and_typecheck(&schema(), "find sum(?name) where Holder(name: ?name)"),
            Err(DatalogError::InvalidAggregateType {
                function: AggregateFunction::Sum,
                ..
            })
        ));
    }

    fn schema() -> SchemaDescriptor {
        SchemaDescriptor::new(
            "LedgerDb",
            vec![
                RelationDescriptor::new(
                    "Holder",
                    RelationKind::Entity,
                    vec![
                        FieldDescriptor::new("id", holder_id_type()),
                        FieldDescriptor::new("name", ValueType::String),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_generated_id(GeneratedIdDescriptor::new("id"))
                .with_constraint(ConstraintDescriptor::unique("name", ["name"])),
                RelationDescriptor::new(
                    "Account",
                    RelationKind::Entity,
                    vec![
                        FieldDescriptor::new(
                            "id",
                            ValueType::Id {
                                name: "AccountId".to_owned(),
                                relation: "Account".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "holder",
                            ValueType::Ref {
                                name: "HolderId".to_owned(),
                                target_relation: "Holder".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "currency",
                            ValueType::Enum {
                                name: "Currency".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_generated_id(GeneratedIdDescriptor::new("id")),
                RelationDescriptor::new(
                    "Posting",
                    RelationKind::Event,
                    vec![
                        FieldDescriptor::new(
                            "id",
                            ValueType::Id {
                                name: "PostingId".to_owned(),
                                relation: "Posting".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "account",
                            ValueType::Ref {
                                name: "AccountId".to_owned(),
                                target_relation: "Account".to_owned(),
                            },
                        ),
                        FieldDescriptor::new("amount", ValueType::Decimal { scale: 4 }),
                        FieldDescriptor::new("at", ValueType::TimestampMicros).range_indexed(),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_generated_id(GeneratedIdDescriptor::new("id")),
            ],
        )
        .with_enum(crate::schema::EnumDescriptor::codes("Currency", [840, 978]))
        .with_ref_foreign_keys()
    }

    fn holder_id_type() -> ValueType {
        ValueType::Id {
            name: "HolderId".to_owned(),
            relation: "Holder".to_owned(),
        }
    }
}
