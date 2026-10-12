//! Recalculate the generated workbook in headless LibreOffice and fail on any
//! formula error (#NAME?, #REF!, #VALUE!, ...): the build-time Checks run that
//! catches Excel-incompatible formulas. Skipped when `soffice` is not installed.

#![cfg(unix)]

mod support;

use std::process::Command;

use calamine::{open_workbook, Data, Reader, Xlsx};
use support::Env;

fn soffice() -> Option<String> {
    ["soffice", "/Applications/LibreOffice.app/Contents/MacOS/soffice", "/opt/homebrew/bin/soffice"]
        .iter()
        .find(|c| Command::new(c).arg("--version").output().is_ok_and(|o| o.status.success()))
        .map(|c| (*c).to_string())
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

    // A private LibreOffice profile that always recalculates OOXML on load.
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
    let status = Command::new(&soffice)
        .arg(format!("-env:UserInstallation=file://{}", profile.display()))
        .args(["--headless", "--convert-to", "xlsx", "--outdir"])
        .arg(&outdir)
        .arg(&out)
        .output()
        .unwrap();
    assert!(status.status.success(), "{}", String::from_utf8_lossy(&status.stderr));

    let mut wb: Xlsx<_> = open_workbook(outdir.join("Health Dashboard (generated).xlsx")).unwrap();
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
    // And the recalculation really happened: derived values are no longer the cached zeros.
    let d = wb.worksheet_range("Dashboard").unwrap();
    let ideal = d.get_value((3, 2)).map(ToString::to_string).unwrap_or_default();
    assert!(ideal.ends_with(" lb"), "ideal weight was recalculated: {ideal:?}");
}
