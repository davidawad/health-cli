//! `Alerts` (GLB-03): tiered alerts per marker plus a live sorted alert list.
//!
//! Tier = the `Calc_Marker` status of the latest draw: 1 watch (left the optimal
//! range), 2 action (left the reference range), 3 critical (beyond a critical limit).
//! Critical limits stay blank in `tblMarkers` unless sourced (MED-VERIFY), so tier 3
//! only fires for markers whose limits were typed. Rows are aligned with `tblMarkers`;
//! the sorted list uses LARGE over a numeric sort key (every Excel), and an Excel 365
//! `TAKE(SORTBY(...))` spill shows the same list. KPIs `K_GLB_alerts` (action +
//! critical), `K_GLB_watch`, `K_GLB_critical`.

use rust_xlsxwriter::{TableStyle, Worksheet};

use super::{calc_marker, data_cell, fx, head, kpi, kv, mirror, TOP};
use crate::error::Result;
use crate::infra::data::MARKERS;
use crate::infra::theme;
use crate::workbook::{add_table, col, range, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Alerts";
pub const HEADERS: &[&str] = &[
    "marker_id",
    "name",
    "system",
    "last",
    "last_date",
    "tier",
    "alert",
    "distance",
    "z_latest",
    "days_since",
    "critical_set",
    "sort_key",
];
/// Zero-based header row of the per-marker table.
pub const HEAD: u32 = TOP + 7;
/// First column of the sorted list.
const LC: u16 = 13;

/// Number of marker rows.
pub fn n(d: &Data) -> u32 {
    MARKERS.nrows(d)
}

/// Absolute range of a per-marker column (`Alerts!$F$14:$F$70`).
pub fn rng(h: &str, d: &Data) -> String {
    let c = HEADERS.iter().position(|x| *x == h).unwrap_or_else(|| panic!("no Alerts column {h}")) as u16;
    range(TITLE, c, HEAD + 2, HEAD + 1 + n(d))
}

pub struct Alerts;

impl SheetModule for Alerts {
    fn id(&self) -> &'static str {
        "glb-alerts"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn available(&self, d: &Data) -> bool {
        d.labs.is_some()
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(ws, cx, "Tiered alerts: watch, action, critical", "GLB-03", "KAU Ch23 (stop-loss levels); quant Q66")?;
        theme::unverified(
            ws,
            3,
            0,
            "critical limits (tblMarkers critical_lo / critical_hi) stay blank unless sourced - MED-VERIFY",
            f,
        )?;
        let rows = (MARKERS.rows)(d).len() as u32;
        let nn = n(d);
        let (x0, x1) = (HEAD + 2, HEAD + 1 + nn);
        let r = |c: &str| format!("${c}${x0}:${c}${x1}");

        kv(ws, cx, TOP, 0, "Watch (left optimal)", &format!("=COUNTIF({},1)", r("F")), None)?;
        kv(ws, cx, TOP + 1, 0, "Action (out of reference)", &format!("=COUNTIF({},2)", r("F")), None)?;
        kv(ws, cx, TOP + 2, 0, "Critical (beyond a sourced limit)", &format!("=COUNTIF({},3)", r("F")), None)?;
        kv(ws, cx, TOP + 3, 0, "Alerts (action + critical)", &format!("=B{}+B{}", TOP + 2, TOP + 3), None)?;
        kv(ws, cx, TOP + 4, 0, "Markers with critical limits set", &format!("=COUNTIF({},\"YES\")", r("K")), None)?;
        kpi(cx, "alerts", "Alerts: markers out of reference or beyond a critical limit", TITLE, TOP + 3, 1, None)?;
        kpi(cx, "watch", "Watch: markers in reference but outside optimal", TITLE, TOP, 1, None)?;
        kpi(cx, "critical", "Critical: markers beyond a sourced critical limit", TITLE, TOP + 2, 1, None)?;

        ws.write_string_with_format(
            HEAD - 1,
            0,
            "Per marker (row-aligned with tblMarkers and Calc_Marker)",
            &f.section,
        )?;
        add_table(ws, "tblGlbAlerts", HEAD, 0, HEADERS, rows as usize, TableStyle::Light9)?;
        for i in 0..rows {
            let (row, x) = (HEAD + 1 + i, HEAD + 2 + i);
            let m = |h: &str| calc_marker(h, i);
            fx(ws, row, 0, &mirror(&data_cell(&MARKERS, "marker_id", i)), None)?;
            fx(ws, row, 1, &mirror(&data_cell(&MARKERS, "name", i)), None)?;
            fx(ws, row, 2, &mirror(&data_cell(&MARKERS, "system", i)), None)?;
            fx(ws, row, 3, &format!("=IF(ISNUMBER({0}),{0},\"\")", m("last")), None)?;
            fx(ws, row, 4, &format!("=IF(ISNUMBER({0}),{0},\"\")", m("last_date")), Some(theme::nf::DATE))?;
            fx(ws, row, 5, &format!("=IF(ISNUMBER({0}),{0},\"\")", m("status")), None)?;
            fx(
                ws,
                row,
                6,
                &format!("=IF(F{x}=\"\",\"\",CHOOSE(F{x}+1,\"ok\",\"watch\",\"action\",\"critical\"))"),
                None,
            )?;
            fx(ws, row, 7, &format!("=IF(ISNUMBER({0}),{0},\"\")", m("distance")), Some(theme::nf::DEC2))?;
            fx(ws, row, 8, &format!("=IF(ISNUMBER({0}),{0},\"\")", m("z_latest")), Some(theme::nf::DEC2))?;
            fx(ws, row, 9, &format!("=IF(ISNUMBER({0}),{0},\"\")", m("days_since_last")), None)?;
            let (lo, hi) = (data_cell(&MARKERS, "critical_lo", i), data_cell(&MARKERS, "critical_hi", i));
            fx(ws, row, 10, &format!("=IF(COUNT({lo},{hi})>0,\"YES\",\"NO\")"), None)?;
            fx(
                ws,
                row,
                11,
                &format!("=IF(N(F{x})>=1,F{x}*1000+MIN(999,100*N(H{x})+ABS(N(I{x})))+ROW()/1000000,0)"),
                None,
            )?;
        }
        let (b0, b1) = (HEAD + 1, HEAD + nn);
        for (code, fill) in [(1, theme::REF_OK), (2, theme::OUT_OF_REF), (3, theme::CRITICAL)] {
            theme::cf_formula(ws, b0, 5, b1, 6, &format!("=$F{}={code}", b0 + 1), fill)?;
        }
        ws.set_column_hidden(11)?;

        // Sorted alert list (classic formulas).
        ws.write_string_with_format(HEAD - 1, LC, "Live alert list, most severe first", &f.section)?;
        let lh = ["k", "marker", "alert", "last", "last_date", "distance", "z_latest"];
        add_table(ws, "tblGlbAlertList", HEAD, LC, &lh, nn as usize, TableStyle::Light9)?;
        let key = r("L");
        for k in 0..nn {
            let (row, x) = (HEAD + 1 + k, HEAD + 2 + k);
            let kc = col(LC);
            ws.write_number(row, LC, f64::from(k + 1))?;
            let m = format!("MATCH(LARGE({key},{kc}{x}),{key},0)");
            let pick = |c: &str| format!("=IF(LARGE({key},{kc}{x})<=0,\"\",INDEX({},{m}))", r(c));
            fx(ws, row, LC + 1, &pick("B"), None)?;
            fx(ws, row, LC + 2, &pick("G"), None)?;
            fx(ws, row, LC + 3, &pick("D"), None)?;
            fx(ws, row, LC + 4, &pick("E"), Some(theme::nf::DATE))?;
            fx(ws, row, LC + 5, &pick("H"), Some(theme::nf::DEC2))?;
            fx(ws, row, LC + 6, &pick("I"), Some(theme::nf::DEC2))?;
        }
        let ac = format!("${}{}", col(LC + 2), b0 + 1);
        for (t, fill) in [("watch", theme::REF_OK), ("action", theme::OUT_OF_REF), ("critical", theme::CRITICAL)] {
            theme::cf_formula(ws, b0, LC + 1, b1, LC + 2, &format!("={ac}=\"{t}\""), fill)?;
        }

        // The same list as one Excel 365 spill (TAKE / SORTBY over the alert rows).
        let sc = LC + 8;
        ws.write_string_with_format(HEAD - 1, sc, "Excel 365: TAKE(SORTBY(...))", &f.section)?;
        ws.write_string_with_format(HEAD, sc, "marker", &f.header)?;
        ws.write_string_with_format(HEAD, sc + 1, "alert", &f.header)?;
        theme::dyn_formula(
            ws,
            HEAD + 1,
            sc,
            &format!(
                "=IFERROR(IF(COUNTIF({key},\">0\")=0,\"no alerts\",TAKE(SORTBY(CHOOSE({{1,2}},{},{}),{key},-1),COUNTIF({key},\">0\"))),\"no alerts\")",
                r("B"),
                r("G")
            ),
        )?;
        set_widths(ws, &[34.0, 30.0, 14.0, 9.0, 11.0, 6.0, 9.0, 9.0, 9.0, 10.0, 11.0, 4.0, 3.0, 5.0, 30.0, 9.0])?;
        for (c, w) in
            [(LC + 3, 9.0), (LC + 4, 11.0), (LC + 5, 9.0), (LC + 6, 9.0), (LC + 7, 3.0), (sc, 30.0), (sc + 1, 9.0)]
        {
            ws.set_column_width(c, w)?;
        }
        ws.set_freeze_panes(HEAD + 1, 2)?;
        Ok(())
    }
}
