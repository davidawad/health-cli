//! W3 supplement views fed by biomarker regimens (hx-ft3.12): `Data_Supp.taking`,
//! `start_date` and `stop_date` follow `biomarker regimen ls` with the store as
//! fallback, and `Sup_Regimen` shows taking vs not plus `interaction grid
//! --catalog`. Synthetic fixtures and stand-in `biomarker` scripts only.

#![cfg(unix)]

mod support;

use std::path::PathBuf;

use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use support::Env;

fn generate(e: &Env) -> (PathBuf, String) {
    e.import();
    let out = e.path("Health Dashboard (generated).xlsx");
    let o = e.cmd().args(["sheet", "--out", out.to_str().unwrap()]).output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    (out, String::from_utf8_lossy(&o.stderr).into_owned())
}

fn sheet(p: &PathBuf, name: &str) -> Range<Data> {
    let mut wb: Xlsx<_> = open_workbook(p).unwrap();
    wb.worksheet_range(name).unwrap()
}

fn t(r: &Range<Data>, row: u32, col: u32) -> String {
    r.get_value((row, col)).map(ToString::to_string).unwrap_or_default()
}

fn col_of(r: &Range<Data>, row: u32, name: &str) -> u32 {
    (0..r.width() as u32).find(|c| t(r, row, *c) == name).unwrap_or_else(|| panic!("no column {name}"))
}

fn row_of(r: &Range<Data>, col: u32, key: &str) -> u32 {
    (0..r.height() as u32).find(|i| t(r, *i, col) == key).unwrap_or_else(|| panic!("no row {key}"))
}

/// Every text cell of a sheet, for "does it say X" checks.
fn text(r: &Range<Data>) -> String {
    r.used_cells().map(|(_, _, v)| v.to_string()).collect::<Vec<_>>().join("\n")
}

/// `Data_Supp` cell of supplement `name`, column `h`.
fn supp(r: &Range<Data>, name: &str, h: &str) -> String {
    t(r, row_of(r, col_of(r, 0, "name"), name), col_of(r, 0, h))
}

#[test]
fn data_supp_follows_biomarker_regimens() {
    let e = Env::new();
    e.write_regimen_stubs();
    let (p, err) = generate(&e);
    assert!(!err.contains("taking status from the store") && !err.contains("no interaction grid"), "{err}");
    let calls = e.biomarker_calls();
    assert!(calls.iter().any(|c| c == "regimen ls -p david --status all --format json"), "{calls:?}");
    assert!(calls.iter().any(|c| c == "interaction grid -p david --catalog --format json"), "{calls:?}");
    let d = sheet(&p, "Data_Supp");
    // The store says YES; biomarker has only a stopped regimen.
    assert_eq!(supp(&d, "Deltamin", "taking"), "NO");
    assert_eq!(supp(&d, "Deltamin", "taking_source"), "biomarker");
    assert_eq!(supp(&d, "Alphazyme", "taking"), "YES");
    // Excel date serials: 2024-01-02 and 2023-06-01.
    assert_eq!(supp(&d, "Alphazyme", "start_date"), "45293");
    assert_eq!(supp(&d, "Betazine", "stop_date"), "45078");
    // No regimen: the store decides and the dates stay blank.
    assert_eq!(supp(&d, "Gammavit", "taking"), "YES");
    assert_eq!(supp(&d, "Gammavit", "taking_source"), "store");
    assert_eq!(supp(&d, "Gammavit", "start_date"), "");
    for v in ["Sup_Churn", "Sup_Cycle"] {
        let s = sheet(&p, v);
        assert!(t(&s, 3, 0).contains("biomarker regimen ls -p david"), "{v}: {}", t(&s, 3, 0));
    }
}

#[test]
fn sup_regimen_shows_taking_vs_not_and_the_grid() {
    let e = Env::new();
    e.write_regimen_stubs();
    let (p, _) = generate(&e);
    let s = sheet(&p, "Sup_Regimen");
    assert!(t(&s, 0, 0).contains("SUP-23"));
    let h = 4;
    for (name, status, store) in
        [("Alphazyme", "taking", "YES"), ("Betazine", "stopped", "NO"), ("Deltamin", "stopped", "YES")]
    {
        let r = h
            + 1
            + ["Alphazyme", "Betazine", "Gammavit", "Deltamin", "Epsilon Oil"].iter().position(|n| *n == name).unwrap()
                as u32;
        assert_eq!(t(&s, r, col_of(&s, h, "biomarker_status")), status, "{name}");
        assert_eq!(t(&s, r, col_of(&s, h, "store_taking")), store, "{name}");
    }
    let dose = col_of(&s, h, "dose");
    assert_eq!(t(&s, h + 1, dose), "500", "biomarker's dose, not the store's 600");
    let all = text(&s);
    assert!(all.contains("Biomarker regimens with no store supplement") && all.contains("Omegatin"), "{all}");
    let g = row_of(&s, 0, "taking \\ with");
    assert_eq!((t(&s, g, 1), t(&s, g, 2), t(&s, g, 3)), ("alphazyme".into(), "omegatin".into(), "synthfarin".into()));
    assert_eq!(t(&s, g + 1, 0), "alphazyme");
    assert_eq!(t(&s, g + 1, 1), "-", "the diagonal is never filled");
    assert_eq!(t(&s, g + 1, 2), "contraindicated");
    assert_eq!(t(&s, g + 1, 3), "caution (drug-class:anticoagulant)");
    assert_eq!(t(&s, g + 2, 1), "", "only listed cells are filled");
    let o = row_of(&s, 2, "tsh");
    assert_eq!((t(&s, o, 0), t(&s, o, 3)), ("omegatin".into(), "lab-interference".into()));
    assert!(!all.contains("PLACEHOLDER"), "{all}");
}

#[test]
fn without_regimen_commands_the_store_decides_and_the_grid_waits() {
    let e = Env::new();
    e.write_stubs(); // biomarker without `regimen` / `interaction`
    let (p, err) = generate(&e);
    assert!(err.contains("taking status from the store"), "{err}");
    let d = sheet(&p, "Data_Supp");
    assert_eq!(supp(&d, "Deltamin", "taking"), "YES");
    for n in ["Alphazyme", "Betazine", "Gammavit", "Deltamin", "Epsilon Oil"] {
        assert_eq!(supp(&d, n, "taking_source"), "store", "{n}");
        assert_eq!(supp(&d, n, "start_date"), "", "{n}");
    }
    let s = sheet(&p, "Sup_Regimen");
    let all = text(&s);
    assert!(all.contains("PLACEHOLDER - awaiting data: biomarker regimens"), "{all}");
    assert!(all.contains("PLACEHOLDER - awaiting data: biomarker interaction grid"), "{all}");
    assert_eq!(t(&s, 5, col_of(&s, 4, "store_taking")), "YES");
    assert_eq!(t(&s, 5, col_of(&s, 4, "biomarker_status")), "");
    assert!(t(&sheet(&p, "Sup_Churn"), 3, 0).contains("GATE-DATES"));
}

#[test]
fn no_labs_skips_biomarker_entirely() {
    let e = Env::new();
    e.write_regimen_stubs();
    e.import();
    let out = e.path("Health Dashboard (generated).xlsx");
    e.run(&["sheet", "--no-labs", "--out", out.to_str().unwrap()]);
    assert!(e.biomarker_calls().is_empty());
    assert_eq!(supp(&sheet(&out, "Data_Supp"), "Deltamin", "taking_source"), "store");
}
