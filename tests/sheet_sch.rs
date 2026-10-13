//! Workstream W7 (SCH): the `Sch_*` sheets and the `Ref_Vax` / `Ref_Screen` rule
//! tables of a generated workbook, built from the synthetic store and the synthetic
//! `biomarker` / `genome` stand-ins, then read back (formulas, tables, names, XML
//! parts) and, when LibreOffice is installed, recalculated after typing a few
//! inputs into the blue cells (a rule override, planned doses, a visit date).

#![cfg(unix)]

mod support;
#[path = "sheet_sch/typing.rs"]
mod typing;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use support::Env;
use typing::{a1, recalc, serial, soffice, type_into, Typed};

/// Every SCH sheet in tab order (SCH-01..14).
const SCH_SHEETS: &[&str] = &[
    "Sch_Vax",
    "Sch_Screen",
    "Sch_Provider",
    "Sch_Monitor",
    "Sch_Retest",
    "Sch_PreDraw",
    "Sch_Dosing",
    "Sch_PillLoad",
    "Sch_Gantt",
    "Sch_Refill",
    "Sch_Chelate",
    "Sch_Morning",
    "Sch_Visit",
    "Sch_POMR",
    "Sch_MedRec",
    "Sch_Low",
];
/// Sheets that need lab data (tblLabs / Calc_Marker / tblPanels).
const NEED_LABS: &[&str] = &["Sch_Monitor", "Sch_Retest", "Sch_PreDraw", "Sch_Visit", "Sch_POMR"];
const KPIS: [&str; 10] = [
    "vax_overdue",
    "vax_next_due",
    "screen_due",
    "provider_overdue",
    "retest_overdue",
    "pills_day",
    "complexity",
    "refill_due",
    "chelate_conflicts",
    "medrec_open",
];

fn generate(e: &Env) -> PathBuf {
    e.write_stubs();
    e.import();
    let out = e.path("Health Dashboard (generated).xlsx");
    e.run(&["sheet", "--out", out.to_str().unwrap()]);
    out
}

type Book = Xlsx<std::io::BufReader<std::fs::File>>;

fn open(p: &Path) -> Book {
    open_workbook(p).expect("generated workbook opens")
}

fn text(r: &Range<Data>, row: u32, col: u32) -> String {
    r.get_value((row, col)).map(ToString::to_string).unwrap_or_default()
}

fn num(r: &Range<Data>, row: u32, col: u32) -> f64 {
    match r.get_value((row, col)) {
        Some(Data::Float(f)) => *f,
        Some(Data::Int(i)) => *i as f64,
        Some(Data::DateTime(d)) => d.as_f64(),
        other => panic!("R{}C{} not a number: {other:?}", row + 1, col + 1),
    }
}

/// Absolute (row, col) of the first cell whose text is `s`.
fn find(r: &Range<Data>, s: &str) -> (u32, u32) {
    let (row, col, _) =
        r.cells().find(|(_, _, v)| matches!(v, Data::String(x) if x == s)).unwrap_or_else(|| panic!("no cell '{s}'"));
    let (r0, c0) = r.start().unwrap_or((0, 0));
    (r0 + row as u32, c0 + col as u32)
}

/// Absolute row of `key` in column `col`, searching below `from`.
fn row_in(r: &Range<Data>, col: u32, key: &str, from: u32) -> u32 {
    (from..r.end().map_or(0, |e| e.0 + 1))
        .find(|i| text(r, *i, col) == key)
        .unwrap_or_else(|| panic!("no '{key}' in column {col}"))
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

fn sheet_part(names: &[String], sheet: &str) -> String {
    let i = names.iter().position(|n| n == sheet).unwrap_or_else(|| panic!("no sheet {sheet}"));
    format!("xl/worksheets/sheet{}.xml", i + 1)
}

fn formulas(wb: &mut Book, sheet: &str) -> String {
    wb.worksheet_formula(sheet).unwrap().cells().map(|(_, _, f)| f.clone()).collect::<Vec<_>>().join("\n")
}

fn all_text(wb: &mut Book, sheet: &str) -> String {
    wb.worksheet_range(sheet).unwrap().cells().map(|(_, _, v)| v.to_string()).collect::<Vec<_>>().join("\n")
}

/// Record a skipped test where the runner asked for it (TEST_SKIP_LOG), and say so on stderr.
fn skip(test: &str, reason: &str) {
    eprintln!("SKIP {test}: {reason}");
    if let Some(p) = std::env::var_os("TEST_SKIP_LOG") {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
            let _ = writeln!(f, "{test}: {reason}");
        }
    }
}

