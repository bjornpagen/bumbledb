#[test]
fn transaction_lifetime_ui_tests() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/ui/read_inside_closure.rs");
    tests.compile_fail("tests/ui/scan_escape.rs");
}
