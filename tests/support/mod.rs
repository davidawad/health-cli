//! Synthetic Health Dashboard workbook for the tests. Every value is made
//! up; the sheet shapes (section headings, formula columns, Google
//! `__xludf.DUMMYFUNCTION` wrappers, the double space in the vaccines sheet
//! name) mirror the real workbook so the importer is exercised on the same
//! structures. Never commit a real workbook or real personal data.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

use rust_xlsxwriter::{ExcelDateTime, Format, Formula, Workbook, Worksheet};

fn s(ws: &mut Worksheet, r: u32, c: u16, v: &str) {
    ws.write_string(r, c, v).unwrap();
}

fn n(ws: &mut Worksheet, r: u32, c: u16, v: f64) {
    ws.write_number(r, c, v).unwrap();
}

/// A formula cell with the cached result a spreadsheet would store.
fn f(ws: &mut Worksheet, r: u32, c: u16, formula: &str, result: &str) {
    ws.write_formula(r, c, Formula::new(formula).set_result(result)).unwrap();
}

fn d(ws: &mut Worksheet, r: u32, c: u16, y: u16, m: u8, day: u8) {
    let fmt = Format::new().set_num_format("yyyy-mm-dd");
    ws.write_datetime_with_format(r, c, ExcelDateTime::from_ymd(y, m, day).unwrap(), &fmt).unwrap();
}

fn row(ws: &mut Worksheet, r: u32, cells: &[&str]) {
    cells.iter().enumerate().for_each(|(c, v)| {
        if !v.is_empty() {
            s(ws, r, c as u16, v);
        }
    });
}