#[test]
fn every_sch_sheet_is_generated_in_order_with_ids_disclaimer_and_banner() {
    let e = Env::new();
    let out = generate(&e);
    let mut wb = open(&out);
    let names = wb.sheet_names();
    let sch: Vec<&String> = names.iter().filter(|n| n.starts_with("Sch_")).collect();
    assert_eq!(sch, SCH_SHEETS);
    assert!(names.contains(&"Ref_Vax".to_string()) && names.contains(&"Ref_Screen".to_string()));
    for s in SCH_SHEETS {
        let t = all_text(&mut wb, s);
        assert!(t.contains("SCH-"), "{s}: names its idea ids");
        assert!(t.contains("not medical advice"), "{s}: carries the disclaimer");
    }
    for s in ["Sch_Monitor", "Sch_PillLoad", "Sch_Gantt", "Sch_Low"] {
        assert!(all_text(&mut wb, s).contains("PLACEHOLDER - awaiting data"), "{s}: placeholder banner");
    }
    assert!(all_text(&mut wb, "Sch_Vax").contains("UNVERIFIED - pending: CDC adult immunization schedule"));
    assert!(all_text(&mut wb, "Sch_Screen").contains("UNVERIFIED - pending: USPSTF"));
    assert!(all_text(&mut wb, "Sch_Chelate").contains("UNVERIFIED - pending: Stockley"));
    let screen = all_text(&mut wb, "Sch_Screen");
    assert!(screen.contains("not a diagnosis") && screen.contains("colon-attia"));
}

