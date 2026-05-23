use super::*;

#[derive(Clone, Debug)]
pub(super) struct EncodedInputs {
    pub(super) values: Vec<EncodedOwned>,
}

impl EncodedInputs {
    pub(super) fn get(&self, input: InputId) -> Option<&EncodedOwned> {
        self.values.get(input.0 as usize)
    }
}

#[derive(Clone, Debug)]
pub(super) struct EncodedBinding {
    values: SmallVec<[Option<EncodedOwned>; 8]>,
}

impl EncodedBinding {
    pub(super) fn new(variable_count: usize) -> Self {
        Self {
            values: std::iter::repeat_with(|| None)
                .take(variable_count)
                .collect(),
        }
    }

    pub(super) fn get(&self, variable: usize) -> Option<&EncodedOwned> {
        self.values[variable].as_ref()
    }

    pub(super) fn bind(&mut self, variable: usize, value: EncodedOwned) -> bool {
        match &self.values[variable] {
            Some(existing) => existing == &value,
            None => {
                self.values[variable] = Some(value);
                true
            }
        }
    }

    pub(super) fn unbind(&mut self, variable: usize) {
        self.values[variable] = None;
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ExecutionPlan {
    pub(super) comparisons: Vec<NormPredicate>,
    pub(super) summary: QueryPlan,
}

pub(super) struct LftjRuntime<'a> {
    pub(super) participants_by_variable: Vec<SmallParticipants>,
    pub(super) iters: Vec<LftjTrieIter<'a>>,
}

pub(super) type SmallParticipants = SmallVec<[usize; 4]>;
pub(super) type SmallEncodedFact = SmallVec<[EncodedOwned; 8]>;
pub(super) type LazyAccessShape = (Vec<u8>, usize, Vec<FieldId>);
