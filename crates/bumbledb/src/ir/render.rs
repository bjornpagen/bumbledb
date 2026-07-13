//! Query rendering back to the rule notation — the statement renderer's
//! sibling (`crate::schema::render`), on the read side of the data
//! surface: **when the write-side query surface is data, the renderer is
//! the pretty syntax** (`docs/architecture/20-query-ir.md` § the data
//! surface). One clause per rule, set-builder shaped:
//!
//! ```text
//! (v0, v1) | Busy(person: v0, during: v1), Allen(v1, INTERSECTS, ?0);
//! ```
//!
//! The grammar is the schema grammar's own query side, promoted
//! (`docs/architecture/20-query-ir.md` owns the normative block; this module emits it):
//! atoms as `Relation(field: var)`, in-atom selections `field == literal`
//! (schema-grammar-verbatim, params admitted as `?N`), `!` negation,
//! membership as `in`, `Allen(term, MASK, term)` with masks as named
//! basics joined by `|` (set union) or the workload composites, `;`
//! terminating each clause exactly as it terminates statements.
//!
//! Deterministic and **total on plain data** — its consumers are
//! diagnostics (roster errors print the offending query, so malformed
//! shapes must render, not panic) and the EXPLAIN/stats surface:
//!
//! - variables render as `v{id}` and params as `?{id}` (the IR carries
//!   dense ids only; names are a debugging sidecar the engine never
//!   stores);
//! - unresolvable ids render as `relation#N` / `field#N` placeholders
//!   (the statement renderer's convention — the bad id can be the very
//!   thing validation rejected);
//! - a literal word bound at a **closed-reference position** (a field
//!   whose declared containment targets a closed relation's id, or the
//!   closed relation's own id field) prints its **handle** (`kind ==
//!   DirectPass`) — the vocabulary's name, resolved through the sealed
//!   extension; an out-of-range word prints visibly wrong as
//!   `Kind(7?)` (the relation's name — the engine never learns host
//!   newtype names), because rendering hides nothing. Comparison terms
//!   carry no field position, so a literal there renders by value;
//!   the notation's selection form is the handle's home;
//! - the Arg terms, absent from the notation grammar (Arg is single-rule
//!   only and its key is rule-internal), render as `ArgMax(carried,
//!   key)` — an honest extension, not grammar;
//! - a nested predicate tree renders functionally (`and(..)` / `or(..)`)
//!   — validated queries are Or-free downstream, so grammar-pure output
//!   holds for every query written in the notation; the functional forms
//!   appear only when diagnostics picture an input tree.
//!
//! Rendering allocates; it runs only in diagnostic contexts (roster
//! errors, EXPLAIN, arbitration bundles), never on a warm path.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::allen::AllenMask;
use crate::ir::{
    AggOp, Atom, CmpOp, Comparison, FindTerm, MaskTerm, ParamId, PredicateTree, Query, Rule, Term,
    Value, VarId,
};
use crate::schema::{Enforcement, FieldDescriptor, FieldId, Relation, RelationId, Schema};

/// The closed-reference position table, built once at renderer
/// construction: `(relation, field)` → the closed relation whose row ids
/// the field's words are. A schema walk over declared containments whose
/// target is a closed relation's id and whose source projection is that
/// single field — the same inference the chase's complement fold runs
/// (`plan/chase/evaluate.rs::containment_into_id`) — plus each closed
/// relation's own id field, which maps to itself.
struct ClosedRefs(BTreeMap<(RelationId, FieldId), RelationId>);

impl ClosedRefs {
    fn build(schema: &Schema) -> Self {
        let mut map = BTreeMap::new();
        for statement in schema.containments() {
            if !matches!(statement.enforcement, Enforcement::Closed { .. }) {
                continue;
            }
            let source = &statement.source;
            let target = &statement.target;
            let target_closed = schema
                .relation_checked(target.relation)
                .is_some_and(Relation::is_closed);
            if target_closed
                && target.projection.as_ref() == [FieldId(0)]
                && let [field] = source.projection.as_ref()
            {
                map.insert((source.relation, *field), target.relation);
            }
        }
        for (index, relation) in schema.relations().iter().enumerate() {
            if relation.is_closed() {
                let id = RelationId(u32::try_from(index).expect("relation count fits u32"));
                map.insert((id, FieldId(0)), id);
            }
        }
        Self(map)
    }

