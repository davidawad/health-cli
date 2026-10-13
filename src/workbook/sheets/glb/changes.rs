//! `Changes` (GLB-04): what changed since the last panel.
//!
//! Per marker (row-aligned with `tblMarkers`): value at the previous and the latest
//! panel (`Prev_Panel`, `Latest_Panel`), change, % change, move in personal SDs
//! (`Calc_Marker` sd), move in RCV units (`RCV_z x sqrt(2) x sqrt(CVa^2 + CVi^2)`;
//! blank until the CVs are typed, pending EFLM), the `Calc_Labs` status at both
//! panels, new and resolved alerts. Top movers up and down (LARGE over sort keys,
//! plus an Excel 365 `TAKE(SORTBY(...))` spill), `TEXTJOIN` narrative cells, and the
//! supplements started / stopped between the two panels (GATE-DATES: `tblSupp`
//! start / stop dates) plus `tblRegimes` events. KPIs `K_GLB_new_alerts`,
//! `K_GLB_top_mover`.

use rust_xlsxwriter::{Formula, TableStyle, Worksheet};

use super::{calc_marker, data_cell, fx, head, kpi, kv, mirror, TOP};
use crate::error::Result;
use crate::infra::calc::{LABS_HEADERS, LABS_TITLE};
use crate::infra::data::{LABS, MARKERS, PANELS, REGIMES, SUPP};
use crate::infra::theme;
use crate::workbook::{add_table, col, range, set_widths, Cx, Data, SheetModule};

pub const TITLE: &str = "Changes";
const HEADERS: &[&str] = &[
    "marker_id",
    "name",
    "previous",
    "latest",
    "change",
    "pct_change",
    "z_move",
    "rcv_pct",
    "rcv_units",
    "status_prev",
    "status_latest",
    "new_alert",
    "resolved",
    "up_key",
    "down_key",
];
/// Zero-based header row of the per-marker table.
const HEAD: u32 = TOP + 13;
/// Rows of the top-movers block.
const TOP_ROWS: u32 = 10;
/// First column of the top-movers block.
const MC: u16 = 16;

fn labs_col(h: &str) -> u16 {
    LABS_HEADERS.iter().position(|x| *x == h).unwrap_or_else(|| panic!("no Calc_Labs column {h}")) as u16
}

/// Whether any supplement carries a start or stop date (GATE-DATES).
pub fn has_supp_dates(d: &Data) -> bool {
    d.supplements.iter().any(|s| {
        ["start_date", "stop_date"].iter().any(|k| s.get(*k).is_some_and(|v| v.as_str().is_some_and(|t| !t.is_empty())))
    })
}

pub struct Changes;

