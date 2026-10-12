//! Import of the legacy `Health Dashboard.xlsx` (read-only): every sheet that
//! is neither labs (biomarker-cli owns those) nor genetics (genome-cli).
//!
//! Rules: section-heading rows become a `category` column, never rows; only
//! inputs are stored (formula-derived cells such as $/yr, weekly dose, totals
//! and ages are recomputed by the workbook generator); a formula cell that
//! *is* an input (`=365 * 0.5 / 4`) is stored as its cached value, which for
//! Google Sheets exports is also what a `__xludf.DUMMYFUNCTION` evaluated to.

use std::path::Path;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::db::{int, opt_real, opt_text, text, Db, Value};
use crate::error::{AppError, Result};
use crate::sheet::{self, classify, Cell, Grid, RowKind};
use crate::store;
use crate::util::slugify;

/// What one worksheet contributed.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SheetReport {
    pub sheet: String,
    /// Non-blank rows below the header row (column A filled), as counted in the workbook.
    pub rows_seen: usize,
    /// Rows stored as data (supplements, products, steps, ...).
    pub items: usize,
    /// Section-heading rows folded into a category / section column.
    pub headers: usize,
    /// Derived rows (totals, sums) that are recomputed, never stored.
    pub derived: usize,
    /// Other stored rows (targets, notes, links) that are not items.
    pub other: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportReport {
    pub source: String,
    pub sha256: String,
    pub sheets: Vec<SheetReport>,
    pub skipped: Vec<String>,
}

/// Sheets owned by sibling tools (or empty / derived): never imported here.
pub const SKIPPED_SHEETS: &[(&str, &str)] = &[
    ("Biomarkers", "labs: biomarker-cli"),
    ("Bryans Biomarkers", "labs: biomarker-cli"),
    ("Genetics", "genome-cli"),
    ("Vitals (Whoop)", "empty"),
];

struct Cx<'a> {
    db: &'a Db,
    price_date: String,
    now: String,
}

fn ins(db: &Db, sql: &str, params: &[Value]) -> Result<i64> {
    db.execute(sql, params)?;
    Ok(db.last_insert_rowid())
}

fn yes_no(c: &Cell) -> Option<i64> {
    match c.text().to_ascii_uppercase().as_str() {
        "YES" | "TRUE" | "Y" => Some(1),
        "NO" | "FALSE" | "N" => Some(0),
        _ => None,
    }
}

fn opt_int(i: Option<i64>) -> Value {
    i.map_or(Value::Null, Value::Integer)
}

fn iso(c: &Cell) -> Option<String> {
    match c {
        Cell::Date(d) => Some(d.format("%Y-%m-%d").to_string()),
        _ => None,
    }
}

fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// Rows below the header whose column A is filled.
fn rows_seen(g: &Grid) -> usize {
    (1..g.height()).filter(|r| g.text(*r, 0).is_some()).count()
}

fn read_sheet(path: &Path, names: &[&str]) -> Result<Grid> {
    for n in names {
        if let Some(g) = sheet::read(path, n)? {
            return Ok(g);
        }
    }
    Err(AppError::not_found(format!("workbook has no sheet '{}'", names[0])))
}

fn sha256_hex(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).map_err(|e| AppError::io(format!("reading {}: {e}", path.display())))?;
    Ok(Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect())
}

/// Whether any imported data is already present.
pub fn has_data(db: &Db) -> Result<bool> {
    Ok(store::counts(db)?.iter().any(|(_, n)| *n > 0))
}

/// Remove every row of every user-data table (children first).
pub fn wipe(db: &Db) -> Result<()> {
    store::TABLES.iter().rev().try_for_each(|t| db.execute(&format!("DELETE FROM {t}"), &[]).map(|_| ()))
}

