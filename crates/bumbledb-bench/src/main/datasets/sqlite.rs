use bumbledb_lmdb::Fact;
use rusqlite::Connection;

use crate::{dec, i64v, id, rf, symbol, text, ts, u64v};

pub(crate) fn insert_ledger_sqlite(
    conn: &Connection,
    facts: &[Fact],
) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for fact in facts {
        match fact.relation() {
            "Holder" => {
                tx.execute(
                    "INSERT INTO holder (id, name) VALUES (?1, ?2)",
                    rusqlite::params![id(fact, "id")?, text(fact, "name")?],
                )?;
            }
            "Account" => {
                tx.execute(
                    "INSERT INTO account (id, holder, currency) VALUES (?1, ?2, ?3)",
                    rusqlite::params![
                        id(fact, "id")?,
                        rf(fact, "holder")?,
                        symbol(fact, "currency")?
                    ],
                )?;
            }
            "Instrument" => {
                tx.execute(
                    "INSERT INTO instrument (id, symbol) VALUES (?1, ?2)",
                    rusqlite::params![id(fact, "id")?, text(fact, "symbol")?],
                )?;
            }
            "JournalEntry" => {
                tx.execute(
                    "INSERT INTO journal_entry (id, source, created_at) VALUES (?1, ?2, ?3)",
                    rusqlite::params![
                        id(fact, "id")?,
                        rf(fact, "source")?,
                        ts(fact, "created_at")?
                    ],
                )?;
            }
            "Posting" => {
                tx.execute("INSERT INTO posting (id, entry, account, instrument, amount, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", rusqlite::params![id(fact, "id")?, rf(fact, "entry")?, rf(fact, "account")?, rf(fact, "instrument")?, dec(fact, "amount")?, ts(fact, "at")?])?;
            }
            "PostingTag" => {
                tx.execute(
                    "INSERT INTO posting_tag (posting, tag) VALUES (?1, ?2)",
                    rusqlite::params![rf(fact, "posting")?, symbol(fact, "tag")?],
                )?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    Ok(())
}

pub(crate) fn insert_sailors_sqlite(
    conn: &Connection,
    facts: &[Fact],
) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for fact in facts {
        match fact.relation() {
            "Sailor" => {
                tx.execute(
                    "INSERT INTO sailor (id, name, rating, age) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![
                        id(fact, "id")?,
                        text(fact, "name")?,
                        u64v(fact, "rating")?,
                        i64v(fact, "age")?
                    ],
                )?;
            }
            "Boat" => {
                tx.execute(
                    "INSERT INTO boat (id, name, color) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(fact, "id")?, text(fact, "name")?, symbol(fact, "color")?],
                )?;
            }
            "Reserve" => {
                tx.execute(
                    "INSERT INTO reserve (sailor, boat, day) VALUES (?1, ?2, ?3)",
                    rusqlite::params![rf(fact, "sailor")?, rf(fact, "boat")?, ts(fact, "day")?],
                )?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    Ok(())
}

pub(crate) fn insert_join_stress_sqlite(
    conn: &Connection,
    facts: &[Fact],
) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for fact in facts {
        match fact.relation() {
            "A" => {
                tx.execute(
                    "INSERT INTO a (id, k) VALUES (?1, ?2)",
                    rusqlite::params![id(fact, "id")?, symbol(fact, "k")?],
                )?;
            }
            "B" => {
                tx.execute(
                    "INSERT INTO b (id, a, k) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(fact, "id")?, rf(fact, "a")?, symbol(fact, "k")?],
                )?;
            }
            "C" => {
                tx.execute(
                    "INSERT INTO c (id, b, k) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(fact, "id")?, rf(fact, "b")?, symbol(fact, "k")?],
                )?;
            }
            "D" => {
                tx.execute(
                    "INSERT INTO d (id, c, k) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(fact, "id")?, rf(fact, "c")?, symbol(fact, "k")?],
                )?;
            }
            "EdgeAB" => {
                tx.execute(
                    "INSERT INTO edge_ab (a, b) VALUES (?1, ?2)",
                    rusqlite::params![rf(fact, "a")?, rf(fact, "b")?],
                )?;
            }
            "EdgeAC" => {
                tx.execute(
                    "INSERT INTO edge_ac (a, c) VALUES (?1, ?2)",
                    rusqlite::params![rf(fact, "a")?, rf(fact, "c")?],
                )?;
            }
            "EdgeBC" => {
                tx.execute(
                    "INSERT INTO edge_bc (b, c) VALUES (?1, ?2)",
                    rusqlite::params![rf(fact, "b")?, rf(fact, "c")?],
                )?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    Ok(())
}

pub(crate) fn insert_tpch_sqlite(
    conn: &Connection,
    facts: &[Fact],
) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for fact in facts {
        match fact.relation() {
            "Customer" => {
                tx.execute(
                    "INSERT INTO customer (id, nation) VALUES (?1, ?2)",
                    rusqlite::params![id(fact, "id")?, symbol(fact, "nation")?],
                )?;
            }
            "Supplier" => {
                tx.execute(
                    "INSERT INTO supplier (id, nation) VALUES (?1, ?2)",
                    rusqlite::params![id(fact, "id")?, symbol(fact, "nation")?],
                )?;
            }
            "Part" => {
                tx.execute(
                    "INSERT INTO part (id, brand) VALUES (?1, ?2)",
                    rusqlite::params![id(fact, "id")?, symbol(fact, "brand")?],
                )?;
            }
            "Orders" => {
                tx.execute(
                    "INSERT INTO orders (id, customer, order_date) VALUES (?1, ?2, ?3)",
                    rusqlite::params![
                        id(fact, "id")?,
                        rf(fact, "customer")?,
                        ts(fact, "order_date")?
                    ],
                )?;
            }
            "LineItem" => {
                tx.execute("INSERT INTO lineitem (id, ord, part, supplier, quantity, extended_price, ship_date) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", rusqlite::params![id(fact, "id")?, rf(fact, "order")?, rf(fact, "part")?, rf(fact, "supplier")?, i64v(fact, "quantity")?, dec(fact, "extended_price")?, ts(fact, "ship_date")?])?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    Ok(())
}
