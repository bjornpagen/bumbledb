//! Scratch-backed payoff rosters. A stable token per logical group lets the
//! existing projection/scalar aggregate sinks compose with multiple expectations.
//! Context admission is a complete first pass. Exact payoff normalization and
//! partition admission run later under the shared observation arithmetic budget.
use super::{Bindings, OutputProgram, events::Faults};
use crate::event::{BoolOp4, FunctionLimits};
use crate::exec::scratch::{ScratchMapId, ScratchRelation, ScratchWordKey};
use crate::exec::sink::FindSpec;
use crate::image::intern::InternerHandle;
use crate::observation::ExpectationInput;
use crate::work::{GenerationHandle, WorkContext};
use crate::{
    Event, EventFaultCategory, EventOperandFault, FindIndex, FindTerm, Result, Value, VarId,
};

// token, tagged ratio/import payload, Event keys and written provenance.
// Ratios are not normalized here: arithmetic belongs to one finalization budget.
type Claim = ScratchWordKey<13>;
const _: () = assert!(Claim::BYTE_LEN <= crate::exec::scratch::MAX_INLINE_KEY);

pub(super) struct Expectations {
    groups: Vec<(usize, usize)>,
    table: Option<ScratchRelation>,
    next_group: u64,
    retained: usize,
    imports: Vec<crate::PayoffImport>,
    import_indices: std::collections::HashMap<crate::PayoffImport, u64>,
    import_bytes: usize,
}

impl Expectations {
    pub(super) fn new(
        finds: &[FindSpec],
        programs: &[(usize, std::sync::Arc<OutputProgram>)],
    ) -> Self {
        let mut value = Self {
            groups: Vec::new(),
            table: None,
            next_group: 0,
            retained: 0,
            imports: Vec::new(),
            import_indices: std::collections::HashMap::new(),
            import_bytes: 0,
        };
        value.aim(finds, programs);
        value
    }

    pub(super) fn aim(
        &mut self,
        finds: &[FindSpec],
        programs: &[(usize, std::sync::Arc<OutputProgram>)],
    ) {
        self.groups = finds
            .iter()
            .filter_map(|find| match find {
                FindSpec::Var { slot, width }
                    if !programs.iter().any(|(output, p)| {
                        output == slot
                            && matches!(
                                p.expression,
                                FindTerm::Expectation { .. } | FindTerm::Pack { .. }
                            )
                    }) =>
                {
                    Some((*slot, *width))
                }
                _ => None,
            })
            .collect();
    }

    pub(super) fn reset(&mut self) {
        self.table = None;
        self.next_group = 0;
        self.retained = 0;
        self.imports.clear();
        self.import_indices.clear();
        self.import_bytes = 0;
    }