/// Import `path` into `db` (one transaction). Fails when data already exists
/// unless `replace` is set. `price_date` stamps every imported price.
pub fn import(db: &Db, path: &Path, price_date: &str, replace: bool) -> Result<ImportReport> {
    if !path.is_file() {
        return Err(AppError::not_found(format!("{} does not exist", path.display())));
    }
    let names = sheet::sheet_names(path)?;
    if !names.iter().any(|n| n.eq_ignore_ascii_case("Supplements")) {
        return Err(AppError::invalid(format!(
            "{} does not look like a Health Dashboard workbook (no Supplements sheet)",
            path.display()
        )));
    }
    if has_data(db)? && !replace {
        return Err(AppError::invalid("the database already holds health data; pass --replace to wipe and re-import"));
    }
    let sha = sha256_hex(path)?;
    let cx = Cx { db, price_date: price_date.to_string(), now: crate::util::now_iso() };
    let sheets = db.transaction(|db| {
        wipe(db)?;
        let sheets = vec![
            dashboard(&cx, &read_sheet(path, &["Dashboard"])?)?,
            supplements(&cx, &read_sheet(path, &["Supplements"])?)?,
            products(&cx, &read_sheet(path, &["Products"])?)?,
            nutrition(&cx, &read_sheet(path, &["Nutrition"])?)?,
            routine(&cx, &read_sheet(path, &["Routine"])?)?,
            workout(&cx, &read_sheet(path, &["Workout"])?)?,
            vaccines(
                &cx,
                &read_sheet(path, &["Vaccines  Conditions", "Vaccines Conditions", "Vaccines & Conditions"])?,
            )?,
            physicals(&cx, &read_sheet(path, &["Physicals"])?)?,
            sizes(&cx, &read_sheet(path, &["Sizes"])?)?,
            resources(&cx, &read_sheet(path, &["Resources"])?)?,
            intake(&cx, &read_sheet(path, &["Weight & Age"])?)?,
        ];
        let counts = serde_json::json!(store::counts(db)?.into_iter().collect::<std::collections::BTreeMap<_, _>>());
        let name = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        ins(
            db,
            "INSERT INTO import_batch (source_name, sha256, imported_at, counts) VALUES (?1, ?2, ?3, ?4)",
            &[text(name), text(&sha), text(&cx.now), text(counts.to_string())],
        )?;
        Ok(sheets)
    })?;
    let skipped = SKIPPED_SHEETS
        .iter()
        .filter(|(n, _)| names.iter().any(|x| x.eq_ignore_ascii_case(n)))
        .map(|(n, why)| format!("{n} ({why})"))
        .collect();
    Ok(ImportReport { source: path.display().to_string(), sha256: sha, sheets, skipped })
}

// ---------------------------------------------------------------- Dashboard

/// `Height (in.)` -> (`Height`, Some(`in`)).
fn split_unit(label: &str) -> (String, Option<String>) {
    match (label.rfind('('), label.ends_with(')')) {
        (Some(i), true) => {
            (label[..i].trim().to_string(), Some(label[i + 1..label.len() - 1].trim_end_matches('.').to_string()))
        }
        _ => (label.trim().to_string(), None),
    }
}

