//! `health export regimens`: the store's supplements (synthetic fixture workbook)
//! as a plan of `biomarker regimen add` commands, matched against a synthetic
//! research tree, and `--apply` against a stand-in `biomarker` that logs its calls.

#![cfg(unix)]

mod support;

use serde_json::Value;
use support::Env;

fn setup() -> (Env, String) {
    let e = Env::new();
    e.write_regimen_stubs();
    e.import();
    let research = e.write_research().display().to_string();
    (e, research)
}

/// `data` rows of a plan, keyed by supplement name.
fn row<'a>(v: &'a Value, name: &str) -> &'a Value {
    v["data"].as_array().unwrap().iter().find(|r| r["supplement"] == name).unwrap_or_else(|| panic!("no {name}"))
}

fn args(v: &Value) -> Vec<&str> {
    v["argv"].as_array().unwrap().iter().map(|a| a.as_str().unwrap()).collect()
}

#[test]
fn dry_run_plans_every_supplement_and_flags_missing_start_dates() {
    let (e, research) = setup();
    let v = e.json(&["export", "regimens", "-p", "synth", "--research", &research]);
    assert_eq!(v["kind"], "regimen_plan");
    assert_eq!((v["count"].as_u64(), v["dry_run"].as_bool()), (Some(5), Some(true)));
    assert_eq!((v["add"].as_u64(), v["needs_start"].as_u64(), v["skip"].as_u64()), (Some(0), Some(3), Some(2)));
    for name in ["Alphazyme", "Gammavit", "Deltamin"] {
        assert_eq!(row(&v, name)["action"], "needs-start", "{name}");
        assert!(args(row(&v, name)).is_empty());
    }
    assert_eq!(row(&v, "Betazine")["action"], "skip");
    assert_eq!(row(&v, "Epsilon Oil")["slug"], Value::Null);
    assert!(e.biomarker_calls().is_empty(), "a dry run never runs biomarker");
}

#[test]
fn start_fills_taken_rows_and_the_plan_is_deterministic() {
    let (e, research) = setup();
    let a = ["export", "regimens", "-p", "synth", "--research", &research, "--start", "2024-01-02"];
    let v = e.json(&a);
    assert_eq!(v["add"].as_u64(), Some(3));
    assert_eq!(
        args(row(&v, "Alphazyme")),
        [
            "regimen",
            "add",
            "alphazyme",
            "-p",
            "synth",
            "--dose",
            "600",
            "--unit",
            "mg",
            "--frequency",
            "Daily",
            "--slot",
            "Day",
            "--start",
            "2024-01-02"
        ]
    );
    let g = args(row(&v, "Gammavit"));
    assert!(g.ends_with(&["--note", "health-cli amount: 1/2 pill 2x / wk"]), "{g:?}");
    assert_eq!(e.json(&a)["data"], v["data"], "same store, same plan");
}

#[test]
fn the_embedded_research_index_does_not_know_synthetic_names() {
    let (e, _) = setup();
    let v = e.json(&["export", "regimens", "--start", "2024-01-02"]);
    assert_eq!(v["research_slugs"].as_u64(), Some(87));
    assert_eq!(v["person"], "david", "the biomarker_person default");
    assert_eq!(v["skip"].as_u64(), Some(5));
}

#[test]
fn apply_refuses_without_start_dates_and_never_writes_files() {
    let (e, research) = setup();
    let out = e.cmd().args(["export", "regimens", "-p", "synth", "--research", &research, "--apply"]).output().unwrap();
    assert_eq!(out.status.code(), Some(4));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Alphazyme") && err.contains("--start"), "{err}");
    assert!(e.biomarker_calls().is_empty());
    let plan = e.path("plan.json");
    let out = e.cmd().args(["export", "regimens", "-o", plan.to_str().unwrap()]).output().unwrap();
    assert_eq!(out.status.code(), Some(4), "-o is refused");
    assert!(!plan.exists());
    let out = e.cmd().args(["export", "regimens", "--start", "2024-13-01"]).output().unwrap();
    assert_eq!(out.status.code(), Some(4), "a bad --start is refused");
}

#[test]
fn apply_adds_what_biomarker_lacks_and_skips_what_it_holds() {
    let (e, research) = setup();
    let v = e.json(&["export", "regimens", "-p", "synth", "--research", &research, "--start", "2024-01-02", "--apply"]);
    assert_eq!((v["dry_run"].as_bool(), v["applied"].as_u64()), (Some(false), Some(2)));
    // biomarker already holds alphazyme from 2024-01-02 (stub `regimen ls`).
    assert_eq!(row(&v, "Alphazyme")["result"], "exists");
    for name in ["Gammavit", "Deltamin"] {
        assert_eq!((&row(&v, name)["result"], row(&v, name)["regimen_id"].as_i64()), (&Value::from("added"), Some(99)));
    }
    assert_eq!(row(&v, "Betazine").get("result"), None);
    let calls = e.biomarker_calls();
    assert_eq!(calls[0], "regimen ls -p synth --status all --format json");
    let adds: Vec<&String> = calls.iter().filter(|c| c.starts_with("regimen add")).collect();
    assert_eq!(adds.len(), 2, "{calls:?}");
    assert!(adds[0].starts_with("regimen add gammavit -p synth") && adds[0].ends_with("--format json"));
    assert!(adds[1].starts_with("regimen add deltamin -p synth --dose 2 --frequency 3/wk --slot Day & Night"));
    assert!(calls.iter().all(|c| !c.split(' ').any(|a| a == "-o" || a == "--output")));
}

#[test]
fn apply_stops_on_a_biomarker_failure() {
    let e = Env::new();
    e.write_stubs(); // a biomarker without the regimen commands
    e.import();
    let research = e.write_research().display().to_string();
    let out = e
        .cmd()
        .args(["export", "regimens", "-p", "synth", "--research", &research, "--start", "2024-01-02", "--apply"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("regimen ls"));
}
