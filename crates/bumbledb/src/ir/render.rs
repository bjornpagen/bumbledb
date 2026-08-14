//! Query rendering back to the rule notation — the statement renderer's
//! sibling (`crate::schema::render`), on the read side of the data
//! surface: **when the write-side query surface is data, the renderer is
//! the pretty syntax** (`docs/architecture/20-query-ir.md` § the data
//! surface). One rendered block per rule, set-builder shaped:
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
//! terminating each rule exactly as it terminates statements.
//!
//! Deterministic and **total on plain data** — its consumers are
//! diagnostics (roster errors print the offending query, so malformed
//! shapes must render, not panic) and the introspection/stats surface:
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
//! - the remaining folds render as `Sum`/`Min`/`Max`/`Count`/`Pack`;
//! - a nested condition tree renders in the notation's own `and(..)` /
//!   `or(..)` forms — grammar, not merely diagnostics (ruled 2026-07-23,
//!   R9): `query!` parses them back, so the render→parse round trip
//!   closes over the full input grammar. Depth past
//!   [`crate::ir::MAX_CONDITION_DEPTH`] elides to `...` — the hostile
//!   nesting validation rejects must still render.
//!
//! Rendering allocates; it runs only in diagnostic contexts (roster
//! errors, introspection, arbitration bundles), never on a warm path.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::ir::{
    AggOp, Atom, CmpOp, Comparison, ConditionTree, FindTerm, ParamId, Query, Rule, Term, Value,
    VarId,
};
use crate::schema::{Enforcement, Relation, Schema};
use bumbledb_theory::allen::AllenMask;
use bumbledb_theory::schema::{FieldDescriptor, FieldId, RelationId};

/// The closed-reference position table, built once at renderer
/// construction: `(relation, field)` → the closed relation whose row ids
/// the field's words are. A schema walk over declared containments whose
/// target is a closed relation's id and whose source projection is that
/// single field — the same inference the grounding's complement fold runs
/// (`plan/ground/evaluate.rs::containment_into_id`) — plus each closed
/// relation's own id field, which maps to itself. Shared with validation:
/// the closed-reference order refusal (ruled 2026-07-23, R4) resolves
/// its positions through this one table, never a second walk.
pub(crate) struct ClosedRefs(BTreeMap<(RelationId, FieldId), RelationId>);

impl ClosedRefs {
    pub(crate) fn build(schema: &Schema) -> Self {
        let mut map = BTreeMap::new();
        for statement in schema.containments() {
            if !matches!(statement.enforcement, Enforcement::Closed { .. }) {
                continue;
            }
            let source = &statement.source;
            let target = &statement.target;
            let target_closed = schema
                .relation_checked(target.relation)
                .is_some_and(|rel| rel.body().closed_rows().is_some());
            if target_closed
                && target.projection.as_ref() == [FieldId(0)]
                && let [field] = source.projection.as_ref()
            {
                map.insert((source.relation, *field), target.relation);
            }
        }
        for (index, relation) in schema.relations().iter().enumerate() {
            if relation.body().closed_rows().is_some() {
                let id = RelationId(u32::try_from(index).expect("relation count fits u32"));
                map.insert((id, FieldId(0)), id);
            }
        }
        Self(map)
    }

    /// The closed relation a `(relation, field)` position references,
    /// if any — the R4 refusal's resolution question, and the dense
    /// group domain's source (finding 049: the sealed extension's row
    /// count is the radix).
    pub(crate) fn target(&self, relation: RelationId, field: FieldId) -> Option<RelationId> {
        self.0.get(&(relation, field)).copied()
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
        let rows = schema.relation_checked(closed)?.body().closed_rows()?;
        match usize::try_from(*word).ok().and_then(|row| rows.get(row)) {
            Some(row) => Some(row.handle.to_string()),
            None => Some(format!(
                "{}({word}?)",
                schema.relation_checked(closed).map_or("?", Relation::name)
            )),
        }
    }
}

/// Renders a query in the rule notation, one block per rule, newline-
/// separated, each rule `;`-terminated. Deterministic (two calls yield
/// one string) and total: malformed queries render with placeholder
/// names — this is the diagnostic surface for the roster's rejections.
#[must_use]
pub fn render(schema: &Schema, query: &Query) -> String {
    let refs = ClosedRefs::build(schema);
    let mut out = String::new();
    for (id, interior) in query.interiors.iter().enumerate() {
        for rule in &interior.rules {
            if !out.is_empty() {
                out.push('\n');
            }
            let _ = write!(out, "interior {id}");
            render_rule(&mut out, schema, &refs, rule);
        }
    }
    if let Some(rec) = &query.rec {
        for rule in rec.base.iter().chain(&rec.rec) {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str("rec");
            render_rule(&mut out, schema, &refs, rule);
        }
    }
    for rule in &query.rules {
        if !out.is_empty() {
            out.push('\n');
        }
        render_rule(&mut out, schema, &refs, rule);
    }
    out
}

/// One rule as `(head) | body;`.
fn render_rule(out: &mut String, schema: &Schema, refs: &ClosedRefs, rule: &Rule) {
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
    for tree in &rule.conditions {
        items.push(tree_item(tree));
    }
    if !items.is_empty() {
        out.push(' ');
        out.push_str(&items.join(", "));
    }
    out.push(';');
}

/// One head position of a rule. `Count` is nullary; a malformed
/// `Count(v)` renders its variable anyway (totality over the shapes the
/// roster rejects).
fn find_term(out: &mut String, term: &FindTerm) {
    match term {
        FindTerm::Var(var) => var_name(out, *var),
        FindTerm::Measure(var) => {
            out.push_str("Duration(");
            var_name(out, *var);
            out.push(')');
        }
        FindTerm::Aggregate { op, over } => aggregate(out, *op, *over, false),
        FindTerm::AggregateMeasure { op, over } => aggregate(out, *op, Some(*over), true),
    }
}

