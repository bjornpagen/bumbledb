use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bumbledb::{Db, Value};

use crate::schema::{Ledger, ids};

const FACTS: u64 = 2 * 4096 + 512;

fn iterations() -> u64 {
    std::env::var("BUMBLEDB_STRESS_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100)
}

fn rows() -> impl Iterator<Item = Vec<Value>> {
    (0..FACTS).map(|i| vec![Value::U64(i), Value::String(format!("holder-{i}").into())])
}

fn io_pressure(dir: std::path::PathBuf, stop: &Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
    let stop = Arc::clone(stop);
    std::thread::spawn(move || {
        use std::io::Write as _;
        let path = dir.join("pressure");
        let payload = vec![0xA5u8; 1 << 22];
        while !stop.load(Ordering::Relaxed) {
            let Ok(mut file) = std::fs::File::create(&path) else {
                return;
            };
            let _ = file.write_all(&payload);
            let _ = file.sync_all();
        }
        let _ = std::fs::remove_file(&path);
    })
}

fn cpu_pressure(stop: &Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
    let stop = Arc::clone(stop);
    std::thread::spawn(move || {
        let mut x = 0x9E37_79B9_7F4A_7C15u64;
        while !stop.load(Ordering::Relaxed) {
            for _ in 0..1 << 16 {
                x = std::hint::black_box(x.wrapping_mul(0x2545_F491_4F6C_DD1D).rotate_left(23));
            }
        }
    })
}

#[test]
fn collection_insert_survives_commit_pressure() {
    let iters = iterations();
    let root = std::env::temp_dir().join("bumbledb-bench-stress");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("stress root");

    let stop = Arc::new(AtomicBool::new(false));
    let mut pressure = Vec::new();
    for worker in 0..3 {
        let dir = root.join(format!("io-{worker}"));
        std::fs::create_dir_all(&dir).expect("pressure dir");
        pressure.push(io_pressure(dir, &stop));
    }
    for _ in 0..3 {
        pressure.push(cpu_pressure(&stop));
    }

    for i in 0..iters {
        let dir = root.join(format!("db-{i}"));
        let db = Db::create(&dir, Ledger)
            .expect("create store")
            .expect("accepted");
        let loaded = db
            .write(|tx| {
                tx.insert_dyn(ids::HOLDER, rows())
                    .map(bumbledb::MutationReport::changed)
            })
            .unwrap_or_else(|e| panic!("iteration {i}: {e}"))
            .unwrap()
            .value;
        assert_eq!(loaded, FACTS, "iteration {i}: short load");
        drop(db);
        std::fs::remove_dir_all(&dir).expect("scratch teardown");
    }

    stop.store(true, Ordering::Relaxed);
    for handle in pressure {
        handle.join().expect("pressure thread");
    }
    let _ = std::fs::remove_dir_all(&root);
}