    pub(super) fn observe(
        &mut self,
        bindings: &Bindings,
        program: &OutputProgram,
        generation: &GenerationHandle,
        work: &WorkContext,
    ) -> Result<u64> {
        work.checkpoint()
            .map_err(super::super::source::work_error)?;
        let FindTerm::Expectation { value, when, given } = &program.expression else {
            unreachable!("sealed expectation");
        };
        let payoff = if let crate::PayoffExpr::Imported(import) = value {
            [1, self.retain_import(import)?, 0, 0]
        } else {
            let [sign, numerator, denominator] = payoff_ratio(bindings, program, value);
            [0, sign, numerator, denominator]
        };
        let key = |var| {
            let (_, slot, _) = program
                .inputs
                .iter()
                .find(|(id, _, _)| *id == var)
                .expect("bound Event");
            [bindings.get(*slot), bindings.get(*slot + 1)]
        };
        let when_key = key(*when);
        let given_key = key(*given);
        let interner = InternerHandle::new(generation, work);
        let (anchor, context) = context_anchor(&interner, [when_key, given_key], value, work)?;
        let mut group = (program.find as u64).to_be_bytes().to_vec();
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
            self.retained = self.retained.saturating_add(group.len() + 32);
            token
        };
        let present = table.get_map(
            ScratchMapId::EventPackContext,
            &token.to_be_bytes(),
            &mut header,
        )?;
        if !present || context.as_slice() < header.get(16..).ok_or_else(corrupt)? {
            let prior = header.len();
            header.clear();
            for word in anchor {
                header.extend_from_slice(&word.to_be_bytes());
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
                payoff[0],
                payoff[1],
                payoff[2],
                payoff[3],
                when_key[0],
                when_key[1],
                given_key[0],
                given_key[1],
                u64::from(rule),
                u64::from(when.0),
                u64::from(given.0),
                program.find as u64,
            ]);
            if table.insert_if_absent(&claim.encode(), &[])? {
                self.retained = self.retained.saturating_add(Claim::BYTE_LEN + 64);
            }
        }
        if !table.spilled() && (table.len() >= 4096 || self.retained >= 4 * 1024 * 1024) {
            table.force_spill()?;
        }
        Ok(token)
    }

    fn retain_import(&mut self, import: &crate::PayoffImport) -> Result<u64> {
        if let Some(index) = self.import_indices.get(import) {
            return Ok(*index);
        }
        let bytes = self
            .import_bytes
            .checked_add(import.bytes().len())
            .filter(|n| *n <= 16 * 1024 * 1024)
            .ok_or(crate::event::Error::Capacity(
                crate::event::Capacity::DescriptorBytes,
            ))?;
        self.imports
            .try_reserve(1)
            .map_err(crate::event::Error::from)?;
        self.import_indices
            .try_reserve(1)
            .map_err(crate::event::Error::from)?;
        let index = self.imports.len() as u64;
        self.imports.push(import.clone());
        self.import_indices.insert(import.clone(), index);
        self.import_bytes = bytes;
        Ok(index)
    }

    pub(super) fn finish(
        &mut self,
        generation: &GenerationHandle,
        work: &WorkContext,
        stage: Option<usize>,
        faults: &mut Faults,
    ) -> Result<Vec<ExpectationInput>> {
        let Some(mut table) = self.table.take() else {
            return Ok(Vec::new());
        };
        let interner = InternerHandle::new(generation, work);
        admit_contexts(&mut table, &interner, &self.imports, work, stage, faults)?;
        if !faults.is_empty() {
            return Ok(Vec::new());
        }
        collect_rosters(&mut table, &interner, &self.imports, work)
    }
}

fn context_anchor(
    interner: &InternerHandle<'_>,
    keys: [[u64; 2]; 2],
    value: &crate::PayoffExpr,
    work: &WorkContext,
) -> Result<([u64; 2], Vec<u8>)> {
    let [a, b] = keys.map(|words| {
        let event = interner.resolve_event(words)?;
        Ok::<_, crate::Error>((words, event.space().full().to_bytes(work)?))
    });
    let a = a?;
    let b = b?;
    let (mut anchor, mut context) = if a.1 <= b.1 { a } else { b };
    if let crate::PayoffExpr::Imported(import) = value
        && let Some(marker) = import.marker()
        && marker < context.as_slice()
    {
        anchor = interner
            .intern_event(&import.space().expect("function space").full())?
            .key()
            .words();
        context = marker.to_vec();
    }
    Ok((anchor, context))
}

// Capture full signed magnitudes, including u64::MAX / -1. Exact reduction is
// deferred until all contexts have been checked; a zero divisor stays visible.
fn payoff_ratio(
    bindings: &Bindings,
    program: &OutputProgram,
    value: &crate::PayoffExpr,
) -> [u64; 3] {
    let integer = |var| match super::read_value(bindings, program, var).expect("typed integer") {
        Value::I64(n) => (n < 0, n.unsigned_abs()),
        Value::U64(n) => (false, n),
        _ => unreachable!("validated exact integer"),
    };
    let ((negative, magnitude), (divisor_negative, denominator)) = match value {
        crate::PayoffExpr::Imported(_) => unreachable!("import capture"),
        crate::PayoffExpr::Integer(var) => (integer(*var), (false, 1)),
        crate::PayoffExpr::Ratio {
            numerator,
            denominator,
        } => (integer(*numerator), integer(*denominator)),
    };
    let sign = u64::from(magnitude != 0 && (negative ^ divisor_negative));
    [sign, magnitude, denominator]
}

