//! Workstream W8 (GLB): the global sheets of a generated workbook (Nav, Dash,
//! Alerts, Changes, Scenario, YearReview, Checks, Export, Sources and the hidden
//! Dash_Trend), built from the synthetic store and the synthetic `biomarker` /
//! `genome` stand-ins, then read back (values, formulas, defined names, XML parts).
//! The recalculation tests (`sheet_glb/recalc.rs`) need `soffice` and are skipped,
//! and logged to `$TEST_SKIP_LOG`, without it.

#![cfg(unix)]

#[path = "sheet_glb/recalc.rs"]
mod recalc;
mod support;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use support::Env;

/// The GLB sheets, in tab order, at the front of the workbook.
pub const GLB_SHEETS: &[&str] =
    &["Nav", "Dash", "Alerts", "Changes", "Scenario", "YearReview", "Checks", "Export", "Sources", "Dash_Trend"];

pub fn generate(e: &Env) -> PathBuf {
    e.write_stubs();
    e.import();
    let out = e.path("Health Dashboard (generated).xlsx");
    e.run(&["sheet", "--out", out.to_str().unwrap()]);
    out
}

pub type Book = Xlsx<std::io::BufReader<std::fs::File>>;

pub fn open(p: &Path) -> Book {
    open_workbook(p).expect("generated workbook opens")
}

pub fn t(r: &Range<Data>, row: u32, col: u32) -> String {
    r.get_value((row, col)).map(ToString::to_string).unwrap_or_default()
}

/// Zero-based row whose column `col` reads `key`.
pub fn row_of(r: &Range<Data>, col: u32, key: &str) -> u32 {
    (0..r.height() as u32).find(|i| t(r, *i, col) == key).unwrap_or_else(|| panic!("no row {key}"))
}

/// The cell right of the first cell reading `label`.
pub fn right_of(r: &Range<Data>, label: &str) -> String {
    let (row, col, _) = r
        .cells()
        .find(|(_, _, v)| matches!(v, Data::String(s) if s == label))
        .unwrap_or_else(|| panic!("no label {label}"));
    t(r, row as u32, col as u32 + 1)
}

pub fn part(p: &Path, name: &str) -> String {
    let mut z = zip::ZipArchive::new(std::fs::File::open(p).unwrap()).unwrap();
    let mut s = String::new();
    z.by_name(name).unwrap().read_to_string(&mut s).unwrap();
    s
}

/// Worksheet part of a sheet by name (workbook order = sheetN.xml order).
pub fn sheet_part(p: &Path, name: &str) -> String {
    let wbx = part(p, "xl/workbook.xml");
    let names: Vec<String> =
        wbx.split("<sheet name=\"").skip(1).map(|x| x.split('"').next().unwrap().to_string()).collect();
    let i = names.iter().position(|n| n == name).unwrap_or_else(|| panic!("no sheet {name}"));
    format!("xl/worksheets/sheet{}.xml", i + 1)
}

/// Every non-empty formula of a sheet.
fn formulas(wb: &mut Book, sheet: &str) -> Vec<String> {
    wb.worksheet_formula(sheet).unwrap().cells().map(|(_, _, f)| f.clone()).filter(|f| !f.is_empty()).collect()
}

/// Record a skipped test where the harness can count it (cargo hides a passing test's output).
pub fn skip(test: &str, reason: &str) {
    eprintln!("SKIP {test}: {reason}");
    if let Some(p) = std::env::var_os("TEST_SKIP_LOG") {
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(p).unwrap();
        writeln!(f, "sheet_glb::{test}: {reason}").unwrap();
    }
}

#[test]
fn nav_opens_the_workbook_and_every_glb_sheet_follows_in_order() {
    let e = Env::new();
    let out = generate(&e);
    let wb = open(&out);
    assert_eq!(&wb.sheet_names()[..GLB_SHEETS.len()], GLB_SHEETS);
    let wbx = part(&out, "xl/workbook.xml");
    let trend = wbx.split("<sheet ").find(|s| s.contains("name=\"Dash_Trend\"")).unwrap();
    assert!(trend.contains("state=\"hidden\""), "the sparkline helper is hidden");
    for n in ["Nav_Status", "K_GLB_alerts", "K_GLB_watch", "K_GLB_critical", "K_GLB_new_alerts", "K_GLB_top_mover"] {
        assert!(wbx.contains(&format!("definedName name=\"{n}\"")), "{n} not defined");
    }
}

