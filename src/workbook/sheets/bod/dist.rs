//! BOD-03 `Bod_Dist`: distribution moments and a histogram with a normal overlay of
//! the daily series (KAU Ch2 "Moments of the Distribution"; PAL Ch2 stylized facts;
//! quant.md Q18, Q3).
//!
//! * Moments table: for `weight_lb`, `dw_day`, `kcal`, `protein_g` (whole `Bod_Cal`
//!   columns): n = COUNT, mean, STDEV.S, SKEW, KURT (excess), MEDIAN, MIN, MAX,
//!   PERCENTILE 5 % / 95 %, and a fat-tail flag (KURT > `Bod_Fat_tail_kurt`).
//! * A series selector (drop-down) feeds a row-aligned helper column
//!   (`CHOOSE(MATCH(sel, series, 0), ...)` over `Bod_Cal`).
//! * 20-bin histogram of the selected series: lo/hi from its MIN/MAX, counts by COUNTIFS
//!   (last bin closed), expected count n*(NORM.DIST(hi) - NORM.DIST(lo)) for a normal
//!   with the same mean and sd. No FREQUENCY array formula.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{abs, at, cal_at, cal_col, head, kv, list, n_days, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Dist";
/// The series with moments, in table order (`Bod_Cal` headers).
pub const SERIES: [&str; 4] = ["weight_lb", "dw_day", "kcal", "protein_g"];
const HR: u32 = TOP + 1;
/// Selector and selected-series statistics.
const SEL_R: u32 = TOP + 7;
const BINS: u32 = 20;
const H_HR: u32 = SEL_R + 9;
/// Row-aligned helper (date, value) at this column.
const X_C: u16 = 14;
const X_HR: u32 = TOP + 1;

pub struct Dist;