#[test]
fn tables_kpis_parameters_and_rule_tables_are_registered() {
    let e = Env::new();
    let out = generate(&e);
    let p = parts(&out);
    let tables: Vec<String> = p
        .iter()
        .filter(|(n, _)| n.starts_with("xl/tables/table"))
        .map(|(_, x)| x.split("displayName=\"").nth(1).unwrap().split('"').next().unwrap().to_string())
        .collect();
    for t in [
        "tblSchVax",
        "tblSchVaxLedger",
        "tblSchVaxLadder",
        "tblSchVaxPlan",
        "tblSchIg",
        "tblSchScreen",
        "tblSchPhys",
        "tblSchProviders",
        "tblSchProviderCal",
        "tblSchMonitorRules",
        "tblSchMonitor",
        "tblSchRetest",
        "tblSchRetestTop",
        "tblSchBiotin",
        "tblSchDraws",
        "tblSchPastDraws",
        "tblSchPrep",
        "tblSchSlotMap",
        "tblSchAbsorb",
        "tblSchDosing",
        "tblSchDosingPrint",
        "tblSchFreq",
        "tblSchPillLoad",
        "tblSchTurnover",
        "tblSchGantt",
        "tblSchRefill",
        "tblSchChelate",
        "tblSchRoutine",
        "tblSchDayPlan",
        "tblSchVisitHelper",
        "tblSchVisitFlags",
        "tblSchQuestions",
        "tblSchPomr",
        "tblSchPomrHelper",
        "tblSchPomrSuggest",
        "tblSchMedRec",
        "tblSchTrips",
        "tblSchTravelChecklist",
        "tblSchStopReview",
        "tblRefVax",
        "tblRefScreen",
    ] {
        assert!(tables.contains(&t.to_string()), "{t} missing");
    }
    let wbx = part(&p, "xl/workbook.xml");
    for k in KPIS {
        assert!(wbx.contains(&format!("definedName name=\"K_SCH_{k}\"")), "K_SCH_{k} not defined");
    }
    for n in ["Sch_Vax_Soon_d", "Sch_Live_Gap_d", "Sch_Biotin_mcg", "Sch_Fast_h", "Sch_Wake_h", "Sch_Beers_age"] {
        assert!(wbx.contains(&format!("definedName name=\"{n}\"")), "{n} not defined");
    }
    assert!(wbx.contains("_xlnm.Print_Area") && wbx.contains("Sch_Dosing!") && wbx.contains("Sch_Visit!"));

    let mut wb = open(&out);
    let r = wb.worksheet_range("Params").unwrap();
    let row = |name: &str| -> Vec<String> {
        let i = (1..r.height() as u32).find(|i| text(&r, *i, 0) == name).unwrap_or_else(|| panic!("no param {name}"));
        (0..6).map(|c| text(&r, i, c)).collect()
    };
    let b = row("Sch_Biotin_mcg");
    assert_eq!((b[1].as_str(), b[3].as_str(), b[4].as_str()), ("5000", "verified", "DIP ch98.md:233"));
    for n in ["Sch_Live_Gap_d", "Sch_Ig_Gap_months", "Sch_Beers_age"] {
        let x = row(n);
        assert_eq!(x[3], "UNVERIFIED", "{n}");
        assert!(x[4].starts_with("pending: "), "{n}: {}", x[4]);
    }
    for (sheet, n) in [("Ref_Vax", 4), ("Ref_Screen", 6)] {
        let t = wb.worksheet_range(sheet).unwrap();
        let heads: Vec<String> = (0..7).map(|c| text(&t, 0, c)).collect();
        assert_eq!(heads, ["id", "rule", "value", "unit", "source", "source_state", "note"], "{sheet}");
        assert_eq!((t.height() - 1) % n, 0, "{sheet}: {n} rows per rule key");
        for i in 1..t.height() as u32 {
            let state = text(&t, i, 5);
            if state == "UNVERIFIED" {
                assert!(text(&t, i, 4).starts_with("pending: "), "{sheet} row {i}");
            }
        }
    }
    let screen = wb.worksheet_range("Ref_Screen").unwrap();
    let attia = (1..screen.height() as u32).find(|i| text(&screen, *i, 1) == "colon-attia").unwrap();
    assert_eq!((text(&screen, attia, 4), text(&screen, attia, 5)), ("OUT ch22.md:31".into(), "verified".into()));
    let k = wb.worksheet_range("Ref_KPI").unwrap();
    assert_eq!((1..k.height() as u32).filter(|i| text(&k, *i, 1) == "SCH").count(), KPIS.len());
}

