//! `Sup_Grade` (SUP-16): GRADE certainty per supplement x outcome (start 4 for
//! randomised trials, 2 for observational; five downgrades, three upgrades;
//! High / Moderate / Low / Very low with plus-stars), summary-of-findings columns,
//! directness flags and the intervention evidence ladder from `tblSuppTags.rung`.
//! The rules live in `Ref_GRADE` (this workstream's rule table).

use rust_xlsxwriter::{Chart, ChartType, DataValidation, Formula, Worksheet};

use super::{book, data_cell, fx, head, inputs, rule_rows, table, x, Rule, HEAD};
use crate::error::Result;
use crate::infra::data::{TableDef, SUPP_TAGS_DEF as TAGS, V};
use crate::infra::refs::RULE_HEADERS;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Sup_Grade";
/// Extra entry rows under the supplement-aligned ones (more outcomes per supplement).
pub const SPARE: u32 = 10;

const SRC: &str = "EV GRADE-3 (Balshem 2011) l.238-261; GRADE-1 (Guyatt 2011)";
pub const RULES: &[Rule] = &[
    ("GR-START-RCT", "Randomised trials start at", Some(4.0), "level", SRC, "verified", "High"),
    ("GR-START-OBS", "Observational studies start at", Some(2.0), "level", SRC, "verified", "Low"),
    ("GR-DOWN-ROB", "Downgrade: risk of bias", Some(2.0), "max levels", SRC, "verified", "0, 1 or 2"),
    ("GR-DOWN-INC", "Downgrade: inconsistency", Some(2.0), "max levels", SRC, "verified", "0, 1 or 2"),
    (
        "GR-DOWN-IND",
        "Downgrade: indirectness",
        Some(2.0),
        "max levels",
        SRC,
        "verified",
        "0, 1 or 2 (animal-only, other population, surrogate)",
    ),
    ("GR-DOWN-IMP", "Downgrade: imprecision", Some(2.0), "max levels", SRC, "verified", "0, 1 or 2"),
    ("GR-DOWN-PUB", "Downgrade: publication bias", Some(2.0), "max levels", SRC, "verified", "0, 1 or 2"),
    ("GR-UP-LARGE", "Upgrade: large effect", Some(2.0), "max levels", SRC, "verified", "0, 1 or 2"),
    ("GR-UP-DOSE", "Upgrade: dose-response gradient", Some(1.0), "max levels", SRC, "verified", "0 or 1"),
    (
        "GR-UP-CONF",
        "Upgrade: plausible confounding would reduce the effect",
        Some(1.0),
        "max levels",
        SRC,
        "verified",
        "0 or 1",
    ),
    ("GR-LEVEL-4", "High", Some(4.0), "level", SRC, "verified", "four plus signs"),
    ("GR-LEVEL-3", "Moderate", Some(3.0), "level", SRC, "verified", "three plus signs"),
    ("GR-LEVEL-2", "Low", Some(2.0), "level", SRC, "verified", "two plus signs"),
    ("GR-LEVEL-1", "Very low", Some(1.0), "level", SRC, "verified", "one plus sign"),
];

fn rows(_: &Data) -> Vec<Vec<V>> {
    rule_rows(RULES)
}

pub static REF_GRADE: TableDef = TableDef {
    id: "ref-grade",
    sheet: "Ref_GRADE",
    table: "tblRefGRADE",
    headers: RULE_HEADERS,
    rows,
    needs_labs: false,
};

const HEADERS: &[&str] = &[
    "supplement",
    "outcome",
    "design",
    "risk_of_bias",
    "inconsistency",
    "indirectness",
    "imprecision",
    "publication_bias",
    "large_effect",
    "dose_response",
    "confounding",
    "start",
    "certainty",
    "label",
    "stars",
    "directness_flag",
    "tag_indirectness",
    "tag_rung",
    "n_studies",
    "n_participants",
    "effect_ci",
];

/// Entry rows: one per supplement plus [`SPARE`].
pub fn nrows(d: &Data) -> u32 {
    d.supplements.len() as u32 + SPARE
}

fn rng_of(d: &Data, c: u16) -> String {
    crate::workbook::range(TITLE, c, x(0), x(nrows(d) - 1))
}

/// Formula (no `=`) for the certainty of the supplement named in `name_cell` (row-aligned
/// with `tblSupp` row `i`): best GRADE row, else `tblSuppTags.evidence_grade`, else
/// `Sup_Default_Certainty`.
pub fn certainty(cx: &Cx, name_cell: &str, i: u32) -> String {
    let (s, c) = (rng_of(cx.d, 0), rng_of(cx.d, 12));
    let tag = data_cell(&TAGS, "evidence_grade", i);
    format!(
        "IF(COUNTIFS({s},{name_cell},{c},\">0\")>0,MAXIFS({c},{s},{name_cell}),IF(ISNUMBER({tag}),MAX(1,MIN(4,{tag})),Sup_Default_Certainty))"
    )
}

pub struct Grade;

