//! The computed-output sink adapter: typed scalar stage outputs
//! (`FindTerm::Compute`) and interval segments, evaluated per surviving
//! binding after every input predicate of the rule. It never licenses a
//! suffix skip or a fused leaf scan: every output sees its complete binding.
//! Scalar outputs run as compiled programs over 64 bindings at a time, one
//! SIMD dispatch per chunk. Errors are sticky: the first failing binding's
//! error is recorded, later rows are dropped, and finalize publishes nothing.
//! The float environment is checked once per execution, at reset.
use std::sync::Arc;

use super::EitherSink;
use crate::exec::kernel::numeric::DefaultFloatEnvironment;
use crate::exec::run::{Bindings, Flow, LeafBatch, LeafSource, Sink};
use crate::exec::sink::FindSpec;
use crate::schema::ValueType;
use crate::{Error, F64, FindIndex, FindTerm, Value, VarId};

mod program;

use program::{Errors, LANES, Program, Register};

#[cfg(test)]
mod tests;

/// One computed find's sealed program: the find position (diagnostics),
/// the validated expression, and its inputs — per referenced variable,
/// the binding slot it reads and the type its word decodes as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputProgram {
    pub(crate) find: usize,
    pub(crate) expression: FindTerm,
    pub(crate) inputs: Vec<(VarId, usize, ValueType)>,
}

/// One output slot and how it is filled.
struct Output {
    slot: usize,
    program: Arc<OutputProgram>,
    /// The compiled scalar program and its first input column; `None` for
    /// interval segments.
    compiled: Option<(Program, usize)>,
}

/// The adapter: fills each appended output slot, then forwards the widened
/// binding row to the inner sink (projection or aggregate), whose find
/// specs were lowered to read those slots.
pub(in crate::api) struct ComputedSink {
    pub(super) inner: EitherSink,
    outputs: Vec<Output>,
    /// The binding slot behind each input column of the compiled programs.
    column_slots: Vec<usize>,
    bindings: Bindings,
    /// The rule's real slot count; slots at and past it are outputs.
    slots: usize,
    pieces: Vec<(usize, [[u64; 2]; 2], usize, usize)>,
    /// The first scalar failure of this execution; sticky until reset.
    pub(super) error: Option<Error>,
    /// This execution's float environment, if it is the IEEE default.
    float: Option<DefaultFloatEnvironment>,
    chunk: Chunk,
}

/// Per-chunk registers: binding columns in, one result and error set per
/// compiled output out.
#[derive(Default)]
struct Chunk {
    columns: Vec<Register>,
    stack: Vec<Register>,
    results: Vec<Register>,
    errors: Vec<Errors>,
    /// Real-slot sources of the current batch.
    sources: Vec<LeafSource>,
}

/// A lowered find-spec list: the rewritten specs, the `(slot, program)`
/// pairs that fill the appended output slots, and the widened slot count.
pub(super) type Lowered = (Vec<FindSpec>, Vec<(usize, Arc<OutputProgram>)>, usize);

/// Lowers a find-spec list for the inner sink: every `Compute` becomes a
/// fresh appended `Var` slot, and the programs that fill those slots are
/// returned beside the widened slot count.
pub(super) fn lower(finds: &[FindSpec], slots: usize) -> Lowered {
    let mut next = slots;
    let mut programs = Vec::new();
    let finds = finds
        .iter()
        .map(|find| match find {
            FindSpec::Compute(program) => {
                let slot = next;
                let width = output_width(program);
                next += width;
                programs.push((slot, Arc::clone(program)));
                FindSpec::Var { slot, width }
            }
            other => other.clone(),
        })
        .collect();
    (finds, programs, next)
}

fn output_width(program: &OutputProgram) -> usize {
    if matches!(program.expression, FindTerm::Segments { .. }) {
        2
    } else {
        1
    }
}

impl ComputedSink {
    pub(super) fn new(
        inner: EitherSink,
        programs: Vec<(usize, Arc<OutputProgram>)>,
        slots: usize,
        total: usize,
    ) -> Self {
        let mut sink = Self {
            inner,
            outputs: Vec::new(),
            column_slots: Vec::new(),
            bindings: Bindings::new(total),
            slots,
            error: None,
            float: None,
            pieces: Vec::new(),
            chunk: Chunk::default(),
        };
        sink.install(programs);
        sink
    }

