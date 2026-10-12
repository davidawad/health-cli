//! `Sup_Audit` (SUP-11): indication / effectiveness / safety / adherence lights per
//! supplement, medication-therapy-problem tags (DiPiro categories), and the gap
//! analysis: abnormal markers with no intervention, interventions with no marker.
//! The supplement -> marker links are an entry table (`tblSupTargets`).

use rust_xlsxwriter::{DataValidation, Formula, Worksheet};

use super::{book, fx, head, inputs, interact, mirror, supp_cell, table, x, HEAD};
use crate::error::Result;
use crate::infra::calc::{MARKER_HEADERS, MARKER_TITLE};
use crate::infra::data::{EVIDENCE, MARKERS};
use crate::infra::theme;
use crate::workbook::{col, range, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Sup_Audit";
const HEADERS: &[&str] = &[
    "name",
    "taking",
    "indication",
    "n_supporting",
    "n_warning",
    "n_interactions",
    "indication_light",
    "effectiveness_light",
    "safety_light",
    "adherence_light",
    "worst",
    "mtp_tag",
    "mtp_manual",
    "markers_tracked",
    "gap",
];

/// DiPiro medication-therapy-problem categories (DIP ch07.md:119).
pub const MTP: &[&str] = &[
    "needs additional therapy",
    "unnecessary drug therapy",
    "wrong drug",
    "dose too low",
    "dose too high",
    "adverse drug reaction",
    "adherence",
];
/// Entry rows of the supplement -> marker link table.
const TARGET_ROWS: u32 = 20;
/// First column of the target and marker-gap blocks.
const GC: u16 = 17;

pub struct Audit;

impl SheetModule for Audit {
    fn id(&self) -> &'static str {
        "sup-audit"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Indication / effectiveness / safety / adherence audit and gap analysis",
            "SUP-11",
            "DIP ch07:105-119",
        )?;
        ws.write_string_with_format(
            3,
            0,
            "Lights: 2 green, 1 amber, 0 red. Tags are prompts for a clinician conversation, not findings.",
            &f.note,
        )?;
        let rows = super::n(d);
        table(ws, "tblSupAudit", HEAD, 0, HEADERS, rows)?;
        let nev = EVIDENCE.nrows(d);
        let (ev_supp, ev_stance) = (EVIDENCE.range_n(nev, "supp_id"), EVIDENCE.range_n(nev, "stance"));
        let t0 = HEAD + 2;
        let tl = t0 + TARGET_ROWS - 1;
        let (t_supp, t_marker) = (range(TITLE, GC, t0, tl), range(TITLE, GC + 1, t0, tl));
        for i in 0..d.supplements.len() as u32 {
            let (r, xr) = (HEAD + 1 + i, x(i));
            fx(ws, r, 0, &format!("={}", book::at("name", xr)), None)?;
            fx(ws, r, 1, &mirror(&book::at("taking", xr)), None)?;
            let (p, notes) = (book::at("protocols", xr), supp_cell("notes", i));
            fx(ws, r, 2, &format!("=IF({p}<>\"\",{p},IF({notes}<>\"\",{notes},\"\"))"), None)?;
            let id = book::at("supp_id", xr);
            fx(ws, r, 3, &format!("=COUNTIFS({ev_supp},{id},{ev_stance},\"supporting\")"), None)?;
            fx(ws, r, 4, &format!("=COUNTIFS({ev_supp},{id},{ev_stance},\"warning\")"), None)?;
            fx(ws, r, 5, &format!("={}", interact::count_at(xr)), None)?;
            fx(ws, r, 6, &format!("=IF(C{xr}<>\"\",2,0)"), None)?;
            fx(ws, r, 7, &format!("=IF(D{xr}>0,2,IF({}>0,1,0))", book::at("n_refs", xr)), None)?;
            fx(ws, r, 8, &format!("=IF(E{xr}>0,0,IF(F{xr}>0,1,2))"), None)?;
            fx(ws, r, 9, &format!("=IF(B{xr}<>\"YES\",\"\",IF(N({})>0,2,1))", book::at("doses_per_wk", xr)), None)?;
            fx(ws, r, 10, &format!("=MIN(G{xr}:J{xr})"), None)?;
            fx(
                ws,
                r,
                11,
                &format!(
                    "=IF(B{xr}<>\"YES\",\"\",IF(G{xr}=0,\"{}?\",IF(E{xr}>0,\"{}?\",IF(H{xr}=0,\"{}? (no evidence on file)\",IF(J{xr}=1,\"{}?\",\"\")))))",
                    MTP[1], MTP[5], MTP[2], MTP[6]
                ),
                None,
            )?;
            fx(ws, r, 13, &format!("=COUNTIF({t_supp},A{xr})"), None)?;
            fx(ws, r, 14, &format!("=IF(AND(B{xr}=\"YES\",N{xr}=0),\"intervention without marker\",\"\")"), None)?;
        }
        let (r0, r1) = (HEAD + 1, HEAD + rows);
        theme::cf_icons(ws, r0, 6, r1, 10)?;
        inputs(ws, cx, r0, r1, 12, Some(MTP))?;
        theme::cf_formula(ws, r0, 0, r1, 0, &format!("=AND($B{}=\"YES\",$K{}=0)", r0 + 1, r0 + 1), theme::OUT_OF_REF)?;
        let (x0, xl) = (x(0), super::last(d));
        super::kpi(
            cx,
            "n_audit_red",
            "Taken supplements with a red audit light",
            &format!("COUNTIFS({TITLE}!$B${x0}:$B${xl},\"YES\",{TITLE}!$K${x0}:$K${xl},0)"),
            None,
        )?;

        // Gap analysis: supplement -> marker links (entry), then markers out of range with none.
        ws.write_string_with_format(HEAD - 1, GC, "Supplement -> marker it should move (entry)", &f.section)?;
        table(ws, "tblSupTargets", HEAD, GC, &["supplement", "marker_id"], TARGET_ROWS)?;
        inputs(ws, cx, HEAD + 1, HEAD + TARGET_ROWS, GC, None)?;
        inputs(ws, cx, HEAD + 1, HEAD + TARGET_ROWS, GC + 1, None)?;
        ws.add_data_validation(
            HEAD + 1,
            GC,
            HEAD + TARGET_ROWS,
            GC,
            &DataValidation::new().allow_list_formula(Formula::new(format!("={}", book::range(cx, "name")))),
        )?;
        let mc = GC + 3;
        ws.write_string_with_format(HEAD - 1, mc, "Markers out of reference with no intervention", &f.section)?;
        if d.labs.is_none() {
            theme::awaiting(ws, HEAD, "lab data (biomarker CLI) for the marker side of the gap analysis", f)?;
        } else {
            let nm = MARKERS.nrows(d);
            ws.add_data_validation(
                HEAD + 1,
                GC + 1,
                HEAD + TARGET_ROWS,
                GC + 1,
                &DataValidation::new()
                    .allow_list_formula(Formula::new(format!("={}", MARKERS.range_n(nm, "marker_id")))),
            )?;
            table(ws, "tblSupMarkerGap", HEAD, mc, &["marker_id", "status", "n_interventions", "gap"], nm)?;
            let status_c = MARKER_HEADERS.iter().position(|h| *h == "status").unwrap_or(13) as u16;
            for j in 0..nm {
                let (r, xr) = (HEAD + 1 + j, HEAD + 2 + j);
                let m = format!("{}{xr}", col(mc));
                fx(ws, r, mc, &format!("={}", super::data_cell(&MARKERS, "marker_id", j)), None)?;
                fx(ws, r, mc + 1, &format!("={}!${}${}", MARKER_TITLE, col(status_c), j + 2), None)?;
                fx(ws, r, mc + 2, &format!("=COUNTIF({t_marker},{m})"), None)?;
                fx(
                    ws,
                    r,
                    mc + 3,
                    &format!(
                        "=IF(AND(ISNUMBER({s}{xr}),N({s}{xr})>=2,{c}{xr}=0),\"marker without intervention\",\"\")",
                        s = col(mc + 1),
                        c = col(mc + 2)
                    ),
                    None,
                )?;
            }
            theme::cf_status_quilt(ws, HEAD + 1, mc + 1, HEAD + nm, mc + 1)?;
        }
        set_widths(
            ws,
            &[
                30.0, 7.0, 26.0, 8.0, 8.0, 9.0, 9.0, 11.0, 8.0, 10.0, 7.0, 34.0, 22.0, 9.0, 26.0, 3.0, 3.0, 28.0, 16.0,
                3.0, 18.0, 8.0, 10.0, 26.0,
            ],
        )?;
        ws.set_freeze_panes(HEAD + 1, 1)?;
        Ok(())
    }
}
