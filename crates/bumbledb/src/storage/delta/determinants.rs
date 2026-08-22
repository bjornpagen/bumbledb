use crate::arena::ArenaSlice;
use crate::schema::KeyId;
use crate::storage::keys;
use bumbledb_theory::schema::RelationId;

use super::{DeterminantOverlay, Disposition, TupleOwners, WriteDelta};

impl TupleOwners {
    fn new(slice: ArenaSlice, disposition: Disposition) -> Self {
        match disposition {
            Disposition::Insert => Self::Insert {
                fact: slice,
                replaced: Vec::new(),
                deletes: Vec::new(),
            },
            Disposition::Delete => Self::Deletes {
                head: slice,
                rest: Vec::new(),
            },
        }
    }

    fn record(&mut self, slice: ArenaSlice, disposition: Disposition) {
        match disposition {
            Disposition::Insert => match self {
                Self::Insert { fact, replaced, .. } => {
                    replaced.push(*fact);
                    *fact = slice;
                }
                Self::Deletes { head, rest } => {
                    let mut deletes = std::mem::take(rest);
                    deletes.insert(0, *head);
                    *self = Self::Insert {
                        fact: slice,
                        replaced: Vec::new(),
                        deletes,
                    };
                }
            },
            Disposition::Delete => match self {
                Self::Insert { deletes, .. } => deletes.push(slice),
                Self::Deletes { rest, .. } => rest.push(slice),
            },
        }
    }

    fn cancel(self, slice: ArenaSlice) -> Option<Self> {
        match self {
            Self::Insert {
                fact,
                mut replaced,
                mut deletes,
            } if fact == slice => {
                if let Some(prev) = replaced.pop() {
                    Some(Self::Insert {
                        fact: prev,
                        replaced,
                        deletes,
                    })
                } else if deletes.is_empty() {
                    None
                } else {
                    let head = deletes.remove(0);
                    Some(Self::Deletes {
                        head,
                        rest: deletes,
                    })
                }
            }
            Self::Insert {
                fact,
                mut replaced,
                mut deletes,
            } => {
                replaced.retain(|owner| *owner != slice);
                deletes.retain(|owner| *owner != slice);
                Some(Self::Insert {
                    fact,
                    replaced,
                    deletes,
                })
            }
            Self::Deletes { head, mut rest } if head == slice => {
                if rest.is_empty() {
                    None
                } else {
                    let head = rest.remove(0);
                    Some(Self::Deletes { head, rest })
                }
            }
            Self::Deletes { head, mut rest } => {
                rest.retain(|owner| *owner != slice);
                Some(Self::Deletes { head, rest })
            }
        }
    }
}

impl WriteDelta<'_> {
    pub(super) fn record_determinants(
        &mut self,
        rel: RelationId,
        fact_bytes: &[u8],
        slice: ArenaSlice,
        disposition: Disposition,
    ) {
        let relation = self.schema.relation(rel);
        for &key_id in relation.keys() {
            let statement = self.schema.key(key_id);
            keys::determinant_image(
                relation.layout().encoded(fact_bytes),
                &statement.projection,
                &mut self.determinant_scratch,
            );
            let per_key = self.determinants.entry(key_id).or_default();
            if let Some(owners) = per_key.get_mut(self.determinant_scratch.as_bytes()) {
                owners.record(slice, disposition);
            } else {
                #[cfg(test)]
                {
                    self.determinant_scratch_clones += 1;
                }
                per_key.insert(
                    self.determinant_scratch.clone(),
                    TupleOwners::new(slice, disposition),
                );
            }
        }
    }

    pub(super) fn cancel_determinants(
        &mut self,
        rel: RelationId,
        fact_bytes: &[u8],
        slice: ArenaSlice,
    ) {
        let relation = self.schema.relation(rel);
        for &key_id in relation.keys() {
            let statement = self.schema.key(key_id);
            keys::determinant_image(
                relation.layout().encoded(fact_bytes),
                &statement.projection,
                &mut self.determinant_scratch,
            );
            let Some(per_key) = self.determinants.get_mut(&key_id) else {
                continue;
            };
            let Some(owners) = per_key.remove(self.determinant_scratch.as_bytes()) else {
                continue;
            };
            if let Some(owners) = owners.cancel(slice) {
                per_key.insert(self.determinant_scratch.clone(), owners);
            }
        }
    }

    #[must_use]
    pub fn determinant_overlay(
        &self,
        key: KeyId,
        determinant: &[u8],
    ) -> Option<DeterminantOverlay<'_>> {
        self.determinants
            .get(&key)?
            .get(determinant)
            .map(|owners| match owners {
                TupleOwners::Insert { fact, .. } => {
                    DeterminantOverlay::Present(self.arena.get(*fact))
                }
                TupleOwners::Deletes { .. } => DeterminantOverlay::Absent,
            })
    }
}
