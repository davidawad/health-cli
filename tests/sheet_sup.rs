//! Workstream W3 (SUP): the `Sup_*` views and the `Ref_Interact` / `Ref_GRADE` /
//! `Ref_UL` rule tables of a generated workbook, built from the synthetic fixtures
//! (`tests/support`). The last test recalculates in headless LibreOffice and checks
//! numbers; it is skipped (and logged to `$TEST_SKIP_LOG`) without `soffice`.

#![cfg(unix)]

mod support;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use support::soffice::soffice;
use support::Env;

const SUP_SHEETS: &[&str] = &[
    "Sup_Book",
    "Sup_Cost",
    "Sup_Unit",
    "Sup_Churn",
    "Sup_Mech",
    "Sup_DropOne",
    "Sup_Value",
    "Sup_Overlap",
    "Sup_DRI",
    "Sup_Audit",
    "Sup_Interact",
    "Sup_Regimen",
    "Sup_LabDrug",
    "Sup_PK",
    "Sup_Cycle",
    "Sup_Grade",
    "Sup_Papers",
    "Sup_Calc",
    "Sup_Balance",
    "Sup_Forest",
    "Sup_Bubble",
    "Sup_TrialDose",
    "Ref_Interact",
    "Ref_GRADE",
    "Ref_UL",
];

/// Views over empty inputs (PLACEHOLDER) and views with UNVERIFIED constants (NEEDS-SOURCE).
const PLACEHOLDERS: &[&str] =
    &["Sup_Unit", "Sup_Churn", "Sup_Mech", "Sup_DropOne", "Sup_Value", "Sup_Overlap", "Sup_Cycle", "Sup_Regimen"];
const UNVERIFIED: &[&str] = &["Sup_DRI", "Sup_Interact", "Sup_LabDrug", "Sup_PK"];

fn generate(e: &Env) -> PathBuf {
    e.write_stubs();
    e.import();
    let out = e.path("Health Dashboard (generated).xlsx");
    e.run(&["sheet", "--out", out.to_str().unwrap()]);
    out
}

fn open(p: &Path) -> Xlsx<std::io::BufReader<std::fs::File>> {
    open_workbook(p).unwrap()
}

fn t(r: &Range<Data>, row: u32, col: u32) -> String {
    r.get_value((row, col)).map(ToString::to_string).unwrap_or_default()
}

fn num(r: &Range<Data>, row: u32, col: u32) -> f64 {
    match r.get_value((row, col)) {
        Some(Data::Float(f)) => *f,
        Some(Data::Int(i)) => *i as f64,
        other => panic!("({row},{col}) is {other:?}"),
    }
}

/// Zero-based column of `name` in header row `row`.
fn col_of(r: &Range<Data>, row: u32, name: &str) -> u32 {
    (0..r.width() as u32).find(|c| t(r, row, *c) == name).unwrap_or_else(|| panic!("no column {name}"))
}

/// Zero-based row whose column `col` reads `key`.
fn row_of(r: &Range<Data>, col: u32, key: &str) -> u32 {
    (0..r.height() as u32).find(|i| t(r, *i, col) == key).unwrap_or_else(|| panic!("no row {key}"))
}

fn part(p: &Path, name: &str) -> String {
    let mut z = zip::ZipArchive::new(std::fs::File::open(p).unwrap()).unwrap();
    let mut s = String::new();
    z.by_name(name).unwrap().read_to_string(&mut s).unwrap();
    s
}

/// Record a skipped test where the harness can count it (cargo hides a passing test's output).
fn skip(test: &str, reason: &str) {
    eprintln!("SKIP {test}: {reason}");
    if let Some(p) = std::env::var_os("TEST_SKIP_LOG") {
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(p).unwrap();
        writeln!(f, "sheet_sup::{test}: {reason}").unwrap();
    }
}

#[test]
fn every_sup_view_and_rule_table_is_generated_in_order() {
    let e = Env::new();
    let wb = open(&generate(&e));
    let names: Vec<String> = wb.sheet_names().into_iter().filter(|n| SUP_SHEETS.contains(&n.as_str())).collect();
    assert_eq!(names, SUP_SHEETS);
}

#[test]
fn the_book_is_row_aligned_with_tblsupp_and_derives_its_numbers() {
    let e = Env::new();
    let mut wb = open(&generate(&e));
    let b = wb.worksheet_range("Sup_Book").unwrap();
    let f = wb.worksheet_formula("Sup_Book").unwrap();
    // Header on row 5, Alphazyme (first tblSupp row) on row 6.
    assert_eq!(t(&b, 0, 0), "Position book: every supplement, its cost and its standing  (SUP-01)");
    let fo = |c: &str| f.get_value((5, col_of(&b, 4, c))).cloned().unwrap_or_default();
    assert_eq!(fo("name"), "IF(Data_Supp!$B$2=\"\",\"\",Data_Supp!$B$2)");
    assert_eq!(fo("usd_yr"), "IF(AND(ISNUMBER(L6),ISNUMBER(M6)),L6*M6,0)");
    assert!(fo("doses_per_wk").contains("MATCH(E6,Lists!$A$2:$A$9,0)"), "{}", fo("doses_per_wk"));
    assert!(fo("stance_net").contains("Data_Evidence!"), "{}", fo("stance_net"));
}

