//! Event Pack retains participating claims until the complete group context is
//! known. A canonical minimum, never visitation order, selects that context.
//! Claims carry written provenance independently of idempotent union values.
use super::{Bindings, EitherSink, OutputProgram, events::Faults};
use crate::exec::run::Sink;
use crate::exec::scratch::{ScratchMapId, ScratchRelation, ScratchWordKey};
use crate::exec::sink::FindSpec;
use crate::image::intern::InternerHandle;
use crate::work::{GenerationHandle, WorkContext};
use crate::{Event, EventFaultCategory, EventOperandFault, FindTerm, Result, VarId};

/// Inline claims preserve group-token order in both RAM and scratch. Wide
/// grouping keys use the substrate's exact-key lookup, never its iteration order.
type Claim = ScratchWordKey<5>;
const _: () = assert!(Claim::BYTE_LEN <= crate::exec::scratch::MAX_INLINE_KEY);

pub(super) struct Pack {
    /// In head order, excluding the aggregate's output slot.
    groups: Vec<(usize, usize)>,
    output: usize,
    find: usize,
    table: Option<ScratchRelation>,
    next_group: u64,
    retained: usize,
}

impl Pack {
    pub(super) fn new(finds: &[FindSpec], output: usize, find: usize) -> Self {
        let mut value = Self {
            groups: Vec::new(),
            output,
            find,
            table: None,
            next_group: 0,
            retained: 0,
        };
        value.aim(finds, output);
        value
    }

    pub(super) fn aim(&mut self, finds: &[FindSpec], output: usize) {
        self.output = output;
        self.groups = finds
            .iter()
            .filter_map(|find| match find {
                FindSpec::Var { slot, width } if *slot != output => Some((*slot, *width)),
                _ => None,
            })
            .collect();
    }

    pub(super) fn reset(&mut self) {
        self.table = None;
        self.next_group = 0;
        self.retained = 0;
    }

    pub(super) fn observe(
        &mut self,
        bindings: &Bindings,
        program: &OutputProgram,
        generation: &GenerationHandle,
        work: &WorkContext,
    ) -> Result<()> {
        work.checkpoint()
            .map_err(super::super::source::work_error)?;
        let FindTerm::Pack { over } = program.expression else {
            unreachable!("sealed Event Pack program");
        };
        let (_, slot, _) = program.inputs[0];
        let words = [bindings.get(slot), bindings.get(slot + 1)];
        let interner = InternerHandle::new(generation, work);
        let event = interner.resolve_event(words)?;
        let context = event.space().full().to_bytes(work)?;
        let mut group = Vec::new();
        for &(slot, width) in &self.groups {
            for i in slot..slot + width {
                group.extend_from_slice(&bindings.get(i).to_be_bytes());
            }
        }
        let table = self.table.get_or_insert_with(|| ScratchRelation::new(work));
        let mut header = Vec::new();
        let token = if table.get_map(ScratchMapId::GroupToToken, &group, &mut header)? {
            word(&header)?
        } else {
            let token = self.next_group;
            self.next_group = token
                .checked_add(1)
                .ok_or(crate::Error::ResultBytesOverflow)?;
            table.put_map(ScratchMapId::GroupToToken, &group, &token.to_be_bytes())?;
            table.put_map(ScratchMapId::TokenToGroup, &token.to_be_bytes(), &group)?;
            self.retained = self
                .retained
                .saturating_add(group.len().saturating_mul(2) + 32);
            token
        };
        let context_present = table.get_map(
            ScratchMapId::EventPackContext,
            &token.to_be_bytes(),
            &mut header,
        )?;
        if !context_present || context.as_slice() < header.get(16..).ok_or_else(corrupt)? {
            let prior = header.len();
            header.clear();
            for key in words {
                header.extend_from_slice(&key.to_be_bytes());
            }
            header.extend_from_slice(&context);
            table.put_map(
                ScratchMapId::EventPackContext,
                &token.to_be_bytes(),
                &header,
            )?;
            self.retained = self
                .retained
                .saturating_sub(prior)
                .saturating_add(header.len());
        }
        for &rule in &program.rules {
            let claim = Claim::new([
                token,
                words[0],
                words[1],
                u64::from(rule),
                u64::from(over.0),
            ]);
            if table.insert_if_absent(&claim.encode(), &[])? {
                self.retained = self.retained.saturating_add(Claim::BYTE_LEN + 64);
            }
        }
        if !table.spilled() && (table.len() >= 4096 || self.retained >= 4 * 1024 * 1024) {
            table.force_spill()?;
        }
        Ok(())
    }

