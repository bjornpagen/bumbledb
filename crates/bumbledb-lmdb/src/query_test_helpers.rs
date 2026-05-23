fn seeded_db() -> Result<(Environment, StorageSchema)> {
    let dir = tempfile::tempdir().map_err(|error| Error::io("tempdir", error))?;
    let path = dir.keep();
    let env = Environment::open(&path)?;
    let schema = StorageSchema::new(ledger_schema(), env.max_key_size())?;
    let facts = seeded_facts();
    env.write(|txn| {
        for fact in &facts {
            txn.insert(&schema, fact.clone())?;
        }
        Ok::<(), Error>(())
    })?;
    Ok((env, schema))
}

fn q24_like_join_schema() -> bumbledb_core::schema::SchemaDescriptor {
    bumbledb_core::schema::SchemaDescriptor::new(
        "Q24LikeJoinDb",
        vec![
            RelationDescriptor::new(
                "Alias",
                vec![FieldDescriptor::new("person", ValueType::U64)],
            )
            .with_unique("person", ["person"]),
            RelationDescriptor::new(
                "Character",
                vec![FieldDescriptor::new("id", ValueType::U64)],
            )
            .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "Appearance",
                vec![
                    FieldDescriptor::new("person", ValueType::U64),
                    FieldDescriptor::new("work", ValueType::U64),
                    FieldDescriptor::new("character", ValueType::U64),
                    FieldDescriptor::new("role", ValueType::U64),
                ],
            )
            .with_unique("person_work_role", ["person", "work", "role", "character"])
            .with_index(IndexDescriptor::equality(
                "by_role_work",
                ["role", "work", "person", "character"],
            )),
            RelationDescriptor::new(
                "Company",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("country", ValueType::String),
                ],
            )
            .with_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_country", ["country", "id"])),
            RelationDescriptor::new(
                "Keyword",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("word", ValueType::String),
                ],
            )
            .with_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_word", ["word", "id"])),
            RelationDescriptor::new(
                "WorkCompany",
                vec![
                    FieldDescriptor::new("work", ValueType::U64),
                    FieldDescriptor::new("company", ValueType::U64),
                ],
            )
            .with_unique("work_company", ["work", "company"])
            .with_index(IndexDescriptor::equality("by_company", ["company", "work"])),
            RelationDescriptor::new(
                "WorkKeyword",
                vec![
                    FieldDescriptor::new("work", ValueType::U64),
                    FieldDescriptor::new("keyword", ValueType::U64),
                ],
            )
            .with_unique("work_keyword", ["work", "keyword"])
            .with_index(IndexDescriptor::equality("by_keyword", ["keyword", "work"])),
            RelationDescriptor::new(
                "Person",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("gender", ValueType::String),
                ],
            )
            .with_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_gender", ["gender", "id"])),
            RelationDescriptor::new(
                "Role",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("name", ValueType::String),
                ],
            )
            .with_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_name", ["name", "id"])),
            RelationDescriptor::new(
                "Title",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("year", ValueType::I64),
                ],
            )
            .with_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_year", ["year", "id"])),
        ],
    )
}

fn seeded_facts() -> Vec<Fact> {
    vec![
        holder_fact(1, "Alice"),
        holder_fact(2, "Bob"),
        account_fact(1, 1, 1),
        account_fact(2, 1, 2),
        account_fact(3, 2, 1),
        posting_fact(1, 1, 100, 10),
        posting_fact(2, 1, 200, 20),
        posting_fact(3, 2, 300, 30),
    ]
}