#[test]
fn nav_links_every_sheet_with_its_coverage_badge() {
    let e = Env::new();
    let out = generate(&e);
    let mut wb = open(&out);
    let names = wb.sheet_names();
    let nav = wb.worksheet_range("Nav").unwrap();
    let all = nav.cells().map(|(_, _, v)| v.to_string()).collect::<Vec<_>>().join("\n");
    assert!(all.contains("GLOBAL DISCLAIMER") && all.contains("not medical"), "global disclaimer");
    let head = row_of(&nav, 1, "sheet");
    let listed: Vec<String> = (head + 1..nav.height() as u32).map(|r| t(&nav, r, 1)).collect();
    for n in &names {
        assert!(listed.contains(n), "{n} is not on Nav");
    }
    assert_eq!(listed.len(), names.len(), "each sheet once");
    let badge = |s: &str| t(&nav, head + 1 + listed.iter().position(|x| x == s).unwrap() as u32, 2);
    assert_eq!(badge("Sup_Cost"), "BN");
    assert_eq!(badge("Lab_Exp"), "PH");
    assert_eq!(badge("Sup_DRI"), "NS");
    assert_eq!(badge("Cln_Lipids"), "BN+PH+NS");
    assert_eq!(badge("Ref_Vax"), "NS", "rule tables carry their owner's status");
    assert_eq!(badge("Dashboard"), "-", "legacy views have no idea status");
    let ws_of = |s: &str| t(&nav, head + 1 + listed.iter().position(|x| x == s).unwrap() as u32, 0);
    assert!(ws_of("Ref_Interact").starts_with("W3 SUP"), "{}", ws_of("Ref_Interact"));
    assert!(ws_of("Data_Labs").starts_with("Shared data layer"));
    // Internal hyperlinks: every visible sheet, none to the hidden ones.
    let xml = part(&out, &sheet_part(&out, "Nav"));
    for s in ["'Weight & Age'!A1", "Sup_Book!A1", "Checks!A1", "Data_Labs!A1"] {
        let s = s.replace('&', "&amp;").replace('\'', "&apos;");
        assert!(
            xml.contains(&format!("location=\"{s}\""))
                || xml.contains(&format!("location=\"{}\"", s.replace("&apos;", "'"))),
            "link to {s}"
        );
    }
    assert!(!xml.contains("location=\"Dash_Trend!A1\""), "hidden sheets are listed, not linked");
}

#[test]
fn the_dashboard_reads_only_kpi_and_params_names() {
    let e = Env::new();
    let out = generate(&e);
    let mut wb = open(&out);
    let fs = formulas(&mut wb, "Dash");
    assert!(fs.len() > 80, "{} formulas", fs.len());
    for f in &fs {
        assert!(!f.contains('!'), "Dash formula reads a sheet: {f}");
    }
    // Every K_* the dashboard names is a defined KPI.
    let wbx = part(&out, "xl/workbook.xml");
    for f in &fs {
        for k in f.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).filter(|w| w.starts_with("K_")) {
            assert!(wbx.contains(&format!("definedName name=\"{k}\"")), "{k} in {f}");
        }
    }
    let d = wb.worksheet_range("Dash").unwrap();
    let text = d.cells().map(|(_, _, v)| v.to_string()).collect::<Vec<_>>().join("\n");
    for tile in [
        "Health index (0-100)",
        "Markers optimal",
        "Alerts (action + critical)",
        "Supplement spend / day",
        "Weight trend (kg / week)",
        "Streak (weeks in rate band)",
        "ApoB (mg/dL)",
        "TG : HDL",
        "Glycemia (TyG index)",
        "hs-CRP (mg/L)",
        "VO2 max (ml/kg/min)",
        "Zone 2 (min / week)",
        "Strength (grip / carry / hang)",
        "Lean mass / ALMI",
        "Sleep (h / night)",
        "Body weight vs target (lb)",
    ] {
        assert!(text.contains(tile), "tile {tile}");
    }
    for ph in ["awaiting data: hs-CRP", "awaiting data: grip, carry", "awaiting data: a DEXA scan"] {
        assert!(text.contains(ph), "placeholder tile {ph}");
    }
    assert!(text.contains("K_SUP_usd_day") && text.contains("K_GLB_alerts"), "the KPI board lists every KPI");
    let xml = part(&out, &sheet_part(&out, "Dash"));
    assert!(xml.contains("sparklineGroup") && xml.contains("Dash_Trend!D"), "sparklines draw from Dash_Trend");
    assert!(xml.contains("<cfRule type=\"iconSet\""), "traffic lights (3TrafficLights1 is the default icon set)");
    assert!(xml.contains("<cfvo type=\"num\" val=\"2\"/>"), "lights switch on the 0/1/2 code");
}

