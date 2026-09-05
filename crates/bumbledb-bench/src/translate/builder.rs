use std::fmt::Write as _;

use bumbledb::ir::{Atom, CmpOp, Comparison, Term};
use bumbledb::{ParamId, Value};

use super::{Builder, ParamSlot, VarCols};

fn sql_string_literal(text: &str) -> Result<String, String> {
    if text.contains('\0') {
        return Err("NUL byte in string literal (would truncate the SQL statement)".to_owned());
    }
    Ok(format!("'{}'", text.replace('\'', "''")))
}

fn sql_u64(value: u64) -> Result<String, String> {
    if value >= 1 << 63 {
        return Err(format!(
            "u64 literal {value} breaks the SQLite mapping axiom"
        ));
    }
    Ok(value.to_string())
}

fn sql_literal(value: &Value) -> Result<String, String> {
    Ok(match value {
        Value::Bool(v) => u8::from(*v).to_string(),
        Value::U64(v) => sql_u64(*v)?,
        Value::I64(v) => v.to_string(),
        Value::F64(v) => crate::float::sql_literal(*v),
        Value::String(text) => sql_string_literal(text)?,
        Value::FixedBytes(raw) => {
            let mut hex = String::with_capacity(raw.len() * 2 + 3);
            hex.push_str("X'");
            for b in raw {
                let _ = write!(hex, "{b:02X}");
            }
            hex.push('\'');
            hex
        }
        Value::IntervalU64(..) | Value::IntervalI64(..) => {
            return Err("interval literal in a scalar position".to_owned());
        }
    })
}

fn interval_halves(value: &Value) -> Result<(String, String), String> {
    match value {
        Value::IntervalU64(interval) => Ok((sql_u64(interval.start())?, sql_u64(interval.end())?)),
        Value::IntervalI64(interval) => {
            Ok((interval.start().to_string(), interval.end().to_string()))
        }
        _ => Err("scalar literal in an interval position".to_owned()),
    }
}

fn op_sql(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Eq => "=",
        CmpOp::Ne => "<>",
        CmpOp::Lt => "<",
        CmpOp::Le => "<=",
        CmpOp::Gt => ">",
        CmpOp::Ge => ">=",
        CmpOp::Allen { .. } | CmpOp::PointIn => {
            unreachable!("interval operators take the endpoint forms")
        }
    }
}

fn basic_sql(basic: bumbledb::Basic, ls: &str, le: &str, rs: &str, re: &str) -> String {
    use bumbledb::Basic;
    match basic {
        Basic::Before => format!("{le} < {rs}"),
        Basic::Meets => format!("{le} = {rs}"),
        Basic::Overlaps => format!("{ls} < {rs} AND {rs} < {le} AND {le} < {re}"),
        Basic::Starts => format!("{ls} = {rs} AND {le} < {re}"),
        Basic::During => format!("{rs} < {ls} AND {le} < {re}"),
        Basic::Finishes => format!("{rs} < {ls} AND {le} = {re}"),
        Basic::Equals => format!("{ls} = {rs} AND {le} = {re}"),
        Basic::FinishedBy => format!("{ls} < {rs} AND {le} = {re}"),
        Basic::Contains => format!("{ls} < {rs} AND {re} < {le}"),
        Basic::StartedBy => format!("{ls} = {rs} AND {re} < {le}"),
        Basic::OverlappedBy => format!("{rs} < {ls} AND {ls} < {re} AND {re} < {le}"),
        Basic::MetBy => format!("{re} = {ls}"),
        Basic::After => format!("{re} < {ls}"),
    }
}

enum Rendered {
    One(String),
    Pair(String, String),
}

fn set_side(comparison: &Comparison) -> Option<(ParamId, &Term)> {
    match (&comparison.lhs, &comparison.rhs) {
        (Term::ParamSet(param), other) | (other, Term::ParamSet(param)) => Some((*param, other)),
        _ => None,
    }
}