fn ledger_schema() -> bumbledb_core::schema::SchemaDescriptor {
    bumbledb_core::schema::SchemaDescriptor::new(
        "LedgerDb",
        vec![
            RelationDescriptor::new(
                "Holder",
                vec![
                    FieldDescriptor::new(
                        "id",
                        ValueType::Serial {
                            type_name: "HolderId".to_owned(),
                            owning_relation: "Holder".to_owned(),
                        },
                    ),
                    FieldDescriptor::new("name", ValueType::String),
                ],
            )
            .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "Account",
                vec![
                    FieldDescriptor::new(
                        "id",
                        ValueType::Serial {
                            type_name: "AccountId".to_owned(),
                            owning_relation: "Account".to_owned(),
                        },
                    ),
                    FieldDescriptor::new(
                        "holder",
                        ValueType::Serial {
                            type_name: "HolderId".to_owned(),
                            owning_relation: "Holder".to_owned(),
                        },
                    ),
                    FieldDescriptor::new(
                        "currency",
                        ValueType::Enum {
                            name: "Currency".to_owned(),
                        },
                    ),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "holder",
                ["holder"],
                "Holder",
                "id",
            )),
            RelationDescriptor::new(
                "Posting",
                vec![
                    FieldDescriptor::new(
                        "id",
                        ValueType::Serial {
                            type_name: "PostingId".to_owned(),
                            owning_relation: "Posting".to_owned(),
                        },
                    ),
                    FieldDescriptor::new(
                        "account",
                        ValueType::Serial {
                            type_name: "AccountId".to_owned(),
                            owning_relation: "Account".to_owned(),
                        },
                    ),
                    FieldDescriptor::new("amount", ValueType::Decimal { scale: 4 }),
                    FieldDescriptor::new("at", ValueType::TimestampMicros).range_indexed(),
                ],
            )
            .with_unique("id", ["id"])
            .with_constraint(ConstraintDescriptor::foreign_key(
                "account",
                ["account"],
                "Account",
                "id",
            )),
        ],
    )
    .with_enum(bumbledb_core::schema::EnumDescriptor::codes(
        "Currency",
        [1, 2],
    ))
}

fn variable_order_schema() -> bumbledb_core::schema::SchemaDescriptor {
    bumbledb_core::schema::SchemaDescriptor::new(
        "VariableOrderDb",
        vec![
            RelationDescriptor::new(
                "Item",
                vec![
                    FieldDescriptor::new(
                        "id",
                        ValueType::Serial {
                            type_name: "ItemId".to_owned(),
                            owning_relation: "Item".to_owned(),
                        },
                    ),
                    FieldDescriptor::new(
                        "kind",
                        ValueType::Enum {
                            name: "Kind".to_owned(),
                        },
                    ),
                ],
            )
            .with_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_kind", ["kind", "id"])),
        ],
    )
    .with_enum(bumbledb_core::schema::EnumDescriptor::codes("Kind", [1, 2]))
}

fn triangle_schema() -> bumbledb_core::schema::SchemaDescriptor {
    bumbledb_core::schema::SchemaDescriptor::new(
        "TriangleDb",
        vec![
            RelationDescriptor::new(
                "EdgeAB",
                vec![
                    FieldDescriptor::new("a", ValueType::U64),
                    FieldDescriptor::new("b", ValueType::U64),
                ],
            )
            .with_unique("a_b", ["a", "b"]),
            RelationDescriptor::new(
                "EdgeAC",
                vec![
                    FieldDescriptor::new("a", ValueType::U64),
                    FieldDescriptor::new("c", ValueType::U64),
                ],
            )
            .with_unique("a_c", ["a", "c"]),
            RelationDescriptor::new(
                "EdgeBC",
                vec![
                    FieldDescriptor::new("b", ValueType::U64),
                    FieldDescriptor::new("c", ValueType::U64),
                ],
            )
            .with_unique("b_c", ["b", "c"]),
        ],
    )
}

fn chain_schema() -> bumbledb_core::schema::SchemaDescriptor {
    bumbledb_core::schema::SchemaDescriptor::new(
        "ChainDb",
        vec![
            RelationDescriptor::new("A", vec![FieldDescriptor::new("id", ValueType::U64)])
                .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "B",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("a", ValueType::U64),
                ],
            )
            .with_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_a", ["a", "id"])),
        ],
    )
}

fn reserve_schema() -> bumbledb_core::schema::SchemaDescriptor {
    bumbledb_core::schema::SchemaDescriptor::new(
        "ReserveDb",
        vec![
            RelationDescriptor::new(
                "Reserve",
                vec![
                    FieldDescriptor::new("sailor", ValueType::U64),
                    FieldDescriptor::new("boat", ValueType::U64),
                    FieldDescriptor::new("day", ValueType::TimestampMicros).range_indexed(),
                ],
            )
            .with_unique("sailor_boat_day", ["sailor", "boat", "day"]),
        ],
    )
}

