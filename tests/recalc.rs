//! Recalculate the generated workbook in headless LibreOffice and fail on any
//! formula error (#NAME?, #REF!, #VALUE!, ...): the build-time Checks run that
//! catches Excel-incompatible formulas. Also checks the `Calc_*` numbers.
//! Skipped when `soffice` is not installed.

#![cfg(unix)]

mod support;

use std::path::Path;
use std::process::Command;

use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use support::Env;

fn soffice() -> Option<String> {
    ["soffice", "/Applications/LibreOffice.app/Contents/MacOS/soffice", "/opt/homebrew/bin/soffice"]
        .iter()
        .find(|c| Command::new(c).arg("--version").output().is_ok_and(|o| o.status.success()))
        .map(|c| (*c).to_string())
}

/// Convert `input` with a private profile that always recalculates OOXML on load.
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
    let outdir = dir.join(format!("recalculated-{}", input.file_stem().unwrap().to_string_lossy().replace(' ', "_")));
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

fn num(r: &Range<Data>, row: u32, col: u32) -> f64 {
    match r.get_value((row, col)) {
        Some(Data::Float(f)) => *f,
        Some(Data::Int(i)) => *i as f64,
        other => panic!("({row},{col}) is {other:?}"),
    }
}

fn row_of(r: &Range<Data>, key: &str) -> u32 {
    (1..r.height() as u32)
        .find(|i| r.get_value((*i, 0)).map(ToString::to_string).as_deref() == Some(key))
        .unwrap_or_else(|| panic!("no row {key}"))
}

#[test]
fn no_formula_errors_after_a_full_recalculation() {
    let Some(soffice) = soffice() else {
        eprintln!("soffice not found: skipping");
        return;
    };
    let e = Env::new();
    e.write_stubs();
    e.import();
    let out = e.path("Health Dashboard (generated).xlsx");
    e.run(&["sheet", "--out", out.to_str().unwrap()]);
    let mut wb = recalc(&soffice, e.dir.path(), &out);

    let mut errors = Vec::new();
    for name in wb.sheet_names() {
        let r = wb.worksheet_range(&name).unwrap();
        for (row, col, v) in r.cells() {
            if let Data::Error(err) = v {
                errors.push(format!("{name}!R{}C{}: {err:?}", row + 1, col + 1));
            }
        }
    }
    assert!(errors.is_empty(), "formula errors after recalculation: {errors:?}");
    // The recalculation really happened: derived values are no longer the cached zeros.
    let d = wb.worksheet_range("Dashboard").unwrap();
    let ideal = d.get_value((3, 2)).map(ToString::to_string).unwrap_or_default();
    assert!(ideal.ends_with(" lb"), "ideal weight was recalculated: {ideal:?}");

    // Calc_Marker for LDL: draws of 131 (2023) and 96 (2024); reference 0-99, optimal < 70.
    let m = wb.worksheet_range("Calc_Marker").unwrap();
    let h: Vec<String> = (0..m.width() as u32).map(|c| m.get_value((0, c)).unwrap().to_string()).collect();
    let c = |name: &str| h.iter().position(|x| x == name).unwrap() as u32;
    let r = row_of(&m, "ldl-c");
    assert_eq!(num(&m, r, c("n")), 2.0);
    assert_eq!((num(&m, r, c("last")), num(&m, r, c("previous"))), (96.0, 131.0));
    assert_eq!((num(&m, r, c("mean")), num(&m, r, c("min")), num(&m, r, c("max"))), (113.5, 96.0, 131.0));
    assert!((num(&m, r, c("sd")) - 24.7487).abs() < 1e-3, "sd");
    assert_eq!(num(&m, r, c("median")), 113.5);
    assert!(num(&m, r, c("slope_per_year")) < 0.0, "falling");
    assert_eq!(num(&m, r, c("dir_sign")), 1.0);
    assert_eq!(num(&m, r, c("status")), 1.0, "96: inside reference, above optimal");
    assert_eq!(num(&m, r, c("pct_in_ref")), 0.5);
    assert_eq!(num(&m, r, c("pct_in_opt")), 0.0);
    assert!(num(&m, r, c("days_since_last")) > 0.0);
    assert_eq!(num(&m, r, c("distance")), 0.0);
    let w = row_of(&m, "wbc");
    assert_eq!(
        (num(&m, w, c("n")), num(&m, w, c("status"))),
        (1.0, 0.0),
        "no optimal range: in reference counts as optimal"
    );
    let u = row_of(&m, "uacr");
    assert_eq!(num(&m, u, c("n")), 0.0, "a marker without data is a row of zeros and blanks");

    // Calc_Panel and the names built on it.
    let p = wb.worksheet_range("Calc_Panel").unwrap();
    assert_eq!(num(&p, 1, 3), 3.0);
    assert!((num(&p, 1, 4) - 1.0 / 3.0).abs() < 1e-9, "2023 panel: only the WBC is inside its range");
    assert_eq!((num(&p, 2, 3), num(&p, 2, 4)), (3.0, 1.0));
    let k = wb.worksheet_range("Ref_KPI").unwrap();
    let kv = |name: &str| {
        let r = row_of(&k, name);
        k.get_value((r, 3)).map(ToString::to_string).unwrap_or_default()
    };
    assert_eq!(kv("K_DATA_n_panels"), "2");
    assert_eq!(kv("K_DATA_n_measurements"), "9");
    assert!(
        kv("K_DATA_latest_panel").starts_with("2024-05-01") || kv("K_DATA_latest_panel").parse::<f64>().is_ok(),
        "{}",
        kv("K_DATA_latest_panel")
    );
}

/// Structured table references (`tblX[col]`) are what workstream authors will be tempted to
/// write; this pins down whether they survive a recalculation in LibreOffice.
#[test]
fn structured_references_work_in_formulas() {
    let Some(soffice) = soffice() else {
        eprintln!("soffice not found: skipping");
        return;
    };
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("probe.xlsx");
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet().set_name("Data_P").unwrap();
    let table = rust_xlsxwriter::Table::new()
        .set_name("tblProbe")
        .set_columns(&[rust_xlsxwriter::TableColumn::new().set_header("val")]);
    ws.add_table(0, 0, 3, 0, &table).unwrap();
    for (i, v) in [1.0, 2.0, 3.0].iter().enumerate() {
        ws.write_number(1 + i as u32, 0, *v).unwrap();
    }
    ws.write_formula(0, 2, "=SUM(tblProbe[val])").unwrap();
    wb.save(&path).unwrap();
    let mut out = recalc(&soffice, dir.path(), &path);
    let r = out.worksheet_range("Data_P").unwrap();
    assert_eq!(num(&r, 0, 2), 6.0);
}
