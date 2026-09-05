//! Query rendering back to the rule notation — the statement renderer's
//! sibling (`crate::schema::render`), on the read side of the data
//! surface: **when the write-side query surface is data, the renderer is
//! the pretty syntax**. One rendered block per rule, set-builder shaped:
//! ```text
//! (v0, v1) | Busy(person: v0, during: v1), Allen(v1, INTERSECTS, ?0);
//! ```
//! The grammar is the schema grammar's own query side, promoted
//! :
//! atoms as `Relation(field: var)`, in-atom selections `field == literal`
//! (schema-grammar-verbatim, params admitted as `?N`), `!` negation,
//! membership as `in`, `Allen(term, MASK, term)` with masks as named
//! basics joined by `|` (set union) or the workload composites, `;`
use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::ir::{
    Atom, CmpOp, Comparison, ConditionTree, FindTerm, ParamId, Query, Rule, Term, Value, VarId,
};
use crate::schema::{Enforcement, Relation, Schema};
use bumbledb_theory::allen::AllenMask;
use bumbledb_theory::schema::{FieldDescriptor, FieldId, RelationId};

/// Shared with validation: the closed-reference order refusal (ruled
/// 2026-07-23, R4) resolves its positions through this one table, never a
/// second walk.
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

    /// if any — the R4 refusal's resolution question, and the dense
    pub(crate) fn target(&self, relation: RelationId, field: FieldId) -> Option<RelationId> {
        self.0.get(&(relation, field)).copied()
    }

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
    match query {
        Query {
            interiors,
            rules,
            rec: None,
            ..
        } => {
            render_interiors(&mut out, schema, &refs, interiors);
            render_main(&mut out, schema, &refs, rules);
        }
        Query {
            interiors,
            rec: Some(rec),
            rules,
            ..
        } => {
            render_interiors(&mut out, schema, &refs, interiors);
            let rec_id = crate::ir::InteriorId(u32::try_from(interiors.len()).unwrap_or(u32::MAX));
            for rule in rec.base.iter().map(crate::ir::RecRule::to_rule) {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str("rec");
                render_rule(&mut out, schema, &refs, &rule);
            }
            for step in rec.rec.iter() {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str("rec");
                render_rule(&mut out, schema, &refs, &step.to_written_rule(rec_id));
            }
            render_main(&mut out, schema, &refs, rules);
        }
    }
    out
}

fn render_interiors(
    out: &mut String,
    schema: &Schema,
    refs: &ClosedRefs,
    interiors: &[crate::ir::Interior],
) {
    for (id, interior) in interiors.iter().enumerate() {
        for rule in &interior.rules {
            if !out.is_empty() {
                out.push('\n');
            }
            let _ = write!(out, "interior {id}");
            render_rule(out, schema, refs, rule);
        }
    }
}

fn render_main(out: &mut String, schema: &Schema, refs: &ClosedRefs, rules: &[Rule]) {
    for rule in rules {
        if !out.is_empty() {
            out.push('\n');
        }
        render_rule(out, schema, refs, rule);
    }
}

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

fn find_term(out: &mut String, term: &FindTerm) {
    match term {
        FindTerm::Var(var) => var_name(out, *var),
        FindTerm::Compute(expr) => {
            use std::fmt::Write as _;
            write!(out, "Compute({expr:?})").expect("writing to String");
        }
        FindTerm::Count => out.push_str("Count"),
        FindTerm::Aggregate { op, over } => {
            aggregate(out, *op, *over);
        }
        FindTerm::Pack { over } => {
            out.push_str("Pack(");
            var_name(out, *over);
            out.push(')');
        }
    }
}

fn aggregate(out: &mut String, op: crate::ir::FoldOp, over: crate::ir::VarId) {
    let name = match op {
        crate::ir::FoldOp::Sum => "Sum",
        crate::ir::FoldOp::Mean => "Mean",
        crate::ir::FoldOp::Min => "Min",
        crate::ir::FoldOp::Max => "Max",
    };
    out.push_str(name);
    out.push('(');
    var_name(out, over);
    out.push(')');
}

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
        }
    }
    out.push(')');
    out
}

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

fn term(out: &mut String, term: &Term) {
    match term {
        Term::Var(var) => var_name(out, *var),
        Term::Param(param) | Term::ParamSet(param) => param_name(out, *param),
        Term::Literal(value) => literal(out, value),
    }
}

fn mask_term(out: &mut String, mask: AllenMask) {
    mask_names(out, mask);
}

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
        Value::F64(v) => {
            let _ = write!(out, "f64:0x{:016x}", v.to_bits());
        }
        Value::Id128(id) => {
            // Quoted, matching `query!`'s lexable spelling (`id128:"…"`):
            // a bare 32-hex body starting `0x…`/`0b…` is not one Rust
            // token, so the quoted form is the render-reparse fixed point
            // (P07's recorded request).
            let _ = write!(out, "id128:\"{id}\"");
        }
        Value::IntervalF64(interval) => {
            let _ = write!(
                out,
                "f64:0x{:016x}..f64:0x{:016x}",
                interval.start().to_bits(),
                interval.end().to_bits()
            );
        }
        Value::IntervalU64(interval) => {
            let _ = write!(out, "{}..{}", interval.start(), interval.end());
        }
        Value::IntervalI64(interval) => {
            let _ = write!(out, "{}..{}", interval.start(), interval.end());
        }
        Value::String(text) => {
            out.push('"');
            for c in text.chars() {
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

/// An atom source: the relation's name for `Edb`; `interior {id}` for
/// `Interior` (the same spelling the rule prefix already emits —
/// interior names are a text-layer sidecar the IR never carries; the
/// macro's names resolve locally and lower to bare `InteriorId`s).
fn source_name(out: &mut String, schema: &Schema, source: crate::ir::AtomSource) {
    match source {
        crate::ir::AtomSource::Edb(relation) => relation_name(out, schema, relation),
        crate::ir::AtomSource::Interior(id) => {
            let _ = write!(out, "interior {}", id.0);
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
