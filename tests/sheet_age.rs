//! Workstream W4 (AGE): the `Age_*` sheets of a generated workbook, built from
//! the synthetic store and the synthetic `biomarker` / `genome` stand-ins, then
//! read back (values, formulas, tables, defined names, XML parts).

#![cfg(unix)]

mod support;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use calamine::{open_workbook, Data, Reader, Xlsx};
use support::soffice::soffice;
use support::Env;

/// Every AGE sheet in tab order (AGE-01..18).
const AGE_SHEETS: &[&str] = &[
    "Age_Pheno",
    "Age_Vendor",
    "Age_PhenoWhatIf",
    "Age_Claims",
    "Age_Epi",
    "Age_Hallmarks",
    "Age_SENS",
    "Age_Protocols",
    "Age_Horsemen",
    "Age_DEXA",
    "Age_Decathlon",
    "Age_VO2",
    "Age_Zone2",
    "Age_Z2Tools",
    "Age_Strength",
    "Age_Volume",
    "Age_Sauna",
    "Age_Decade",
];
/// Sheets that need lab data (tblPanels / Calc_Marker).
const NEED_LABS: &[&str] = &["Age_Pheno", "Age_Vendor", "Age_PhenoWhatIf"];

fn generate(e: &Env) -> PathBuf {
    e.write_stubs();
    e.import();
    let out = e.path("Health Dashboard (generated).xlsx");
    e.run(&["sheet", "--out", out.to_str().unwrap()]);
    out
}

fn open(p: &Path) -> Xlsx<std::io::BufReader<std::fs::File>> {
    open_workbook(p).expect("generated workbook opens")
}

fn text(r: &calamine::Range<Data>, row: u32, col: u32) -> String {
    r.get_value((row, col)).map(ToString::to_string).unwrap_or_default()
}

fn parts(p: &Path) -> Vec<(String, String)> {
    let mut z = zip::ZipArchive::new(std::fs::File::open(p).unwrap()).unwrap();
    (0..z.len())
        .filter_map(|i| {
            let mut f = z.by_index(i).ok()?;
            let name = f.name().to_string();
            let mut s = String::new();
            f.read_to_string(&mut s).ok()?;
            Some((name, s))
        })
        .collect()
}

fn part<'a>(p: &'a [(String, String)], name: &str) -> &'a str {
    &p.iter().find(|(n, _)| n == name).unwrap_or_else(|| panic!("no part {name}")).1
}

/// Worksheet XML of a sheet by its tab position.
fn sheet_xml<'a>(p: &'a [(String, String)], names: &[String], sheet: &str) -> &'a str {
    let i = names.iter().position(|n| n == sheet).unwrap_or_else(|| panic!("no sheet {sheet}"));
    part(p, &format!("xl/worksheets/sheet{}.xml", i + 1))
}

/// Every non-empty cell text of a sheet, joined (for banner and label checks).
fn all_text(wb: &mut Xlsx<std::io::BufReader<std::fs::File>>, sheet: &str) -> String {
    let r = wb.worksheet_range(sheet).unwrap();
    r.cells().map(|(_, _, v)| v.to_string()).collect::<Vec<_>>().join("\n")
}

fn all_formulas(wb: &mut Xlsx<std::io::BufReader<std::fs::File>>, sheet: &str) -> Vec<String> {
    wb.worksheet_formula(sheet).unwrap().cells().map(|(_, _, f)| f.clone()).filter(|f| !f.is_empty()).collect()
}

/// Record a skipped test where the runner asked for it (TEST_SKIP_LOG), and say so on stderr.
fn skip(test: &str, reason: &str) {
    eprintln!("SKIP {test}: {reason}");
    if let Some(p) = std::env::var_os("TEST_SKIP_LOG") {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
            let _ = writeln!(f, "{test}: {reason}");
        }
    }
}