    /// Accumulate faults and tentative values together. The caller publishes
    /// neither until both computed heads and all Pack groups have been checked.
    pub(super) fn finish(
        &mut self,
        generation: &GenerationHandle,
        work: &WorkContext,
        stage: Option<usize>,
        faults: &mut Faults,
        inner: &mut EitherSink,
    ) -> Result<()> {
        let Some(mut table) = self.table.take() else {
            return Ok(());
        };
        let interner = InternerHandle::new(generation, work);
        let mut current: Option<u64> = None;
        let mut group = Vec::new();
        let mut context = Vec::new();
        let mut union: Option<Event> = None;
        let mut group_valid = true;
        table.visit_with_lookup(ScratchMapId::Default, &mut |lookup, key, _| {
            let [token, a, b, rule, variable] = Claim::decode(key).ok_or_else(corrupt)?.words();
            if current != Some(token) {
                if group_valid && let Some(value) = union.take() {
                    self.emit(&group, &value, &interner, inner)?;
                }
                if !lookup.get(ScratchMapId::TokenToGroup, &token.to_be_bytes(), &mut group)?
                    || !lookup.get(
                        ScratchMapId::EventPackContext,
                        &token.to_be_bytes(),
                        &mut context,
                    )?
                    || context.len() < 16
                {
                    return Err(corrupt());
                }
                let anchor =
                    interner.resolve_event([word(&context[..8])?, word(&context[8..16])?])?;
                union = Some(anchor.space().empty());
                current = Some(token);
                group_valid = true;
            }
            let value = interner.resolve_event([a, b])?;
            let accumulated = union.as_ref().expect("a claim has a group");
            match value.align_to(&accumulated.space(), work) {
                Ok(value) => {
                    // Context checks continue after saturation and after a fault.
                    if group_valid {
                        union = Some(accumulated.apply(crate::event::BoolOp4::OR, &value, work)?);
                    }
                }
                Err(crate::event::Error::SpaceMismatch) => {
                    group_valid = false;
                    faults.insert(EventOperandFault {
                        stage,
                        rule: u16::try_from(rule).map_err(|_| corrupt())?,
                        find: self.find,
                        operand: 0,
                        source: crate::EventOperandSource::Variable(VarId(
                            u16::try_from(variable).map_err(|_| corrupt())?,
                        )),
                        category: EventFaultCategory::SpaceMismatch,
                        expected_space: context[16..].into(),
                        offending_value: value.to_bytes(work)?.into_boxed_slice(),
                    })?;
                }
                Err(error) => return Err(error.into()),
            }
            Ok(true)
        })?;
        if group_valid && let Some(value) = union {
            self.emit(&group, &value, &interner, inner)?;
        }
        Ok(())
    }

    fn emit(
        &self,
        group: &[u8],
        value: &Event,
        interner: &InternerHandle<'_>,
        inner: &mut EitherSink,
    ) -> Result<()> {
        let total = self
            .groups
            .iter()
            .map(|(slot, width)| slot + width)
            .chain([self.output + 2])
            .max()
            .unwrap();
        let mut bindings = Bindings::new(total);
        let (bytes, remainder) = group.as_chunks::<8>();
        let mut bytes = bytes.iter();
        for &(slot, width) in &self.groups {
            for i in slot..slot + width {
                bindings.set(i, word(bytes.next().ok_or_else(corrupt)?)?);
            }
        }
        if bytes.next().is_some() || !remainder.is_empty() {
            return Err(corrupt());
        }
        let words = interner.intern_event(value)?.key().words();
        bindings.set(self.output, words[0]);
        bindings.set(self.output + 1, words[1]);
        inner.emit(&bindings);
        if let Some(error) = inner.take_error() {
            return Err(error);
        }
        Ok(())
    }
}

fn word(bytes: &[u8]) -> Result<u64> {
    Ok(u64::from_be_bytes(bytes.try_into().map_err(|_| corrupt())?))
}