fn dashboard(cx: &Cx, g: &Grid) -> Result<SheetReport> {
    let mut rep = SheetReport { sheet: "Dashboard".into(), ..Default::default() };
    let mut pos = 0;
    // Label/value column pairs of the "Base Stats" block (rows 1-11).
    for r in 0..11 {
        for (lc, vc) in [(1, 2), (4, 5), (7, 8)] {
            let (Some(label), Some(_)) = (g.text(r, lc), g.text(r, vc)) else { continue };
            if g.has_formula(r, vc) {
                continue; // derived: weight, ideal weight, water, protein, age, genetics counts
            }
            let (label, unit) = split_unit(&label);
            let cell = g.cell(r, vc);
            let kind = match cell {
                Cell::Date(_) => "date",
                Cell::Number(_) => "number",
                _ => "text",
            };
            pos += 1;
            ins(
                cx.db,
                "INSERT INTO profile_fact (key, label, value, kind, unit, position) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                &[text(slugify(&label)), text(&label), text(cell.text()), text(kind), opt_text(unit), int(pos)],
            )?;
            rep.items += 1;
        }
    }
    // Predisposition table: kind in A (carried down), name B, description C, prevalence D.
    let mut kind = String::new();
    let mut n = 0;
    for r in 11..60 {
        if let Some(k) = g.text(r, 0) {
            kind = k;
        }
        let Some(name) = g.text(r, 1) else { continue };
        n += 1;
        ins(
            cx.db,
            "INSERT INTO predisposition (position, kind, name, description, prevalence) VALUES (?1, ?2, ?3, ?4, ?5)",
            &[int(n), text(&kind), text(name), opt_text(g.text(r, 2)), opt_text(g.text(r, 3))],
        )?;
        rep.other += 1;
    }
    // Free-text notes in column F below the charts.
    for r in 20..g.height() {
        if let Some(t) = g.text(r, 5).filter(|_| !g.has_formula(r, 5)) {
            ins(
                cx.db,
                "INSERT INTO note (scope, position, text) VALUES ('dashboard', ?1, ?2)",
                &[int(r as i64 + 1), text(t)],
            )?;
            rep.other += 1;
        }
    }
    rep.rows_seen = rep.items + rep.other;
    Ok(rep)
}

// -------------------------------------------------------------- Supplements

fn get_or_create_source(db: &Db, name: &str) -> Result<i64> {
    let existing = db.query_opt("SELECT id FROM source WHERE lower(name) = lower(?1)", &[text(name)])?;
    match existing {
        Some(r) => Ok(crate::db::RowExt::i(&r[..], 0).unwrap_or_default()),
        None => ins(db, "INSERT INTO source (name) VALUES (?1)", &[text(name)]),
    }
}

fn supplements(cx: &Cx, g: &Grid) -> Result<SheetReport> {
    let mut rep = SheetReport { sheet: "Supplements".into(), rows_seen: rows_seen(g), ..Default::default() };
    let mut pos = 0;
    for r in 1..g.height() {
        let Some(name) = g.text(r, 0) else { continue };
        pos += 1;
        let kind = g.text(r, 4).filter(|k| matches!(k.as_str(), "Rx" | "Nt" | "Sp"));
        let (price, price_note) = match g.cell(r, 11) {
            Cell::Number(n) => (Some(*n), None),
            other => (None, g.text(r, 11).map(|_| other.text())),
        };
        let sid = ins(
            cx.db,
            "INSERT INTO supplement (position, name, kind, notes, experience, units_per_year, price, price_currency, price_date, price_note, buy_url)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'USD', ?8, ?9, ?10)",
            &[
                int(pos),
                text(&name),
                opt_text(kind),
                opt_text(g.text(r, 8)),
                opt_text(g.text(r, 9)),
                opt_real(g.number(r, 10)),
                opt_real(price),
                if price.is_some() { text(&cx.price_date) } else { Value::Null },
                opt_text(price_note),
                opt_text(g.text(r, 13)),
            ],
        )?;
        let (amount, amount_text) = match g.cell(r, 5) {
            Cell::Number(n) => (Some(*n), None),
            other => (None, g.text(r, 5).map(|_| other.text())),
        };
        ins(
            cx.db,
            "INSERT INTO dose_schedule (supplement_id, taking, frequency, time_of_day, amount, amount_text, unit)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            &[
                int(sid),
                opt_int(yes_no(g.cell(r, 1))),
                opt_text(g.text(r, 2)),
                opt_text(g.text(r, 3)),
                opt_real(amount),
                opt_text(amount_text),
                opt_text(g.text(r, 6)),
            ],
        )?;
        if let Some(url) = g.text(r, 14) {
            ins(
                cx.db,
                "INSERT INTO evidence_ref (supplement_id, url, stance, added_at) VALUES (?1, ?2, 'neutral', ?3)",
                &[int(sid), text(url), text(&cx.now)],
            )?;
        }
        let mut seen: Vec<String> = Vec::new();
        for src in g.text(r, 15).unwrap_or_default().split(',').map(str::trim).filter(|s| !s.is_empty()) {
            if seen.contains(&src.to_lowercase()) {
                continue;
            }
            seen.push(src.to_lowercase());
            let src_id = get_or_create_source(cx.db, src)?;
            ins(
                cx.db,
                "INSERT INTO supplement_source (supplement_id, source_id) VALUES (?1, ?2)",
                &[int(sid), int(src_id)],
            )?;
        }
        rep.items += 1;
    }
    Ok(rep)
}

