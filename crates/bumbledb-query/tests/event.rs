use bumbledb::{
    AnswerValue, Db,
    event::{Space, SpaceId},
};
use bumbledb_query::{params, query};

mod common;

bumbledb::schema! {
    pub Events;
    relation Region { id: u64, condition: event }
    Region(id) -> Region;
}

#[test]
fn named_event_arguments_retain_joint_world_identity() {
    let dir = common::TempDir::new("typed-event-argument");
    let db = Db::create(dir.path(), Events, common::work())
        .unwrap()
        .unwrap();
    let source = Space::new(SpaceId([9; 32]), 3, &()).unwrap();
    let x = source.coordinate(0, &()).unwrap();
    db.write(common::work(), |tx| {
        tx.insert([&Region {
            id: 1,
            condition: x.clone(),
        }])
    })
    .unwrap()
    .unwrap();
    let template = query!(Events {
        (id) | Region(id, condition == ?condition);
    });
    let mut prepared = db.prepare(&template, common::work()).unwrap();
    // A fresh decode has a different runtime handle but the same meaning.
    let decoded = bumbledb::Event::from_bytes(&x.to_bytes(&()).unwrap(), &()).unwrap();
    db.read(common::work(), |snapshot| {
        let args = template.bind(params! { condition: &decoded });
        let answers = snapshot.execute_collect(&mut prepared, &args)?;
        assert_eq!(answers.len(), 1);
        assert_eq!(answers.get(0, 0), AnswerValue::U64(1));
        Ok(())
    })
    .unwrap();
}
