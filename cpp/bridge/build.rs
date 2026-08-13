fn main() {
    println!("cargo:rerun-if-changed=../foreign/callback_trampoline.cc");
    cc::Build::new()
        .cpp(true)
        .file("../foreign/callback_trampoline.cc")
        .flag_if_supported("-fexceptions")
        .flag_if_supported("-std=c++17")
        .compile("bdb_callback_trampoline");
}
