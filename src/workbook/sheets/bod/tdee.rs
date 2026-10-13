//! BOD-19 `Bod_TDEE` (clinical.md C4.4; longevity.md D3; BUILDABLE-NOW): energy
//! expenditure implied by intake and weight change.
//!
//! Row-aligned with `Bod_Cal`; for every day with a full window of `Bod_TDEE_days`
//! days behind it: mean logged kcal over the window (`AVERAGEIFS` by date, unlogged
//! days skipped), the change of the `Bod_Weight` EMA over the window in kg (lb /
//! `Bod_lb_per_kg`), and `implied TDEE = mean kcal - Bod_kcal_per_kg x dkg / days`.
//! The logged `energy_out_kcal` mean is shown for comparison. `Bod_kcal_per_kg`
//! (7,700) is the crude energy-density rule (HELMS ch01 l.957; Hall & Chow), kept as
//! a heuristic with a footnote. KPI `K_BOD_tdee` = latest implied TDEE.

use rust_xlsxwriter::Worksheet;

use super::{
    abs, at, cal_at, cal_col, cal_head, head, kpi, kv, last_num, n_days, rng, section, table, Banner, Col, TOP,
};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::sheets::bod::weight;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_TDEE";
const HR: u32 = TOP + 1;
const C_DATE: u16 = 0;
const C_DN: u16 = 3;
const C_EMA: u16 = 4;
const C_KMEAN: u16 = 6;
const C_OMEAN: u16 = 7;
const C_DKG: u16 = 9;
const C_TDEE: u16 = 10;
const C_PLOT: u16 = 11;
const SC: u16 = 16;

pub struct Tdee;