/// Every BUILDABLE-NOW idea computes with formulas over the data layer.
#[test]
fn buildable_views_compute_with_formulas_over_the_data_layer() {
    let e = Env::new();
    let mut wb = open(&generate(&e));
    let checks: &[(&str, &[&str])] = &[
        // SCH-01 (NS): due dates from dose dates and Ref_Vax intervals.
        (
            "Sch_Vax",
            &["EDATE(M", "Ref_Vax!$B$", "Data_Vax!", "MAXIFS(", "Sch_Live_Gap_d", "Sch_Ig_Gap_months", "COUNTIFS("],
        ),
        // SCH-02 (NS): ages from DOB, rules from Ref_Screen, last lab draw.
        ("Sch_Screen", &["DATEDIF(DOB,Today", "Ref_Screen!", "Data_Labs!", "Data_Phys!", "EDATE(R", "Sex"]),
        // SCH-03: next visit from the last one and the kind's interval.
        ("Sch_Provider", &["Data_Phys!", "Data_Providers!", "EDATE(G", "Sch_Int_dental_m", "EOMONTH(Today"]),
        // SCH-04: follow-up window and first draw inside it.
        ("Sch_Monitor", &["MINIFS(Data_Labs!", "Data_Supp!$S$", "Sch_Mon_lipid_lo_wk", "+7*INDEX("]),
        // SCH-05: n = ((z SD) / delta)^2, inverse-vol cadence, value of information.
        ("Sch_Retest", &["Calc_Marker!", "((RCV_z*G", ")^2", "Sch_Retest_cv_ref/MAX(H", "COUNTIF("]),
        // SCH-06: biotin hold and fasting hours.
        (
            "Sch_PreDraw",
            &["Sch_Biotin_hold_h/24", "Sch_Fast_h", "Data_Panels!", "Data_SuppIngredients!", "SEARCH(\"biotin\""],
        ),
        // SCH-07: slot grid from the slot map, rules by keyword, compact print list.
        ("Sch_Dosing", &["Data_Supp!", "MATCH(D", "SEARCH(", "INDEX(", "MAX($N$"]),
        // SCH-08: doses/week x slots, complexity score.
        ("Sch_PillLoad", &["Sch_Dosing!", "SUMPRODUCT(", "Sch_CX_item", "Sch_CX_slot", "EDATE(A"]),
        // SCH-09: Gantt offsets and durations by kind.
        ("Sch_Gantt", &["Data_Supp!$S$", "Data_Panels!", "Sch_Gantt_panel_d", "E11-C11", "Today"]),
        // SCH-10: run-out, reorder, purchases per month.
        ("Sch_Refill", &["Data_Products!", "365/E", "Sch_Refill_lead_d", "INT((EDATE(", "ROUNDUP("]),
        // SCH-11 (NS): slots of each agent and the smallest clock gap.
        ("Sch_Chelate", &["Sch_Dosing!", "Sch_Slot_morning_h-Sch_Slot_bedtime_h", "COUNTIFS(Sch_Dosing!", "CONFLICT"]),
        // SCH-12: blocking-step offsets, wake/bed anchors, caffeine cut-off.
        (
            "Sch_Morning",
            &["Data_Routine!", "SUMPRODUCT((", "Sch_Wake_h/24", "Caffeine_HL*Sch_Caffeine_HLs", "Data_Supp!"],
        ),
        // SCH-13: SOAP auto-fill and question generator, POMR, medication reconciliation.
        ("Sch_Visit", &["Calc_Marker!", "Latest_Panel", "Ask about", "Data_Daily!", "never measured"]),
        ("Sch_POMR", &["Calc_Marker!", "Data_Supp!", "Sch_POMR_monitor_d", "at goal"]),
        ("Sch_MedRec", &["Data_Supp!", "Sch_MedRec_review_d", "status differs"]),
        // SCH-14 (PH): entry forms that compute once filled.
        ("Sch_Low", &["Sch_Beers_age", "Sch_Travel_lead_d", "DATEDIF(DOB"]),
    ];
    for (sheet, needles) in checks {
        let f = formulas(&mut wb, sheet);
        assert!(f.lines().count() > 20, "{sheet}: only {} formulas", f.lines().count());
        for n in *needles {
            assert!(f.contains(n), "{sheet}: no '{n}' in its formulas");
        }
        assert!(!f.contains("FILTER("), "{sheet}: Excel-native only");
    }
}

