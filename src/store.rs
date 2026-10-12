//! Typed-enough read access to the health tables. Rows come back as JSON
//! records (`Rec`) keyed by column name, which is what the CLI renders and
//! the workbook generator consumes; writes live next to their callers
//! (importer, edit commands).

use serde_json::{json, Map, Value};

use crate::db::{Db, Value as Sql};
use crate::error::Result;
use crate::output::Record;

/// Every user-data table, in dependency order (parents first).
pub const TABLES: &[&str] = &[
    "profile_fact",
    "predisposition",
    "source",
    "supplement",
    "dose_schedule",
    "supplement_source",
    "evidence_ref",
    "product",
    "nutrition_target",
    "nutrition_item",
    "routine_step",
    "exercise",
    "exercise_alt",
    "vaccine",
    "vaccine_dose",
    "physical_check",
    "provider",
    "size",
    "resource_link",
    "note",
];

/// Column names of each table (kept in step with `migrations::SCHEMA_V1`).
pub fn columns(table: &str) -> &'static [&'static str] {
    match table {
        "profile_fact" => &["key", "label", "value", "kind", "unit", "position"],
        "predisposition" => &["id", "position", "kind", "name", "description", "prevalence"],
        "source" => &["id", "name", "url", "notes"],
        "supplement" => &[
            "id",
            "position",
            "name",
            "kind",
            "notes",
            "experience",
            "units_per_year",
            "price",
            "price_currency",
            "price_date",
            "price_note",
            "buy_url",
        ],
        "dose_schedule" => {
            &["id", "supplement_id", "taking", "frequency", "time_of_day", "amount", "amount_text", "unit"]
        }
        "supplement_source" => &["supplement_id", "source_id"],
        "evidence_ref" => &["id", "supplement_id", "url", "kd_work_id", "stance", "title", "note", "added_at"],
        "product" => &[
            "id",
            "position",
            "category",
            "name",
            "price",
            "price_currency",
            "price_date",
            "dosage",
            "in_use",
            "units_per_year",
            "notes",
            "url",
            "alt_url",
        ],
        "nutrition_target" => &[
            "id",
            "position",
            "label",
            "calories",
            "protein",
            "net_carbs",
            "fiber",
            "sugar",
            "sodium",
            "cholesterol",
            "total_fat",
            "unsat_fat",
            "trans_fat",
            "sat_fat",
            "yuka_score",
            "extra",
        ],
        "nutrition_item" => &[
            "id",
            "position",
            "category",
            "name",
            "calories",
            "protein",
            "net_carbs",
            "fiber",
            "sugar",
            "sodium",
            "cholesterol",
            "total_fat",
            "unsat_fat",
            "trans_fat",
            "sat_fat",
            "yuka_score",
            "ingredients_ok",
            "no_prep",
            "vegan",
            "qualifying",
            "portions",
            "notes",
            "price",
            "price_currency",
            "price_date",
            "url",
            "alt_url",
        ],
        "routine_step" => &["id", "routine", "position", "name", "effect_time", "exec_seconds", "kind", "notes"],
        "exercise" => &["id", "position", "name", "muscles"],
        "exercise_alt" => &["id", "exercise_id", "position", "text"],
        "vaccine" => &["id", "position", "kind", "name", "disease_date", "disease_note", "notes"],
        "vaccine_dose" => &["id", "vaccine_id", "dose_no", "given_on", "raw"],
        "physical_check" => &["id", "position", "section", "name", "detail", "done"],
        "provider" => &["id", "position", "name", "note", "url"],
        "size" => &["id", "position", "category", "name", "value", "unit", "note"],
        "resource_link" => &["id", "position", "section", "label", "url"],
        "note" => &["id", "scope", "position", "text"],
        _ => &[],
    }
}

pub fn sql_to_json(v: &Sql) -> Value {
    match v {
        Sql::Null => Value::Null,
        Sql::Integer(i) => json!(i),
        Sql::Float(f) => json!(f),
        Sql::Text(s) => json!(s.to_string()),
        other => json!(other.to_text()),
    }
}

/// `SELECT <all columns> FROM table [WHERE ..] ORDER BY ..` as records.
pub fn select(db: &Db, table: &str, order: &str) -> Result<Vec<Record>> {
    let cols = columns(table);
    let rows = db.query(&format!("SELECT {} FROM {table} ORDER BY {order}", cols.join(", ")), &[])?;
    Ok(rows.iter().map(|r| cols.iter().zip(r).map(|(c, v)| ((*c).to_string(), sql_to_json(v))).collect()).collect())
}