fn admit_contexts(
    table: &mut ScratchRelation,
    interner: &InternerHandle<'_>,
    imports: &[crate::PayoffImport],
    work: &WorkContext,
    stage: Option<usize>,
    faults: &mut Faults,
) -> Result<()> {
    let mut context = Vec::new();
    let mut current = None;
    let mut space = None;
    // Validate every written occurrence, including zero and empty inputs.
    table.visit_with_lookup(ScratchMapId::Default, &mut |lookup, key, _| {
        let [
            token,
            kind,
            payload,
            _,
            _,
            a,
            b,
            c,
            d,
            rule,
            when_var,
            given_var,
            find,
        ] = Claim::decode(key).ok_or_else(corrupt)?.words();
        if current != Some(token) {
            if !lookup.get(
                ScratchMapId::EventPackContext,
                &token.to_be_bytes(),
                &mut context,
            )? || context.len() < 16
            {
                return Err(corrupt());
            }
            space = Some(
                interner
                    .resolve_event([word(&context[..8])?, word(&context[8..16])?])?
                    .space(),
            );
            current = Some(token);
        }
        for (operand, words, variable) in [(0, [a, b], when_var), (1, [c, d], given_var)] {
            let event = interner.resolve_event(words)?;
            match event.align_to(space.as_ref().expect("group context"), work) {
                Ok(_) => {}
                Err(crate::event::Error::SpaceMismatch) => {
                    faults.insert(EventOperandFault {
                        stage,
                        rule: u16::try_from(rule).map_err(|_| corrupt())?,
                        find: usize::try_from(find).map_err(|_| corrupt())?,
                        operand,
                        source: crate::EventOperandSource::Variable(VarId(
                            u16::try_from(variable).map_err(|_| corrupt())?,
                        )),
                        category: EventFaultCategory::SpaceMismatch,
                        expected_space: context[16..].into(),
                        offending_value: event.to_bytes(work)?.into_boxed_slice(),
                    })?;
                }
                Err(error) => return Err(error.into()),
            }
        }
        if kind == 1 {
            let import = imports
                .get(usize::try_from(payload).map_err(|_| corrupt())?)
                .ok_or_else(corrupt)?;
            if let Some(source) = import.space() {
                match source
                    .full()
                    .align_to(space.as_ref().expect("group context"), work)
                {
                    Ok(_) => {}
                    Err(crate::event::Error::SpaceMismatch) => {
                        faults.insert(EventOperandFault {
                            stage,
                            rule: u16::try_from(rule).map_err(|_| corrupt())?,
                            find: usize::try_from(find).map_err(|_| corrupt())?,
                            operand: 2,
                            source: crate::EventOperandSource::PayoffImport,
                            category: EventFaultCategory::SpaceMismatch,
                            expected_space: context[16..].into(),
                            offending_value: import.bytes().into(),
                        })?;
                    }
                    Err(error) => return Err(error.into()),
                }
            }
        }
        Ok(true)
    })?;
    Ok(())
}