pub fn write_workbook(path: &Path) {
    let mut wb = Workbook::new();

    let ws = wb.add_worksheet().set_name("Dashboard").unwrap();
    s(ws, 0, 0, "Base Stats");
    s(ws, 0, 1, "Height (in.)");
    n(ws, 0, 2, 70.0);
    s(ws, 0, 4, "Blood Group");
    s(ws, 0, 5, "O");
    s(ws, 0, 7, "Paternal Haplogroup");
    s(ws, 0, 8, "R1b");
    s(ws, 1, 1, "Weight (lb.)");
    f(
        ws,
        1,
        2,
        r#"=IFERROR(__xludf.DUMMYFUNCTION("INDEX(FILTER('Weight & Age'!C:C, NOT(ISBLANK('Weight & Age'!C:C)), 1)"),"180.5")"#,
        "180.5",
    );
    s(ws, 2, 1, "Ideal BMI");
    n(ws, 2, 2, 22.5);
    s(ws, 2, 4, "Birthdate");
    d(ws, 2, 5, 1990, 1, 15);
    s(ws, 3, 4, "Chron. Age");
    f(ws, 3, 5, "=(TODAY() - F3) / 365.25", "35.2");
    s(ws, 8, 1, "avg. calories burned per day");
    n(ws, 8, 2, 2200.0);
    s(ws, 12, 0, "Genetic Disease");
    s(ws, 12, 1, "Example disease");
    s(ws, 12, 2, "made-up description");
    s(ws, 12, 3, "rare");
    s(ws, 13, 0, "Predispositions");
    s(ws, 13, 1, "Example risk");
    s(ws, 13, 3, "common");
    s(ws, 97, 5, "TODO a made-up chart");

    let ws = wb.add_worksheet().set_name("Vitals (Whoop)").unwrap();
    s(ws, 0, 0, "empty");

    let ws = wb.add_worksheet().set_name("Biomarkers").unwrap();
    s(ws, 0, 0, "Test");
    s(ws, 1, 0, "Lab only row");

    let ws = wb.add_worksheet().set_name("Weight & Age").unwrap();
    s(ws, 0, 0, "Date");

    let ws = wb.add_worksheet().set_name("Routine").unwrap();
    row(ws, 0, &["MORNING ROUTINE", "Time for effect", "Execution Time", "Type", "Notes"]);
    for (i, (name, eff, secs, kind, note)) in [
        ("Workout", "", 1800.0, "asynchronous", ""),
        ("Shampoo", "2 minutes", 30.0, "asynchronous", "START SHOWER"),
        ("Brush Teeth", "2 minutes", 120.0, "blocking", ""),
    ]
    .iter()
    .enumerate()
    {
        let r = 1 + i as u32;
        s(ws, r, 0, name);
        if !eff.is_empty() {
            s(ws, r, 1, eff);
        }
        n(ws, r, 2, *secs);
        s(ws, r, 3, kind);
        if !note.is_empty() {
            s(ws, r, 4, note);
        }
    }
    s(ws, 4, 0, "Total");
    f(ws, 4, 2, "=SUM(C2:C4)", "1950");
    s(ws, 6, 0, "NIGHT ROUTINE");
    s(ws, 7, 0, "Dinner");
    s(ws, 7, 1, "still a variable");
    s(ws, 8, 0, "Floss");
    n(ws, 8, 2, 30.0);
    s(ws, 8, 3, "blocking");
    s(ws, 9, 0, "Total");
    f(ws, 9, 2, "=SUM(C8:C9)", "30");

    let ws = wb.add_worksheet().set_name("Products").unwrap();
    row(
        ws,
        0,
        &["Name", "$/unit", "Dosage", "Using", "# units/month", "# units/year", "$/mo.", "$/yr", "Notes", "link"],
    );
    s(ws, 1, 0, "Skin & Hair");
    for (r, name, price, using, per_year) in [(2u32, "Shampoo", 12.5, "YES", 2.0), (3, "Cologne", 100.0, "NO", 0.0)] {
        s(ws, r, 0, name);
        n(ws, r, 1, price);
        s(ws, r, 3, using);
        f(ws, r, 4, &format!("=F{}/12", r + 1), "0.1");
        n(ws, r, 5, per_year);
        f(ws, r, 7, &format!("=F{0}*B{0}", r + 1), "25");
        s(ws, r, 9, "https://example.test/p");
    }
    s(ws, 5, 0, "Oral Health");
    s(ws, 6, 0, "Toothpaste");
    n(ws, 6, 1, 6.0);
    s(ws, 6, 3, "YES");
    f(ws, 6, 5, "=365 * 0.5 / 4", "45.625");
    s(ws, 8, 0, "Supplements");
    f(ws, 8, 7, r#"=SUMIF(Supplements!B:B, "YES", Supplements!M:M)"#, "123");
    s(ws, 10, 0, "Total");
    f(ws, 10, 7, r#"=SUMIF($D3:$D51, "YES", H3:H51)"#, "25");
    s(ws, 12, 0, "Alternative");
    f(ws, 12, 6, "= 20 * 2", "40");
    s(ws, 12, 7, "cost of eating out");

    let ws = wb.add_worksheet().set_name("Supplements").unwrap();
    row(
        ws,
        0,
        &[
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
            "Source",
        ],
    );
    // name, taking, freq, time, type, amount, unit, notes, units/yr, price, buy, research, sources
    type Sup = (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        Option<f64>,
        &'static str,
        &'static str,
        f64,
        Option<f64>,
        &'static str,
        &'static str,
        &'static str,
    );
    let sups: [Sup; 5] = [
        (
            "Alphazyme",
            "YES",
            "Daily",
            "Day",
            "Sp",
            Some(600.0),
            "mg",
            "synthetic",
            3.0,
            Some(32.0),
            "https://example.test/a",
            "https://examine.test/alphazyme",
            "Test Protocol, Box One",
        ),
        ("Betazine", "NO", "Daily", "Night", "Rx", Some(200.0), "mg", "", 0.0, None, "", "", "Test Protocol"),
        (
            "Gammavit",
            "YES",
            "2x/wk",
            "Day",
            "Sp",
            None,
            "2x / wk",
            "half pill",
            2.0,
            Some(25.0),
            "",
            "",
            "box one, Test Protocol",
        ),
        (
            "Deltamin",
            "YES",
            "3/wk",
            "Day & Night",
            "Nt",
            Some(2.0),
            "",
            "",
            6.0,
            Some(13.5),
            "https://example.test/d",
            "",
            "",
        ),
        ("Epsilon Oil", "", "", "", "", Some(1.0), "tbsp", "unset taking", 1.0, Some(9.0), "", "", "Other, source"),
    ];
    for (i, sp) in sups.iter().enumerate() {
        let r = 1 + i as u32;
        s(ws, r, 0, sp.0);
        for (c, v) in [(1u16, sp.1), (2, sp.2), (3, sp.3), (4, sp.4)] {
            if !v.is_empty() {
                s(ws, r, c, v);
            }
        }
        match sp.5 {
            Some(a) => n(ws, r, 5, a),
            None => s(ws, r, 5, "1/2 pill"),
        }
        if !sp.6.is_empty() {
            s(ws, r, 6, sp.6);
        }
        f(ws, r, 7, &format!("=F{0}&G{0}", r + 1), "dose");
        if !sp.7.is_empty() {
            s(ws, r, 8, sp.7);
        }
        n(ws, r, 10, sp.8);
        match sp.9 {
            Some(p) => n(ws, r, 11, p),
            None => s(ws, r, 11, "rx only"),
        }
        f(ws, r, 12, &format!("=L{0}*K{0}", r + 1), "96");
        for (c, v) in [(13u16, sp.10), (14, sp.11), (15, sp.12)] {
            if !v.is_empty() {
                s(ws, r, c, v);
            }
        }
    }

    let ws = wb.add_worksheet().set_name("Nutrition").unwrap();
    row(
        ws,
        0,
        &[
            "Name ",
            "Converted Calories (g)",
            "Calories (kcal)",
            "Protein (g)",
            "Net Carbs (g)",
            "Fiber (g)",
            "Sugar (g)",
            "Sodium (mg)",
            "Cholesterol (mg)",
            "Total Fat (g)",
            "Unsat. Fat (g)",
            "Trans Fat (g)",
            "Sat. Fat (g)",
            "Yuka Score",
            "Ingredients",
            "No Prep",
            "Vegan?",
            "Qualifying Meal?",
            "Sum?",
        ],
    );
    s(ws, 1, 0, "Recommended DV");
    for (c, v) in [(2u16, 2000.0), (3, 140.0), (4, 130.0)] {
        n(ws, 1, c, v);
    }
    s(ws, 1, 10, "n/a");
    s(ws, 1, 14, "Good?");
    s(ws, 2, 0, "Your goal values");
    f(ws, 2, 2, r#"="< "&Dashboard!C10"#, "< 1632");
    n(ws, 2, 4, 0.0);
    n(ws, 2, 5, 28.0);
    s(ws, 3, 0, "Quick Sums");
    f(ws, 3, 2, "=SUMPRODUCT(C5:C1037, --($S5:$S1037 > 0), $S5:$S1037)", "1380");
    s(ws, 4, 0, "Core Meals");
    s(ws, 5, 0, "Test Bowl");
    n(ws, 5, 2, 400.0);
    n(ws, 5, 3, 25.0);
    f(ws, 5, 9, "=sum(K6:M6)", "11");
    f(ws, 5, 10, "=11 - 1.5", "9.5");
    n(ws, 5, 11, 0.0);
    n(ws, 5, 12, 1.5);
    s(ws, 5, 14, "YES");
    n(ws, 5, 18, 0.5);
    f(ws, 5, 17, "=IF(AND($C$6 + C6 < 2000), \"YES\", \"NO\")", "YES");
    s(ws, 7, 0, "Snacks");
    f(ws, 7, 17, "=IF(AND($C$6 > 0), \"YES\", \"NO\")", "YES");
    s(ws, 8, 0, "Test Chips");
    n(ws, 8, 2, 150.0);
    n(ws, 8, 9, 5.0);
    n(ws, 8, 21, 3.5);
    s(ws, 8, 22, "https://example.test/chips");
    f(ws, 8, 17, "=IF(TRUE, \"YES\", \"NO\")", "YES");

    let ws = wb.add_worksheet().set_name("Genetics").unwrap();
    s(ws, 0, 0, "rsid");

    let ws = wb.add_worksheet().set_name("Vaccines  Conditions").unwrap();
    row(
        ws,
        0,
        &[
            "VACCINE TYPE",
            "Disease Date",
            "1st Dose Mo/Day/Yr",
            "2nd Dose Mo/Day/Yr",
            "3rd Dose Mo/Day/Yr",
            "4th Dose Mo/Day/Yr",
            "5th Dose Mo/Day/Yr",
            "6th Dose Mo/Day/Yr",
            "Notes",
        ],
    );
    s(ws, 1, 0, "Test Vaccine");
    d(ws, 1, 2, 2000, 1, 2);
    d(ws, 1, 3, 2000, 3, 4);
    s(ws, 1, 5, "NOTSURE");
    s(ws, 2, 0, "Unvaccinated");
    s(ws, 3, 0, "Made-up condition");
    s(ws, 3, 1, "randomly");
    s(ws, 3, 8, "https://example.test/condition");

    let ws = wb.add_worksheet().set_name("Sizes").unwrap();
    row(ws, 0, &["Category", "Imperial (lb)", "Metric (kg)", "Note"]);
    s(ws, 1, 0, "JACKETS");
    s(ws, 2, 0, "Chest");
    n(ws, 2, 1, 40.0);
    f(ws, 2, 2, "=B3*2.54", "101.6");
    s(ws, 3, 0, "PANTS");
    f(ws, 3, 2, "=B4*2.54", "0");
    s(ws, 4, 0, "Shoes");
    n(ws, 4, 1, 10.5);
    f(ws, 4, 2, "=B5*2.54", "26.67");
    s(ws, 4, 3, "US");

    let ws = wb.add_worksheet().set_name("Physicals").unwrap();
    s(ws, 0, 0, "Regular Checks");
    s(ws, 0, 3, "Doctors");
    s(ws, 1, 0, "Blood Panel");
    s(ws, 2, 0, "Lipid Panel");
    s(ws, 2, 3, "Dr. Example");
    s(ws, 2, 4, "not taking patients");
    s(ws, 3, 0, "Hormone Panel");
    s(ws, 3, 1, "(Testosterone)");
    s(ws, 3, 3, "Dr. Sample");
    s(ws, 3, 5, "https://clinic.test/");
    s(ws, 6, 0, "Referrals");
    s(ws, 7, 0, "ask about palpitations");
    s(ws, 7, 1, "✅ ");
    s(ws, 8, 0, "ask about knees");

    let ws = wb.add_worksheet().set_name("Workout").unwrap();
    row(ws, 0, &["Exercise (30 reps each)", "Muscle(s)"]);
    s(ws, 1, 0, "Rows w/ band");
    s(ws, 1, 1, "upper back");
    s(ws, 1, 2, "Chin Ups Replace");
    s(ws, 1, 3, "Pull Ups Replace");
    s(ws, 2, 0, "Plank 1 minute");
    s(ws, 2, 1, "core");
    s(ws, 5, 0, "Workout outlined here;");
    s(ws, 5, 1, "https://docs.example.test/workout");

    let ws = wb.add_worksheet().set_name("Resources").unwrap();
    s(ws, 0, 1, "https://example.test/one");
    s(ws, 3, 0, "gene therapy");
    s(ws, 3, 1, "https://example.test/two");

    wb.save(path).unwrap();
}

/// Raw 256-bit test KEK (never use a fixed key outside tests).
const TEST_KEY: &str = "raw:000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

pub struct Env {
    pub dir: TempDir,
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}

impl Env {
    pub fn new() -> Self {
        Self { dir: TempDir::new().unwrap() }
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    pub fn db(&self) -> PathBuf {
        self.path("test.db")
    }

    /// Install stand-ins for `biomarker` and `genome` that print synthetic
    /// `biomarker/v1` / `genome/v1` JSON.
    #[cfg(unix)]
    pub fn write_stubs(&self) {
        use std::os::unix::fs::PermissionsExt;
        let dir = self.path("bin");
        std::fs::create_dir_all(&dir).unwrap();
        for f in ["query.json", "markers.json", "kits.json", "summary.json"] {
            std::fs::write(dir.join(f), stub_json(f)).unwrap();
        }
        let script = |first: &str, second: &str, a: &str, b: &str| {
            format!(
                "#!/bin/sh\ncase \"$1\" in\n  {first}) cat '{}' ;;\n  {second}) cat '{}' ;;\n  *) echo \"unexpected: $*\" >&2; exit 2 ;;\nesac\n",
                dir.join(a).display(),
                dir.join(b).display()
            )
        };
        for (name, body) in [
            ("biomarker", script("query", "marker", "query.json", "markers.json")),
            ("genome", script("kits", "summary", "kits.json", "summary.json")),
        ] {
            let p = dir.join(name);
            std::fs::write(&p, body).unwrap();
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    pub fn workbook(&self) -> PathBuf {
        let p = self.path("Health Dashboard.xlsx");
        if !p.exists() {
            write_workbook(&p);
        }
        p
    }

    /// A command isolated from the user's environment and config.
    pub fn cmd(&self) -> Command {
        let mut c = Command::cargo_bin("health").unwrap();
        c.env_clear()
            .envs(std::env::var_os("SYSTEMROOT").map(|v| ("SYSTEMROOT", v)))
            .env("HOME", self.dir.path())
            .env("XDG_CONFIG_HOME", self.path("config"))
            .env("XDG_DATA_HOME", self.path("data"))
            .env("HEALTH_DB", self.db())
            .env("HEALTH_TZ", "UTC")
            .env("HEALTH_KEY", TEST_KEY)
            .env("HEALTH_KEY_SOURCE", "env")
            // Never reach the real sibling tools (and so real labs) from a test.
            .env("HEALTH_BIOMARKER_CMD", self.path("bin/biomarker"))
            .env("HEALTH_GENOME_CMD", self.path("bin/genome"))
            .env("NO_COLOR", "1");
        c
    }

    pub fn run(&self, args: &[&str]) -> String {
        let out = self.cmd().args(args).output().unwrap();
        assert!(
            out.status.success(),
            "health {args:?} failed ({:?}):\n{}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    }

    pub fn json(&self, args: &[&str]) -> Value {
        let mut a = args.to_vec();
        a.extend(["--format", "json"]);
        let v: Value = serde_json::from_str(&self.run(&a)).unwrap();
        assert_eq!(v["schema"], "health/v1");
        v
    }

    pub fn import(&self) -> Value {
        let wb = self.workbook();
        self.json(&["import", "xlsx", wb.to_str().unwrap()])
    }
}

/// Synthetic tool output per file name.
#[cfg(unix)]
fn stub_json(file: &str) -> String {
    use serde_json::json;
    let m = |marker: &str, name: &str, cat: &str, at: &str, v: f64, unit: &str, lo: Option<f64>, hi: Option<f64>| {
        json!({"marker": marker, "marker_name": name, "category": cat, "taken_at": at, "qualifier": null, "value": v,
               "unit": unit, "ref_low": lo, "ref_high": hi, "flag": "normal"})
    };
    let v = match file {
        "query.json" => json!({"schema": "biomarker/v1", "kind": "measurements", "data": [
            m("weight", "Weight", "body", "2024-01-01", 190.0, "lb", None, None),
            m("weight", "Weight", "body", "2024-02-01", 186.5, "lb", None, None),
            m("biological-age", "Biological Age", "body", "2024-02-01", 28.5, "years", None, None),
            m("ldl-c", "LDL Cholesterol", "lipid", "2023-05-01", 131.0, "mg/dL", Some(0.0), Some(99.0)),
            m("ldl-c", "LDL Cholesterol", "lipid", "2024-05-01", 96.0, "mg/dL", Some(0.0), Some(99.0)),
            m("hdl-c", "HDL Cholesterol", "lipid", "2024-05-01", 55.0, "mg/dL", Some(39.0), None),
            m("glucose", "Glucose", "metabolic", "2024-05-01", 88.0, "mg/dL", Some(70.0), Some(99.0)),
            m("hba1c", "Hemoglobin A1c", "metabolic", "2023-05-01", 5.7, "%", Some(4.8), Some(5.6)),
            m("wbc", "White Blood Cells", "cbc", "2023-05-01", 5.6, "10^3/uL", Some(3.4), Some(10.8)),
        ]}),
        "markers.json" => json!({"schema": "biomarker/v1", "kind": "markers", "data": [
            {"slug": "wbc", "name": "White Blood Cells", "category": "cbc", "unit": "10^3/uL", "description": "synthetic note"},
            {"slug": "ldl-c", "name": "LDL Cholesterol", "category": "lipid", "unit": "mg/dL", "description": null},
            {"slug": "hdl-c", "name": "HDL Cholesterol", "category": "lipid", "unit": "mg/dL", "description": null},
            {"slug": "glucose", "name": "Glucose", "category": "metabolic", "unit": "mg/dL", "description": null},
            {"slug": "hba1c", "name": "Hemoglobin A1c", "category": "metabolic", "unit": "%", "description": null},
        ]}),
        "kits.json" => json!({"schema": "genome/v1", "kind": "kits", "data": [
            {"id": "k1", "name": "synthetic-kit", "source_format": "23andme", "assay": "array", "build": "GRCh37", "records": 1000}]}),
        _ => json!({"schema": "genome/v1", "kind": "summary", "data": [
            {"kit": "k1", "records": 1000, "no_calls": 5, "het": 300, "hom_alt": 0, "hemizygous": 20,
             "by_chrom": {"1": 80, "2": 70, "X": 20}, "sex": {"call": "male"}}]}),
    };
    v.to_string()
}
