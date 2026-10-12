//! `Sup_TrialDose` (SUP-22): dose taken vs dose studied, with "no trial at this
//! dose" when the daily dose falls outside trial dose x (1 +/- `Sup_Trial_Tol`).
//! Trial doses are a rule table (`tblSupTrialDose`) seeded from the catalog; a
//! blue override column takes any other trial the user has read.

use rust_xlsxwriter::Worksheet;

use super::{book, fx, head, inputs, match_idx, mirror, table, write_rules, x, Rule, HEAD};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Sup_TrialDose";

const TRIALS: &[Rule] = &[
    (
        "TD-01",
        "st john",
        Some(900.0),
        "mg",
        "KAT ch80.md:247",
        "verified",
        "900 mg/day in 3 doses, 2-5% hyperforin extract",
    ),
    (
        "TD-02",
        "vitamin d",
        Some(4000.0),
        "IU",
        "SR-papers: vitamin D stress-fracture study",
        "verified",
        "~4,000 IU/day to reach >= 40 ng/mL",
    ),
    (
        "TD-03",
        "ashwagandha",
        Some(120.0),
        "mg",
        "SR-papers: ashwagandha SSRI-adjunct RCT",
        "verified",
        "120 mg/day, n = 15 per arm",
    ),
];
const HEADERS: &[&str] = &[
    "name",
    "taking",
    "daily_dose",
    "unit",
    "trial_dose",
    "trial_unit",
    "trial_override",
    "trial_used",
    "ratio",
    "reading",
];
/// First column of the trial rule table.
const TC: u16 = 12;

pub struct TrialDose;

impl SheetModule for TrialDose {
    fn id(&self) -> &'static str {
        "sup-trialdose"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(ws, cx, "Dose taken vs dose studied", "SUP-22", "KAT ch80:247; SR-papers")?;
        ws.write_string_with_format(
            3,
            0,
            "Override: type a trial dose (same unit) in the blue column when you have read another trial.",
            &f.note,
        )?;
        ws.write_string_with_format(HEAD - 1, TC, "Trial doses (tblSupTrialDose)", &f.section)?;
        let rr = write_rules(ws, cx, TITLE, "tblSupTrialDose", HEAD, TC, TRIALS)?;
        let (kw, dose, unit) = (&rr[1], &rr[2], &rr[3]);
        let rows = super::n(d);
        table(ws, "tblSupTrialCompare", HEAD, 0, HEADERS, rows)?;
        inputs(ws, cx, HEAD + 1, HEAD + rows, 6, None)?;
        for i in 0..d.supplements.len() as u32 {
            let (r, xr) = (HEAD + 1 + i, x(i));
            let idx = match_idx(kw, &format!("A{xr}"));
            fx(ws, r, 0, &format!("={}", book::at("name", xr)), None)?;
            fx(ws, r, 1, &mirror(&book::at("taking", xr)), None)?;
            fx(ws, r, 2, &mirror(&book::at("daily_dose", xr)), Some(&f.dec1))?;
            fx(ws, r, 3, &mirror(&book::at("unit", xr)), None)?;
            fx(ws, r, 4, &format!("=IF({idx}>0,INDEX({dose},{idx}),\"\")"), None)?;
            fx(ws, r, 5, &format!("=IF({idx}>0,INDEX({unit},{idx}),\"\")"), None)?;
            fx(ws, r, 7, &format!("=IF(ISNUMBER(G{xr}),G{xr},E{xr})"), None)?;
            fx(
                ws,
                r,
                8,
                &format!("=IF(AND(ISNUMBER(C{xr}),ISNUMBER(H{xr}),N(H{xr})>0),C{xr}/H{xr},\"\")"),
                Some(&f.dec2),
            )?;
            fx(
                ws,
                r,
                9,
                &format!(
                    "=IF(NOT(ISNUMBER(H{xr})),\"no trial dose on file\",IF(AND(NOT(ISNUMBER(G{xr})),D{xr}<>F{xr}),\"units differ\",IF(NOT(ISNUMBER(I{xr})),\"\",IF(ABS(I{xr}-1)<=Sup_Trial_Tol,\"within trial dose\",\"no trial at this dose\"))))"
                ),
                None,
            )?;
        }
        theme::cf_formula(
            ws,
            HEAD + 1,
            9,
            HEAD + rows,
            9,
            &format!("=$J{}=\"no trial at this dose\"", HEAD + 2),
            theme::OUT_OF_REF,
        )?;
        theme::cf_formula(
            ws,
            HEAD + 1,
            9,
            HEAD + rows,
            9,
            &format!("=$J{}=\"within trial dose\"", HEAD + 2),
            theme::OPTIMAL,
        )?;
        set_widths(
            ws,
            &[30.0, 7.0, 10.0, 6.0, 10.0, 9.0, 12.0, 10.0, 7.0, 24.0, 3.0, 3.0, 7.0, 12.0, 7.0, 5.0, 30.0, 9.0, 40.0],
        )?;
        ws.set_freeze_panes(HEAD + 1, 1)?;
        Ok(())
    }
}
