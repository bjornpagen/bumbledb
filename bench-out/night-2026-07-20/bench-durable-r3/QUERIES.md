# The read query families

Family-list digest: `308e79a1103bb1e5d09e2454a8888792f0199d0e469af3ff66583b960259e6b2`.

## point

Kind: gate.

```text
Query {
    head: [
        Var,
        Var,
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                Var(
                    VarId(
                        1,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            4,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Param(
                                ParamId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                4,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                5,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [],
        },
    ],
}
```

```sql
SELECT DISTINCT t0."amount", t0."at" FROM "Posting" AS t0 WHERE t0."id" = ?1
```

Params: 3 existing posting ids + 1 miss (id = postings + 10^6).

## containment_walk

Kind: gate.

```text
Query {
    head: [
        Var,
        Var,
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                Var(
                    VarId(
                        1,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            4,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                2,
                            ),
                            Param(
                                ParamId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                4,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                    ],
                },
                Atom {
                    source: Edb(
                        RelationId(
                            1,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Param(
                                ParamId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                1,
                            ),
                            Var(
                                VarId(
                                    2,
                                ),
                            ),
                        ),
                    ],
                },
                Atom {
                    source: Edb(
                        RelationId(
                            0,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    2,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                1,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [],
        },
    ],
}
```

```sql
SELECT DISTINCT t2."name", t0."amount" FROM "Posting" AS t0, "Account" AS t1, "Holder" AS t2 WHERE t0."account" = ?1 AND t1."id" = ?1 AND t1."holder" = t2."id"
```

Params: 2 cold accounts, 1 hot account, 1 miss (id = accounts + 10^6).

## chain

Kind: gate.

```text
Query {
    head: [
        Var,
        Var,
        Var,
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                Var(
                    VarId(
                        1,
                    ),
                ),
                Var(
                    VarId(
                        2,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            4,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                1,
                            ),
                            Var(
                                VarId(
                                    3,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                2,
                            ),
                            Var(
                                VarId(
                                    4,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                4,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                5,
                            ),
                            Var(
                                VarId(
                                    2,
                                ),
                            ),
                        ),
                    ],
                },
                Atom {
                    source: Edb(
                        RelationId(
                            3,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    3,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                1,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                    ],
                },
                Atom {
                    source: Edb(
                        RelationId(
                            1,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    4,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                2,
                            ),
                            Literal(
                                U64(
                                    0,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [
                Leaf(
                    Comparison {
                        op: Ge,
                        lhs: Var(
                            VarId(
                                2,
                            ),
                        ),
                        rhs: Param(
                            ParamId(
                                0,
                            ),
                        ),
                    },
                ),
            ],
        },
    ],
}
```

```sql
SELECT DISTINCT t1."source", t0."amount", t0."at" FROM "Posting" AS t0, "JournalEntry" AS t1, "Account" AS t2 WHERE t0."entry" = t1."id" AND t0."account" = t2."id" AND t2."currency" = 0 AND t0."at" >= ?1
```

Params: 4 suffix edges near the corpus end (at >= edge selects ~2/4/6/8%).

## range

Kind: gate.

```text
Query {
    head: [
        Var,
        Var,
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                Var(
                    VarId(
                        1,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            4,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                4,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                5,
                            ),
                            Var(
                                VarId(
                                    2,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [
                Leaf(
                    Comparison {
                        op: Ge,
                        lhs: Var(
                            VarId(
                                2,
                            ),
                        ),
                        rhs: Param(
                            ParamId(
                                0,
                            ),
                        ),
                    },
                ),
                Leaf(
                    Comparison {
                        op: Lt,
                        lhs: Var(
                            VarId(
                                2,
                            ),
                        ),
                        rhs: Param(
                            ParamId(
                                1,
                            ),
                        ),
                    },
                ),
            ],
        },
    ],
}
```

```sql
SELECT DISTINCT t0."id", t0."amount" FROM "Posting" AS t0 WHERE t0."at" >= ?1 AND t0."at" < ?2
```

Params: 4 windows of the pinned ~2% selectivity, spread over the span.

## balance

Kind: gate.

```text
Query {
    head: [
        Var,
        Aggregate(
            Sum,
        ),
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                Aggregate {
                    op: Sum,
                    over: Some(
                        VarId(
                            1,
                        ),
                    ),
                },
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            4,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    2,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                2,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                4,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                    ],
                },
                Atom {
                    source: Edb(
                        RelationId(
                            1,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                1,
                            ),
                            Param(
                                ParamId(
                                    0,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [],
        },
    ],
}
```

