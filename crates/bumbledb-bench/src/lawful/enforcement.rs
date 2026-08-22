//! The `SQLite` enforcement map AS DATA: one [`Enforcement`] row per
//! MATERIALIZED engine statement (fresh auto-keys first, then the closed
//! auto-keys, then the declared statements — the engine's materialized order,
//! `SchemaDescriptor::materialized_statements`), and the twin DDL assembled
//! FROM the table ([`ddl`]) — an engine law without a `SQLite` enforcement row
//! is a failing totality test, never where no declarative constraint form
//! exists), appended after the

use bumbledb::schema::ValueType;

use super::{ids, schema};

#[derive(Debug, Clone, Copy)]
pub struct Enforcement {
    pub law: &'static str,

    pub notation: &'static str,

    pub sqlite: &'static str,
}

pub const MAP: &[Enforcement] = &[
    Enforcement {
        law: "fresh auto-key",
        notation: "Task(id) -> Task",
        sqlite: "PRIMARY KEY (\"id\")",
    },
    Enforcement {
        law: "fresh auto-key",
        notation: "Attempt(id) -> Attempt",
        sqlite: "PRIMARY KEY (\"id\")",
    },
    Enforcement {
        law: "fresh auto-key",
        notation: "Steer(id) -> Steer",
        sqlite: "PRIMARY KEY (\"id\")",
    },
    Enforcement {
        law: "closed auto-key",
        notation: "TaskKinds(id) -> TaskKinds",
        sqlite: "-- unmirrored: the closed roster is static schema data; its identity lives in \
                 the referencing kind roster constraint on \"Task\"",
    },
    Enforcement {
        law: "closed auto-key",
        notation: "SteerKinds(id) -> SteerKinds",
        sqlite: "-- unmirrored: the closed roster is static schema data; its identity lives in \
                 the referencing kind roster constraint on \"Steer\"",
    },
    Enforcement {
        law: "closed auto-key",
        notation: "Outcome(id) -> Outcome",
        sqlite: "-- unmirrored: the closed roster is static schema data; its identity lives in \
                 the referencing outcome roster constraint on \"Verdict\"",
    },
    Enforcement {
        law: "declared key",
        notation: "Task(kind, subject) -> Task",
        sqlite: "UNIQUE (\"kind\", \"subject\")",
    },
    Enforcement {
        law: "declared key",
        notation: "Attempt(task, n) -> Attempt",
        sqlite: "UNIQUE (\"task\", \"n\")",
    },
    Enforcement {
        law: "declared key",
        notation: "Verdict(attempt) -> Verdict",
        sqlite: "UNIQUE (\"attempt\")",
    },
    Enforcement {
        law: "declared key",
        notation: "SteerScope(steer, grp) -> SteerScope",
        sqlite: "UNIQUE (\"steer\", \"grp\")",
    },
    Enforcement {
        law: "closed-vocabulary containment",
        notation: "Task(kind) <= TaskKinds(id)",
        sqlite: "CHECK (\"kind\" IN (0, 1, 2))",
    },
    Enforcement {
        law: "foreign key",
        notation: "Attempt(task) <= Task(id)",
        sqlite: "FOREIGN KEY (\"task\") REFERENCES \"Task\" (\"id\")",
    },
    Enforcement {
        law: "foreign key",
        notation: "Verdict(attempt) <= Attempt(id)",
        sqlite: "FOREIGN KEY (\"attempt\") REFERENCES \"Attempt\" (\"id\")",
    },
    Enforcement {
        law: "closed-vocabulary containment",
        notation: "Verdict(outcome) <= Outcome(id)",
        sqlite: "CHECK (\"outcome\" IN (0, 1, 2))",
    },
    Enforcement {
        law: "closed-vocabulary containment",
        notation: "Steer(kind) <= SteerKinds(id)",
        sqlite: "CHECK (\"kind\" IN (0, 1))",
    },
    Enforcement {
        law: "foreign key",
        notation: "Steer(task) <= Task(id)",
        sqlite: "FOREIGN KEY (\"task\") REFERENCES \"Task\" (\"id\")",
    },
    Enforcement {
        law: "ψ-selected containment",
        notation: "SteerScope(steer) <= Steer(id | kind == Repartition)",
        sqlite: "CREATE TRIGGER \"lawful_steer_scope_psi\" BEFORE INSERT ON \"SteerScope\" WHEN \
                 NOT EXISTS (SELECT 1 FROM \"Steer\" WHERE \"id\" = NEW.\"steer\" AND \"kind\" = \
                 1) BEGIN SELECT RAISE(ABORT, 'steer scope requires a Repartition steer'); END",
    },
    Enforcement {
        law: "attempt-count capacity law",
        notation: "Task(id) <={0..8} Attempt(task)",
        sqlite: "CREATE TRIGGER \"lawful_attempt_window\" BEFORE INSERT ON \"Attempt\" WHEN \
                 (SELECT COUNT(*) FROM \"Attempt\" WHERE \"task\" = NEW.\"task\") >= 8 BEGIN \
                 SELECT RAISE(ABORT, 'attempt window exceeded'); END",
    },
];

fn sql_type(ty: &ValueType) -> &'static str {
    match ty {
        ValueType::Bool | ValueType::U64 | ValueType::I64 => "INTEGER",
        ValueType::String => "TEXT",
        ValueType::FixedBytes { .. } => "BLOB",
        ValueType::Interval { .. } | ValueType::FixedInterval { .. } => {
            unreachable!("the lawful world declares scalar fields only")
        }
    }
}

fn constrained_table(notation: &'static str) -> &'static str {
    notation
        .split('(')
        .next()
        .expect("a statement notation opens with its relation")
}

/// Nothing here is written by totality test before this assembly could silently
/// omit it.
#[must_use]
pub fn ddl() -> Vec<String> {
    let schema = schema();
    let mut statements = Vec::new();
    for rel in [
        ids::TASK,
        ids::ATTEMPT,
        ids::VERDICT,
        ids::STEER,
        ids::STEER_SCOPE,
    ] {
        let relation = schema.relation(rel);
        let mut items: Vec<String> = relation
            .fields()
            .iter()
            .map(|field| {
                format!(
                    "\"{}\" {} NOT NULL",
                    field.name,
                    sql_type(&field.value_type)
                )
            })
            .collect();
        for row in MAP {
            if row.sqlite.starts_with("--") || row.sqlite.starts_with("CREATE ") {
                continue;
            }
            if constrained_table(row.notation) == relation.name() {
                items.push(row.sqlite.to_owned());
            }
        }
        statements.push(format!(
            "CREATE TABLE \"{}\" ({}) STRICT",
            relation.name(),
            items.join(", ")
        ));
    }
    for row in MAP {
        if row.sqlite.starts_with("CREATE ") {
            statements.push(row.sqlite.to_owned());
        }
    }
    statements
}
