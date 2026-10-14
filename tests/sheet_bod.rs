//! W5 body / nutrition / sleep views (`Bod_*`, BOD-01..30): structure, coverage,
//! parameters, KPIs, real formulas per BUILDABLE-NOW idea, and numbers after a full
//! LibreOffice recalculation of a workbook built from a synthetic 120-day weight and
//! intake log (`sheet_bod/fixture.rs`). Every value is made up.

#![cfg(unix)]

#[path = "sheet_bod/fixture.rs"]
mod fixture;
#[path = "sheet_bod/lane_a.rs"]
mod lane_a;
#[path = "sheet_bod/lane_b.rs"]
mod lane_b;
#[path = "sheet_bod/lane_c.rs"]
mod lane_c;
#[path = "sheet_bod/lane_d.rs"]
mod lane_d;
mod support;

use std::path::Path;

use calamine::Reader;
use fixture::*;
use health_cli::workbook::sheets::bod::COVERAGE;
use tempfile::TempDir;

#[test]
fn every_bod_idea_is_covered_once_in_order_and_documented() {
    let ids: Vec<&str> = COVERAGE.iter().map(|c| c.0).collect();
    let want: Vec<String> = (1..=30).map(|i| format!("BOD-{i:02}")).collect();
    assert_eq!(ids, want);
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let doc = std::fs::read_to_string(root.join("docs/ideas/coverage-bod.md")).unwrap();
    for (id, sheet, status, _) in COVERAGE {
        assert!(doc.contains(&format!("| {id} | {sheet} | {status} |")), "coverage-bod.md lacks the row for {id}");
    }
    let master = std::fs::read_to_string(root.join("docs/ideas/MASTER.md")).unwrap();
    for id in &want {
        assert!(master.contains(&format!("| {id} |")), "{id} not in MASTER.md");
    }
}

#[test]
fn weight_dashboard_recalculates_to_the_reference_ema() {
    let dir = TempDir::new().unwrap();
    let Some(mut wb) = recalculated(dir.path()) else { return };
    let errs = errors(&mut wb);
    assert!(errs.is_empty(), "formula errors after recalculation: {errs:?}");
    let cal = wb.worksheet_range("Bod_Cal").unwrap();
    assert_eq!(num(&at(&cal, "date", "weight_ff", DAYS as u32 - 1)), weight(DAYS - 1).unwrap());
    let w = wb.worksheet_range("Bod_Weight").unwrap();
    let want = ema(7.0);
    for i in [0u32, 5, 13, 52, 80, 119] {
        close(num(&at(&w, "date", "ema", i)), want[i as usize].unwrap(), &format!("EMA day {i}"));
    }
    close(num(&right_of(&w, "EMA weight (lb)")), want[DAYS as usize - 1].unwrap(), "EMA summary");
}

const ALL_KPIS: &[&str] = &[
    "weight_ema",
    "rate_wk",
    "band_streak",
    "slope_lb_wk",
    "cusum_alarms",
    "bmi",
    "protein_days_pct",
    "tdee",
    "sleep_h",
    "caffeine_bed",
];

#[test]
fn sheets_are_in_order_and_announce_id_status_disclaimer_and_banner() {
    let dir = TempDir::new().unwrap();
    let p = build(dir.path());
    let mut wb = open(&p);
    let bod: Vec<String> = wb.sheet_names().into_iter().filter(|n| n.starts_with("Bod_")).collect();
    let mut want: Vec<&str> = COVERAGE.iter().map(|c| c.1).collect();
    want.push("Bod_Cal");
    assert_eq!(bod, want, "tab order: BOD-01..30 then the calendar helper");
    for (id, sheet, status, _) in COVERAGE {
        let s = sheet_text(&mut wb, sheet);
        assert!(s.contains(&format!("{id} {status}")), "{sheet} does not announce {id} {status}");
        assert!(s.contains("Visualisation only - not medical advice"), "{sheet}: disclaimer");
        assert_eq!(
            s.contains("PLACEHOLDER - awaiting data"),
            *status == "PLACEHOLDER",
            "{sheet}: an awaiting banner exactly on placeholders"
        );
        for verdict in ["you have", "diagnosed with", "You have"] {
            assert!(!s.contains(verdict), "{sheet} says '{verdict}'");
        }
    }
}

