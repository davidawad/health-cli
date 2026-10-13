//! BOD-18 `Bod_Protein` (clinical.md C3.9, C4.5; longevity.md D1; quant Q83;
//! BUILDABLE-NOW: `tblDaily.protein_g` comes from the intake import, so MASTER's
//! assumption A1 is met and the PH becomes BN).
//!
//! Row-aligned with `Bod_Cal`: protein g/kg of the carried-forward weight, its band
//! (`Protein_Floor/Mid/High/Max_g_kg`, OUT ch50), `at_target` (1 when g/kg >=
//! `Protein_Mid_g_kg`, 0 on a logged miss, blank when protein was not logged), a
//! run-length streak of consecutive logged days at target (a logged miss resets it;
//! an UNLOGGED day carries it unchanged, so a forgotten log neither breaks nor extends
//! a streak), and per-meal / daily targets `Bod_Protein_meal_g_lb x weight x Bod_Meals`
//! (OUT ch50:9-17). Summary: % of logged days at target (COUNTIFS), mean g/kg, longest
//! / current / mean streak and the number of streaks (a streak starts where the
//! counter is 1 on a target day). Kidney overlay: the latest eGFR from `Calc_Marker`
//! when labs exist (else "no eGFR on file"); below `Bod_eGFR_kidney` the ceiling is
//! `Bod_Protein_kidney_g_kg` instead of `Protein_Max_g_kg` (both UNVERIFIED, KDIGO).
//! KPI `K_BOD_protein_days_pct`.

use rust_xlsxwriter::Worksheet;