fn chain4_schema() -> bumbledb_core::schema::SchemaDescriptor {
    bumbledb_core::schema::SchemaDescriptor::new(
        "Chain4Db",
        vec![
            RelationDescriptor::new("A", vec![FieldDescriptor::new("id", ValueType::U64)])
                .with_unique("id", ["id"]),
            RelationDescriptor::new(
                "B",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("a", ValueType::U64),
                ],
            )
            .with_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_a", ["a", "id"])),
            RelationDescriptor::new(
                "C",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("b", ValueType::U64),
                ],
            )
            .with_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_b", ["b", "id"])),
            RelationDescriptor::new(
                "D",
                vec![
                    FieldDescriptor::new("id", ValueType::U64),
                    FieldDescriptor::new("c", ValueType::U64),
                ],
            )
            .with_unique("id", ["id"])
            .with_index(IndexDescriptor::equality("by_c", ["c", "id"])),
        ],
    )
}

fn chain_existence_filter_query(schema: &StorageSchema) -> QueryBuildResult<TypedQuery> {
    typed_query(schema, |query| {
        query.rel("A")?.input("id", "a")?.done();
        query.rel("B")?.var("id", "b")?.input("a", "a")?.done();
        query.rel("C")?.var("id", "b")?.integer("b", 99)?.done();
        query.find_var("b")?;
        Ok(())
    })
}

fn seed_title_company_range_facts(env: &Environment, schema: &StorageSchema) -> Result<()> {
    env.write(|txn| {
        for (id, year, company) in [(1, 2004, 10), (2, 2005, 20), (3, 2015, 30), (4, 2020, 40)] {
            txn.insert(
                schema,
                Fact::new(
                    "Title",
                    [("id", Value::U64(id)), ("year", Value::I64(year))],
                ),
            )?;
            txn.insert(
                schema,
                Fact::new(
                    "WorkCompany",
                    [("work", Value::U64(id)), ("company", Value::U64(company))],
                ),
            )?;
        }
        Ok::<_, Error>(())
    })
}

fn title_company_count_query(
    schema: &StorageSchema,
    max_year: OperandRef,
) -> QueryBuildResult<TypedQuery> {
    typed_query(schema, |query| {
        query
            .rel("WorkCompany")?
            .var("work", "work")?
            .var("company", "company")?
            .done();
        query
            .rel("Title")?
            .var("id", "work")?
            .var("year", "year")?
            .done();
        query.cmp(
            OperandRef::var("year"),
            ComparisonOperator::Gte,
            OperandRef::integer(2005),
        )?;
        query.cmp(OperandRef::var("year"), ComparisonOperator::Lte, max_year)?;
        query.find_var("company")?;
        Ok(())
    })
}

fn edge_cross_comparison_query(
    schema: &StorageSchema,
    operator: ComparisonOperator,
) -> QueryBuildResult<TypedQuery> {
    typed_query(schema, |query| {
        query.rel("EdgeAB")?.var("a", "a")?.var("b", "b")?.done();
        query.rel("EdgeAC")?.var("a", "a")?.var("c", "c")?.done();
        query.cmp(OperandRef::var("b"), operator, OperandRef::var("c"))?;
        query.find_var("b")?;
        Ok(())
    })
}

fn holder_fact(id: u64, name: &str) -> Fact {
    Fact::new(
        "Holder",
        [
            ("id", Value::Serial(id)),
            ("name", Value::String(name.to_owned())),
        ],
    )
}

fn account_fact(id: u64, holder: u64, currency: u8) -> Fact {
    Fact::new(
        "Account",
        [
            ("id", Value::Serial(id)),
            ("holder", Value::Serial(holder)),
            ("currency", Value::Enum(currency)),
        ],
    )
}

fn posting_fact(id: u64, account: u64, amount: i128, at: i64) -> Fact {
    Fact::new(
        "Posting",
        [
            ("id", Value::Serial(id)),
            ("account", Value::Serial(account)),
            ("amount", Value::Decimal(DecimalRaw(amount))),
            ("at", Value::Timestamp(TimestampMicros(at))),
        ],
    )
}

fn item_fact(id: u64, kind: u8) -> Fact {
    Fact::new(
        "Item",
        [("id", Value::Serial(id)), ("kind", Value::Enum(kind))],
    )
}