```sql
SELECT v0, SUM(v1) FROM (SELECT DISTINCT t0."account" AS v0, t0."amount" AS v1, t0."id" AS v2 FROM "Posting" AS t0, "Account" AS t1 WHERE t0."account" = t1."id" AND t1."holder" = ?1) GROUP BY v0
```

Params: 4 holders, the first owning hot account 0.

## stats

Kind: gate.

```text
Query {
    head: [
        Var,
        Aggregate(
            Min,
        ),
        Aggregate(
            Max,
        ),
        Aggregate(
            Count,
        ),
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                Aggregate {
                    op: Min,
                    over: Some(
                        VarId(
                            2,
                        ),
                    ),
                },
                Aggregate {
                    op: Max,
                    over: Some(
                        VarId(
                            1,
                        ),
                    ),
                },
                Aggregate {
                    op: Count,
                    over: None,
                },
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            4,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                2,
                            ),
                            Var(
                                VarId(
                                    3,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                4,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                5,
                            ),
                            Var(
                                VarId(
                                    2,
                                ),
                            ),
                        ),
                    ],
                },
                Atom {
                    source: Edb(
                        RelationId(
                            1,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    3,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                2,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [],
        },
    ],
}
```

```sql
SELECT v0, MIN(v2), MAX(v1), COUNT(*) FROM (SELECT DISTINCT t1."currency" AS v0, t0."amount" AS v1, t0."at" AS v2, t0."account" AS v3 FROM "Posting" AS t0, "Account" AS t1 WHERE t0."account" = t1."id") GROUP BY v0
```

Params: No params — literal-free full fold; one empty draw.

## string

Kind: gate.

```text
Query {
    head: [
        Var,
        Var,
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                Var(
                    VarId(
                        1,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            4,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                4,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                3,
                            ),
                            Var(
                                VarId(
                                    2,
                                ),
                            ),
                        ),
                    ],
                },
                Atom {
                    source: Edb(
                        RelationId(
                            2,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    2,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                1,
                            ),
                            Param(
                                ParamId(
                                    0,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [],
        },
    ],
}
```

```sql
SELECT DISTINCT t0."id", t0."amount" FROM "Posting" AS t0, "Instrument" AS t1 WHERE t0."instrument" = t1."id" AND t1."symbol" = ?1
```

Params: 3 existing symbols + 1 never-interned miss.

## skew

Kind: gate.

```text
Query {
    head: [
        Var,
        Var,
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                Var(
                    VarId(
                        1,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            4,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                4,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                    ],
                },
                Atom {
                    source: Edb(
                        RelationId(
                            5,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                1,
                            ),
                            Param(
                                ParamId(
                                    0,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [],
        },
    ],
}
```

```sql
SELECT DISTINCT t0."id", t0."amount" FROM "Posting" AS t0, "PostingTag" AS t1 WHERE t0."id" = t1."posting" AND t1."tag" = ?1
```

Params: The hot tag (Fee, ~60% of first tags), then the two uniform tags.

## spread

Kind: gate.

```text
Query {
    head: [
        Var,
        Var,
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                Var(
                    VarId(
                        1,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            4,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                1,
                            ),
                            Var(
                                VarId(
                                    2,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                4,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                    ],
                },
                Atom {
                    source: Edb(
                        RelationId(
                            4,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                1,
                            ),
                            Var(
                                VarId(
                                    2,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                4,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [
                Leaf(
                    Comparison {
                        op: Lt,
                        lhs: Var(
                            VarId(
                                0,
                            ),
                        ),
                        rhs: Var(
                            VarId(
                                1,
                            ),
                        ),
                    },
                ),
            ],
        },
    ],
}
```

```sql
SELECT DISTINCT t0."amount", t1."amount" FROM "Posting" AS t0, "Posting" AS t1 WHERE t0."entry" = t1."entry" AND t0."amount" < t1."amount"
```

Params: No params — full-relation cross-atom residual; one empty draw.

## triangle

Kind: gate.

