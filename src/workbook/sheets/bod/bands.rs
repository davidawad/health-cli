//! BOD-05 `Bod_Bands`: personal Bollinger bands on the daily weight (KAU Ch7, Ch4;
//! quant.md Q23).
//!
//! Row-aligned with `Bod_Cal` (`k` = 1-based day index); the window is the last
//! `N = Bod_Boll_days` calendar days of observed weigh-ins, `W = INDEX(w,k-N+1):INDEX(w,k)`:
//! * `n_win` = COUNT(W) (blank until k >= N); `mid` = AVERAGE(W), `sd` = STDEV.S(W), both
//!   blank unless n_win >= 3;
//! * `upper` / `lower` = mid +- Boll_K * sd; `pct_b` = (w - lower) / (upper - lower);
//!   `bandwidth` = (upper - lower) / mid;
//! * `breach` = 1 above the upper band, -1 below the lower band, 0 inside;
//! * `breach_hi` / `breach_lo` = the weigh-in or NA(): chart markers only.
//!
//! Summary: latest band, %b and width, and breach counts.

use rust_xlsxwriter::{ChartFormat, ChartMarker, ChartMarkerType, Worksheet};

use super::signal::{date_chart, series, win};
use super::{at, cal_head, head, kv, last_num, n_days, plot_col, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Bands";
const HR: u32 = TOP + 1;
const C_W: u16 = 1;
const C_N: u16 = 3;
const C_MID: u16 = 4;
const C_SD: u16 = 5;
const C_UP: u16 = 6;
const C_LO: u16 = 7;
const C_PB: u16 = 8;
const C_BW: u16 = 9;
const C_BR: u16 = 10;
const C_WPLOT: u16 = 11;
const C_BHI: u16 = 15;
const C_BLO: u16 = 16;
const S_C: u16 = 18;

pub struct Bands;

impl SheetModule for Bands {
    fn id(&self) -> &'static str {
        "bod-bands"
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
            "Personal Bollinger bands, %b, width, breaches",
            "BOD-05",
            "Q23 | KAU Ch7 (standard deviation moving average), Ch4 (charting systems)",
            Banner::Note(
                "Band = mean +- Boll_K sd of the weigh-ins in the last Bod_Boll_days days (at least 3); \
                 a breach is a weigh-in outside the band: unusual for you, not a diagnosis.",
            ),
        )?;
        let n = n_days(cx);
        let (r0, r1) = (HR + 1, HR + n);
        let w = rng(TITLE, C_W, r0, r1);
        section(ws, cx, TOP, 0, "Daily series (one row per calendar day, Bod_Cal)")?;
        let mut cols = cal_head(HR);
        cols.push(Col::f("n_win", |r| {
            let k = r - HR - 1;
            format!("=IF({k}<Bod_Boll_days,\"\",COUNT({}))", win(&w, k, "Bod_Boll_days"))
        }));
        cols.push(Col::f("mid", |r| {
            format!("=IF(N({})>=3,AVERAGE({}),\"\")", at(C_N, r), win(&w, r - HR - 1, "Bod_Boll_days"))
        }));
        cols.push(Col::f("sd", |r| {
            format!("=IF(N({})>=3,STDEV.S({}),\"\")", at(C_N, r), win(&w, r - HR - 1, "Bod_Boll_days"))
        }));
        for (h, sign) in [("upper", "+"), ("lower", "-")] {
            cols.push(Col::f(h, move |r| {
                let (m, s) = (at(C_MID, r), at(C_SD, r));
                format!("=IF(AND(ISNUMBER({m}),ISNUMBER({s})),{m}{sign}Boll_K*{s},\"\")")
            }));
        }
        cols.push(Col::f("pct_b", |r| {
            let (x, u, l) = (at(C_W, r), at(C_UP, r), at(C_LO, r));
            format!("=IF(AND(ISNUMBER({x}),ISNUMBER({u})),IF({u}>{l},({x}-{l})/({u}-{l}),\"\"),\"\")")
        }));
        cols.push(Col::f("bandwidth", |r| {
            let (m, u, l) = (at(C_MID, r), at(C_UP, r), at(C_LO, r));
            format!("=IF(ISNUMBER({u}),IF({m}<>0,({u}-{l})/{m},\"\"),\"\")")
        }));
        cols.push(Col::f("breach", |r| {
            let (x, u, l) = (at(C_W, r), at(C_UP, r), at(C_LO, r));
            format!("=IF(AND(ISNUMBER({x}),ISNUMBER({u})),IF({x}>{u},1,IF({x}<{l},-1,0)),\"\")")
        }));
        cols.push(plot_col("weight_plot", C_W));
        cols.push(plot_col("mid_plot", C_MID));
        cols.push(plot_col("upper_plot", C_UP));
        cols.push(plot_col("lower_plot", C_LO));
        for (h, v) in [("breach_hi", 1), ("breach_lo", -1)] {
            cols.push(Col::f(h, move |r| format!("=IF(N({})={v},{},NA())", at(C_BR, r), at(C_W, r))));
        }
        table(ws, cx, "tblBodBands", HR, 0, &cols, n)?;
        super::date_rows(ws, cx, 0, r0, r1)?;
        super::fmt_rows(ws, r0, r1, C_MID, C_LO, "0.0")?;
        super::fmt_rows(ws, r0, r1, C_SD, C_SD, "0.00")?;
        super::fmt_rows(ws, r0, r1, C_PB, C_PB, "0.00")?;
        super::fmt_rows(ws, r0, r1, C_BW, C_BW, "0.0%")?;
        let b = at(C_BR, r0 + 1);
        theme::cf_formula(ws, r0, C_BR, r1, C_BR, &format!("={b}=1"), theme::CRITICAL)?;
        theme::cf_formula(ws, r0, C_BR, r1, C_BR, &format!("={b}=-1"), theme::BAND_BLUEPRINT)?;
        theme::cf_three_color(ws, r0, C_PB, r1, C_PB, true)?;
        theme::cf_data_bar(ws, r0, C_BW, r1, C_BW, theme::BAND_BLUEPRINT)?;

        // Summary.
        let last = |c: u16| format!("={}", last_num(&rng(TITLE, c, r0, r1)));
        let br = rng(TITLE, C_BR, r0, r1);
        section(ws, cx, TOP, S_C, "Summary (latest day)")?;
        kv(ws, cx, TOP + 1, S_C, "Middle band (lb)", &last(C_MID), Some("0.0"))?;
        kv(ws, cx, TOP + 2, S_C, "Upper band (lb)", &last(C_UP), Some("0.0"))?;
        kv(ws, cx, TOP + 3, S_C, "Lower band (lb)", &last(C_LO), Some("0.0"))?;
        kv(ws, cx, TOP + 4, S_C, "%b (last weigh-in)", &last(C_PB), Some("0.00"))?;
        kv(ws, cx, TOP + 5, S_C, "Band width (% of mid)", &last(C_BW), Some("0.0%"))?;
        kv(ws, cx, TOP + 6, S_C, "Breaches above", &format!("=COUNTIF({br},1)"), Some("0"))?;
        kv(ws, cx, TOP + 7, S_C, "Breaches below", &format!("=COUNTIF({br},-1)"), Some("0"))?;
        kv(
            ws,
            cx,
            TOP + 8,
            S_C,
            "Weigh-ins inside the band (%)",
            &format!("=IFERROR(COUNTIF({br},0)/COUNT({br}),\"\")"),
            Some("0%"),
        )?;

        let mut chart = date_chart("Weight with Bollinger bands (Bod_Boll_days, Boll_K)");
        series(&mut chart, TITLE, "weigh-in (lb)", r0, r1, C_WPLOT, true);
        series(&mut chart, TITLE, "mid", r0, r1, C_WPLOT + 1, false);
        series(&mut chart, TITLE, "upper", r0, r1, C_WPLOT + 2, false);
        series(&mut chart, TITLE, "lower", r0, r1, C_WPLOT + 3, false);
        for (name, c, hex) in [("above band", C_BHI, "C00000"), ("below band", C_BLO, "2E75B6")] {
            let fill = theme::solid(hex);
            chart
                .add_series()
                .set_name(name)
                .set_categories((TITLE, r0, 0, r1, 0))
                .set_values((TITLE, r0, c, r1, c))
                .set_format(ChartFormat::new().set_no_line())
                .set_marker(
                    ChartMarker::new()
                        .set_type(ChartMarkerType::Diamond)
                        .set_size(8)
                        .set_format(ChartFormat::new().set_solid_fill(&fill)),
                );
        }
        ws.insert_chart(TOP + 11, S_C, &chart)?;
        set_widths(
            ws,
            &[
                12.0, 10.0, 10.0, 7.0, 8.0, 7.0, 8.0, 8.0, 7.0, 9.0, 7.0, 10.0, 9.0, 10.0, 10.0, 9.0, 9.0, 2.0, 28.0,
                10.0,
            ],
        )?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
