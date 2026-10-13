//! Supplement regimens and the interaction grid for the workbook, read from
//! biomarker-cli (>= 0.7.1, `biomarker/v1`): `regimen ls -p PERSON` (taking,
//! stopped or planned as of today, with start and stop days) and
//! `interaction grid -p PERSON --catalog`. Like the labs, health-cli only reads
//! biomarker's stdout. Without them the W3 views fall back to the store's
//! `taking` column.

use serde_json::Value;

use super::labs::{data_of, run, text};
use super::Data;
use crate::error::{AppError, Result};
use crate::regimens::{norm, Catalog};

/// One row of `regimen ls`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Regimen {
    pub id: i64,
    pub intervention: String,
    pub name: String,
    pub dose: Option<f64>,
    pub unit: Option<String>,
    pub frequency: Option<String>,
    pub time_slot: Option<String>,
    pub started_on: Option<String>,
    pub stopped_on: Option<String>,
    /// `taking`, `stopped` or `planned` as of `as_of`.
    pub status: String,
    pub as_of: Option<String>,
    pub days: Option<i64>,
}

/// A filled cell of the grid: the highest-ranked relation between two interventions.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Cell {
    pub row: String,
    pub column: String,
    pub kind: String,
    pub severity: String,
    /// `drug-class:...` when the edge applies through a class, else empty.
    pub via: String,
}

/// An edge from a row to a condition, genotype, lab test or class no column belongs to.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Other {
    pub intervention: String,
    pub target_kind: String,
    pub target: String,
    pub kind: String,
    pub severity: String,
}

/// `interaction grid` for one person on one day.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Grid {
    pub as_of: String,
    pub rows: Vec<String>,
    pub columns: Vec<String>,
    pub cells: Vec<Cell>,
    pub other: Vec<Other>,
}

impl Grid {
    pub fn cell(&self, row: &str, column: &str) -> Option<&Cell> {
        self.cells.iter().find(|c| c.row == row && c.column == column)
    }
}

/// What biomarker knows about one person's supplements.
#[derive(Debug, Clone, Default)]
pub struct Regimens {
    pub person: String,
    pub rows: Vec<Regimen>,
    /// `None` when the grid could not be read (the regimens still apply).
    pub grid: Option<Grid>,
    /// Research slugs that join store supplement names to interventions.
    pub research: Catalog,
}

impl Data {
    /// The biomarker regimen standing for a store supplement, if any.
    pub fn regimen(&self, name: &str) -> Option<&Regimen> {
        self.regimens.as_ref().and_then(|r| r.current(name))
    }

    /// Taking per biomarker when it has a regimen for the supplement, else `None`.
    pub fn regimen_taking(&self, name: &str) -> Option<bool> {
        self.regimens.as_ref().and_then(|r| r.taking(name))
    }
}

impl Regimens {
    /// The regimens of a store supplement: its research slug, else its name or
    /// slugified name against the intervention slug and display name.
    pub fn of(&self, name: &str) -> Vec<&Regimen> {
        let slug = self.research.slug_for(name);
        let n = norm(name);
        self.rows
            .iter()
            .filter(|r| slug == Some(r.intervention.as_str()) || norm(&r.intervention) == n || norm(&r.name) == n)
            .collect()
    }

    /// The regimen standing for a supplement: the one being taken (latest start),
    /// else the most recent stopped one, else a planned one.
    pub fn current(&self, name: &str) -> Option<&Regimen> {
        let rank = |r: &Regimen| match r.status.as_str() {
            "taking" => 2,
            "stopped" => 1,
            _ => 0,
        };
        self.of(name).into_iter().max_by(|a, b| {
            rank(a).cmp(&rank(b)).then_with(|| a.started_on.cmp(&b.started_on)).then_with(|| a.id.cmp(&b.id))
        })
    }

    /// `Some(true)` when biomarker has the supplement as taking, `Some(false)` when it
    /// only has stopped or planned regimens, `None` when it has none (store decides).
    pub fn taking(&self, name: &str) -> Option<bool> {
        let rs = self.of(name);
        (!rs.is_empty()).then(|| rs.iter().any(|r| r.status == "taking"))
    }
}

fn opt_text(v: &Value, k: &str) -> Option<String> {
    text(v, k).filter(|s| !s.is_empty())
}

pub fn parse_regimens(json: &str) -> Result<Vec<Regimen>> {
    Ok(data_of(json, "biomarker/")?
        .iter()
        .filter_map(|r| {
            Some(Regimen {
                id: r["id"].as_i64().unwrap_or_default(),
                intervention: text(r, "intervention")?,
                name: text(r, "intervention_name").unwrap_or_default(),
                dose: r["dose"].as_f64(),
                unit: opt_text(r, "unit"),
                frequency: opt_text(r, "frequency"),
                time_slot: opt_text(r, "time_slot"),
                started_on: opt_text(r, "started_on"),
                stopped_on: opt_text(r, "stopped_on"),
                status: text(r, "status").unwrap_or_default(),
                as_of: opt_text(r, "as_of"),
                days: r["days"].as_i64(),
            })
        })
        .collect())
}