#[test]
fn sch_sheets_have_validation_conditional_formats_and_charts() {
    let e = Env::new();
    let out = generate(&e);
    let names = open(&out).sheet_names();
    let p = parts(&out);
    for s in ["Sch_Screen", "Sch_Provider", "Sch_Monitor", "Sch_PreDraw", "Sch_MedRec", "Sch_POMR", "Sch_Low"] {
        assert!(part(&p, &sheet_part(&names, s)).contains("<dataValidation "), "{s}: drop-downs");
    }
    for s in SCH_SHEETS {
        assert!(part(&p, &sheet_part(&names, s)).contains("<conditionalFormatting"), "{s}: conditional formats");
    }
    for s in SCH_SHEETS.iter().filter(|s| !matches!(**s, "Sch_Visit" | "Sch_POMR")) {
        assert!(part(&p, &sheet_part(&names, s)).contains("<drawing "), "{s}: has a chart");
    }
    assert!(p.iter().any(|(n, x)| n.starts_with("xl/comments") && x.contains("Source: DIP ch37.md:451")));
    // Excel 2019+ functions carry their future-function prefix (no #NAME? in Excel).
    let vax = part(&p, &sheet_part(&names, "Sch_Vax"));
    assert!(vax.contains("_xlfn.MAXIFS(") && vax.contains("_xlfn.MINIFS("));
    assert!(!vax.contains(">MAXIFS(") && !vax.contains(",MAXIFS("), "no unprefixed MAXIFS");
}

#[test]
fn lab_free_build_keeps_the_sch_views_that_do_not_need_labs() {
    let e = Env::new();
    e.import(); // no stubs: biomarker/genome are "not installed"
    let out = e.path("lab-less (generated).xlsx");
    let res = e.cmd().args(["sheet", "--out", out.to_str().unwrap()]).output().unwrap();
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    let names = open(&out).sheet_names();
    for s in SCH_SHEETS {
        assert_eq!(names.contains(&s.to_string()), !NEED_LABS.contains(s), "{s}");
    }
    let mut wb = open(&out);
    assert!(!formulas(&mut wb, "Sch_Screen").contains("Data_Labs!"), "no lab references without labs");
    assert!(!formulas(&mut wb, "Sch_Gantt").contains("Data_Panels!"));
}

#[test]
fn only_brings_the_dosing_grid_along_with_its_readers() {
    let e = Env::new();
    e.write_stubs();
    e.import();
    let out = e.path("only (generated).xlsx");
    e.run(&["sheet", "--only", "sch-chelate", "--out", out.to_str().unwrap()]);
    let names = open(&out).sheet_names();
    assert!(names.contains(&"Sch_Chelate".to_string()) && names.contains(&"Sch_Dosing".to_string()));
    assert!(!names.contains(&"Sch_Vax".to_string()));
    assert!(part(&parts(&out), "xl/workbook.xml").contains("definedName name=\"Sch_Slot_bedtime_h\""));
}

// ------------------------------------------------------------ typing + recalculation (helpers: sheet_sch/typing.rs)