impl SheetModule for Tdee {
    fn id(&self) -> &'static str {
        "bod-tdee"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        &["bod-cal", "bod-weight", "data-daily"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        head(
            ws,
            cx,
            "Implied TDEE from intake vs weight change",
            "BOD-19",
            "C4.4, L-D3 | HELMS ch01:957 (3500 kcal/lb is a crude rule); DIP ch164:401-409",
            Banner::Note("Heuristic: Bod_kcal_per_kg converts weight change to energy; the true density of tissue lost or gained varies."),
        )?;
        let n = n_days(cx);
        let (r0, r1) = (HR + 1, HR + n);
        let (cd, ck, co) = (cal_col(cx, "date"), cal_col(cx, "kcal"), cal_col(cx, "energy_out_kcal"));
        let ema_src = weight::ema_range(cx);
        let my_ema = rng(TITLE, C_EMA, r0, r1);
        section(ws, cx, TOP, 0, "Rolling window of Bod_TDEE_days calendar days (Bod_Cal)")?;
        let full = |r: u32| format!("AND(ISNUMBER({dn}),{dn}>=Bod_TDEE_days)", dn = at(C_DN, r));
        let win = |r: u32| format!("{cd},\">\"&({d}-Bod_TDEE_days),{cd},\"<=\"&{d}", d = at(C_DATE, r));
        let mut cols = cal_head(HR);
        cols.push(Col::f("day_no", move |r| format!("={}", cal_at("day_no", HR, r))));
        cols.push(Col::f("ema_lb", move |r| {
            let dn = at(C_DN, r);
            format!("=IF(ISNUMBER({dn}),IFERROR(INDEX({ema_src},{dn}+1)+0,\"\"),\"\")")
        }));
        cols.push(Col::f("kcal", move |r| format!("={}", cal_at("kcal", HR, r))));
        let (ck2, co2) = (ck.clone(), co.clone());
        cols.push(Col::f("kcal_mean", move |r| {
            format!("=IF({},IFERROR(AVERAGEIFS({ck2},{}),\"\"),\"\")", full(r), win(r))
        }));
        cols.push(Col::f("out_mean", move |r| {
            format!("=IF({},IFERROR(AVERAGEIFS({co2},{}),\"\"),\"\")", full(r), win(r))
        }));
        cols.push(Col::f("n_logged", move |r| format!("=IF({},COUNTIFS({},{ck},\">=0\"),\"\")", full(r), win(r))));
        cols.push(Col::f("dkg", move |r| {
            let (dn, e) = (at(C_DN, r), at(C_EMA, r));
            format!("=IF({},IFERROR(({e}-INDEX({my_ema},{dn}+1-Bod_TDEE_days))/Bod_lb_per_kg,\"\"),\"\")", full(r))
        }));
        cols.push(Col::f("implied_tdee", move |r| {
            let (k, d) = (at(C_KMEAN, r), at(C_DKG, r));
            format!("=IF(AND(ISNUMBER({k}),ISNUMBER({d}),Bod_TDEE_days>0),{k}-Bod_kcal_per_kg*{d}/Bod_TDEE_days,\"\")")
        }));
        cols.push(super::plot_col("tdee_plot", C_TDEE));
        cols.push(super::plot_col("kcal_mean_plot", C_KMEAN));
        cols.push(super::plot_col("out_mean_plot", C_OMEAN));
        table(ws, cx, "tblBodTdee", HR, 0, &cols, n)?;
        super::date_rows(ws, cx, 0, r0, r1)?;
        super::fmt_rows(ws, r0, r1, C_EMA, C_EMA, "0.0")?;
        super::fmt_rows(ws, r0, r1, C_KMEAN, C_OMEAN, "0")?;
        super::fmt_rows(ws, r0, r1, C_DKG, C_DKG, "+0.00;-0.00;0.00")?;
        super::fmt_rows(ws, r0, r1, C_TDEE, C_TDEE, "0")?;
        // Implied TDEE far from the logged energy_out is worth a look.
        let (t, o) = (at(C_TDEE, r0 + 1), at(C_OMEAN, r0 + 1));
        theme::cf_formula(
            ws,
            r0,
            C_TDEE,
            r1,
            C_TDEE,
            &format!("=AND(ISNUMBER({t}),ISNUMBER({o}),ABS({t}-{o})>500)"),
            theme::OUT_OF_REF,
        )?;

        let (tdee, km, om) = (rng(TITLE, C_TDEE, r0, r1), rng(TITLE, C_KMEAN, r0, r1), rng(TITLE, C_OMEAN, r0, r1));
        let s = TOP;
        section(ws, cx, s, SC, "Summary")?;
        kv(ws, cx, s + 1, SC, "Implied TDEE, latest (kcal/day)", &format!("={}", last_num(&tdee)), Some("0"))?;
        kv(ws, cx, s + 2, SC, "Implied TDEE, mean over log", &format!("=IFERROR(AVERAGE({tdee}),\"\")"), Some("0"))?;
        kv(ws, cx, s + 3, SC, "Mean kcal, latest window", &format!("={}", last_num(&km)), Some("0"))?;
        kv(ws, cx, s + 4, SC, "Logged energy_out, latest window", &format!("={}", last_num(&om)), Some("0"))?;
        let (a, b) = (abs(SC + 1, s + 1), abs(SC + 1, s + 4));
        kv(
            ws,
            cx,
            s + 5,
            SC,
            "Implied minus logged out",
            &format!("=IF(AND(ISNUMBER({a}),ISNUMBER({b})),{a}-{b},\"\")"),
            Some("+0;-0;0"),
        )?;
        kv(ws, cx, s + 6, SC, "Window (days)", "=Bod_TDEE_days", Some("0"))?;
        kpi(cx, "tdee", "Implied TDEE (kcal/day, latest window)", TITLE, s + 1, SC + 1, Some("0"))?;
        ws.write_string_with_format(
            s + 8,
            SC,
            "Footnote: 7,700 kcal/kg (3,500 kcal/lb) is a crude rule (HELMS ch01 l.957; Hall & Chow): \
             early weight change is mostly water and glycogen, so short windows over- or under-state TDEE.",
            &cx.f.note,
        )?;
        theme::note_source(ws, s + 8, SC, "HELMS ch01 l.957 (via longevity.md D3)")?;

        let chart = super::line_chart(
            TITLE,
            "Implied TDEE vs logged kcal (window means)",
            r0,
            r1,
            0,
            &[("implied TDEE", C_PLOT), ("mean kcal in", C_PLOT + 1), ("mean energy_out", C_PLOT + 2)],
        );
        ws.insert_chart(s + 10, SC, &chart)?;
        set_widths(
            ws,
            &[12.0, 10.0, 10.0, 7.0, 8.0, 8.0, 10.0, 10.0, 9.0, 8.0, 12.0, 9.0, 9.0, 9.0, 2.0, 2.0, 32.0, 12.0],
        )?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