impl SheetModule for Changes {
    fn id(&self) -> &'static str {
        "glb-changes"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn available(&self, d: &Data) -> bool {
        d.labs.is_some()
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(ws, cx, "What changed since the last panel", "GLB-04", "PAL Ch14 (ex-post attribution); quant Q94")?;
        if has_supp_dates(d) {
            ws.write_string_with_format(
                3,
                0,
                "Supplement start / stop dates present: interval list is live.",
                &f.note,
            )?;
        } else {
            theme::awaiting(
                ws,
                3,
                "tblSupp start_date / stop_date (GATE-DATES) for the supplements started or stopped in the interval",
                f,
            )?;
        }
        let rows = (MARKERS.rows)(d).len() as u32;
        let nm = MARKERS.nrows(d);
        let (x0, x1) = (HEAD + 2, HEAD + 1 + nm);
        let r = |c: &str| format!("${c}${x0}:${c}${x1}");
        let np = PANELS.nrows(d);
        let (pid, pdate) = (PANELS.range_n(np, "panel_id"), PANELS.range_n(np, "date"));
        let nl = LABS.nrows(d);
        let (lm, lp, lv) = (LABS.range_n(nl, "marker_id"), LABS.range_n(nl, "panel_id"), LABS.range_n(nl, "value"));
        let cl = |h: &str| range(LABS_TITLE, labs_col(h), 2, 1 + nl);

        // Header block: the two panels and the interval.
        // Panel ids on rows TOP and TOP + 1, dates on TOP + 2 and TOP + 3 (column B).
        let b = |r: u32| format!("$B${}", r + 1);
        let (lat, prev) = (b(TOP), b(TOP + 1));
        let (lat, prev) = (lat.as_str(), prev.as_str());
        kv(
            ws,
            cx,
            TOP,
            0,
            "Latest panel id",
            &format!("=IFERROR(INDEX({pid},MATCH(Latest_Panel,{pdate},0)),\"\")"),
            None,
        )?;
        kv(
            ws,
            cx,
            TOP + 1,
            0,
            "Previous panel id",
            &format!("=IFERROR(INDEX({pid},MATCH(Prev_Panel,{pdate},0)),\"\")"),
            None,
        )?;
        kv(ws, cx, TOP + 2, 0, "Latest panel date", "=Latest_Panel", Some(theme::nf::DATE))?;
        kv(ws, cx, TOP + 3, 0, "Previous panel date", "=Prev_Panel", Some(theme::nf::DATE))?;
        let (dl, dp) = (b(TOP + 2), b(TOP + 3));
        kv(
            ws,
            cx,
            TOP + 4,
            0,
            "Days between",
            &format!("=IF(AND(ISNUMBER({dl}),ISNUMBER({dp})),{dl}-{dp},\"\")"),
            None,
        )?;
        kv(ws, cx, TOP + 5, 0, "New alerts", &format!("=SUM({})", r("L")), None)?;
        kv(ws, cx, TOP + 6, 0, "Resolved alerts", &format!("=SUM({})", r("M")), None)?;
        kv(ws, cx, TOP + 7, 0, "Markers measured at both panels", &format!("=COUNT({})", r("E")), None)?;
        let names = r("B");
        let tj = |cond: &str, what: &str| format!("=IFERROR(TEXTJOIN(\", \",TRUE,IF({cond},{what},\"\")),\"\")");
        ws.write_string_with_format(TOP + 5, 3, "New:", &f.bold)?;
        ws.write_array_formula(TOP + 5, 4, TOP + 5, 4, Formula::new(tj(&format!("{}=1", r("L")), &names)))?;
        ws.write_string_with_format(TOP + 6, 3, "Resolved:", &f.bold)?;
        ws.write_array_formula(TOP + 6, 4, TOP + 6, 4, Formula::new(tj(&format!("{}=1", r("M")), &names)))?;
        kpi(cx, "new_alerts", "New alerts since the previous panel", TITLE, TOP + 5, 1, None)?;

        // Supplements and regime events in the interval.
        let ns = SUPP.nrows(d);
        let (sn, ss, se) = (SUPP.range_n(ns, "name"), SUPP.range_n(ns, "start_date"), SUPP.range_n(ns, "stop_date"));
        let within = |rg: &str| format!("ISNUMBER({rg})*({rg}>Prev_Panel)*({rg}<=Latest_Panel)");
        kv(
            ws,
            cx,
            TOP + 8,
            0,
            "Supplements started in interval",
            &format!("=IFERROR(SUMPRODUCT({}),0)", within(&ss)),
            None,
        )?;
        kv(
            ws,
            cx,
            TOP + 9,
            0,
            "Supplements stopped in interval",
            &format!("=IFERROR(SUMPRODUCT({}),0)", within(&se)),
            None,
        )?;
        ws.write_array_formula(TOP + 8, 4, TOP + 8, 4, Formula::new(tj(&format!("({})>0", within(&ss)), &sn)))?;
        ws.write_array_formula(TOP + 9, 4, TOP + 9, 4, Formula::new(tj(&format!("({})>0", within(&se)), &sn)))?;
        let nr = REGIMES.nrows(d);
        let (rd, rn, rk) = (REGIMES.range_n(nr, "date"), REGIMES.range_n(nr, "name"), REGIMES.range_n(nr, "kind"));
        kv(
            ws,
            cx,
            TOP + 10,
            0,
            "Regime events in interval",
            &format!("=IFERROR(SUMPRODUCT({}),0)", within(&rd)),
            None,
        )?;
        ws.write_array_formula(
            TOP + 10,
            4,
            TOP + 10,
            4,
            Formula::new(tj(&format!("({})>0", within(&rd)), &format!("{rk}&\": \"&{rn}"))),
        )?;
        ws.write_string_with_format(TOP + 8, 3, "Started:", &f.bold)?;
        ws.write_string_with_format(TOP + 9, 3, "Stopped:", &f.bold)?;
        ws.write_string_with_format(TOP + 10, 3, "Events:", &f.bold)?;

        // Per marker.
        ws.write_string_with_format(HEAD - 1, 0, "Per marker (row-aligned with tblMarkers)", &f.section)?;
        add_table(ws, "tblGlbChanges", HEAD, 0, HEADERS, rows as usize, TableStyle::Light9)?;
        for i in 0..rows {
            let (row, x) = (HEAD + 1 + i, HEAD + 2 + i);
            let m = format!("$A{x}");
            let at = |p: &str| {
                format!("=IF(OR({p}=\"\",COUNTIFS({lm},{m},{lp},{p})=0),\"\",SUMIFS({lv},{lm},{m},{lp},{p}))")
            };
            let st = |p: &str| {
                let (cp, cm, cs) = (cl("panel_id"), cl("marker_id"), cl("status"));
                format!(
                    "=IF(OR({p}=\"\",COUNTIFS({cp},{p},{cm},{m},{cs},\">=0\")=0),\"\",SUMIFS({cs},{cp},{p},{cm},{m}))"
                )
            };
            fx(ws, row, 0, &mirror(&data_cell(&MARKERS, "marker_id", i)), None)?;
            fx(ws, row, 1, &mirror(&data_cell(&MARKERS, "name", i)), None)?;
            fx(ws, row, 2, &at(prev), None)?;
            fx(ws, row, 3, &at(lat), None)?;
            fx(ws, row, 4, &format!("=IF(AND(ISNUMBER(C{x}),ISNUMBER(D{x})),D{x}-C{x},\"\")"), Some(theme::nf::DEC2))?;
            fx(ws, row, 5, &format!("=IF(AND(ISNUMBER(E{x}),N(C{x})<>0),E{x}/ABS(C{x}),\"\")"), Some(theme::nf::PCT1))?;
            let sd = calc_marker("sd", i);
            fx(
                ws,
                row,
                6,
                &format!("=IF(AND(ISNUMBER(E{x}),ISNUMBER({sd})),IF({sd}>0,E{x}/{sd},\"\"),\"\")"),
                Some(theme::nf::DEC2),
            )?;
            let (cvi, cva) = (data_cell(&MARKERS, "cvi", i), data_cell(&MARKERS, "cva", i));
            fx(
                ws,
                row,
                7,
                &format!("=IF(AND(ISNUMBER({cvi}),ISNUMBER({cva})),RCV_z*SQRT(2)*SQRT({cvi}^2+{cva}^2)/100,\"\")"),
                Some(theme::nf::PCT1),
            )?;
            fx(ws, row, 8, &format!("=IF(AND(ISNUMBER(F{x}),N(H{x})>0),F{x}/H{x},\"\")"), Some(theme::nf::DEC2))?;
            fx(ws, row, 9, &st(prev), None)?;
            fx(ws, row, 10, &st(lat), None)?;
            fx(ws, row, 11, &format!("=IF(AND(ISNUMBER(K{x}),N(K{x})>=2,OR(J{x}=\"\",N(J{x})<2)),1,0)"), None)?;
            fx(ws, row, 12, &format!("=IF(AND(ISNUMBER(J{x}),N(J{x})>=2,ISNUMBER(K{x}),N(K{x})<2),1,0)"), None)?;
            fx(ws, row, 13, &format!("=IF(N(G{x})>0,G{x}+ROW()/1000000000,0)"), None)?;
            fx(ws, row, 14, &format!("=IF(N(G{x})<0,-G{x}+ROW()/1000000000,0)"), None)?;
        }
        let (b0, b1) = (HEAD + 1, HEAD + nm);
        theme::cf_three_color(ws, b0, 6, b1, 6, true)?;
        theme::cf_formula(ws, b0, 11, b1, 11, &format!("=$L{}=1", b0 + 1), theme::OUT_OF_REF)?;
        theme::cf_formula(ws, b0, 12, b1, 12, &format!("=$M{}=1", b0 + 1), theme::OPTIMAL)?;
        theme::unverified(ws, HEAD - 1, 7, "CVi / CVa per marker (EFLM Biological Variation Database)", f)?;
        ws.set_column_hidden(13)?;
        ws.set_column_hidden(14)?;

        // Top movers.
        ws.write_string_with_format(HEAD - 1, MC, "Top movers in personal SDs (Glb_Top_n rows)", &f.section)?;
        let mh = ["k", "up", "z_up", "rcv_up", "down", "z_down", "rcv_down"];
        add_table(ws, "tblGlbMovers", HEAD, MC, &mh, TOP_ROWS as usize, TableStyle::Light9)?;
        let kc = col(MC);
        for k in 0..TOP_ROWS {
            let (row, x) = (HEAD + 1 + k, HEAD + 2 + k);
            ws.write_number(row, MC, f64::from(k + 1))?;
            for (c0, key) in [(MC + 1, r("N")), (MC + 4, r("O"))] {
                let pick = |src: String| {
                    format!(
                        "=IFERROR(IF(OR({kc}{x}>Glb_Top_n,LARGE({key},{kc}{x})<=0),\"\",INDEX({src},MATCH(LARGE({key},{kc}{x}),{key},0))),\"\")"
                    )
                };
                fx(ws, row, c0, &pick(r("B")), None)?;
                fx(ws, row, c0 + 1, &pick(r("G")), Some(theme::nf::DEC2))?;
                fx(ws, row, c0 + 2, &pick(r("I")), Some(theme::nf::DEC2))?;
            }
        }
        let top_up = format!("{}{}", col(MC + 1), HEAD + 2);
        kv(ws, cx, TOP + 11, 0, "Top mover (up, in SDs)", &format!("=IF({top_up}=\"\",\"none\",{top_up})"), None)?;
        kpi(cx, "top_mover", "Largest upward move since the previous panel (SDs)", TITLE, TOP + 11, 1, None)?;
        let sc = MC + 8;
        ws.write_string_with_format(HEAD - 1, sc, "Excel 365: TAKE(SORTBY(...))", &f.section)?;
        ws.write_string_with_format(HEAD, sc, "largest |z| moves", &f.header)?;
        theme::dyn_formula(
            ws,
            HEAD + 1,
            sc,
            &format!("=IFERROR(TAKE(SORTBY({names},{}+{},-1),Glb_Top_n),\"\")", r("N"), r("O")),
        )?;
        set_widths(
            ws,
            &[30.0, 26.0, 9.0, 9.0, 9.0, 9.0, 8.0, 8.0, 9.0, 10.0, 11.0, 9.0, 9.0, 3.0, 3.0, 3.0, 4.0, 24.0],
        )?;
        for (c, w) in [(MC + 2, 7.0), (MC + 3, 7.0), (MC + 4, 24.0), (MC + 5, 7.0), (MC + 6, 8.0), (sc, 24.0)] {
            ws.set_column_width(c, w)?;
        }
        ws.set_freeze_panes(HEAD + 1, 2)?;
        Ok(())
    }
}
