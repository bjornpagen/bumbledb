//! Private link-test adapter, not a public C API or a shipped header surface.
//! The C driver links this object against the actual core static archive.

#[allow(dead_code)]
#[path = "../../examples/consumers/rust/src/main.rs"]
mod consumer;

/// Keep unwinding on the Rust side of the private C test boundary.
#[unsafe(no_mangle)]
pub extern "C" fn bumbledb_static_link_probe() -> i32 {
    match std::panic::catch_unwind(|| {
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let recovered = std::panic::catch_unwind(|| panic!("private static unwinder probe"));
        std::panic::set_hook(previous_hook);
        assert!(
            recovered.is_err(),
            "static Rust unwinder must catch the panic"
        );
        // Exercise musl pthread/TLS support as well as allocation, file locks,
        // real LMDB transactions, queries, exact floating aggregates and close.
        std::thread::spawn(|| consumer::run().map_err(|error| error.to_string()))
            .join()
            .expect("static consumer thread")
    }) {
        Ok(Ok(())) => 0,
        Ok(Err(error)) => {
            eprintln!("static consumer failed: {error}");
            1
        }
        Err(_) => 2,
    }
}
