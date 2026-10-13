//! Supplement regimens moving to biomarker-cli (>= 0.7.1), which owns per-person
//! regimens and the interaction grid next to the labs. This module turns the
//! store's supplement rows into a deterministic plan of `biomarker regimen add`
//! commands; `health export regimens` prints it and, with `--apply`, runs it.
//!
//! Each supplement is matched to its research slug (`research/supplements/<slug>`,
//! the interventions biomarker imported) by name, slug or alias after case and
//! punctuation normalisation. The store has no start dates yet, so a supplement
//! being taken needs one (its own `start_date` when present, else `--start`) before
//! it can be applied: biomarker's `regimen add` would otherwise default to today.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::error::{AppError, Result};
use crate::output::Record;
use crate::store::Rec;

/// The research index shipped with the source (`research/supplements/index.json`).
const INDEX: &str = include_str!("../research/supplements/index.json");

/// Lower-case letters and digits only: `Vitamin D-3` and `vitamin d3` meet.
pub fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// Research slugs keyed by every normalised spelling (slug, name, aliases).
#[derive(Debug, Default, Clone)]
pub struct Catalog {
    keys: BTreeMap<String, String>,
}

impl Catalog {
    /// The 87 research slugs and names of the index compiled into the binary.
    pub fn embedded() -> Self {
        let rows: Vec<Value> = serde_json::from_str(INDEX).unwrap_or_default();
        let mut c = Self::default();
        for r in &rows {
            if let (Some(slug), Some(name)) = (r["slug"].as_str(), r["name"].as_str()) {
                c.add(slug, &[name]);
            }
        }
        c
    }

    /// Every `<slug>/interactions.json` (`supplement-interactions/1`) under `dir`:
    /// slug from the document (else the directory), plus its name and aliases.
    pub fn from_dir(dir: &Path) -> Result<Self> {
        let rd = std::fs::read_dir(dir).map_err(|e| AppError::io(format!("reading {}: {e}", dir.display())))?;
        let mut dirs: Vec<_> = rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.is_dir()).collect();
        dirs.sort();
        let mut c = Self::default();
        for d in dirs {
            let file = d.join("interactions.json");
            let Ok(bytes) = std::fs::read(&file) else { continue };
            let v: Value = serde_json::from_slice(&bytes)
                .map_err(|e| AppError::invalid(format!("{} is not JSON: {e}", file.display())))?;
            let dir_slug = d.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let slug = v["slug"].as_str().map_or(dir_slug, str::to_string);
            let mut names: Vec<&str> = v["supplement"].as_str().into_iter().collect();
            names.extend(v["aliases"].as_array().into_iter().flatten().filter_map(Value::as_str));
            c.add(&slug, &names);
        }
        Ok(c)
    }

    /// Register `slug` under itself and `names`; the first slug to claim a spelling keeps it.
    fn add(&mut self, slug: &str, names: &[&str]) {
        for k in std::iter::once(slug).chain(names.iter().copied()).map(norm).filter(|k| !k.is_empty()) {
            self.keys.entry(k).or_insert_with(|| slug.to_string());
        }
    }

    pub fn len(&self) -> usize {
        self.keys.values().collect::<std::collections::BTreeSet<_>>().len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// The research slug of a supplement name, if it has one.
    pub fn slug_for(&self, name: &str) -> Option<&str> {
        self.keys.get(&norm(name)).map(String::as_str)
    }
}

/// What the plan does with one supplement row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    /// `biomarker regimen add ...`
    Add,
    /// Taken, but no start date: needs one before it can be applied.
    NeedsStart,
    /// Nothing to record in biomarker.
    Skip,
}

/// One supplement row and the biomarker command it becomes.
#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub position: i64,
    pub supplement: String,
    pub slug: Option<String>,
    pub taking: Option<bool>,
    pub dose: Option<f64>,
    pub unit: Option<String>,
    pub amount_text: Option<String>,
    pub frequency: Option<String>,
    pub time_slot: Option<String>,
    pub started_on: Option<String>,
    pub stopped_on: Option<String>,
    pub action: Action,
    pub reason: Option<String>,
    /// Arguments after the biomarker executable (empty unless `action` is `add`).
    pub argv: Vec<String>,
}

/// Options of a plan: who, and the start date for rows that have none.
#[derive(Debug, Clone, Default)]
pub struct PlanOpts {
    pub person: String,
    pub default_start: Option<String>,
}

