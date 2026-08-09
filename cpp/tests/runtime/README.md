# tests/runtime/

Ordinary runtime unit tests of the C++ SDK modules: plain executables
registered with CTest, no framework. Tests that import reflection-using
modules from `meta/` are excluded from the Clang lint graph and carry the
scoped `-Wno-shadow` for the GCC 16.1 `template for` quirk where they
instantiate expansion statements.