#[test]
fn checks_sweep_every_sheet_and_conclude_in_one_cell() {
    let e = Env::new();
    let out = generate(&e);
    let mut wb = open(&out);
    let names = wb.sheet_names();
    let c = wb.worksheet_range("Checks").unwrap();
    let f = wb.worksheet_formula("Checks").unwrap();
    let verdict = f.get_value((3, 0)).cloned().unwrap_or_default();
    assert!(verdict.contains("ALL CHECKS PASS") && verdict.contains("\"block\""), "{verdict}");
    let ids: Vec<String> = (0..c.height() as u32).map(|r| t(&c, r, 0)).collect();
    for id in ["D01", "D02", "D03", "L02", "L03", "L04", "M01", "P01", "R01", "R03", "R05", "C01", "V03", "E01", "E02"]
    {
        assert!(ids.iter().any(|x| x == id), "check {id}");
    }
    let sh = row_of(&c, 0, "sheet");
    let swept: Vec<String> = (sh + 1..c.height() as u32).map(|r| t(&c, r, 0)).filter(|s| !s.is_empty()).collect();
    let expect: Vec<String> = names.iter().filter(|n| *n != "Checks").cloned().collect();
    assert_eq!(swept, expect, "every sheet but Checks itself is swept");
    let fo = |r: u32, col: u32| f.get_value((r, col)).cloned().unwrap_or_default();
    let bod = sh + 1 + swept.iter().position(|s| s == "Bod_Weight").unwrap() as u32;
    let rg = t(&c, bod, 1);
    assert!(rg.starts_with("Bod_Weight!$A$1:$") && rg != "Bod_Weight!$A$1:$A$1", "measured extent {rg}");
    assert_eq!(fo(bod, 2), format!("SUMPRODUCT(--ISERROR({rg}))-SUMPRODUCT(--ISNA({rg}))"));
    let wa = sh + 1 + swept.iter().position(|s| s == "Weight & Age").unwrap() as u32;
    assert!(t(&c, wa, 1).starts_with("'Weight & Age'!$A$1:"), "quoted sheet names");
    let row = |id: &str| row_of(&c, 0, id);
    assert!(fo(row("V03"), 3).contains("Nav_Status"));
    assert!(fo(row("R05"), 3).contains("K_SUP_usd_yr"), "reconciles the SUP KPI against tblSupp");
}

#[test]
fn alerts_changes_scenario_review_and_export_are_formula_views() {
    let e = Env::new();
    let mut wb = open(&generate(&e));
    let joined = |wb: &mut Book, s: &str| formulas(wb, s).join("\n").replace("_xlfn.", "").replace("_xlws.", "");
    let a = joined(&mut wb, "Alerts");
    assert!(
        a.contains("Calc_Marker!$N$") && a.contains("CHOOSE(F") && a.contains("LARGE("),
        "tiers, labels, sorted list"
    );
    assert!(a.contains("SORTBY(") && !a.to_uppercase().contains("FILTER("), "365 spill without FILTER");
    let c = joined(&mut wb, "Changes");
    for needle in
        ["Latest_Panel", "Prev_Panel", "Calc_Labs!$K$", "RCV_z*SQRT(2)", "TEXTJOIN(", "TAKE(SORTBY(", "Data_Supp!$S$"]
    {
        assert!(c.contains(needle), "Changes: {needle}");
    }
    let s = joined(&mut wb, "Scenario");
    for needle in ["Cln_TGHDL_ok", "Cln_FH_NonHDL", "Bod_BMI_over", "Height_cm", "\"triglycerides\"", "Calc_Marker!$E$"]
    {
        assert!(s.contains(needle), "Scenario: {needle}");
    }
    let y = joined(&mut wb, "YearReview");
    assert!(
        y.contains("MEDIAN(IF((YEAR(Data_Labs!$A$") && y.contains("\"apob\"") && y.contains("\"In \"&$A"),
        "{y:.300}"
    );
    let x = joined(&mut wb, "Export");
    assert!(x.contains("[redacted]") && x.contains("TEXTJOIN(CHAR(10)"), "redaction and bundle");
    let ex = wb.worksheet_range("Export").unwrap();
    let dob = row_of(&ex, 1, "date of birth");
    assert_eq!(t(&ex, dob, 6), "YES", "DOB is redacted by default");
    let sections: Vec<String> = (0..ex.height() as u32).map(|r| t(&ex, r, 0)).collect();
    for sct in ["demographics", "latest panel", "trend", "gap", "genome flag", "stack", "goal", "kpi"] {
        assert!(sections.iter().any(|x| x == sct), "Export section {sct}");
    }
    let so = wb.worksheet_range("Sources").unwrap();
    let all = so.cells().map(|(_, _, v)| v.to_string()).collect::<Vec<_>>().join("\n");
    for needle in ["biomarker-cli", "genome-cli", "Resources (tblResources)", "Sizes (tblSizes)", "EFLM", "Outlive"] {
        assert!(all.contains(needle), "Sources: {needle}");
    }
}

