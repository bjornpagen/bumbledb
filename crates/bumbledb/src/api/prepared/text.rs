//! Production activation of L04's scratch-backed text resolver.
//!
//! Resident intern/image admit [`ResidentAdmit::Ready`]. On
//! [`ResidentAdmit::BeyondMemory`] execute calls
//! [`ResidentTextExhausted::open_nonresident`] — never
//! [`NonresidentTextStore::bind`] / `new`. Scratch binds the **execute**
//! ledger via [`ScratchCapability::on_work`].

use crate::error::{CorruptionError, Error, Result};
use crate::exec::scratch::capability::{ScratchCapability, ScratchPolicy};
use crate::image::canon::{RowWords, TextWords};
use crate::image::intern::InternerHandle;
use crate::image::view::{Const, FilterPredicate, Operands};
use crate::image::{
    is_resident_token, is_scratch_token, NonresidentTextStore, ResidentAdmit, ResidentTextExhausted,
};
use crate::ir::WordCmp;
use crate::work::WorkContext;
use bumbledb_theory::schema::FieldDescriptor;

use super::source::work_error;

/// Share the already-running execute ledger. Never `start` a twin.
pub(super) fn scratch_capability(work: &WorkContext) -> Result<ScratchCapability> {
    ScratchCapability::on_work(work, ScratchPolicy::from_work(work)).map_err(work_error)
}

/// The one production constructor: L04's refusal handle, not `bind`/`new`.
pub(super) fn open_from_exhausted(
    exhausted: &ResidentTextExhausted,
    work: &WorkContext,
) -> Result<NonresidentTextStore> {
    Ok(exhausted.open_nonresident(&scratch_capability(work)?))
}

pub(super) fn install<'a>(
    slot: &'a mut Option<NonresidentTextStore>,
    exhausted: &ResidentTextExhausted,
    work: &WorkContext,
) -> Result<&'a mut NonresidentTextStore> {
    if slot.is_none() {
        *slot = Some(open_from_exhausted(exhausted, work)?);
    }
    Ok(slot.as_mut().expect("just opened"))
}

/// Intern through the resident handle; on spill open scratch via `exhausted`.
pub(super) fn intern_admitted(
    interner: &InternerHandle<'_>,
    slot: &mut Option<NonresidentTextStore>,
    text: &str,
    work: &WorkContext,
) -> Result<u64> {
    match interner.intern_or_spill(text)? {
        ResidentAdmit::Ready(token) => Ok(token),
        ResidentAdmit::BeyondMemory(exhausted) => install(slot, &exhausted, work)?.intern(text, work),
    }
}

/// Decode one row; on intern spill open scratch and retry that row.
pub(super) fn decode_row(
    row: &mut RowWords,
    fields: &[FieldDescriptor],
    bytes: &[u8],
    interner: &InternerHandle<'_>,
    store: &mut Option<NonresidentTextStore>,
    work: &WorkContext,
    intern: bool,
) -> Result<()> {
    loop {
        let mut text = match store.as_mut() {
            Some(store) => TextWords::Nonresident { store, work },
            None if intern => TextWords::HandleIntern(interner),
            None => TextWords::HandleLookup(interner),
        };
        match row.decode(fields, bytes, &mut text)? {
            ResidentAdmit::Ready(()) => return Ok(()),
            ResidentAdmit::BeyondMemory(exhausted) => {
                drop(text);
                install(store, &exhausted, work)?;
            }
        }
    }
}

/// Dispatch on the token tag. Never try intern then store.
pub(super) fn resolve_tagged(
    interner: &InternerHandle<'_>,
    store: Option<&mut NonresidentTextStore>,
    token: u64,
    write: impl FnOnce(&str),
) -> Result<Option<usize>> {
    if is_scratch_token(token) {
        let Some(store) = store else {
            return Ok(None);
        };
        let mut bytes = Vec::new();
        if !store.resolve(token, &mut bytes)? {
            return Ok(None);
        }
        let text = std::str::from_utf8(&bytes).map_err(|_| {
            Error::Corruption(CorruptionError::MalformedValue("nonresident text"))
        })?;
        let len = text.len();
        write(text);
        return Ok(Some(len));
    }
    if is_resident_token(token) {
        return Ok(interner.with_text(token, |text| {
            let len = text.len();
            write(text);
            len
        }));
    }
    Ok(None)
}