fn collect_rosters(
    table: &mut ScratchRelation,
    interner: &InternerHandle<'_>,
    imports: &[crate::PayoffImport],
    work: &WorkContext,
) -> Result<Vec<ExpectationInput>> {
    let mut inputs = Vec::new();
    let mut group: Option<Group> = None;
    table.visit_with_lookup(ScratchMapId::Default, &mut |_, key, _| {
        let [
            token,
            kind,
            payload,
            numerator,
            denominator,
            a,
            b,
            c,
            d,
            _,
            _,
            _,
            find,
        ] = Claim::decode(key).ok_or_else(corrupt)?.words();
        if group.as_ref().is_none_or(|g| g.token != token) {
            if let Some(prior) = group.take() {
                inputs.try_reserve(1).map_err(crate::event::Error::from)?;
                inputs.push(prior.finish(imports)?);
            }
            group = Some(Group {
                token,
                given: interner.resolve_event([c, d])?,
                values: Vec::new(),
            });
        }
        let group = group.as_mut().expect("claim group");
        let space = group.given.space();
        let given = interner.resolve_event([c, d])?.align_to(&space, work)?;
        if given != group.given {
            return Err(crate::Error::ExpectationEvidenceMismatch {
                find: FindIndex(usize::try_from(find).map_err(|_| corrupt())?),
            });
        }
        let when = interner.resolve_event([a, b])?.align_to(&space, work)?;
        if let Some((_, prior)) = group
            .values
            .last_mut()
            .filter(|(value, _)| *value == [kind, payload, numerator, denominator])
        {
            *prior = prior.apply(BoolOp4::OR, &when, work)?;
        } else {
            if group.values.len() >= FunctionLimits::default().cells {
                return Err(
                    crate::event::Error::Capacity(crate::event::Capacity::FunctionCells).into(),
                );
            }
            group
                .values
                .try_reserve(1)
                .map_err(crate::event::Error::from)?;
            group
                .values
                .push(([kind, payload, numerator, denominator], when));
        }
        Ok(true)
    })?;
    if let Some(group) = group {
        inputs.try_reserve(1).map_err(crate::event::Error::from)?;
        inputs.push(group.finish(imports)?);
    }
    Ok(inputs)
}

struct Group {
    token: u64,
    given: Event,
    // Inline keys group identical presentations. Equal ratios merge at admission.
    values: Vec<([u64; 4], Event)>,
}