#[test]
fn kpis_params_and_names_are_registered() {
    let e = Env::new();
    let out = generate(&e);
    let wbx = part(&out, "xl/workbook.xml");
    for n in [
        "K_SUP_n_active",
        "K_SUP_usd_yr",
        "K_SUP_usd_month",
        "K_SUP_usd_day",
        "K_SUP_ter",
        "K_SUP_hhi",
        "K_SUP_usd_no_evidence",
        "K_SUP_n_interactions",
        "K_SUP_n_audit_red",
        "K_SUP_usd_low_certainty",
        "Sup_Mech_Filter",
        "Sup_Budget_yr",
        "Sup_SS_HalfLives",
        "Sup_ALT_ULN_x",
    ] {
        assert!(wbx.contains(&format!("definedName name=\"{n}\"")), "{n} not defined");
    }
    let mut wb = open(&out);
    let p = wb.worksheet_range("Params").unwrap();
    let state = |name: &str| {
        let r = row_of(&p, 0, name);
        (t(&p, r, 3), t(&p, r, 4))
    };
    assert_eq!(state("Sup_ALT_ULN_x"), ("UNVERIFIED".into(), "pending: KAT ch46.md:201 (MED-VERIFY)".into()));
    assert_eq!(state("Sup_SS_HalfLives"), ("verified".into(), "KAT ch09.md:107-111".into()));
    assert_eq!(state("Sup_Budget_yr").0, "input");
}

#[test]
fn rule_tables_follow_the_shared_schema_and_label_unverified_rows() {
    let e = Env::new();
    let mut wb = open(&generate(&e));
    let schema = ["id", "rule", "value", "unit", "source", "source_state", "note"];
    for sheet in ["Ref_Interact", "Ref_GRADE", "Ref_UL"] {
        let r = wb.worksheet_range(sheet).unwrap();
        assert_eq!((0..7).map(|c| t(&r, 0, c)).collect::<Vec<_>>(), schema, "{sheet}");
        assert!(r.height() > 5, "{sheet} is seeded");
    }
    let ul = wb.worksheet_range("Ref_UL").unwrap();
    for i in 1..ul.height() as u32 {
        assert_eq!(t(&ul, i, 5), "UNVERIFIED");
        assert!(t(&ul, i, 4).starts_with("pending: "), "{}", t(&ul, i, 4));
    }
    let ix = wb.worksheet_range("Ref_Interact").unwrap();
    let sjw = row_of(&ix, 0, "INT-01");
    assert_eq!((t(&ix, sjw, 1).as_str(), t(&ix, sjw, 3).as_str()), ("st john", "warfarin"));
}

#[test]
fn placeholders_and_needs_source_views_say_so() {
    let e = Env::new();
    let mut wb = open(&generate(&e));
    let has = |r: &Range<Data>, needle: &str| r.cells().any(|(_, _, v)| v.to_string().contains(needle));
    for s in PLACEHOLDERS {
        let r = wb.worksheet_range(s).unwrap();
        assert!(has(&r, "PLACEHOLDER - awaiting data"), "{s}");
    }
    for s in UNVERIFIED {
        let r = wb.worksheet_range(s).unwrap();
        assert!(has(&r, "UNVERIFIED - pending"), "{s}");
    }
    for s in SUP_SHEETS.iter().filter(|s| s.starts_with("Sup_")) {
        let r = wb.worksheet_range(s).unwrap();
        assert!(has(&r, "not medical advice"), "{s}: disclaimer");
    }
}

#[test]
fn charts_validation_and_conditional_formats_are_present() {
    let e = Env::new();
    let out = generate(&e);
    let mut z = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
    let charts = (0..z.len()).filter(|i| z.by_index(*i).unwrap().name().starts_with("xl/charts/chart")).count();
    assert!(charts >= 12, "{charts} charts");
    // Sup_Forest's error bars and Sup_Mech's radar.
    let all: String = (0..z.len())
        .filter_map(|i| {
            let mut f = z.by_index(i).ok()?;
            let mut s = String::new();
            f.name().starts_with("xl/charts/").then(|| f.read_to_string(&mut s).ok()).flatten()?;
            Some(s)
        })
        .collect();
    assert!(all.contains("<c:errBars>") && all.contains("<c:radarChart>"));
}

// ------------------------------------------------------------ recalculation