fn edge_ab_fact(a: u64, b: u64) -> Fact {
    Fact::new("EdgeAB", [("a", Value::U64(a)), ("b", Value::U64(b))])
}

fn edge_ac_fact(a: u64, c: u64) -> Fact {
    Fact::new("EdgeAC", [("a", Value::U64(a)), ("c", Value::U64(c))])
}

fn edge_bc_fact(b: u64, c: u64) -> Fact {
    Fact::new("EdgeBC", [("b", Value::U64(b)), ("c", Value::U64(c))])
}

fn b_fact(id: u64, a: u64) -> Fact {
    Fact::new("B", [("id", Value::U64(id)), ("a", Value::U64(a))])
}

fn reserve_fact(sailor: u64, boat: u64, day: i64) -> Fact {
    Fact::new(
        "Reserve",
        [
            ("sailor", Value::U64(sailor)),
            ("boat", Value::U64(boat)),
            ("day", Value::Timestamp(TimestampMicros(day))),
        ],
    )
}

fn chain_a_fact(id: u64) -> Fact {
    Fact::new("A", [("id", Value::U64(id))])
}

fn chain_b_fact(id: u64, a: u64) -> Fact {
    Fact::new("B", [("id", Value::U64(id)), ("a", Value::U64(a))])
}

fn chain_c_fact(id: u64, b: u64) -> Fact {
    Fact::new("C", [("id", Value::U64(id)), ("b", Value::U64(b))])
}

fn chain_d_fact(id: u64, c: u64) -> Fact {
    Fact::new("D", [("id", Value::U64(id)), ("c", Value::U64(c))])
}

fn assert_same_facts(actual: &[Vec<Value>], expected: &[Vec<Value>]) {
    let mut actual = actual.to_vec();
    let mut expected = expected.to_vec();
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected);
}

struct ReferenceDb {
    facts: BTreeMap<String, Vec<Fact>>,
}

#[derive(Clone, Debug)]
struct ReferenceBinding {
    values: Vec<Option<Value>>,
}

impl ReferenceBinding {
    fn new(variable_count: usize) -> Self {
        Self {
            values: vec![None; variable_count],
        }
    }

    fn get(&self, variable: usize) -> Option<&Value> {
        self.values[variable].as_ref()
    }

    fn bind(&mut self, variable: usize, value: Value) -> bool {
        match &self.values[variable] {
            Some(existing) => existing == &value,
            None => {
                self.values[variable] = Some(value);
                true
            }
        }
    }
}

impl ReferenceDb {
    fn from_facts(facts: Vec<Fact>) -> Self {
        let mut by_relation: BTreeMap<String, Vec<Fact>> = BTreeMap::new();
        for fact in facts {
            by_relation
                .entry(fact.relation().to_owned())
                .or_default()
                .push(fact);
        }
        Self { facts: by_relation }
    }

    fn execute(&self, query: &TypedQuery, inputs: &InputBindings) -> Result<Vec<Vec<Value>>> {
        let atoms = query
            .clauses
            .iter()
            .filter_map(|clause| match clause {
                TypedClause::Relation(atom) => Some(atom),
                TypedClause::Comparison(_) => None,
            })
            .collect::<Vec<_>>();
        let comparisons = query
            .clauses
            .iter()
            .filter_map(|clause| match clause {
                TypedClause::Comparison(comparison) => Some(comparison),
                TypedClause::Relation(_) => None,
            })
            .collect::<Vec<_>>();
        let mut output = Vec::new();
        let mut counters = PlanCounters::default();
        self.recurse(
            query,
            inputs,
            &atoms,
            &comparisons,
            0,
            ReferenceBinding::new(query.variables.len()),
            &mut output,
            &mut counters,
        )?;
        reference_project_results(query, &output)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "test reference recursion carries explicit evaluator state"
    )]
    fn recurse(
        &self,
        query: &TypedQuery,
        inputs: &InputBindings,
        atoms: &[&TypedRelationAtom],
        comparisons: &[&TypedComparison],
        depth: usize,
        binding: ReferenceBinding,
        output: &mut Vec<ReferenceBinding>,
        counters: &mut PlanCounters,
    ) -> Result<()> {
        if depth == atoms.len() {
            if reference_comparisons_pass(comparisons, query, inputs, &binding, counters)? {
                output.push(binding);
            }
            return Ok(());
        }

        let atom = atoms[depth];
        for fact in self.facts.get(&atom.relation).into_iter().flatten() {
            let Some(next) = reference_match_atom(atom, query, inputs, &binding, fact)? else {
                continue;
            };
            if reference_comparisons_pass(comparisons, query, inputs, &next, counters)? {
                self.recurse(
                    query,
                    inputs,
                    atoms,
                    comparisons,
                    depth + 1,
                    next,
                    output,
                    counters,
                )?;
            }
        }
        Ok(())
    }
}

