//! BOD-06 `Bod_Kalman`: local-level Kalman filter of the daily weight: the "true"
//! weight behind a noisy scale, its uncertainty band and the gain (PAL Ch2.4, KAU Ch6;
//! quant.md Q24).
//!
//! Row-aligned with `Bod_Cal`; one predict step per calendar day, an update only on
//! weigh-in days. `Kalman_q` (process variance per day) and `Kalman_r` (scale variance)
//! are in lb^2.
//! * `p_pred` = P_{t-1} + Kalman_q;
//! * `gain` K = p_pred / (p_pred + Kalman_r) on a weigh-in day, else 0;
//! * `x` = x_{t-1} + K*(w - x_{t-1}) (x_{t-1} without a weigh-in); `p` = (1-K)*p_pred;
//! * start: x0 = the first weigh-in, P0 = Kalman_r;
//! * `lo` / `hi` = x -+ RCV_z*SQRT(p); `innovation` = w - x_{t-1}.
//!
//! Summary: latest true weight +- band, mean gain on weigh-in days, innovation sd.

use rust_xlsxwriter::Worksheet;

use super::signal::{date_chart, series};
use super::{at, cal_head, head, kv, last_num, n_days, plot_col, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Kalman";
const HR: u32 = TOP + 1;
const C_W: u16 = 1;
const C_PP: u16 = 3;
const C_K: u16 = 4;
const C_X: u16 = 5;
const C_P: u16 = 6;
const C_LO: u16 = 7;
const C_HI: u16 = 8;
const C_INN: u16 = 9;
const C_WPLOT: u16 = 10;
const S_C: u16 = 15;

pub struct Kalman;

impl SheetModule for Kalman {
    fn id(&self) -> &'static str {
        "bod-kalman"
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
            "Local-level Kalman filter: true weight, band, gain",
            "BOD-06",
            "Q24 | PAL Ch2.4 (The Kalman Filter, Insight 2.2: EWMA is its steady state); KAU Ch6",
            Banner::Note(
                "Kalman_q (day-to-day change of true weight) and Kalman_r (scale and water noise) are variances \
                 in lb^2 (Params); a larger r trusts each weigh-in less.",
            ),
        )?;
        let n = n_days(cx);
        let (r0, r1) = (HR + 1, HR + n);
        let first = HR + 2;
        section(ws, cx, TOP, 0, "Daily filter (one row per calendar day, Bod_Cal)")?;
        let mut cols = cal_head(HR);
        cols.push(Col::f("p_pred", move |r| {
            if r == first {
                "=\"\"".into()
            } else {
                let p = at(C_P, r - 1);
                format!("=IF(ISNUMBER({p}),{p}+Kalman_q,\"\")")
            }
        }));
        cols.push(Col::f("gain", |r| {
            let (w, pp) = (at(C_W, r), at(C_PP, r));
            format!("=IF(ISNUMBER({pp}),IF(ISNUMBER({w}),IFERROR({pp}/({pp}+Kalman_r),0),0),\"\")")
        }));
        cols.push(Col::f("x", move |r| {
            let w = at(C_W, r);
            if r == first {
                format!("=IF(ISNUMBER({w}),{w},\"\")")
            } else {
                let (p, k) = (at(C_X, r - 1), at(C_K, r));
                format!("=IF(ISNUMBER({p}),IF(ISNUMBER({w}),{p}+{k}*({w}-{p}),{p}),IF(ISNUMBER({w}),{w},\"\"))")
            }
        }));
        cols.push(Col::f("p", move |r| {
            let w = at(C_W, r);
            if r == first {
                format!("=IF(ISNUMBER({w}),Kalman_r,\"\")")
            } else {
                let (p, k, pp) = (at(C_X, r - 1), at(C_K, r), at(C_PP, r));
                format!("=IF(ISNUMBER({p}),(1-{k})*{pp},IF(ISNUMBER({w}),Kalman_r,\"\"))")
            }
        }));
        for (h, sign) in [("lo", "-"), ("hi", "+")] {
            cols.push(Col::f(h, move |r| {
                let (x, p) = (at(C_X, r), at(C_P, r));
                format!("=IF(AND(ISNUMBER({x}),ISNUMBER({p})),{x}{sign}RCV_z*SQRT(MAX(0,{p})),\"\")")
            }));
        }
        cols.push(Col::f("innovation", move |r| {
            if r == first {
                "=\"\"".into()
            } else {
                let (w, p) = (at(C_W, r), at(C_X, r - 1));
                format!("=IF(AND(ISNUMBER({w}),ISNUMBER({p})),{w}-{p},\"\")")
            }
        }));
        cols.push(plot_col("weight_plot", C_W));
        cols.push(plot_col("x_plot", C_X));
        cols.push(plot_col("lo_plot", C_LO));
        cols.push(plot_col("hi_plot", C_HI));
        table(ws, cx, "tblBodKalman", HR, 0, &cols, n)?;
        super::date_rows(ws, cx, 0, r0, r1)?;
        super::fmt_rows(ws, r0, r1, C_PP, C_K, "0.000")?;
        super::fmt_rows(ws, r0, r1, C_X, C_X, "0.0")?;
        super::fmt_rows(ws, r0, r1, C_P, C_P, "0.000")?;
        super::fmt_rows(ws, r0, r1, C_LO, C_HI, "0.0")?;
        super::fmt_rows(ws, r0, r1, C_INN, C_INN, "+0.0;-0.0;0.0")?;
        // An innovation outside the predicted band (RCV_z * sqrt(p_pred + r)) is a surprising weigh-in.
        let (i, pp) = (at(C_INN, r0 + 1), at(C_PP, r0 + 1));
        theme::cf_formula(
            ws,
            r0,
            C_INN,
            r1,
            C_INN,
            &format!("=IF(AND(ISNUMBER({i}),ISNUMBER({pp})),ABS({i})>RCV_z*SQRT({pp}+Kalman_r),FALSE)"),
            theme::CRITICAL,
        )?;
        theme::cf_data_bar(ws, r0, C_K, r1, C_K, theme::BAND_BLUEPRINT)?;

        // Summary.
        let (x, p, k, inn) =
            (rng(TITLE, C_X, r0, r1), rng(TITLE, C_P, r0, r1), rng(TITLE, C_K, r0, r1), rng(TITLE, C_INN, r0, r1));
        let w = rng(TITLE, C_W, r0, r1);
        section(ws, cx, TOP, S_C, "Summary (latest day)")?;
        kv(ws, cx, TOP + 1, S_C, "True weight (lb)", &format!("={}", last_num(&x)), Some("0.0"))?;
        let pl = last_num(&p);
        kv(
            ws,
            cx,
            TOP + 2,
            S_C,
            "Band +- (lb, RCV_z sd)",
            &format!("=IF(ISNUMBER({pl}),RCV_z*SQRT(MAX(0,{pl})),\"\")"),
            Some("0.00"),
        )?;
        kv(ws, cx, TOP + 3, S_C, "Last weigh-in (lb)", &format!("={}", last_num(&w)), Some("0.0"))?;
        kv(
            ws,
            cx,
            TOP + 4,
            S_C,
            "Mean gain on weigh-in days",
            &format!("=IFERROR(AVERAGEIF({k},\">0\"),\"\")"),
            Some("0.000"),
        )?;
        kv(ws, cx, TOP + 5, S_C, "Innovation sd (lb)", &format!("=IFERROR(STDEV.S({inn}),\"\")"), Some("0.00"))?;
        kv(
            ws,
            cx,
            TOP + 6,
            S_C,
            "Signal share of a weigh-in (q/(q+r))",
            "=IFERROR(Kalman_q/(Kalman_q+Kalman_r),\"\")",
            Some("0.0%"),
        )?;
        ws.write_string_with_format(
            TOP + 7,
            S_C,
            "Gain near 0: the filter trusts its trend; near 1: it follows the scale. Red innovation = outside \
             the predicted band.",
            &cx.f.note,
        )?;

        let mut chart = date_chart("Weigh-ins and filtered true weight with band");
        series(&mut chart, TITLE, "weigh-in (lb)", r0, r1, C_WPLOT, true);
        series(&mut chart, TITLE, "true weight", r0, r1, C_WPLOT + 1, false);
        series(&mut chart, TITLE, "band low", r0, r1, C_WPLOT + 2, false);
        series(&mut chart, TITLE, "band high", r0, r1, C_WPLOT + 3, false);
        ws.insert_chart(TOP + 9, S_C, &chart)?;
        set_widths(ws, &[12.0, 10.0, 10.0, 8.0, 7.0, 9.0, 8.0, 8.0, 8.0, 10.0, 10.0, 9.0, 9.0, 9.0, 2.0, 32.0, 10.0])?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