    /// The handle spelling for a literal at `(relation, field)`: `Some`
    /// iff the position is a closed reference and the value is a word —
    /// the handle for an in-range row id, the visibly-wrong `Kind(7?)`
    /// for an out-of-range one (rendering hides nothing). `None` means
    /// the position is no closed reference (or the value no word) and the
    /// literal renders plainly.
    fn handle(
        &self,
        schema: &Schema,
        relation: RelationId,
        field: FieldId,
        value: &Value,
    ) -> Option<String> {
        let closed = *self.0.get(&(relation, field))?;
        let Value::U64(word) = value else {
            return None;
        };
        let rows = schema.relation_checked(closed)?.extension()?;
        match usize::try_from(*word).ok().and_then(|row| rows.get(row)) {
            Some(row) => Some(row.handle.to_string()),
            None => Some(format!(
                "{}({word}?)",
                schema.relation_checked(closed).map_or("?", Relation::name)
            )),
        }
    }
}

/// Renders a query in the rule notation, one clause per rule, newline-
/// separated, each clause `;`-terminated. Deterministic (two calls yield
/// one string) and total: malformed queries render with placeholder
/// names — this is the diagnostic surface for the roster's rejections.
#[must_use]
pub fn render(schema: &Schema, query: &Query) -> String {
    let refs = ClosedRefs::build(schema);
    let mut out = String::new();
    for (index, rule) in query.rules.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        clause(&mut out, schema, &refs, rule);
    }
    out
}

/// One rule as one clause: `(head) | body;`.
fn clause(out: &mut String, schema: &Schema, refs: &ClosedRefs, rule: &Rule) {
    out.push('(');
    for (index, term) in rule.finds.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        find_term(out, term);
    }
    out.push_str(") |");
    let mut items: Vec<String> = Vec::new();
    for atom in &rule.atoms {
        items.push(atom_item(schema, refs, atom, false));
    }
    for atom in &rule.negated {
        items.push(atom_item(schema, refs, atom, true));
    }
    for tree in &rule.predicates {
        items.push(tree_item(tree));
    }
    if !items.is_empty() {
        out.push(' ');
        out.push_str(&items.join(", "));
    }
    out.push(';');
}

/// One head position of a clause. `Count` is nullary; a malformed
/// `Count(v)` renders its variable anyway (totality over the shapes the
/// roster rejects).
fn find_term(out: &mut String, term: &FindTerm) {
    match term {
        FindTerm::Var(var) => var_name(out, *var),
        FindTerm::Duration(var) => {
            out.push_str("Duration(");
            var_name(out, *var);
            out.push(')');
        }
        FindTerm::Aggregate { op, over } => aggregate(out, *op, *over, false),
        FindTerm::AggregateDuration { op, over } => aggregate(out, *op, Some(*over), true),
    }
}

/// One aggregate head term: `Sum(v0)`, `Count`, `Pack(v1)`,
/// `Sum(Duration(v0))`, `ArgMax(v0, v1)` (carried, key — the honest
/// extension; see the module doc).
fn aggregate(out: &mut String, op: AggOp, over: Option<VarId>, measure: bool) {
    let name = match op {
        AggOp::Sum => "Sum",
        AggOp::Min => "Min",
        AggOp::Max => "Max",
        AggOp::Count => "Count",
        AggOp::CountDistinct => "CountDistinct",
        AggOp::ArgMax { .. } => "ArgMax",
        AggOp::ArgMin { .. } => "ArgMin",
        AggOp::Pack => "Pack",
    };
    out.push_str(name);
    let key = match op {
        AggOp::ArgMax { key } | AggOp::ArgMin { key } => Some(key),
        _ => None,
    };
    if over.is_none() && key.is_none() {
        return;
    }
    out.push('(');
    if let Some(var) = over {
        if measure {
            out.push_str("Duration(");
            var_name(out, var);
            out.push(')');
        } else {
            var_name(out, var);
        }
    }
    if let Some(key) = key {
        if over.is_some() {
            out.push_str(", ");
        }
        var_name(out, key);
    }
    out.push(')');
}