fn reference_match_atom(
    atom: &TypedRelationAtom,
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &ReferenceBinding,
    fact: &Fact,
) -> Result<Option<ReferenceBinding>> {
    let mut next = binding.clone();
    for field in &atom.fields {
        let Some(fact_value) = fact.value(&field.field) else {
            return Ok(None);
        };
        match &field.term {
            TypedTerm::Variable(variable) => {
                let normalized =
                    reference_value_for_type(fact_value, &query.variables[*variable].value_type);
                if !next.bind(*variable, normalized) {
                    return Ok(None);
                }
            }
            TypedTerm::Input(input) => {
                let input_value = reference_input_value(query, inputs, *input)?;
                let normalized =
                    reference_value_for_type(fact_value, &query.inputs[*input].value_type);
                if input_value != &normalized {
                    return Ok(None);
                }
            }
            TypedTerm::Literal(literal) => {
                let normalized = reference_value_for_type(fact_value, &literal.value_type);
                if literal_to_value(literal)? != normalized {
                    return Ok(None);
                }
            }
            TypedTerm::Wildcard => {}
        }
    }
    Ok(Some(next))
}

fn reference_comparisons_pass(
    comparisons: &[&TypedComparison],
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &ReferenceBinding,
    counters: &mut PlanCounters,
) -> Result<bool> {
    for comparison in comparisons {
        let Some(left) = reference_operand_value(&comparison.left, query, inputs, binding)? else {
            continue;
        };
        let Some(right) = reference_operand_value(&comparison.right, query, inputs, binding)?
        else {
            continue;
        };
        counters.comparisons_evaluated += 1;
        let left = reference_value_for_type(&left, &comparison.value_type);
        let right = reference_value_for_type(&right, &comparison.value_type);
        if !compare_values(&left, comparison.operator, &right) {
            counters.comparisons_failed += 1;
            return Ok(false);
        }
    }
    Ok(true)
}

fn reference_input_value<'a>(
    query: &'a TypedQuery,
    inputs: &'a InputBindings,
    input: usize,
) -> Result<&'a Value> {
    let input = &query.inputs[input];
    inputs
        .get(&input.name)
        .ok_or_else(|| Error::missing_input(&input.name))
}

fn reference_operand_value(
    operand: &TypedOperand,
    query: &TypedQuery,
    inputs: &InputBindings,
    binding: &ReferenceBinding,
) -> Result<Option<Value>> {
    Ok(match operand {
        TypedOperand::Variable(variable) => binding.get(*variable).cloned(),
        TypedOperand::Input(input) => Some(reference_input_value(query, inputs, *input)?.clone()),
        TypedOperand::Literal(literal) => Some(literal_to_value(literal)?),
    })
}

fn reference_value_for_type(value: &Value, _value_type: &ValueType) -> Value {
    value.clone()
}

fn reference_project_results(
    query: &TypedQuery,
    bindings: &[ReferenceBinding],
) -> Result<Vec<Vec<Value>>> {
    let mut set = BTreeSet::new();
    for binding in bindings {
        let mut fact = Vec::new();
        for term in &query.find {
            let TypedFindTerm::Variable { variable } = term;
            fact.push(reference_bound_variable(binding, *variable)?.clone());
        }
        set.insert(fact);
    }
    Ok(set.into_iter().collect())
}

fn reference_bound_variable(binding: &ReferenceBinding, variable: usize) -> Result<&Value> {
    binding
        .get(variable)
        .ok_or_else(|| Error::internal(format!("variable {variable} is unbound at projection")))
}
