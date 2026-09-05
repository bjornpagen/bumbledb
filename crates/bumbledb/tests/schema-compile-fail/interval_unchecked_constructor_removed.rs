//@ error: no associated function or constant named `__ground_axiom`

fn main() {
    let _ = bumbledb::Interval::<u64>::__ground_axiom(9, 1);
}
