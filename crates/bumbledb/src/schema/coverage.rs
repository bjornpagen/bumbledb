//! Shared ground and transaction semantics for contextual Event coverage.
//! Keep the group's context even when a contribution is empty or its target is
//! `true`: every subsequent Event must still align with that context.

use crate::event::{BoolOp4, Control, Event, Result};

pub(super) enum Coverage {
    ContextualFull,
    Region(Event),
}

/// `None` as a source denotes contextual full. Missing target coverage instead
/// denotes the empty union, whose context is learned from the first Event.
pub(super) fn covers(
    target: &mut Option<Coverage>,
    source: Option<&Event>,
    control: &dyn Control,
) -> Result<bool> {
    control.checkpoint()?;
    let Some(source) = source else {
        return Ok(match target {
            Some(Coverage::ContextualFull) => true,
            Some(Coverage::Region(target)) => target.is_full(),
            None => false, // Every admitted world space is nonempty.
        });
    };
    match target {
        Some(Coverage::ContextualFull) => {
            *target = Some(Coverage::Region(source.space().full()));
            Ok(true)
        }
        Some(Coverage::Region(target)) => {
            let source = source.align_to(&target.space(), control)?;
            Ok(source
                .apply(BoolOp4::AND, &target.complement(), control)?
                .is_empty())
        }
        None => {
            *target = Some(Coverage::Region(source.space().empty()));
            Ok(source.is_empty())
        }
    }
}
