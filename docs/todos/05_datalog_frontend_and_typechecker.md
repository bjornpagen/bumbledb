# 05: Datalog Frontend And Typechecker

**Goal**
- Parse named-field Datalog and turn it into typed logical IR validated against the hardcoded schema.

**Why This Stage Exists**
- The database is Datalog-only, so query syntax and type errors are part of the core user experience.
- The planner must receive typed, normalized IR, not raw strings.

**Concrete Work**
- Implement the canonical named-field query syntax.
- Parse `find`, `where`, relation atoms, variables, inputs, wildcards, constants, comparisons, and aggregate projections.
- Produce an untyped AST with useful source spans.
- Resolve relation names and field names against schema descriptors.
- Infer variable types from field positions.
- Validate input parameter types.
- Validate comparison operand compatibility.
- Validate aggregate argument types.
- Reject unsupported features explicitly: rules, recursion, negation, disjunction, ordering, limit, and user-defined functions.
- Produce typed logical IR with dense variable IDs.
- Add diagnostic tests for common invalid queries.

**Out Of Scope**
- Physical planning.
- Query execution.
- Compile-time query macro.
- Positional Datalog syntax.
- Recursive rules.
- Stratified negation.
- As-of syntax.

**Passing Criteria**
- Valid single-relation queries parse and typecheck.
- Valid multi-relation join queries parse and typecheck.
- Valid comparison and range predicates parse and typecheck.
- Valid aggregate projections parse and typecheck.
- Unknown relations produce clear errors.
- Unknown fields produce clear errors.
- Variables unified across incompatible logical types are rejected.
- Inputs bound to incompatible types are rejected.
- Projection of unbound variables is rejected.
- Unsupported Datalog features are rejected with intentional errors, not parser confusion.
- Typed logical IR contains enough information for planning without consulting source text.

**Notes**
- Named-field syntax is canonical.
- Runtime query strings come before query macros.
- Error quality matters because this is the main query interface.