// ----------------------------------------------------------------- Products

fn products(cx: &Cx, g: &Grid) -> Result<SheetReport> {
    let mut rep = SheetReport { sheet: "Products".into(), rows_seen: rows_seen(g), ..Default::default() };
    let (mut category, mut pos) = (None::<String>, 0);
    for r in 1..g.height() {
        let Some(name) = g.text(r, 0) else { continue };
        // Input columns: $/unit, dosage, using, #units/yr, notes, link, 2nd link.
        let inputs = [1, 2, 3, 5, 8, 9, 10].iter().any(|c| g.text(r, *c).is_some() && !g.has_formula(r, *c));
        match classify(g, r, &[]) {
            RowKind::Header => {
                category = Some(name);
                rep.headers += 1;
            }
            _ if !inputs => {
                rep.derived += 1;
                if g.text(r, 7).is_some_and(|t| !t.starts_with(|c: char| c.is_ascii_digit())) {
                    // "Alternative" row: the comparison text is an input worth keeping.
                    let t = format!("{name}: {}", g.text(r, 7).unwrap_or_default());
                    ins(
                        cx.db,
                        "INSERT INTO note (scope, position, text) VALUES ('products', ?1, ?2)",
                        &[int(r as i64 + 1), text(t)],
                    )?;
                    rep.other += 1;
                    rep.derived -= 1;
                }
            }
            _ => {
                pos += 1;
                ins(
                    cx.db,
                    "INSERT INTO product (position, category, name, price, price_currency, price_date, dosage, in_use, units_per_year, notes, url, alt_url)
                     VALUES (?1, ?2, ?3, ?4, 'USD', ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    &[
                        int(pos),
                        opt_text(category.as_deref()),
                        text(&name),
                        opt_real(g.number(r, 1)),
                        if g.number(r, 1).is_some() { text(&cx.price_date) } else { Value::Null },
                        opt_text(g.text(r, 2)),
                        opt_int(yes_no(g.cell(r, 3))),
                        opt_real(g.number(r, 5)),
                        opt_text(g.text(r, 8)),
                        opt_text(g.text(r, 9)),
                        opt_text(g.text(r, 10)),
                    ],
                )?;
                rep.items += 1;
            }
        }
    }
    Ok(rep)
}

// ---------------------------------------------------------------- Nutrition

/// Column index -> nutrition column, in `nutrition_item` / `nutrition_target` order.
const NUTRI_COLS: &[(usize, &str)] = &[
    (2, "calories"),
    (3, "protein"),
    (4, "net_carbs"),
    (5, "fiber"),
    (6, "sugar"),
    (7, "sodium"),
    (8, "cholesterol"),
    (9, "total_fat"),
    (10, "unsat_fat"),
    (11, "trans_fat"),
    (12, "sat_fat"),
    (13, "yuka_score"),
];

fn nutrition(cx: &Cx, g: &Grid) -> Result<SheetReport> {
    let mut rep = SheetReport { sheet: "Nutrition".into(), rows_seen: rows_seen(g), ..Default::default() };
    let (mut category, mut pos, mut tpos) = (None::<String>, 0, 0);
    // Per-row `Qualifying meal?` (R) and `protein/cal` (T) formulas also sit on heading rows.
    let ignore = [17, 19];
    for r in 1..g.height() {
        let Some(name) = g.text(r, 0) else { continue };
        let lower = name.to_lowercase();
        // Total fat (col J) is `=sum(K:M)` where it is a formula: derived, left out. Every other
        // formula cell is an input written as arithmetic (`=11 - 1.5`): its value is stored.
        let num = |c: usize| if c == 9 && g.has_formula(r, c) { None } else { g.number(r, c) };
        if lower.contains("recommended") || lower.contains("goal values") {
            tpos += 1;
            let hints: Vec<String> = (14..18).filter_map(|c| g.text(r, c)).collect();
            let vals: Vec<Value> = NUTRI_COLS.iter().map(|(c, _)| opt_real(num(*c))).collect();
            let mut params = vec![int(tpos), text(&name)];
            params.extend(vals);
            params.push(if hints.is_empty() { Value::Null } else { text(serde_json::json!(hints).to_string()) });
            ins(
                cx.db,
                "INSERT INTO nutrition_target (position, label, calories, protein, net_carbs, fiber, sugar, sodium, cholesterol, total_fat, unsat_fat, trans_fat, sat_fat, yuka_score, extra)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                &params,
            )?;
            rep.other += 1;
            continue;
        }
        match classify(g, r, &ignore) {
            RowKind::Header => {
                category = Some(name);
                rep.headers += 1;
            }
            RowKind::Summary | RowKind::Blank => rep.derived += 1,
            RowKind::Data => {
                pos += 1;
                let mut params = vec![int(pos), opt_text(category.as_deref()), text(&name)];
                params.extend(NUTRI_COLS.iter().map(|(c, _)| opt_real(num(*c))));
                params.extend([
                    opt_text(g.text(r, 14)),
                    opt_text(g.text(r, 15)),
                    opt_text(g.text(r, 16)),
                    opt_text(g.text(r, 17).filter(|_| !g.has_formula(r, 17))),
                    opt_real(g.number(r, 18)),
                    opt_text(g.text(r, 20)),
                    opt_real(g.number(r, 21)),
                    if g.number(r, 21).is_some() { text(&cx.price_date) } else { Value::Null },
                    opt_text(g.text(r, 22)),
                    opt_text(g.text(r, 23)),
                ]);
                ins(
                    cx.db,
                    "INSERT INTO nutrition_item (position, category, name, calories, protein, net_carbs, fiber, sugar, sodium, cholesterol, total_fat, unsat_fat, trans_fat, sat_fat, yuka_score,
                        ingredients_ok, no_prep, vegan, qualifying, portions, notes, price, price_date, url, alt_url)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)",
                    &params,
                )?;
                rep.items += 1;
            }
        }
    }
    Ok(rep)
}