impl Group {
    fn finish(self, imports: &[crate::PayoffImport]) -> Result<ExpectationInput> {
        let mut payoffs = Vec::new();
        payoffs
            .try_reserve_exact(self.values.len())
            .map_err(crate::event::Error::from)?;
        for ([kind, payload, numerator, denominator], region) in self.values {
            let value = if kind == 0 {
                crate::observation::PayoffInput::Ratio([payload, numerator, denominator])
            } else {
                crate::observation::PayoffInput::Imported(
                    imports
                        .get(usize::try_from(payload).map_err(|_| corrupt())?)
                        .ok_or_else(corrupt)?
                        .clone(),
                )
            };
            payoffs.push((value, region));
        }
        Ok(ExpectationInput {
            given: self.given,
            payoffs,
        })
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
    use crate::event::{Space, SpaceId};
    use crate::schema::ValueType;
    use std::sync::Arc;

    fn program() -> Arc<OutputProgram> {
        Arc::new(OutputProgram {
            find: 1,
            rules: vec![0, 2],
            expression: FindTerm::Expectation {
                value: VarId(0).into(),
                when: VarId(1),
                given: VarId(2),
            },
            inputs: vec![
                (VarId(0), 64, ValueType::I64),
                (VarId(1), 65, ValueType::Event),
                (VarId(2), 67, ValueType::Event),
            ],
        })
    }

    #[test]
    fn expectations_preserve_wide_groups_faults_and_rosters_through_spill_and_reaim() {
        let source = Space::new(SpaceId([71; 32]), 1, &()).unwrap();
        let foreign = Space::new(SpaceId([72; 32]), 1, &()).unwrap();
        let mut baseline = None;
        for spill in [false, true] {
            for bad in [false, true] {
                let work = WorkContext::new();
                let generation = crate::image::test_generation();
                let interner = InternerHandle::new(&generation, &work);
                let finds = [
                    FindSpec::Var { slot: 0, width: 64 },
                    FindSpec::Var { slot: 69, width: 1 },
                ];
                let program = program();
                let mut collector = Expectations::new(&finds, &[(69, program.clone())]);
                let mut row = Bindings::new(70);
                for i in 0..4 {
                    row.set(63, i % 2);
                    row.set(64, 1 << 63); // supplied zero, never an omitted value
                    let region = if i < 2 {
                        source.full()
                    } else if bad {
                        foreign.empty()
                    } else {
                        source.empty()
                    };
                    for (slot, value) in [(65, region), (67, source.full())] {
                        let words = interner.intern_event(&value).unwrap().key().words();
                        row.set(slot, words[0]);
                        row.set(slot + 1, words[1]);
                    }
                    assert_eq!(
                        collector
                            .observe(&row, &program, &generation, &work)
                            .unwrap(),
                        i % 2
                    );
                    if i == 0 && spill {
                        collector.table.as_mut().unwrap().force_spill().unwrap();
                    }
                }
                assert_eq!(collector.table.as_ref().unwrap().spilled(), spill);
                // A later union arm changes slots without losing group tokens.
                collector.aim(
                    &[
                        FindSpec::Var { slot: 4, width: 64 },
                        FindSpec::Var { slot: 0, width: 1 },
                    ],
                    &[(0, program.clone())],
                );
                let mut faults = Faults::default();
                let inputs = collector
                    .finish(&generation, &work, Some(3), &mut faults)
                    .unwrap();
                if bad {
                    assert!(inputs.is_empty());
                    let crate::Error::EventFaults(faults) = faults.finish().unwrap_err() else {
                        panic!("fault set");
                    };
                    assert_eq!(faults.len(), 2); // same written occurrences across groups deduplicate
                    assert!(faults.iter().all(|f| f.stage == Some(3) && f.operand == 0));
                    if let Some(expected) = &baseline {
                        assert_eq!(&faults, expected);
                    } else {
                        baseline = Some(faults);
                    }
                } else {
                    faults.finish().unwrap();
                    assert_eq!(inputs.len(), 2);
                    for input in inputs {
                        let input = input
                            .admit(
                                &work,
                                &mut crate::event::ExactArithmetic::new(
                                    crate::event::ArithmeticLimits::default(),
                                    &work,
                                ),
                            )
                            .unwrap();
                        let crate::ExpectationPayoff::Scalar { values, partition } = input else {
                            panic!("scalar")
                        };
                        assert_eq!(values, [crate::event::ExactRational::zero()]);
                        assert!(partition.cells()[0].is_full());
                    }
                }
            }
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn imported_payoffs_survive_spill_reaim_and_reset_without_losing_owners() {
        use crate::event::{
            ArithmeticLimits, ExactArithmetic, ExactRational, FiniteFunction,
            SourceDescriptorLimits,
        };
        let source = Space::new(SpaceId([76; 32]), 1, &()).unwrap();
        let function = FiniteFunction::constant(
            &source,
            ExactRational::from(7u64),
            FunctionLimits::default(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &()),
        )
        .unwrap();
        let import = crate::PayoffImport::capture(
            crate::ImportedPayoff::Finite(function),
            SourceDescriptorLimits::default(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &()),
        )
        .unwrap();
        for spill in [false, true] {
            let work = WorkContext::new();
            let generation = crate::image::test_generation();
            let interner = InternerHandle::new(&generation, &work);
            let mut program = (*program()).clone();
            program.expression = FindTerm::Expectation {
                value: crate::PayoffExpr::Imported(import.clone()),
                when: VarId(1),
                given: VarId(2),
            };
            program.inputs.remove(0);
            let program = Arc::new(program);
            let finds = [
                FindSpec::Var { slot: 0, width: 64 },
                FindSpec::Var { slot: 69, width: 1 },
            ];
            let mut collector = Expectations::new(&finds, &[(69, program.clone())]);
            for turn in 0..2 {
                let offset = if turn == 0 { 0 } else { 4 };
                if turn == 1 {
                    collector.aim(
                        &[
                            FindSpec::Var { slot: 4, width: 64 },
                            FindSpec::Var { slot: 70, width: 1 },
                        ],
                        &[(70, program.clone())],
                    );
                }
                let mut relocated = (*program).clone();
                relocated.inputs = vec![
                    (VarId(1), 71, ValueType::Event),
                    (VarId(2), 73, ValueType::Event),
                ];
                for group in 0..2 {
                    let mut row = Bindings::new(75);
                    row.set(offset + 63, group);
                    let head = source.coordinate(0, &()).unwrap();
                    for (slot, region) in [
                        (71, if turn == 0 { head } else { head.complement() }),
                        (73, source.full()),
                    ] {
                        let words = interner.intern_event(&region).unwrap().key().words();
                        row.set(slot, words[0]);
                        row.set(slot + 1, words[1]);
                    }
                    assert_eq!(
                        collector
                            .observe(&row, &relocated, &generation, &work)
                            .unwrap(),
                        group
                    );
                }
                if turn == 0 && spill {
                    collector.table.as_mut().unwrap().force_spill().unwrap();
                }
            }
            assert_eq!(collector.imports.len(), 1);
            assert_eq!(collector.import_bytes, import.bytes().len());
            assert_eq!(collector.table.as_ref().unwrap().spilled(), spill);
            let mut faults = Faults::default();
            let inputs = collector
                .finish(&generation, &work, None, &mut faults)
                .unwrap();
            faults.finish().unwrap();
            collector.reset();
            assert_eq!(collector.import_bytes, 0);
            assert!(collector.imports.is_empty() && collector.import_indices.is_empty());
            assert_eq!(inputs.len(), 2);
            for input in inputs {
                let crate::ExpectationPayoff::Finite(cover) = input
                    .admit(
                        &work,
                        &mut ExactArithmetic::new(ArithmeticLimits::default(), &work),
                    )
                    .unwrap()
                else {
                    panic!("finite cover")
                };
                assert_eq!(cover.patches().len(), 1);
                assert!(cover.patches()[0].region.is_full());
                assert_eq!(
                    cover.function().at(0, &()).unwrap(),
                    ExactRational::from(7u64)
                );
            }
        }
    }

    #[test]
    fn expectation_checks_all_contexts_before_coverage_or_evidence_and_cancels_spill() {
        let source = Space::new(SpaceId([73; 32]), 1, &()).unwrap();
        let foreign = Space::new(SpaceId([74; 32]), 1, &()).unwrap();
        for cancel in [false, true] {
            let work = WorkContext::new();
            let generation = crate::image::test_generation();
            let interner = InternerHandle::new(&generation, &work);
            let program = program();
            let mut collector = Expectations::new(
                &[
                    FindSpec::Var { slot: 0, width: 64 },
                    FindSpec::Var { slot: 69, width: 1 },
                ],
                &[(69, program.clone())],
            );
            let mut row = Bindings::new(70);
            for (group, region, evidence) in [
                (0, source.empty(), source.full()),
                (1, source.empty(), source.full()),
                (1, foreign.empty(), source.empty()),
            ] {
                row.set(63, group);
                row.set(64, 1 << 63);
                for (slot, value) in [(65, region), (67, evidence)] {
                    let words = interner.intern_event(&value).unwrap().key().words();
                    row.set(slot, words[0]);
                    row.set(slot + 1, words[1]);
                }
                collector
                    .observe(&row, &program, &generation, &work)
                    .unwrap();
            }
            collector.table.as_mut().unwrap().force_spill().unwrap();
            if cancel {
                work.cancel();
            }
            let mut faults = Faults::default();
            let result = collector.finish(&generation, &work, None, &mut faults);
            if cancel {
                assert!(result.is_err());
            } else {
                assert!(result.unwrap().is_empty());
                assert!(matches!(faults.finish(), Err(crate::Error::EventFaults(_))));
            }
        }
    }
}
