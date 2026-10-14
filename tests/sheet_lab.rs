//! Workstream LAB (`Lab_*` sheets): `health sheet` over the synthetic store and
//! the synthetic `biomarker` stand-in. Structure is read back with calamine
//! and from the XML parts; values are checked after a headless LibreOffice
//! recalculation when `soffice` is installed (otherwise that test records a
//! skip in `$TEST_SKIP_LOG` and returns).

#![cfg(unix)]

mod support;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use calamine::{open_workbook, Data, Reader, Xlsx};
use support::soffice::soffice;
use support::Env;

const LAB_SHEETS: &[&str] = &["Lab_Tear", "Lab_Stats", "Lab_Change", "Lab_Quilt", "Lab_Exp", "Lab_Ranges", "Lab_Long"];

/// Lab markers of the stub in display order (CBC, lipids, metabolic; body measures excluded).
const MARKERS: &[&str] = &["wbc", "ldl-c", "hdl-c", "glucose", "hba1c"];

fn generate(e: &Env, extra: &[&str]) -> PathBuf {
    e.write_stubs();
    e.import();
    let out = e.path("Health Dashboard (generated).xlsx");
    let mut args = vec!["sheet", "--out", out.to_str().unwrap()];
    args.extend(extra);
    e.run(&args);
    out
}

fn open(p: &Path) -> Xlsx<std::io::BufReader<std::fs::File>> {
    open_workbook(p).expect("generated workbook opens in a reader")
}

fn text(r: &calamine::Range<Data>, row: u32, col: u32) -> String {
    r.get_value((row, col)).map(ToString::to_string).unwrap_or_default()
}

fn num(r: &calamine::Range<Data>, row: u32, col: u32) -> f64 {
    match r.get_value((row, col)) {
        Some(Data::Float(f)) => *f,
        Some(Data::Int(i)) => *i as f64,
        Some(Data::DateTime(d)) => d.as_f64(),
        other => panic!("R{}C{} is not a number: {other:?}", row + 1, col + 1),
    }
}

fn part(p: &Path, name: &str) -> String {
    let mut z = zip::ZipArchive::new(std::fs::File::open(p).unwrap()).unwrap();
    let mut s = String::new();
    z.by_name(name).unwrap_or_else(|_| panic!("no part {name}")).read_to_string(&mut s).unwrap();
    s
}

/// The worksheet XML of `sheet` (sheetN.xml follows workbook order).
fn sheet_xml(p: &Path, wb: &Xlsx<std::io::BufReader<std::fs::File>>, sheet: &str) -> String {
    let i = wb.sheet_names().iter().position(|s| s == sheet).unwrap();
    part(p, &format!("xl/worksheets/sheet{}.xml", i + 1))
}

/// Record a skipped test where the runner collects them (cargo hides a passing test's stderr).
fn skip(test: &str, reason: &str) {
    eprintln!("SKIP {test}: {reason}");
    if let Some(log) = std::env::var_os("TEST_SKIP_LOG") {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(log).unwrap();
        writeln!(f, "sheet_lab::{test}: {reason}").unwrap();
    }
}

#[test]
fn lab_sheets_follow_the_views_and_the_data_sheet_is_hidden() {
    let e = Env::new();
    let out = generate(&e, &[]);
    let wb = open(&out);
    let names = wb.sheet_names();
    let lab: Vec<&str> = names.iter().map(String::as_str).filter(|n| n.starts_with("Lab_")).collect();
    assert_eq!(lab, LAB_SHEETS);
    let wbx = part(&out, "xl/workbook.xml");
    assert!(wbx.contains("name=\"Lab_Long\" sheetId=\"") && wbx.contains("state=\"hidden\""));
    assert!(sheet_xml(&out, &wb, "Lab_Stats").contains("<tabColor"), "Lab_ tabs carry the LAB colour");
}

