//! Authored F1, executed F3. Gate mapping: `model_*`/`census_*`/`sqlite_*` →
//! SPACE-01; `variants_*` → SPACE-02.

use std::collections::BTreeSet;

use rusqlite::Connection;

use super::census::{self, CensusSource, EntrySize, NamespaceCensus, PageStats};
use super::sqlite_stat;
use super::variants::{self, Axis, Baseline};
use super::{NAMESPACES, Namespace, audited_layout, current_layout, successor_layout};

#[test]
fn live_tags_match_keys_rs() {
    assert_eq!(Namespace::from_census_tag(false, 0x01), Namespace::Fact);
    assert_eq!(Namespace::from_census_tag(false, 0x02), Namespace::Membership);
    assert_eq!(Namespace::from_census_tag(false, 0x03), Namespace::Determinant);
    assert_eq!(Namespace::from_census_tag(true, 0x01), Namespace::HostMeta);
    assert_eq!(Namespace::from_census_tag(false, 0xFF), Namespace::Unknown);
    assert_eq!(current_layout::KEY_BYTES_FACT_MEMBERSHIP_FP_DET, 69);
}

// SPACE-01 — the raw-byte model reproduces chapter 41's bill.

#[test]
fn model_reproduces_the_chapter41_entry_table() {
    // Historical 0.x bill — attribution only.
    assert_eq!(audited_layout::fact_entry(24), 37);
    assert_eq!(audited_layout::MEMBERSHIP_ENTRY, 45);
    assert_eq!(audited_layout::determinant_entry(8), 23);
    assert_eq!(audited_layout::reverse_edge_entry(8, false), 23);
    assert_eq!(audited_layout::reverse_edge_entry(8, true), 31);
    assert_eq!(audited_layout::DICT_FORWARD_ENTRY, 41);
    assert_eq!(audited_layout::dict_reverse_entry(12), 21);
    assert_eq!(audited_layout::dict_total(12), 62);
}

#[test]
fn current_layout_matches_live_keys_and_compiled_overhead() {
    use super::current_layout;
    assert_eq!(current_layout::ROW_KEY, 13);
    assert_eq!(current_layout::MEMBERSHIP_ENTRY, 29);
    assert_eq!(current_layout::determinant_exact_u64(), 19);
    assert_eq!(current_layout::determinant_fingerprint(), 27);
    assert_eq!(current_layout::KEY_BYTES_FACT_MEMBERSHIP_FP_DET, 69);
    assert_eq!(current_layout::fact_membership_fp_det(24), 24 + 69);
    assert_eq!(current_layout::DETERMINANT_OVERHEAD, 11);
}

#[test]
fn model_reproduces_the_chapter41_worked_example() {
    // Hypothetical non-text 24-byte row, one 8-byte determinant, one 8-byte
    // unweighted reverse edge: 24 + 58 + 23 + 23 = 128 raw bytes.
    let total = audited_layout::fact_plus_membership(24)
        + audited_layout::determinant_entry(8)
        + audited_layout::reverse_edge_entry(8, false);
    assert_eq!(total, 128);
    // And the F+M law itself: W + 58.
    assert_eq!(audited_layout::fact_plus_membership(0), 58);
}

#[test]
fn model_successor_membership_moves_the_row_id_without_duplicating_it() {
    // (relation, fingerprint16, row8) key + empty value = 29; the audited
    // entry was 45; the saving is exactly the 16 truncated digest bytes.
    assert_eq!(successor_layout::MEMBERSHIP_ENTRY, 29);
    assert_eq!(successor_layout::MEMBERSHIP_SAVING_PER_FACT, 16);
}

// SPACE-01 — census mechanics over a synthetic source.

struct FakeSource {
    entries: Vec<EntrySize>,
    pages: PageStats,
}

impl CensusSource for FakeSource {
    fn walk(&mut self, visit: &mut dyn FnMut(EntrySize)) -> Result<(), String> {
        for entry in &self.entries {
            visit(*entry);
        }
        Ok(())
    }

