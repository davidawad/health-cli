//! Lab and genetics data for the workbook, read from the sibling tools'
//! `--format json` output (`biomarker/v1`, `genome/v1`). health-cli never
//! opens their databases: it shells out, so their own encryption, audit and
//! unit/range logic stay the single source of truth.

use std::collections::BTreeMap;
use std::process::Command;

use chrono::NaiveDate;
use serde_json::Value;

use crate::error::{AppError, Result};

#[derive(Debug, Clone, Default)]
pub struct Measurement {
    pub marker: String,
    pub name: String,
    pub category: String,
    pub taken_at: NaiveDate,
    pub qualifier: Option<String>,
    pub value: f64,
    pub unit: String,
    pub ref_low: Option<f64>,
    pub ref_high: Option<f64>,
    pub flag: Option<String>,
    pub value_raw: Option<f64>,
    pub unit_raw: String,
    pub lab: Option<String>,
    pub fasting: Option<bool>,
    pub opt_low: Option<f64>,
    pub opt_high: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct MarkerInfo {
    pub slug: String,
    pub name: String,
    pub category: String,
    pub unit: String,
    pub description: Option<String>,
}

/// (low, high) bounds per marker slug.
pub type Bounds = BTreeMap<String, (Option<f64>, Option<f64>)>;

#[derive(Debug, Clone, Default)]
pub struct Labs {
    /// In US units (`--units us`): what the legacy sheets show (weight in lb).
    pub measurements: Vec<Measurement>,
    /// In canonical units (biomarker's default): what the `Data_*` tables carry.
    pub canonical: Vec<Measurement>,
    pub markers: Vec<MarkerInfo>,
    /// Blueprint optimal bounds (low, high) per marker slug, canonical units: the
    /// built-in `blueprint` range set of biomarker v0.6+. Empty when unavailable.
    pub blueprint: Bounds,
}

impl Labs {
    /// Measurements of one marker, oldest first.
    pub fn series(&self, slug: &str) -> Vec<&Measurement> {
        let mut v: Vec<&Measurement> = self.measurements.iter().filter(|m| m.marker == slug).collect();
        v.sort_by_key(|m| m.taken_at);
        v
    }

    pub fn marker(&self, slug: &str) -> Option<&MarkerInfo> {
        self.markers.iter().find(|m| m.slug == slug)
    }

    /// Distinct draw dates of the measurements outside `exclude_categories`, ascending.
    pub fn dates_excluding(&self, exclude_categories: &[&str]) -> Vec<NaiveDate> {
        let mut d: Vec<NaiveDate> = self
            .measurements
            .iter()
            .filter(|m| !exclude_categories.contains(&m.category.as_str()))
            .map(|m| m.taken_at)
            .collect();
        d.sort();
        d.dedup();
        d
    }

