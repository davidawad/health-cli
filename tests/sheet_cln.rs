//! W2 clinical views (`Cln_*`, CLN-01..25): structure, coverage, parameters, KPIs,
//! and numbers after a full LibreOffice recalculation of a workbook built from a
//! synthetic three-panel lab history (`sheet_cln/fixture.rs`). Every value is made up.

#![cfg(unix)]

#[path = "sheet_cln/fixture.rs"]
mod fixture;
mod support;

use std::path::Path;

use calamine::{Data as Cell, Reader};
use fixture::*;
use health_cli::infra::params::State;
use health_cli::workbook;
use health_cli::workbook::sheets::cln::{self, COVERAGE};
use tempfile::TempDir;

const CLN_SHEETS: &[&str] = &[
    "Cln_Systems",
    "Cln_RiskChecklist",
    "Cln_Lipids",
    "Cln_FH",
    "Cln_Glycemia",
    "Cln_Insulin",
    "Cln_MetSyn",
    "Cln_AnionGap",
    "Cln_Renal",
    "Cln_Liver",
    "Cln_Iron",
    "Cln_Thyroid",
    "Cln_Uric",
    "Cln_Derived",
    "Cln_NutStatus",
    "Cln_Loops",
];

// ------------------------------------------------------------------ structure

#[test]
fn every_cln_idea_is_covered_once_in_order_and_documented() {
    let ids: Vec<&str> = COVERAGE.iter().map(|c| c.0).collect();
    let want: Vec<String> = (1..=25).map(|i| format!("CLN-{i:02}")).collect();
    assert_eq!(ids, want);
    for (id, sheet, status, _) in COVERAGE {
        assert!(CLN_SHEETS.contains(sheet), "{id}: {sheet}");
        assert!(["BUILDABLE-NOW", "PLACEHOLDER", "NEEDS-SOURCE"].contains(status), "{id}: {status}");
    }
    let doc =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/ideas/coverage-cln.md")).unwrap();
    for (id, sheet, status, _) in COVERAGE {
        assert!(doc.contains(&format!("| {id} | {sheet} | {status} |")), "coverage-cln.md lacks the row for {id}");
    }
    let master = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/ideas/MASTER.md")).unwrap();
    for id in &want {
        assert!(master.contains(&format!("| {id} |")), "{id} not in MASTER.md");
    }
}

#[test]
fn sheets_follow_the_shared_layer_in_order_and_name_their_ideas() {
    let dir = TempDir::new().unwrap();
    let p = build(dir.path());
    let mut wb = open(&p);
    let names = wb.sheet_names();
    let cln: Vec<&String> = names.iter().filter(|n| n.starts_with("Cln_")).collect();
    assert_eq!(cln, CLN_SHEETS, "tab order");
    let calc = names.iter().position(|n| n == "Calc_Panel").unwrap();
    assert_eq!(names[calc + 1], "Cln_Systems", "Cln_ views follow the Calc_ layer");
    for (id, sheet, status, _) in COVERAGE {
        let s = sheet_text(&mut wb, sheet);
        assert!(s.contains(&format!("{id} {status}")), "{sheet} does not announce {id} {status}");
        assert!(s.contains("Visualisation only - not medical advice"), "{sheet}: disclaimer");
        if *status == "PLACEHOLDER" {
            assert!(s.contains("PLACEHOLDER - awaiting data"), "{sheet}: awaiting banner for {id}");
        }
    }
    // [DX-RISK] ideas speak only in the neutral pattern wording.
    for sheet in [
        "Cln_Lipids",
        "Cln_FH",
        "Cln_Glycemia",
        "Cln_AnionGap",
        "Cln_Renal",
        "Cln_Liver",
        "Cln_Iron",
        "Cln_Thyroid",
        "Cln_MetSyn",
    ] {
        let s = sheet_text(&mut wb, sheet);
        assert!(s.contains("Pattern consistent with") && s.contains("not a diagnosis"), "{sheet}");
        for verdict in ["you have", "diagnosed with", "You have"] {
            assert!(!s.contains(verdict), "{sheet} says '{verdict}'");
        }
    }
}

