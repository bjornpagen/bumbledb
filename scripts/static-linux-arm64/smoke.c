/* Private static-link regression driver. No supported C API is exposed. */
#include <stdio.h>

extern int bumbledb_static_link_probe(void);

int main(void) {
    const int result = bumbledb_static_link_probe();
    if (result != 0) {
        fprintf(stderr, "static C/Rust/musl/LMDB probe failed: %d\n", result);
        return result;
    }
    puts("static C/Rust/musl/LMDB link probe: OK");
    return 0;
}