/// One atom: `Relation(field: v0, field == literal, field in ?1)`, with
/// `!` prefixed for a negated occurrence. Binding forms: a variable binds
/// as `field: vN` (the join spelling); a literal or scalar param is an
/// in-atom selection `field == term` (the schema grammar's selections
/// with params admitted — on an interval field an element-typed term
/// reads as membership under the same bivalent typing rule the IR
/// binding carries); a param set is membership, `field in ?N`. A literal
/// word at a closed-reference position prints its handle (module doc).
fn atom_item(schema: &Schema, refs: &ClosedRefs, atom: &Atom, negated: bool) -> String {
    let mut out = String::new();
    if negated {
        out.push('!');
    }
    relation_name(&mut out, schema, atom.relation);
    out.push('(');
    for (index, (field, term)) in atom.bindings.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        field_name(&mut out, schema, atom.relation, *field);
        match term {
            Term::Var(var) => {
                out.push_str(": ");
                var_name(&mut out, *var);
            }
            Term::ParamSet(param) => {
                out.push_str(" in ");
                param_name(&mut out, *param);
            }
            Term::Param(param) => {
                out.push_str(" == ");
                param_name(&mut out, *param);
            }
            Term::Literal(value) => {
                out.push_str(" == ");
                match refs.handle(schema, atom.relation, *field, value) {
                    Some(handle) => out.push_str(&handle),
                    None => literal(&mut out, value),
                }
            }
            // Rejected by validation (`DurationInBinding`); rendered
            // anyway — the diagnostic pictures the mistake.
            Term::Duration(var) => {
                out.push_str(" == Duration(");
                var_name(&mut out, *var);
                out.push(')');
            }
        }
    }
    out.push(')');
    out
}

/// One predicate tree: a leaf is a comparison item; `And`/`Or` render
/// functionally (module doc — the input grammar's trees are pictures,
/// not notation). Depth-budgeted at [`crate::ir::MAX_PREDICATE_DEPTH`]:
/// the renderer recurses by depth and must stay total on the hostile
/// nesting validation rejects, so anything past the boundary guard's own
/// cap elides to `...` instead of exhausting the stack.
fn tree_item(tree: &PredicateTree) -> String {
    tree_item_within(tree, crate::ir::MAX_PREDICATE_DEPTH)
}

fn tree_item_within(tree: &PredicateTree, budget: usize) -> String {
    if budget == 0 {
        return "...".to_owned();
    }
    match tree {
        PredicateTree::Leaf(cmp) => comparison(cmp),
        PredicateTree::And(children) => functional("and", children, budget),
        PredicateTree::Or(children) => functional("or", children, budget),
    }
}

fn functional(name: &str, children: &[PredicateTree], budget: usize) -> String {
    let inner: Vec<String> = children
        .iter()
        .map(|child| tree_item_within(child, budget - 1))
        .collect();
    format!("{name}({})", inner.join(", "))
}

/// One comparison item: `Allen(a, MASK, b)`, membership `point in
/// interval` (the `Contains` predicate's notation), or infix `lhs op rhs`.
fn comparison(cmp: &Comparison) -> String {
    let mut out = String::new();
    match cmp.op {
        CmpOp::Allen { mask } => {
            out.push_str("Allen(");
            term(&mut out, &cmp.lhs);
            out.push_str(", ");
            mask_term(&mut out, mask);
            out.push_str(", ");
            term(&mut out, &cmp.rhs);
            out.push(')');
        }
        // `Contains(interval, point)` is point membership as a predicate:
        // the notation reads point-first.
        CmpOp::Contains => {
            term(&mut out, &cmp.rhs);
            out.push_str(" in ");
            term(&mut out, &cmp.lhs);
        }
        CmpOp::Eq | CmpOp::Ne | CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge => {
            let op = match cmp.op {
                CmpOp::Eq => "==",
                CmpOp::Ne => "!=",
                CmpOp::Lt => "<",
                CmpOp::Le => "<=",
                CmpOp::Gt => ">",
                CmpOp::Ge => ">=",
                CmpOp::Allen { .. } | CmpOp::Contains => unreachable!("matched above"),
            };
            term(&mut out, &cmp.lhs);
            out.push(' ');
            out.push_str(op);
            out.push(' ');
            term(&mut out, &cmp.rhs);
        }
    }
    out
}

/// One comparison term. Literals render by value — a comparison carries
/// no field position, so no closed-reference resolution applies here
/// (module doc: the selection form is the handle's home).
fn term(out: &mut String, term: &Term) {
    match term {
        Term::Var(var) => var_name(out, *var),
        Term::Param(param) | Term::ParamSet(param) => param_name(out, *param),
        Term::Literal(value) => literal(out, value),
        Term::Duration(var) => {
            out.push_str("Duration(");
            var_name(out, *var);
            out.push(')');
        }
    }
}

/// The mask position: a named mask expression, or the param.
fn mask_term(out: &mut String, mask: MaskTerm) {
    match mask {
        MaskTerm::Literal(mask) => mask_names(out, mask),
        MaskTerm::Param(param) => param_name(out, param),
    }
}