/// One aggregate head term: `Sum(v0)`, `Count`, `Pack(v1)`,
/// `Sum(Duration(v0))`.
fn aggregate(out: &mut String, op: AggOp, over: Option<VarId>, measure: bool) {
    let name = match op {
        AggOp::Sum => "Sum",
        AggOp::Min => "Min",
        AggOp::Max => "Max",
        AggOp::Count => "Count",
        AggOp::Pack => "Pack",
    };
    out.push_str(name);
    if over.is_none() {
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
/// A predicate atom whose bindings are dense, in-order, and
/// variable-only renders in the ordered bare form (`p0(v1, v2)`) — the
/// notation's one dense spelling; sparse positions and selections keep
/// the indexed `i:`/selection spellings.
fn atom_item(schema: &Schema, refs: &ClosedRefs, atom: &Atom, negated: bool) -> String {
    let mut out = String::new();
    if negated {
        out.push('!');
    }
    source_name(&mut out, schema, atom.source);
    out.push('(');
    let ordered_dense = matches!(atom.source, crate::ir::AtomSource::Interior(_))
        && atom
            .bindings
            .iter()
            .enumerate()
            .all(|(index, (field, term))| {
                usize::from(field.0) == index && matches!(term, Term::Var(_))
            });
    for (index, (field, term)) in atom.bindings.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        if ordered_dense && let Term::Var(var) = term {
            var_name(&mut out, *var);
            continue;
        }
        source_field_name(&mut out, schema, atom.source, *field);
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
                match atom
                    .source
                    .edb()
                    .and_then(|relation| refs.handle(schema, relation, *field, value))
                {
                    Some(handle) => out.push_str(&handle),
                    None => literal(&mut out, value),
                }
            }
            // Rejected by validation (`DurationInBinding`); rendered
            // anyway — the diagnostic pictures the mistake.
            Term::Measure(var) => {
                out.push_str(" == Duration(");
                var_name(&mut out, *var);
                out.push(')');
            }
        }
    }
    out.push(')');
    out
}

/// One condition tree: a leaf is a comparison item; `And`/`Or` render
/// in the notation's own functional forms (grammar — ruled 2026-07-23,
/// R9). Depth-budgeted at [`crate::ir::MAX_CONDITION_DEPTH`]:
/// the renderer recurses by depth and must stay total on the hostile
/// nesting validation rejects, so anything past the boundary check's own
/// cap elides to `...` instead of exhausting the stack.
fn tree_item(tree: &ConditionTree) -> String {
    tree_item_within(tree, crate::ir::MAX_CONDITION_DEPTH)
}

fn tree_item_within(tree: &ConditionTree, budget: usize) -> String {
    if budget == 0 {
        return "...".to_owned();
    }
    match tree {
        ConditionTree::Leaf(cmp) => comparison(cmp),
        ConditionTree::And(children) => functional("and", children, budget),
        ConditionTree::Or(children) => functional("or", children, budget),
    }
}

fn functional(name: &str, children: &[ConditionTree], budget: usize) -> String {
    let inner: Vec<String> = children
        .iter()
        .map(|child| tree_item_within(child, budget - 1))
        .collect();
    format!("{name}({})", inner.join(", "))
}

/// One comparison item: `Allen(a, MASK, b)`, membership `point in
/// interval` (the `PointIn` predicate's notation), or infix `lhs op rhs`.
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
        // `PointIn(interval, point)` is point membership as a predicate:
        // the notation reads point-first.
        CmpOp::PointIn => {
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
                CmpOp::Allen { .. } | CmpOp::PointIn => unreachable!("matched above"),
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
        Term::Measure(var) => {
            out.push_str("Duration(");
            var_name(out, *var);
            out.push(')');
        }
    }
}

/// The mask position: a named mask expression.
fn mask_term(out: &mut String, mask: AllenMask) {
    mask_names(out, mask);
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
        Value::IntervalU64(interval) => {
            let _ = write!(out, "{}..{}", interval.start(), interval.end());
        }
        Value::IntervalI64(interval) => {
            let _ = write!(out, "{}..{}", interval.start(), interval.end());
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
    }
}

fn var_name(out: &mut String, var: VarId) {
    let _ = write!(out, "v{}", var.0);
}

fn param_name(out: &mut String, param: ParamId) {
    let _ = write!(out, "?{}", param.0);
}

/// An atom source: the relation's name for `Edb`; the synthesized
/// `p{id}` for `Interior` (the `v{id}`/`?{id}` convention extended —
/// interior names are a text-layer sidecar the IR never carries; the
/// macro's names resolve locally and lower to bare `InteriorId`s).
fn source_name(out: &mut String, schema: &Schema, source: crate::ir::AtomSource) {
    match source {
        crate::ir::AtomSource::Edb(relation) => relation_name(out, schema, relation),
        crate::ir::AtomSource::Interior(pred) => {
            let _ = write!(out, "p{}", pred.0);
        }
    }
}

/// A binding's field position: the schema name for `Edb`; the numeric
/// head position for `Interior` (`FieldId(i)` addresses the target
/// table's column `i` — positional, never nominal; the indexed
/// spelling is sparse/selection's — a dense in-order variable-only atom
/// renders bare in [`atom_item`], the ordered form).
fn source_field_name(
    out: &mut String,
    schema: &Schema,
    source: crate::ir::AtomSource,
    field: FieldId,
) {
    match source {
        crate::ir::AtomSource::Edb(relation) => field_name(out, schema, relation, field),
        crate::ir::AtomSource::Interior(_) => {
            let _ = write!(out, "{}", field.0);
        }
    }
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
