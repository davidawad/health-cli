//! End-to-end: `health sheet` on a store imported from the synthetic workbook,
//! with synthetic stand-ins for `biomarker` and `genome`. The generated
//! workbook is read back with calamine (values, formulas, tables) and its
//! XML parts are inspected for validation, conditional formats and charts.

#![cfg(unix)]

mod support;

use std::io::Read;
use std::path::{Path, PathBuf};

use calamine::{open_workbook, Data, Reader, Xlsx};
use support::Env;

/// Names of the sheets in tab order (hidden ones included).
const SHEETS: &[&str] = &[
    "Dashboard",
    "Biomarkers",
    "Weight & Age",
    "Routine",
    "Products",
    "Supplements",
    "Nutrition",
    "Genetics",
    "Vaccines  Conditions",
    "Sizes",
    "Physicals",
    "Workout",
    "Resources",
    "Evidence",
    "Protocols",
    "Costs",
    "Lists",
    "Params",
    "Ref_KPI",
];

fn generate(e: &Env) -> PathBuf {
    e.write_stubs();
    e.import();
    let out = e.path("Health Dashboard (generated).xlsx");
    e.run(&["sheet", "--out", out.to_str().unwrap()]);
    out
}

fn open(p: &Path) -> Xlsx<std::io::BufReader<std::fs::File>> {
    open_workbook(p).expect("generated workbook opens in a reader")
}

fn text(r: &calamine::Range<Data>, row: u32, col: u32) -> String {
    r.get_value((row, col)).map(ToString::to_string).unwrap_or_default()
}

/// Every XML part of the package as (name, content).
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

fn part<'a>(parts: &'a [(String, String)], name: &str) -> &'a str {
    &parts.iter().find(|(n, _)| n == name).unwrap_or_else(|| panic!("no part {name}")).1
}

/// Path of the worksheet XML for a sheet (workbook order = sheetN.xml order).
fn sheet_xml(p: &[(String, String)], index: usize) -> &str {
    part(p, &format!("xl/worksheets/sheet{}.xml", index + 1))
}

#[test]
fn opens_with_every_sheet_in_order() {
    let e = Env::new();
    let wb = open(&generate(&e));
    assert_eq!(wb.sheet_names(), SHEETS);
}

#[test]
fn columns_match_the_original_sheets() {
    let e = Env::new();
    let mut wb = open(&generate(&e));
    let header = |wb: &mut Xlsx<_>, sheet: &str, n: u32| -> Vec<String> {
        let r = wb.worksheet_range(sheet).unwrap();
        (0..n).map(|c| text(&r, 0, c)).collect()
    };
    assert_eq!(
        header(&mut wb, "Supplements", 16),
        [
            "Supplement",
            "Taking",
            "Frequency",
            "Time",
            "Type",
            "Amount",
            "Unit",
            "Dosage",
            "Notes",
            "Experience",
            "#units/yr",
            "Price",
            "$/yr",
            "Links",
            "Research",
            "Source"
        ]
    );
    assert_eq!(
        header(&mut wb, "Products", 11),
        [
            "Category",
            "Name",
            "$/unit",
            "Dosage",
            "Using",
            "# units/month",
            "# units/year",
            "$/mo.",
            "$/yr",
            "Notes",
            "link"
        ]
    );
    assert_eq!(
        header(&mut wb, "Vaccines  Conditions", 9),
        [
            "VACCINE TYPE",
            "Disease Date",
            "1st Dose Mo/Day/Yr",
            "2nd Dose Mo/Day/Yr",
            "3rd Dose Mo/Day/Yr",
            "4th Dose Mo/Day/Yr",
            "5th Dose Mo/Day/Yr",
            "6th Dose Mo/Day/Yr",
            "Notes"
        ]
    );
    assert_eq!(header(&mut wb, "Routine", 6)[1..5], ["Step", "Time for effect", "Execution Time (s)", "Type"]);
    let nut = wb.worksheet_range("Nutrition").unwrap();
    let nut_header: Vec<String> = (0..24).map(|c| text(&nut, 5, c)).collect();
    assert_eq!(nut_header[2..6], ["Calories (kcal)", "Protein (g)", "Net Carbs (g)", "Fiber (g)"]);
    assert_eq!(nut_header[17], "Qualifying Meal?");
    // The weekly chart data sits to the right of the supplements, as in the original.
    let sup = wb.worksheet_range("Supplements").unwrap();
    assert_eq!(text(&sup, 0, 18), "Day");
    assert_eq!(text(&sup, 0, 19), "Monday");
    assert_eq!(text(&sup, 0, 25), "Sunday");
}