#[test]
fn glb_params_are_registered_and_heuristic() {
    let e = Env::new();
    let mut wb = open(&generate(&e));
    let p = wb.worksheet_range("Params").unwrap();
    for n in ["Glb_Amber_pct", "Glb_HI_w_ref", "Glb_Index_green", "Glb_Index_amber", "Glb_Weight_tol", "Glb_Top_n"] {
        let r = row_of(&p, 0, n);
        assert_eq!(t(&p, r, 3), "heuristic", "{n}");
    }
    for s in GLB_SHEETS.iter().filter(|s| **s != "Dash_Trend") {
        let r = wb.worksheet_range(s).unwrap();
        assert!(r.cells().any(|(_, _, v)| v.to_string().contains("not medical advice")), "{s}: disclaimer");
    }
}

#[test]
fn only_builds_keep_nav_and_bring_the_trend_helper_with_the_dashboard() {
    let e = Env::new();
    e.write_stubs();
    e.import();
    let out = e.path("part (generated).xlsx");
    e.run(&["sheet", "--only", "glb-dash", "--out", out.to_str().unwrap()]);
    let names = open(&out).sheet_names();
    assert_eq!(names, ["Nav", "Dash", "Dash_Trend", "Params", "Ref_KPI"]);
    let mut wb = open(&out);
    let d = wb.worksheet_range("Dash").unwrap();
    assert!(d.cells().any(|(_, _, v)| v.to_string().contains("awaiting data: ApoB")), "tiles without their KPI wait");
}

#[test]
fn without_labs_the_lab_views_drop_out_and_the_rest_still_builds() {
    let e = Env::new();
    e.import(); // no stubs: biomarker is "not installed"
    let out = e.path("no-labs (generated).xlsx");
    let res = e.cmd().args(["sheet", "--out", out.to_str().unwrap()]).output().unwrap();
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    let mut wb = open(&out);
    let names = wb.sheet_names();
    for s in ["Nav", "Dash", "YearReview", "Checks", "Export", "Sources", "Dash_Trend"] {
        assert!(names.iter().any(|n| n == s), "{s}");
    }
    for s in ["Alerts", "Changes", "Scenario"] {
        assert!(!names.iter().any(|n| n == s), "{s} needs labs");
    }
    let c = wb.worksheet_range("Checks").unwrap();
    assert_eq!(t(&c, row_of(&c, 0, "L02"), 6), "sheet not in this build");
    assert!(!formulas(&mut wb, "Checks").iter().any(|f| f.contains("Data_Labs!")), "no reference to a missing sheet");
    assert!(!formulas(&mut wb, "Dash_Trend").iter().any(|f| f.contains("Calc_Panel!")));
}

#[test]
fn coverage_doc_lists_every_glb_idea_and_sheet() {
    let doc = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/ideas/coverage-glb.md")).unwrap();
    for i in 1..=8 {
        assert!(doc.contains(&format!("| GLB-0{i} |")), "GLB-0{i}");
    }
    for s in GLB_SHEETS {
        assert!(doc.contains(s), "{s}");
    }
}