/// A literal mask as named values of the algebra: an exact workload
/// composite by its name, else the singleton basics joined by `|` (the
/// mask-level bar is set union over the 13). The vacuous masks — typed
/// rejections, but diagnostics picture them — render as `EMPTY`/`FULL`.
/// Crate-visible for the statically-empty verdict pictures
/// (`ir/normalize/fold.rs`) — one mask notation on every diagnostic
/// surface.
pub(crate) fn mask_names(out: &mut String, mask: AllenMask) {
    const COMPOSITES: [(&str, AllenMask); 4] = [
        ("INTERSECTS", AllenMask::INTERSECTS),
        ("DISJOINT", AllenMask::DISJOINT),
        ("COVERS", AllenMask::COVERS),
        ("COVERED_BY", AllenMask::COVERED_BY),
    ];
    const SINGLETONS: [(&str, AllenMask); 13] = [
        ("BEFORE", AllenMask::BEFORE),
        ("MEETS", AllenMask::MEETS),
        ("OVERLAPS", AllenMask::OVERLAPS),
        ("STARTS", AllenMask::STARTS),
        ("DURING", AllenMask::DURING),
        ("FINISHES", AllenMask::FINISHES),
        ("EQUALS", AllenMask::EQUALS),
        ("FINISHED_BY", AllenMask::FINISHED_BY),
        ("CONTAINS", AllenMask::CONTAINS),
        ("STARTED_BY", AllenMask::STARTED_BY),
        ("OVERLAPPED_BY", AllenMask::OVERLAPPED_BY),
        ("MET_BY", AllenMask::MET_BY),
        ("AFTER", AllenMask::AFTER),
    ];
    if mask.is_empty() {
        out.push_str("EMPTY");
        return;
    }
    if mask.is_full() {
        out.push_str("FULL");
        return;
    }
    if let Some((name, _)) = COMPOSITES.iter().find(|(_, value)| *value == mask) {
        out.push_str(name);
        return;
    }
    let mut first = true;
    for (name, singleton) in SINGLETONS {
        if mask.bits() & singleton.bits() != 0 {
            if !first {
                out.push('|');
            }
            out.push_str(name);
            first = false;
        }
    }
}

/// One literal, in the statement renderer's value formats (one notation,
/// schema to query): intervals as `start..end`, strings and byte strings
/// escaped. Field-blind by design — closed-reference resolution happens
/// at the positions that carry a field ([`ClosedRefs::handle`]).
/// Crate-visible for the statically-empty verdict pictures
/// (`ir/normalize/fold.rs`) — one value notation on every diagnostic
/// surface.
pub(crate) fn literal(out: &mut String, value: &Value) {
    match value {
        Value::Bool(v) => {
            let _ = write!(out, "{v}");
        }
        Value::U64(v) => {
            let _ = write!(out, "{v}");
        }
        Value::I64(v) => {
            let _ = write!(out, "{v}");
        }
        Value::IntervalU64(start, end) => {
            let _ = write!(out, "{start}..{end}");
        }
        Value::IntervalI64(start, end) => {
            let _ = write!(out, "{start}..{end}");
        }
        Value::String(bytes) => {
            out.push('"');
            for c in String::from_utf8_lossy(bytes).chars() {
                let _ = write!(out, "{}", c.escape_debug());
            }
            out.push('"');
        }
        Value::FixedBytes(bytes) => {
            out.push_str("b\"");
            for byte in bytes.as_ref() {
                let _ = write!(out, "{}", byte.escape_ascii());
            }
            out.push('"');
        }
        // A mask value is never a term of a well-formed query (it is only
        // legal inside `Allen`'s mask position); rendered anyway — the
        // statement renderer's format, totality on plain data.
        Value::AllenMask(mask) => {
            let _ = write!(out, "allen({:#015b})", mask.bits());
        }
    }
}

fn var_name(out: &mut String, var: VarId) {
    let _ = write!(out, "v{}", var.0);
}

fn param_name(out: &mut String, param: ParamId) {
    let _ = write!(out, "?{}", param.0);
}

fn relation_name(out: &mut String, schema: &Schema, relation: RelationId) {
    match schema.relation_checked(relation) {
        Some(rel) => out.push_str(rel.name()),
        None => {
            let _ = write!(out, "relation#{}", relation.0);
        }
    }
}

fn field_name(out: &mut String, schema: &Schema, relation: RelationId, field: FieldId) {
    match field_descriptor(schema, relation, field) {
        Some(descriptor) => out.push_str(&descriptor.name),
        None => {
            let _ = write!(out, "field#{}", field.0);
        }
    }
}

fn field_descriptor(
    schema: &Schema,
    relation: RelationId,
    field: FieldId,
) -> Option<&FieldDescriptor> {
    schema
        .relation_checked(relation)?
        .fields()
        .get(usize::from(field.0))
}

#[cfg(test)]
mod tests;
