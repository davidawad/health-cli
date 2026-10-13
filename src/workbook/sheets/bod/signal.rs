//! BOD-04 `Bod_Signal`: Kaufman efficiency ratio and adaptive moving average (KAMA)
//! of the daily weight (KAU Ch17; quant.md Q21, Q22).
//!
//! Row-aligned with `Bod_Cal` (one row per calendar day, `k` = 1-based day index):
//! * `abs_d` = |weight_ff_k - weight_ff_{k-1}| (carried-forward weight, so a gap is a 0 move);
//! * `change` = |ff_k - INDEX(ff, k-Bod_KAMA_n)| (blank until k > Bod_KAMA_n);
//! * `volatility` = SUM(INDEX(abs_d, k-n+1):INDEX(abs_d, k));
//! * `er` = change / volatility (0 when volatility is 0);
//! * `sc` = (er*(2/(KAMA_fast+1) - 2/(KAMA_slow+1)) + 2/(KAMA_slow+1))^2;
//! * `kama` = kama_{k-1} + sc*(ff_k - kama_{k-1}); it tracks ff until the first ER exists;
//! * `ema` = the `Bod_Weight` 7-day EMA on the same day, for comparison.
//!
//! Summary: latest ER, KAMA, EMA and a regime label (trending when ER > `Bod_ER_trend`).
//! Also hosts two small helpers the other lane-A sheets share ([`win`], [`series`]).

use rust_xlsxwriter::{Chart, ChartFormat, ChartMarker, ChartMarkerType, ChartType, Worksheet};

