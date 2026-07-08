use bumbledb::schema::ValueType;
use rusqlite::Connection;

use crate::translate::Translated;

use super::PreparedFamily;

impl<'c> PreparedFamily<'c> {
    /// Prepares the translated SQL once against the bench connection.
    ///
    /// # Errors
    ///
    /// `SQLite` errors, stringified.
    pub fn new(
        conn: &'c Connection,
        translated: &Translated,
        result_types: Vec<ValueType>,
    ) -> Result<Self, String> {
        Ok(Self {
            stmt: conn
                .prepare(&translated.sql)
                .map_err(|e| format!("prepare: {e}"))?,
            param_order: translated.params.clone(),
            result_types,
        })
    }
}