/// Row ordering of a table in listings and the generated workbook.
pub fn default_order(table: &str) -> &'static str {
    match table {
        "routine_step" => "routine, position",
        "profile_fact" | "supplement" | "product" | "nutrition_item" | "nutrition_target" | "exercise" | "vaccine"
        | "physical_check" | "provider" | "size" | "resource_link" | "predisposition" => "position, rowid",
        "note" => "scope, position, id",
        _ => "rowid",
    }
}

pub fn all(db: &Db, table: &str) -> Result<Vec<Record>> {
    select(db, table, default_order(table))
}

pub fn count(db: &Db, table: &str) -> Result<i64> {
    db.query_scalar_i64(&format!("SELECT count(*) FROM {table}"), &[])
}

pub fn counts(db: &Db) -> Result<Vec<(&'static str, i64)>> {
    TABLES.iter().map(|t| count(db, t).map(|n| (*t, n))).collect()
}

/// Child rows whose parent is missing (foreign keys are on, so normally 0).
pub fn orphan_count(db: &Db) -> Result<i64> {
    [
        "SELECT count(*) FROM dose_schedule d LEFT JOIN supplement s ON s.id = d.supplement_id WHERE s.id IS NULL",
        "SELECT count(*) FROM supplement_source x LEFT JOIN supplement s ON s.id = x.supplement_id LEFT JOIN source o ON o.id = x.source_id WHERE s.id IS NULL OR o.id IS NULL",
        "SELECT count(*) FROM evidence_ref e LEFT JOIN supplement s ON s.id = e.supplement_id WHERE e.supplement_id IS NOT NULL AND s.id IS NULL",
        "SELECT count(*) FROM exercise_alt a LEFT JOIN exercise e ON e.id = a.exercise_id WHERE e.id IS NULL",
        "SELECT count(*) FROM vaccine_dose d LEFT JOIN vaccine v ON v.id = d.vaccine_id WHERE v.id IS NULL",
    ]
    .iter()
    .try_fold(0, |acc, q| db.query_scalar_i64(q, &[]).map(|n| acc + n))
}

/// Accessors on a JSON record.
pub trait Rec {
    fn s(&self, k: &str) -> Option<String>;
    fn f(&self, k: &str) -> Option<f64>;
    fn i(&self, k: &str) -> Option<i64>;
    fn b(&self, k: &str) -> Option<bool> {
        self.i(k).map(|v| v != 0)
    }
}

impl Rec for Map<String, Value> {
    fn s(&self, k: &str) -> Option<String> {
        match self.get(k)? {
            Value::Null => None,
            Value::String(s) => Some(s.clone()),
            other => Some(other.to_string()),
        }
    }
    fn f(&self, k: &str) -> Option<f64> {
        self.get(k).and_then(Value::as_f64)
    }
    fn i(&self, k: &str) -> Option<i64> {
        self.get(k).and_then(Value::as_i64)
    }
}

/// Supplements with their (first) dose schedule, comma-joined sources and the
/// number of evidence references: the shape of one row of the Supplements sheet.
pub fn supplements(db: &Db) -> Result<Vec<Record>> {
    let mut sups = all(db, "supplement")?;
    let doses = all(db, "dose_schedule")?;
    let links = all(db, "supplement_source")?;
    let sources = all(db, "source")?;
    let evidence = all(db, "evidence_ref")?;
    for s in &mut sups {
        let id = s.i("id");
        if let Some(d) = doses.iter().find(|d| d.i("supplement_id") == id) {
            for k in ["taking", "frequency", "time_of_day", "amount", "amount_text", "unit"] {
                s.insert(k.to_string(), d.get(k).cloned().unwrap_or(Value::Null));
            }
        }
        let names: Vec<String> = links
            .iter()
            .filter(|l| l.i("supplement_id") == id)
            .filter_map(|l| sources.iter().find(|o| o.i("id") == l.i("source_id")).and_then(|o| o.s("name")))
            .collect();
        s.insert("sources".into(), json!(names));
        let ev: Vec<&Record> = evidence.iter().filter(|e| e.i("supplement_id") == id).collect();
        s.insert("evidence".into(), json!(ev.len()));
        s.insert("research_url".into(), ev.iter().find_map(|e| e.s("url")).map_or(Value::Null, Value::from));
    }
    Ok(sups)
}