fn recalc(soffice: &str, dir: &Path, input: &Path) -> Xlsx<std::io::BufReader<std::fs::File>> {
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

/// Synthetic stack (tests/support): Alphazyme YES daily 600 mg, 3 x $32 = $96/yr,
/// protocols "Test Protocol, Box One", one research URL (neutral); Betazine NO (Rx);
/// Gammavit YES 2x/wk, 2 x $25 = $50, "box one, Test Protocol"; Deltamin YES 3/wk,
/// 6 x $13.5 = $81, no protocol; Epsilon Oil taking unset.
#[test]
fn recalculated_sup_views_have_the_expected_numbers() {
    let Some(soffice) = soffice() else {
        skip("recalculated_sup_views_have_the_expected_numbers", "soffice not found");
        return;
    };
    let e = Env::new();
    let out = generate(&e);
    let mut wb = recalc(&soffice, e.dir.path(), &out);
    let mut errors = Vec::new();
    for s in SUP_SHEETS {
        let r = wb.worksheet_range(s).unwrap();
        for (row, col, v) in r.cells() {
            if let Data::Error(err) = v {
                errors.push(format!("{s}!R{}C{}: {err:?}", row + 1, col + 1));
            }
        }
    }
    assert!(errors.is_empty(), "formula errors: {errors:?}");

    let b = wb.worksheet_range("Sup_Book").unwrap();
    let c = |n: &str| col_of(&b, 4, n);
    let a = row_of(&b, c("name"), "Alphazyme");
    assert_eq!(a, 5, "first tblSupp row sits on Excel row 6");
    assert_eq!(num(&b, a, c("usd_yr")), 96.0);
    assert_eq!(num(&b, a, c("doses_per_wk")), 7.0);
    assert_eq!(num(&b, a, c("daily_dose")), 600.0);
    assert_eq!(num(&b, a, c("n_protocols")), 2.0);
    assert_eq!((num(&b, a, c("n_refs")), t(&b, a, c("stance"))), (1.0, "mixed".into()), "one neutral reference");
    let g = row_of(&b, c("name"), "Gammavit");
    assert_eq!((num(&b, g, c("active_usd_yr")), t(&b, g, c("stance"))), (50.0, "none".into()));
    let dm = row_of(&b, c("name"), "Deltamin");
    assert_eq!((num(&b, dm, c("usd_yr")), num(&b, dm, c("doses_per_wk"))), (81.0, 3.0));
    assert_eq!(num(&b, row_of(&b, c("name"), "Betazine"), c("active_usd_yr")), 0.0, "not taking");
    assert!((num(&b, a, c("weight")) - 96.0 / 227.0).abs() < 1e-9);

    // Attribution: multi-protocol items split equally, case-insensitive protocol match.
    let k = wb.worksheet_range("Sup_Cost").unwrap();
    let p = |name: &str| num(&k, row_of(&k, 0, name), 2);
    assert_eq!((p("Test Protocol"), p("Box One"), p("(none)")), (73.0, 73.0, 81.0));
    let spend = row_of(&k, 0, "Spend with no evidence on file ($/yr)");
    assert_eq!(num(&k, spend, 2), 131.0);
    assert_eq!(num(&k, row_of(&k, 0, "Supplements $/yr"), 1), 227.0);
    let pareto = col_of(&k, 4, "name");
    assert_eq!(t(&k, 5, pareto), "Alphazyme", "largest spend first");
    let hhi = row_of(&k, pareto, "HHI (sum of squared weights)");
    let want = (96f64.powi(2) + 50f64.powi(2) + 81f64.powi(2)) / 227f64.powi(2);
    assert!((num(&k, hhi, pareto + 1) - want).abs() < 1e-9);

    // Calculators: PPV (R 0.1, power 0.8, alpha 0.05) and screening PPV (1%, 0.85, 0.92).
    let calc = wb.worksheet_range("Sup_Calc").unwrap();
    assert!((num(&calc, row_of(&calc, 0, "PPV without bias"), 1) - 0.08 / 0.13).abs() < 1e-9);
    let ppv = 0.85 * 0.01 / (0.85 * 0.01 + 0.08 * 0.99);
    assert!((num(&calc, row_of(&calc, 0, "PPV"), 1) - ppv).abs() < 1e-9);
    assert_eq!(num(&calc, row_of(&calc, 0, "NNT over the horizon"), 1), 50.0);

    // GRADE defaults, the audit lights and the KPIs.
    let v = wb.worksheet_range("Sup_Value").unwrap();
    assert_eq!(num(&v, a, col_of(&v, 4, "certainty")), 1.0, "Sup_Default_Certainty until graded");
    let au = wb.worksheet_range("Sup_Audit").unwrap();
    let au_c = |n: &str| col_of(&au, 4, n);
    assert_eq!(num(&au, a, au_c("indication_light")), 2.0);
    assert_eq!(num(&au, a, au_c("effectiveness_light")), 1.0, "a reference but no supporting one");
    assert_eq!(num(&au, dm, au_c("indication_light")), 0.0);
    assert_eq!(t(&au, dm, au_c("gap")), "intervention without marker");
    let kpi = wb.worksheet_range("Ref_KPI").unwrap();
    let kv = |n: &str| t(&kpi, row_of(&kpi, 0, n), 3);
    assert_eq!(kv("K_SUP_n_active"), "3");
    assert_eq!(kv("K_SUP_usd_yr"), "227");
    assert_eq!(kv("K_SUP_n_interactions"), "0");
    assert_eq!(kv("K_SUP_usd_low_certainty"), "227");
    assert_eq!(kv("K_SUP_ter"), "n/a", "no budget set");
}
