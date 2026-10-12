//! End-to-end: `health import xlsx` on a synthetic workbook.

mod support;

use support::Env;

use predicates::prelude::*;
use serde_json::Value;
use sha2::{Digest, Sha256};

fn sheet<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["data"].as_array().unwrap().iter().find(|s| s["sheet"] == name).unwrap_or_else(|| panic!("no sheet {name}"))
}

fn sha(p: &std::path::Path) -> Vec<u8> {
    Sha256::digest(std::fs::read(p).unwrap()).to_vec()
}

#[test]
fn import_counts_match_the_workbook() {
    let e = Env::new();
    let r = e.import();
    let check = |name: &str, seen: u64, items: u64, headers: u64, derived: u64, other: u64| {
        let s = sheet(&r, name);
        let got =
            [&s["rows_seen"], &s["items"], &s["headers"], &s["derived"], &s["other"]].map(|v| v.as_u64().unwrap());
        assert_eq!(got, [seen, items, headers, derived, other], "{name}");
        // Every non-blank row is accounted for: stored, a heading, or derived.
        // (Physicals also stores doctors from a side block, which are not counted rows.)
        if name != "Physicals" {
            assert_eq!(seen, items + headers + derived + other, "{name} rows add up");
        }
    };
    check("Supplements", 5, 5, 0, 0, 0);
    check("Products", 8, 3, 2, 2, 1);
    check("Nutrition", 7, 2, 2, 1, 2);
    check("Routine", 8, 5, 1, 2, 0);
    check("Workout", 3, 2, 0, 0, 1);
    check("Vaccines  Conditions", 3, 3, 0, 0, 0);
    check("Physicals", 7, 5, 2, 0, 2);
    check("Sizes", 4, 2, 2, 0, 0);
    check("Resources", 2, 2, 0, 0, 0);
    let skipped = r["skipped"].to_string();
    assert!(
        skipped.contains("Biomarkers") && skipped.contains("Genetics") && skipped.contains("Weight & Age"),
        "{skipped}"
    );
    assert!(!r["data"].to_string().contains("Biomarkers"));
}

