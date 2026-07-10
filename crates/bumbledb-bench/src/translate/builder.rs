use std::fmt::Write as _;

use bumbledb::ir::{Atom, CmpOp, Comparison, Term};
use bumbledb::schema::ValueType;
use bumbledb::{ParamId, Value};

use super::{Builder, ParamSlot, VarCols};

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

fn sql_u64(value: u64) -> Result<String, String> {
    if value >= 1 << 63 {
        return Err(format!(
            "u64 literal {value} breaks the SQLite mapping axiom"
        ));
    }
    Ok(value.to_string())
}

/// One scalar literal. Interval literals render as their two halves
/// ([`interval_halves`]) — reaching here with one is a translator bug
/// surfaced as an error, never silently-wrong SQL.
fn sql_literal(value: &Value) -> Result<String, String> {
    Ok(match value {
        Value::Bool(v) => u8::from(*v).to_string(),
        Value::Enum(ordinal) => ordinal.to_string(),
        Value::U64(v) => sql_u64(*v)?,
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
        Value::IntervalU64(..) | Value::IntervalI64(..) => {
            return Err("interval literal in a scalar position".to_owned())
        }
        Value::AllenMask(_) => return Err("mask value in a scalar position".to_owned()),
    })
}

/// An interval literal's two halves as SQL integers — the raw typed
/// endpoints (u64 halves under the same `< 2⁶³` axiom as scalar u64).
fn interval_halves(value: &Value) -> Result<(String, String), String> {
    match value {
        Value::IntervalU64(start, end) => Ok((sql_u64(*start)?, sql_u64(*end)?)),
        Value::IntervalI64(start, end) => Ok((start.to_string(), end.to_string())),
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
        CmpOp::Allen { .. } | CmpOp::Contains => {
            unreachable!("interval operators take the endpoint forms")
        }
    }
}

/// One Allen basic's endpoint formula over half-open interval halves —
/// the per-basic SQL the mask disjunction ORs together (kept deliberately
/// naive: correctness first; PRD 15 systematizes the translation).
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

/// A rendered comparison side: one SQL expression for a scalar term, the
/// two half expressions for an interval-typed one.
enum Rendered {
    One(String),
    Pair(String, String),
}

/// The set side of an `Eq`, if any (validation admits sets under `Eq`
/// only, and never on both sides).
fn set_side(comparison: &Comparison) -> Option<(ParamId, &Term)> {
    match (&comparison.lhs, &comparison.rhs) {
        (Term::ParamSet(param), other) | (other, Term::ParamSet(param)) => Some((*param, other)),
        _ => None,
    }
}