```text
Query {
    head: [
        Var,
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            4,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                2,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                3,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                    ],
                },
                Atom {
                    source: Edb(
                        RelationId(
                            4,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                1,
                            ),
                            Var(
                                VarId(
                                    2,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                3,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                    ],
                },
                Atom {
                    source: Edb(
                        RelationId(
                            4,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                1,
                            ),
                            Var(
                                VarId(
                                    2,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                2,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [
                Leaf(
                    Comparison {
                        op: Ge,
                        lhs: Var(
                            VarId(
                                0,
                            ),
                        ),
                        rhs: Param(
                            ParamId(
                                0,
                            ),
                        ),
                    },
                ),
                Leaf(
                    Comparison {
                        op: Lt,
                        lhs: Var(
                            VarId(
                                0,
                            ),
                        ),
                        rhs: Param(
                            ParamId(
                                1,
                            ),
                        ),
                    },
                ),
            ],
        },
    ],
}
```

```sql
SELECT DISTINCT t0."account" FROM "Posting" AS t0, "Posting" AS t1, "Posting" AS t2 WHERE t0."instrument" = t1."instrument" AND t1."entry" = t2."entry" AND t0."account" = t2."account" AND t0."account" >= ?1 AND t0."account" < ?2
```

Params: 3 cold ~1%-of-accounts windows (?0 <= a < ?1, past the hot set) + the empty window.

## entries_for_account_set

Kind: gate.

```text
Query {
    head: [
        Var,
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            4,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                1,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                2,
                            ),
                            ParamSet(
                                ParamId(
                                    0,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [],
        },
    ],
}
```

```sql
SELECT DISTINCT t0."entry" FROM "Posting" AS t0 WHERE t0."account" IN (3, 7, 9)
```

Params: Account sets of sizes 1, 3 (hot account 0 included), 8, and 0 — the golden pins the representative set {3, 7, 9}.

## postings_without_tag

Kind: gate.

```text
Query {
    head: [
        Var,
        Var,
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                Var(
                    VarId(
                        1,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            4,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                2,
                            ),
                            Param(
                                ParamId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                4,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [
                Atom {
                    source: Edb(
                        RelationId(
                            5,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            conditions: [],
        },
    ],
}
```

```sql
SELECT DISTINCT t0."id", t0."amount" FROM "Posting" AS t0 WHERE t0."account" = ?1 AND NOT EXISTS (SELECT 1 FROM "PostingTag" AS n0 WHERE n0."posting" = t0."id")
```

Params: 2 cold accounts, 1 hot account, 1 miss (id = accounts + 10^6).

## latest_posting_per_account

Kind: gate.

```text
Query {
    head: [
        Var,
        Aggregate(
            ArgMax,
        ),
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                Aggregate {
                    op: ArgMax {
                        key: VarId(
                            2,
                        ),
                    },
                    over: Some(
                        VarId(
                            1,
                        ),
                    ),
                },
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            4,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                2,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                5,
                            ),
                            Var(
                                VarId(
                                    2,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [],
        },
    ],
}
```

```sql
WITH d AS (SELECT DISTINCT t0."account" AS v0, t0."id" AS v1, t0."at" AS v2 FROM "Posting" AS t0) SELECT DISTINCT d.v0, d.v1 FROM d JOIN (SELECT v0, MAX(v2) AS mk FROM d GROUP BY v0) m ON d.v0 = m.v0 AND d.v2 = m.mk
```

Params: No params — full Arg-restriction over every account; one empty draw.

## mandate_at_instant

Kind: gate.

```text
Query {
    head: [
        Var,
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            4,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                2,
                            ),
                            Param(
                                ParamId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                5,
                            ),
                            Param(
                                ParamId(
                                    1,
                                ),
                            ),
                        ),
                    ],
                },
                Atom {
                    source: Edb(
                        RelationId(
                            8,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Param(
                                ParamId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                1,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                2,
                            ),
                            Param(
                                ParamId(
                                    1,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [],
        },
    ],
}
```

```sql
SELECT DISTINCT t1."org" FROM "Posting" AS t0, "Mandate" AS t1 WHERE t0."account" = ?1 AND t0."at" = ?2 AND t1."account" = ?1 AND t1."active_start" <= ?2 AND ?2 < t1."active_end"
```

Params: 3 real postings' (account, at) instants + 1 account miss — gap instants occur naturally (segments 1-2 and 2-3 are gapped).

## mandate_overlap

Kind: gate.

