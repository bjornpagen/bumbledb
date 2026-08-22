use bumbledb::schema::ValueType;
use rusqlite::Connection;

use crate::translate::Translated;

use super::PreparedFamily;

impl<'c> PreparedFamily<'c> {
    /// # Errors

    pub fn new(
        conn: &'c Connection,
        translated: &Translated,
        signature: Vec<ValueType>,
    ) -> Result<Self, String> {
        Ok(Self {
            stmt: conn
                .prepare(&translated.sql)
                .map_err(|e| format!("prepare: {e}"))?,
            param_order: translated.params.clone(),
            signature,
        })
    }
}
