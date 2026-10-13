//! BOD-13 `Bod_Glide`: plan vs actual glide path, tracking error and the weight
//! "underwater" (regain) curve (Q84, Q85).
//!
//! Plan: from the start (`Bod_Plan_start`, else the first weigh-in; start weight =
//! `weight_ff` that day) a straight line to `Bod_Goal_lb` on `Bod_Goal_date`, flat after.
//! With either goal input blank the plan defaults to a cut at `Bod_Cut_lo_pct` %/week
//! (compounded daily: `start * (1 - pct/100)^(days/7)`) so the view still computes; the
//! summary says which plan is in use. Row-aligned with `Bod_Cal`: plan, deviation of
//! `weight_ff` from plan, tracking error (running `STDEV.S` of the deviations), best
//! (running `MIN` of weigh-ins since the start) and its date, underwater = `weight_ff`
//! minus best (the regain curve), days since best.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{
    abs, at, cal_head, head, kv, last_num, line_chart, n_days, plot_col, rng, section, table, Banner, Col, TOP,
};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{col as letter, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Glide";
const HR: u32 = TOP + 1;
const C_DATE: u16 = 0;
const C_W: u16 = 1;
const C_FF: u16 = 2;
const C_PLAN: u16 = 3;
const C_DEV: u16 = 4;
const C_TE: u16 = 5;
const C_BEST: u16 = 6;
const C_BD: u16 = 7;
const C_UW: u16 = 8;
const C_DSB: u16 = 9;
const C_PPLOT: u16 = 10;
const C_FPLOT: u16 = 11;
const C_UPLOT: u16 = 12;
const SC: u16 = 14;

pub struct Glide;

impl SheetModule for Glide {
    fn id(&self) -> &'static str {
        "bod-glide"
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
            "Plan vs actual glide path, tracking error, underwater curve",
            "BOD-13",
            "Q84, Q85 | PAL Ch14 (tracking error); KAU Ch23, CHAN L3169 (drawdown / underwater)",
            Banner::Note(
                "Plan: Bod_Goal_lb by Bod_Goal_date from Bod_Plan_start (or the first weigh-in); blank goal -> default cut at Bod_Cut_lo_pct %/week.",
            ),
        )?;
        let n = n_days(cx);
        let first = HR + 2;
        let (r0, r1) = (HR + 1, HR + n);
        let col = |c: u16| rng(TITLE, c, r0, r1);
        let v = |k: u32| abs(SC + 1, TOP + k);
        let (start, sw, mode) = (v(1), v(2), v(3));
        let (dates, wl, ff) = (col(C_DATE), col(C_W), col(C_FF));

        // Plan inputs.
        section(ws, cx, TOP, SC, "Plan")?;
        let first_w = format!("MINIFS({dates},{wl},\">0\")");
        let goal_ok = "AND(ISNUMBER(Bod_Goal_lb),ISNUMBER(Bod_Goal_date))";
        let head_rows: [(&str, String, &str); 5] = [
            (
                "Plan start",
                format!("=IF(ISNUMBER(Bod_Plan_start),Bod_Plan_start,IF(COUNT({wl})=0,\"\",{first_w}))"),
                "yyyy-mm-dd",
            ),
            (
                "Start weight (lb)",
                format!(
                    "=IF(NOT(ISNUMBER({start})),\"\",IF(ISNUMBER(IFERROR(INDEX({ff},MATCH({start},{dates},0)),\"\")),INDEX({ff},MATCH({start},{dates},0)),IF(COUNT({wl})=0,\"\",INDEX({wl},MATCH({first_w},{dates},0)))))"
                ),
                "0.0",
            ),
            (
                "Goal plan in use (1) / default (0)",
                format!("=IF(AND({goal_ok},ISNUMBER({start})),IF(Bod_Goal_date>{start},1,0),0)"),
                "0",
            ),
            (
                "Plan",
                format!(
                    "=IF({mode}=1,\"goal: \"&TEXT(Bod_Goal_lb,\"0.0\")&\" lb by \"&TEXT(Bod_Goal_date,\"yyyy-mm-dd\"),\"default: -\"&Bod_Cut_lo_pct&\" %/week (set Bod_Goal_lb and Bod_Goal_date for your own plan)\")"
                ),
                "@",
            ),
            (
                "Planned change, first week (lb)",
                format!(
                    "=IF(NOT(ISNUMBER({sw})),\"\",IF({mode}=1,7*(Bod_Goal_lb-{sw})/(Bod_Goal_date-{start}),{sw}*((1-Bod_Cut_lo_pct/100)-1)))"
                ),
                "+0.00;-0.00",
            ),
        ];
        for (i, (label, f, nf)) in head_rows.iter().enumerate() {
            kv(ws, cx, TOP + 1 + i as u32, SC, label, f, Some(nf))?;
        }

        section(ws, cx, TOP, 0, "Glide path (one row per calendar day, Bod_Cal)")?;
        let mut cols = cal_head(HR);
        cols.push(Col::f("plan", |r| {
            let d = at(C_DATE, r);
            format!(
                "=IF(AND(ISNUMBER({d}),ISNUMBER({start}),ISNUMBER({sw})),IF({d}<{start},\"\",IF({mode}=1,IF({d}>=Bod_Goal_date,Bod_Goal_lb,{sw}+(Bod_Goal_lb-{sw})*({d}-{start})/(Bod_Goal_date-{start})),{sw}*(1-Bod_Cut_lo_pct/100)^(({d}-{start})/7))),\"\")"
            )
        }));
        cols.push(Col::f("deviation", |r| {
            let (f, p) = (at(C_FF, r), at(C_PLAN, r));
            format!("=IF(AND(ISNUMBER({f}),ISNUMBER({p})),{f}-{p},\"\")")
        }));
        let dcol = letter(C_DEV);
        cols.push(Col::f("tracking_err", move |r| {
            let w = format!("${dcol}${first}:{dcol}{r}");
            format!("=IF(COUNT({w})>=2,STDEV.S({w}),\"\")")
        }));
        cols.push(Col::f("best", |r| {
            let (w, d) = (at(C_W, r), at(C_DATE, r));
            let now = format!("AND(ISNUMBER({w}),ISNUMBER({start}),ISNUMBER({d}))");
            if r == first {
                format!("=IF({now},IF({d}>={start},{w},\"\"),\"\")")
            } else {
                let p = at(C_BEST, r - 1);
                format!(
                    "=IF({now},IF({d}>={start},IF(ISNUMBER({p}),MIN({p},{w}),{w}),\"\"),IF(ISNUMBER({p}),{p},\"\"))"
                )
            }
        }));
        cols.push(Col::f("best_date", |r| {
            let (b, d) = (at(C_BEST, r), at(C_DATE, r));
            if r == first {
                format!("=IF(ISNUMBER({b}),{d},\"\")")
            } else {
                let (pb, pd) = (at(C_BEST, r - 1), at(C_BD, r - 1));
                format!("=IF(ISNUMBER({b}),IF(OR(NOT(ISNUMBER({pb})),{b}<{pb}),{d},{pd}),\"\")")
            }
        }));
        cols.push(Col::f("underwater", |r| {
            let (f, b) = (at(C_FF, r), at(C_BEST, r));
            format!("=IF(AND(ISNUMBER({f}),ISNUMBER({b})),{f}-{b},\"\")")
        }));
        cols.push(Col::f("days_since_best", |r| {
            let (bd, d) = (at(C_BD, r), at(C_DATE, r));
            format!("=IF(AND(ISNUMBER({bd}),ISNUMBER({d})),{d}-{bd},\"\")")
        }));
        cols.push(plot_col("plan_plot", C_PLAN));
        cols.push(plot_col("ff_plot", C_FF));
        cols.push(plot_col("underwater_plot", C_UW));
        table(ws, cx, "tblBodGlide", HR, 0, &cols, n)?;
        super::date_rows(ws, cx, C_DATE, r0, r1)?;
        super::date_rows(ws, cx, C_BD, r0, r1)?;
        super::fmt_rows(ws, r0, r1, C_PLAN, C_PLAN, "0.0")?;
        super::fmt_rows(ws, r0, r1, C_DEV, C_DEV, "+0.0;-0.0;0.0")?;
        super::fmt_rows(ws, r0, r1, C_TE, C_TE, "0.00")?;
        super::fmt_rows(ws, r0, r1, C_BEST, C_BEST, "0.0")?;
        super::fmt_rows(ws, r0, r1, C_UW, C_UW, "0.0")?;
        let d = at(C_DEV, r0 + 1);
        theme::cf_formula(ws, r0, C_DEV, r1, C_DEV, &format!("=AND(ISNUMBER({d}),{d}>0)"), theme::OUT_OF_REF)?;
        theme::cf_formula(ws, r0, C_DEV, r1, C_DEV, &format!("=AND(ISNUMBER({d}),{d}<=0)"), theme::OPTIMAL)?;
        theme::cf_data_bar(ws, r0, C_UW, r1, C_UW, theme::CRITICAL)?;

        // Results.
        let (uw, dsb) = (col(C_UW), col(C_DSB));
        let res: [(&str, String, &str); 7] = [
            ("Plan weight, last day (lb)", format!("={}", last_num(&col(C_PLAN))), "0.0"),
            ("Deviation from plan, last day (lb)", format!("={}", last_num(&col(C_DEV))), "+0.0;-0.0"),
            ("Tracking error (SD of deviation, lb)", format!("={}", last_num(&col(C_TE))), "0.00"),
            ("Best (lowest weigh-in since start, lb)", format!("={}", last_num(&col(C_BEST))), "0.0"),
            ("Above best now (lb)", format!("={}", last_num(&uw)), "0.0"),
            ("Max regain above best (lb)", format!("=IF(COUNT({uw})>0,MAX({uw}),\"\")"), "0.0"),
            ("Days since best", format!("={}", last_num(&dsb)), "0"),
        ];
        section(ws, cx, TOP + 7, SC, "Tracking")?;
        for (i, (label, f, nf)) in res.iter().enumerate() {
            kv(ws, cx, TOP + 8 + i as u32, SC, label, f, Some(nf))?;
        }
        let dev_now = abs(SC + 1, TOP + 9);
        theme::cf_formula(
            ws,
            TOP + 9,
            SC + 1,
            TOP + 9,
            SC + 1,
            &format!("=AND(ISNUMBER({dev_now}),{dev_now}>0)"),
            theme::OUT_OF_REF,
        )?;

        let mut plan =
            line_chart(TITLE, "Plan vs actual (lb)", r0, r1, C_DATE, &[("plan", C_PPLOT), ("weight_ff", C_FPLOT)]);
        plan.set_width(680);
        ws.insert_chart(TOP + 16, SC, &plan)?;
        let mut under = Chart::new(ChartType::Area);
        under
            .add_series()
            .set_name("above best (lb)")
            .set_categories((TITLE, r0, C_DATE, r1, C_DATE))
            .set_values((TITLE, r0, C_UPLOT, r1, C_UPLOT));
        under.title().set_name("Underwater: regain above the best weigh-in so far");
        under.x_axis().set_num_format("yyyy-mm-dd");
        under.show_na_as_empty_cell();
        under.set_width(680).set_height(260);
        ws.insert_chart(TOP + 33, SC, &under)?;
        set_widths(ws, &[12.0, 10.0, 10.0, 9.0, 9.0, 10.0, 8.0, 11.0, 10.0, 9.0, 8.0, 8.0, 9.0, 2.0, 34.0, 14.0])?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
