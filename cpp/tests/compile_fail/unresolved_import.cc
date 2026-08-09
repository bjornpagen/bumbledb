// Harness seed case: an ill-formed import — no rule in the module graph
// provides `bumbledb_nonexistent`, so the build must fail at module
// collation with the module name in the diagnostic.
import bumbledb_nonexistent;