impl SheetModule for Grade {
    fn id(&self) -> &'static str {
        "sup-grade"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(ws, cx, "GRADE certainty per supplement x outcome, summary of findings, evidence ladder", "SUP-16", SRC)?;
        ws.write_string_with_format(
            3,
            0,
            "Entry form: fill outcome, design (RCT / observational) and 0-2 per downgrade / upgrade. Rules: Ref_GRADE.",
            &f.note,
        )?;
        let total = nrows(d);
        table(ws, "tblSupGrade", HEAD, 0, HEADERS, total)?;
        let rule =
            |id: &str| format!("INDEX({},MATCH(\"{id}\",{},0))", REF_GRADE.range(d, "value"), REF_GRADE.range(d, "id"));
        let label = |lvl: u8| {
            format!("INDEX({},MATCH(\"GR-LEVEL-{lvl}\",{},0))", REF_GRADE.range(d, "rule"), REF_GRADE.range(d, "id"))
        };
        let (r0, r1) = (HEAD + 1, HEAD + total);
        for c in 1..=10u16 {
            inputs(ws, cx, r0, r1, c, None)?;
        }
        ws.add_data_validation(r0, 2, r1, 2, &DataValidation::new().allow_list_strings(&["RCT", "observational"])?)?;
        ws.add_data_validation(r0, 3, r1, 8, &DataValidation::new().allow_list_strings(&["0", "1", "2"])?)?;
        ws.add_data_validation(r0, 9, r1, 10, &DataValidation::new().allow_list_strings(&["0", "1"])?)?;
        inputs(ws, cx, r0 + d.supplements.len() as u32, r1, 0, None)?;
        ws.add_data_validation(
            r0 + d.supplements.len() as u32,
            0,
            r1,
            0,
            &DataValidation::new().allow_list_formula(Formula::new(format!("={}", book::range(cx, "name")))),
        )?;
        for c in 18..=20u16 {
            inputs(ws, cx, r0, r1, c, None)?;
        }
        for i in 0..total {
            let (r, xr) = (HEAD + 1 + i, x(i));
            if i < d.supplements.len() as u32 {
                fx(ws, r, 0, &format!("={}", book::at("name", xr)), None)?;
                fx(ws, r, 16, &super::mirror(&data_cell(&TAGS, "indirectness", i)), None)?;
                fx(ws, r, 17, &super::mirror(&data_cell(&TAGS, "rung", i)), None)?;
            }
            fx(
                ws,
                r,
                11,
                &format!(
                    "=IF(C{xr}=\"RCT\",{},IF(C{xr}=\"observational\",{},\"\"))",
                    rule("GR-START-RCT"),
                    rule("GR-START-OBS")
                ),
                None,
            )?;
            fx(
                ws,
                r,
                12,
                &format!("=IF(L{xr}=\"\",\"\",MAX(1,MIN(4,L{xr}-SUM(D{xr}:H{xr})+SUM(I{xr}:K{xr}))))"),
                None,
            )?;
            fx(
                ws,
                r,
                13,
                &format!("=IF(M{xr}=\"\",\"\",CHOOSE(M{xr},{},{},{},{}))", label(1), label(2), label(3), label(4)),
                None,
            )?;
            fx(ws, r, 14, &format!("=IF(M{xr}=\"\",\"\",REPT(\"+\",M{xr})&REPT(\"o\",4-M{xr}))"), None)?;
            fx(
                ws,
                r,
                15,
                &format!("=IF(OR(N(F{xr})>0,Q{xr}<>\"\"),\"indirect: \"&IF(Q{xr}=\"\",\"downgraded\",Q{xr}),\"\")"),
                None,
            )?;
        }
        for (lvl, fill) in [(4, theme::OPTIMAL), (3, theme::REF_OK), (2, theme::OUT_OF_REF), (1, theme::CRITICAL)] {
            theme::cf_formula(ws, r0, 12, r1, 14, &format!("=$M{}={lvl}", r0 + 1), fill)?;
        }

        // Summary: certainty distribution and the evidence ladder (tblSuppTags.rung).
        let s = r1 + 3;
        ws.write_string_with_format(s - 1, 0, "Certainty distribution (rows graded)", &f.section)?;
        table(ws, "tblSupGradeSummary", s, 0, &["label", "level", "n_rows"], 4)?;
        let cert = format!("$M${}:$M${}", x(0), x(total - 1));
        for (k, lvl) in [4u8, 3, 2, 1].iter().enumerate() {
            let r = s + 1 + k as u32;
            fx(ws, r, 0, &format!("={}", label(*lvl)), None)?;
            ws.write_number(r, 1, f64::from(*lvl))?;
            fx(ws, r, 2, &format!("=COUNTIF({cert},B{})", r + 1), None)?;
        }
        let rungs = ["RCT", "biomarker", "animal", "mechanistic"];
        ws.write_string_with_format(s - 1, 4, "Evidence ladder (tblSuppTags.rung)", &f.section)?;
        table(ws, "tblSupLadder", s, 4, &["rung", "n_supplements"], rungs.len() as u32)?;
        let tag_rung = TAGS.range(d, "rung");
        for (k, rung) in rungs.iter().enumerate() {
            let r = s + 1 + k as u32;
            ws.write_string(r, 4, *rung)?;
            fx(ws, r, 5, &format!("=COUNTIF({tag_rung},E{})", r + 1), None)?;
        }
        let mut chart = Chart::new(ChartType::BarStacked);
        chart.add_series().set_name("rows graded").set_categories((TITLE, s + 1, 0, s + 4, 0)).set_values((
            TITLE,
            s + 1,
            2,
            s + 4,
            2,
        ));
        chart.add_series().set_name("supplements per rung").set_categories((TITLE, s + 1, 4, s + 4, 4)).set_values((
            TITLE,
            s + 1,
            5,
            s + 4,
            5,
        ));
        chart.title().set_name("Certainty and evidence ladder");
        chart.set_width(560).set_height(260);
        ws.insert_chart(s + 7, 0, &chart)?;
        let mut w = vec![28.0, 22.0, 13.0];
        w.extend(std::iter::repeat_n(8.0, 8));
        w.extend([6.0, 8.0, 10.0, 7.0, 20.0, 16.0, 11.0, 8.0, 10.0, 18.0]);
        set_widths(ws, &w)?;
        ws.set_freeze_panes(HEAD + 1, 1)?;
        Ok(())
    }
}
