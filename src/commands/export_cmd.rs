//! `export regimens`: the store's supplements as `biomarker regimen add` commands.
//! A dry run (the default) prints the plan; `--apply` runs it with `biomarker_cmd`,
//! skipping regimens biomarker already holds (same intervention and start day), so
//! re-running is safe. Nothing is written to files on either side: the plan goes to
//! stdout and biomarker is never given `-o`.

use std::collections::BTreeSet;

use chrono::NaiveDate;
use serde_json::{json, Value};

use crate::cli::{ExportCmd, ExportRegimensArgs};
use crate::context::Ctx;
use crate::error::{AppError, Result};
use crate::output::{to_record, Report};
use crate::regimens::{plan, Action, Catalog, Entry, PlanOpts};
use crate::store;
use crate::workbook::labs;

pub fn run(ctx: &Ctx, c: ExportCmd) -> Result<()> {
    match c {
        ExportCmd::Regimens(a) => regimens(ctx, a),
    }
}

const COLUMNS: &[&str] = &["position", "supplement", "slug", "taking", "started_on", "action", "reason", "result"];

fn regimens(ctx: &Ctx, a: ExportRegimensArgs) -> Result<()> {
    if ctx.out.output.as_ref().is_some_and(|p| p.as_os_str() != "-") {
        return Err(AppError::invalid("export regimens prints its plan to stdout only; drop -o/--output"));
    }
    if let Some(s) = &a.start {
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|_| AppError::invalid(format!("--start {s}: expected YYYY-MM-DD")))?;
    }
    let cat = match &a.research {
        Some(dir) => Catalog::from_dir(dir)?,
        None => Catalog::embedded(),
    };
    let person = a.person.clone().unwrap_or_else(|| ctx.resolved.get("biomarker_person").to_string());
    let sups = store::supplements(&ctx.db()?)?;
    let entries = plan(&sups, &cat, &PlanOpts { person: person.clone(), default_start: a.start.clone() });
    let count = |act: Action| entries.iter().filter(|e| e.action == act).count();
    let (adds, needs) = (count(Action::Add), count(Action::NeedsStart));
    let mut rows: Vec<_> = entries.iter().map(to_record).collect();
    let mut applied = 0;
    if a.apply {
        if needs > 0 {
            let names: Vec<&str> =
                entries.iter().filter(|e| e.action == Action::NeedsStart).map(|e| e.supplement.as_str()).collect();
            return Err(AppError::invalid(format!(
                "{needs} taken supplement(s) have no start date ({}); pass --start YYYY-MM-DD. Nothing was applied",
                names.join(", ")
            )));
        }
        applied = apply(ctx, &person, &entries, &mut rows)?;
        ctx.info(&format!("applied {applied} regimen(s) for {person} ({} already present)", adds - applied));
    } else {
        ctx.info(&format!(
            "dry run: {adds} regimen(s) to add for {person}, {needs} need a start date (--start); --apply runs them"
        ));
    }
    let report = Report::list("regimen_plan", rows)
        .table_columns(COLUMNS)
        .meta("person", json!(person))
        .meta("dry_run", json!(!a.apply))
        .meta("research_slugs", json!(cat.len()))
        .meta("add", json!(adds))
        .meta("needs_start", json!(needs))
        .meta("skip", json!(count(Action::Skip)));
    ctx.emit(&if a.apply { report.meta("applied", json!(applied)) } else { report })
}

/// Run every `add` entry not already in `biomarker regimen ls`; record a `result`
/// (and the new `regimen_id`) on each row. Returns the number added.
fn apply(ctx: &Ctx, person: &str, entries: &[Entry], rows: &mut [crate::output::Record]) -> Result<usize> {
    let cmd = ctx.resolved.get("biomarker_cmd");
    let existing: BTreeSet<(String, String)> = labs::data_of(
        &labs::run(cmd, &["regimen", "ls", "-p", person, "--status", "all", "--format", "json"])?,
        "biomarker/",
    )?
    .iter()
    .filter_map(|r| Some((labs::text(r, "intervention")?, labs::text(r, "started_on")?)))
    .collect();
    let mut added = 0;
    for (e, row) in entries.iter().zip(rows.iter_mut()) {
        if e.action != Action::Add {
            continue;
        }
        let key = (e.slug.clone().unwrap_or_default(), e.started_on.clone().unwrap_or_default());
        if existing.contains(&key) {
            row.insert("result".into(), json!("exists"));
            continue;
        }
        let mut args: Vec<&str> = e.argv.iter().map(String::as_str).collect();
        args.extend(["--format", "json"]);
        let out = labs::run(cmd, &args).map_err(|err| {
            AppError::general(format!(
                "{} ({}): {}; {added} regimen(s) were added before it, re-run to continue",
                e.supplement, key.0, err.message
            ))
        })?;
        let id = serde_json::from_str::<Value>(&out).ok().and_then(|v| v["data"]["id"].as_i64());
        row.insert("result".into(), json!("added"));
        row.insert("regimen_id".into(), json!(id));
        added += 1;
    }
    Ok(added)
}