#[test]
fn contains_no_google_only_formulas() {
    let e = Env::new();
    let mut wb = open(&generate(&e));
    let mut formulas = 0;
    for name in wb.sheet_names() {
        let f = wb.worksheet_formula(&name).unwrap();
        for (_, _, x) in f.cells() {
            if x.is_empty() {
                continue;
            }
            formulas += 1;
            let up = x.to_uppercase();
            for banned in ["DUMMYFUNCTION", "__XLUDF", "FILTER(", "QUERY(", "ARRAYFORMULA(", "IMPORTRANGE(", "GOOGLE"] {
                assert!(!up.contains(banned), "{name}: {x} uses {banned}");
            }
        }
    }
    assert!(formulas > 100, "derived values are formulas, found {formulas}");
}

#[test]
fn derived_values_are_formulas_and_inputs_are_values() {
    let e = Env::new();
    let mut wb = open(&generate(&e));
    // Supplements: $/yr and the weekly dose are formulas; price and units are plain numbers.
    let f = wb.worksheet_formula("Supplements").unwrap();
    assert_eq!(f.get_value((1, 12)).unwrap(), "IF(AND(ISNUMBER(L2),ISNUMBER(K2)),L2*K2,0)");
    assert!(
        f.get_value((1, 20)).unwrap().contains("INDEX(Lists!$B$2:$H$9,MATCH($C2,Lists!$A$2:$A$9,0),2)"),
        "{:?}",
        f.get_value((1, 20))
    );
    let r = wb.worksheet_range("Supplements").unwrap();
    assert_eq!(text(&r, 1, 0), "Alphazyme");
    assert_eq!(text(&r, 1, 11), "32");
    assert_eq!(text(&r, 1, 10), "3");
    assert_eq!(text(&r, 1, 1), "YES");
    // Products: per-month and per-year cost are formulas over the typed units/year and price.
    let f = wb.worksheet_formula("Products").unwrap();
    assert!(f.get_value((1, 8)).unwrap().contains("G2*C2"));
    assert_eq!(f.get_value((1, 7)).unwrap(), "I2/12");
    // Dashboard: base stats are inputs; ideal weight, protein and age are formulas.
    let d = wb.worksheet_range("Dashboard").unwrap();
    assert_eq!(text(&d, 0, 1), "Height (in.)");
    assert_eq!(text(&d, 0, 2), "70");
    assert_eq!(text(&d, 0, 5), "O");
    let df = wb.worksheet_formula("Dashboard").unwrap();
    assert!(df.get_value((3, 2)).unwrap().starts_with("ROUND((C3*(C1^2)/703)"));
    assert_eq!(df.get_value((3, 5)).unwrap(), "(TODAY()-F3)/365.25");
    // Weight & Age: chronological age and BMI come from the Dashboard's birthdate and height.
    let wf = wb.worksheet_formula("Weight & Age").unwrap();
    assert_eq!(wf.get_value((1, 2)).unwrap(), "(A2-Dashboard!$F$3)/365.25");
    assert!(wf.get_value((1, 4)).unwrap().contains("Dashboard!$C$1^2"));
}

#[test]
fn labs_come_from_biomarker_json() {
    let e = Env::new();
    let mut wb = open(&generate(&e));
    let r = wb.worksheet_range("Biomarkers").unwrap();
    let header: Vec<String> = (0..r.width() as u32).map(|c| text(&r, 0, c)).collect();
    assert_eq!(header[..7], ["Test", "Category", "Notes", "Units", "Reference Interval", "ref. low", "ref. high"]);
    assert_eq!(header[7..9], ["2023-05-01", "2024-05-01"], "one column per draw date");
    assert_eq!(header[9..], ["Latest", "Status"]);
    let rows: Vec<String> = (1..r.height() as u32).map(|i| text(&r, i, 0)).collect();
    assert_eq!(rows[0], "White Blood Cells", "categories in clinical order: CBC first");
    assert!(rows.contains(&"LDL Cholesterol".to_string()) && !rows.contains(&"Weight".to_string()), "{rows:?}");
    let ldl = (1..r.height() as u32).find(|i| text(&r, *i, 0) == "LDL Cholesterol").unwrap();
    assert_eq!((text(&r, ldl, 7), text(&r, ldl, 8)), ("131".into(), "96".into()));
    assert_eq!(text(&r, ldl, 4), "0-99");
    // Weight & Age: weight and biological age, one row per measured date.
    let w = wb.worksheet_range("Weight & Age").unwrap();
    assert_eq!(w.height(), 4, "header + dates with a weight, a biological age or an intake entry");
    assert_eq!(text(&w, 1, 1), "190");
    assert_eq!(text(&w, 3, 3), "28.5");
    // Intake comes from the store (imported from the workbook), not from biomarker.
    assert_eq!((text(&w, 1, 7), text(&w, 1, 8), text(&w, 1, 9)), ("1500".into(), "2200".into(), "120".into()));
    assert_eq!(text(&w, 2, 7), "1400");
    let wf = wb.worksheet_formula("Weight & Age").unwrap();
    assert_eq!(wf.get_value((1, 10)).unwrap(), "IF(AND(ISNUMBER(J2),ISNUMBER(G2)),J2/G2,\"\")");
    assert_eq!(
        wf.get_value((2, 11)).unwrap(),
        "IF(AND(ISNUMBER(B2),ISNUMBER(H2),ISNUMBER(I2)),B2-((I2-H2)/3500),\"\")"
    );
    // Genetics: counts per kit, never genotypes.
    let g = wb.worksheet_range("Genetics").unwrap();
    assert_eq!(text(&g, 1, 1), "synthetic-kit");
    assert_eq!(text(&g, 1, 5), "1000");
}

