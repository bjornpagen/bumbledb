use super::*;

pub(super) fn lahman_dataset(
    dir: &Path,
    limit: Option<usize>,
) -> Result<Dataset, Box<dyn std::error::Error>> {
    let mut player_ids = BTreeMap::new();
    let mut team_ids = BTreeMap::new();
    let mut facts = Vec::new();

    read_csv(dir, "People.csv", limit, |headers, record| {
        let player_id = col(headers, record, &["playerID"]);
        let id = (player_ids.len() + 1) as u64;
        player_ids.insert(player_id.to_owned(), id);
        facts.push(Fact::new(
            "Player",
            [
                ("id", Value::Serial(id)),
                (
                    "first",
                    Value::String(col(headers, record, &["nameFirst"]).to_owned()),
                ),
                (
                    "last",
                    Value::String(col(headers, record, &["nameLast"]).to_owned()),
                ),
            ],
        ));
        Ok(())
    })?;

    read_csv(
        dir,
        "Teams.csv",
        scaled_limit(limit, 4),
        |headers, record| {
            let key = format!(
                "{}:{}",
                col(headers, record, &["yearID"]),
                col(headers, record, &["teamID"])
            );
            let id = (team_ids.len() + 1) as u64;
            team_ids.insert(key, id);
            facts.push(Fact::new(
                "Team",
                [
                    ("id", Value::Serial(id)),
                    (
                        "year",
                        Value::I64(parse_optional_i64(col(headers, record, &["yearID"]))),
                    ),
                    (
                        "league",
                        Value::String(col(headers, record, &["lgID"]).to_owned()),
                    ),
                    (
                        "name",
                        Value::String(col(headers, record, &["name"]).to_owned()),
                    ),
                ],
            ));
            Ok(())
        },
    )?;

    read_csv(
        dir,
        "Batting.csv",
        scaled_limit(limit, 10),
        |headers, record| {
            let player_key = col(headers, record, &["playerID"]);
            let team_key = format!(
                "{}:{}",
                col(headers, record, &["yearID"]),
                col(headers, record, &["teamID"])
            );
            let (Some(player), Some(team)) = (
                player_ids.get(player_key).copied(),
                team_ids.get(&team_key).copied(),
            ) else {
                return Ok(());
            };
            facts.push(Fact::new(
                "Batting",
                [
                    ("player", Value::Serial(player)),
                    ("team", Value::Serial(team)),
                    (
                        "year",
                        Value::I64(parse_optional_i64(col(headers, record, &["yearID"]))),
                    ),
                    (
                        "games",
                        Value::I64(parse_optional_i64(col(headers, record, &["G"]))),
                    ),
                    (
                        "hits",
                        Value::I64(parse_optional_i64(col(headers, record, &["H"]))),
                    ),
                ],
            ));
            Ok(())
        },
    )?;

    read_csv(
        dir,
        "Salaries.csv",
        scaled_limit(limit, 4),
        |headers, record| {
            let player_key = col(headers, record, &["playerID"]);
            let team_key = format!(
                "{}:{}",
                col(headers, record, &["yearID"]),
                col(headers, record, &["teamID"])
            );
            let (Some(player), Some(team)) = (
                player_ids.get(player_key).copied(),
                team_ids.get(&team_key).copied(),
            ) else {
                return Ok(());
            };
            facts.push(Fact::new(
                "Salary",
                [
                    ("player", Value::Serial(player)),
                    ("team", Value::Serial(team)),
                    (
                        "year",
                        Value::I64(parse_optional_i64(col(headers, record, &["yearID"]))),
                    ),
                    (
                        "salary",
                        Value::I64(parse_optional_i64(col(headers, record, &["salary"]))),
                    ),
                ],
            ));
            Ok(())
        },
    )?;

    Ok(lahman_from_facts(facts))
}
