//! GLB sheets recalculated in headless LibreOffice: no formula errors, Checks says
//! ALL CHECKS PASS on the synthetic store, and the numbers match hand-computed values.
//! A shock is typed into a blue Scenario cell first, as a person would.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use calamine::{Data, Reader};

use super::{generate, open, right_of, row_of, sheet_part, skip, t, Book, GLB_SHEETS};
use crate::support::Env;

fn soffice() -> Option<String> {
    ["soffice", "/Applications/LibreOffice.app/Contents/MacOS/soffice", "/opt/homebrew/bin/soffice"]
        .iter()
        .find(|c| Command::new(c).arg("--version").output().is_ok_and(|o| o.status.success()))
        .map(|c| (*c).to_string())
}

/// Copy `src` to `dst` with numbers typed into blank cells (`B7`) of the worksheet part `xml`.
fn type_into(src: &Path, dst: &Path, xml: &str, edits: &[(String, f64)]) {
    let mut z = zip::ZipArchive::new(std::fs::File::open(src).unwrap()).unwrap();
    let mut w = zip::ZipWriter::new(std::fs::File::create(dst).unwrap());
    for i in 0..z.len() {
        let mut f = z.by_index(i).unwrap();
        let name = f.name().to_string();
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).unwrap();
        if name == xml {
            let mut s = String::from_utf8(buf).unwrap();
            for (cell, v) in edits {
                let open = format!("<c r=\"{cell}\"");
                let at = s.find(&open).unwrap_or_else(|| panic!("{xml}: no cell {cell}"));
                let end = at + s[at..].find("/>").unwrap() + 2;
                let attrs = s[at + open.len()..end - 2].to_string();
                assert!(!attrs.contains('>'), "{cell} is not a blank cell");
                s.replace_range(at..end, &format!("{open}{attrs}><v>{v}</v></c>"));
            }
            buf = s.into_bytes();
        }
        w.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
        w.write_all(&buf).unwrap();
    }
    w.finish().unwrap();
}

fn recalc(soffice: &str, dir: &Path, input: &Path) -> PathBuf {
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
    let res = Command::new(soffice)
        .arg(format!("-env:UserInstallation=file://{}", profile.display()))
        .args(["--headless", "--convert-to", "xlsx", "--outdir"])
        .arg(&outdir)
        .arg(input)
        .output()
        .unwrap();
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    outdir.join(input.file_name().unwrap())
}

