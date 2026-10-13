//! BOD-08 `Bod_Cusum`: two-sided CUSUM change-point detector on the daily weight
//! (SPC technique [EXT]; motivation CHAN regime switching, KAU Ch22; quant.md Q26).
//!
//! Row-aligned with `Bod_Cal`. The monitored series `x` is chosen in a cell:
//! * `level` (default): the one-step forecast error of a weigh-in against the
//!   `Bod_Weight` EMA of the day before, `w_t - ema_{t-1}`: a level shift (a step)
//!   gives a run of same-signed errors while the EMA catches up;
//! * `rate`: the daily change `dw_day` from `Bod_Cal` (lb/day since the previous
//!   weigh-in): detects a sustained change of the rate, but a one-day step only once.
//!
//! mu and sd of `x` over the first `Bod_CUSUM_base` days (cells); on weigh-in days
//! z = (x - mu)/sd; S+ = MAX(0, S+_{t-1} + z - CUSUM_k), S- = MAX(0, S-_{t-1} - z - CUSUM_k)
//! (held on other days); an alarm when either arm exceeds CUSUM_h, after which both arms
//! restart from 0. A running alarm counter lets INDEX/MATCH list the first 20 alarms.

use rust_xlsxwriter::{ChartFormat, ChartMarker, ChartMarkerType, Worksheet};