// ------------------------------------------------------------------ Routine

fn routine(cx: &Cx, g: &Grid) -> Result<SheetReport> {
    let mut rep = SheetReport { sheet: "Routine".into(), rows_seen: rows_seen(g), ..Default::default() };
    let title = |t: &str| t.to_lowercase().trim_end_matches("routine").trim().to_string();
    let mut routine = g.text(0, 0).map_or_else(|| "morning".to_string(), |t| title(&t));
    let mut pos = 0;
    for r in 1..g.height() {
        let Some(name) = g.text(r, 0) else { continue };
        match classify(g, r, &[]) {
            RowKind::Header => {
                routine = title(&name);
                pos = 0;
                rep.headers += 1;
            }
            RowKind::Summary => rep.derived += 1,
            _ => {
                pos += 1;
                ins(
                    cx.db,
                    "INSERT INTO routine_step (routine, position, name, effect_time, exec_seconds, kind, notes) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    &[
                        text(&routine),
                        int(pos),
                        text(&name),
                        opt_text(g.text(r, 1)),
                        opt_real(g.number(r, 2)),
                        opt_text(g.text(r, 3)),
                        opt_text(g.text(r, 4)),
                    ],
                )?;
                rep.items += 1;
            }
        }
    }
    Ok(rep)
}

// ------------------------------------------------------------------ Workout