```text
Query {
    head: [
        Var,
        Var,
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                Var(
                    VarId(
                        1,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            8,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                1,
                            ),
                            Param(
                                ParamId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                2,
                            ),
                            Var(
                                VarId(
                                    2,
                                ),
                            ),
                        ),
                    ],
                },
                Atom {
                    source: Edb(
                        RelationId(
                            8,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                1,
                            ),
                            Param(
                                ParamId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                2,
                            ),
                            Var(
                                VarId(
                                    3,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [
                Leaf(
                    Comparison {
                        op: Allen {
                            mask: Literal(
                                AllenMask(
                                    2044,
                                ),
                            ),
                        },
                        lhs: Var(
                            VarId(
                                2,
                            ),
                        ),
                        rhs: Var(
                            VarId(
                                3,
                            ),
                        ),
                    },
                ),
            ],
        },
    ],
}
```

```sql
SELECT DISTINCT t0."account", t1."account" FROM "Mandate" AS t0, "Mandate" AS t1 WHERE t0."org" = ?1 AND t1."org" = ?1 AND ((t0."active_start" < t1."active_start" AND t1."active_start" < t0."active_end" AND t0."active_end" < t1."active_end") OR (t0."active_start" = t1."active_start" AND t0."active_end" < t1."active_end") OR (t1."active_start" < t0."active_start" AND t0."active_end" < t1."active_end") OR (t1."active_start" < t0."active_start" AND t0."active_end" = t1."active_end") OR (t0."active_start" = t1."active_start" AND t0."active_end" = t1."active_end") OR (t0."active_start" < t1."active_start" AND t0."active_end" = t1."active_end") OR (t0."active_start" < t1."active_start" AND t1."active_end" < t0."active_end") OR (t0."active_start" = t1."active_start" AND t1."active_end" < t0."active_end") OR (t1."active_start" < t0."active_start" AND t0."active_start" < t1."active_end" AND t1."active_end" < t0."active_end"))
```

Params: 4 org ids (mandates spread uniformly over 64 orgs).

# The calendar query families

Family-list digest: `2eb3e965f2da45a3eeb89e28be196f00a8f880e93bd2e2712127b829f47a21f9`.

## busy_scan

Kind: gate.

```text
Query {
    head: [
        Var,
        Var,
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                Var(
                    VarId(
                        1,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            5,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                1,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                2,
                            ),
                            Literal(
                                U64(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                3,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [
                Leaf(
                    Comparison {
                        op: Allen {
                            mask: Literal(
                                AllenMask(
                                    2044,
                                ),
                            ),
                        },
                        lhs: Var(
                            VarId(
                                1,
                            ),
                        ),
                        rhs: Param(
                            ParamId(
                                0,
                            ),
                        ),
                    },
                ),
            ],
        },
    ],
}
```

```sql
SELECT DISTINCT t0."person", t0."span_start", t0."span_end" FROM "Claim" AS t0 WHERE t0."arm" = 0 AND ((t0."span_start" < ?1 AND ?1 < t0."span_end" AND t0."span_end" < ?2) OR (t0."span_start" = ?1 AND t0."span_end" < ?2) OR (?1 < t0."span_start" AND t0."span_end" < ?2) OR (?1 < t0."span_start" AND t0."span_end" = ?2) OR (t0."span_start" = ?1 AND t0."span_end" = ?2) OR (t0."span_start" < ?1 AND t0."span_end" = ?2) OR (t0."span_start" < ?1 AND ?2 < t0."span_end") OR (t0."span_start" = ?1 AND ?2 < t0."span_end") OR (?1 < t0."span_start" AND t0."span_start" < ?2 AND ?2 < t0."span_end"))
```

Params: 3 ~1.6%-of-span windows spread over the active span + 1 pre-epoch miss.

## meets_chain

Kind: gate.

```text
Query {
    head: [
        Var,
        Var,
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        1,
                    ),
                ),
                Var(
                    VarId(
                        2,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            5,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                1,
                            ),
                            Param(
                                ParamId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                3,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                    ],
                },
                Atom {
                    source: Edb(
                        RelationId(
                            5,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                1,
                            ),
                            Param(
                                ParamId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                3,
                            ),
                            Var(
                                VarId(
                                    2,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [
                Leaf(
                    Comparison {
                        op: Allen {
                            mask: Literal(
                                AllenMask(
                                    2,
                                ),
                            ),
                        },
                        lhs: Var(
                            VarId(
                                1,
                            ),
                        ),
                        rhs: Var(
                            VarId(
                                2,
                            ),
                        ),
                    },
                ),
                Leaf(
                    Comparison {
                        op: Allen {
                            mask: Literal(
                                AllenMask(
                                    16,
                                ),
                            ),
                        },
                        lhs: Var(
                            VarId(
                                1,
                            ),
                        ),
                        rhs: Param(
                            ParamId(
                                1,
                            ),
                        ),
                    },
                ),
            ],
        },
    ],
}
```

