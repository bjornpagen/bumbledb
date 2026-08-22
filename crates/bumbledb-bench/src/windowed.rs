use std::path::Path;

use bumbledb::{Db, RelationId, Value};

use crate::corpus_gen::{GenConfig, Rng};
use crate::harness::{self, Measurement, Protocol};
use crate::writebench::write_protocol;

#[cfg(test)]
mod tests;

pub mod world {
    bumbledb::schema! {
        pub WindowedWorld;

        relation WParent {
            id: u64 as WParentId, fresh,
            kind: u64,
        }
        relation WChild {
            id: u64 as WChildId, fresh,
            parent: u64 as WParentId,
            flag: u64,
        }

        WChild(parent) <= WParent(id);
        WParent(id) <={0..64} WChild(parent);
        WParent(id | kind == 1) <={0} WChild(parent | flag == 1);
    }
}

pub mod baseline {
    bumbledb::schema! {
        pub UnwindowedWorld;

        relation WParent {
            id: u64 as WParentId, fresh,
            kind: u64,
        }
        relation WChild {
            id: u64 as WChildId, fresh,
            parent: u64 as WParentId,
            flag: u64,
        }

        WChild(parent) <= WParent(id);
    }
}

pub mod ids {
    use bumbledb::RelationId;

    pub const PARENT: RelationId = RelationId(0);
    pub const CHILD: RelationId = RelationId(1);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mass {
    pub parents: u64,

    pub children_per_parent: u64,
}

impl Mass {

    pub const BENCH: Self = Self {
        parents: 4_096,
        children_per_parent: 8,
    };

    #[must_use]
    pub const fn unit() -> Self {
        Self {
            parents: 16,
            children_per_parent: 4,
        }
    }
}

pub const PARENTS: u64 = Mass::BENCH.parents;

#[must_use]
pub fn parent_kind(i: u64) -> u64 {
    u64::from(i.is_multiple_of(8))
}

pub fn relation_rows(mass: Mass, rel: RelationId) -> Box<dyn Iterator<Item = Vec<Value>>> {
    match rel {
        ids::PARENT => {
            Box::new((0..mass.parents).map(|i| vec![Value::U64(i), Value::U64(parent_kind(i))]))
        }
        ids::CHILD => Box::new((0..mass.parents * mass.children_per_parent).map(move |i| {

            vec![
                Value::U64(i),
                Value::U64(i / mass.children_per_parent),
                Value::U64(0),
            ]
        })),
        _ => unreachable!("two windowed relations"),
    }
}

/// # Errors
pub fn load<S>(db: &Db<S>, mass: Mass) -> Result<(), String> {
    for rel in [ids::PARENT, ids::CHILD] {
        db.write(|tx| {
            tx.insert_dyn(rel, relation_rows(mass, rel))
                .map(bumbledb::MutationReport::changed)
        })
        .map_err(|e| format!("windowed load: {e:?}"))?
        .unwrap();
    }
    Ok(())
}

fn unselected_parent(rng: &mut Rng) -> u64 {
    loop {
        let p = rng.range(PARENTS);
        if parent_kind(p) == 0 {
            return p;
        }
    }
}

/// # Errors
/// # Panics
pub fn commit_window_admission(
    db: &Db<world::WindowedWorld>,
    proto: Protocol,
) -> Result<Measurement, String> {
    let mut rng = Rng::new(0x0117_0001);
    harness::measure(proto, || {
        let parent = world::WParentId(rng.range(PARENTS));
        db.write(|tx| {
            let id: world::WChildId = tx.reserve(1)?.start().expect("nonempty");
            tx.insert([&world::WChild {
                id,
                parent,
                flag: 0,
            }])
        })
        .map(|_| 1)
        .map_err(|e| format!("commit_window_admission: {e:?}"))
    })
}

/// # Errors
/// # Panics
pub fn commit_window_baseline(
    db: &Db<baseline::UnwindowedWorld>,
    proto: Protocol,
) -> Result<Measurement, String> {
    let mut rng = Rng::new(0x0117_0001);
    harness::measure(proto, || {
        let parent = baseline::WParentId(rng.range(PARENTS));
        db.write(|tx| {
            let id: baseline::WChildId = tx.reserve(1)?.start().expect("nonempty");
            tx.insert([&baseline::WChild {
                id,
                parent,
                flag: 0,
            }])
        })
        .map(|_| 1)
        .map_err(|e| format!("commit_window_baseline: {e:?}"))
    })
}

/// # Errors
/// # Panics
pub fn commit_window_exclusion(
    db: &Db<world::WindowedWorld>,
    proto: Protocol,
) -> Result<Measurement, String> {
    let mut rng = Rng::new(0x0117_0002);
    harness::measure(proto, || {
        let parent = world::WParentId(unselected_parent(&mut rng));
        db.write(|tx| {
            let id: world::WChildId = tx.reserve(1)?.start().expect("nonempty");
            tx.insert([&world::WChild {
                id,
                parent,
                flag: 1,
            }])
        })
        .map(|_| 1)
        .map_err(|e| format!("commit_window_exclusion: {e:?}"))
    })
}

/// # Errors
pub fn write_families(
    _cfg: GenConfig,
    scratch: &Path,
    selected: &dyn Fn(&str) -> bool,
    mode: crate::storemode::StoreMode,
    trace_dir: Option<&Path>,
    flames: &mut Vec<crate::report::FlameEmbed>,
) -> Result<Vec<crate::report::WriteFamilyReport>, String> {
    let names = [
        "commit_window_admission",
        "commit_window_baseline",
        "commit_window_exclusion",
    ];
    if !names.iter().any(|name| selected(name)) {
        return Ok(Vec::new());
    }

    std::fs::create_dir_all(scratch).map_err(|e| format!("windowed scratch: {e}"))?;
    eprintln!("bench: loading the windowed twin worlds");
    let windowed = mode.create(&scratch.join("windowed"), world::WindowedWorld)?;
    load(&windowed, Mass::BENCH)?;
    let unwindowed = mode.create(&scratch.join("baseline"), baseline::UnwindowedWorld)?;
    load(&unwindowed, Mass::BENCH)?;

    let mut out = Vec::new();
    let mut push = |name: &str,
                    run: &mut dyn FnMut(Protocol) -> Result<Measurement, String>|
     -> Result<(), String> {
        if !selected(name) {
            return Ok(());
        }
        eprintln!("bench: {name}");
        let (ours, ghz) = crate::clockproxy::stamped(|| run(write_protocol(name)))?;

        if let Some(table) = crate::trace_out::traced_solo(trace_dir, name, run)? {
            flames.push(crate::report::FlameEmbed {
                name: name.to_owned(),
                table,
            });
        }
        out.push(crate::report::WriteFamilyReport {
            name: name.to_owned(),
            ours: ours.stats,
            theirs: None,
            facts_per_sec: None,
            ghz: Some(ghz.into()),
        });
        Ok(())
    };
    // Baseline first: the control's clock shadow must not carry the

    push("commit_window_baseline", &mut |proto| {
        commit_window_baseline(&unwindowed, proto)
    })?;
    push("commit_window_admission", &mut |proto| {
        commit_window_admission(&windowed, proto)
    })?;
    push("commit_window_exclusion", &mut |proto| {
        commit_window_exclusion(&windowed, proto)
    })?;
    Ok(out)
}