fn workout(cx: &Cx, g: &Grid) -> Result<SheetReport> {
    let mut rep = SheetReport { sheet: "Workout".into(), rows_seen: rows_seen(g), ..Default::default() };
    let mut pos = 0;
    for r in 1..g.height() {
        let Some(name) = g.text(r, 0) else { continue };
        if g.text(r, 1).is_some_and(|b| is_url(&b)) {
            ins(
                cx.db,
                "INSERT INTO resource_link (position, section, label, url) VALUES (?1, 'workout', ?2, ?3)",
                &[int(r as i64 + 1), text(name), text(g.text(r, 1).unwrap_or_default())],
            )?;
            rep.other += 1;
            continue;
        }
        pos += 1;
        let id = ins(
            cx.db,
            "INSERT INTO exercise (position, name, muscles) VALUES (?1, ?2, ?3)",
            &[int(pos), text(&name), opt_text(g.text(r, 1))],
        )?;
        for (i, c) in (2..g.rows.get(r).map_or(0, Vec::len)).enumerate() {
            if let Some(alt) = g.text(r, c) {
                ins(
                    cx.db,
                    "INSERT INTO exercise_alt (exercise_id, position, text) VALUES (?1, ?2, ?3)",
                    &[int(id), int(i as i64 + 1), text(alt)],
                )?;
            }
        }
        rep.items += 1;
    }
    Ok(rep)
}

// ---------------------------------------------------------------- Vaccines

fn vaccines(cx: &Cx, g: &Grid) -> Result<SheetReport> {
    let mut rep = SheetReport { sheet: "Vaccines  Conditions".into(), rows_seen: rows_seen(g), ..Default::default() };
    let mut pos = 0;
    for r in 1..g.height() {
        let Some(name) = g.text(r, 0) else { continue };
        pos += 1;
        let doses: Vec<(usize, &Cell)> = (2..8).map(|c| (c - 1, g.cell(r, c))).filter(|(_, c)| !c.is_empty()).collect();
        let (disease_date, disease_note) =
            (iso(g.cell(r, 1)), if iso(g.cell(r, 1)).is_none() { g.text(r, 1) } else { None });
        let kind = if doses.is_empty() && disease_note.is_some() { "condition" } else { "vaccine" };
        let id = ins(
            cx.db,
            "INSERT INTO vaccine (position, kind, name, disease_date, disease_note, notes) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            &[int(pos), text(kind), text(&name), opt_text(disease_date), opt_text(disease_note), opt_text(g.text(r, 8))],
        )?;
        for (no, c) in doses {
            ins(
                cx.db,
                "INSERT INTO vaccine_dose (vaccine_id, dose_no, given_on, raw) VALUES (?1, ?2, ?3, ?4)",
                &[
                    int(id),
                    int(no as i64),
                    opt_text(iso(c)),
                    opt_text(if iso(c).is_none() { Some(c.text()) } else { None }),
                ],
            )?;
        }
        rep.items += 1;
    }
    Ok(rep)
}

// --------------------------------------------------------------- Physicals

/// Section titles of the Physicals sheet. They are plain bold cells with no
/// structural marker (a heading row looks exactly like an item row), so the
/// known titles are listed here.
const PHYSICAL_SECTIONS: &[&str] = &["regular checks", "genetic tests", "referrals"];

fn physicals(cx: &Cx, g: &Grid) -> Result<SheetReport> {
    let mut rep = SheetReport { sheet: "Physicals".into(), ..Default::default() };
    let mut section = String::from("Regular Checks");
    for r in 0..g.height() {
        // `position` is the sheet row (1-based) so the generator reproduces the layout.
        let row_no = int(r as i64 + 1);
        if let Some(name) = g.text(r, 0) {
            rep.rows_seen += 1;
            if PHYSICAL_SECTIONS.contains(&name.to_lowercase().as_str()) {
                section = name;
                rep.headers += 1;
            } else {
                let detail = g.text(r, 1);
                let done = detail.as_deref().is_some_and(|d| d.contains('✅'));
                ins(
                    cx.db,
                    "INSERT INTO physical_check (position, section, name, detail, done) VALUES (?1, ?2, ?3, ?4, ?5)",
                    &[row_no.clone(), text(&section), text(name), opt_text(detail), int(i64::from(done))],
                )?;
                rep.items += 1;
            }
        }
        if r > 0 {
            if let Some(doc) = g.text(r, 3) {
                ins(
                    cx.db,
                    "INSERT INTO provider (position, name, note, url) VALUES (?1, ?2, ?3, ?4)",
                    &[row_no, text(doc), opt_text(g.text(r, 4)), opt_text(g.text(r, 5))],
                )?;
                rep.other += 1;
            }
        }
    }
    Ok(rep)
}

