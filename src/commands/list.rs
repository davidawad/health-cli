//! `list ENTITY` and `stats`.

use serde_json::{json, Value};

use crate::cli::{Entity, ListArgs};
use crate::context::Ctx;
use crate::db::Db;
use crate::error::Result;
use crate::output::{Record, Report};
use crate::store::{self, Rec};

/// (report kind, rows, compact table columns) of an entity.
fn load(db: &Db, e: Entity) -> Result<(&'static str, Vec<Record>, &'static [&'static str])> {
    Ok(match e {
        Entity::Facts => ("facts", store::all(db, "profile_fact")?, &["key", "label", "value", "unit"]),
        Entity::Predispositions => {
            ("predispositions", store::all(db, "predisposition")?, &["kind", "name", "prevalence", "description"])
        }
        Entity::Supplements => {
            let mut rows = store::supplements(db)?;
            for r in &mut rows {
                let joined = r
                    .get("sources")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", "));
                r.insert("sources".into(), joined.map_or(Value::Null, Value::from));
            }
            (
                "supplements",
                rows,
                &["name", "taking", "frequency", "time_of_day", "kind", "amount", "unit", "price", "sources"],
            )
        }
        Entity::Doses => (
            "doses",
            store::all(db, "dose_schedule")?,
            &["supplement_id", "taking", "frequency", "time_of_day", "amount", "unit"],
        ),
        Entity::Sources => ("sources", store::all(db, "source")?, &["id", "name", "url"]),
        Entity::Evidence => {
            ("evidence", store::all(db, "evidence_ref")?, &["supplement_id", "stance", "url", "kd_work_id", "title"])
        }
        Entity::Products => (
            "products",
            store::all(db, "product")?,
            &["category", "name", "price", "in_use", "units_per_year", "notes"],
        ),
        Entity::Nutrition => (
            "nutrition",
            store::all(db, "nutrition_item")?,
            &["category", "name", "calories", "protein", "net_carbs", "fiber", "sugar", "sodium", "portions"],
        ),
        Entity::Targets => (
            "nutrition_targets",
            store::all(db, "nutrition_target")?,
            &["label", "calories", "protein", "net_carbs", "fiber"],
        ),
        Entity::Routine => (
            "routine",
            store::all(db, "routine_step")?,
            &["routine", "position", "name", "effect_time", "exec_seconds", "kind", "notes"],
        ),
        Entity::Workout => ("workout", store::all(db, "exercise")?, &["position", "name", "muscles"]),
        Entity::Vaccines => {
            ("vaccines", store::all(db, "vaccine")?, &["kind", "name", "disease_date", "disease_note", "notes"])
        }
        Entity::VaccineDoses => {
            ("vaccine_doses", store::all(db, "vaccine_dose")?, &["vaccine_id", "dose_no", "given_on", "raw"])
        }
        Entity::Physicals => ("physicals", store::all(db, "physical_check")?, &["section", "name", "detail", "done"]),
        Entity::Providers => ("providers", store::all(db, "provider")?, &["name", "note", "url"]),
        Entity::Sizes => ("sizes", store::all(db, "size")?, &["category", "name", "value", "unit", "note"]),
        Entity::Resources => ("resources", store::all(db, "resource_link")?, &["section", "label", "url"]),
        Entity::Intake => {
            ("intake", store::all(db, "intake_log")?, &["date", "net_kcal", "energy_out_kcal", "protein_g"])
        }
        Entity::Notes => ("notes", store::all(db, "note")?, &["scope", "position", "text"]),
    })
}

pub fn run(ctx: &Ctx, a: ListArgs) -> Result<()> {
    let db = ctx.db()?;
    let (kind, mut rows, cols) = load(&db, a.entity)?;
    if let Some(want) = &a.category {
        let want = want.to_lowercase();
        rows.retain(|r| {
            ["category", "section", "routine"].iter().any(|k| r.s(k).is_some_and(|v| v.to_lowercase() == want))
        });
    }
    ctx.emit(&Report::list(kind, rows).table_columns(cols))
}

pub fn stats(ctx: &Ctx) -> Result<()> {
    let db = ctx.db()?;
    let rows = store::counts(&db)?
        .into_iter()
        .map(|(t, n)| crate::output::to_record(&json!({"table": t, "rows": n})))
        .collect();
    ctx.emit(&Report::list("stats", rows))
}