#[test]
fn has_tables_validation_conditional_formats_and_charts() {
    let e = Env::new();
    let out = generate(&e);
    let p = parts(&out);
    let tables: Vec<&(String, String)> = p.iter().filter(|(n, _)| n.starts_with("xl/tables/table")).collect();
    assert!(tables.len() >= 14, "{} tables", tables.len());
    let names: Vec<String> = tables
        .iter()
        .map(|(_, x)| x.split("displayName=\"").nth(1).unwrap().split('"').next().unwrap().to_string())
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "table names are unique: {names:?}");
    for t in ["tblSupplements", "tblProducts", "tblBiomarkers", "tblWeightAge", "tblNutrition", "tblWeeklyDose"] {
        assert!(names.iter().any(|n| n == t), "{t} in {names:?}");
    }
    // Supplements (index 5): YES/NO, type and frequency drop-downs.
    let sup = sheet_xml(&p, 5);
    assert!(sup.matches("<dataValidation ").count() >= 4);
    assert!(sup.contains("YES,NO") && sup.contains("Rx,Nt,Sp") && sup.contains("Lists!$A$2:$A$9"), "{sup:.0}");
    // Biomarkers (index 1): out-of-range and in-range conditional formats against ref. low/high.
    let bio = sheet_xml(&p, 1);
    assert!(bio.contains("<conditionalFormatting") && bio.contains("$F2") && bio.contains("$G2"));
    // Nutrition: colour scales and YES/NO.
    let nut = sheet_xml(&p, 6);
    assert!(nut.contains("colorScale") && nut.contains("YES,NO"));
    let charts: Vec<&(String, String)> = p.iter().filter(|(n, _)| n.starts_with("xl/charts/chart")).collect();
    assert!(charts.len() >= 10, "{} charts", charts.len());
    let all: String = charts.iter().map(|(_, x)| x.as_str()).collect();
    for kind in ["<c:lineChart>", "<c:barChart>", "<c:pieChart>"] {
        assert!(all.contains(kind), "no {kind}");
    }
    assert!(all.contains("Weekly Supplement Breakdown") && all.contains("Cost breakdown"));
    assert!(all.contains("Biomarkers!$H$1:$I$1"), "lab charts use the date header row");
    // Excel recalculates formulas when the file opens.
    assert!(part(&p, "xl/workbook.xml").contains("fullCalcOnLoad"));
}

#[test]
fn refuses_to_overwrite_a_hand_made_workbook() {
    let e = Env::new();
    e.write_stubs();
    e.import();
    let original = e.workbook();
    let before = std::fs::read(&original).unwrap();
    let out = e.cmd().args(["sheet", "--out", original.to_str().unwrap()]).output().unwrap();
    assert_eq!(out.status.code(), Some(4), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8_lossy(&out.stderr).contains("not a generated workbook"));
    assert_eq!(std::fs::read(&original).unwrap(), before, "untouched");
    // A "(generated)" file may be regenerated in place.
    let gen = e.path("Health Dashboard (generated).xlsx");
    e.run(&["sheet", "--out", gen.to_str().unwrap()]);
    e.run(&["sheet", "--out", gen.to_str().unwrap()]);
}

#[test]
fn output_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let e = Env::new();
    let out = generate(&e);
    assert_eq!(std::fs::metadata(out).unwrap().permissions().mode() & 0o777, 0o600);
}