#[test]
fn formulas_are_excel_native_and_read_the_shared_layer() {
    let dir = TempDir::new().unwrap();
    let p = build(dir.path());
    let mut wb = open(&p);
    let mut n = 0;
    for sheet in CLN_SHEETS {
        let f = wb.worksheet_formula(sheet).unwrap();
        for (_, _, x) in f.cells() {
            let up = x.to_uppercase();
            for banned in ["DUMMYFUNCTION", "__XLUDF", "FILTER(", "QUERY(", "ARRAYFORMULA(", "IMPORTRANGE("] {
                assert!(!up.contains(banned), "{sheet}: {x}");
            }
            assert!(!x.contains('[') || x.contains("tblCln"), "{sheet}: unexpanded template {x}");
            n += 1;
        }
    }
    assert!(n > 500, "views are formulas: {n}");
    let lip = wb.worksheet_formula("Cln_Lipids").unwrap();
    let all: String = lip.cells().map(|(_, _, x)| x.clone()).collect::<Vec<_>>().join("\n");
    assert!(all.contains("Data_Labs!$C$2:$C$") && all.contains("Data_Panels!$B$"), "reads tblLabs / tblPanels");
    assert!(all.contains("Cln_Friedewald_div") && all.contains("Cln_TGHDL_ok"), "constants by name");
    let sys = wb.worksheet_formula("Cln_Systems").unwrap();
    assert!(sys.cells().any(|(_, _, x)| x.contains("Calc_Marker!") && x.contains("COUNTIFS")));
    // Tables, charts, conditional formats and sparklines are present.
    let parts: Vec<String> = {
        let z = zip::ZipArchive::new(std::fs::File::open(&p).unwrap()).unwrap();
        z.file_names().map(str::to_string).collect()
    };
    let tables: Vec<String> = parts
        .iter()
        .filter(|n| n.starts_with("xl/tables/"))
        .map(|n| part(&p, n).split("displayName=\"").nth(1).unwrap().split('"').next().unwrap().to_string())
        .collect();
    for t in [
        "tblViewClnLipids",
        "tblClnMHFactor",
        "tblViewClnExposure",
        "tblClnLdlExpected",
        "tblClnFamilyHx",
        "tblClnOGTT",
        "tblClnVitals",
        "tblClnLoopPairs",
        "tblViewClnSystems",
    ] {
        assert!(tables.iter().any(|x| x == t), "{t} missing");
    }
    assert!(parts.iter().filter(|n| n.starts_with("xl/charts/chart")).count() >= 20, "charts");
}

#[test]
fn params_and_kpis_are_registered_with_their_sources() {
    let set = cln::params::all();
    assert!(set.iter().all(|p| p.name.starts_with("Cln_")));
    for p in set.iter().filter(|p| p.state == State::Unverified) {
        assert!(p.source.starts_with("pending: "), "{}", p.name);
    }
    for p in set.iter().filter(|p| p.state == State::Verified) {
        assert!(["OUT", "DIP", "GLU", "KAT"].iter().any(|s| p.source.starts_with(s)), "{} cites {}", p.name, p.source);
    }
    for (name, state) in
        [("Cln_CKDEPI_A", State::Unverified), ("Cln_AG_hi", State::Verified), ("Cln_HOMA_div", State::Unverified)]
    {
        assert_eq!(set.iter().find(|p| p.name == name).unwrap().state, state, "{name}");
    }
    let dir = TempDir::new().unwrap();
    let p = build(dir.path());
    let wbx = part(&p, "xl/workbook.xml");
    for k in
        ["tg_hdl", "apob", "apob_years", "priority_tested", "tyg", "metsyn", "anion_gap", "egfr", "fib4", "pct_optimal"]
    {
        assert!(wbx.contains(&format!("definedName name=\"K_CLN_{k}\"")), "K_CLN_{k}");
    }
    for n in ["Cln_TGHDL_ok", "Cln_CKDEPI_kappa_m", "Cln_Exposure_Marker", "Cln_Loop_Pair"] {
        assert!(wbx.contains(&format!("definedName name=\"{n}\"")), "{n}");
    }
    let mut wb = open(&p);
    let r = wb.worksheet_range("Params").unwrap();
    let row = (1..r.height() as u32).find(|i| text(&r, *i, 0) == "Cln_eAG_slope").unwrap();
    assert_eq!(
        (text(&r, row, 3), text(&r, row, 4)),
        ("UNVERIFIED".into(), "pending: Nathan 2008 Diabetes Care (ADAG)".into())
    );
}

