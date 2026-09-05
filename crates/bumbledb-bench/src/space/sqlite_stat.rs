//! SPACE-01's SQLite side: the comparison must state its actual DDL/index
//! roster and page accounting, under the same rows and comparable
//! durability/constraints as the engine store — "indexed" is a roster, not a
//! label. Index equivalence is checked, not inferred: interval/capacity law
//! enforcement can need structures SQLite does not build, and those semantic
//! differences are reported instead of hidden.

use rusqlite::Connection;

/// One index as SQLite actually holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexRow {
    pub name: String,
    pub table: String,
    /// `None` for implicit indexes (rowid alias / UNIQUE-constraint b-trees
    /// carry no `sql` text).
    pub sql: Option<String>,
}

/// The page-level accounting available on every build: page size, total
/// pages, freelist pages. `dbstat`-level per-btree accounting requires the
/// `SQLITE_ENABLE_DBSTAT_VTAB` build flag; when the bundled build lacks it,
/// the census records a typed refusal instead of silently reporting less.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteCensus {
    pub page_size: u64,
    pub page_count: u64,
    pub freelist_count: u64,
    pub indexes: Vec<IndexRow>,
    /// Per-btree `(name, pageno-count, payload bytes, unused bytes)` from
    /// `dbstat`, or the refusal reason.
    pub dbstat: Result<Vec<BtreeStat>, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BtreeStat {
    pub name: String,
    pub pages: u64,
    pub payload_bytes: u64,
    pub unused_bytes: u64,
}

impl SqliteCensus {
    #[must_use]
    pub const fn page_bytes(&self) -> u64 {
        self.page_size * self.page_count
    }

    #[must_use]
    pub const fn freelist_bytes(&self) -> u64 {
        self.page_size * self.freelist_count
    }
}

fn pragma_u64(conn: &Connection, pragma: &str) -> Result<u64, String> {
    let value: i64 = conn
        .query_row(&format!("PRAGMA {pragma}"), [], |row| row.get(0))
        .map_err(|e| format!("PRAGMA {pragma}: {e}"))?;
    u64::try_from(value).map_err(|_| format!("PRAGMA {pragma}: negative {value}"))
}

/// The complete index roster, including implicit ones, from `sqlite_master`.
///
/// # Errors
pub fn index_roster(conn: &Connection) -> Result<Vec<IndexRow>, String> {
    let mut statement = conn
        .prepare("SELECT name, tbl_name, sql FROM sqlite_master WHERE type = 'index' ORDER BY name")
        .map_err(|e| format!("index roster: {e}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(IndexRow {
                name: row.get(0)?,
                table: row.get(1)?,
                sql: row.get(2)?,
            })
        })
        .map_err(|e| format!("index roster: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("index roster: {e}"))
}

fn dbstat(conn: &Connection) -> Result<Vec<BtreeStat>, String> {
    let mut statement = conn
        .prepare(
            "SELECT name, COUNT(*), SUM(payload), SUM(unused) FROM dbstat GROUP BY name ORDER BY name",
        )
        .map_err(|e| {
            format!(
                "dbstat unavailable (needs SQLITE_ENABLE_DBSTAT_VTAB in the bundled build): {e}"
            )
        })?;
    let rows = statement
        .query_map([], |row| {
            Ok(BtreeStat {
                name: row.get(0)?,
                pages: row.get::<_, i64>(1)?.unsigned_abs(),
                payload_bytes: row.get::<_, i64>(2)?.unsigned_abs(),
                unused_bytes: row.get::<_, i64>(3)?.unsigned_abs(),
            })
        })
        .map_err(|e| format!("dbstat query: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("dbstat rows: {e}"))
}

/// The census. Callers checkpoint-TRUNCATE and keep the connection open only
/// for these reads (file-length stats happen after drop, exactly like the
/// storage lane).
///
/// # Errors
pub fn census(conn: &Connection) -> Result<SqliteCensus, String> {
    Ok(SqliteCensus {
        page_size: pragma_u64(conn, "page_size")?,
        page_count: pragma_u64(conn, "page_count")?,
        freelist_count: pragma_u64(conn, "freelist_count")?,
        indexes: index_roster(conn)?,
        dbstat: dbstat(conn),
    })
}
