//! Portable identity for schema-owned Event literals. Resident Event equality
//! intentionally compares owner-scoped handles; a declaration instead names the
//! complete canonical value. Capture those bytes at fallible admission, then
//! reuse them for normalization, fingerprinting and row selection.

use std::{collections::BTreeMap, sync::Arc};

use super::{Side, StatementDescriptor};
use crate::{
    Value,
    event::{Control, Event, Result},
};

#[derive(Debug, Clone, Default)]
pub(super) struct EventLiterals(BTreeMap<[u64; 2], Arc<[u8]>>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum LiteralIdentity {
    Scalar(Value),
    Event(Arc<[u8]>),
}

impl EventLiterals {
    pub(super) fn prepare(
        statements: &[StatementDescriptor],
        control: &dyn Control,
    ) -> Result<Self> {
        let mut literals = Self::default();
        for statement in statements {
            control.checkpoint()?;
            let sides = match statement {
                StatementDescriptor::Functionality { .. } => continue,
                StatementDescriptor::Containment { source, target }
                | StatementDescriptor::Capacity { source, target, .. } => [source, target],
            };
            for side in sides {
                for (_, set) in &side.selection {
                    for value in set.literals() {
                        control.checkpoint()?;
                        if let Value::Event(event) = value
                            && let std::collections::btree_map::Entry::Vacant(entry) =
                                literals.0.entry(event.key().words())
                        {
                            entry.insert(event.to_bytes(control)?.into());
                        }
                    }
                }
            }
        }
        Ok(literals)
    }

    pub(super) fn bytes(&self, event: &Event) -> &[u8] {
        &self.0[&event.key().words()]
    }

    pub(super) fn identity(&self, value: &Value) -> LiteralIdentity {
        match value {
            Value::Event(event) => LiteralIdentity::Event(self.0[&event.key().words()].clone()),
            scalar => LiteralIdentity::Scalar(scalar.clone()),
        }
    }

    pub(super) fn matches(
        &self,
        side: &Side,
        row: &[Value],
        control: &dyn Control,
    ) -> Result<bool> {
        for (field, set) in &side.selection {
            control.checkpoint()?;
            let actual = &row[usize::from(field.0)];
            let matched = match actual {
                Value::Event(event) => {
                    // Encode each selected Event cell once, including for a set
                    // of alternatives. Different contexts are unequal values,
                    // not an attempted algebraic alignment.
                    let bytes = event.to_bytes(control)?;
                    set.literals().iter().any(|literal| {
                        matches!(literal, Value::Event(expected) if self.bytes(expected) == bytes)
                    })
                }
                scalar => set.literals().contains(scalar),
            };
            if !matched {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