fn num(r: &calamine::Range<Data>, row: u32, col: u32) -> f64 {
    match r.get_value((row, col)) {
        Some(Data::Float(f)) => *f,
        Some(Data::Int(i)) => *i as f64,
        other => panic!("({row},{col}) is {other:?}"),
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn glb_views_recalculate_with_the_expected_numbers() {
    let Some(so) = soffice() else {
        skip("glb_views_recalculate_with_the_expected_numbers", "soffice (LibreOffice) not installed");
        return;
    };
    let e = Env::new();
    let out = generate(&e);
    // Scenario: type a -10 % shock on HbA1c (5.7 -> 5.13, back inside 4.8-5.6).
    let mut wb = open(&out);
    let m = wb.worksheet_range("Data_Markers").unwrap();
    let a1c = row_of(&m, 0, "hba1c");
    let sc = wb.worksheet_range("Scenario").unwrap();
    let head = row_of(&sc, 0, "marker_id");
    let typed = e.path("typed (generated).xlsx");
    type_into(&out, &typed, &sheet_part(&out, "Scenario"), &[(format!("D{}", head + a1c + 1), -10.0)]);
    let mut wb: Book = calamine::open_workbook(recalc(&so, e.dir.path(), &typed)).unwrap();

    let mut errors = Vec::new();
    for s in GLB_SHEETS {
        for (row, col, v) in wb.worksheet_range(s).unwrap().cells() {
            if let Data::Error(err) = v {
                let deliberate = *s == "Dash_Trend" && matches!(err, calamine::CellErrorType::NA);
                if !deliberate {
                    errors.push(format!("{s}!R{}C{}: {err:?}", row + 1, col + 1));
                }
            }
        }
    }
    assert!(errors.is_empty(), "formula errors after recalculation: {errors:?}");

    // GLB-02: the whole workbook passes its checks on the synthetic store.
    let c = wb.worksheet_range("Checks").unwrap();
    assert_eq!(t(&c, 3, 0), "ALL CHECKS PASS");
    assert_eq!(num(&c, row_of(&c, 0, "E01"), 3), 0.0, "no formula error in any sheet");
    assert_eq!(num(&c, row_of(&c, 0, "R03"), 3), 0.0, "Calc_Marker reconciles with tblLabs");
    assert!(num(&c, row_of(&c, 0, "V03"), 3) > 0.0, "PH / NS views counted from Nav");

    // GLB-03: HbA1c 5.7 (ref 4.8-5.6) is an action alert, LDL-C 96 (optimal <= 70) a watch.
    let a = wb.worksheet_range("Alerts").unwrap();
    assert_eq!(right_of(&a, "Watch (left optimal)"), "1");
    assert_eq!(right_of(&a, "Action (out of reference)"), "1");
    assert_eq!(right_of(&a, "Alerts (action + critical)"), "1");
    let list = row_of(&a, 13, "k");
    assert_eq!((t(&a, list + 1, 14).as_str(), t(&a, list + 1, 15).as_str()), ("Hemoglobin A1c", "action"));
    assert_eq!((t(&a, list + 2, 14).as_str(), t(&a, list + 2, 15).as_str()), ("LDL Cholesterol", "watch"));

    // GLB-01: index = 100 x (0.5 x (1 - 0.2) + 0.5 x 0.6) = 70.
    let d = wb.worksheet_range("Dash").unwrap();
    let hi = row_of(&d, 0, "Health index (0-100)");
    assert!(close(num(&d, hi + 1, 0), 70.0), "{}", t(&d, hi + 1, 0));
    assert_eq!(t(&d, hi + 3, 1), "amber", "70 is between Glb_Index_amber and Glb_Index_green");
    assert!(close(num(&d, hi + 1, 3), 0.6));
    assert_eq!(t(&d, hi + 1, 6), "1", "alerts tile reads K_GLB_alerts");

    // GLB-04: LDL-C 131 -> 96 resolved its out-of-reference alert.
    let ch = wb.worksheet_range("Changes").unwrap();
    assert_eq!(right_of(&ch, "Latest panel id"), "P20240501");
    assert_eq!(right_of(&ch, "Previous panel id"), "P20230501");
    assert_eq!(right_of(&ch, "Resolved alerts"), "1");
    let ldl = row_of(&ch, 0, "ldl-c");
    assert!(close(num(&ch, ldl, 4), -35.0));
    assert!(close(num(&ch, ldl, 6), -35.0 / (35.0f64 / 2f64.sqrt())), "change in personal SDs");
    assert!(
        ch.cells().any(|(_, _, v)| matches!(v, Data::String(s) if s == "LDL Cholesterol")),
        "TEXTJOIN of resolved names"
    );

    // GLB-05: the typed shock brings HbA1c back into its range.
    let s = wb.worksheet_range("Scenario").unwrap();
    let r = row_of(&s, 0, "hba1c");
    assert!(close(num(&s, r, 5), 5.13));
    assert_eq!((t(&s, r, 14).as_str(), t(&s, r, 15).as_str(), t(&s, r, 16).as_str()), ("2", "0", "YES"));
    assert_eq!(right_of(&s, "Out of reference after"), "0");
    assert_eq!(right_of(&s, "Statuses that flip"), "1");

    // GLB-06: per-year medians and deltas.
    let y = wb.worksheet_range("YearReview").unwrap();
    let (y23, y24) = (row_of(&y, 0, "2023"), row_of(&y, 0, "2024"));
    assert_eq!(num(&y, y23, 7), 131.0, "LDL-C median 2023");
    assert_eq!(num(&y, y24, 8), -35.0, "LDL-C delta 2024");
    assert!(t(&y, y23, 21).starts_with("In 2023: 1 lab panel(s)"), "{}", t(&y, y23, 21));

    // GLB-07: DOB redacted, the latest panel exported.
    let x = wb.worksheet_range("Export").unwrap();
    assert_eq!(t(&x, row_of(&x, 1, "date of birth"), 7), "[redacted]");
    let ldl_line = (0..x.height() as u32).map(|r| t(&x, r, 8)).find(|l| l.starts_with("latest panel | LDL")).unwrap();
    assert_eq!(ldl_line, "latest panel | LDL Cholesterol: 96 mg/dL (2024-05-01); in reference");

    // GLB-08: Nav summary counts every sheet; Sources counts the lab rows.
    let nav = wb.worksheet_range("Nav").unwrap();
    assert_eq!(right_of(&nav, "All sheets"), wb.sheet_names().len().to_string());
    let src = wb.worksheet_range("Sources").unwrap();
    let labs = row_of(&src, 0, "tblLabs");
    assert_eq!((t(&src, labs, 3).as_str(), t(&src, labs, 6).as_str()), ("9", "present"));
    assert_eq!(t(&src, row_of(&src, 0, "tblWearable"), 6), "awaiting data");
}