use super::{abs, at, cal_at, cal_head, head, kpi, kv, last_num, n_days, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::calc::{MARKER_HEADERS, MARKER_TITLE};
use crate::infra::data::{LABS, MARKERS};
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Protein";
const HR: u32 = TOP + 1;
const C_W: u16 = 2;
const C_PKG: u16 = 4;
const C_BAND: u16 = 5;
const C_AT: u16 = 6;
const C_STREAK: u16 = 7;
const C_CEIL: u16 = 8;
const C_ABOVE: u16 = 9;
const C_MEAL: u16 = 10;
const C_DAILY: u16 = 11;
const C_PLOT: u16 = 12;
/// Summary column.
const SC: u16 = 18;

/// `Calc_Marker.<header>` of marker `slug` ("" when absent).
fn marker_stat(cx: &Cx, header: &str, slug: &str) -> String {
    let n = MARKERS.nrows(cx.d);
    let c = |h: &str| {
        let i = MARKER_HEADERS.iter().position(|x| *x == h).expect("Calc_Marker column") as u16;
        crate::workbook::range(MARKER_TITLE, i, 2, 1 + n)
    };
    format!("IFERROR(INDEX({},MATCH(\"{slug}\",{},0)),\"\")", c(header), c("marker_id"))
}

/// Band label of the g/kg value in cell `p`.
fn band(p: &str) -> String {
    let t = |n: &str| format!("TEXT({n},\"0.0\")");
    let (f, m, h, x) = (t("Protein_Floor_g_kg"), t("Protein_Mid_g_kg"), t("Protein_High_g_kg"), t("Protein_Max_g_kg"));
    format!(
        "=IF(ISNUMBER({p}),IF({p}<Protein_Floor_g_kg,\"below \"&{f},IF({p}<Protein_Mid_g_kg,{f}&\"-\"&{m},\
         IF({p}<Protein_High_g_kg,{m}&\"-\"&{h},IF({p}<=Protein_Max_g_kg,{h}&\"-\"&{x},\"above \"&{x})))),\"\")"
    )
}

pub struct Protein;

impl SheetModule for Protein {
    fn id(&self) -> &'static str {
        "bod-protein"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        &["bod-cal", "data-daily"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        head(
            ws,
            cx,
            "Protein g/kg bands, eGFR ceiling, % days at target, streaks",
            "BOD-18",
            "C3.9, C4.5, L-D1, Q83 | OUT ch50:9-17; HELMS ch01:1435-1467; KAU Ch22; kidney ceiling KDIGO (UNVERIFIED)",
            Banner::Unverified("KDIGO 2012 CKD guideline / KDIGO 2020 nutrition update (eGFR ceiling)"),
        )?;
        let n = n_days(cx);
        let (r0, r1) = (HR + 1, HR + n);
        let first = HR + 2;
        let s = TOP;
        let ceil_cell = abs(SC + 1, s + 14);
        section(ws, cx, TOP, 0, "Protein per calendar day (streak: an unlogged day carries the streak)")?;
        let mut cols = cal_head(HR);
        cols.push(Col::f("protein_g", move |r| format!("={}", cal_at("protein_g", HR, r))));
        cols.push(Col::f("protein_g_kg", move |r| format!("={}", cal_at("protein_g_kg", HR, r))));
        cols.push(Col::f("band", move |r| band(&at(C_PKG, r))));
        cols.push(Col::f("at_target", move |r| {
            let p = at(C_PKG, r);
            format!("=IF(ISNUMBER({p}),IF({p}>=Protein_Mid_g_kg,1,0),\"\")")
        }));
        cols.push(Col::f("streak", move |r| {
            let t = at(C_AT, r);
            if r == first {
                format!("=IF(ISNUMBER({t}),{t},0)")
            } else {
                let p = at(C_STREAK, r - 1);
                format!("=IF(ISNUMBER({t}),IF({t}=1,{p}+1,0),{p})")
            }
        }));
        let cc = ceil_cell.clone();
        cols.push(Col::f("ceiling_g_kg", move |_| format!("={cc}")));
        cols.push(Col::f("above_ceiling", move |r| {
            let (p, c) = (at(C_PKG, r), at(C_CEIL, r));
            format!("=IF(AND(ISNUMBER({p}),ISNUMBER({c})),IF({p}>{c},1,0),\"\")")
        }));
        cols.push(Col::f("meal_target_g", move |r| {
            let w = at(C_W, r);
            format!("=IF(ISNUMBER({w}),Bod_Protein_meal_g_lb*{w},\"\")")
        }));
        cols.push(Col::f("daily_target_g", move |r| {
            let m = at(C_MEAL, r);
            format!("=IF(ISNUMBER({m}),{m}*Bod_Meals,\"\")")
        }));
        cols.push(super::plot_col("g_kg_plot", C_PKG));
        for (h, name) in [
            ("floor_line", "Protein_Floor_g_kg"),
            ("mid_line", "Protein_Mid_g_kg"),
            ("high_line", "Protein_High_g_kg"),
            ("max_line", "Protein_Max_g_kg"),
        ] {
            cols.push(Col::f(h, move |_| format!("={name}")));
        }
        table(ws, cx, "tblBodProtein", HR, 0, &cols, n)?;
        super::date_rows(ws, cx, 0, r0, r1)?;
        super::fmt_rows(ws, r0, r1, C_PKG, C_PKG, "0.00")?;
        super::fmt_rows(ws, r0, r1, C_CEIL, C_CEIL, "0.0")?;
        super::fmt_rows(ws, r0, r1, C_MEAL, C_DAILY, "0")?;
        let p = at(C_PKG, r0 + 1);
        theme::cf_formula(
            ws,
            r0,
            C_PKG,
            r1,
            C_BAND,
            &format!("=AND(ISNUMBER({p}),{p}<Protein_Floor_g_kg)"),
            theme::OUT_OF_REF,
        )?;
        theme::cf_formula(ws, r0, C_PKG, r1, C_BAND, &format!("=AND(ISNUMBER({p}),{p}>{ceil_cell})"), theme::CRITICAL)?;
        theme::cf_formula(
            ws,
            r0,
            C_PKG,
            r1,
            C_BAND,
            &format!("=AND(ISNUMBER({p}),{p}>=Protein_Mid_g_kg)"),
            theme::OPTIMAL,
        )?;
        theme::cf_data_bar(ws, r0, C_STREAK, r1, C_STREAK, theme::BAND_OPT)?;

        // Summary.
        let (pkg, tgt, streak, above) = (
            rng(TITLE, C_PKG, r0, r1),
            rng(TITLE, C_AT, r0, r1),
            rng(TITLE, C_STREAK, r0, r1),
            rng(TITLE, C_ABOVE, r0, r1),
        );
        section(ws, cx, s, SC, "Summary")?;
        kv(ws, cx, s + 1, SC, "Logged days (protein)", &format!("=COUNT({pkg})"), Some("0"))?;
        kv(
            ws,
            cx,
            s + 2,
            SC,
            "Days at target (% of logged)",
            &format!("=IFERROR(COUNTIFS({tgt},1)/COUNT({tgt}),\"\")"),
            Some("0.0%"),
        )?;
        kv(ws, cx, s + 3, SC, "Mean protein (g/kg)", &format!("=IFERROR(AVERAGE({pkg}),\"\")"), Some("0.00"))?;
        kv(ws, cx, s + 4, SC, "Longest streak (days)", &format!("=MAX({streak})"), Some("0"))?;
        kv(ws, cx, s + 5, SC, "Current streak (days)", &format!("=INDEX({streak},{n})"), Some("0"))?;
        kv(ws, cx, s + 6, SC, "Number of streaks", &format!("=COUNTIFS({streak},1,{tgt},1)"), Some("0"))?;
        let ns = abs(SC + 1, s + 6);
        kv(
            ws,
            cx,
            s + 7,
            SC,
            "Mean streak length (days)",
            &format!("=IF({ns}>0,COUNTIFS({tgt},1)/{ns},\"\")"),
            Some("0.0"),
        )?;
        let meal = rng(TITLE, C_MEAL, r0, r1);
        kv(ws, cx, s + 8, SC, "Per-meal target now (g)", &format!("={}", last_num(&meal)), Some("0"))?;
        let daily = rng(TITLE, C_DAILY, r0, r1);
        kv(ws, cx, s + 9, SC, "Daily target now (g)", &format!("={}", last_num(&daily)), Some("0"))?;
        kpi(cx, "protein_days_pct", "Protein days at target (% of logged)", TITLE, s + 2, SC + 1, Some("0%"))?;

        // Kidney overlay.
        section(ws, cx, s + 11, SC, "Kidney-safe ceiling (eGFR)")?;
        theme::unverified(
            ws,
            s + 11,
            SC + 2,
            "KDIGO 2012 CKD guideline (Bod_eGFR_kidney, Bod_Protein_kidney_g_kg)",
            cx.f,
        )?;
        if cx.d.labs.is_some() {
            let ids = LABS.range(cx.d, "marker_id");
            kv(
                ws,
                cx,
                s + 12,
                SC,
                "Latest eGFR (mL/min/1.73m2)",
                &format!("=IF(COUNTIFS({ids},\"egfr\")=0,\"\",{})", marker_stat(cx, "last", "egfr")),
                Some("0"),
            )?;
        } else {
            ws.write_string_with_format(s + 12, SC, "Latest eGFR (mL/min/1.73m2)", &cx.f.bold)?;
            ws.write_string_with_format(s + 12, SC + 1, "no eGFR on file", &cx.f.note)?;
        }
        let e = abs(SC + 1, s + 12);
        kv(
            ws,
            cx,
            s + 13,
            SC,
            "eGFR status",
            &format!(
                "=IF(ISNUMBER({e}),IF({e}<Bod_eGFR_kidney,\"below Bod_eGFR_kidney: kidney ceiling applies\",\
                 \"at or above Bod_eGFR_kidney\"),\"no eGFR on file\")"
            ),
            None,
        )?;
        kv(
            ws,
            cx,
            s + 14,
            SC,
            "Ceiling (g/kg)",
            &format!(
                "=IF(ISNUMBER({e}),IF({e}<Bod_eGFR_kidney,Bod_Protein_kidney_g_kg,Protein_Max_g_kg),Protein_Max_g_kg)"
            ),
            Some("0.0"),
        )?;
        kv(ws, cx, s + 15, SC, "Days above ceiling", &format!("=SUM({above})"), Some("0"))?;
        theme::cf_formula(ws, s + 15, SC + 1, s + 15, SC + 1, &format!("={}>0", abs(SC + 1, s + 15)), theme::CRITICAL)?;

        let chart = super::line_chart(
            TITLE,
            "Protein g/kg with band lines",
            r0,
            r1,
            0,
            &[
                ("g/kg", C_PLOT),
                ("floor", C_PLOT + 1),
                ("mid (target)", C_PLOT + 2),
                ("high", C_PLOT + 3),
                ("max", C_PLOT + 4),
            ],
        );
        ws.insert_chart(s + 18, SC, &chart)?;
        set_widths(
            ws,
            &[
                12.0, 10.0, 10.0, 9.0, 11.0, 10.0, 9.0, 7.0, 11.0, 12.0, 13.0, 13.0, 9.0, 9.0, 9.0, 9.0, 9.0, 2.0,
                30.0, 26.0,
            ],
        )?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
