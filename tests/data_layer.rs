//! The shared data layer (`Ref_*`, `Data_*`, `Calc_*`) of a generated workbook.

#![cfg(unix)]

mod support;

use std::io::Read;
use std::path::{Path, PathBuf};

use calamine::{open_workbook, Data, Reader, Xlsx};
use support::Env;

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

fn t(r: &calamine::Range<Data>, row: u32, col: u32) -> String {
    r.get_value((row, col)).map(ToString::to_string).unwrap_or_default()
}

fn header(r: &calamine::Range<Data>) -> Vec<String> {
    (0..r.width() as u32).map(|c| t(r, 0, c)).collect()
}

/// Rows of a sheet as `header -> value` lookups.
fn col_of(r: &calamine::Range<Data>, name: &str) -> u32 {
    header(r).iter().position(|h| h == name).unwrap_or_else(|| panic!("no column {name} in {:?}", header(r))) as u32
}

fn find(r: &calamine::Range<Data>, key_col: &str, key: &str) -> u32 {
    let c = col_of(r, key_col);
    (1..r.height() as u32).find(|i| t(r, *i, c) == key).unwrap_or_else(|| panic!("no row {key}"))
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

#[test]
fn every_planned_table_exists_with_a_stable_name() {
    let e = Env::new();
    let p = parts(&generate(&e));
    let names: Vec<String> = p
        .iter()
        .filter(|(n, _)| n.starts_with("xl/tables/table"))
        .map(|(_, x)| x.split("displayName=\"").nth(1).unwrap().split('"').next().unwrap().to_string())
        .collect();
    for t in [
        "tblLabs",
        "tblMarkers",
        "tblPanels",
        "tblDaily",
        "tblWearable",
        "tblSupp",
        "tblProducts",
        "tblNutrition",
        "tblRoutine",
        "tblWorkout",
        "tblVax",
        "tblPhys",
        "tblSizes",
        "tblResources",
        "tblEvidence",
        "tblGenome",
        "tblProfile",
        "tblSuppTags",
        "tblRegimes",
        "tblSuppIngredients",
        "tblRefSystems",
        "tblRefUnits",
        "tblRefStance",
        "tblRefTimeSlots",
        "tblRefHallmarks",
        "tblRefSENS",
        "tblRefMechanisms",
        "tblRefDirection",
    ] {
        assert!(names.iter().any(|n| n == t), "{t} missing from {names:?}");
    }
    let mut s = names.clone();
    s.sort();
    s.dedup();
    assert_eq!(s.len(), names.len(), "unique table names");
}

#[test]
fn labs_markers_and_panels_carry_the_biomarker_data() {
    let e = Env::new();
    let mut wb = open(&generate(&e));
    let l = wb.worksheet_range("Data_Labs").unwrap();
    assert_eq!(
        header(&l),
        ["panel_date", "panel_id", "marker_id", "value_raw", "unit_raw", "value", "lab", "method", "fasting"]
    );
    assert_eq!(l.height(), 1 + 9, "every measurement of the stub");
    assert_eq!(t(&l, 1, 2), "hba1c", "ordered by date, then marker");
    let ldl = find(&l, "marker_id", "ldl-c");
    assert_eq!(t(&l, ldl, 1), "P20230501");
    assert_eq!(t(&l, ldl, 5), "131");
    let weight = find(&l, "marker_id", "weight");
    assert_eq!(t(&l, weight, 1), "", "body measurements are not lab panels");

    let m = wb.worksheet_range("Data_Markers").unwrap();
    assert_eq!(
        header(&m),
        [
            "marker_id",
            "name",
            "system",
            "unit",
            "ref_lo",
            "ref_hi",
            "opt_lo",
            "opt_hi",
            "blueprint_lo",
            "blueprint_hi",
            "dir",
            "critical_lo",
            "critical_hi",
            "weight",
            "cvi",
            "cva"
        ]
    );
    let ids: Vec<String> = (1..m.height() as u32).map(|i| t(&m, i, 0)).collect();
    for want in ["wbc", "ldl-c", "weight", "biological-age", "uacr"] {
        assert!(ids.iter().any(|i| i == want), "{want} in {ids:?}");
    }
    let r = find(&m, "marker_id", "ldl-c");
    assert_eq!(
        (t(&m, r, 2).as_str(), t(&m, r, 5).as_str(), t(&m, r, 7).as_str(), t(&m, r, 10).as_str()),
        ("Lipids", "99", "70", "hi")
    );
    let r = find(&m, "marker_id", "hdl-c");
    assert_eq!((t(&m, r, 4).as_str(), t(&m, r, 10).as_str()), ("39", "lo"));
    assert_eq!(t(&m, r, 8), "", "a personal range shadows the blueprint set: blank");
    let (bl, bh) = (col_of(&m, "blueprint_lo"), col_of(&m, "blueprint_hi"));
    let r = find(&m, "marker_id", "ldl-c");
    assert_eq!((t(&m, r, bl), t(&m, r, bh)), (String::new(), "60".into()), "blueprint from `profile show`");
    let r = find(&m, "marker_id", "hba1c");
    assert_eq!((t(&m, r, bl), t(&m, r, bh)), ("4.5".into(), "5.6".into()));
    let r = find(&m, "marker_id", "uacr");
    assert_eq!(t(&m, r, 2), "Renal");
    assert_eq!(t(&m, r, 11), "", "critical bounds stay blank unless sourced");

    let p = wb.worksheet_range("Data_Panels").unwrap();
    assert_eq!(header(&p), ["panel_id", "date", "age_at_draw", "lab", "fasting", "weight_at_panel", "n_markers"]);
    assert_eq!((t(&p, 1, 0), t(&p, 2, 0)), ("P20230501".into(), "P20240501".into()));
    assert_eq!(p.height(), 3);
    let pf = wb.worksheet_formula("Data_Panels").unwrap();
    assert_eq!(pf.get_value((1, 6)).unwrap(), "COUNTIFS(Data_Labs!$B$2:$B$10,A2)");
    assert!(pf.get_value((1, 2)).unwrap().contains("DOB"));
}

#[test]
fn store_tables_mirror_the_store() {
    let e = Env::new();
    let mut wb = open(&generate(&e));
    let s = wb.worksheet_range("Data_Supp").unwrap();
    assert_eq!(s.height(), 6);
    let a = find(&s, "name", "Alphazyme");
    assert_eq!(t(&s, a, col_of(&s, "taking")), "YES");
    assert_eq!(t(&s, a, col_of(&s, "sources")), "Test Protocol, Box One");
    assert_eq!(t(&s, find(&s, "name", "Epsilon Oil"), col_of(&s, "taking")), "", "unknown stays blank");
    assert_eq!(t(&s, a, col_of(&s, "start_date")), "", "dates are not tracked yet");
    let vax = wb.worksheet_range("Data_Vax").unwrap();
    assert_eq!(vax.height(), 1 + 3 + 1 + 1, "one row per dose, one for a vaccine without doses, one condition");
    let raw = (1..vax.height() as u32).find(|i| t(&vax, *i, col_of(&vax, "raw")) == "NOTSURE");
    assert!(raw.is_some());
    let d = wb.worksheet_range("Data_Daily").unwrap();
    assert_eq!(header(&d), ["date", "weight", "kcal", "protein_g", "energy_out_kcal", "other"]);
    assert_eq!((t(&d, 1, 1), t(&d, 1, 2), t(&d, 1, 3)), ("190".into(), "1500".into(), "120".into()));
    let g = wb.worksheet_range("Data_SuppTags").unwrap();
    assert_eq!((g.height(), g.width()), (6, 31), "one blank tagging row per supplement");
    assert_eq!(wb.worksheet_range("Data_Wearable").unwrap().width(), 9);
    let prof = wb.worksheet_formula("Data_Profile").unwrap();
    assert!(prof.get_value((1, 2)).unwrap().contains("Height_cm"));
}

#[test]
fn ref_tables_hold_the_lookups() {
    let e = Env::new();
    let mut wb = open(&generate(&e));
    let u = wb.worksheet_range("Ref_Units").unwrap();
    assert_eq!(header(&u), ["marker_id", "unit_raw", "factor", "canonical_unit"]);
    let r = find(&u, "marker_id", "ldl-c");
    assert_eq!((t(&u, r, 2), t(&u, r, 3)), ("1".into(), "mg/dL".into()));
    let s = wb.worksheet_range("Ref_Stance").unwrap();
    assert_eq!(
        (t(&s, find(&s, "stance", "supporting"), 1), t(&s, find(&s, "stance", "warning"), 1)),
        ("1".into(), "-1".into())
    );
    let h = wb.worksheet_range("Ref_Hallmarks").unwrap();
    assert_eq!(h.height(), 10, "nine hallmarks");
    assert_eq!(t(&h, 1, 3), "verified");
    assert!(t(&h, 1, 2).contains("doi:10.1016/j.cell.2013.05.039"), "Lopez-Otin 2013");
    let sens = wb.worksheet_range("Ref_SENS").unwrap();
    assert_eq!(sens.height(), 8);
    assert_eq!(t(&sens, 7, 3), "verified");
    assert!(t(&sens, 7, 2).contains("doi:10.1111/j.1749-6632.2002.tb02115.x"), "de Grey 2002");
    let sys = wb.worksheet_range("Ref_Systems").unwrap();
    assert_eq!(t(&sys, find(&sys, "marker_id", "wbc"), 1), "Hematology");
    assert_eq!(wb.worksheet_range("Ref_Direction").unwrap().height(), 4);
}

#[test]
fn calc_layer_is_formulas_over_the_data_tables() {
    let e = Env::new();
    let p = generate(&e);
    let mut wb = open(&p);
    let f = wb.worksheet_formula("Calc_Marker").unwrap();
    let m = wb.worksheet_range("Data_Markers").unwrap();
    let r = find(&m, "marker_id", "ldl-c");
    assert_eq!(f.get_value((r, 1)).unwrap(), &format!("COUNTIFS(Data_Labs!$C$2:$C$10,$A{})", r + 1));
    assert!(f.get_value((r, 2)).unwrap().contains("MINIFS"));
    let cl = wb.worksheet_formula("Calc_Labs").unwrap();
    assert!(
        cl.get_value((1, 10)).unwrap().contains("INDEX(Data_Markers!$L$2:$L$9,$D2)"),
        "{:?}",
        cl.get_value((1, 10))
    );
    let cp = wb.worksheet_formula("Calc_Panel").unwrap();
    assert_eq!(cp.get_value((1, 3)).unwrap(), "COUNTIFS(Data_Labs!$B$2:$B$10,$A2)");
    let x = parts(&p);
    let wbx = &x.iter().find(|(n, _)| n == "xl/workbook.xml").unwrap().1;
    for n in ["N_Panels", "Latest_Panel", "Prev_Panel", "K_DATA_n_panels"] {
        assert!(wbx.contains(&format!("definedName name=\"{n}\"")), "{n}");
    }
}

#[test]
fn no_labs_means_no_lab_tables_but_the_rest_is_there() {
    let e = Env::new();
    e.import(); // no stub tools
    let out = e.path("lab-less (generated).xlsx");
    e.run(&["sheet", "--out", out.to_str().unwrap()]);
    let names = open(&out).sheet_names();
    for gone in ["Data_Labs", "Data_Markers", "Data_Panels", "Calc_Labs", "Calc_Marker", "Calc_Panel"] {
        assert!(!names.contains(&gone.to_string()), "{gone}");
    }
    for kept in ["Data_Supp", "Data_Daily", "Ref_Stance", "Data_Profile"] {
        assert!(names.contains(&kept.to_string()), "{kept}");
    }
}