```sql
SELECT DISTINCT t0."span_start", t0."span_end", t1."span_start", t1."span_end" FROM "Claim" AS t0, "Claim" AS t1 WHERE t0."person" = ?1 AND t1."person" = ?1 AND ((t0."span_end" = t1."span_start")) AND ((?2 < t0."span_start" AND t0."span_end" < ?3))
```

Params: The Zipf-head person, a mid person, person 63 under a quarter window, + 1 person miss.

## rsvp_union

Kind: gate.

```text
Query {
    head: [
        Var,
        Var,
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                Var(
                    VarId(
                        1,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            4,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                1,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                2,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                3,
                            ),
                            Literal(
                                U64(
                                    0,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [],
        },
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                Var(
                    VarId(
                        1,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            4,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                1,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                2,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                3,
                            ),
                            Literal(
                                U64(
                                    1,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [],
        },
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                Var(
                    VarId(
                        1,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            4,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                1,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                2,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                3,
                            ),
                            Literal(
                                U64(
                                    2,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [],
        },
    ],
}
```

```sql
SELECT DISTINCT t0."event", t0."person" FROM "Attendance" AS t0 WHERE t0."rsvp" = 0 UNION SELECT DISTINCT t0."event", t0."person" FROM "Attendance" AS t0 WHERE t0."rsvp" = 1 UNION SELECT DISTINCT t0."event", t0."person" FROM "Attendance" AS t0 WHERE t0."rsvp" = 2
```

Params: No params — the DU whole-read; one empty draw.

## conflict_pairs

Kind: gate.

```text
Query {
    head: [
        Var,
        Var,
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                Var(
                    VarId(
                        1,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            1,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                1,
                            ),
                            Param(
                                ParamId(
                                    0,
                                ),
                            ),
                        ),
                    ],
                },
                Atom {
                    source: Edb(
                        RelationId(
                            5,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                1,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                3,
                            ),
                            Var(
                                VarId(
                                    2,
                                ),
                            ),
                        ),
                    ],
                },
                Atom {
                    source: Edb(
                        RelationId(
                            1,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                1,
                            ),
                            Param(
                                ParamId(
                                    0,
                                ),
                            ),
                        ),
                    ],
                },
                Atom {
                    source: Edb(
                        RelationId(
                            5,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                1,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                3,
                            ),
                            Var(
                                VarId(
                                    3,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [
                Leaf(
                    Comparison {
                        op: Allen {
                            mask: Literal(
                                AllenMask(
                                    2044,
                                ),
                            ),
                        },
                        lhs: Var(
                            VarId(
                                2,
                            ),
                        ),
                        rhs: Var(
                            VarId(
                                3,
                            ),
                        ),
                    },
                ),
            ],
        },
    ],
}
```

```sql
SELECT DISTINCT t0."id", t2."id" FROM "Person" AS t0, "Claim" AS t1, "Person" AS t2, "Claim" AS t3 WHERE t0."account" = ?1 AND t0."id" = t1."person" AND t2."account" = ?1 AND t2."id" = t3."person" AND ((t1."span_start" < t3."span_start" AND t3."span_start" < t1."span_end" AND t1."span_end" < t3."span_end") OR (t1."span_start" = t3."span_start" AND t1."span_end" < t3."span_end") OR (t3."span_start" < t1."span_start" AND t1."span_end" < t3."span_end") OR (t3."span_start" < t1."span_start" AND t1."span_end" = t3."span_end") OR (t1."span_start" = t3."span_start" AND t1."span_end" = t3."span_end") OR (t1."span_start" < t3."span_start" AND t1."span_end" = t3."span_end") OR (t1."span_start" < t3."span_start" AND t3."span_end" < t1."span_end") OR (t1."span_start" = t3."span_start" AND t3."span_end" < t1."span_end") OR (t3."span_start" < t1."span_start" AND t1."span_start" < t3."span_end" AND t3."span_end" < t1."span_end"))
```

Params: The head account (persons 0..8 — the dense stratum), two others, + 1 miss.

## conflict_free

Kind: gate.

