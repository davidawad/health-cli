//! Cln_RiskChecklist (CLN-08): Attia's priority markers - measured?, how often,
//! last date and value, a NOT TESTED tile and a sparkline over every panel.

use rust_xlsxwriter::{Formula, TableStyle, Worksheet};

use super::ids::{ALT, APOB, CRP, HDL, HOMOCYSTEINE, INSULIN, LPA, TG, URIC};
use super::kit::{self, Lab};
use crate::error::Result;
use crate::infra::data::PANELS;
use crate::infra::theme;
use crate::workbook::{add_table, col, set_widths, Cx};

pub const TITLE: &str = "Cln_RiskChecklist";
pub const TABLE: &str = "tblViewClnChecklist";

/// (item, marker slug or "" for TG/HDL, source).
const ITEMS: &[(&str, &str, &str)] = &[
    ("ApoB", APOB, "OUT ch15:7"),
    ("Lp(a) - once in a lifetime", LPA, "OUT ch14:157-167"),
    ("TG/HDL ratio", "", "OUT ch13:186"),
    ("Fasting insulin", INSULIN, "OUT ch13:186"),
    ("Uric acid", URIC, "OUT ch13:186"),
    ("Homocysteine", HOMOCYSTEINE, "KAT ch46:169"),
    ("ALT", ALT, "OUT ch13:186"),
    ("hs-CRP", CRP, "OUT ch13:186; DIP ch37:191"),
];

const HEADS: &[&str] = &["Item", "Marker", "Source", "n", "Last date", "Last value", "Days since", "Tile", "Trend"];
/// First column of the per-panel helper grid that feeds the sparklines.
const GRID: u16 = 11;

pub fn build(ws: &mut Worksheet, cx: &Cx) -> Result<()> {
    let (d, f) = (cx.d, cx.f);
    let lab = Lab::new(d);
    let row = kit::head(
        ws,
        cx,
        "Priority-marker checklist (Attia): measured?, last draw, trend",
        &["CLN-08 PLACEHOLDER: the view is complete; most priority markers have never been measured, so their tiles read NOT TESTED until they are."],
    )?;
    theme::awaiting(
        ws,
        row,
        "Lp(a), fasting insulin, uric acid, homocysteine, hs-CRP (not in the lab history yet)",
        f,
    )?;
    let hr = row + 2;
    let np = kit::n_panels(d);
    add_table(ws, TABLE, hr, 0, HEADS, ITEMS.len(), TableStyle::Light9)?;
    ws.write_string_with_format(hr - 1, GRID, "Per-panel values (feed the sparklines)", &f.note)?;
    let dc = col(PANELS.col("date"));
    for p in 0..np {
        let c = GRID + p as u16;
        ws.write_formula_with_format(
            hr,
            c,
            Formula::new(format!("=IF({s}!${dc}${x}=\"\",\"\",{s}!${dc}${x})", s = PANELS.sheet, x = p + 2)),
            &f.date,
        )?;
    }
    let (g0, g1) = (col(GRID), col(GRID + np as u16 - 1));
    for (i, (item, marker, source)) in ITEMS.iter().enumerate() {
        let r = hr + 1 + i as u32;
        let x = r + 1;
        ws.write_string(r, 0, *item)?;
        ws.write_string(r, 1, if marker.is_empty() { "triglycerides / hdl-c" } else { marker })?;
        ws.write_string(r, 2, *source)?;
        for p in 0..np {
            let c = GRID + p as u16;
            let date = format!("{}${}", col(c), hr + 1);
            let v = if marker.is_empty() {
                format!("=IFERROR({}/{},\"\")", lab.at(TG, &date), lab.at(HDL, &date))
            } else {
                format!("={}", lab.at(marker, &date))
            };
            ws.write_formula_with_format(r, c, Formula::new(v), &f.dec2)?;
        }
        let grid = format!("${g0}{x}:${g1}{x}");
        let dates = format!("${g0}${}:${g1}${}", hr + 1, hr + 1);
        ws.write_formula(r, 3, Formula::new(format!("=COUNT({grid})")))?;
        ws.write_formula_with_format(
            r,
            4,
            Formula::new(format!("=IF($D{x}=0,\"\",LOOKUP(2,1/ISNUMBER({grid}),{dates}))")),
            &f.date,
        )?;
        ws.write_formula_with_format(
            r,
            5,
            Formula::new(format!("=IF($D{x}=0,\"\",LOOKUP(2,1/ISNUMBER({grid}),{grid}))")),
            &f.dec2,
        )?;
        ws.write_formula(r, 6, Formula::new(format!("=IF($D{x}=0,\"\",TODAY()-$E{x})")))?;
        let tested = if *marker == LPA { "measured (once is enough)" } else { "measured" };
        ws.write_formula(
            r,
            7,
            Formula::new(format!("=IF($D{x}=0,\"NOT TESTED\",IF(AND(\"{marker}\"<>\"{LPA}\",$G{x}>365),\"stale (> 1 year)\",\"{tested}\"))")),
        )?;
        theme::note_source(ws, r, 2, source)?;
        theme::sparkline(ws, r, 8, &format!("{}!{grid}", TITLE))?;
    }
    let (t0, t1) = (hr + 1, hr + ITEMS.len() as u32);
    for (text, fill) in [("NOT TESTED", theme::CRITICAL), ("stale", theme::REF_OK), ("measured", theme::OPTIMAL)] {
        theme::cf_formula(ws, t0, 7, t1, 7, &format!("=ISNUMBER(SEARCH(\"{text}\",H{}))", t0 + 1), fill)?;
    }
    set_widths(ws, &[28.0, 22.0, 26.0, 6.0, 11.0, 10.0, 10.0, 24.0, 16.0])?;
    kit::kpi(
        cx,
        "priority_tested",
        "Priority markers measured (of 8)",
        &format!("COUNTIF({TITLE}!$D${}:$D${},\">0\")", t0 + 1, t1 + 1),
        None,
    )?;
    Ok(())
}