/// Synthetic store (tests/support): "Test Vaccine" doses 2000-01-02 and 2000-03-04 (a
/// third "NOTSURE"), "Unvaccinated", "Made-up condition"; DOB 1990-01-15, Sex blank;
/// labs ldl-c 131 (2023-05-01) and 96 (2024-05-01, optimal <= 70), hba1c 5.7 > 5.6
/// (2023-05-01); supplements Alphazyme (YES Daily Day 600 mg), Betazine (NO), Gammavit
/// (YES 2x/wk Day), Deltamin (YES 3/wk "Day & Night" 2), Epsilon Oil (unset); routine
/// morning Workout (async 1800 s), Shampoo (async 30 s), Brush Teeth (blocking 120 s).
#[test]
fn recalculated_sch_views_have_the_expected_numbers() {
    let Some(soffice) = soffice() else {
        skip("sheet_sch::recalculated_sch_views_have_the_expected_numbers", "soffice (LibreOffice) not installed");
        return;
    };
    let e = Env::new();
    let out = generate(&e);
    let mut wb = open(&out);
    let names = wb.sheet_names();
    // Type: the Tdap rule on Test Vaccine, two live vaccines 10 days apart after an
    // immune globulin dose.
    let v = wb.worksheet_range("Sch_Vax").unwrap();
    let (vh, vc) = find(&v, "rule_override");
    let tv = vh + 1; // Test Vaccine is the first vaccine of the store
    let (ph, pc) = find(&v, "planned_date");
    let (_, ic) = find(&v, "product");
    let edits = vec![
        (a1(tv, vc), Typed::S("tdap")),
        (a1(ph + 1, pc), Typed::N(serial(2030, 1, 10))),
        (a1(ph + 1, pc + 1), Typed::S("MMR")),
        (a1(ph + 2, pc), Typed::N(serial(2030, 1, 20))),
        (a1(ph + 2, pc + 1), Typed::S("Varicella")),
        (a1(ph + 1, ic - 1), Typed::N(serial(2029, 12, 1))),
    ];
    let typed = e.path("typed.xlsx");
    type_into(&out, &typed, &sheet_part(&names, "Sch_Vax"), &edits);
    let mut wb = recalc(&soffice, e.dir.path(), &typed);

    let mut errors = Vec::new();
    for s in SCH_SHEETS.iter().chain(&["Ref_Vax", "Ref_Screen"]) {
        let r = wb.worksheet_range(s).unwrap();
        for (row, col, v) in r.cells() {
            if let Data::Error(err) = v {
                errors.push(format!("{s}!R{}C{}: {err:?}", row + 1, col + 1));
            }
        }
    }
    assert!(errors.is_empty(), "formula errors after recalculation: {errors:?}");

    // SCH-01: two dated doses, Tdap booster 120 months after the last one.
    let v = wb.worksheet_range("Sch_Vax").unwrap();
    let (vh, _) = find(&v, "vaccine");
    let col = |name: &str| (0..30).find(|c| text(&v, vh, *c) == name).unwrap();
    let r = row_in(&v, col("vaccine"), "Test Vaccine", vh);
    assert_eq!(num(&v, r, col("doses")), 2.0);
    assert_eq!(num(&v, r, col("last_dose")), serial(2000, 3, 4));
    assert_eq!(text(&v, r, col("rule")), "tdap");
    assert_eq!(num(&v, r, col("booster_m")), 120.0);
    assert_eq!(num(&v, r, col("next_due")), serial(2010, 3, 4), "EDATE(last, 120)");
    assert_eq!(text(&v, r, col("status")), "overdue");
    assert_eq!(text(&v, row_in(&v, col("vaccine"), "Made-up condition", vh), col("status")), "condition");
    assert_eq!(text(&v, row_in(&v, col("vaccine"), "Unvaccinated", vh), col("status")), "no rule");
    let (ph, pc) = find(&v, "planned_date");
    assert_eq!(num(&v, ph + 1, pc + 3), 1.0, "MMR is live (Ref_Vax)");
    assert_eq!(text(&v, ph + 1, pc + 4), "space live vaccines");
    assert_eq!(text(&v, ph + 1, pc + 5), "immune globulin within window");
    assert_eq!(text(&v, ph + 2, pc + 6), "review with clinician");

    // SCH-02: the lipid panel counts the last ldl-c draw; Lp(a) was never measured.
    let s = wb.worksheet_range("Sch_Screen").unwrap();
    let (sh, _) = find(&s, "key");
    let sc = |name: &str| (0..30).find(|c| text(&s, sh, *c) == name).unwrap();
    let lip = row_in(&s, 0, "lipids", sh);
    assert_eq!(num(&s, lip, sc("last_lab")), serial(2024, 5, 1));
    assert_eq!(num(&s, lip, sc("interval_m")), 60.0);
    assert_eq!(text(&s, row_in(&s, 0, "psa", sh), sc("applies")), "no", "Sex blank: sex-specific items off");
    assert_eq!(text(&s, row_in(&s, 0, "colon-attia", sh), sc("framing")), "Attia practice - not guideline");

    // SCH-05: ldl-c 131 -> 96: SD 24.75, CV 0.218, delta 11.35, n = ceil((1.96 x 24.75 / 11.35)^2) = 19.
    let rt = wb.worksheet_range("Sch_Retest").unwrap();
    let (rh, _) = find(&rt, "marker_id");
    let rc = |name: &str| (0..22).find(|c| text(&rt, rh, *c) == name).unwrap();
    let ldl = row_in(&rt, 0, "ldl-c", rh);
    assert_eq!(num(&rt, ldl, rc("draws_needed")), 19.0);
    let cadence = 180.0 * 0.10 / (24.748_737_341_529_164 / 113.5);
    assert!((num(&rt, ldl, rc("cadence_d")) - cadence).abs() < 1e-6, "inverse-vol cadence");
    let (_, tc) = find(&rt, "k");
    assert_eq!(text(&rt, rh + 1, tc + 1), "hba1c", "out of range ranks first");

    // SCH-07/08: grid and burden.
    let d = wb.worksheet_range("Sch_Dosing").unwrap();
    let (gh, _) = find(&d, "supplement");
    let morning = (0..20).find(|c| text(&d, gh, *c) == "morning").unwrap();
    assert_eq!(text(&d, row_in(&d, 0, "Alphazyme", gh), morning), "600 mg");
    assert_eq!(text(&d, row_in(&d, 0, "Betazine", gh), morning), "", "not taking");
    let dm = row_in(&d, 0, "Deltamin", gh);
    assert_eq!((text(&d, dm, morning + 2), text(&d, dm, morning + 3)), ("2".into(), "".into()), "Day & Night: evening");
    let p = wb.worksheet_range("Sch_PillLoad").unwrap();
    let pv = |label: &str| num(&p, row_in(&p, 0, label, 0), 1);
    assert_eq!(pv("Items taken"), 3.0);
    assert!((pv("Pills per day") - (1.0 + 2.0 / 7.0 + 2.0 * 3.0 / 7.0)).abs() < 1e-9);
    assert_eq!(pv("Regimen complexity score"), 7.0, "3 items + 2 non-daily + 2 slots");

    // SCH-10: Alphazyme 3 units/yr at $32.
    let f = wb.worksheet_range("Sch_Refill").unwrap();
    let (fh, _) = find(&f, "item");
    let fc = |name: &str| (0..30).find(|c| text(&f, fh, *c) == name).unwrap();
    let az = row_in(&f, fc("item"), "Alphazyme", fh);
    assert!((num(&f, az, fc("days_per_unit")) - 365.0 / 3.0).abs() < 1e-9);
    assert_eq!(num(&f, az, fc("per_year")), 96.0);
    assert_eq!(text(&f, az, fc("status")), "enter last purchase");

    // SCH-11, 12, 13.
    let c = wb.worksheet_range("Sch_Chelate").unwrap();
    assert_eq!(num(&c, row_in(&c, 0, "Conflicts in the dosing grid", 0), 1), 0.0);
    let m = wb.worksheet_range("Sch_Morning").unwrap();
    assert_eq!(num(&m, row_in(&m, 0, "Morning: planned minutes", 0), 1), 2.0, "only Brush Teeth blocks");
    let vis = wb.worksheet_range("Sch_Visit").unwrap();
    let (fl, _) = find(&vis, "marker");
    assert_eq!(text(&vis, fl + 1, 1), "Hemoglobin A1c");
    assert_eq!(text(&vis, fl + 1, 6), "outside the reference range");
    assert_eq!(text(&vis, fl + 2, 1), "LDL Cholesterol");
    assert!(all_text(&mut wb, "Sch_Visit").contains("Ask about Hemoglobin A1c: 5.7 %"));
    let mr = wb.worksheet_range("Sch_MedRec").unwrap();
    assert_eq!(num(&mr, row_in(&mr, 0, "unreviewed", 0), 1), 5.0);

    let kpi = wb.worksheet_range("Ref_KPI").unwrap();
    let kv = |n: &str| text(&kpi, row_in(&kpi, 0, n, 0), 3);
    assert_eq!(kv("K_SCH_vax_overdue"), "1");
    assert_eq!(kv("K_SCH_chelate_conflicts"), "0");
    assert_eq!(kv("K_SCH_medrec_open"), "0");
}