#[test]
fn every_age_sheet_is_generated_in_order_with_its_banner_and_disclaimer() {
    let e = Env::new();
    let out = generate(&e);
    let mut wb = open(&out);
    let ages: Vec<String> = wb.sheet_names().into_iter().filter(|n| n.starts_with("Age_")).collect();
    assert_eq!(ages, AGE_SHEETS);
    for s in AGE_SHEETS {
        let t = all_text(&mut wb, s);
        assert!(t.contains("AGE-"), "{s}: names its idea ids");
        assert!(t.contains("not medical advice"), "{s}: carries the disclaimer");
    }
    // Placeholders say what they wait for.
    for s in [
        "Age_Pheno",
        "Age_Epi",
        "Age_Hallmarks",
        "Age_SENS",
        "Age_DEXA",
        "Age_Decathlon",
        "Age_VO2",
        "Age_Zone2",
        "Age_Strength",
        "Age_Volume",
    ] {
        assert!(all_text(&mut wb, s).contains("PLACEHOLDER - awaiting data"), "{s}: placeholder banner");
    }
    assert!(all_text(&mut wb, "Age_Z2Tools").contains("UNVERIFIED - pending: Tanaka 2001"));
    assert!(all_text(&mut wb, "Age_Decade").contains("not additive"));
}

#[test]
fn age_tables_kpis_and_parameters_are_registered() {
    let e = Env::new();
    let out = generate(&e);
    let p = parts(&out);
    let tables: Vec<String> = p
        .iter()
        .filter(|(n, _)| n.starts_with("xl/tables/table"))
        .map(|(_, x)| x.split("displayName=\"").nth(1).unwrap().split('"').next().unwrap().to_string())
        .collect();
    for t in [
        "tblAgePheno",
        "tblAgePhenoInputs",
        "tblAgeVendor",
        "tblAgeWhatIf",
        "tblAgeClaims",
        "tblAgeEpi",
        "tblAgeHallmarks",
        "tblAgeHallmarkMatrix",
        "tblAgeSens",
        "tblAgeSensMatrix",
        "tblAgeSinclair",
        "tblAgeProtocolCompare",
        "tblAgeDrugWatch",
        "tblAgeHorsemen",
        "tblAgeDexa",
        "tblAgeDecathlon",
        "tblAgeVO2",
        "tblAgeVO2Projection",
        "tblAgeVO2Norms",
        "tblAgeZone2Weeks",
        "tblAgeCardio",
        "tblAgeWkg",
        "tblAge4x4",
        "tblAgeDrift",
        "tblAgeStrengthTests",
        "tblAgeMuscleAccount",
        "tblAgeRadaelli",
        "tblAgeLandmarks",
        "tblAgeMeso",
        "tblAgeLift",
        "tblAgeSaunaWeeks",
        "tblAgeSaunaLog",
        "tblAgeWalkLog",
        "tblAgeDecade",
        "tblAgeRiskLedger",
    ] {
        assert!(tables.contains(&t.to_string()), "{t} missing");
    }
    let wbx = part(&p, "xl/workbook.xml");
    let kpis = [
        "phenoage",
        "pheno_accel",
        "vendor_gap",
        "hallmark_gaps",
        "horsemen_thin",
        "decathlon_pass",
        "vo2_latest",
        "z2_week",
        "sessions_week",
        "sauna_week",
    ];
    for k in kpis {
        assert!(wbx.contains(&format!("definedName name=\"K_AGE_{k}\"")), "K_AGE_{k} not defined");
    }
    for n in ["Age_PA_w_rdw", "Age_Z2_target_min", "Age_HRmax_a", "Age_VO2_RHR_k", "Age_Target_age"] {
        assert!(wbx.contains(&format!("definedName name=\"{n}\"")), "{n} not defined");
    }
    // Params rows: weights verified with their citation, the secondary-source constants pending.
    let mut wb = open(&out);
    let r = wb.worksheet_range("Params").unwrap();
    let row = |name: &str| -> Vec<String> {
        let i = (1..r.height() as u32).find(|i| text(&r, *i, 0) == name).unwrap_or_else(|| panic!("no param {name}"));
        (0..6).map(|c| text(&r, i, c)).collect()
    };
    let w = row("Age_PA_w_rdw");
    assert_eq!((w[1].as_str(), w[3].as_str()), ("0.3306", "verified"));
    assert!(w[4].contains("Levine 2018 Table 1"));
    for n in ["Age_HRmax_a", "Age_HRmax_b", "Age_VO2_RHR_k", "Age_PA_t_months", "Age_VO2_decline_post50_pct"] {
        let x = row(n);
        assert_eq!(x[3], "UNVERIFIED", "{n}");
        assert!(x[4].starts_with("pending: "), "{n}: {}", x[4]);
    }
    let k = wb.worksheet_range("Ref_KPI").unwrap();
    let n_age = (1..k.height() as u32).filter(|i| text(&k, *i, 1) == "AGE").count();
    assert_eq!(n_age, kpis.len(), "every AGE KPI is listed on Ref_KPI");
}

