use std::time::Instant;

use super::stats::stats;
use super::{Measurement, Protocol};

/// The cold protocol, defined exactly: per sample (warmups included),
/// # Errors
pub fn measure_cold<T, F>(proto: Protocol, mut touch: T, mut f: F) -> Result<Measurement, String>
where
    T: FnMut() -> Result<(), String>,
    F: FnMut() -> Result<u64, String>,
{
    let mut samples = Vec::with_capacity(proto.samples as usize);
    let mut work = 0u64;
    for round in 0..proto.warmups + proto.samples {
        touch()?;

        crate::clockproxy::warm_up(std::time::Duration::from_millis(2));
        let start = Instant::now();
        let count = f()?;
        let elapsed = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
        if round >= proto.warmups {
            samples.push(elapsed);
            work += std::hint::black_box(count);
        }
    }
    Ok(Measurement {
        stats: stats(&mut samples),
        work,
        p50_norm: None,
        alloc: None,
        trace: None,
    })
}

/// # Panics
pub fn org_touch(
    db: &bumbledb::Db<crate::schema::Ledger>,
) -> impl FnMut() -> Result<(), String> + '_ {
    move || {
        db.write(|tx| {
            let id: crate::schema::OrgId = tx.reserve(1)?.start().expect("nonempty");
            tx.insert([&crate::schema::Org {
                id: crate::schema::OrgId(id.0),
                name: &format!("__touch_{}", id.0),
            }])
        })
        .map(|_| ())
        .map_err(|e| format!("cold touch: {e:?}"))
    }
}