#[test]
fn every_view_is_formulas_over_the_data_layer_with_tables_cf_and_charts() {
    let dir = TempDir::new().unwrap();
    let p = build(dir.path());
    let mut wb = open(&p);
    let names = wb.sheet_names();
    for (id, sheet, status, _) in COVERAGE {
        let f = formulas(&mut wb, sheet);
        let n = f.lines().count();
        let up = f.to_uppercase();
        for banned in ["DUMMYFUNCTION", "__XLUDF", "FILTER(", "QUERY(", "ARRAYFORMULA(", "IMPORTRANGE("] {
            assert!(!up.contains(banned), "{sheet}: {banned}");
        }
        // A buildable view computes from the shared data layer; a placeholder from its typed log.
        let min = if *status == "PLACEHOLDER" { 20 } else { 50 };
        assert!(n >= min, "{id} {sheet}: only {n} formulas");
        if *status == "BUILDABLE-NOW" {
            assert!(
                ["Bod_Cal!", "Data_Daily!", "Data_Nutrition!", "Data_Sizes!", "Data_Labs!"]
                    .iter()
                    .any(|t| f.contains(t)),
                "{id} {sheet}: reads no data table"
            );
        }
        let xml = sheet_xml(&p, &names, sheet);
        assert!(xml.contains("<tableParts"), "{sheet}: an Excel table");
        assert!(xml.contains("<conditionalFormatting"), "{sheet}: conditional formats");
        assert!(xml.contains("<drawing "), "{sheet}: a chart");
    }
    let cal = formulas(&mut wb, "Bod_Cal");
    assert!(cal.contains("AVERAGEIFS(Data_Daily!$B$2:$B$") && cal.contains("Bod_lb_per_kg"));
    let w = formulas(&mut wb, "Bod_Weight");
    assert!(w.contains("Bod_EMA_span") && w.contains("Protein_Mid_g_kg") && w.contains("Bod_Cal!N"));
}

#[test]
fn kpis_and_parameters_are_registered_and_honest() {
    let dir = TempDir::new().unwrap();
    let p = build(dir.path());
    let wbx = part(&p, "xl/workbook.xml");
    for k in ALL_KPIS {
        assert!(wbx.contains(&format!("definedName name=\"K_BOD_{k}\"")), "K_BOD_{k} not defined");
    }
    for n in health_cli::workbook::sheets::bod::params::names() {
        assert!(wbx.contains(&format!("definedName name=\"{n}\"")), "{n} not defined");
    }
    let mut wb = open(&p);
    let k = wb.worksheet_range("Ref_KPI").unwrap();
    let n_bod = (1..k.height() as u32).filter(|i| text(&k, *i, 1) == "BOD").count();
    assert_eq!(n_bod, ALL_KPIS.len(), "every BOD KPI is listed on Ref_KPI");
    let r = wb.worksheet_range("Params").unwrap();
    let row = |name: &str| -> Vec<String> {
        let i = (1..r.height() as u32).find(|i| text(&r, *i, 0) == name).unwrap_or_else(|| panic!("no param {name}"));
        (0..6).map(|c| text(&r, i, c)).collect()
    };
    let cut = row("Bod_Cut_hi_pct");
    assert_eq!((cut[1].as_str(), cut[3].as_str()), ("1", "verified"));
    assert!(cut[4].contains("HELMS ch01"));
    for n in ["Bod_ACWR_lo", "Bod_Waist_m_cm", "Bod_eGFR_kidney", "Bod_Protein_kidney_g_kg", "Bod_STOPBANG_flag"] {
        let x = row(n);
        assert_eq!(x[3], "UNVERIFIED", "{n}");
        assert!(x[4].starts_with("pending: "), "{n}: {}", x[4]);
    }
    for n in ["Bod_Bedtime", "Bod_Goal_lb", "Bod_Kcal_target"] {
        assert_eq!(row(n)[3], "input", "{n}");
    }
}

#[test]
fn an_empty_store_still_builds_every_bod_view() {
    // Only the BOD views (and what they require): other workstreams own their empty-store behaviour.
    let ids: Vec<String> = health_cli::workbook::sheets::bod::modules().iter().map(|m| m.id().to_string()).collect();
    let mut wb = health_cli::workbook::build(&health_cli::workbook::Data::default(), &ids).expect("builds");
    let dir = TempDir::new().unwrap();
    let p = dir.path().join("empty.xlsx");
    wb.save(&p).unwrap();
    let names = open(&p).sheet_names();
    for (_, sheet, _, _) in COVERAGE {
        assert!(names.contains(&sheet.to_string()), "{sheet} without any data");
    }
    if let Some(so) = soffice() {
        let mut r = recalc(&so, dir.path(), &p);
        let errs = errors(&mut r);
        assert!(errs.is_empty(), "formula errors on an empty store: {errs:?}");
    } else {
        skip("soffice (LibreOffice) not installed: empty-store recalculation not checked");
    }
}

#[test]
fn only_builds_a_single_bod_sheet_with_its_calendar() {
    let mut wb = health_cli::workbook::build(&fixture(), &["bod-weight".to_string()]).expect("builds");
    let dir = TempDir::new().unwrap();
    let p = dir.path().join("only.xlsx");
    wb.save(&p).unwrap();
    let names = open(&p).sheet_names();
    for s in ["Bod_Weight", "Bod_Cal", "Data_Daily", "Params"] {
        assert!(names.contains(&s.to_string()), "{s} with --only bod-weight: {names:?}");
    }
    assert!(!names.contains(&"Bod_Rate".to_string()));
    assert!(part(&p, "xl/workbook.xml").contains("definedName name=\"Bod_EMA_span\""));
}