use super::signal::{date_chart, series};
use super::{abs, at, cal_at, cal_head, head, kpi, kv, list, n_days, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Cusum";
const HR: u32 = TOP + 1;
const C_DW: u16 = 3;
const C_FE: u16 = 4;
const C_X: u16 = 5;
const C_Z: u16 = 6;
const C_SHI: u16 = 7;
const C_SLO: u16 = 8;
const C_AL: u16 = 9;
const C_DIR: u16 = 10;
const C_NO: u16 = 11;
const C_H: u16 = 12;
const C_APLOT: u16 = 13;
const S_C: u16 = 15;
const A_HR: u32 = TOP + 12;
const ALARMS: u32 = 20;
pub const INPUTS: [&str; 2] = ["level", "rate"];

pub struct Cusum;

impl SheetModule for Cusum {
    fn id(&self) -> &'static str {
        "bod-cusum"
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
            "Two-sided CUSUM change-point detector",
            "BOD-08",
            "Q26 | CUSUM (SPC, [EXT]); CHAN regime switching; KAU Ch22 (price shocks)",
            Banner::Note(
                "S+ / S- add up standardised deviations beyond CUSUM_k sd; an alarm (> CUSUM_h) marks a likely \
                 shift in your weight, then both arms restart. Baseline: first Bod_CUSUM_base days.",
            ),
        )?;
        let n = n_days(cx);
        let (r0, r1) = (HR + 1, HR + n);
        let first = HR + 2;
        let ema = super::weight::ema_range(cx);
        let (sel, mu, sd) = (abs(S_C + 1, TOP + 1), abs(S_C + 1, TOP + 2), abs(S_C + 1, TOP + 3));
        section(ws, cx, TOP, 0, "Daily detector (one row per calendar day, Bod_Cal)")?;
        let mut cols = cal_head(HR);
        cols.push(Col::f("dw_day", |r| format!("={}", cal_at("dw_day", HR, r))));
        cols.push(Col::f("fc_err", move |r| {
            if r == first {
                "=\"\"".into()
            } else {
                let w = at(1, r);
                let e = format!("INDEX({ema},{})", r - HR - 2);
                format!("=IF(ISNUMBER({w}),IFERROR({w}-{e},\"\"),\"\")")
            }
        }));
        cols.push(Col::f("x", |r| format!("=IF({sel}=\"rate\",{},{})", at(C_DW, r), at(C_FE, r))));
        cols.push(Col::f("z", |r| {
            let x = at(C_X, r);
            format!("=IF(AND(ISNUMBER({x}),ISNUMBER({sd}),ISNUMBER({mu})),IF({sd}>0,({x}-{mu})/{sd},\"\"),\"\")")
        }));
        for (hd, c, sign) in [("s_hi", C_SHI, ""), ("s_lo", C_SLO, "-")] {
            cols.push(Col::f(hd, move |r| {
                let z = at(C_Z, r);
                if r == first {
                    format!("=IF(ISNUMBER({z}),MAX(0,{sign}{z}-CUSUM_k),0)")
                } else {
                    let base = format!("IF({}=1,0,{})", at(C_AL, r - 1), at(c, r - 1));
                    format!(
                        "=IF(ISNUMBER({z}),MAX(0,{base}{m}{z}-CUSUM_k),{base})",
                        m = if sign.is_empty() { "+" } else { "-" }
                    )
                }
            }));
        }
        cols.push(Col::f("alarm", |r| format!("=IF(OR({}>CUSUM_h,{}>CUSUM_h),1,0)", at(C_SHI, r), at(C_SLO, r))));
        cols.push(Col::f("direction", |r| {
            let (a, hi, lo) = (at(C_AL, r), at(C_SHI, r), at(C_SLO, r));
            format!("=IF({a}=1,IF(AND({hi}>CUSUM_h,{lo}>CUSUM_h),\"both\",IF({hi}>CUSUM_h,\"up\",\"down\")),\"\")")
        }));
        cols.push(Col::f("alarm_no", move |r| {
            if r == first {
                format!("={}", at(C_AL, r))
            } else {
                format!("={}+{}", at(C_NO, r - 1), at(C_AL, r))
            }
        }));
        cols.push(Col::f("h_line", |_| "=CUSUM_h".into()));
        cols.push(Col::f("alarm_plot", |r| {
            format!("=IF({}=1,MAX({},{}),NA())", at(C_AL, r), at(C_SHI, r), at(C_SLO, r))
        }));
        table(ws, cx, "tblBodCusum", HR, 0, &cols, n)?;
        super::date_rows(ws, cx, 0, r0, r1)?;
        super::fmt_rows(ws, r0, r1, C_DW, C_X, "+0.00;-0.00;0.00")?;
        super::fmt_rows(ws, r0, r1, C_Z, C_SLO, "0.00")?;
        let a = at(C_AL, r0 + 1);
        theme::cf_formula(ws, r0, C_AL, r1, C_DIR, &format!("=${a}=1"), theme::CRITICAL)?;
        theme::cf_data_bar(ws, r0, C_SHI, r1, C_SHI, theme::OUT_OF_REF)?;
        theme::cf_data_bar(ws, r0, C_SLO, r1, C_SLO, theme::BAND_BLUEPRINT)?;

        // Summary.
        let (x, al, no, dates) =
            (rng(TITLE, C_X, r0, r1), rng(TITLE, C_AL, r0, r1), rng(TITLE, C_NO, r0, r1), rng(TITLE, 0, r0, r1));
        let base = format!("INDEX({x},1):INDEX({x},MIN({n},Bod_CUSUM_base))");
        section(ws, cx, TOP, S_C, "Detector")?;
        ws.write_string_with_format(TOP + 1, S_C, "Input series (level / rate)", &cx.f.bold)?;
        ws.write_string_with_format(TOP + 1, S_C + 1, INPUTS[0], &cx.f.input)?;
        list(ws, TOP + 1, S_C + 1, TOP + 1, &INPUTS)?;
        kv(
            ws,
            cx,
            TOP + 2,
            S_C,
            "Baseline mean of x",
            &format!("=IFERROR(AVERAGE({base}),\"\")"),
            Some("+0.000;-0.000"),
        )?;
        kv(ws, cx, TOP + 3, S_C, "Baseline sd of x", &format!("=IFERROR(STDEV.S({base}),\"\")"), Some("0.000"))?;
        kv(ws, cx, TOP + 4, S_C, "Alarms", &format!("=SUM({al})"), Some("0"))?;
        let cnt = abs(S_C + 1, TOP + 4);
        let last_row = format!("MATCH({cnt},{no},0)");
        kv(
            ws,
            cx,
            TOP + 5,
            S_C,
            "Last alarm",
            &format!("=IF({cnt}>0,IFERROR(INDEX({dates},{last_row}),\"\"),\"\")"),
            Some("yyyy-mm-dd"),
        )?;
        let dir = rng(TITLE, C_DIR, r0, r1);
        kv(
            ws,
            cx,
            TOP + 6,
            S_C,
            "Last alarm direction",
            &format!("=IF({cnt}>0,IFERROR(INDEX({dir},{last_row}),\"\"),\"\")"),
            None,
        )?;
        let lastv = |c: u16| format!("=INDEX({},{n})", rng(TITLE, c, r0, r1));
        kv(ws, cx, TOP + 7, S_C, "S+ today", &lastv(C_SHI), Some("0.00"))?;
        kv(ws, cx, TOP + 8, S_C, "S- today", &lastv(C_SLO), Some("0.00"))?;
        kpi(cx, "cusum_alarms", "CUSUM weight-shift alarms", TITLE, TOP + 4, S_C + 1, Some("0"))?;
        theme::cf_formula(ws, TOP + 4, S_C + 1, TOP + 4, S_C + 1, &format!("={cnt}>0"), theme::OUT_OF_REF)?;
        ws.write_string_with_format(
            TOP + 9,
            S_C,
            "level: weigh-in minus the previous day's Bod_Weight EMA; rate: Bod_Cal dw_day. k, h: CUSUM_k, CUSUM_h.",
            &cx.f.note,
        )?;

        // Alarm list: the i-th alarm is the row where the running counter first equals i.
        section(ws, cx, A_HR - 1, S_C, "Alarms (first 20)")?;
        let wr = rng(TITLE, 1, r0, r1);
        let acols = vec![
            Col::f("alarm_seq", |r| format!("={}", r - A_HR - 1)),
            Col::f("day_row", |r| {
                let i = at(S_C, r);
                format!("=IF({i}<={cnt},IFERROR(MATCH({i},{no},0),\"\"),\"\")")
            }),
            Col::f("date", |r| {
                let k = at(S_C + 1, r);
                format!("=IF(ISNUMBER({k}),INDEX({dates},{k}),\"\")")
            }),
            Col::f("direction", |r| {
                let k = at(S_C + 1, r);
                format!("=IF(ISNUMBER({k}),INDEX({dir},{k}),\"\")")
            }),
            Col::f("weight_lb", |r| {
                let k = at(S_C + 1, r);
                format!("=IF(ISNUMBER({k}),INDEX({wr},{k}),\"\")")
            }),
        ];
        table(ws, cx, "tblBodCusumAlarms", A_HR, S_C, &acols, ALARMS)?;
        super::date_rows(ws, cx, S_C + 2, A_HR + 1, A_HR + ALARMS)?;

        let mut chart = date_chart("CUSUM arms vs threshold h");
        series(&mut chart, TITLE, "S+ (up)", r0, r1, C_SHI, false);
        series(&mut chart, TITLE, "S- (down)", r0, r1, C_SLO, false);
        series(&mut chart, TITLE, "h", r0, r1, C_H, false);
        chart
            .add_series()
            .set_name("alarm")
            .set_categories((TITLE, r0, 0, r1, 0))
            .set_values((TITLE, r0, C_APLOT, r1, C_APLOT))
            .set_format(ChartFormat::new().set_no_line())
            .set_marker(
                ChartMarker::new()
                    .set_type(ChartMarkerType::Diamond)
                    .set_size(8)
                    .set_format(ChartFormat::new().set_solid_fill(&theme::solid("C00000"))),
            );
        ws.insert_chart(A_HR + ALARMS + 2, S_C, &chart)?;
        set_widths(
            ws,
            &[
                12.0, 10.0, 10.0, 8.0, 8.0, 8.0, 7.0, 7.0, 7.0, 6.0, 9.0, 8.0, 6.0, 9.0, 2.0, 28.0, 12.0, 12.0, 9.0,
                10.0,
            ],
        )?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