fn corrupt() -> crate::Error {
    crate::event::Error::UnknownKey.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exec::sink::ProjectionSink;
    use crate::schema::ValueType;
    use crate::{
        Error,
        event::{Space, SpaceId},
    };

    #[test]
    fn wide_groups_preserve_values_and_faults_across_forced_spill_and_reaim() {
        let a = Space::new(SpaceId([51; 32]), 2, &()).unwrap();
        let b = Space::new(SpaceId([52; 32]), 2, &()).unwrap();
        let mut baseline = None;
        for spill in [false, true] {
            for bad in [false, true] {
                let work = WorkContext::new();
                let generation = crate::image::test_generation();
                let interner = InternerHandle::new(&generation, &work);
                let finds = [
                    FindSpec::Var { slot: 0, width: 64 },
                    FindSpec::Var { slot: 66, width: 2 },
                ];
                let mut pack = Pack::new(&finds, 66, 1);
                let program = OutputProgram {
                    find: 1,
                    rules: vec![0, 2],
                    expression: FindTerm::Pack { over: VarId(1) },
                    inputs: vec![(VarId(1), 64, ValueType::Event.into())],
                };
                let mut row = Bindings::new(68);
                for (i, event) in [
                    a.full(),
                    a.empty(),
                    a.empty(),
                    if bad { b.empty() } else { a.empty() },
                ]
                .iter()
                .enumerate()
                {
                    // Distinct oversized exact group keys have deliberately
                    // shared long prefixes. Groups 0 and 1 interleave.
                    row.set(63, (i % 2) as u64);
                    let words = interner.intern_event(event).unwrap().key().words();
                    row.set(64, words[0]);
                    row.set(65, words[1]);
                    pack.observe(&row, &program, &generation, &work).unwrap();
                    if spill && i == 0 {
                        pack.table.as_mut().unwrap().force_spill().unwrap();
                    }
                }
                assert_eq!(pack.table.as_ref().unwrap().spilled(), spill);
                // Final output slots can belong to a different rule layout.
                let finds = [
                    FindSpec::Var { slot: 4, width: 64 },
                    FindSpec::Var { slot: 0, width: 2 },
                ];
                pack.aim(&finds, 0);
                let mut inner =
                    EitherSink::Projection(ProjectionSink::with_capacity_hint(&finds, 68, 0));
                inner.begin_execution(Some(work.clone()));
                let mut faults = Faults::default();
                pack.finish(&generation, &work, Some(2), &mut faults, &mut inner)
                    .unwrap();
                if bad {
                    let Error::EventFaults(faults) = faults.finish().unwrap_err() else {
                        panic!("fault set");
                    };
                    assert_eq!(faults.len(), 2);
                    assert!(faults.iter().all(|f| f.stage == Some(2) && f.operand == 0));
                    if let Some(expected) = &baseline {
                        assert_eq!(&faults, expected);
                    } else {
                        baseline = Some(faults);
                    }
                } else {
                    faults.finish().unwrap();
                    let EitherSink::Projection(sink) = &mut inner else {
                        unreachable!();
                    };
                    let mut seen = Vec::new();
                    sink.for_each_answer(&mut |words| {
                        let value = interner.resolve_event([words[64], words[65]])?;
                        seen.push((words[63], value.is_full(), value.is_empty()));
                        Ok(())
                    })
                    .unwrap();
                    seen.sort_unstable();
                    assert_eq!(seen, vec![(0, true, false), (1, false, true)]);
                }
            }
        }
    }

    #[test]
    fn spill_cancellation_refuses_before_a_complete_fault_set_is_claimed() {
        let work = WorkContext::new();
        let generation = crate::image::test_generation();
        let interner = InternerHandle::new(&generation, &work);
        let finds = [FindSpec::Var { slot: 2, width: 2 }];
        let mut pack = Pack::new(&finds, 2, 0);
        let program = OutputProgram {
            find: 0,
            rules: vec![0],
            expression: FindTerm::Pack { over: VarId(0) },
            inputs: vec![(VarId(0), 0, ValueType::Event.into())],
        };
        let mut row = Bindings::new(4);
        for id in [53, 54] {
            let source = Space::new(SpaceId([id; 32]), 1, &()).unwrap();
            let words = interner.intern_event(&source.full()).unwrap().key().words();
            row.set(0, words[0]);
            row.set(1, words[1]);
            pack.observe(&row, &program, &generation, &work).unwrap();
        }
        pack.table.as_mut().unwrap().force_spill().unwrap();
        let mut inner = EitherSink::Projection(ProjectionSink::with_capacity_hint(&finds, 4, 0));
        let mut faults = Faults::default();
        work.cancel();
        let error = pack
            .finish(&generation, &work, None, &mut faults, &mut inner)
            .unwrap_err();
        assert!(!matches!(error, Error::EventFaults(_)));
        let EitherSink::Projection(sink) = inner else {
            unreachable!();
        };
        assert_eq!(sink.len(), 0);
    }
}