```text
Query {
    head: [
        Var,
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            1,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                1,
                            ),
                            Param(
                                ParamId(
                                    0,
                                ),
                            ),
                        ),
                    ],
                },
                Atom {
                    source: Edb(
                        RelationId(
                            3,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                3,
                            ),
                            Param(
                                ParamId(
                                    1,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [
                Atom {
                    source: Edb(
                        RelationId(
                            5,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                1,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                3,
                            ),
                            Param(
                                ParamId(
                                    1,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            conditions: [],
        },
    ],
}
```

```sql
SELECT DISTINCT t0."id" FROM "Person" AS t0, "Event" AS t1 WHERE t0."account" = ?1 AND t1."created_at" = ?2 AND NOT EXISTS (SELECT 1 FROM "Claim" AS n0 WHERE n0."person" = t0."id" AND n0."span_start" <= ?2 AND ?2 < n0."span_end")
```

Params: 3 (account, event-creation instant) pairs + 1 account miss; instants scatter over the active span.

## free_busy

Kind: gate.

```text
Query {
    head: [
        Var,
        Aggregate(
            Pack,
        ),
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                Aggregate {
                    op: Pack,
                    over: Some(
                        VarId(
                            2,
                        ),
                    ),
                },
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            1,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                1,
                            ),
                            Param(
                                ParamId(
                                    0,
                                ),
                            ),
                        ),
                    ],
                },
                Atom {
                    source: Edb(
                        RelationId(
                            5,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                1,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                3,
                            ),
                            Var(
                                VarId(
                                    2,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [
                Leaf(
                    Comparison {
                        op: Allen {
                            mask: Literal(
                                AllenMask(
                                    2044,
                                ),
                            ),
                        },
                        lhs: Var(
                            VarId(
                                2,
                            ),
                        ),
                        rhs: Param(
                            ParamId(
                                1,
                            ),
                        ),
                    },
                ),
            ],
        },
    ],
}
```

```sql
SELECT p, MIN(s), MAX(e) FROM (SELECT p, s, e, SUM(head) OVER (PARTITION BY p ORDER BY s, e ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) AS island FROM (SELECT p, s, e, CASE WHEN s <= MAX(e) OVER (PARTITION BY p ORDER BY s, e ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING) THEN 0 ELSE 1 END AS head FROM (SELECT DISTINCT t1."person" AS p, t1."span_start" AS s, t1."span_end" AS e FROM "Person" AS t0, "Claim" AS t1 WHERE t0."account" = ?1 AND t0."id" = t1."person" AND ((t1."span_start" < ?2 AND ?2 < t1."span_end" AND t1."span_end" < ?3) OR (t1."span_start" = ?2 AND t1."span_end" < ?3) OR (?2 < t1."span_start" AND t1."span_end" < ?3) OR (?2 < t1."span_start" AND t1."span_end" = ?3) OR (t1."span_start" = ?2 AND t1."span_end" = ?3) OR (t1."span_start" < ?2 AND t1."span_end" = ?3) OR (t1."span_start" < ?2 AND ?3 < t1."span_end") OR (t1."span_start" = ?2 AND ?3 < t1."span_end") OR (?2 < t1."span_start" AND t1."span_start" < ?3 AND ?3 < t1."span_end"))))) GROUP BY p, island
```

Params: The head account wide + narrow, a mid account wide, + 1 miss (translator-unpaired: hand coalesce).

## claim_hours

Kind: gate.

```text
Query {
    head: [
        Var,
        Aggregate(
            Sum,
        ),
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                AggregateMeasure {
                    op: Sum,
                    over: VarId(
                        2,
                    ),
                },
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            5,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                2,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                3,
                            ),
                            Var(
                                VarId(
                                    2,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [
                Leaf(
                    Comparison {
                        op: Allen {
                            mask: Literal(
                                AllenMask(
                                    6147,
                                ),
                            ),
                        },
                        lhs: Var(
                            VarId(
                                2,
                            ),
                        ),
                        rhs: Literal(
                            IntervalI64(
                                Interval {
                                    start: 1800000000,
                                    end: 9223372036854775807,
                                },
                            ),
                        ),
                    },
                ),
            ],
        },
    ],
}
```

```sql
SELECT v0, SUM(v2_end - v2_start) FROM (SELECT DISTINCT t0."arm" AS v0, t0."source" AS v1, t0."span_start" AS v2_start, t0."span_end" AS v2_end FROM "Claim" AS t0 WHERE ((t0."span_end" < 1800000000) OR (t0."span_end" = 1800000000) OR (9223372036854775807 = t0."span_start") OR (9223372036854775807 < t0."span_start"))) GROUP BY v0
```

