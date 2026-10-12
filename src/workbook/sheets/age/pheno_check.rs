//! End-to-end check of the PhenoAge formula chain: the exact formulas `Age_Pheno`
//! writes, over one synthetic panel, recalculated in headless LibreOffice and
//! compared with the same model computed in Rust. Skipped without `soffice`.

use std::process::Command;

use calamine::{open_workbook, Data, Reader, Xlsx};
use rust_xlsxwriter::{Formula, Workbook};

use super::pheno::{phenoage_of_xb, xb_of, CRP, C_SI, C_W, INPUTS, INPUT_HR, TITLE};

/// Synthetic panel in the units of [`INPUTS`] (albumin g/dL ... WBC 10^3/uL; CRP mg/L), age 40.
const PANEL: [f64; 9] = [4.5, 0.9, 88.0, 0.5, 30.0, 90.0, 12.5, 60.0, 5.6];
const AGE: f64 = 40.0;

fn rust_phenoage() -> f64 {
    let w = [-0.0336, 0.0095, 0.1953, 0.0954, -0.0120, 0.0268, 0.3306, 0.0019, 0.0554];
    let si = [10.0, 88.42, 1.0 / 18.016, 0.1, 1.0, 1.0, 1.0, 1.0, 1.0];
    let mut xb = -19.907 + 0.0804 * AGE;
    for i in 0..9 {
        let v = PANEL[i] * si[i];
        xb += w[i] * if i == CRP { v.max(0.01).ln() } else { v };
    }
    let g = 0.0076927_f64;
    let m = 1.0 - (-(xb.exp()) * ((120.0 * g).exp() - 1.0) / g).exp();
    141.50225 + (-0.00553 * (1.0 - m).ln()).ln() / 0.090165
}

fn soffice() -> Option<String> {
    ["soffice", "/Applications/LibreOffice.app/Contents/MacOS/soffice", "/opt/homebrew/bin/soffice"]
        .iter()
        .find(|c| Command::new(c).arg("--version").output().is_ok_and(|o| o.status.success()))
        .map(|c| (*c).to_string())
}

#[test]
fn phenoage_formulas_recalculate_to_the_model_value() {
    let Some(soffice) = soffice() else {
        eprintln!("SKIP phenoage_formulas_recalculate_to_the_model_value: soffice not installed");
        if let Some(p) = std::env::var_os("TEST_SKIP_LOG") {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new().create(true).append(true).open(p).unwrap();
            writeln!(f, "age::pheno_check::phenoage_formulas_recalculate_to_the_model_value: soffice not installed")
                .unwrap();
        }
        return;
    };
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("pheno.xlsx");
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet().set_name(TITLE).unwrap();
    // The inputs table: weight and conversion per input, as Age_Pheno writes them.
    let weights = ["-0.0336", "0.0095", "0.1953", "0.0954", "-0.0120", "0.0268", "0.3306", "0.0019", "0.0554"];
    for (i, (_, _, _, si, _)) in INPUTS.iter().enumerate() {
        let r = INPUT_HR + 1 + i as u32;
        ws.write_formula(r, C_SI, Formula::new(*si)).unwrap();
        ws.write_formula(r, C_W, Formula::new(format!("={}", weights[i]))).unwrap();
        ws.write_number(r, 10, PANEL[i]).unwrap();
    }
    let vals: [String; 9] = std::array::from_fn(|i| format!("$K${}", INPUT_HR + 2 + i as u32));
    ws.write_number(0, 12, AGE).unwrap();
    ws.write_formula(1, 12, Formula::new(format!("={}", xb_of(&vals, "$M$1")))).unwrap();
    ws.write_formula(2, 12, Formula::new(format!("={}", phenoage_of_xb("$M$2")))).unwrap();
    for (n, v) in [
        ("PhenoAge_intercept", "-19.907"),
        ("PhenoAge_gamma", "0.0076927"),
        ("PhenoAge_k141", "141.50225"),
        ("PhenoAge_k0055", "0.00553"),
        ("PhenoAge_k0090", "0.090165"),
        ("Age_PA_t_months", "120"),
        ("Age_PA_crp_floor", "0.01"),
        ("Age_PA_w_age", "0.0804"),
        ("Age_PA_u_albumin", "10"),
        ("Age_PA_u_creatinine", "88.42"),
        ("Age_PA_u_glucose", "18.016"),
        ("Age_PA_u_crp", "10"),
    ] {
        wb.define_name(n, &format!("={v}")).unwrap();
    }
    wb.save(&path).unwrap();
    let profile = dir.path().join("lo-profile");
    std::fs::create_dir_all(profile.join("user")).unwrap();
    std::fs::write(
        profile.join("user/registrymodifications.xcu"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<oor:items xmlns:oor="http://openoffice.org/2001/registry" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
<item oor:path="/org.openoffice.Office.Calc/Formula/Load"><prop oor:name="OOXMLRecalcMode" oor:op="fuse"><value>0</value></prop></item>
</oor:items>"#,
    )
    .unwrap();
    let outdir = dir.path().join("out");
    let res = Command::new(&soffice)
        .arg(format!("-env:UserInstallation=file://{}", profile.display()))
        .args(["--headless", "--convert-to", "xlsx", "--outdir"])
        .arg(&outdir)
        .arg(&path)
        .output()
        .unwrap();
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    let mut out: Xlsx<_> = open_workbook(outdir.join("pheno.xlsx")).unwrap();
    let r = out.worksheet_range(TITLE).unwrap();
    let got = match r.get_value((2, 12)) {
        Some(Data::Float(f)) => *f,
        other => panic!("PhenoAge cell is {other:?}"),
    };
    let want = rust_phenoage();
    assert!((got - want).abs() < 1e-6, "LibreOffice {got} vs Rust {want}");
    assert!((25.0..35.0).contains(&got), "{got}");
}