impl Builder<'_> {
    fn param_ref(&mut self, slot: ParamSlot) -> String {
        let next = self.params.len();
        let index = *self.param_index.entry(slot).or_insert_with(|| {
            self.params.push(slot);
            next
        });
        format!("?{}", index + 1)
    }

    /// The bound element list of a set param — translation *input*, not a
    /// placeholder: set params are re-rendered per execution.
    fn set_values(&self, param: ParamId) -> Result<&[Value], String> {
        self.sets
            .iter()
            .find(|(id, _)| *id == param)
            .map(|(_, values)| values.as_slice())
            .ok_or_else(|| format!("param set {} has no bound element list", param.0))
    }

    /// `column IN (v1, ..., vk)` with the set rendered as literals —
    /// re-rendered per execution, so prepared-statement parity is not
    /// claimed for set-bound families
    /// (`docs/architecture/60-validation.md` says so). The empty set
    /// renders the honest constant `1 = 0`: `IN ()` is unwritable SQL,
    /// and `IN (NULL)` is the three-valued trap — it yields NULL, not
    /// false, which flips to the wrong answer under negation.
    fn in_list(&self, column: &str, param: ParamId) -> Result<String, String> {
        let values = self.set_values(param)?;
        if values.is_empty() {
            return Ok("1 = 0".to_owned());
        }
        let rendered: Vec<String> = values.iter().map(sql_literal).collect::<Result<_, _>>()?;
        Ok(format!("{column} IN ({})", rendered.join(", ")))
    }

    /// Membership of *any* element of a set in the interval's half
    /// columns — an OR of endpoint tests (`IN` has no interval form);
    /// the empty set renders `1 = 0` exactly as [`Builder::in_list`].
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
        if tests.len() == 1 {
            Ok(tests.into_iter().next().expect("one test"))
        } else {
            Ok(format!("({})", tests.join(" OR ")))
        }
    }

    /// The non-variable arms of a scalar-field binding — one rule for
    /// both polarities (negation is a position, not a kind of atom).
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

    /// The non-variable arms of an interval-field binding: an
    /// interval-typed term is value equality on the halves; an
    /// element-typed term is point membership `start <= t AND t < end`;
    /// a set is membership per element.
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
        let mut out = Vec::new();
        for (field, term) in &atom.bindings {
            let descriptor = &relation.fields()[usize::from(field.0)];
            if matches!(descriptor.value_type, ValueType::Interval { .. }) {
                let start = format!("{alias}.\"{}_start\"", descriptor.name);
                let end = format!("{alias}.\"{}_end\"", descriptor.name);
                match term {
                    Term::Var(var) if self.types.var_is_interval(*var) => {
                        match self.columns.get(var) {
                            Some(VarCols::Interval {
                                start: first_start,
                                end: first_end,
                            }) => {
                                // A repeat occurrence equates the halves
                                // to the first, pairwise.
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
                    // Point membership through a variable: its scalar
                    // anchor may be bound by a later atom — defer.
                    Term::Var(var) => self.deferred.push((start, end, *var)),
                    _ => self.interval_constant(&start, &end, term, &mut out)?,
                }
            } else {
                let column = format!("{alias}.\"{}\"", descriptor.name);
                match term {
                    Term::Var(var) => match self.columns.get(var) {
                        Some(VarCols::Scalar(first)) => {
                            // A later binding (cross-atom or in-atom
                            // repeat) equates to the first.
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
        self.predicates.append(&mut out);
        Ok(())
    }

    /// Flushes the deferred membership tests: every positive atom has
    /// been walked, so each point variable's scalar column exists.
    pub(super) fn flush_deferred(&mut self) -> Result<(), String> {
        for (start, end, var) in std::mem::take(&mut self.deferred) {
            let Some(VarCols::Scalar(column)) = self.columns.get(&var) else {
                return Err(format!(
                    "membership variable {} has no scalar binding",
                    var.0
                ));
            };
            self.predicates.push(format!("{start} <= {column}"));
            self.predicates.push(format!("{column} < {end}"));
        }
        Ok(())
    }

    /// One negated atom as a `NOT EXISTS` correlated subquery appended to
    /// the core's WHERE (`docs/architecture/60-validation.md`). A negated
    /// atom binds nothing (the safety rule), so every variable correlates
    /// to its positive column; the `n{index}` alias space is disjoint
    /// from `t0..`, so self-negation is aliased fresh by construction.
    pub(super) fn negated_atom(&mut self, index: usize, atom: &Atom) -> Result<(), String> {
        let relation = self.schema.relation(atom.relation);
        if atom.bindings.is_empty() {
            // The negated nonemptiness gate: the relation must be empty.
            self.predicates.push(format!(
                "NOT EXISTS (SELECT 1 FROM \"{}\")",
                relation.name()
            ));
            return Ok(());
        }
        let alias = format!("n{index}");
        let mut conjuncts = Vec::new();
        for (field, term) in &atom.bindings {
            let descriptor = &relation.fields()[usize::from(field.0)];
            if matches!(descriptor.value_type, ValueType::Interval { .. }) {
                let start = format!("{alias}.\"{}_start\"", descriptor.name);
                let end = format!("{alias}.\"{}_end\"", descriptor.name);
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
                let column = format!("{alias}.\"{}\"", descriptor.name);
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
        self.predicates.push(format!(
            "NOT EXISTS (SELECT 1 FROM \"{}\" AS {alias} WHERE {})",
            relation.name(),
            conjuncts.join(" AND ")
        ));
        Ok(())
    }

    fn side(&mut self, term: &Term) -> Result<Rendered, String> {
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
        // Eq against a set: "any element" — the literal IN form.
        if matches!(comparison.op, CmpOp::Eq) {
            if let Some((param, other)) = set_side(comparison) {
                let Rendered::One(column) = self.side(other)? else {
                    return Err(format!("param set {} compared to an interval", param.0));
                };
                let rendered = self.in_list(&column, param)?;
                self.predicates.push(rendered);
                return Ok(());
            }
        }
        let lhs = self.side(&comparison.lhs)?;
        let rhs = self.side(&comparison.rhs)?;
        let conjunct = match (comparison.op, lhs, rhs) {
            // Interval value equality is pairwise on the halves; the
            // negation is a disjunction, parenthesized because it joins
            // the WHERE by AND.
            (CmpOp::Eq, Rendered::Pair(ls, le), Rendered::Pair(rs, re)) => {
                format!("{ls} = {rs} AND {le} = {re}")
            }
            (CmpOp::Ne, Rendered::Pair(ls, le), Rendered::Pair(rs, re)) => {
                format!("({ls} <> {rs} OR {le} <> {re})")
            }
            // The Allen mask: per-basic endpoint formulas OR'd — the
            // query's SELECT DISTINCT keeps the disjunction honest
            // (`60-validation.md`; PRD 15 systematizes).
            (CmpOp::Allen { mask }, Rendered::Pair(ls, le), Rendered::Pair(rs, re)) => {
                let bumbledb::MaskTerm::Literal(mask) = mask else {
                    return Err("param masks are not translated (PRD 15)".to_owned());
                };
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
            // Point containment: the membership form.
            (CmpOp::Contains, Rendered::Pair(ls, le), Rendered::One(point)) => {
                format!("{ls} <= {point} AND {point} < {le}")
            }
            (
                op @ (CmpOp::Eq | CmpOp::Ne | CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge),
                Rendered::One(l),
                Rendered::One(r),
            ) => format!("{l} {} {r}", op_sql(op)),
            _ => return Err("comparison mixes interval and scalar operands".to_owned()),
        };
        self.predicates.push(conjunct);
        Ok(())
    }
}