#[test]
fn the_long_table_holds_each_lab_draw_grouped_by_marker_oldest_first() {
    let e = Env::new();
    let mut wb = open(&generate(&e, &[]));
    let r = wb.worksheet_range("Lab_Long").unwrap();
    assert_eq!((0..5).map(|c| text(&r, 0, c)).collect::<Vec<_>>(), ["Marker", "Name", "Category", "Date", "Value"]);
    let rows: Vec<(String, f64)> = (1..r.height() as u32).map(|i| (text(&r, i, 0), num(&r, i, 4))).collect();
    let expect = [("wbc", 5.6), ("ldl-c", 131.0), ("ldl-c", 96.0), ("hdl-c", 55.0), ("glucose", 88.0), ("hba1c", 5.7)];
    assert_eq!(rows.iter().map(|(m, v)| (m.as_str(), *v)).collect::<Vec<_>>(), expect);
    assert!(!rows.iter().any(|(m, _)| m == "weight" || m == "biological-age"), "body measures stay out");
}

#[test]
fn views_are_formulas_over_named_lab_columns() {
    let e = Env::new();
    let out = generate(&e, &[]);
    let mut wb = open(&out);
    let stats = wb.worksheet_range("Lab_Stats").unwrap();
    assert_eq!(text(&stats, 3, 0), "Marker");
    assert_eq!((4..9).map(|i| text(&stats, i, 0)).collect::<Vec<_>>(), MARKERS);
    let sf = wb.worksheet_formula("Lab_Stats").unwrap();
    let at = |r: &calamine::Range<String>, row: u32, col: u32| r.get_value((row, col)).cloned().unwrap_or_default();
    assert_eq!(at(&sf, 5, 4), "COUNTIF(Lab_L_Marker,$A6)");
    assert!(at(&sf, 5, 9).contains("AVERAGE(INDEX(Lab_L_Value,$F6):INDEX(Lab_L_Value,$F6+$E6-1))"));
    assert!(at(&sf, 5, 15).contains("SLOPE(") && at(&sf, 5, 15).contains("Lab_Min_n_trend"));
    assert!(sheet_xml(&out, &wb, "Lab_Stats").contains("sparklineGroup"), "trend sparklines");

    let tf = wb.worksheet_formula("Lab_Tear").unwrap();
    assert!(at(&tf, 3, 1).starts_with("IF(LEN(Sel_Marker)>0,Sel_Marker,\"wbc\")"), "{}", at(&tf, 3, 1));
    let hist = (29..31).map(|r| (1..16).map(|c| at(&tf, r, c)).collect::<Vec<_>>()).collect::<Vec<_>>();
    for needle in ["Boll_Window", "Boll_K", "Winsor_Cap", "CUSUM_k", "Half_Life_days", "Kalman_q", "Kalman_r"] {
        assert!(hist.iter().flatten().any(|f| f.contains(needle)), "{needle} drives the tear sheet");
    }
    assert!(!part(&out, "xl/charts/chart1.xml").is_empty());

    let cf = wb.worksheet_formula("Lab_Change").unwrap();
    assert!(at(&cf, 6, 13).contains("RCV_z*SQRT(2)*SQRT($L7^2+$M7^2)"), "{}", at(&cf, 6, 13));

    let q = wb.worksheet_range("Lab_Quilt").unwrap();
    assert_eq!(text(&q, 4, 0), "Marker");
    assert_eq!(num(&q, 4, 7), 45047.0, "one column per distinct lab draw date (2023-05-01)");
    assert_eq!(text(&q, 4, 9), "", "two lab draw dates in the stub");
    assert!(sheet_xml(&out, &wb, "Lab_Quilt").contains("AND(ISNUMBER(H6),H6=3)"), "status quilt colours");
}

#[test]
fn experiments_ship_as_an_empty_input_table_awaiting_start_dates() {
    let e = Env::new();
    let out = generate(&e, &[]);
    let mut wb = open(&out);
    let r = wb.worksheet_range("Lab_Exp").unwrap();
    assert!(text(&r, 2, 0).starts_with("PLACEHOLDER - awaiting data: intervention start dates"));
    assert_eq!(
        (text(&r, 5, 0), text(&r, 5, 2), text(&r, 5, 15)),
        ("Experiment".into(), "Start date".into(), "Welch t".into())
    );
    for row in 6..16 {
        assert!((0..6).all(|c| text(&r, row, c).is_empty()), "input row {row} is empty");
    }
    let f = wb.worksheet_formula("Lab_Exp").unwrap();
    assert!(f.get_value((6, 17)).unwrap().contains("TDIST("));
    let x = sheet_xml(&out, &wb, "Lab_Exp");
    assert!(x.contains("<dataValidation ") && x.contains("Lab_R_Marker"), "marker picked from the reference list");
    let z = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
    let tables: Vec<String> = z.file_names().filter(|n| n.starts_with("xl/tables/")).map(String::from).collect();
    assert!(tables.iter().any(|t| part(&out, t).contains("name=\"tblLabExp\"")), "tblLabExp is a real table");
}