#[test]
fn phenoage_reads_the_data_layer_and_keeps_the_crp_and_intercept_honest() {
    let e = Env::new();
    let mut wb = open(&generate(&e));
    let f = wb.worksheet_formula("Age_Pheno").unwrap();
    let v = wb.worksheet_range("Age_Pheno").unwrap();
    let joined = f.cells().map(|(_, _, x)| x.clone()).collect::<Vec<_>>().join("\n");
    assert!(joined.contains("Data_Panels!A2"), "one row per tblPanels row");
    assert!(joined.contains("Data_Labs!$C$2:$C$"), "values come from tblLabs");
    assert!(
        joined.contains("Assumed_CRP") && joined.contains("PhenoAge_intercept"),
        "lite mode and the guarded intercept"
    );
    assert!(joined.contains("LN(MAX(") && joined.contains("Age_PA_crp_floor"), "CRP log is guarded");
    assert!(joined.contains("PhenoAge_k141") && joined.contains("Age_PA_t_months*PhenoAge_gamma"));
    // The inputs table carries the nine slugs and the hs-CRP row says it is missing on every panel.
    let slugs: Vec<String> = (0..v.height() as u32).map(|r| text(&v, r, 1)).collect();
    for s in ["albumin", "creatinine", "glucose", "hs-crp", "lymphocytes-pct", "mcv", "rdw", "alp", "wbc"] {
        assert!(slugs.iter().any(|x| x == s), "{s} slug");
    }
    // Synthetic labs have two panels: two per-panel rows (panel ids from Data_Panels A2, A3).
    assert!(joined.contains("Data_Panels!A3") && !joined.contains("Data_Panels!A4"));
    // What-if: each swap row references the inputs table weights.
    let w = all_formulas(&mut wb, "Age_PhenoWhatIf").join("\n");
    assert!(
        w.contains("Age_Pheno!$E$") && w.contains("Age_Pheno!$G$"),
        "weights and latest values from Age_Pheno: {w:.400}"
    );
}

#[test]
fn seeded_reference_rows_round_trip_as_values() {
    let e = Env::new();
    let mut wb = open(&generate(&e));
    let claims = all_text(&mut wb, "Age_Claims");
    assert!(claims.contains("Dave Pascoe") && claims.contains("62") && claims.contains("38"));
    assert!(claims.contains("different clock") || claims.contains("Different clock"));
    let proto = all_text(&mut wb, "Age_Protocols");
    for x in ["NMN", "Resveratrol", "Metformin", "Rapamycin", "LIFE l.10605-10662", "Blueprint", "Greger"] {
        assert!(proto.contains(x), "{x} on Age_Protocols");
    }
    let dec = all_text(&mut wb, "Age_Decathlon");
    assert!(dec.contains("Pick up a young child from the floor") && dec.contains("90 steps in 2 min"));
    let strength = all_text(&mut wb, "Age_Strength");
    assert!(strength.contains("0.73") && strength.contains("2.33"), "Radaelli effect sizes");
    let horse = all_text(&mut wb, "Age_Horsemen");
    for h in ["Atherosclerotic disease", "Cancer", "Neurodegeneration", "Metabolic dysfunction"] {
        assert!(horse.contains(h), "{h}");
    }
    // Decathlon: required = target / (1 - decline)^(years / 10).
    let d = all_formulas(&mut wb, "Age_Decathlon").join("\n");
    assert!(d.contains("/(1-H") && d.contains("/10))"), "{d:.300}");
    assert!(d.contains("Decline_Strength_pct_decade") && d.contains("Decline_VO2_pct_decade"));
}

