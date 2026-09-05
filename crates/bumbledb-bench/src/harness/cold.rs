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

/// The invalidation touch: one committed Org row per round. Application-owned
/// ids (E-NO-RESERVE): the cursor seeds from `MAX(id) + 1` over the live Org
/// rows at closure creation (Org corpora are tiny — the probe is setup, never
/// timed) and advances locally per touch.
pub fn org_touch(
    db: &bumbledb::Db<crate::schema::Ledger>,
) -> impl FnMut() -> Result<(), String> + '_ {
    let mut next: Option<u64> = None;
    move || {
        let id = if let Some(id) = next {
            id
        } else {
            let probed = db
                .read(|snap| {
                    let mut max: Option<u64> = None;
                    for fact in snap.scan(crate::schema::ids::ORG)? {
                        let row = fact?;
                        if let Some(bumbledb::Value::U64(id)) = row.first() {
                            max = Some(max.map_or(*id, |seen| seen.max(*id)));
                        }
                    }
                    Ok(max.map_or(0, |seen| seen + 1))
                })
                .map_err(|e| format!("cold touch probe: {e:?}"))?;
            next = Some(probed);
            probed
        };
        db.write(|tx| {
            tx.insert([&crate::schema::Org {
                id: crate::schema::OrgId(id),
                name: &format!("__touch_{id}"),
            }])
        })
        .map(|_| ())
        .map_err(|e| format!("cold touch: {e:?}"))?;
        next = Some(id + 1);
        Ok(())
    }
}