#[test]
fn names_parameters_and_kpis_are_registered() {
    let e = Env::new();
    let out = generate(&e, &[]);
    let wbx = part(&out, "xl/workbook.xml");
    let kpis = [
        "n_markers",
        "n_draw_dates",
        "last_draw",
        "n_out",
        "pct_out",
        "n_stale",
        "n_beyond_rcv",
        "n_delta",
        "n_critical",
        "n_exp_signal",
    ];
    let names = ["Lab_L_Marker", "Lab_L_Value", "Lab_R_Marker", "Lab_R_CVi", "Lab_Sel", "Lab_T_n", "Lab_T_first"];
    for n in names.iter().map(|s| s.to_string()).chain(kpis.iter().map(|k| format!("K_LAB_{k}"))) {
        assert!(wbx.contains(&format!("definedName name=\"{n}\"")), "{n} not defined");
    }
    let mut wb = open(&out);
    let p = wb.worksheet_range("Params").unwrap();
    let row = |name: &str| {
        (1..p.height() as u32)
            .find(|&i| text(&p, i, 0) == name)
            .map(|i| (0..5).map(|c| text(&p, i, c)).collect::<Vec<_>>())
            .unwrap_or_else(|| panic!("{name} not on Params"))
    };
    assert_eq!(row("Lab_Stale_days")[1..4], ["365", "days", "heuristic"]);
    assert_eq!(row("Lab_Exp_Baseline_days")[3], "heuristic");
    let ranges = wb.worksheet_range("Lab_Ranges").unwrap();
    assert!(text(&ranges, 1, 0).starts_with("UNVERIFIED - pending:"));
    for i in 4..9 {
        assert_eq!(text(&ranges, i, 11), "UNVERIFIED");
        assert!(text(&ranges, i, 12).starts_with("pending: EFLM"));
        assert!((5..11).all(|c| text(&ranges, i, c).is_empty()), "optimal / critical / CV inputs start blank");
    }
    assert_eq!((num(&ranges, 5, 3), num(&ranges, 5, 4)), (0.0, 99.0), "ldl-c reference interval from biomarker");
}

#[test]
fn only_a_lab_view_still_brings_its_data_sheets() {
    let e = Env::new();
    let wb = open(&generate(&e, &["--only", "lab-tear"]));
    let names = wb.sheet_names();
    for s in ["Lab_Tear", "Lab_Ranges", "Lab_Long", "Params", "Ref_KPI"] {
        assert!(names.iter().any(|n| n == s), "{s} missing from {names:?}");
    }
    assert!(!names.iter().any(|n| n == "Lab_Stats"));
}

#[test]
fn no_lab_sheets_without_labs() {
    let e = Env::new();
    e.import(); // no stubs: biomarker is "not installed"
    let out = e.path("no-labs.xlsx");
    let res = e.cmd().args(["sheet", "--out", out.to_str().unwrap()]).output().unwrap();
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    let wb = open(&out);
    assert!(!wb.sheet_names().iter().any(|n| n.starts_with("Lab_")), "{:?}", wb.sheet_names());
}