    fn page_stats(&mut self) -> Result<PageStats, String> {
        Ok(self.pages)
    }
}

fn scratch_file(tag: &str, len: usize) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("bumbledb-bench-space-tests");
    std::fs::create_dir_all(&dir).expect("tempdir");
    let path = dir.join(tag);
    std::fs::write(&path, vec![0xA5u8; len]).expect("write");
    path
}

#[test]
fn census_accumulates_per_namespace_and_separates_the_four_measures() {
    let path = scratch_file("census-data.mdb", 8192);
    let mut source = FakeSource {
        entries: vec![
            EntrySize {
                namespace: Namespace::Fact,
                key_bytes: 13,
                value_bytes: 24,
            },
            EntrySize {
                namespace: Namespace::Fact,
                key_bytes: 13,
                value_bytes: 40,
            },
            EntrySize {
                namespace: Namespace::Membership,
                key_bytes: 29,
                value_bytes: 0,
            },
            EntrySize {
                namespace: Namespace::HostMeta,
                key_bytes: 7,
                value_bytes: 8,
            },
        ],
        pages: PageStats {
            page_size: 4096,
            depth: 2,
            branch_pages: 1,
            leaf_pages: 3,
            overflow_pages: 0,
            entries: 4,
            free_pages: 2,
        },
    };
    let report = census::run(&mut source, &path).expect("census runs");
    let facts = report.namespace(Namespace::Fact);
    assert_eq!(
        facts,
        NamespaceCensus {
            entries: 2,
            key_bytes: 26,
            value_bytes: 64
        }
    );
    assert_eq!(report.namespace(Namespace::Membership).raw_bytes(), 29);
    assert_eq!(report.namespace(Namespace::HostMeta).entries, 1);
    assert_eq!(report.namespace(Namespace::Unknown).entries, 0);
    assert_eq!(report.live_raw_bytes(), 26 + 64 + 29 + 15);
    // Page accounting: used vs freelist vs live raw vs file vs allocated
    // are all distinct numbers.
    assert_eq!(report.pages.used_page_bytes(), 4 * 4096);
    assert_eq!(report.pages.free_page_bytes(), 2 * 4096);
    assert_eq!(
        report.page_overhead_bytes(),
        4 * 4096 - report.live_raw_bytes()
    );
    assert_eq!(report.file_bytes, 8192);
    // Fully written file: allocated blocks are a distinct measure from length.
    // Do not treat "nonzero" as success — require the split exists and
    // allocated is at least the written length on this Unix temp file.
    assert_eq!(report.file_bytes, 8192);
    assert!(
        report.allocated_bytes >= report.file_bytes,
        "dense write: allocated {} < length {}",
        report.allocated_bytes,
        report.file_bytes
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn census_page_overhead_saturates_instead_of_underflowing() {
    let report = census::StoreCensus {
        per_namespace: {
            let mut cells = [NamespaceCensus::default(); NAMESPACES.len()];
            cells[0] = NamespaceCensus {
                entries: 1,
                key_bytes: 1_000_000,
                value_bytes: 0,
            };
            cells
        },
        pages: PageStats {
            page_size: 4096,
            leaf_pages: 1,
            ..PageStats::default()
        },
        file_bytes: 0,
        allocated_bytes: 0,
        virtual_map_bytes: None,
        live_transactions: None,
    };
    assert_eq!(
        report.page_overhead_bytes(),
        0,
        "inconsistent walks saturate"
    );
}

#[test]
fn census_allocated_bytes_reports_sparse_files_smaller_than_length() {
    // A sparse file: length large, blocks small. APFS/ext4 both support seek
    // holes through set_len.
    let dir = std::env::temp_dir().join("bumbledb-bench-space-tests");
    std::fs::create_dir_all(&dir).expect("tempdir");
    let path = dir.join("sparse.bin");
    let file = std::fs::File::create(&path).expect("create");
    file.set_len(64 * 1024 * 1024).expect("set_len");
    drop(file);
    let length = std::fs::metadata(&path).expect("stat").len();
    let allocated = census::allocated_bytes(&path).expect("blocks");
    assert_eq!(length, 64 * 1024 * 1024);
    assert!(
        allocated < length,
        "a hole-punched file must show fewer allocated bytes ({allocated}) than length ({length})"
    );
    let _ = std::fs::remove_file(&path);
}

// SPACE-01 — SQLite side: roster and page census are real reads.

#[test]
fn sqlite_census_reports_pages_freelist_and_the_actual_index_roster() {
    let conn = Connection::open_in_memory().expect("open");
    conn.execute_batch(
        "CREATE TABLE t(id INTEGER PRIMARY KEY, k INTEGER, v BLOB);\
         CREATE INDEX t_k ON t(k);\
         CREATE UNIQUE INDEX t_kv ON t(k, v);\
         INSERT INTO t(k, v) VALUES (1, x'00'), (2, x'01'), (3, x'02');",
    )
    .expect("ddl");
    let report = sqlite_stat::census(&conn).expect("census");
    assert!(report.page_size >= 512);
    assert!(report.page_count > 0);
    assert_eq!(
        report.freelist_bytes(),
        report.page_size * report.freelist_count
    );
    let names: BTreeSet<&str> = report.indexes.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains("t_k"), "explicit index in the roster");
    assert!(names.contains("t_kv"));
    for index in &report.indexes {
        assert_eq!(index.table, "t");
    }
    // dbstat either works or refuses with its build-flag reason — never an
    // empty success.
    match &report.dbstat {
        Ok(stats) => assert!(!stats.is_empty(), "dbstat success must carry rows"),
        Err(reason) => assert!(
            reason.contains("dbstat"),
            "refusal names the mechanism: {reason}"
        ),
    }
}

// SPACE-02 — the variant matrix.

#[test]
fn variants_matrix_covers_all_axes_and_baselines_without_netting() {
    let matrix = variants::matrix();
    assert!(
        matrix
            .iter()
            .any(|cell| cell.axis == Axis::FingerprintWidth && cell.baseline == Baseline::Audited0x)
    );
    assert!(
        matrix
            .iter()
            .any(|cell| cell.axis == Axis::IdWidth && cell.baseline == Baseline::Audited0x)
    );
    assert!(
        matrix.iter().any(
            |cell| cell.axis == Axis::IdWidth && cell.baseline == Baseline::Superseded28ByteIds
        )
    );
    let text_cells = matrix
        .iter()
        .filter(|cell| cell.axis == Axis::TextLayout)
        .count();
    assert!(
        text_cells >= 2,
        "inline-text needs both repeated-label and unique-churn populations"
    );
    for cell in &matrix {
        assert_eq!(
            cell.regimes,
            &variants::REQUIRED_REGIMES,
            "{}: every variant reports every required regime",
            cell.name
        );
    }
}

#[test]
fn variants_id_width_deltas_keep_their_baselines_apart() {
    assert_eq!(variants::id_width_delta(Baseline::Audited0x), 8);
    assert_eq!(variants::id_width_delta(Baseline::Superseded28ByteIds), -12);
    // The forbidden fiction: (-12) + something is never a "net saving"
    // against the audited tree. Against Audited0x the sign is positive.
    assert!(variants::id_width_delta(Baseline::Audited0x) > 0);
}

#[test]
fn variants_fingerprint_arithmetic_matches_the_chapter41_prose() {
    assert_eq!(variants::fingerprint_saving_bytes(1_000_000), 16_000_000);
    assert_eq!(
        variants::fingerprint_saving_bytes(100_000_000),
        1_600_000_000
    );
    // Illustrative shares of the historical bytes/row (arithmetic, not a
    // measured file reduction): 16/167 ~= 9.6%, 16/228 ~= 7.0%.
    assert!((16.0_f64 / 167.0 - 0.0958).abs() < 0.001);
    assert!((16.0_f64 / 228.0 - 0.0702).abs() < 0.001);
}