use super::{at, cal_head, head, kv, last_num, n_days, plot_col, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Signal";
const HR: u32 = TOP + 1;
const C_FF: u16 = 2;
const C_ABS: u16 = 3;
const C_CHG: u16 = 4;
const C_VOL: u16 = 5;
const C_ER: u16 = 6;
const C_SC: u16 = 7;
const C_KAMA: u16 = 8;
const C_EMA: u16 = 9;
const C_WPLOT: u16 = 10;
const S_C: u16 = 14;

/// `INDEX(range,k-n+1):INDEX(range,k)`: the `n`-row window ending on 1-based row `k` of
/// `range` (`n` is a formula expression, usually a parameter name). Guard `k >= n` first.
pub(super) fn win(range: &str, k: u32, n: &str) -> String {
    format!("INDEX({range},{k}-{n}+1):INDEX({range},{k})")
}

/// Add column `c` over zero-based rows `r0..=r1` of `title` (dates in column A) to
/// `chart`, as dots (markers, no line) or as a line.
pub(super) fn series(chart: &mut Chart, title: &str, name: &str, r0: u32, r1: u32, c: u16, dots: bool) {
    let s = chart.add_series().set_name(name).set_categories((title, r0, 0, r1, 0)).set_values((title, r0, c, r1, c));
    if dots {
        s.set_format(ChartFormat::new().set_no_line())
            .set_marker(ChartMarker::new().set_type(ChartMarkerType::Circle).set_size(4));
    }
}

/// A date-axis line chart with gaps for `#N/A`.
pub(super) fn date_chart(name: &str) -> Chart {
    let mut chart = Chart::new(ChartType::Line);
    chart.title().set_name(name);
    chart.x_axis().set_num_format("yyyy-mm-dd");
    chart.show_na_as_empty_cell();
    chart.set_width(720).set_height(320);
    chart
}

pub struct Signal;

impl SheetModule for Signal {
    fn id(&self) -> &'static str {
        "bod-signal"
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
            "Efficiency ratio and KAMA adaptive trend",
            "BOD-04",
            "Q21, Q22 | KAU Ch17 (Kaufman's Adaptive Moving Average)",
            Banner::Note(
                "ER = |net move over Bod_KAMA_n days| / sum of |daily moves| (0 = noise, 1 = straight line); \
                 KAMA speeds up when ER is high.",
            ),
        )?;
        let n = n_days(cx);
        let (r0, r1) = (HR + 1, HR + n);
        let first = HR + 2;
        let ff = rng(TITLE, C_FF, r0, r1);
        let absd = rng(TITLE, C_ABS, r0, r1);
        let ema = super::weight::ema_range(cx);
        section(ws, cx, TOP, 0, "Daily series (one row per calendar day, Bod_Cal)")?;
        let mut cols = cal_head(HR);
        cols.push(Col::f("abs_d", move |r| {
            if r == first {
                "=\"\"".into()
            } else {
                let (x, p) = (at(C_FF, r), at(C_FF, r - 1));
                format!("=IF(AND(ISNUMBER({x}),ISNUMBER({p})),ABS({x}-{p}),\"\")")
            }
        }));
        cols.push(Col::f("change", |r| {
            let (k, x) = (r - HR - 1, at(C_FF, r));
            let lag = format!("INDEX({ff},{k}-Bod_KAMA_n)");
            format!("=IF({k}>Bod_KAMA_n,IF(AND(ISNUMBER({x}),ISNUMBER({lag})),ABS({x}-{lag}),\"\"),\"\")")
        }));
        cols.push(Col::f("volatility", |r| {
            let k = r - HR - 1;
            format!("=IF({k}>Bod_KAMA_n,SUM({}),\"\")", win(&absd, k, "Bod_KAMA_n"))
        }));
        cols.push(Col::f("er", |r| {
            let (c, v) = (at(C_CHG, r), at(C_VOL, r));
            format!("=IF(ISNUMBER({c}),IF({v}>0,{c}/{v},0),\"\")")
        }));
        cols.push(Col::f("sc", |r| {
            let e = at(C_ER, r);
            format!("=IF(ISNUMBER({e}),({e}*(2/(KAMA_fast+1)-2/(KAMA_slow+1))+2/(KAMA_slow+1))^2,\"\")")
        }));
        cols.push(Col::f("kama", move |r| {
            let x = at(C_FF, r);
            if r == first {
                format!("=IF(ISNUMBER({x}),{x},\"\")")
            } else {
                let (s, p) = (at(C_SC, r), at(C_KAMA, r - 1));
                format!(
                    "=IF(AND(ISNUMBER({s}),ISNUMBER({p}),ISNUMBER({x})),{p}+{s}*({x}-{p}),IF(ISNUMBER({x}),{x},{p}))"
                )
            }
        }));
        cols.push(Col::f("ema", |r| format!("=IFERROR(INDEX({ema},{}),\"\")", r - HR - 1)));
        cols.push(plot_col("weight_plot", 1));
        cols.push(plot_col("kama_plot", C_KAMA));
        cols.push(plot_col("ema_plot", C_EMA));
        table(ws, cx, "tblBodSignal", HR, 0, &cols, n)?;
        super::date_rows(ws, cx, 0, r0, r1)?;
        super::fmt_rows(ws, r0, r1, C_ABS, C_VOL, "0.0")?;
        super::fmt_rows(ws, r0, r1, C_ER, C_ER, "0.00")?;
        super::fmt_rows(ws, r0, r1, C_SC, C_SC, "0.000")?;
        super::fmt_rows(ws, r0, r1, C_KAMA, C_EMA, "0.0")?;
        theme::cf_two_color(ws, r0, C_ER, r1, C_ER, theme::REF_OK, theme::BAND_OPT)?;
        let e = at(C_ER, r0 + 1);
        theme::cf_formula(ws, r0, C_ER, r1, C_ER, &format!("=AND(ISNUMBER({e}),{e}>Bod_ER_trend)"), theme::OPTIMAL)?;

        // Summary.
        let (er, kama, emar, sc) =
            (rng(TITLE, C_ER, r0, r1), rng(TITLE, C_KAMA, r0, r1), rng(TITLE, C_EMA, r0, r1), rng(TITLE, C_SC, r0, r1));
        section(ws, cx, TOP, S_C, "Summary (latest day)")?;
        kv(ws, cx, TOP + 1, S_C, "Efficiency ratio", &format!("={}", last_num(&er)), Some("0.00"))?;
        kv(ws, cx, TOP + 2, S_C, "KAMA (lb)", &format!("={}", last_num(&kama)), Some("0.0"))?;
        kv(ws, cx, TOP + 3, S_C, "EMA (lb, Bod_Weight)", &format!("={}", last_num(&emar)), Some("0.0"))?;
        kv(ws, cx, TOP + 4, S_C, "Smoothing constant", &format!("={}", last_num(&sc)), Some("0.000"))?;
        let erc = super::abs(S_C + 1, TOP + 1);
        kv(
            ws,
            cx,
            TOP + 5,
            S_C,
            "Regime",
            &format!("=IF(ISNUMBER({erc}),IF({erc}>Bod_ER_trend,\"trending\",\"noisy\"),\"\")"),
            None,
        )?;
        kv(
            ws,
            cx,
            TOP + 6,
            S_C,
            "Trending days (% of ER days)",
            &format!("=IFERROR(COUNTIF({er},\">\"&Bod_ER_trend)/COUNT({er}),\"\")"),
            Some("0%"),
        )?;
        let reg = super::abs(S_C + 1, TOP + 5);
        theme::cf_formula(ws, TOP + 5, S_C + 1, TOP + 5, S_C + 1, &format!("={reg}=\"trending\""), theme::OPTIMAL)?;
        theme::cf_formula(ws, TOP + 5, S_C + 1, TOP + 5, S_C + 1, &format!("={reg}=\"noisy\""), theme::REF_OK)?;
        ws.write_string_with_format(
            TOP + 7,
            S_C,
            "Trending when ER > Bod_ER_trend (Params); window Bod_KAMA_n, speeds KAMA_fast / KAMA_slow.",
            &cx.f.note,
        )?;

        let mut chart = date_chart("Weight, KAMA and 7-day EMA");
        series(&mut chart, TITLE, "weigh-in (lb)", r0, r1, C_WPLOT, true);
        series(&mut chart, TITLE, "KAMA", r0, r1, C_WPLOT + 1, false);
        series(&mut chart, TITLE, "EMA", r0, r1, C_WPLOT + 2, false);
        ws.insert_chart(TOP + 9, S_C, &chart)?;
        set_widths(ws, &[12.0, 10.0, 10.0, 7.0, 8.0, 10.0, 7.0, 7.0, 9.0, 9.0, 10.0, 10.0, 10.0, 2.0, 28.0, 10.0])?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