#[test]
fn a_partial_build_registers_the_params_once_and_lab_less_builds_have_no_cln_sheets() {
    let wb = workbook::build(&fixture(), &["cln-renal".into(), "cln-lipids".into()]);
    assert!(wb.is_ok(), "two Cln_ modules share the parameter set without a collision");
    let e = support::Env::new();
    e.import();
    let out = e.path("lab-less (generated).xlsx");
    e.run(&["sheet", "--out", out.to_str().unwrap()]);
    assert!(open(&out).sheet_names().iter().all(|n| !n.starts_with("Cln_")));
}

#[test]
fn the_cli_generated_workbook_round_trips_with_the_cln_sheets() {
    let e = support::Env::new();
    e.write_stubs();
    e.import();
    let out = e.path("Health Dashboard (generated).xlsx");
    e.run(&["sheet", "--out", out.to_str().unwrap()]);
    let mut wb = open(&out);
    let names: Vec<String> = wb.sheet_names().into_iter().filter(|n| n.starts_with("Cln_")).collect();
    assert_eq!(names, CLN_SHEETS);
    let r = wb.worksheet_range("Cln_Lipids").unwrap();
    assert_eq!(text(&r, 0, 0), "Lipids: ratios, LDL method cross-checks, ApoB concordance and exposure");
    let list = e.json(&["sheet", "--list"]);
    let ids: Vec<&str> = list["data"].as_array().unwrap().iter().map(|s| s["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"cln-lipids") && ids.contains(&"cln-loops"));
}

// ------------------------------------------------------------------ numbers

#[test]
fn recalculated_numbers_match_hand_computed_values() {
    let Some(so) = soffice() else {
        skip("recalculated_numbers_match_hand_computed_values: soffice not installed");
        return;
    };
    let dir = TempDir::new().unwrap();
    let p = build(dir.path());
    let mut wb = recalc(&so, dir.path(), &p);
    let mut errors = Vec::new();
    for name in wb.sheet_names() {
        let r = wb.worksheet_range(&name).unwrap();
        for (row, col, v) in r.cells() {
            if let Cell::Error(e) = v {
                errors.push(format!("{name}!R{}C{}: {e:?}", row + 1, col + 1));
            }
        }
    }
    assert!(errors.is_empty(), "formula errors: {errors:?}");

    let age = |d: &str| (day(d) - day(DOB)).num_days() as f64 / 365.25;
    let years = |a: &str, b: &str| (day(b) - day(a)).num_days() as f64 / 365.25;

    let l = wb.worksheet_range("Cln_Lipids").unwrap();
    let v = |h: &str, i: u32| num(&at(&l, "Date", h, i));
    close(v("TG/HDL", 0), 3.0, "TG/HDL");
    close(v("TG/HDL", 2), 80.0 / 60.0, "TG/HDL latest");
    close(v("non-HDL", 0), 170.0, "non-HDL");
    close(v("TC:HDL diff", 0), 0.0, "TC:HDL vs reported");
    close(v("Remnant", 0), 30.0, "remnant");
    close(v("Friedewald LDL", 0), 140.0, "Friedewald");
    close(v("VLDL diff", 0), 0.0, "VLDL vs reported");
    close(v("ApoB fit", 1), 95.0, "ApoB = 0.75 LDL + 5 exactly");
    assert_eq!(at(&l, "Date", "Martin-Hopkins LDL", 0).to_string(), "", "no factor table yet");
    assert_eq!(at(&l, "Date", "FH pattern note", 0).to_string(), "");
    let s = (0..l.height() as u32).find(|i| text(&l, *i, 0) == "slope").unwrap();
    close(num(&l.get_value((s, 1)).unwrap().clone()), 0.75, "slope");
    close(num(&l.get_value((s + 2, 1)).unwrap().clone()), 1.0, "R squared");
    let e1 = years(P[0], P[1]) * ((110.0 + 95.0) / 2.0 - 25.0);
    let e2 = years(P[1], P[2]) * ((95.0 + 80.0) / 2.0 - 25.0);
    // The exposure table is the second one whose first header is "Date": find it by "Cumulative".
    let (hr, c) = (0..l.height() as u32)
        .find_map(|i| (0..l.width() as u32).find(|c| text(&l, i, *c) == "Cumulative").map(|c| (i, c)))
        .unwrap();
    close(num(&l.get_value((hr + 3, c)).unwrap().clone()), e1 + e2, "ApoB-years");

    let k = wb.worksheet_range("Ref_KPI").unwrap();
    let kpi = |name: &str| {
        let r = (1..k.height() as u32).find(|i| text(&k, *i, 0) == name).unwrap_or_else(|| panic!("{name}"));
        k.get_value((r, 3)).cloned().unwrap()
    };
    close(num(&kpi("K_CLN_tg_hdl")), 80.0 / 60.0, "K_CLN_tg_hdl");
    close(num(&kpi("K_CLN_apob")), 80.0, "K_CLN_apob");
    close(num(&kpi("K_CLN_apob_years")), e1 + e2, "K_CLN_apob_years");
    close(num(&kpi("K_CLN_priority_tested")), 5.0, "apob, TG/HDL, insulin, uric acid, ALT");
    close(num(&kpi("K_CLN_anion_gap")), 13.0, "K_CLN_anion_gap");
    close(num(&kpi("K_CLN_metsyn")), 1.0, "glucose 130 > 110 on the latest panel");
    let pct = num(&kpi("K_CLN_pct_optimal"));
    assert!((0.0..=1.0).contains(&pct), "{pct}");

    let g = wb.worksheet_range("Cln_Glycemia").unwrap();
    close(num(&at(&g, "Date", "eAG used", 0)), 108.0, "reported eAG is primary");
    close(num(&at(&g, "Date", "eAG used", 1)), 28.7 * 5.3 - 46.7, "computed eAG fills the gap");
    assert_eq!(at(&g, "Date", "Glucose band", 2).to_string(), ">= 126");
    assert!(at(&g, "Date", "Pattern", 2).to_string().starts_with("Pattern consistent with"));

    let ins = wb.worksheet_range("Cln_Insulin").unwrap();
    close(num(&at(&ins, "Date", "TyG", 0)), (150.0f64 * 92.0 / 2.0).ln(), "TyG");
    close(num(&at(&ins, "Date", "HOMA-IR", 1)), 6.0 * 88.0 / 405.0, "HOMA-IR");
    close(num(&at(&ins, "Date", "QUICKI", 1)), 1.0 / (6.0f64.log10() + 88.0f64.log10()), "QUICKI");

    let a = wb.worksheet_range("Cln_AnionGap").unwrap();
    close(num(&at(&a, "Date", "AG computed", 0)), 13.0, "AG");
    close(num(&at(&a, "Date", "AG diff", 0)), 0.0, "AG vs reported");
    assert_eq!(at(&a, "Date", "Band", 0).to_string(), "12-16");
    close(num(&at(&a, "Date", "AG albumin-adjusted", 0)), 13.0 + 2.5 * (4.4 - 4.5), "albumin-adjusted AG");

    let r = wb.worksheet_range("Cln_Renal").unwrap();
    let a0 = age(P[0]);
    let x = 1.0f64 / 0.9;
    let male = 142.0 * x.min(1.0).powf(-0.302) * x.max(1.0).powf(-1.2) * 0.9938f64.powf(a0);
    let xf = 1.0f64 / 0.7;
    let female = 142.0 * xf.min(1.0).powf(-0.241) * xf.max(1.0).powf(-1.2) * 0.9938f64.powf(a0) * 1.012;
    close(num(&at(&r, "Date", "eGFR if male", 0)), male, "CKD-EPI 2021 male");
    close(num(&at(&r, "Date", "eGFR if female", 0)), female, "CKD-EPI 2021 female");
    assert_eq!(at(&r, "Date", "eGFR CKD-EPI 2021", 0).to_string(), "", "Sex not set");
    assert_eq!(at(&r, "Date", "Creatine in current stack", 0).to_string(), "YES (dates unknown)");
    assert_eq!(at(&r, "Date", "Creatine before draw", 0).to_string(), "", "GATE-DATES");
    close(num(&kpi("K_CLN_egfr")), 95.0, "falls back to the reported eGFR");

    let lv = wb.worksheet_range("Cln_Liver").unwrap();
    close(num(&at(&lv, "Date", "FIB-4", 0)), a0 * 25.0 / (250.0 * 30.0f64.sqrt()), "FIB-4");
    close(num(&at(&lv, "Date", "De Ritis AST/ALT", 0)), 25.0 / 30.0, "De Ritis");
    close(num(&kpi("K_CLN_fib4")), age(P[1]) * 24.0 / (240.0 * 28.0f64.sqrt()), "K_CLN_fib4");

    let dv = wb.worksheet_range("Cln_Derived").unwrap();
    close(num(&at(&dv, "Date", "NLR", 0)), 1.5, "NLR");
    close(num(&at(&dv, "Date", "PLR", 0)), 125.0, "PLR");
    assert!(at(&dv, "Date", "TLC note", 1).to_string().starts_with("below 1,200"));
    close(num(&at(&dv, "Date", "BUN:Cr diff", 0)), 0.0, "BUN:Cr");
    close(num(&at(&dv, "Date", "A/G diff", 0)), 0.0, "A/G");
    close(num(&at(&dv, "Date", "Corrected Ca", 0)), 9.2, "corrected calcium");
    close(num(&at(&dv, "Date", "Mentzer", 0)), 18.0, "Mentzer");

    let ms = wb.worksheet_range("Cln_MetSyn").unwrap();
    close(num(&at(&ms, "Date", "Met", 2)), 1.0, "glucose criterion");
    close(num(&at(&ms, "Date", "Assessed", 2)), 2.0, "TG and glucose (HDL / waist need Sex, BP needs vitals)");
    close(num(&at(&ms, "Date", "BMI", 0)), 190.0 * 0.45359237 / (1.778f64 * 1.778), "BMI from weight at panel");

    let t = wb.worksheet_range("Cln_Thyroid").unwrap();
    assert_eq!(at(&t, "Date", "TSH position", 0).to_string(), "in reference");
    let ts = sheet_text(&mut wb, "Cln_Thyroid");
    assert!(ts.contains("hold biotin 24-48 h"), "10,000 mcg biotin in the current stack");

    let c = wb.worksheet_range("Cln_RiskChecklist").unwrap();
    let tile = |item: &str| {
        let r = (0..c.height() as u32).find(|i| text(&c, *i, 0).starts_with(item)).unwrap();
        text(&c, r, 7)
    };
    assert_eq!(tile("Lp(a)"), "NOT TESTED");
    assert_eq!(tile("Homocysteine"), "NOT TESTED");
    assert!(tile("ApoB").starts_with("measured") || tile("ApoB").starts_with("stale"));

    let sy = wb.worksheet_range("Cln_Systems").unwrap();
    let lip = (0..sy.height() as u32).find(|i| text(&sy, *i, 0) == "Lipids").unwrap();
    close(num(&sy.get_value((lip, 1)).unwrap().clone()), 7.0, "seven lipid markers");
    close(num(&sy.get_value((lip, 2)).unwrap().clone()), 7.0, "all measured");

    let ns = wb.worksheet_range("Cln_NutStatus").unwrap();
    let tlc = (0..ns.height() as u32).find(|i| text(&ns, *i, 0) == "Total lymphocyte count").unwrap();
    close(num(&ns.get_value((tlc, 4)).unwrap().clone()), 1000.0, "TLC in cells/mm3");
    assert_eq!(text(&ns, tlc + 1, 6), "NOT TESTED", "prealbumin");

    let u = wb.worksheet_range("Cln_Uric").unwrap();
    assert_eq!(at(&u, "Date", "Position", 2).to_string(), "above solubility");
}