/// Recalculate `src` in headless LibreOffice; returns the recalculated copy.
fn recalc(e: &Env, soffice: &str, src: &Path) -> PathBuf {
    let profile = e.path("lo-profile");
    std::fs::create_dir_all(profile.join("user")).unwrap();
    std::fs::write(
        profile.join("user/registrymodifications.xcu"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<oor:items xmlns:oor="http://openoffice.org/2001/registry" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
<item oor:path="/org.openoffice.Office.Calc/Formula/Load"><prop oor:name="OOXMLRecalcMode" oor:op="fuse"><value>0</value></prop></item>
</oor:items>"#,
    )
    .unwrap();
    let outdir = e.path("recalculated");
    let res = Command::new(soffice)
        .arg(format!("-env:UserInstallation=file://{}", profile.display()))
        .args(["--headless", "--convert-to", "xlsx", "--outdir"])
        .arg(&outdir)
        .arg(src)
        .output()
        .unwrap();
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    outdir.join(src.file_name().unwrap())
}

#[test]
fn recalculated_values_match_the_synthetic_draws() {
    let Some(soffice) = soffice() else {
        skip("recalculated_values_match_the_synthetic_draws", "soffice (LibreOffice) not installed");
        return;
    };
    let e = Env::new();
    let mut wb = open(&recalc(&e, &soffice, &generate(&e, &[])));
    let mut errors = Vec::new();
    for s in LAB_SHEETS {
        for (row, col, v) in wb.worksheet_range(s).unwrap().cells() {
            if let Data::Error(err) = v {
                errors.push(format!("{s}!R{}C{}: {err:?}", row + 1, col + 1));
            }
        }
    }
    assert!(errors.is_empty(), "formula errors: {errors:?}");

    // Lab_Stats, ldl-c (row 6): 131 then 96.
    let s = wb.worksheet_range("Lab_Stats").unwrap();
    assert_eq!((num(&s, 5, 4), num(&s, 5, 8), num(&s, 5, 9)), (2.0, 96.0, 113.5));
    assert!((num(&s, 5, 10) - 35.0 / 2f64.sqrt()).abs() < 1e-9, "sample SD");
    assert_eq!(text(&s, 5, 15), "", "two draws are below Lab_Min_n_trend");
    let status: Vec<String> = (4..9).map(|r| text(&s, r, 18)).collect();
    assert_eq!(status, ["OK", "OK", "OK", "OK", "High"], "hba1c 5.7 > 5.6");

    // Lab_Change, ldl-c (row 7): -35, -26.7 %; RCV blank without CVs.
    let c = wb.worksheet_range("Lab_Change").unwrap();
    assert_eq!(num(&c, 6, 9), -35.0);
    assert!((num(&c, 6, 10) + 35.0 / 131.0).abs() < 1e-9);
    assert_eq!((text(&c, 6, 13), text(&c, 6, 14)), (String::new(), String::new()));

    // Lab_Quilt: ldl-c 2023 outside reference (2), 2024 within (1); hba1c 2023 outside.
    let q = wb.worksheet_range("Lab_Quilt").unwrap();
    assert_eq!((num(&q, 6, 7), num(&q, 6, 8)), (2.0, 1.0));
    assert_eq!(num(&q, 9, 7), 2.0);
    assert_eq!(text(&q, 9, 8), "", "no hba1c draw on the second date");

    // Lab_Tear defaults to the first marker (wbc, one draw).
    let t = wb.worksheet_range("Lab_Tear").unwrap();
    assert_eq!((text(&t, 3, 1), num(&t, 8, 1), num(&t, 12, 1)), ("wbc".into(), 1.0, 5.6));
    assert_eq!((num(&t, 29, 2), num(&t, 29, 13), num(&t, 29, 14)), (5.6, 5.6, 5.6), "value, EWMA, Kalman");
    assert_eq!(text(&t, 30, 2), "", "rows past the marker's draws are blank");

    // Headline KPIs.
    let k = wb.worksheet_range("Ref_KPI").unwrap();
    let kpi = |name: &str| {
        let i = (1..k.height() as u32).find(|&i| text(&k, i, 0) == name).unwrap_or_else(|| panic!("{name}"));
        k.get_value((i, 3)).cloned().unwrap()
    };
    assert_eq!(kpi("K_LAB_n_markers"), Data::Float(5.0));
    assert_eq!(kpi("K_LAB_n_out"), Data::Float(1.0));
    assert_eq!(kpi("K_LAB_n_draw_dates"), Data::Float(2.0));
    assert_eq!(kpi("K_LAB_n_critical"), Data::Float(0.0));
}
