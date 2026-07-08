use std::fmt::Write as _;

use bumbledb::ir::{Atom, CmpOp, Comparison, Term};
use bumbledb::{ParamId, Value};

use super::Builder;

fn sql_string_literal(raw: &[u8]) -> Result<String, String> {
    let text = std::str::from_utf8(raw).map_err(|_| "non-UTF-8 string literal".to_owned())?;
    // A NUL truncates SQLite's tokenizer mid-statement — the rest of the
    // SQL silently vanishes. The generator's grammar never emits NUL
    // (asserted in querygen's coverage test), so this boundary stays loud
    // instead of buying CAST(X'..' AS TEXT) generality nobody generates.
    if text.contains('\0') {
        return Err("NUL byte in string literal (would truncate the SQL statement)".to_owned());
    }
    Ok(format!("'{}'", text.replace('\'', "''")))
}

fn sql_literal(value: &Value) -> Result<String, String> {
    Ok(match value {
        Value::Bool(v) => u8::from(*v).to_string(),
        Value::Enum(ordinal) => ordinal.to_string(),
        Value::U64(v) => {
            if *v >= 1 << 63 {
                return Err(format!("u64 literal {v} breaks the SQLite mapping axiom"));
            }
            v.to_string()
        }
        Value::I64(v) => v.to_string(),
        Value::String(raw) => sql_string_literal(raw)?,
        Value::Bytes(raw) => {
            let mut hex = String::with_capacity(raw.len() * 2 + 3);
            hex.push_str("X'");
            for b in raw {
                let _ = write!(hex, "{b:02X}");
            }
            hex.push('\'');
            hex
        }
    })
}

fn op_sql(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Eq => "=",
        CmpOp::Ne => "<>",
        CmpOp::Lt => "<",
        CmpOp::Le => "<=",
        CmpOp::Gt => ">",
        CmpOp::Ge => ">=",
    }
}

impl Builder<'_> {
    fn param_ref(&mut self, param: ParamId) -> String {
        let next = self.params.len();
        let index = *self.param_index.entry(param).or_insert_with(|| {
            self.params.push(param);
            next
        });
        format!("?{}", index + 1)
    }

    pub(super) fn atom(&mut self, atom: &Atom) -> Result<(), String> {
        let relation = self.schema.relation(atom.relation);
        if atom.bindings.is_empty() {
            // The nonemptiness gate.
            self.predicates
                .push(format!("EXISTS (SELECT 1 FROM \"{}\")", relation.name()));
            return Ok(());
        }
        let alias = format!("t{}", self.from.len());
        self.from
            .push(format!("\"{}\" AS {alias}", relation.name()));
        for (field, term) in &atom.bindings {
            let column = format!(
                "{alias}.\"{}\"",
                relation.fields()[usize::from(field.0)].name
            );
            match term {
                Term::Var(var) => {
                    if let Some(first) = self.columns.get(var) {
                        // A later binding (cross-atom or in-atom repeat)
                        // equates to the first.
                        self.predicates.push(format!("{first} = {column}"));
                    } else {
                        self.columns.insert(*var, column);
                    }
                }
                Term::Literal(value) => {
                    self.predicates
                        .push(format!("{column} = {}", sql_literal(value)?));
                }
                Term::Param(param) => {
                    let placeholder = self.param_ref(*param);
                    self.predicates.push(format!("{column} = {placeholder}"));
                }
            }
        }
        Ok(())
    }

    fn side(&mut self, term: &Term) -> Result<String, String> {
        match term {
            Term::Var(var) => self
                .columns
                .get(var)
                .cloned()
                .ok_or_else(|| format!("comparison variable {} unbound", var.0)),
            Term::Literal(value) => sql_literal(value),
            Term::Param(param) => Ok(self.param_ref(*param)),
        }
    }

    pub(super) fn comparison(&mut self, comparison: &Comparison) -> Result<(), String> {
        let lhs = self.side(&comparison.lhs)?;
        let rhs = self.side(&comparison.rhs)?;
        self.predicates
            .push(format!("{lhs} {} {rhs}", op_sql(comparison.op)));
        Ok(())
    }
}
