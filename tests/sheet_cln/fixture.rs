//! Fixture and helpers for `tests/sheet_cln.rs`: a synthetic three-panel lab history
//! (every value made up), workbook reading helpers and the LibreOffice recalculation.

#![allow(dead_code)]

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub use crate::support::soffice::soffice;
use calamine::{open_workbook, Data as Cell, Range, Reader, Xlsx};
use chrono::NaiveDate;
use health_cli::workbook::labs::{Labs, MarkerInfo, Measurement};
use health_cli::workbook::{self, Data};
use serde_json::{json, Map, Value};

// ------------------------------------------------------------------ fixture

pub const P: [&str; 3] = ["2023-01-10", "2024-01-15", "2025-02-20"];
pub const DOB: &str = "1990-01-15";

/// (marker, category, unit, ref_lo, ref_hi, values on P[0..3]).
type Row = (&'static str, &'static str, &'static str, Option<f64>, Option<f64>, [Option<f64>; 3]);
const LABS: &[Row] = &[
    ("total-cholesterol", "lipid", "mg/dL", None, Some(199.0), [Some(220.0), Some(200.0), Some(180.0)]),
    ("hdl-c", "lipid", "mg/dL", Some(39.0), None, [Some(50.0), Some(55.0), Some(60.0)]),
    ("ldl-c", "lipid", "mg/dL", None, Some(99.0), [Some(140.0), Some(120.0), Some(100.0)]),
    ("triglycerides", "lipid", "mg/dL", None, Some(149.0), [Some(150.0), Some(110.0), Some(80.0)]),
    ("apob", "lipid", "mg/dL", None, Some(90.0), [Some(110.0), Some(95.0), Some(80.0)]),
    ("vldl-c", "lipid", "mg/dL", Some(5.0), Some(40.0), [Some(30.0), None, None]),
    ("chol-hdl-ratio", "lipid", "ratio", None, Some(5.0), [Some(4.4), None, None]),
    ("glucose", "metabolic", "mg/dL", Some(70.0), Some(99.0), [Some(92.0), Some(88.0), Some(130.0)]),
    ("hba1c", "metabolic", "%", Some(4.8), Some(5.6), [Some(5.4), Some(5.3), Some(5.5)]),
    ("estimated-average-glucose", "metabolic", "mg/dL", None, None, [Some(108.0), None, None]),
    ("insulin", "metabolic", "uIU/mL", Some(2.6), Some(24.9), [None, Some(6.0), None]),
    ("sodium", "cmp", "mmol/L", Some(134.0), Some(144.0), [Some(140.0), None, None]),
    ("chloride", "cmp", "mmol/L", Some(96.0), Some(106.0), [Some(102.0), None, None]),
    ("co2", "cmp", "mmol/L", Some(20.0), Some(29.0), [Some(25.0), None, None]),
    ("anion-gap", "cmp", "mmol/L", Some(3.0), Some(11.0), [Some(13.0), None, None]),
    ("albumin", "cmp", "g/dL", Some(3.8), Some(4.9), [Some(4.5), None, None]),
    ("globulin", "cmp", "g/dL", Some(1.5), Some(4.5), [Some(2.5), None, None]),
    ("a-g-ratio", "cmp", "ratio", Some(1.2), Some(2.2), [Some(1.8), None, None]),
    ("total-protein", "cmp", "g/dL", Some(6.0), Some(8.5), [Some(7.0), None, None]),
    ("creatinine", "cmp", "mg/dL", Some(0.76), Some(1.27), [Some(1.0), Some(1.1), None]),
    ("egfr", "cmp", "mL/min/1.73", Some(59.0), None, [Some(95.0), None, None]),
    ("bun", "cmp", "mg/dL", Some(6.0), Some(24.0), [Some(15.0), None, None]),
    ("bun-creatinine-ratio", "cmp", "ratio", Some(9.0), Some(20.0), [Some(15.0), None, None]),
    ("calcium", "cmp", "mg/dL", Some(8.7), Some(10.2), [Some(9.6), None, None]),
    ("alt", "cmp", "U/L", Some(0.0), Some(44.0), [Some(30.0), Some(28.0), None]),
    ("ast", "cmp", "U/L", Some(0.0), Some(40.0), [Some(25.0), Some(24.0), None]),
    ("platelets", "cbc", "10^3/uL", Some(150.0), Some(450.0), [Some(250.0), Some(240.0), None]),
    ("hemoglobin", "cbc", "g/dL", Some(13.0), Some(17.7), [Some(15.0), None, None]),
    ("mcv", "cbc", "fL", Some(79.0), Some(97.0), [Some(90.0), None, None]),
    ("rdw", "cbc", "%", Some(11.6), Some(15.4), [Some(13.0), None, None]),
    ("rbc", "cbc", "10^6/uL", Some(4.14), Some(5.8), [Some(5.0), None, None]),
    ("neutrophils-abs", "cbc", "10^3/uL", Some(1.4), Some(7.0), [Some(3.0), Some(4.0), None]),
    ("lymphocytes-abs", "cbc", "10^3/uL", Some(0.7), Some(3.1), [Some(2.0), Some(1.0), None]),
    ("tsh", "thyroid", "uIU/mL", Some(0.45), Some(4.5), [Some(2.0), None, None]),
    ("uric-acid", "metabolic", "mg/dL", Some(3.8), Some(8.4), [None, None, Some(7.5)]),
];

pub fn day(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

fn m(marker: &str, cat: &str, at: &str, v: f64, unit: &str, lo: Option<f64>, hi: Option<f64>) -> Measurement {
    Measurement {
        marker: marker.into(),
        name: marker.into(),
        category: cat.into(),
        taken_at: day(at),
        qualifier: None,
        value: v,
        unit: unit.into(),
        ref_low: lo,
        ref_high: hi,
        flag: None,
        value_raw: Some(v),
        unit_raw: unit.into(),
        lab: Some("Synthetic Lab".into()),
        fasting: Some(true),
        opt_low: None,
        opt_high: None,
    }
}

fn rec(v: Value) -> Map<String, Value> {
    v.as_object().unwrap().clone()
}

pub fn fixture() -> Data {
    let mut canonical = Vec::new();
    for (marker, cat, unit, lo, hi, vals) in LABS {
        for (i, v) in vals.iter().enumerate() {
            if let Some(v) = v {
                canonical.push(m(marker, cat, P[i], *v, unit, *lo, *hi));
            }
        }
    }
    let mut measurements = canonical.clone();
    measurements.push(m("weight", "body", P[0], 190.0, "lb", None, None));
    canonical.push(m("weight", "body", P[0], 190.0 * 0.45359237, "kg", None, None));
    let markers = LABS
        .iter()
        .map(|(slug, cat, unit, ..)| MarkerInfo {
            slug: (*slug).into(),
            name: (*slug).into(),
            category: (*cat).into(),
            unit: (*unit).into(),
            description: None,
        })
        .collect();
    let supp = |id: i64, name: &str, amount: f64, unit: &str| {
        rec(json!({"id": id, "name": name, "taking": 1, "amount": amount, "unit": unit, "frequency": "Daily"}))
    };
    Data {
        facts: vec![
            rec(json!({"key": "birthdate", "label": "Birthdate", "value": DOB, "kind": "date"})),
            rec(json!({"key": "height", "label": "Height", "value": "70", "kind": "number", "unit": "in"})),
        ],
        supplements: vec![
            supp(1, "Biotin", 10000.0, "mcg"),
            supp(2, "Creatine Monohydrate", 5.0, "g"),
            supp(3, "Vitamin D3", 2000.0, "IU"),
        ],
        // Protocols charts its sources and fails on none (outside this workstream).
        sources: vec![rec(json!({"id": 1, "name": "Test Protocol"}))],
        labs: Some(Labs { measurements, canonical, markers, ..Labs::default() }),
        generated_on: Some(day("2025-03-01")),
        ..Data::default()
    }
}

pub fn build(dir: &Path) -> PathBuf {
    let mut wb = workbook::build(&fixture(), &[]).expect("workbook builds");
    let p = dir.join("cln.xlsx");
    wb.save(&p).unwrap();
    // Keep a copy for inspection: CLN_WORKBOOK_OUT=/path/to/out.xlsx cargo test --test sheet_cln
    if let Some(out) = std::env::var_os("CLN_WORKBOOK_OUT") {
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

/// Every cell of a sheet as text, for "contains" checks.
pub fn sheet_text(wb: &mut Xlsx<std::io::BufReader<std::fs::File>>, name: &str) -> String {
    let r = wb.worksheet_range(name).unwrap();
    r.cells().map(|(_, _, v)| v.to_string()).collect::<Vec<_>>().join("\n")
}

/// (row, col) of the header cell `h` of the table whose first header is `first`.
pub fn header(r: &Range<Cell>, first: &str, h: &str) -> (u32, u32) {
    let hr = (0..r.height() as u32).find(|i| text(r, *i, 0) == first).unwrap_or_else(|| panic!("no header {first}"));
    let c = (0..r.width() as u32).find(|c| text(r, hr, *c) == h).unwrap_or_else(|| panic!("no column {h}"));
    (hr, c)
}

/// Value under header `h` on data row `i` (0-based) of the table starting with `first`.
pub fn at(r: &Range<Cell>, first: &str, h: &str, i: u32) -> Cell {
    let (hr, c) = header(r, first, h);
    r.get_value((hr + 1 + i, c)).cloned().unwrap_or(Cell::Empty)
}

pub fn num(v: &Cell) -> f64 {
    match v {
        Cell::Float(f) => *f,
        Cell::Int(i) => *i as f64,
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
        writeln!(f, "sheet_cln: {reason}").unwrap();
    }
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