#[test]
fn only_restricts_the_sheets_and_list_names_them() {
    let e = Env::new();
    e.write_stubs();
    e.import();
    let out = e.path("part (generated).xlsx");
    e.run(&["sheet", "--out", out.to_str().unwrap(), "--only", "supplements,lists"]);
    assert_eq!(
        open(&out).sheet_names(),
        ["Supplements", "Lists", "Params", "Ref_KPI"],
        "infra sheets always come along"
    );
    e.cmd().args(["sheet", "--only", "nope", "--out", out.to_str().unwrap()]).assert().failure().code(2);
    let list = e.json(&["sheet", "--list"]);
    let ids: Vec<&str> = list["data"].as_array().unwrap().iter().map(|s| s["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"dashboard") && ids.contains(&"costs") && ids.contains(&"lists"));
}

#[test]
fn works_without_the_sibling_tools() {
    let e = Env::new();
    e.import(); // no stubs: biomarker/genome are "not installed"
    let out = e.path("lab-less (generated).xlsx");
    let res = e.cmd().args(["sheet", "--out", out.to_str().unwrap()]).output().unwrap();
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    assert!(String::from_utf8_lossy(&res.stderr).contains("no lab sheets"));
    let names = open(&out).sheet_names();
    assert!(!names.contains(&"Biomarkers".to_string()) && !names.contains(&"Genetics".to_string()));
    assert!(names.contains(&"Supplements".to_string()) && names.contains(&"Dashboard".to_string()));
    let mut wb = open(&out);
    let d = wb.worksheet_range("Dashboard").unwrap();
    assert_eq!(text(&d, 1, 2), "no labs");
}

#[test]
fn the_default_output_path_is_a_setting() {
    let e = Env::new();
    e.write_stubs();
    e.import();
    let out = e.path("from-setting (generated).xlsx");
    let res = e.cmd().env("HEALTH_SHEET_OUT", &out).args(["sheet"]).output().unwrap();
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    assert!(out.exists());
}

#[test]
fn infrastructure_sheets_expose_params_names_and_kpis() {
    let e = Env::new();
    let out = generate(&e);
    let p = parts(&out);
    let wbx = part(&p, "xl/workbook.xml");
    for n in [
        "Today",
        "DOB",
        "Height_in",
        "Height_cm",
        "Half_Life_days",
        "RCV_z",
        "Sel_Marker",
        "K_INFRA_n_params",
        "K_INFRA_n_unverified",
    ] {
        assert!(wbx.contains(&format!("definedName name=\"{n}\"")), "{n} not defined");
    }
    let mut wb = open(&out);
    let r = wb.worksheet_range("Params").unwrap();
    assert_eq!(
        (0..8).map(|c| text(&r, 0, c)).collect::<Vec<_>>(),
        ["name", "value", "unit", "state", "source", "note", "min", "max"]
    );
    let rows: Vec<Vec<String>> = (1..r.height() as u32).map(|i| (0..6).map(|c| text(&r, i, c)).collect()).collect();
    let pheno = rows.iter().find(|r| r[0] == "PhenoAge_gamma").unwrap();
    assert_eq!((pheno[3].as_str(), pheno[4].as_str()), ("UNVERIFIED", "pending: Levine 2018 (PhenoAge)"));
    let rcv = rows.iter().find(|r| r[0] == "RCV_z").unwrap();
    assert_eq!((rcv[1].as_str(), rcv[3].as_str()), ("1.96", "verified"));
    // Limits are enforced by validation; UNVERIFIED rows are highlighted.
    let sheet = sheet_xml(&p, SHEETS.iter().position(|s| *s == "Params").unwrap());
    assert!(sheet.contains("<dataValidation ") && sheet.contains("decimal") && sheet.contains("UNVERIFIED"));
    let k = wb.worksheet_range("Ref_KPI").unwrap();
    assert_eq!(text(&k, 1, 0), "K_INFRA_n_params");
    assert!(wb.worksheet_formula("Ref_KPI").unwrap().get_value((1, 3)).unwrap().starts_with("COUNTA(Params!$A$2:$A$"));
}

#[test]
fn workstream_tabs_use_their_theme_colour_and_infra_tabs_are_grey() {
    let e = Env::new();
    let p = parts(&generate(&e));
    let n = SHEETS.iter().position(|s| *s == "Params").unwrap();
    assert!(sheet_xml(&p, n).contains("<tabColor"), "Params tab is coloured");
    assert!(sheet_xml(&p, 0).contains("<tabColor"), "Dashboard tab is coloured");
}