#[test]
fn stores_inputs_not_derived_values() {
    let e = Env::new();
    e.import();
    // Base facts: height/blood group/birthdate are inputs; weight and age are formulas.
    let facts = e.json(&["list", "facts"]);
    let keys: Vec<&str> = facts["data"].as_array().unwrap().iter().map(|f| f["key"].as_str().unwrap()).collect();
    assert_eq!(
        keys,
        ["height", "blood-group", "paternal-haplogroup", "ideal-bmi", "birthdate", "avg-calories-burned-per-day"]
    );
    assert_eq!(facts["data"][0]["unit"], "in");
    assert_eq!(facts["data"][4]["kind"], "date");
    assert_eq!(facts["data"][4]["value"], "1990-01-15");
    // Products: section headings became categories; Total/Supplements rows are gone.
    let products = e.json(&["list", "products"]);
    let names: Vec<(&str, &str)> = products["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["category"].as_str().unwrap(), p["name"].as_str().unwrap()))
        .collect();
    assert_eq!(names, [("Skin & Hair", "Shampoo"), ("Skin & Hair", "Cologne"), ("Oral Health", "Toothpaste")]);
    assert_eq!(products["data"][0]["price_currency"], "USD");
    assert!(products["data"][0]["price_date"].is_string(), "prices carry a date");
    // A formula that is itself an input (=365 * 0.5 / 4) is kept as its value.
    assert_eq!(products["data"][2]["units_per_year"], 45.625);
    // Nutrition: Total Fat is a formula for the bowl (not stored), 9.5 unsat is an input formula.
    let nut = e.json(&["list", "nutrition"]);
    assert_eq!(nut["data"][0]["unsat_fat"], 9.5);
    assert!(nut["data"][0]["total_fat"].is_null());
    assert_eq!(nut["data"][1]["total_fat"], 5.0);
    assert_eq!(nut["data"][1]["category"], "Snacks");
    assert_eq!(nut["data"][1]["price"], 3.5);
    let targets = e.json(&["list", "targets"]);
    assert_eq!(targets["data"][0]["calories"], 2000.0);
    assert!(targets["data"][1]["calories"].is_null(), "derived goal is not stored");
    assert_eq!(targets["data"][1]["fiber"], 28.0);
}

#[test]
fn supplements_split_sources_and_doses() {
    let e = Env::new();
    e.import();
    let sups = e.json(&["list", "supplements"]);
    let by = |n: &str| sups["data"].as_array().unwrap().iter().find(|s| s["name"] == n).unwrap().clone();
    assert_eq!(by("Alphazyme")["sources"], "Test Protocol, Box One");
    assert_eq!(by("Gammavit")["sources"], "Box One, Test Protocol", "case-insensitive source names merge");
    assert_eq!(by("Gammavit")["amount_text"], "1/2 pill");
    assert!(by("Gammavit")["amount"].is_null());
    assert_eq!(by("Betazine")["taking"], 0);
    assert_eq!(by("Betazine")["price_note"], "rx only");
    assert!(by("Betazine")["price"].is_null());
    assert!(by("Epsilon Oil")["taking"].is_null(), "blank Taking stays unknown");
    assert!(by("Alphazyme")["research_url"].as_str().unwrap().contains("alphazyme"));
    let sources = e.json(&["list", "sources"]);
    assert_eq!(sources["count"], 4);
    let ev = e.json(&["list", "evidence"]);
    assert_eq!(ev["count"], 1);
    assert_eq!(ev["data"][0]["stance"], "neutral");
    assert!(ev["data"][0]["kd_work_id"].is_null());
}

#[test]
fn small_sheets_land_in_their_tables() {
    let e = Env::new();
    e.import();
    let routine = e.json(&["list", "routine"]);
    let rows: Vec<(&str, &str)> = routine["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["routine"].as_str().unwrap(), r["name"].as_str().unwrap()))
        .collect();
    assert_eq!(
        rows,
        [
            ("morning", "Workout"),
            ("morning", "Shampoo"),
            ("morning", "Brush Teeth"),
            ("night", "Dinner"),
            ("night", "Floss")
        ]
    );
    let vac = e.json(&["list", "vaccines"]);
    assert_eq!(vac["data"][0]["name"], "Test Vaccine");
    assert_eq!(vac["data"][2]["kind"], "condition");
    let doses = e.json(&["list", "vaccine-doses"]);
    assert_eq!(doses["count"], 3);
    assert_eq!(doses["data"][2]["raw"], "NOTSURE");
    assert!(doses["data"][2]["given_on"].is_null());
    let phys = e.json(&["list", "physicals"]);
    assert_eq!(phys["data"][2]["detail"], "(Testosterone)");
    assert_eq!(phys["data"][3]["section"], "Referrals");
    assert_eq!(phys["data"][3]["done"], 1);
    let docs = e.json(&["list", "providers"]);
    assert_eq!(docs["count"], 2);
    let sizes = e.json(&["list", "sizes"]);
    assert_eq!(sizes["data"][1]["unit"], "us");
    assert_eq!(sizes["data"][0]["category"], "JACKETS");
    let res = e.json(&["list", "resources"]);
    assert_eq!(res["count"], 3, "two resources plus the workout link");
    assert_eq!(e.json(&["list", "workout"])["count"], 2);
}

#[test]
fn the_original_workbook_is_never_modified() {
    let e = Env::new();
    let wb = e.workbook();
    let before = (sha(&wb), std::fs::metadata(&wb).unwrap().modified().unwrap());
    e.import();
    e.run(&["import", "xlsx", wb.to_str().unwrap(), "--replace"]);
    assert_eq!(before, (sha(&wb), std::fs::metadata(&wb).unwrap().modified().unwrap()));
}

#[test]
fn reimport_needs_replace_and_is_idempotent() {
    let e = Env::new();
    e.import();
    let before = e.json(&["stats"]);
    let wb = e.workbook();
    e.cmd()
        .args(["import", "xlsx", wb.to_str().unwrap()])
        .assert()
        .failure()
        .code(4)
        .stderr(predicate::str::contains("--replace"));
    e.run(&["import", "xlsx", wb.to_str().unwrap(), "--replace"]);
    assert_eq!(before["data"], e.json(&["stats"])["data"]);
}

#[test]
fn rejects_files_that_are_not_a_dashboard() {
    let e = Env::new();
    let junk = e.path("junk.xlsx");
    std::fs::write(&junk, b"not a workbook").unwrap();
    e.cmd().args(["import", "xlsx", junk.to_str().unwrap()]).assert().failure();
    e.cmd().args(["import", "xlsx", e.path("missing.xlsx").to_str().unwrap()]).assert().failure().code(3);
}

#[test]
fn the_database_is_encrypted_and_doctor_is_green() {
    let e = Env::new();
    e.import();
    let raw = std::fs::read(e.db()).unwrap();
    assert!(!raw.windows(9).any(|w| w == b"Alphazyme"), "plaintext leaked into the database file");
    assert!(!raw.starts_with(b"SQLite format 3"));
    let doctor = e.json(&["doctor"]);
    let bad: Vec<&Value> = doctor["data"].as_array().unwrap().iter().filter(|c| c["status"] == "fail").collect();
    assert!(bad.is_empty(), "{bad:?}");
    e.cmd().args(["db", "check"]).assert().success();
    let log = e.json(&["audit", "log"]);
    assert!(log["data"].as_array().unwrap().iter().any(|r| r["command"] == "import xlsx"), "{log}");
}