/// Production equality: L04 [`TextEq::tokens_equal`]. Dispatch still
/// uses this signature (`Option<&mut _>`); it does not mutate the store.
pub(crate) fn text_tokens_equal(
    interner: &InternerHandle<'_>,
    store: Option<&mut NonresidentTextStore>,
    left: u64,
    right: u64,
) -> Result<bool> {
    interner
        .generation()
        .text_eq(store.as_deref())
        .tokens_equal(left, right)
}

/// Resolve one tagged token to owned text. Sentinel / unknown → `None`.
pub(crate) fn owned_text(
    interner: &InternerHandle<'_>,
    store: Option<&mut NonresidentTextStore>,
    token: u64,
) -> Result<Option<Box<str>>> {
    let mut out = None;
    let found = resolve_tagged(interner, store, token, |text| {
        out = Some(Box::<str>::from(text));
    })?;
    Ok(found.and(out))
}

/// Live intern or scratch text. Numeric words may share the scratch bit
/// pattern; only a dictionary hit or `store.live` is text.
fn live_text_word(
    interner: &InternerHandle<'_>,
    store: Option<&NonresidentTextStore>,
    word: u64,
) -> bool {
    if is_resident_token(word) {
        return interner.with_text(word, |_| ()).is_some();
    }
    store.is_some_and(|store| store.live(word))
}

/// One equality for parameters, literals, joins, and negation.
/// [`TextEq::tokens_equal`] is the text verdict. Raw identity remains
/// for words that are not live text (numeric filters, i64 high-bit).
pub(super) fn words_equal(
    interner: &InternerHandle<'_>,
    store: &mut Option<NonresidentTextStore>,
    left: u64,
    right: u64,
) -> Result<bool> {
    if interner
        .generation()
        .text_eq(store.as_ref())
        .tokens_equal(left, right)?
    {
        return Ok(true);
    }
    if live_text_word(interner, store.as_ref(), left)
        || live_text_word(interner, store.as_ref(), right)
    {
        return Ok(false);
    }
    Ok(left == right)
}

/// Grouping/dedup identity. Stale scratch tokens do not group.
pub(super) fn canonical_token(
    interner: &InternerHandle<'_>,
    store: Option<&NonresidentTextStore>,
    token: u64,
) -> Result<Option<u64>> {
    interner.generation().text_eq(store).canonical(token)
}