    /// Latest measurement of each marker.
    pub fn latest(&self) -> BTreeMap<&str, &Measurement> {
        let mut out: BTreeMap<&str, &Measurement> = BTreeMap::new();
        for m in &self.measurements {
            if out.get(m.marker.as_str()).is_none_or(|cur| cur.taken_at <= m.taken_at) {
                out.insert(m.marker.as_str(), m);
            }
        }
        out
    }
}

/// One kit of `genome kits` joined with its `genome summary`.
#[derive(Debug, Clone)]
pub struct Kit {
    pub id: String,
    pub name: String,
    pub source_format: String,
    pub assay: String,
    pub build: String,
    pub records: i64,
    pub no_calls: Option<i64>,
    pub het: Option<i64>,
    pub hom_alt: Option<i64>,
    pub hemizygous: Option<i64>,
    pub sex_call: Option<String>,
    pub by_chrom: Vec<(String, i64)>,
}

fn data_of(json: &str, schema_prefix: &str) -> Result<Vec<Value>> {
    let v: Value = serde_json::from_str(json)
        .map_err(|e| AppError::invalid(format!("{schema_prefix} output is not JSON: {e}")))?;
    let schema = v["schema"].as_str().unwrap_or_default();
    if !schema.starts_with(schema_prefix) {
        return Err(AppError::invalid(format!("unexpected schema '{schema}' (wanted {schema_prefix}*)")));
    }
    Ok(v["data"].as_array().cloned().unwrap_or_default())
}

fn text(v: &Value, k: &str) -> Option<String> {
    v[k].as_str().map(str::to_string)
}

pub fn parse_measurements(json: &str) -> Result<Vec<Measurement>> {
    data_of(json, "biomarker/")?
        .iter()
        .filter_map(|r| {
            let taken = r["taken_at"].as_str()?.get(..10)?;
            Some(Measurement {
                marker: text(r, "marker")?,
                name: text(r, "marker_name").unwrap_or_default(),
                category: text(r, "category").unwrap_or_default(),
                taken_at: NaiveDate::parse_from_str(taken, "%Y-%m-%d").ok()?,
                qualifier: text(r, "qualifier"),
                value: r["value"].as_f64()?,
                unit: text(r, "unit").unwrap_or_default(),
                ref_low: r["ref_low"].as_f64(),
                ref_high: r["ref_high"].as_f64(),
                flag: text(r, "flag"),
                value_raw: r["value_raw"].as_f64(),
                unit_raw: text(r, "unit_raw").unwrap_or_default(),
                lab: text(r, "lab"),
                fasting: r["fasting"].as_bool(),
                opt_low: r["opt_low"].as_f64(),
                opt_high: r["opt_high"].as_f64(),
            })
        })
        .map(Ok)
        .collect()
}

pub fn parse_markers(json: &str) -> Result<Vec<MarkerInfo>> {
    Ok(data_of(json, "biomarker/")?
        .iter()
        .filter_map(|r| {
            Some(MarkerInfo {
                slug: text(r, "slug")?,
                name: text(r, "name").unwrap_or_default(),
                category: text(r, "category").unwrap_or_default(),
                unit: text(r, "unit").unwrap_or_default(),
                description: text(r, "description"),
            })
        })
        .collect())
}

/// The `blueprint` rows of `biomarker profile show PERSON --format json` run with
/// `BIOMARKER_RANGE_SET=blueprint`: optimal ranges whose source is `set:blueprint`.
/// A marker whose personal range shadows the set has no row and stays blank.
pub fn parse_blueprint(json: &str) -> Result<Bounds> {
    Ok(data_of(json, "biomarker/")?
        .iter()
        .filter(|r| r["kind"].as_str() == Some("optimal") && r["source"].as_str() == Some(BLUEPRINT_SOURCE))
        .filter_map(|r| {
            let b = (r["low"].as_f64(), r["high"].as_f64());
            (b.0.is_some() || b.1.is_some()).then_some((text(r, "marker")?, b))
        })
        .collect())
}

const BLUEPRINT_SOURCE: &str = "set:blueprint";

pub fn parse_kits(kits_json: &str, summary_json: &str) -> Result<Vec<Kit>> {
    let summaries = data_of(summary_json, "genome/")?;
    Ok(data_of(kits_json, "genome/")?
        .iter()
        .filter_map(|k| {
            let id = text(k, "id")?;
            let s = summaries.iter().find(|s| s["kit"].as_str() == Some(id.as_str()));
            let n = |key: &str| s.and_then(|s| s[key].as_i64());
            Some(Kit {
                name: text(k, "name").unwrap_or_default(),
                source_format: text(k, "source_format").unwrap_or_default(),
                assay: text(k, "assay").unwrap_or_default(),
                build: text(k, "build").unwrap_or_default(),
                records: k["records"].as_i64().unwrap_or_default(),
                no_calls: n("no_calls"),
                het: n("het"),
                hom_alt: n("hom_alt"),
                hemizygous: n("hemizygous"),
                sex_call: s.and_then(|s| s["sex"]["call"].as_str().map(str::to_string)),
                by_chrom: s
                    .and_then(|s| s["by_chrom"].as_object())
                    .map(|m| m.iter().filter_map(|(c, n)| n.as_i64().map(|n| (c.clone(), n))).collect())
                    .unwrap_or_default(),
                id,
            })
        })
        .collect())
}

/// Run `cmd args...` and return stdout. Personal data flows through the pipe
/// only; stderr is shown on failure.
fn run(cmd: &str, args: &[&str]) -> Result<String> {
    run_env(cmd, args, &[])
}

fn run_env(cmd: &str, args: &[&str], env: &[(&str, &str)]) -> Result<String> {
    let out = Command::new(cmd).args(args).envs(env.iter().copied()).output().map_err(|e| {
        AppError::io(format!(
            "running `{cmd} {}`: {e} (is it installed? see the biomarker_cmd / genome_cmd settings)",
            args.join(" ")
        ))
    })?;
    if !out.status.success() {
        return Err(AppError::general(format!(
            "`{cmd} {}` failed ({}): {}",
            args.join(" "),
            out.status.code().map_or_else(|| "signal".into(), |c| c.to_string()),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    String::from_utf8(out.stdout).map_err(|e| AppError::io(format!("`{cmd}` output is not UTF-8: {e}")))
}

/// `biomarker query -p PERSON --units us --format json` plus the marker
/// catalog (names, descriptions). Weights come back in lb, labs in US units.
pub fn fetch_labs(cmd: &str, person: &str) -> Result<Labs> {
    let q = run(cmd, &["query", "-p", person, "--units", "us", "--format", "json"])?;
    let c = run(cmd, &["query", "-p", person, "--format", "json"])?;
    let m = run(cmd, &["marker", "list", "--format", "json"])?;
    Ok(Labs {
        measurements: parse_measurements(&q)?,
        canonical: parse_measurements(&c)?,
        markers: parse_markers(&m)?,
        blueprint: fetch_blueprint(cmd, person),
    })
}

/// The `blueprint` range set's optimal bounds in canonical units, or nothing when
/// this biomarker predates the set (v0.6 without it) or `profile show` fails: the
/// `blueprint_*` columns then stay blank rather than failing the lab sheets.
pub fn fetch_blueprint(cmd: &str, person: &str) -> Bounds {
    run_env(
        cmd,
        &["profile", "show", person, "--units", "canonical", "--format", "json"],
        &[("BIOMARKER_RANGE_SET", "blueprint")],
    )
    .and_then(|j| parse_blueprint(&j))
    .unwrap_or_default()
}

/// `genome kits` + `genome summary` (counts only; genotypes stay in genome-cli).
pub fn fetch_kits(cmd: &str) -> Result<Vec<Kit>> {
    parse_kits(&run(cmd, &["kits", "--format", "json"])?, &run(cmd, &["summary", "--format", "json"])?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const Q: &str = r#"{"schema":"biomarker/v1","kind":"measurements","data":[
      {"marker":"ldl-c","marker_name":"LDL","category":"lipid","taken_at":"2024-12-04","qualifier":null,"value":124.0,"unit":"mg/dL","ref_low":0.0,"ref_high":99.0,"flag":"high"},
      {"marker":"ldl-c","marker_name":"LDL","category":"lipid","taken_at":"2023-01-02T08:00:00","qualifier":null,"value":100.0,"unit":"mg/dL","ref_low":0.0,"ref_high":99.0,"flag":"high"},
      {"marker":"weight","marker_name":"Weight","category":"body","taken_at":"2024-01-01","value":180.5,"unit":"lb"}]}"#;

    #[test]
    fn parses_measurements_and_series() {
        let labs = Labs { measurements: parse_measurements(Q).unwrap(), ..Labs::default() };
        assert_eq!(labs.measurements.len(), 3);
        let s = labs.series("ldl-c");
        assert_eq!(s.iter().map(|m| m.value).collect::<Vec<_>>(), [100.0, 124.0], "oldest first");
        assert_eq!(labs.dates_excluding(&["body"]).len(), 2);
        assert_eq!(labs.latest()["ldl-c"].value, 124.0);
    }

    #[test]
    fn wrong_schema_is_rejected() {
        assert!(parse_measurements(r#"{"schema":"other/v1","data":[]}"#).is_err());
        assert!(parse_measurements("nope").is_err());
    }

    #[test]
    fn blueprint_keeps_only_the_sets_optimal_bounds() {
        // Synthetic `profile show` output shaped like biomarker v0.6 (PR #4).
        let j = r#"{"schema":"biomarker/v1","kind":"profile","person":"p","range_set":"blueprint","data":[
          {"marker":"hba1c","unit":"%","kind":"optimal","low":4.5,"high":5.6,"source":"set:blueprint"},
          {"marker":"hba1c","unit":"%","kind":"reference","low":4.0,"high":5.6,"source":"catalog"},
          {"marker":"alt","unit":"U/L","kind":"optimal","low":null,"high":15.0,"source":"set:blueprint"},
          {"marker":"ldl-c","unit":"mg/dL","kind":"optimal","low":null,"high":70.0,"source":"person"},
          {"marker":"ggt","unit":"U/L","kind":"optimal","low":null,"high":null,"source":"set:blueprint"}]}"#;
        let b = parse_blueprint(j).unwrap();
        assert_eq!(b.len(), 2, "{b:?}");
        assert_eq!(b["hba1c"], (Some(4.5), Some(5.6)));
        assert_eq!(b["alt"], (None, Some(15.0)));
        assert!(parse_blueprint(r#"{"schema":"other/v1","data":[]}"#).is_err());
    }

    #[test]
    fn blueprint_fetch_falls_back_to_empty() {
        assert!(fetch_blueprint("/nonexistent/biomarker-cli-for-test", "p").is_empty());
        assert!(fetch_blueprint("false", "p").is_empty(), "a failing command yields no bounds");
    }

    #[test]
    fn kits_join_their_summary() {
        let kits = r#"{"schema":"genome/v1","data":[{"id":"k1","name":"a","source_format":"23andme","assay":"array","build":"GRCh37","records":10}]}"#;
        let sum = r#"{"schema":"genome/v1","data":[{"kit":"k1","no_calls":1,"het":2,"hom_alt":0,"hemizygous":3,"by_chrom":{"1":5,"X":2},"sex":{"call":"male"}}]}"#;
        let k = &parse_kits(kits, sum).unwrap()[0];
        assert_eq!((k.records, k.het, k.sex_call.as_deref()), (10, Some(2), Some("male")));
        assert_eq!(k.by_chrom.len(), 2);
    }
}
