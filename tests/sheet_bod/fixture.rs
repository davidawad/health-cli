//! Fixture and helpers for `tests/sheet_bod.rs`: a synthetic 120-day weight / intake
//! log with gaps, a weekly cycle and a step change (every value made up), workbook
//! reading helpers and the LibreOffice recalculation.

#![allow(dead_code)]

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use calamine::{open_workbook, Data as Cell, Range, Reader, Xlsx};
use chrono::{Duration, NaiveDate};
use health_cli::workbook::labs::{Labs, Measurement};
use health_cli::workbook::{self, Data};
use serde_json::{json, Map, Value};

pub const START: &str = "2024-01-01";
pub const DAYS: i64 = 120;
pub const DOB: &str = "1990-01-15";

pub fn day(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

/// Synthetic weigh-in on day `i` (lb), or None on skipped days (every 9th day and days 50-53).
pub fn weight(i: i64) -> Option<f64> {
    if i % 9 == 4 || (50..=53).contains(&i) {
        return None;
    }
    let step = if i >= 80 { 3.0 } else { 0.0 };
    let wave = [0.0, 0.6, 0.2, -0.4, -0.8, -0.2, 0.6][(i % 7) as usize];
    Some(((200.0 - 0.1 * i as f64 + wave + step) * 10.0).round() / 10.0)
}

/// Synthetic intake on day `i`: (kcal, protein g, energy out), None every 6th day.
pub fn intake(i: i64) -> Option<(f64, f64, f64)> {
    if i % 6 == 5 {
        return None;
    }
    Some((1800.0 + 100.0 * (i % 5) as f64, 150.0 + 10.0 * (i % 3) as f64, 2300.0))
}

fn weigh(at: NaiveDate, v: f64) -> Measurement {
    Measurement {
        marker: "weight".into(),
        name: "Weight".into(),
        category: "body".into(),
        taken_at: at,
        qualifier: None,
        value: v,
        unit: "lb".into(),
        ref_low: None,
        ref_high: None,
        flag: None,
        value_raw: Some(v),
        unit_raw: "lb".into(),
        lab: None,
        fasting: None,
        opt_low: None,
        opt_high: None,
    }
}

fn rec(v: Value) -> Map<String, Value> {
    v.as_object().unwrap().clone()
}

pub fn fixture() -> Data {
    let start = day(START);
    let mut measurements = Vec::new();
    let mut log = Vec::new();
    for i in 0..DAYS {
        let at = start + Duration::days(i);
        if let Some(w) = weight(i) {
            measurements.push(weigh(at, w));
        }
        if let Some((k, p, o)) = intake(i) {
            log.push(rec(json!({"date": at.to_string(), "net_kcal": k, "protein_g": p, "energy_out_kcal": o})));
        }
    }
    let supp = |id: i64, name: &str, amount: f64, unit: &str, tod: &str| {
        rec(json!({"id": id, "name": name, "taking": 1, "amount": amount, "unit": unit, "frequency": "Daily",
                   "time_of_day": tod, "price": 20.0, "units_per_year": 4.0}))
    };
    let food = |id: i64, name: &str, kcal: f64, protein: f64, price: f64, yuka: f64, cat: &str| {
        rec(json!({"id": id, "name": name, "category": cat, "calories": kcal, "protein": protein, "net_carbs": 20.0,
                   "fiber": 8.0, "total_fat": 5.0, "sat_fat": 1.0, "yuka_score": yuka, "price": price, "portions": 10.0}))
    };
    Data {
        facts: vec![
            rec(json!({"key": "birthdate", "label": "Birthdate", "value": DOB, "kind": "date"})),
            rec(json!({"key": "height", "label": "Height", "value": "70", "kind": "number", "unit": "in"})),
        ],
        supplements: vec![
            supp(1, "Caffeine", 100.0, "mg", "Day"),
            supp(2, "Creatine Monohydrate", 5.0, "g", "Day"),
            supp(3, "Magnesium Glycinate", 200.0, "mg", "Before bed"),
        ],
        nutrition: vec![
            food(1, "Black beans", 220.0, 15.0, 1.5, 90.0, "beans"),
            food(2, "Whey isolate", 120.0, 25.0, 1.2, 70.0, "protein"),
            food(3, "Blueberries", 80.0, 1.0, 3.0, 100.0, "fruit"),
            food(4, "Broccoli", 50.0, 4.0, 1.0, 100.0, "vegetable"),
        ],
        sizes: vec![rec(json!({"id": 1, "category": "body", "name": "Waist", "value": 34.0, "unit": "in"}))],
        // Protocols charts its sources and fails on none (outside this workstream).
        sources: vec![rec(json!({"id": 1, "name": "Test Protocol"}))],
        intake: log,
        labs: Some(Labs {
            measurements: measurements.clone(),
            canonical: Vec::new(),
            markers: Vec::new(),
            ..Labs::default()
        }),
        generated_on: Some(day("2024-05-01")),
        ..Data::default()
    }
}

pub fn build(dir: &Path) -> PathBuf {
    let mut wb = workbook::build(&fixture(), &[]).expect("workbook builds");
    let p = dir.join("bod.xlsx");
    wb.save(&p).unwrap();
    // Keep a copy for inspection: BOD_WORKBOOK_OUT=/path/to/out.xlsx cargo test --test sheet_bod
    if let Some(out) = std::env::var_os("BOD_WORKBOOK_OUT") {
        std::fs::copy(&p, out).unwrap();
    }
    p
}

// ------------------------------------------------------------------ helpers

pub fn open(p: &Path) -> Xlsx<std::io::BufReader<std::fs::File>> {
    open_workbook(p).unwrap()
}

pub fn text(r: &Range<Cell>, row: u32, col: u32) -> String {
    r.get_value((row, col)).map(ToString::to_string).unwrap_or_default()
}

pub fn part(p: &Path, name: &str) -> String {
    let mut z = zip::ZipArchive::new(std::fs::File::open(p).unwrap()).unwrap();
    let mut s = String::new();
    z.by_name(name).unwrap().read_to_string(&mut s).unwrap();
    s
}

/// Worksheet XML of a sheet by its tab position.
pub fn sheet_xml(p: &Path, names: &[String], sheet: &str) -> String {
    let i = names.iter().position(|n| n == sheet).unwrap_or_else(|| panic!("no sheet {sheet}"));
    part(p, &format!("xl/worksheets/sheet{}.xml", i + 1))
}

/// Every cell of a sheet as text, for "contains" checks.
pub fn sheet_text(wb: &mut Xlsx<std::io::BufReader<std::fs::File>>, name: &str) -> String {
    let r = wb.worksheet_range(name).unwrap();
    r.cells().map(|(_, _, v)| v.to_string()).collect::<Vec<_>>().join("\n")
}

/// Every formula of a sheet, joined by newlines.
pub fn formulas(wb: &mut Xlsx<std::io::BufReader<std::fs::File>>, name: &str) -> String {
    let f = wb.worksheet_formula(name).unwrap();
    f.cells().map(|(_, _, x)| x.clone()).filter(|x| !x.is_empty()).collect::<Vec<_>>().join("\n")
}

/// (row, col) of the header cell `h` of the table whose first header (in any column) is `first`.
pub fn header(r: &Range<Cell>, first: &str, h: &str) -> (u32, u32) {
    let (hr, c0) = r
        .cells()
        .find(|(_, _, v)| matches!(v, Cell::String(s) if s == first))
        .map(|(row, c, _)| (row as u32, c as u32))
        .unwrap_or_else(|| panic!("no header {first}"));
    let c = (c0..r.width() as u32).find(|c| text(r, hr, *c) == h).unwrap_or_else(|| panic!("no column {h}"));
    (hr, c)
}

/// Value under header `h` on data row `i` (0-based) of the table starting with `first`.
pub fn at(r: &Range<Cell>, first: &str, h: &str, i: u32) -> Cell {
    let (hr, c) = header(r, first, h);
    r.get_value((hr + 1 + i, c)).cloned().unwrap_or(Cell::Empty)
}

/// The value right of the first cell whose text is `label`.
pub fn right_of(r: &Range<Cell>, label: &str) -> Cell {
    let (row, col, _) = r
        .cells()
        .find(|(_, _, v)| matches!(v, Cell::String(s) if s == label))
        .unwrap_or_else(|| panic!("no label {label}"));
    r.get_value((row as u32, col as u32 + 1)).cloned().unwrap_or(Cell::Empty)
}

pub fn num(v: &Cell) -> f64 {
    match v {
        Cell::Float(f) => *f,
        Cell::Int(i) => *i as f64,
        Cell::DateTime(d) => d.as_f64(),
        other => panic!("not a number: {other:?}"),
    }
}

pub fn close(a: f64, b: f64, what: &str) {
    assert!((a - b).abs() < 1e-3 * b.abs().max(1.0), "{what}: {a} vs expected {b}");
}

pub fn skip(reason: &str) {
    eprintln!("SKIP: {reason}");
    if let Some(p) = std::env::var_os("TEST_SKIP_LOG") {
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(p).unwrap();
        writeln!(f, "sheet_bod: {reason}").unwrap();
    }
}

pub fn soffice() -> Option<String> {
    ["soffice", "/Applications/LibreOffice.app/Contents/MacOS/soffice", "/opt/homebrew/bin/soffice"]
        .iter()
        .find(|c| Command::new(c).arg("--version").output().is_ok_and(|o| o.status.success()))
        .map(|c| (*c).to_string())
}

/// Recalculate in headless LibreOffice (private profile, always recalc on load).
pub fn recalc(soffice: &str, dir: &Path, input: &Path) -> Xlsx<std::io::BufReader<std::fs::File>> {
    let profile = dir.join("lo-profile");
    std::fs::create_dir_all(profile.join("user")).unwrap();
    std::fs::write(
        profile.join("user/registrymodifications.xcu"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<oor:items xmlns:oor="http://openoffice.org/2001/registry" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
<item oor:path="/org.openoffice.Office.Calc/Formula/Load"><prop oor:name="OOXMLRecalcMode" oor:op="fuse"><value>0</value></prop></item>
</oor:items>"#,
    )
    .unwrap();
    let outdir = dir.join("recalculated");
    let out = Command::new(soffice)
        .arg(format!("-env:UserInstallation=file://{}", profile.display()))
        .args(["--headless", "--convert-to", "xlsx", "--outdir"])
        .arg(&outdir)
        .arg(input)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    open_workbook(outdir.join(input.file_name().unwrap())).unwrap()
}

/// Recalculated workbook, or None (and a recorded SKIP) without LibreOffice.
pub fn recalculated(dir: &Path) -> Option<Xlsx<std::io::BufReader<std::fs::File>>> {
    let Some(so) = soffice() else {
        skip("soffice (LibreOffice) not installed: recalculated values not checked");
        return None;
    };
    let p = build(dir);
    Some(recalc(&so, dir, &p))
}

/// Every formula error in the `Bod_*` sheets of a recalculated workbook. `#N/A` is
/// accepted where the formula asks for it (`NA()`: a gap in a chart-only column).
pub fn errors(wb: &mut Xlsx<std::io::BufReader<std::fs::File>>) -> Vec<String> {
    let mut out = Vec::new();
    for s in wb.sheet_names().into_iter().filter(|n| n.starts_with("Bod_")) {
        let r = wb.worksheet_range(&s).unwrap();
        let f = wb.worksheet_formula(&s).unwrap();
        for (row, col, v) in r.cells() {
            if let Cell::Error(e) = v {
                let (ar, ac) = (r.start().unwrap().0 + row as u32, r.start().unwrap().1 + col as u32);
                let formula = f.get_value((ar, ac)).cloned().unwrap_or_default();
                if matches!(e, calamine::CellErrorType::NA) && formula.contains("NA()") {
                    continue;
                }
                out.push(format!("{s}!R{}C{}: {e:?} {formula}", ar + 1, ac + 1));
            }
        }
    }
    out
}

/// Time-based EMA of the synthetic weigh-ins, as `Bod_Weight` computes it.
pub fn ema(span: f64) -> Vec<Option<f64>> {
    let a = 2.0 / (span + 1.0);
    let (mut prev, mut last_obs): (Option<f64>, Option<i64>) = (None, None);
    (0..DAYS)
        .map(|i| {
            if let Some(w) = weight(i) {
                let g = last_obs.map_or(1, |l| (i - l).max(1));
                prev = Some(match prev {
                    Some(p) => p + (1.0 - (1.0 - a).powi(g as i32)) * (w - p),
                    None => w,
                });
                last_obs = Some(i);
            }
            prev
        })
        .collect()
}