#[test]
fn stack_views_read_supplements_tags_and_evidence_from_the_data_layer() {
    let e = Env::new();
    let mut wb = open(&generate(&e));
    let h = all_formulas(&mut wb, "Age_Hallmarks").join("\n");
    assert!(h.contains("Data_SuppTags!") && h.contains("Data_Supp!") && h.contains("Data_Evidence!"));
    assert!(h.contains("Ref_Hallmarks!B2") && h.contains("Age_Gap_max"));
    let s = all_formulas(&mut wb, "Age_SENS").join("\n");
    assert!(s.contains("Ref_SENS!B7") && s.contains("Data_SuppTags!"));
    let z = all_formulas(&mut wb, "Age_Zone2").join("\n");
    assert!(z.contains("Age_Z2_target_min") && z.contains("WEEKDAY(Today,2)"));
    let v = all_formulas(&mut wb, "Age_Volume").join("\n");
    assert!(v.contains("Age_HardSet_RIR") && v.contains("Age_Zone2!") && v.contains("Data_Workout!"));
}

#[test]
fn age_sheets_have_validation_conditional_formats_and_charts() {
    let e = Env::new();
    let out = generate(&e);
    let names = open(&out).sheet_names();
    let p = parts(&out);
    for s in ["Age_Horsemen", "Age_Zone2", "Age_Volume", "Age_Decathlon"] {
        assert!(sheet_xml(&p, &names, s).contains("<dataValidation "), "{s}: drop-downs");
    }
    for s in ["Age_Pheno", "Age_Hallmarks", "Age_Decathlon", "Age_Zone2", "Age_Volume"] {
        assert!(sheet_xml(&p, &names, s).contains("<conditionalFormatting"), "{s}: conditional formats");
    }
    for s in AGE_SHEETS.iter().filter(|s| !matches!(**s, "Age_Decade" | "Age_Protocols")) {
        assert!(sheet_xml(&p, &names, s).contains("<drawing "), "{s}: has a chart");
    }
    assert!(p.iter().any(|(n, x)| n.starts_with("xl/comments") && x.contains("Source: Levine 2018 Table 1")));
}

#[test]
fn lab_free_build_keeps_the_age_views_that_do_not_need_labs() {
    let e = Env::new();
    e.import(); // no stubs: biomarker/genome are "not installed"
    let out = e.path("lab-less (generated).xlsx");
    let res = e.cmd().args(["sheet", "--out", out.to_str().unwrap()]).output().unwrap();
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    let names = open(&out).sheet_names();
    for s in AGE_SHEETS {
        assert_eq!(names.contains(&s.to_string()), !NEED_LABS.contains(s), "{s}");
    }
}

#[test]
fn only_builds_a_single_age_sheet() {
    let e = Env::new();
    e.write_stubs();
    e.import();
    let out = e.path("only (generated).xlsx");
    e.run(&["sheet", "--only", "age-vo2", "--out", out.to_str().unwrap()]);
    let names = open(&out).sheet_names();
    assert!(names.contains(&"Age_VO2".to_string()) && !names.contains(&"Age_Pheno".to_string()));
    assert!(part(&parts(&out), "xl/workbook.xml").contains("definedName name=\"Age_VO2_RHR_k\""));
}