fn trimmed(r: &Record, k: &str) -> Option<String> {
    r.s(k).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// A date column of a row, `YYYY-MM-DD` (a datetime is cut to its day).
fn day(r: &Record, k: &str) -> Option<String> {
    trimmed(r, k).map(|s| s.get(..10).unwrap_or(&s).to_string())
}

/// `600` rather than `600.0`; Rust's shortest round-trip form otherwise.
pub fn fmt_num(x: f64) -> String {
    format!("{x}")
}

/// The plan for `supplements` (rows of [`crate::store::supplements`], in store order).
pub fn plan(supplements: &[Record], cat: &Catalog, o: &PlanOpts) -> Vec<Entry> {
    supplements.iter().map(|r| entry(r, cat, o)).collect()
}

fn entry(r: &Record, cat: &Catalog, o: &PlanOpts) -> Entry {
    let name = trimmed(r, "name").unwrap_or_default();
    let taking = r.b("taking");
    let mut e = Entry {
        position: r.i("position").unwrap_or_default(),
        slug: cat.slug_for(&name).map(str::to_string),
        supplement: name,
        taking,
        dose: r.f("amount"),
        unit: trimmed(r, "unit"),
        amount_text: trimmed(r, "amount_text"),
        frequency: trimmed(r, "frequency"),
        time_slot: trimmed(r, "time_of_day"),
        started_on: day(r, "start_date"),
        stopped_on: day(r, "stop_date"),
        action: Action::Skip,
        reason: None,
        argv: Vec::new(),
    };
    if taking == Some(true) && e.started_on.is_none() {
        e.started_on.clone_from(&o.default_start);
    }
    let skip = |e: &mut Entry, why: &str| {
        e.action = Action::Skip;
        e.reason = Some(why.to_string());
    };
    match (taking, &e.slug, &e.started_on, &e.stopped_on) {
        (_, None, _, _) => skip(&mut e, "no research/supplements slug matches this name"),
        (None, ..) => skip(&mut e, "taking status unknown"),
        (Some(false), _, None, _) => skip(&mut e, "not taking (no start date: no history to record)"),
        (Some(false), _, Some(_), None) => skip(&mut e, "not taking, but no stop date for its past regimen"),
        (Some(true), _, None, _) => {
            e.action = Action::NeedsStart;
            e.reason = Some("taking, but no start date (pass --start or record one)".into());
        }
        _ => {
            e.action = Action::Add;
            e.argv = argv(&e, &o.person);
        }
    }
    e
}

/// `regimen add SLUG -p PERSON [--dose N [--unit U]] [--frequency F] [--slot S] --start D [--stop D] [--note T]`.
/// Never `-o`/`--output`: the plan only ever reads biomarker's stdout.
fn argv(e: &Entry, person: &str) -> Vec<String> {
    let mut a: Vec<String> = vec!["regimen".into(), "add".into(), e.slug.clone().unwrap_or_default()];
    let mut opt = |flag: &str, v: Option<String>| {
        if let Some(v) = v {
            a.extend([flag.to_string(), v]);
        }
    };
    opt("-p", Some(person.to_string()));
    opt("--dose", e.dose.map(fmt_num));
    opt("--unit", e.dose.and(e.unit.clone()));
    opt("--frequency", e.frequency.clone());
    opt("--slot", e.time_slot.clone());
    opt("--start", e.started_on.clone());
    opt("--stop", e.stopped_on.clone());
    // A dose the store holds only as text ("1/2 pill") travels as a note.
    let amount = match (e.dose, &e.amount_text, &e.unit) {
        (None, Some(t), Some(u)) => Some(format!("{t} {u}")),
        (None, Some(t), None) => Some(t.clone()),
        (None, None, Some(u)) => Some(u.clone()),
        _ => None,
    };
    opt("--note", amount.map(|t| format!("health-cli amount: {t}")));
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rec(v: Value) -> Record {
        serde_json::from_value(v).unwrap()
    }

    fn cat() -> Catalog {
        let mut c = Catalog::default();
        c.add("alphazyme", &["Alphazyme", "AZ-1"]);
        c.add("vitamin-d-3", &["Vitamin D-3", "D3"]);
        c.add("gammavit", &["Gammavit"]);
        c
    }

    fn opts(start: Option<&str>) -> PlanOpts {
        PlanOpts { person: "p1".into(), default_start: start.map(str::to_string) }
    }

    #[test]
    fn embedded_index_knows_every_research_slug() {
        let c = Catalog::embedded();
        assert_eq!(c.len(), 87);
        assert_eq!(c.slug_for("Calcium Alpha-Ketoglutarate (Ca-AKG)"), Some("calcium-alpha-ketoglutarate-ca-akg"));
        assert_eq!(c.slug_for("17α-E2 (transdermal)"), Some("17-alpha-estradiol-transdermal"));
        assert_eq!(c.slug_for("vitamin d3"), Some("vitamin-d-3"));
        assert_eq!(c.slug_for("Not A Supplement"), None);
    }

    #[test]
    fn names_slugs_and_aliases_match_after_normalisation() {
        let c = cat();
        assert_eq!(c.slug_for("ALPHAZYME"), Some("alphazyme"));
        assert_eq!(c.slug_for("az 1"), Some("alphazyme"));
        assert_eq!(c.slug_for("Vitamin D3"), Some("vitamin-d-3"));
        assert_eq!(c.slug_for(""), None);
    }

    #[test]
    fn a_taken_supplement_becomes_a_regimen_add() {
        let r = rec(json!({"position": 1, "name": "Alphazyme", "taking": 1, "amount": 600.0, "unit": "mg",
                           "frequency": "Daily", "time_of_day": "Day"}));
        let e = &plan(&[r], &cat(), &opts(Some("2024-01-02")))[0];
        assert_eq!(e.action, Action::Add);
        assert_eq!(
            e.argv,
            [
                "regimen",
                "add",
                "alphazyme",
                "-p",
                "p1",
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
    }

    #[test]
    fn a_text_amount_travels_as_a_note_and_own_dates_win() {
        let r = rec(json!({"position": 3, "name": "Gammavit", "taking": 1, "amount": null, "amount_text": "1/2 pill",
                           "unit": "2x / wk", "start_date": "2023-05-06T00:00:00", "stop_date": "2024-01-01"}));
        let e = &plan(&[r], &cat(), &opts(Some("2024-01-02")))[0];
        assert_eq!(e.started_on.as_deref(), Some("2023-05-06"), "the row's own start beats --start");
        assert!(!e.argv.contains(&"--dose".to_string()) && !e.argv.contains(&"--unit".to_string()));
        let tail: Vec<&str> = e.argv.iter().rev().take(6).rev().map(String::as_str).collect();
        assert_eq!(
            tail,
            ["--start", "2023-05-06", "--stop", "2024-01-01", "--note", "health-cli amount: 1/2 pill 2x / wk"]
        );
    }

    #[test]
    fn rows_without_what_a_regimen_needs_are_not_applied() {
        let rows = [
            rec(json!({"name": "Alphazyme", "taking": 1})),
            rec(json!({"name": "Gammavit", "taking": 0})),
            rec(json!({"name": "Gammavit", "taking": 0, "start_date": "2020-01-01"})),
            rec(json!({"name": "Gammavit", "taking": null})),
            rec(json!({"name": "Unknownium", "taking": 1})),
            rec(json!({"name": "Vitamin D3", "taking": 0, "start_date": "2020-01-01", "stop_date": "2021-01-01"})),
        ];
        let p = plan(&rows, &cat(), &opts(None));
        let acts: Vec<Action> = p.iter().map(|e| e.action).collect();
        use Action::*;
        assert_eq!(acts, [NeedsStart, Skip, Skip, Skip, Skip, Add]);
        assert!(p.iter().all(|e| (e.action == Add) != e.argv.is_empty()));
        assert!(p[1].reason.as_deref().unwrap().contains("not taking"));
        assert!(p[4].reason.as_deref().unwrap().contains("slug"));
        // --start fills only taken rows; a stopped past regimen keeps its own dates.
        let p = plan(&rows, &cat(), &opts(Some("2024-01-02")));
        assert_eq!(p[0].action, Add);
        assert_eq!(p[1].action, Skip);
        assert_eq!(p[5].argv[3..], ["-p", "p1", "--start", "2020-01-01", "--stop", "2021-01-01"]);
    }

    #[test]
    fn plans_are_deterministic_and_never_write_files() {
        let rows: Vec<Record> = (0..5)
            .map(|i| rec(json!({"position": i, "name": "Alphazyme", "taking": 1, "amount": 1.5, "unit": "g"})))
            .collect();
        let a = serde_json::to_string(&plan(&rows, &cat(), &opts(Some("2024-01-02")))).unwrap();
        let b = serde_json::to_string(&plan(&rows, &cat(), &opts(Some("2024-01-02")))).unwrap();
        assert_eq!(a, b);
        for e in plan(&rows, &cat(), &opts(Some("2024-01-02"))) {
            assert!(!e.argv.iter().any(|x| x == "-o" || x.starts_with("--output")), "{:?}", e.argv);
            assert!(e.argv.windows(2).any(|w| w[0] == "--dose" && w[1] == "1.5"));
        }
    }

    #[test]
    fn from_dir_reads_slug_name_and_aliases() {
        let dir = tempfile::TempDir::new().unwrap();
        let sub = dir.path().join("betazine");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(
            sub.join("interactions.json"),
            r#"{"schema":"supplement-interactions/1","slug":"betazine","supplement":"Betazine","aliases":["BTZ"]}"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("empty")).unwrap();
        let c = Catalog::from_dir(dir.path()).unwrap();
        assert_eq!((c.len(), c.slug_for("btz")), (1, Some("betazine")));
        assert!(Catalog::from_dir(&dir.path().join("missing")).is_err());
    }
}