Params: No params — the ray-filtered full measure fold; one empty draw.

## slot_scan

Kind: report.

```text
Query {
    head: [
        Var,
        Var,
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                Var(
                    VarId(
                        1,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            9,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                1,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [
                Leaf(
                    Comparison {
                        op: Allen {
                            mask: Literal(
                                AllenMask(
                                    2044,
                                ),
                            ),
                        },
                        lhs: Var(
                            VarId(
                                1,
                            ),
                        ),
                        rhs: Param(
                            ParamId(
                                0,
                            ),
                        ),
                    },
                ),
            ],
        },
    ],
}
```

```sql
SELECT DISTINCT t0."room", t0."span_start", t0."span_end" FROM "Slot" AS t0 WHERE ((t0."span_start" < ?1 AND ?1 < t0."span_end" AND t0."span_end" < ?2) OR (t0."span_start" = ?1 AND t0."span_end" < ?2) OR (?1 < t0."span_start" AND t0."span_end" < ?2) OR (?1 < t0."span_start" AND t0."span_end" = ?2) OR (t0."span_start" = ?1 AND t0."span_end" = ?2) OR (t0."span_start" < ?1 AND t0."span_end" = ?2) OR (t0."span_start" < ?1 AND ?2 < t0."span_end") OR (t0."span_start" = ?1 AND ?2 < t0."span_end") OR (?1 < t0."span_start" AND t0."span_start" < ?2 AND ?2 < t0."span_end"))
```

Params: 3 ~6%-of-grid windows spread over the slot grid + 1 pre-epoch miss (fixed-width lane).

## slot_booking_overlap

Kind: report.

```text
Query {
    head: [
        Var,
        Var,
    ],
    rules: [
        Rule {
            finds: [
                Var(
                    VarId(
                        0,
                    ),
                ),
                Var(
                    VarId(
                        1,
                    ),
                ),
            ],
            atoms: [
                Atom {
                    source: Edb(
                        RelationId(
                            9,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Param(
                                ParamId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                1,
                            ),
                            Var(
                                VarId(
                                    0,
                                ),
                            ),
                        ),
                    ],
                },
                Atom {
                    source: Edb(
                        RelationId(
                            7,
                        ),
                    ),
                    bindings: [
                        (
                            FieldId(
                                0,
                            ),
                            Param(
                                ParamId(
                                    0,
                                ),
                            ),
                        ),
                        (
                            FieldId(
                                2,
                            ),
                            Var(
                                VarId(
                                    1,
                                ),
                            ),
                        ),
                    ],
                },
            ],
            negated: [],
            conditions: [
                Leaf(
                    Comparison {
                        op: Allen {
                            mask: Literal(
                                AllenMask(
                                    2044,
                                ),
                            ),
                        },
                        lhs: Var(
                            VarId(
                                0,
                            ),
                        ),
                        rhs: Var(
                            VarId(
                                1,
                            ),
                        ),
                    },
                ),
            ],
        },
    ],
}
```

```sql
SELECT DISTINCT t0."span_start", t0."span_end", t1."span_start", t1."span_end" FROM "Slot" AS t0, "Booking" AS t1 WHERE t0."room" = ?1 AND t1."room" = ?1 AND ((t0."span_start" < t1."span_start" AND t1."span_start" < t0."span_end" AND t0."span_end" < t1."span_end") OR (t0."span_start" = t1."span_start" AND t0."span_end" < t1."span_end") OR (t1."span_start" < t0."span_start" AND t0."span_end" < t1."span_end") OR (t1."span_start" < t0."span_start" AND t0."span_end" = t1."span_end") OR (t0."span_start" = t1."span_start" AND t0."span_end" = t1."span_end") OR (t0."span_start" < t1."span_start" AND t0."span_end" = t1."span_end") OR (t0."span_start" < t1."span_start" AND t1."span_end" < t0."span_end") OR (t0."span_start" = t1."span_start" AND t1."span_end" < t0."span_end") OR (t1."span_start" < t0."span_start" AND t0."span_start" < t1."span_end" AND t1."span_end" < t0."span_end"))
```

Params: The head room, room 1, a mid room, + 1 room miss (fixed x general Allen join).