/// Recalculate in headless LibreOffice and fail on any formula error in an `Age_*` sheet.
#[test]
fn age_formulas_recalculate_without_errors() {
    let Some(soffice) = soffice() else {
        skip("sheet_age::age_formulas_recalculate_without_errors", "soffice (LibreOffice) not installed");
        return;
    };
    let e = Env::new();
    let out = generate(&e);
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
    let res = Command::new(&soffice)
        .arg(format!("-env:UserInstallation=file://{}", profile.display()))
        .args(["--headless", "--convert-to", "xlsx", "--outdir"])
        .arg(&outdir)
        .arg(&out)
        .output()
        .unwrap();
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    let mut wb: Xlsx<_> = open_workbook(outdir.join(out.file_name().unwrap())).unwrap();
    let mut errors = Vec::new();
    for s in AGE_SHEETS {
        let r = wb.worksheet_range(s).unwrap();
        for (row, col, v) in r.cells() {
            if let Data::Error(err) = v {
                errors.push(format!("{s}!R{}C{}: {err:?}", row + 1, col + 1));
            }
        }
    }
    assert!(errors.is_empty(), "formula errors after recalculation: {errors:?}");
    // Values on the synthetic data (labs: two panels, one `biological-age` draw of 28.5 on 2024-02-01).
    let num = |v: Option<&Data>| match v {
        Some(Data::Float(f)) => *f,
        Some(Data::Int(i)) => *i as f64,
        Some(Data::DateTime(d)) => d.as_f64(),
        other => panic!("not a number: {other:?}"),
    };
    let right_of = |r: &calamine::Range<Data>, label: &str| -> Option<Data> {
        let (row, col, _) = r.cells().find(|(_, _, v)| matches!(v, Data::String(s) if s == label))?;
        r.get_value((row as u32, col as u32 + 1)).cloned()
    };
    let p = wb.worksheet_range("Age_Pheno").unwrap();
    let row_of = |id: &str| (0..p.height() as u32).find(|r| text(&p, *r, 0) == id).unwrap_or_else(|| panic!("no {id}"));
    let (p23, p24) = (row_of("P20230501"), row_of("P20240501"));
    assert_eq!(num(p.get_value((p24, 5))), 88.0, "glucose of the 2024 panel");
    assert_eq!(num(p.get_value((p23, 11))), 5.6, "WBC of the 2023 panel");
    assert_eq!(text(&p, p24, 13), "incomplete", "seven inputs missing: no PhenoAge");
    assert_eq!(text(&p, p24, 17), "", "no number on an incomplete panel");
    let v = wb.worksheet_range("Age_Vendor").unwrap();
    let (vr, _, _) = v
        .cells()
        .find(|(_, c, x)| *c == 1 && matches!(x, Data::Float(f) if (*f - 28.5).abs() < 1e-9))
        .expect("vendor value");
    let (bio, chrono, gap) =
        (num(v.get_value((vr as u32, 1))), num(v.get_value((vr as u32, 2))), num(v.get_value((vr as u32, 3))));
    assert!((gap - (bio - chrono)).abs() < 1e-9 && chrono > 0.0, "gap = vendor - chronological");
    let h = wb.worksheet_range("Age_Horsemen").unwrap();
    let ascvd = (0..h.height() as u32).find(|r| text(&h, *r, 0) == "Atherosclerotic disease").unwrap();
    // LDL-C and HDL-C were drawn 2024-05-01 (stale), ApoB / Lp(a) / hs-CRP / BP have nothing.
    assert_eq!(
        (num(h.get_value((ascvd, 1))), num(h.get_value((ascvd, 3))), num(h.get_value((ascvd, 4)))),
        (6.0, 2.0, 4.0)
    );
    let z = wb.worksheet_range("Age_Zone2").unwrap();
    let monday = num(z.get_value((7, 0)));
    assert_eq!((monday as i64 + 5) % 7, 0, "weeks start on a Monday (serial {monday})");
    let t = wb.worksheet_range("Age_Z2Tools").unwrap();
    let hrmax = num(right_of(&t, "HRmax used").as_ref());
    let lo = num(right_of(&t, "Zone 2 from / to (bpm)").as_ref());
    assert_eq!(lo, (hrmax * 0.70).round(), "zone 2 floor = 70% of HRmax");
    let k = wb.worksheet_range("Ref_KPI").unwrap();
    let kv = |name: &str| {
        let row = (0..k.height() as u32).find(|r| text(&k, *r, 0) == name)?;
        k.get_value((row, 3)).cloned()
    };
    assert_eq!(num(kv("K_AGE_hallmark_gaps").as_ref()), 9.0, "no tags yet: every hallmark is a gap");
    assert!((num(kv("K_AGE_vendor_gap").as_ref()) - gap).abs() < 1e-9);
    assert_eq!(kv("K_AGE_phenoage").map(|v| v.to_string()).as_deref(), Some("n/a"));
}
