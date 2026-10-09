//! Fixed-width physical keys under LMDB's default byte order.
//!
//! `rows`: `[relation u16][home 16][ordinal u64]` → canonical row bytes.
//! `det`:  `[projection u16][routing 16][ordinal u64]` → the row's home.
//!
//! A home is the relation's selected exact scalar key, zero-padded, or else
//! the row's fingerprint; a routing is a projection's exact scalar bytes,
//! zero-padded, or else its fingerprint. Within one relation or projection
//! every home or routing has the same width, so padding preserves order.
//! Equality is always decided by full canonical bytes.

use bumbledb_theory::schema::RelationId;

use super::format::{RowId, relation_word};
use crate::error::CorruptionError;
use crate::error::{Error, Result};
use crate::schema::ProjectionId;

pub(crate) const HOME_LEN: usize = 16;
pub(crate) const KEY_LEN: usize = 2 + HOME_LEN + 8;
pub(crate) const BUCKET_LEN: usize = 2 + HOME_LEN;
/// The longest host key: LMDB's 511-byte key bound minus the host tag.
pub(crate) const HOST_KEY_MAX: usize = 510;

/// A row home or a determinant routing: exact scalar bytes zero-padded, or
/// a fingerprint.
pub(crate) type Route = [u8; HOME_LEN];

pub(crate) fn padded(bytes: &[u8]) -> Result<Route> {
    let mut route = [0; HOME_LEN];
    route
        .get_mut(..bytes.len())
        .ok_or(Error::ForeignSchema)?
        .copy_from_slice(bytes);
    Ok(route)
}

pub(crate) fn bucket(prefix: [u8; 2], route: &Route) -> [u8; BUCKET_LEN] {
    let mut key = [0; BUCKET_LEN];
    key[..2].copy_from_slice(&prefix);
    key[2..].copy_from_slice(route);
    key
}

pub(crate) fn entry(prefix: [u8; 2], route: &Route, row: RowId) -> [u8; KEY_LEN] {
    let mut key = [0; KEY_LEN];
    key[..BUCKET_LEN].copy_from_slice(&bucket(prefix, route));
    key[BUCKET_LEN..].copy_from_slice(&row.0.to_be_bytes());
    key
}

pub(crate) fn row_prefix(relation: RelationId) -> Result<[u8; 2]> {
    relation_word(relation)
}

pub(crate) fn det_prefix(projection: ProjectionId) -> [u8; 2] {
    projection.0.to_be_bytes()
}

/// One validated physical key, borrowed from the transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Parsed {
    pub(crate) prefix: [u8; 2],
    pub(crate) route: Route,
    pub(crate) row: RowId,
}

pub(crate) fn parse(key: &[u8]) -> Result<Parsed> {
    let key: &[u8; KEY_LEN] = key
        .try_into()
        .map_err(|_| Error::Corruption(CorruptionError::MalformedKey("physical key width")))?;
    let (prefix, rest) = key.split_at(2);
    let (route, row) = rest.split_at(HOME_LEN);
    Ok(Parsed {
        prefix: prefix.try_into().expect("2 bytes"),
        route: route.try_into().expect("16 bytes"),
        row: RowId(u64::from_be_bytes(row.try_into().expect("8 bytes"))),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip_and_sort_by_prefix_route_then_ordinal() {
        let mut keys = Vec::new();
        for prefix in [[0, 0], [0, 1], [255, 255]] {
            for route in [[0; HOME_LEN], [7; HOME_LEN], [255; HOME_LEN]] {
                for row in [RowId(0), RowId(1), RowId(u64::MAX)] {
                    let key = entry(prefix, &route, row);
                    assert_eq!(parse(&key).unwrap(), Parsed { prefix, route, row });
                    assert!(key.starts_with(&bucket(prefix, &route)));
                    keys.push(key);
                }
            }
        }
        assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn malformed_widths_refuse_and_padding_rejects_oversized_routes() {
        for length in (0..KEY_LEN).chain([KEY_LEN + 1]) {
            assert!(parse(&vec![0; length]).is_err());
        }
        assert_eq!(padded(&[1, 2]).unwrap()[..3], [1, 2, 0]);
        assert!(padded(&[0; HOME_LEN + 1]).is_err());
    }
}