// ------------------------------------------------------------------- Sizes

fn sizes(cx: &Cx, g: &Grid) -> Result<SheetReport> {
    let mut rep = SheetReport { sheet: "Sizes".into(), rows_seen: rows_seen(g), ..Default::default() };
    let (mut category, mut pos) = (String::new(), 0);
    for r in 1..g.height() {
        let Some(name) = g.text(r, 0) else { continue };
        // The cm column is a formula that also sits on heading rows.
        match classify(g, r, &[2]) {
            RowKind::Header => {
                category = name;
                rep.headers += 1;
            }
            _ => {
                pos += 1;
                // Everything is a length in inches except the shoe size, which the sheet
                // multiplied by 2.54 anyway; it is stored as a bare (US) size.
                let unit = if name.eq_ignore_ascii_case("shoes") { "us" } else { "in" };
                ins(
                    cx.db,
                    "INSERT INTO size (position, category, name, value, unit, note) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    &[
                        int(pos),
                        text(&category),
                        text(&name),
                        opt_real(g.number(r, 1)),
                        text(unit),
                        opt_text(g.text(r, 3)),
                    ],
                )?;
                rep.items += 1;
            }
        }
    }
    Ok(rep)
}

// ------------------------------------------------------------ Weight & Age

/// The intake columns of `Weight & Age`: net calories (F), energy out (G) and
/// protein (H) per day. Weight is a lab series and stays in biomarker-cli; the
/// projected weight, ages, BMI and ideal protein columns are formulas.
fn intake(cx: &Cx, g: &Grid) -> Result<SheetReport> {
    let mut rep = SheetReport { sheet: "Weight & Age (intake)".into(), ..Default::default() };
    for r in 1..g.height() {
        // Dates are typed once and then `=A2+1`: the cached value is the date either way.
        let Some(date) = iso(g.cell(r, 0)) else { continue };
        let vals = [g.number(r, 5), g.number(r, 6), g.number(r, 7)];
        if vals.iter().all(Option::is_none) {
            continue;
        }
        ins(
            cx.db,
            "INSERT INTO intake_log (date, net_kcal, energy_out_kcal, protein_g) VALUES (?1, ?2, ?3, ?4)",
            &[text(date), opt_real(vals[0]), opt_real(vals[1]), opt_real(vals[2])],
        )?;
        rep.items += 1;
    }
    rep.rows_seen = rep.items;
    Ok(rep)
}

// --------------------------------------------------------------- Resources

fn resources(cx: &Cx, g: &Grid) -> Result<SheetReport> {
    let mut rep = SheetReport { sheet: "Resources".into(), ..Default::default() };
    for r in 0..g.height() {
        let Some(url) = g.text(r, 1).filter(|u| is_url(u)) else { continue };
        ins(
            cx.db,
            "INSERT INTO resource_link (position, section, label, url) VALUES (?1, 'resources', ?2, ?3)",
            &[int(r as i64 + 1), opt_text(g.text(r, 0)), text(url)],
        )?;
        rep.items += 1;
    }
    rep.rows_seen = rep.items;
    Ok(rep)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_come_off_labels() {
        assert_eq!(split_unit("Height (in.)"), ("Height".into(), Some("in".into())));
        assert_eq!(split_unit("Blood Group"), ("Blood Group".into(), None));
    }
}