impl Builder<'_> {
    fn source_table(&self, atom: &Atom) -> String {
        match atom.source {
            bumbledb::AtomSource::Edb(relation) => self.schema.relation(relation).name().to_owned(),
            bumbledb::AtomSource::Interior(id) => super::derived_cte_name(id, self.shape),
        }
    }

    /// columns are refused before any rule renders).
    fn source_column(&self, atom: &Atom, field: bumbledb::FieldId) -> (String, bool) {
        match atom.source {
            bumbledb::AtomSource::Edb(relation) => {
                let descriptor = &self.schema.relation(relation).fields()[usize::from(field.0)];
                (
                    descriptor.name.to_string(),
                    descriptor.value_type.is_interval(),
                )
            }
            bumbledb::AtomSource::Interior(_) => (format!("c{}", field.0), false),
        }
    }

    fn param_ref(&mut self, slot: ParamSlot) -> String {
        let next = self.params.len();
        let index = *self.param_index.entry(slot).or_insert_with(|| {
            self.params.push(slot);
            next
        });
        format!("?{}", index + 1)
    }

    fn set_values(&self, param: ParamId) -> Result<&[Value], String> {
        self.sets
            .iter()
            .find(|(id, _)| *id == param)
            .map(|(_, values)| values.as_slice())
            .ok_or_else(|| format!("param set {} has no bound element list", param.0))
    }

    fn in_list(&self, column: &str, param: ParamId) -> Result<String, String> {
        let values = self.set_values(param)?;
        if values.is_empty() {
            return Ok("1 = 0".to_owned());
        }
        let rendered: Vec<String> = values.iter().map(sql_literal).collect::<Result<_, _>>()?;
        Ok(format!("{column} IN ({})", rendered.join(", ")))
    }

    fn set_membership(&self, start: &str, end: &str, param: ParamId) -> Result<String, String> {
        let values = self.set_values(param)?;
        if values.is_empty() {
            return Ok("1 = 0".to_owned());
        }
        let tests: Vec<String> = values
            .iter()
            .map(|value| {
                let point = sql_literal(value)?;
                Ok(format!("{start} <= {point} AND {point} < {end}"))
            })
            .collect::<Result<_, String>>()?;

        Ok(format!("({})", tests.join(" OR ")))
    }

    fn scalar_constant(
        &mut self,
        column: &str,
        term: &Term,
        out: &mut Vec<String>,
    ) -> Result<(), String> {
        match term {
            Term::Literal(value) => out.push(format!("{column} = {}", sql_literal(value)?)),
            Term::Param(param) => {
                let placeholder = self.param_ref(ParamSlot::Whole(*param));
                out.push(format!("{column} = {placeholder}"));
            }
            Term::ParamSet(param) => out.push(self.in_list(column, *param)?),
            Term::Var(_) => unreachable!("variable arms are polarity-specific"),
        }
        Ok(())
    }

    fn interval_constant(
        &mut self,
        start: &str,
        end: &str,
        term: &Term,
        out: &mut Vec<String>,
    ) -> Result<(), String> {
        match term {
            Term::Param(param) if self.types.param_is_interval(*param) => {
                let start_ref = self.param_ref(ParamSlot::Start(*param));
                let end_ref = self.param_ref(ParamSlot::End(*param));
                out.push(format!("{start} = {start_ref}"));
                out.push(format!("{end} = {end_ref}"));
            }
            Term::Param(param) => {
                let placeholder = self.param_ref(ParamSlot::Whole(*param));
                out.push(format!("{start} <= {placeholder}"));
                out.push(format!("{placeholder} < {end}"));
            }
            Term::Literal(value @ (Value::IntervalU64(..) | Value::IntervalI64(..))) => {
                let (start_lit, end_lit) = interval_halves(value)?;
                out.push(format!("{start} = {start_lit}"));
                out.push(format!("{end} = {end_lit}"));
            }
            Term::Literal(value) => {
                let point = sql_literal(value)?;
                out.push(format!("{start} <= {point}"));
                out.push(format!("{point} < {end}"));
            }
            Term::ParamSet(param) => out.push(self.set_membership(start, end, *param)?),
            Term::Var(_) => unreachable!("variable arms are polarity-specific"),
        }
        Ok(())
    }

    pub(super) fn render_atom(&mut self, atom: &Atom) -> Result<(), String> {
        let table = self.source_table(atom);
        if atom.bindings.is_empty() {
            self.conditions
                .push(format!("EXISTS (SELECT 1 FROM \"{table}\")"));
            return Ok(());
        }
        let alias = format!("t{}", self.from.len());
        self.from.push(format!("\"{table}\" AS {alias}"));
        let mut out = Vec::new();
        for (field, term) in &atom.bindings {
            let (name, interval_field) = self.source_column(atom, *field);
            if interval_field {
                let start = format!("{alias}.\"{name}_start\"");
                let end = format!("{alias}.\"{name}_end\"");
                match term {
                    Term::Var(var) if self.types.var_is_interval(*var) => {
                        match self.columns.get(var) {
                            Some(VarCols::Interval {
                                start: first_start,
                                end: first_end,
                            }) => {
                                out.push(format!("{first_start} = {start}"));
                                out.push(format!("{first_end} = {end}"));
                            }
                            Some(VarCols::Scalar(_)) => {
                                return Err(format!(
                                    "variable {} bound as both interval and scalar",
                                    var.0
                                ));
                            }
                            None => {
                                self.columns.insert(*var, VarCols::Interval { start, end });
                            }
                        }
                    }

                    Term::Var(var) => self.deferred.push((start, end, *var)),
                    _ => self.interval_constant(&start, &end, term, &mut out)?,
                }
            } else {
                let column = format!("{alias}.\"{name}\"");
                match term {
                    Term::Var(var) => match self.columns.get(var) {
                        Some(VarCols::Scalar(first)) => {
                            out.push(format!("{first} = {column}"));
                        }
                        Some(VarCols::Interval { .. }) => {
                            return Err(format!(
                                "variable {} bound as both interval and scalar",
                                var.0
                            ));
                        }
                        None => {
                            self.columns.insert(*var, VarCols::Scalar(column));
                        }
                    },
                    _ => self.scalar_constant(&column, term, &mut out)?,
                }
            }
        }
        self.conditions.append(&mut out);
        Ok(())
    }

    pub(super) fn flush_deferred(&mut self) -> Result<(), String> {
        for (start, end, var) in std::mem::take(&mut self.deferred) {
            let Some(VarCols::Scalar(column)) = self.columns.get(&var) else {
                return Err(format!(
                    "membership variable {} has no scalar binding",
                    var.0
                ));
            };
            self.conditions.push(format!("{start} <= {column}"));
            self.conditions.push(format!("{column} < {end}"));
        }
        Ok(())
    }

    pub(super) fn negated_atom(&mut self, index: usize, atom: &Atom) -> Result<(), String> {
        let table = self.source_table(atom);
        if atom.bindings.is_empty() {
            self.conditions
                .push(format!("NOT EXISTS (SELECT 1 FROM \"{table}\")"));
            return Ok(());
        }
        let alias = format!("n{index}");
        let mut conjuncts = Vec::new();
        for (field, term) in &atom.bindings {
            let (name, interval_field) = self.source_column(atom, *field);
            if interval_field {
                let start = format!("{alias}.\"{name}_start\"");
                let end = format!("{alias}.\"{name}_end\"");
                match term {
                    Term::Var(var) => match self.columns.get(var) {
                        Some(VarCols::Interval {
                            start: outer_start,
                            end: outer_end,
                        }) => {
                            conjuncts.push(format!("{start} = {outer_start}"));
                            conjuncts.push(format!("{end} = {outer_end}"));
                        }
                        Some(VarCols::Scalar(column)) => {
                            conjuncts.push(format!("{start} <= {column}"));
                            conjuncts.push(format!("{column} < {end}"));
                        }
                        None => {
                            return Err(format!("negated-atom variable {} unbound", var.0));
                        }
                    },
                    _ => self.interval_constant(&start, &end, term, &mut conjuncts)?,
                }
            } else {
                let column = format!("{alias}.\"{name}\"");
                match term {
                    Term::Var(var) => match self.columns.get(var) {
                        Some(VarCols::Scalar(outer)) => {
                            conjuncts.push(format!("{column} = {outer}"));
                        }
                        Some(VarCols::Interval { .. }) => {
                            return Err(format!(
                                "variable {} bound as both interval and scalar",
                                var.0
                            ));
                        }
                        None => {
                            return Err(format!("negated-atom variable {} unbound", var.0));
                        }
                    },
                    _ => self.scalar_constant(&column, term, &mut conjuncts)?,
                }
            }
        }
        self.conditions.push(format!(
            "NOT EXISTS (SELECT 1 FROM \"{table}\" AS {alias} WHERE {})",
            conjuncts.join(" AND ")
        ));
        Ok(())
    }

    fn render_term(&mut self, term: &Term) -> Result<Rendered, String> {
        match term {
            Term::Var(var) => match self.columns.get(var) {
                Some(VarCols::Scalar(column)) => Ok(Rendered::One(column.clone())),
                Some(VarCols::Interval { start, end }) => {
                    Ok(Rendered::Pair(start.clone(), end.clone()))
                }
                None => Err(format!("comparison variable {} unbound", var.0)),
            },
            Term::Literal(value @ (Value::IntervalU64(..) | Value::IntervalI64(..))) => {
                let (start, end) = interval_halves(value)?;
                Ok(Rendered::Pair(start, end))
            }
            Term::Literal(value) => Ok(Rendered::One(sql_literal(value)?)),
            Term::Param(param) if self.types.param_is_interval(*param) => {
                let start = self.param_ref(ParamSlot::Start(*param));
                let end = self.param_ref(ParamSlot::End(*param));
                Ok(Rendered::Pair(start, end))
            }
            Term::Param(param) => Ok(Rendered::One(self.param_ref(ParamSlot::Whole(*param)))),
            Term::ParamSet(param) => Err(format!("param set {} outside Eq", param.0)),
        }
    }

    pub(super) fn comparison(&mut self, comparison: &Comparison) -> Result<(), String> {
        if matches!(comparison.op, CmpOp::Eq)
            && let Some((param, other)) = set_side(comparison)
        {
            let Rendered::One(column) = self.render_term(other)? else {
                return Err(format!("param set {} compared to an interval", param.0));
            };
            let rendered = self.in_list(&column, param)?;
            self.conditions.push(rendered);
            return Ok(());
        }
        let lhs = self.render_term(&comparison.lhs)?;
        let rhs = self.render_term(&comparison.rhs)?;
        let conjunct = match (comparison.op, lhs, rhs) {
            (CmpOp::Eq, Rendered::Pair(ls, le), Rendered::Pair(rs, re)) => {
                format!("{ls} = {rs} AND {le} = {re}")
            }
            (CmpOp::Ne, Rendered::Pair(ls, le), Rendered::Pair(rs, re)) => {
                format!("({ls} <> {rs} OR {le} <> {re})")
            }

            (CmpOp::Allen { mask }, Rendered::Pair(ls, le), Rendered::Pair(rs, re)) => {
                let arms: Vec<String> = bumbledb::Basic::ALL
                    .iter()
                    .filter(|basic| mask.contains(**basic))
                    .map(|basic| format!("({})", basic_sql(*basic, &ls, &le, &rs, &re)))
                    .collect();
                if arms.is_empty() {
                    return Err("empty Allen mask reached translation".to_owned());
                }
                format!("({})", arms.join(" OR "))
            }

            (CmpOp::PointIn, Rendered::Pair(ls, le), Rendered::One(point)) => {
                format!("{ls} <= {point} AND {point} < {le}")
            }
            (
                op @ (CmpOp::Eq | CmpOp::Ne | CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge),
                Rendered::One(l),
                Rendered::One(r),
            ) => format!("{l} {} {r}", op_sql(op)),
            _ => return Err("comparison mixes interval and scalar operands".to_owned()),
        };
        self.conditions.push(conjunct);
        Ok(())
    }
}