/// Filter/join/negation: L04 [`crate::image::view::holds`] with this
/// execution's [`crate::image::TextEq`].
pub(super) fn holds_with_text<O: Operands>(
    predicate: &FilterPredicate,
    ops: &O,
    params: &[Const],
    interner: &InternerHandle<'_>,
    store: &mut Option<NonresidentTextStore>,
) -> Result<Option<bool>>
where
    Error: From<O::Error>,
{
    crate::image::view::holds(
        predicate,
        ops,
        params,
        interner.generation().text_eq(store.as_ref()),
    )
    .map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::db::InstanceBuilder;
    use crate::api::prepared::{Answers, BindValue, PreparedQuery};
    use crate::image::intern::InternerHandle;
    use crate::image::view::{Const, FilterPredicate, Loaded, OperandAddr, Operands};
    use crate::image::{ResidentAdmit, is_resident_token, is_scratch_token};
    use crate::ir::{Atom, AtomSource, FindTerm, Query, Rule, Term, VarId};
    use crate::schema::Theory;
    use crate::work::{CacheLedger, CachePolicy, GenerationHandle, GenerationState};
    use bumbledb_theory::schema::{
        FieldDescriptor, FieldId, RelationDescriptor, RelationId, SchemaDescriptor, ValueType,
    };

    struct WordRow(u64);
    impl Operands for WordRow {
        type Error = std::convert::Infallible;
        fn word(&self, _: OperandAddr) -> Result<u64, Self::Error> {
            Ok(self.0)
        }
        fn pair(&self, _: OperandAddr) -> Result<(u64, u64), Self::Error> {
            unreachable!("filter equality test is a word compare")
        }
        fn block(&self, _: OperandAddr) -> Result<([u64; 8], u8), Self::Error> {
            unreachable!("filter equality test is a word compare")
        }
        fn loaded(&self, _: OperandAddr) -> Result<Loaded, Self::Error> {
            Ok(Loaded::Word(self.0))
        }
    }

    fn work() -> crate::work::WorkContext {
        crate::api::prepared::source::UNBOUNDED_POLICY
            .start()
            .expect("work")
    }

    /// Force real cache exhaustion. Same string is equal across intern
    /// `0` and scratch `TAG` on the filter/join equality (`holds_with_text`).
    /// Verification: NotRun.
    #[test]
    fn filter_join_equality_survives_cache_exhaustion() {
        let work = work();
        let fat = GenerationHandle::new(GenerationState::new(
            crate::image::CacheGeneration::initial(),
            CacheLedger::unbounded(),
        ));
        let intern_tok = match InternerHandle::new(&fat, &work)
            .intern_or_spill("shared")
            .expect("fat")
        {
            ResidentAdmit::Ready(tok) => tok,
            ResidentAdmit::BeyondMemory(_) => panic!("unbounded cache must admit"),
        };
        assert_eq!(intern_tok, 0);
        assert!(is_resident_token(intern_tok));

        let tiny = GenerationHandle::new(GenerationState::new(
            crate::image::CacheGeneration::initial(),
            CacheLedger::new(CachePolicy { cache_bytes: 8 }),
        ));
        let ResidentAdmit::BeyondMemory(exhausted) = InternerHandle::new(&tiny, &work)
            .intern_or_spill("a-text-that-cannot-fit-eight-cache-bytes")
            .expect("spill")
        else {
            panic!("tiny cache intern must spill");
        };
        let mut store = open_from_exhausted(&exhausted, &work).expect("open");
        let scratch_tok = store.intern("shared", &work).expect("scratch");
        assert!(is_scratch_token(scratch_tok));
        assert_ne!(intern_tok, scratch_tok);
        assert_eq!(
            canonical_token(&InternerHandle::new(&fat, &work), Some(&store), scratch_tok)
                .expect("canonical"),
            Some(intern_tok),
            "TextEq::canonical maps scratch to the intern alias"
        );

        let handle = InternerHandle::new(&fat, &work);
        let mut slot = Some(store);
        let filter = FilterPredicate::Compare {
            field: OperandAddr::from_slot(0),
            op: WordCmp::Eq,
            value: Const::Word(intern_tok),
        };
        let hit = holds_with_text(
            &filter,
            &WordRow(scratch_tok),
            &[],
            &handle,
            &mut slot,
        )
        .expect("filter")
        .expect("verdict");
        assert!(
            hit,
            "filter/join path must treat intern 0 and scratch TAG as the same text"
        );
        assert!(words_equal(&handle, &mut slot, intern_tok, scratch_tok).expect("eq"));
        let miss = FilterPredicate::Compare {
            field: OperandAddr::from_slot(0),
            op: WordCmp::Eq,
            value: Const::Word(intern_tok),
        };
        let other = slot.as_mut().expect("store").intern("other", &work).expect("other");
        assert!(!holds_with_text(&miss, &WordRow(other), &[], &handle, &mut slot)
            .expect("ne")
            .expect("verdict"));
    }

    /// Numeric words are not intern tokens. A U64/i64 compare must not
    /// depend on the dictionary. Verification: NotRun.
    #[test]
    fn numeric_words_equal_without_intern() {
        let work = work();
        let fat = GenerationHandle::new(GenerationState::new(
            crate::image::CacheGeneration::initial(),
            CacheLedger::unbounded(),
        ));
        let handle = InternerHandle::new(&fat, &work);
        let mut slot = None;
        assert!(words_equal(&handle, &mut slot, 7, 7).expect("eq"));
        assert!(!words_equal(&handle, &mut slot, 7, 8).expect("ne"));
        let i64_zero = 1u64 << 63;
        assert!(words_equal(&handle, &mut slot, i64_zero, i64_zero).expect("i64 0"));
        let filter = FilterPredicate::Compare {
            field: OperandAddr::from_slot(0),
            op: WordCmp::Eq,
            value: Const::Word(7),
        };
        assert!(holds_with_text(&filter, &WordRow(7), &[], &handle, &mut slot)
            .expect("filter")
            .expect("verdict"));
    }

    /// Store/work refusal during text compare fails the operation.
    /// It is not inequality. Verification: NotRun.
    #[test]
    fn text_compare_refusal_is_not_inequality() {
        let work = work();
        let fat = GenerationHandle::new(GenerationState::new(
            crate::image::CacheGeneration::initial(),
            CacheLedger::unbounded(),
        ));
        let intern_tok = match InternerHandle::new(&fat, &work)
            .intern_or_spill("shared")
            .expect("fat")
        {
            ResidentAdmit::Ready(tok) => tok,
            ResidentAdmit::BeyondMemory(_) => panic!("unbounded cache must admit"),
        };
        let tiny = GenerationHandle::new(GenerationState::new(
            crate::image::CacheGeneration::initial(),
            CacheLedger::new(CachePolicy { cache_bytes: 8 }),
        ));
        let ResidentAdmit::BeyondMemory(exhausted) = InternerHandle::new(&tiny, &work)
            .intern_or_spill("a-text-that-cannot-fit-eight-cache-bytes")
            .expect("spill")
        else {
            panic!("tiny cache intern must spill");
        };
        let mut store = open_from_exhausted(&exhausted, &work).expect("open");
        let scratch_tok = store.intern("shared", &work).expect("scratch");
        work.cancel();
        let handle = InternerHandle::new(&fat, &work);
        let mut slot = Some(store);
        assert!(
            words_equal(&handle, &mut slot, intern_tok, scratch_tok).is_err(),
            "resolver refusal is not inequality"
        );
        let filter = FilterPredicate::Compare {
            field: OperandAddr::from_slot(0),
            op: WordCmp::Eq,
            value: Const::Word(intern_tok),
        };
        assert!(
            holds_with_text(&filter, &WordRow(scratch_tok), &[], &handle, &mut slot).is_err(),
            "holds refusal fails the compare"
        );
    }

    #[derive(Clone)]
    struct MemoTheory(SchemaDescriptor);
    impl Theory for MemoTheory {
        fn descriptor(self) -> SchemaDescriptor {
            self.0
        }
    }

    fn memo_descriptor() -> SchemaDescriptor {
        SchemaDescriptor {
            relations: vec![RelationDescriptor {
                extension: None,
                name: "Posting".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "account".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "memo".into(),
                        value_type: ValueType::String,
                    },
                    FieldDescriptor {
                        name: "amount".into(),
                        value_type: ValueType::I64,
                    },
                ],
            }],
            statements: vec![bumbledb_theory::schema::StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::new([FieldId(0)]),
            }],
        }
    }

    fn amounts(buffer: &Answers) -> Vec<i64> {
        (0..buffer.len())
            .map(|answer| match buffer.get(answer, 0) {
                crate::api::prepared::AnswerValue::I64(amount) => amount,
                _ => panic!("amount column"),
            })
            .collect()
    }

    /// Reuse one prepared query with repeated and changed text parameters.
    /// Exact answers stay correct. Verification: NotRun.
    #[test]
    fn reused_prepared_query_changed_text_params_stay_exact() {
        let mut builder =
            InstanceBuilder::new(MemoTheory(memo_descriptor()), work()).expect("schema");
        builder
            .load_dyn(
                RelationId(0),
                [
                    vec![
                        crate::ir::Value::U64(1),
                        crate::ir::Value::U64(7),
                        crate::ir::Value::String("alpha".into()),
                        crate::ir::Value::I64(10),
                    ],
                    vec![
                        crate::ir::Value::U64(2),
                        crate::ir::Value::U64(7),
                        crate::ir::Value::String("beta".into()),
                        crate::ir::Value::I64(20),
                    ],
                ]
                .iter(),
            )
            .expect("load");
        let instance = builder
            .admit()
            .expect("admit")
            .expect("law-abiding");
        let query = Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: AtomSource::Edb(RelationId(0)),
                bindings: vec![
                    (FieldId(2), Term::Param(crate::ir::ParamId(0))),
                    (FieldId(3), Term::Var(VarId(0))),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        });
        let mut prepared: PreparedQuery<MemoTheory> = instance.prepare(&query).expect("prepare");
        prepared.force_cursor_fallback(true);
        let mut out = Answers::new();
        prepared
            .execute_owned(&instance, &[BindValue::Str("alpha")], &mut out)
            .expect("alpha");
        assert_eq!(amounts(&out), vec![10]);
        prepared
            .execute_owned(&instance, &[BindValue::Str("alpha")], &mut out)
            .expect("alpha again");
        assert_eq!(amounts(&out), vec![10]);
        prepared
            .execute_owned(&instance, &[BindValue::Str("beta")], &mut out)
            .expect("changed");
        assert_eq!(amounts(&out), vec![20]);
        prepared
            .execute_owned(&instance, &[BindValue::Str("alpha")], &mut out)
            .expect("alpha restored");
        assert_eq!(amounts(&out), vec![10]);
    }

    /// Forced work refusal during a text query fails; it does not
    /// return a successful empty or wrong answer. Verification: NotRun.
    #[test]
    fn text_compare_refusal_fails_the_query() {
        let mut builder =
            InstanceBuilder::new(MemoTheory(memo_descriptor()), work()).expect("schema");
        builder
            .load_dyn(
                RelationId(0),
                [vec![
                    crate::ir::Value::U64(1),
                    crate::ir::Value::U64(7),
                    crate::ir::Value::String("alpha".into()),
                    crate::ir::Value::I64(10),
                ]]
                .iter(),
            )
            .expect("load");
        let instance = builder
            .admit()
            .expect("admit")
            .expect("law-abiding");
        let query = Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: AtomSource::Edb(RelationId(0)),
                bindings: vec![
                    (FieldId(2), Term::Param(crate::ir::ParamId(0))),
                    (FieldId(3), Term::Var(VarId(0))),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        });
        let mut prepared: PreparedQuery<MemoTheory> = instance.prepare(&query).expect("prepare");
        prepared.force_cursor_fallback(true);
        let work = work();
        work.cancel();
        let source = crate::api::prepared::source::QuerySource::heap(&instance, 1, work);
        let mut out = Answers::new();
        let err = prepared.execute_source(&source, &[BindValue::Str("alpha")], &mut out);
        assert!(err.is_err(), "refusal must fail the query");
    }

    /// A scratch `TAG` resolved under one store must not decode as that
    /// text after the store is forgotten and a new store mints `TAG`
    /// for different bytes. Verification: NotRun.
    #[test]
    fn scratch_tag_does_not_reuse_prior_text() {
        let work = work();
        let tiny = GenerationHandle::new(GenerationState::new(
            crate::image::CacheGeneration::initial(),
            CacheLedger::new(CachePolicy { cache_bytes: 8 }),
        ));
        let ResidentAdmit::BeyondMemory(exhausted) = InternerHandle::new(&tiny, &work)
            .intern_or_spill("a-text-that-cannot-fit-eight-cache-bytes")
            .expect("spill")
        else {
            panic!("tiny cache intern must spill");
        };
        let handle = InternerHandle::new(&tiny, &work);
        let mut memo = super::super::ResolveMemo::new();
        let mut answers = Answers::new();

        let mut first = open_from_exhausted(&exhausted, &work).expect("first store");
        let tag = first.intern("first-text", &work).expect("first");
        assert!(is_scratch_token(tag));
        let first_epoch = first.epoch();
        let (start, len) = memo
            .resolve(&handle, Some(&mut first), tag, &mut answers)
            .expect("resolve first");
        assert_eq!(&answers.text[start..start + len], "first-text");
        memo.forget_scratch();
        drop(first);

        let mut second = open_from_exhausted(&exhausted, &work).expect("second store");
        let tag2 = second.intern("second-text", &work).expect("second");
        assert!(is_scratch_token(tag2));
        assert_ne!(first_epoch, second.epoch());
        assert!(!second
            .text_eq()
            .with_memo_stamp(first_epoch)
            .accepts_stamp(first_epoch));
        assert!(second.text_eq().accepts_stamp(second.epoch()));
        assert_eq!(crate::image::scratch_token_epoch(tag2), None);
        assert_ne!(tag, tag2, "a later store has a new owner epoch");
        let (start, len) = memo
            .resolve(&handle, Some(&mut second), tag2, &mut answers)
            .expect("resolve second");
        assert_eq!(&answers.text[start..start + len], "second-text");
        assert_eq!(memo.uncharged_copy_bytes(), 0);
    }

    /// Live memo/dictionary memory is bounded: resolve does not keep an
    /// uncharged copy of every interned string. Verification: NotRun.
    #[test]
    fn live_memo_dictionary_memory_is_bounded() {
        let work = work();
        let fat = GenerationHandle::new(GenerationState::new(
            crate::image::CacheGeneration::initial(),
            CacheLedger::unbounded(),
        ));
        let handle = InternerHandle::new(&fat, &work);
        let mut memo = super::super::ResolveMemo::new();
        let mut answers = Answers::new();
        for index in 0..32u32 {
            let text = format!("interned-{index:04}");
            let ResidentAdmit::Ready(tok) = handle.intern_or_spill(&text).expect("intern") else {
                panic!("unbounded cache must admit");
            };
            let (start, len) = memo
                .resolve(&handle, None, tok, &mut answers)
                .expect("resolve");
            assert_eq!(&answers.text[start..start + len], text);
        }
        assert_eq!(
            memo.uncharged_copy_bytes(),
            0,
            "resolve must not retain an uncharged intern dictionary"
        );
        memo.clear();
        memo.forget_scratch();
        assert_eq!(memo.uncharged_copy_bytes(), 0);
    }
}