    /// Compiles `programs`, reusing the compiled form of any program the
    /// sink already holds, and sizes the chunk registers.
    fn install(&mut self, programs: Vec<(usize, Arc<OutputProgram>)>) {
        let previous = std::mem::take(&mut self.outputs);
        self.column_slots.clear();
        for (slot, program) in programs {
            let compiled = match &program.expression {
                FindTerm::Compute(expression) => {
                    let first = self.column_slots.len();
                    for (_, input, ty) in &program.inputs {
                        self.column_slots.push(*input);
                        if ty.interval_element().is_some() {
                            self.column_slots.push(input + 1);
                        }
                    }
                    let reused = previous
                        .iter()
                        .find(|output| Arc::ptr_eq(&output.program, &program))
                        .and_then(|output| output.compiled.as_ref())
                        .map(|(compiled, _)| compiled.clone());
                    let compiled = reused.unwrap_or_else(|| {
                        Program::compile(expression, |var| input_column(&program, first, var))
                    });
                    Some((compiled, first))
                }
                _ => None,
            };
            self.outputs.push(Output {
                slot,
                program,
                compiled,
            });
        }
        self.size_chunk();
    }

    /// Sizes the chunk registers for the installed programs.
    fn size_chunk(&mut self) {
        let compiled = self.outputs.iter().filter_map(|o| o.compiled.as_ref());
        let depth = compiled.clone().map(|(p, _)| p.depth()).max().unwrap_or(0);
        let count = compiled.count();
        let chunk = &mut self.chunk;
        chunk.columns.resize(self.column_slots.len(), [0; LANES]);
        chunk.stack.resize(depth, [0; LANES]);
        chunk.results.resize(count, [0; LANES]);
        chunk.errors.resize_with(count, Errors::new);
    }

    pub(super) fn reset(&mut self) {
        self.error = None;
        self.float = DefaultFloatEnvironment::check().ok();
        self.size_chunk();
        if self.bindings.slot_count() == 0 {
            self.bindings.resize(
                self.slots
                    + self
                        .outputs
                        .iter()
                        .map(|output| output_width(&output.program))
                        .sum::<usize>(),
            );
        }
        self.inner.reset();
    }

    pub(super) fn release_memory(&mut self) {
        self.error = None;
        self.bindings = Bindings::new(0);
        self.pieces = Vec::new();
        self.chunk = Chunk::default();
        self.inner.release_memory();
    }

    pub(super) fn aim(&mut self, finds: &[FindSpec], slots: usize, shared: &[(usize, usize)]) {
        let (finds, programs, total) = lower(finds, slots);
        self.slots = slots;
        self.install(programs);
        self.bindings.resize(total);
        self.inner.aim(&finds, total, shared);
    }

    /// Runs every compiled output over the first `lanes` lanes of the
    /// gathered columns.
    fn compute(&mut self, lanes: usize) {
        let Self {
            outputs,
            chunk,
            float,
            ..
        } = self;
        let float = *float;
        fearless_simd::dispatch!(crate::exec::kernel::level(), simd => {
            let compiled = outputs.iter().filter_map(|o| o.compiled.as_ref());
            for (((program, _), result), errors) in
                compiled.zip(&mut chunk.results).zip(&mut chunk.errors)
            {
                *errors = Errors::new();
                program.run(simd, lanes, &chunk.columns, float, &mut chunk.stack, result, errors);
            }
        });
    }

    /// Fills the output slots of lane `lane` and emits its rows. Interval
    /// pieces come first: a binding with an empty piece is eliminated before
    /// any scalar output can fail. The Cartesian product of pieces is walked
    /// as an odometer.
    fn row(&mut self, lane: usize) {
        if self.error.is_some() {
            return;
        }
        self.pieces.clear();
        for output in &self.outputs {
            let FindTerm::Segments { op, left, right } = &output.program.expression else {
                continue;
            };
            let a = read_interval(&self.bindings, &output.program, *left);
            let b = read_interval(&self.bindings, &output.program, *right);
            let segments = match (a, b) {
                (Value::IntervalU64(a), Value::IntervalU64(b)) => {
                    op.apply(a, b).map(|s| s.map(|s| [s.start(), s.end()]))
                }
                (Value::IntervalI64(a), Value::IntervalI64(b)) => op.apply(a, b).map(|s| {
                    s.map(|s| {
                        [
                            s.start().cast_unsigned() ^ (1 << 63),
                            s.end().cast_unsigned() ^ (1 << 63),
                        ]
                    })
                }),
                (Value::IntervalF64(a), Value::IntervalF64(b)) => op
                    .apply(a, b)
                    .map(|s| s.map(|s| [s.start().to_order_key(), s.end().to_order_key()])),
                _ => unreachable!("validated same-kind intervals"),
            };
            let mut values = [[0; 2]; 2];
            let mut len = 0;
            for segment in segments.into_iter().flatten() {
                values[len] = segment;
                len += 1;
            }
            if len == 0 {
                return;
            }
            self.pieces.push((output.slot, values, len, 0));
        }
        let compiled = self.outputs.iter().filter(|o| o.compiled.is_some());
        for ((output, result), errors) in compiled.zip(&self.chunk.results).zip(&self.chunk.errors)
        {
            if let Some(source) = errors.of(lane) {
                self.error = Some(Error::Scalar {
                    find: FindIndex(output.program.find),
                    source,
                });
                return;
            }
            self.bindings.set(output.slot, result[lane]);
        }
        loop {
            for &(slot, values, _, position) in &self.pieces {
                self.bindings.set(slot, values[position][0]);
                self.bindings.set(slot + 1, values[position][1]);
            }
            if self.inner.emit(&self.bindings).is_terminal() {
                return;
            }
            let mut advanced = false;
            for (_, _, len, position) in self.pieces.iter_mut().rev() {
                *position += 1;
                if *position < *len {
                    advanced = true;
                    break;
                }
                *position = 0;
            }
            if !advanced {
                return;
            }
        }
    }