/// The grid object of `interaction grid --format json` (`data` is one object).
pub fn parse_grid(json: &str) -> Result<Grid> {
    let v: Value =
        serde_json::from_str(json).map_err(|e| AppError::invalid(format!("biomarker grid output is not JSON: {e}")))?;
    let schema = v["schema"].as_str().unwrap_or_default();
    if !schema.starts_with("biomarker/") {
        return Err(AppError::invalid(format!("unexpected schema '{schema}' (wanted biomarker/*)")));
    }
    let d = &v["data"];
    let strs = |k: &str| -> Vec<String> {
        d[k].as_array().into_iter().flatten().filter_map(|s| s.as_str().map(str::to_string)).collect()
    };
    let s = |r: &Value, k: &str| text(r, k).unwrap_or_default();
    let objs = |k: &str| d[k].as_array().cloned().unwrap_or_default();
    Ok(Grid {
        as_of: s(d, "as_of"),
        rows: strs("rows"),
        columns: strs("columns"),
        cells: objs("cells")
            .iter()
            .map(|c| Cell {
                row: s(c, "row"),
                column: s(c, "column"),
                kind: s(c, "kind"),
                severity: s(c, "severity"),
                via: c["edges"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find_map(|e| e["via"].as_str().map(str::to_string))
                    .unwrap_or_default(),
            })
            .collect(),
        other: objs("other")
            .iter()
            .map(|o| Other {
                intervention: s(o, "intervention"),
                target_kind: s(o, "target_kind"),
                target: s(o, "target"),
                kind: s(o, "kind"),
                severity: s(o, "severity"),
            })
            .collect(),
    })
}

/// `regimen ls -p PERSON` (every status, as of today) plus `interaction grid -p
/// PERSON --catalog`. Fails when the regimens cannot be read (a biomarker older
/// than 0.7.1); a failing grid alone leaves `grid` empty.
pub fn fetch(cmd: &str, person: &str) -> Result<Regimens> {
    let rows = parse_regimens(&run(cmd, &["regimen", "ls", "-p", person, "--status", "all", "--format", "json"])?)?;
    let grid = run(cmd, &["interaction", "grid", "-p", person, "--catalog", "--format", "json"])
        .and_then(|j| parse_grid(&j))
        .ok();
    Ok(Regimens { person: person.to_string(), rows, grid, research: Catalog::embedded() })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LS: &str = r#"{"schema":"biomarker/v1","kind":"regimens","data":[
      {"id":1,"person":"p","intervention":"alphazyme","intervention_name":"Alphazyme","dose":600.0,"unit":"mg","frequency":"Daily","time_slot":"Day","started_on":"2024-01-02","stopped_on":null,"note":null,"status":"taking","as_of":"2025-01-01","days":365},
      {"id":2,"person":"p","intervention":"betazine","intervention_name":"Betazine","dose":null,"unit":null,"frequency":"","time_slot":null,"started_on":"2023-01-01","stopped_on":"2023-06-01","status":"stopped","as_of":"2025-01-01","days":151},
      {"id":3,"person":"p","intervention":"alphazyme","intervention_name":"Alphazyme","started_on":"2022-01-01","stopped_on":"2023-01-01","status":"stopped"},
      {"id":4,"person":"p","intervention_name":"no slug"}]}"#;

    const GRID: &str = r#"{"schema":"biomarker/v1","kind":"interaction_grid","data":{
      "person":"p","as_of":"2025-01-01","rows":["alphazyme"],"columns":["alphazyme","warfarin"],
      "cells":[{"row":"alphazyme","column":"warfarin","kind":"caution","severity":"moderate",
                "edges":[{"id":3,"kind":"caution","severity":"moderate","via":"drug-class:anticoagulant"}]}],
      "other":[{"intervention":"alphazyme","target_kind":"lab-test","target":"tsh","kind":"lab-interference","severity":"minor","id":7}]}}"#;

    #[test]
    fn regimens_parse_and_match_store_names() {
        let rows = parse_regimens(LS).unwrap();
        assert_eq!(rows.len(), 3, "a row without an intervention is dropped");
        assert_eq!(rows[1].frequency, None, "empty text reads as missing");
        let r = Regimens { person: "p".into(), rows, grid: None, research: Catalog::default() };
        assert_eq!(r.taking("ALPHAZYME"), Some(true));
        assert_eq!(r.taking("Betazine"), Some(false));
        assert_eq!(r.taking("Gammavit"), None);
        assert_eq!(r.current("Alphazyme").map(|r| r.id), Some(1), "the taking regimen wins");
        assert_eq!(r.of("alphazyme").len(), 2);
        assert!(parse_regimens(r#"{"schema":"other/v1","data":[]}"#).is_err());
    }

    #[test]
    fn grid_parses_cells_and_other_edges() {
        let g = parse_grid(GRID).unwrap();
        assert_eq!((g.rows.len(), g.columns.len(), g.as_of.as_str()), (1, 2, "2025-01-01"));
        let c = g.cell("alphazyme", "warfarin").unwrap();
        assert_eq!((c.kind.as_str(), c.via.as_str()), ("caution", "drug-class:anticoagulant"));
        assert!(g.cell("warfarin", "alphazyme").is_none());
        assert_eq!(g.other[0].target, "tsh");
        assert!(parse_grid("nope").is_err());
        assert!(parse_grid(r#"{"schema":"genome/v1","data":{}}"#).is_err());
    }

    #[test]
    fn fetch_fails_without_a_regimen_capable_biomarker() {
        assert!(fetch("/nonexistent/biomarker-cli-for-test", "p").is_err());
        assert!(fetch("false", "p").is_err());
    }
}