impl SheetModule for Dist {
    fn id(&self) -> &'static str {
        "bod-dist"
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
            "Moments (SKEW/KURT) and histogram with normal overlay",
            "BOD-03",
            "Q18, Q3 | KAU Ch2 (moments using Excel); PAL Ch2 (stylized facts: fat tails)",
            Banner::Note(
                "Daily series from Bod_Cal (blank days excluded). KURT is excess kurtosis: above \
                 Bod_Fat_tail_kurt the tails are fatter than a normal (binge / skip days).",
            ),
        )?;
        let n = n_days(cx);

        // Moments.
        section(ws, cx, TOP, 0, "Moments of the daily series")?;
        let src = |r: u32| cal_col(cx, SERIES[(r - HR - 2) as usize]);
        let stat = |guard: u32, f: &'static str| {
            move |r: u32| {
                let x = src(r);
                let body = f.replace('@', &x);
                format!("=IF({}>={guard},IFERROR({body},\"\"),\"\")", at(1, r))
            }
        };
        let mcols = vec![
            Col::f("series", |r| format!("=\"{}\"", SERIES[(r - HR - 2) as usize])),
            Col::f("n", |r| format!("=COUNT({})", src(r))),
            Col::f("mean", stat(1, "AVERAGE(@)")),
            Col::f("sd", stat(2, "STDEV.S(@)")),
            Col::f("skew", stat(3, "SKEW(@)")),
            Col::f("kurt", stat(4, "KURT(@)")),
            Col::f("median", stat(1, "MEDIAN(@)")),
            Col::f("min", stat(1, "MIN(@)")),
            Col::f("max", stat(1, "MAX(@)")),
            Col::f("p05", stat(1, "PERCENTILE(@,0.05)")),
            Col::f("p95", stat(1, "PERCENTILE(@,0.95)")),
            Col::f("tails", |r| {
                let k = at(5, r);
                format!("=IF(ISNUMBER({k}),IF({k}>Bod_Fat_tail_kurt,\"fat\",\"normal-like\"),\"\")")
            }),
        ];
        table(ws, cx, "tblBodDistMoments", HR, 0, &mcols, SERIES.len() as u32)?;
        let (m0, m1) = (HR + 1, HR + SERIES.len() as u32);
        super::fmt_rows(ws, m0, m1, 2, 3, "0.00")?;
        super::fmt_rows(ws, m0, m1, 4, 5, "+0.00;-0.00;0.00")?;
        super::fmt_rows(ws, m0, m1, 6, 10, "0.0")?;
        let t = at(11, m0 + 1);
        theme::cf_formula(ws, m0, 11, m1, 11, &format!("={t}=\"fat\""), theme::OUT_OF_REF)?;
        let s = at(4, m0 + 1);
        theme::cf_formula(ws, m0, 4, m1, 4, &format!("=AND(ISNUMBER({s}),ABS({s})>1)"), theme::OUT_OF_REF)?;

        // Selector and the selected series.
        let sel = abs(1, SEL_R);
        let names = rng(TITLE, 0, m0, m1);
        let xr = rng(TITLE, X_C + 1, X_HR + 1, X_HR + n);
        ws.write_string_with_format(SEL_R, 0, "Histogram of", &cx.f.bold)?;
        ws.write_string_with_format(SEL_R, 1, SERIES[0], &cx.f.input)?;
        list(ws, SEL_R, 1, SEL_R, &SERIES)?;
        kv(ws, cx, SEL_R + 1, 0, "n", &format!("=COUNT({xr})"), Some("0"))?;
        kv(ws, cx, SEL_R + 2, 0, "mean", &format!("=IFERROR(AVERAGE({xr}),\"\")"), Some("0.00"))?;
        kv(ws, cx, SEL_R + 3, 0, "sd", &format!("=IFERROR(STDEV.S({xr}),\"\")"), Some("0.00"))?;
        kv(ws, cx, SEL_R + 4, 0, "min", &format!("=IFERROR(1*MIN({xr}),\"\")"), Some("0.00"))?;
        kv(ws, cx, SEL_R + 5, 0, "max", &format!("=IFERROR(1*MAX({xr}),\"\")"), Some("0.00"))?;
        let (nn, mean, sd, lo, hi) =
            (abs(1, SEL_R + 1), abs(1, SEL_R + 2), abs(1, SEL_R + 3), abs(1, SEL_R + 4), abs(1, SEL_R + 5));
        kv(ws, cx, SEL_R + 6, 0, "bin width", &format!("=IF(N({nn})>=2,({hi}-{lo})/{BINS},\"\")"), Some("0.000"))?;
        let bw = abs(1, SEL_R + 6);

        // Histogram.
        section(ws, cx, H_HR - 1, 0, "Histogram (20 bins) with the normal overlay")?;
        let h_first = H_HR + 2;
        let hcols = vec![
            Col::f("bin", |r| format!("={}", r - H_HR - 1)),
            Col::f(
                "lo",
                |r| {
                    if r == h_first {
                        format!("=IF(N({bw})>0,{lo},\"\")")
                    } else {
                        format!("={}", at(2, r - 1))
                    }
                },
            ),
            Col::f("hi", |r| {
                let i = r - H_HR - 1;
                if i == BINS {
                    format!("=IF(N({bw})>0,{hi},\"\")")
                } else {
                    format!("=IF(N({bw})>0,{lo}+{i}*{bw},\"\")")
                }
            }),
            Col::f("mid", |r| {
                let (a, b) = (at(1, r), at(2, r));
                format!("=IF(ISNUMBER({a}),({a}+{b})/2,\"\")")
            }),
            Col::f("count", |r| {
                let (a, b) = (at(1, r), at(2, r));
                let op = if r - H_HR - 1 == BINS { "<=" } else { "<" };
                format!("=IF(ISNUMBER({a}),COUNTIFS({xr},\">=\"&{a},{xr},\"{op}\"&{b}),\"\")")
            }),
            Col::f("expected", |r| {
                let (a, b) = (at(1, r), at(2, r));
                format!(
                    "=IF(AND(ISNUMBER({a}),N({sd})>0),{nn}*(NORM.DIST({b},{mean},{sd},TRUE)-NORM.DIST({a},{mean},{sd},TRUE)),\"\")"
                )
            }),
            Col::f("share", |r| {
                let c = at(4, r);
                format!("=IF(AND(ISNUMBER({c}),N({nn})>0),{c}/{nn},\"\")")
            }),
        ];
        table(ws, cx, "tblBodDistHist", H_HR, 0, &hcols, BINS)?;
        let (h0, h1) = (H_HR + 1, H_HR + BINS);
        super::fmt_rows(ws, h0, h1, 1, 3, "0.00")?;
        super::fmt_rows(ws, h0, h1, 5, 5, "0.0")?;
        super::fmt_rows(ws, h0, h1, 6, 6, "0%")?;
        theme::cf_data_bar(ws, h0, 4, h1, 4, theme::BAND_BLUEPRINT)?;
        kv(ws, cx, h1 + 1, 0, "total", &format!("=SUM({})", rng(TITLE, 4, h0, h1)), Some("0"))?;

        let mut chart = Chart::new(ChartType::Column);
        chart
            .add_series()
            .set_name("count")
            .set_categories((TITLE, h0, 3, h1, 3))
            .set_values((TITLE, h0, 4, h1, 4))
            .set_gap(10);
        let mut line = Chart::new(ChartType::Line);
        line.add_series().set_name("normal").set_categories((TITLE, h0, 3, h1, 3)).set_values((TITLE, h0, 5, h1, 5));
        chart.combine(&line);
        chart.title().set_name("Histogram of the selected series vs normal");
        chart.x_axis().set_num_format("0.0");
        chart.set_width(640).set_height(300);
        ws.insert_chart(h1 + 3, 0, &chart)?;

        // Row-aligned helper: the selected series per calendar day.
        section(ws, cx, X_HR - 1, X_C, "Selected series per day (helper)")?;
        let choose = |r: u32| SERIES.iter().map(|s| cal_at(s, X_HR, r)).collect::<Vec<_>>().join(",");
        let xcols = vec![
            Col::f("date", |r| format!("={}", cal_at("date", X_HR, r))),
            Col::f("value", |r| format!("=IFERROR(CHOOSE(MATCH({sel},{names},0),{}),\"\")", choose(r))),
        ];
        table(ws, cx, "tblBodDistSeries", X_HR, X_C, &xcols, n)?;
        super::date_rows(ws, cx, X_C, X_HR + 1, X_HR + n)?;
        set_widths(ws, &[16.0, 10.0, 9.0, 9.0, 8.0, 8.0, 9.0, 8.0, 8.0, 8.0, 8.0, 11.0, 2.0, 2.0, 12.0, 10.0])?;
        ws.set_freeze_panes(TOP, 0)?;
        Ok(())
    }
}
