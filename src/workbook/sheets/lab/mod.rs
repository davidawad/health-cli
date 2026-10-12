//! Workstream LAB: lab tear sheets, statistics and N-of-1 experiments.
//!
//! Every view reads labs through the private long table on the hidden
//! `Lab_Long` sheet ([`long`]), one row per measurement sorted by marker then
//! date, so a marker's draws are contiguous: `MATCH` finds the first, `COUNTIF`
//! the count, and `INDEX(..):INDEX(..)` spans them. Per-marker reference
//! inputs (optimal and critical bounds, biological variation) live on
//! `Lab_Ranges` ([`ranges`]). Both are reached through defined names
//! (`Lab_L_*`, `Lab_R_*`), never through cell addresses. When the shared data
//! layer (`tblLabs`, hx-rx3.21) lands, [`long`] is the one module to swap.

use crate::workbook::labs::Measurement;
use crate::workbook::sheets::biomarkers::{self, MarkerRow};
use crate::workbook::{Data, SheetModule};

pub mod change;
pub mod exp;
pub mod long;
pub mod quilt;
pub mod ranges;
pub mod stats;
pub mod tear;

/// KPI workstream tag (`K_LAB_*`).
pub const WS: &str = "LAB";
/// Owner tag for defined names and parameters.
pub const OWNER: &str = "lab";
/// The data sheets every LAB view reads by name.
pub const REQUIRES: &[&str] = &["lab-ranges", "lab-long"];

/// The LAB sheets, in tab order.
pub fn modules() -> Vec<Box<dyn SheetModule>> {
    vec![
        Box::new(tear::Tear),
        Box::new(stats::Stats),
        Box::new(change::Change),
        Box::new(quilt::Quilt),
        Box::new(exp::Exp),
        Box::new(ranges::Ranges),
        Box::new(long::Long),
    ]
}

/// Lab markers (body measures excluded) in display order, with their draws
/// oldest first. `None` without labs.
pub fn lab_data(d: &Data) -> Option<(Vec<MarkerRow>, Vec<&Measurement>)> {
    let labs = d.labs.as_ref()?;
    let rows = biomarkers::layout(labs).rows;
    if rows.is_empty() {
        return None;
    }
    let draws = rows.iter().flat_map(|r| labs.series(&r.slug)).collect();
    Some((rows, draws))
}

/// Whether there is at least one lab marker (every LAB view needs one).
pub fn has_labs(d: &Data) -> bool {
    lab_data(d).is_some()
}

/// Most draws of any one marker (at least 1): the height of per-draw tables.
pub fn max_draws(d: &Data) -> usize {
    lab_data(d).map_or(1, |(rows, draws)| {
        rows.iter().map(|r| draws.iter().filter(|m| m.marker == r.slug).count()).max().unwrap_or(1).max(1)
    })
}

/// `INDEX(name, first):INDEX(name, first + n - 1)`: one marker's span of a `Lab_L_*` column.
pub fn span(name: &str, first: &str, n: &str) -> String {
    format!("INDEX({name},{first}):INDEX({name},{first}+{n}-1)")
}

/// A number from a `Lab_R_*` column for the marker in `marker_cell`, `""` when blank or absent.
pub fn ref_num(name: &str, marker_cell: &str) -> String {
    let i = format!("INDEX({name},MATCH({marker_cell},Lab_R_Marker,0))");
    format!("IFERROR(IF(ISNUMBER({i}),{i},\"\"),\"\")")
}

/// `Low` / `High` / `OK` for `v` against bounds that may be `""`.
pub fn status(v: &str, lo: &str, hi: &str) -> String {
    format!(
        "IF(NOT(ISNUMBER({v})),\"\",IF(AND(ISNUMBER({lo}),{v}<{lo}),\"Low\",IF(AND(ISNUMBER({hi}),{v}>{hi}),\"High\",\"OK\")))"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbook::labs::{Labs, MarkerInfo};
    use chrono::NaiveDate;

    pub(crate) fn synthetic() -> Data {
        let m = |marker: &str, cat: &str, at: &str, value: f64| Measurement {
            marker: marker.into(),
            name: marker.to_uppercase(),
            category: cat.into(),
            taken_at: NaiveDate::parse_from_str(at, "%Y-%m-%d").unwrap(),
            qualifier: None,
            value,
            unit: "mg/dL".into(),
            ref_low: Some(0.0),
            ref_high: Some(99.0),
            flag: None,
            ..Measurement::default()
        };
        let labs = Labs {
            measurements: vec![
                m("ldl-c", "lipid", "2024-05-01", 96.0),
                m("ldl-c", "lipid", "2023-05-01", 131.0),
                m("ldl-c", "lipid", "2025-05-01", 90.0),
                m("glucose", "metabolic", "2024-05-01", 88.0),
                m("weight", "body", "2024-05-01", 180.0),
            ],
            markers: vec![MarkerInfo {
                slug: "ldl-c".into(),
                name: "LDL".into(),
                category: "lipid".into(),
                unit: "mg/dL".into(),
                description: None,
            }],
            ..Labs::default()
        };
        Data { labs: Some(labs), ..Data::default() }
    }

    #[test]
    fn lab_data_groups_draws_by_marker_oldest_first_and_drops_body_measures() {
        let d = synthetic();
        let (rows, draws) = lab_data(&d).unwrap();
        assert_eq!(rows.iter().map(|r| r.slug.as_str()).collect::<Vec<_>>(), ["ldl-c", "glucose"]);
        let order: Vec<(&str, f64)> = draws.iter().map(|m| (m.marker.as_str(), m.value)).collect();
        assert_eq!(order, [("ldl-c", 131.0), ("ldl-c", 96.0), ("ldl-c", 90.0), ("glucose", 88.0)]);
        assert_eq!(max_draws(&d), 3);
        assert!(!has_labs(&Data::default()));
        assert_eq!(max_draws(&Data::default()), 1);
    }

    #[test]
    fn formula_helpers_guard_blanks() {
        assert_eq!(span("Lab_L_Value", "F3", "E3"), "INDEX(Lab_L_Value,F3):INDEX(Lab_L_Value,F3+E3-1)");
        assert!(ref_num("Lab_R_RefLo", "$A3").starts_with("IFERROR(IF(ISNUMBER(INDEX(Lab_R_RefLo,MATCH($A3,"));
        assert!(status("I3", "Q3", "R3").contains("AND(ISNUMBER(Q3),I3<Q3)"));
    }
}