    fn flow_after_row(&self) -> Flow {
        if self.error.is_some() {
            Flow::Error
        } else {
            Flow::from_sink_progress(self.inner.progress())
        }
    }
}

impl Sink for ComputedSink {
    fn emit(&mut self, bindings: &Bindings) -> Flow {
        for slot in 0..self.slots {
            self.bindings.set(slot, bindings.get(slot));
        }
        for (column, &slot) in self.chunk.columns.iter_mut().zip(&self.column_slots) {
            column[0] = bindings.get(slot);
        }
        self.compute(1);
        self.row(0);
        self.flow_after_row()
    }

    fn emit_batch(&mut self, batch: &LeafBatch<'_>) -> Flow {
        let mut sources = std::mem::take(&mut self.chunk.sources);
        sources.clear();
        sources.extend((0..self.slots).map(|slot| batch.source_of(slot)));
        let word = |slot: usize, entry: u32| match sources[slot] {
            LeafSource::Key(word) => batch.key(entry, word),
            LeafSource::Outer => batch.bindings.get(slot),
        };
        for entries in batch.survivors.chunks(LANES) {
            if self.flow_after_row().is_terminal() {
                break;
            }
            for (column, &slot) in self.chunk.columns.iter_mut().zip(&self.column_slots) {
                for (lane, &entry) in entries.iter().enumerate() {
                    column[lane] = word(slot, entry);
                }
            }
            self.compute(entries.len());
            for (lane, &entry) in entries.iter().enumerate() {
                if self.flow_after_row().is_terminal() {
                    break;
                }
                for slot in 0..self.slots {
                    self.bindings.set(slot, word(slot, entry));
                }
                self.row(lane);
            }
        }
        self.chunk.sources = sources;
        self.flow_after_row()
    }

    fn progress(&self) -> crate::exec::sink::SinkProgress {
        if self.error.is_some() {
            crate::exec::sink::SinkProgress::Error
        } else {
            self.inner.progress()
        }
    }

    fn take_error(&mut self) -> Option<crate::error::Error> {
        self.error.take().or_else(|| self.inner.take_error())
    }
}

/// The first column of `var` in a program whose input columns start at
/// `first`, with the variable's type.
fn input_column(program: &OutputProgram, first: usize, var: VarId) -> Option<(usize, ValueType)> {
    let mut column = first;
    for &(id, _, ty) in &program.inputs {
        if id == var {
            return Some((column, ty));
        }
        column += if ty.interval_element().is_some() {
            2
        } else {
            1
        };
    }
    None
}

fn read_interval(bindings: &Bindings, program: &OutputProgram, var: VarId) -> Value {
    let &(_, slot, ty) = program
        .inputs
        .iter()
        .find(|(id, _, _)| *id == var)
        .expect("validated segment input");
    let (start, end) = (bindings.get(slot), bindings.get(slot + 1));
    match ty.interval_element().expect("validated interval input") {
        crate::schema::IntervalElement::U64 => {
            Value::IntervalU64(crate::Interval::new(start, end).expect("valid interval binding"))
        }
        crate::schema::IntervalElement::I64 => Value::IntervalI64(
            crate::Interval::new(
                (start ^ (1 << 63)).cast_signed(),
                (end ^ (1 << 63)).cast_signed(),
            )
            .expect("valid interval binding"),
        ),
        crate::schema::IntervalElement::F64 => Value::IntervalF64(
            crate::Interval::new(
                F64::from_order_key(start).expect("canonical endpoint"),
                F64::from_order_key(end).expect("canonical endpoint"),
            )
            .expect("valid interval binding"),
        ),
    }
}
