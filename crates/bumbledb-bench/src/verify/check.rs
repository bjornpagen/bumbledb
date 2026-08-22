use super::{Case, MAX_BUNDLES, Run};

use bumbledb::Answers;
use bumbledb::schema::ValueType;

use crate::compare;
use crate::families::param_args;
use crate::naive::ParamValue;
use crate::translate::ParamSlot;

impl<S> Run<'_, S> {

    pub(super) fn check(
        &mut self,
        case: &Case<'_>,
        param_order: &[ParamSlot],
        params: &[ParamValue],
    ) -> bool {

        let mut rendered_query = None;
        let (ours, theirs): (
            Result<Vec<compare::Answer>, String>,
            Result<Vec<compare::Answer>, String>,
        ) = match self.db.prepare(case.query) {
            Err(e) => (
                Err(format!("{e}")),
                Err("not executed: no column types without a prepared query".to_owned()),
            ),
            Ok(mut prepared) => {
                rendered_query = Some(prepared.rendered_query().to_owned());
                let types: Vec<ValueType> = prepared
                    .signature()
                    .columns
                    .iter()
                    .map(|column| *column.ty())
                    .collect();
                let mut buffer = Answers::new();
                let args = param_args(params);
                let ours = self
                    .db
                    .read(|snap| snap.execute(&mut prepared, &args, &mut buffer))
                    .map(|()| compare::from_answers(&buffer, &types))
                    .map_err(|e| format!("{e}"));
                let theirs = self
                    .conn
                    .prepare_cached(case.sql)
                    .map_err(|e| e.to_string())
                    .and_then(|mut stmt| {
                        compare::from_sqlite(&mut stmt, param_order, params, &types)
                    });
                (ours, theirs)
            }
        };

        self.cases += 1;
        if self.cases.is_multiple_of(100) {
            eprintln!("verify: {}/{} cases", self.cases, self.total);
        }

        let verdict: Result<(), (String, String, String)> = match (ours, theirs) {
            (Ok(ours), Ok(theirs)) => {
                compare::multisets(ours.clone(), theirs.clone()).map_err(|m| {
                    (
                        m.to_string(),
                        render_answers(&ours),
                        render_answers(&theirs),
                    )
                })
            }
            (Err(engine), Ok(theirs)) => Err((
                "divergence by error: the engine errored where the oracle answered".to_owned(),
                format!("ERROR: {engine}"),
                render_answers(&theirs),
            )),
            (Ok(ours), Err(oracle)) => Err((
                "divergence by error: the oracle errored where the engine answered".to_owned(),
                render_answers(&ours),
                format!("ERROR: {oracle}"),
            )),
            (Err(engine), Err(oracle)) => Err((
                "both sides errored — a tool defect must not look like verification".to_owned(),
                format!("ERROR: {engine}"),
                format!("ERROR: {oracle}"),
            )),
        };

        if let Err((mismatch, ours_text, theirs_text)) = verdict {
            let bundle = self
                .out_dir
                .join(format!("mismatch-{}", self.bundles.len()));
            std::fs::create_dir_all(&bundle).expect("bundle dir");

            // raw IR after for arbitration by structure.
            std::fs::write(
                bundle.join("query.txt"),
                format!(
                    "{}\n{}\n\n{:#?}\n",
                    case.label,
                    rendered_query.as_deref().unwrap_or(""),
                    case.query
                ),
            )
            .expect("bundle");
            std::fs::write(bundle.join("query.sql"), case.sql).expect("bundle");
            std::fs::write(bundle.join("params.txt"), format!("{params:#?}\n")).expect("bundle");
            std::fs::write(bundle.join("mismatch.txt"), mismatch).expect("bundle");
            std::fs::write(bundle.join("ours.txt"), ours_text).expect("bundle");
            std::fs::write(bundle.join("theirs.txt"), theirs_text).expect("bundle");
            if let Some(golden) = case.golden_sql {
                std::fs::write(bundle.join("golden.sql"), golden).expect("bundle");
            }
            eprintln!("verify: MISMATCH {} -> {}", case.label, bundle.display());
            self.bundles.push(bundle);
        }
        self.bundles.len() < MAX_BUNDLES
    }
}

fn render_answers(answers: &[compare::Answer]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "{} answer(s)", answers.len());
    for answer in answers {
        let _ = writeln!(out, "{answer:?}");
    }
    out
}
