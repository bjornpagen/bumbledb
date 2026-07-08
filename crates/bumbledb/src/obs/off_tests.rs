#[test]
fn the_guard_is_a_zst_when_off() {
    assert_eq!(std::mem::size_of::<super::SpanGuard>(), 0);
}
