//! BOD-02 `Bod_Rate`: weekly rate of weight change against the cut / gain guard-rails,
//! weight lost against the 5 % line, and the in-band streak (clinical.md C4.1, C4.2;
//! longevity.md L-D2; HELMS ch01 l.1090-1127, DIP ch167:19).
//!
//! * Daily table (row-aligned with `Bod_Cal`): `mean7` = AVERAGEIFS of the weigh-ins
//!   dated in (d-7, d], `mean7_prev` over (d-14, d-7], `rate_pct_wk` = (mean7/mean7_prev - 1)*100,
//!   the selected band and `in_band` (1/0), `pct_lost` = (1 - weight_ff/start)*100 against
//!   `Bod_Loss_line_pct`.
//! * Mode cell (drop-down) and a modes table: cut = -Bod_Cut_hi_pct .. -Bod_Cut_lo_pct %/week;
//!   gain bands are %/month in Params, shown as %/week (*12/52); maintain = +-Bod_Cut_lo_pct/2.
//! * Weekly table: the last 26 Monday-start weeks up to the last log date: mean weight,
//!   rate vs the previous week, in band, streak of consecutive in-band weeks
//!   (KPI `band_streak`).
//! * Start weight = the carried-forward weight on `Bod_Plan_start` when set, else the
//!   first weigh-in.

use rust_xlsxwriter::Worksheet;

use super::signal::{date_chart, series};
use super::{abs, at, cal_col, cal_head, head, kpi, kv, last_num, list, n_days, plot_col, rng, section, table, Banner};
use super::{Col, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Rate";
pub const MODES: [&str; 5] = ["cut", "gain-beginner", "gain-intermediate", "gain-advanced", "maintain"];
const HR: u32 = TOP + 1;
const C_FF: u16 = 2;
const C_M7: u16 = 3;
const C_RATE: u16 = 5;
const C_LO: u16 = 6;
const C_IN: u16 = 8;
const C_LOST: u16 = 9;
const C_LINE: u16 = 10;
const C_RPLOT: u16 = 11;
const S_C: u16 = 14;
const M_HR: u32 = TOP + 12;
const W_HR: u32 = TOP + 20;
const WEEKS: u32 = 26;

pub struct Rate;

impl SheetModule for Rate {
    fn id(&self) -> &'static str {
        "bod-rate"
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
            "Weekly rate vs cut/gain guard-rails, % lost vs 5% line, in-band streak",
            "BOD-02",
            "C4.1, C4.2, L-D2 | HELMS ch01:955,1090-1127 (rate bands); DIP ch167:19 (5% loss), ch164:89,229",
            Banner::Note(
                "Rate = change of the 7-day mean weight vs the 7 days before (%/week). Pick the mode on the right; \
                 bands come from Params (gain bands are %/month, shown per week).",
            ),
        )?;
        let n = n_days(cx);
        let (r0, r1) = (HR + 1, HR + n);
        let (cw, cd) = (cal_col(cx, "weight_lb"), cal_col(cx, "date"));
        let (mode, blo, bhi, start) =
            (abs(S_C + 1, TOP + 1), abs(S_C + 1, TOP + 2), abs(S_C + 1, TOP + 3), abs(S_C + 1, TOP + 6));
        let mean =
            |from: &str, to: &str| format!("IFERROR(AVERAGEIFS({cw},{cd},\">\"&({from}),{cd},\"<=\"&({to})),\"\")");
        section(ws, cx, TOP, 0, "Daily series (one row per calendar day, Bod_Cal)")?;
        let mut cols = cal_head(HR);
        cols.push(Col::f("mean7", |r| {
            let d = at(0, r);
            format!("=IF(ISNUMBER({d}),{},\"\")", mean(&format!("{d}-7"), &d))
        }));
        cols.push(Col::f("mean7_prev", |r| {
            let d = at(0, r);
            format!("=IF(ISNUMBER({d}),{},\"\")", mean(&format!("{d}-14"), &format!("{d}-7")))
        }));
        cols.push(Col::f("rate_pct_wk", |r| {
            let (m, p) = (at(C_M7, r), at(C_M7 + 1, r));
            format!("=IF(AND(ISNUMBER({m}),ISNUMBER({p})),IF({p}>0,({m}/{p}-1)*100,\"\"),\"\")")
        }));
        cols.push(Col::f("band_lo", |_| format!("={blo}")));
        cols.push(Col::f("band_hi", |_| format!("={bhi}")));
        cols.push(Col::f("in_band", |r| {
            let x = at(C_RATE, r);
            format!("=IF(AND(ISNUMBER({x}),ISNUMBER({blo})),IF(AND({x}>={blo},{x}<={bhi}),1,0),\"\")")
        }));
        cols.push(Col::f("pct_lost", |r| {
            let (d, f) = (at(0, r), at(C_FF, r));
            format!(
                "=IF(AND(ISNUMBER({f}),ISNUMBER({start})),IF(AND(ISNUMBER(Bod_Plan_start),{d}<Bod_Plan_start),\"\",\
                 IF({start}>0,(1-{f}/{start})*100,\"\")),\"\")"
            )
        }));
        cols.push(Col::f("loss_line", |_| "=Bod_Loss_line_pct".into()));
        cols.push(plot_col("rate_plot", C_RATE));
        cols.push(plot_col("lost_plot", C_LOST));
        table(ws, cx, "tblBodRate", HR, 0, &cols, n)?;
        super::date_rows(ws, cx, 0, r0, r1)?;
        super::fmt_rows(ws, r0, r1, C_M7, C_M7 + 1, "0.0")?;
        super::fmt_rows(ws, r0, r1, C_RATE, C_LO + 1, "+0.00;-0.00;0.00")?;
        super::fmt_rows(ws, r0, r1, C_LOST, C_LINE, "0.0")?;
        let i = at(C_IN, r0 + 1);
        theme::cf_formula(ws, r0, C_RATE, r1, C_RATE, &format!("=${i}=1"), theme::OPTIMAL)?;
        theme::cf_formula(ws, r0, C_RATE, r1, C_RATE, &format!("=${i}=0"), theme::OUT_OF_REF)?;
        let l = at(C_LOST, r0 + 1);
        theme::cf_formula(
            ws,
            r0,
            C_LOST,
            r1,
            C_LOST,
            &format!("=AND(ISNUMBER({l}),{l}>=Bod_Loss_line_pct)"),
            theme::OPTIMAL,
        )?;

        // Mode, band and summary.
        section(ws, cx, TOP, S_C, "Guard-rail band and summary")?;
        ws.write_string_with_format(TOP + 1, S_C, "Mode", &cx.f.bold)?;
        ws.write_string_with_format(TOP + 1, S_C + 1, MODES[0], &cx.f.input)?;
        list(ws, TOP + 1, S_C + 1, TOP + 1, &MODES)?;
        let (mm, ml, mh) = (
            rng(TITLE, S_C, M_HR + 1, M_HR + 5),
            rng(TITLE, S_C + 1, M_HR + 1, M_HR + 5),
            rng(TITLE, S_C + 2, M_HR + 1, M_HR + 5),
        );
        let pick = |r: &str| format!("=IFERROR(INDEX({r},MATCH({mode},{mm},0)),\"\")");
        kv(ws, cx, TOP + 2, S_C, "Band low (%/week)", &pick(&ml), Some("+0.00;-0.00;0.00"))?;
        kv(ws, cx, TOP + 3, S_C, "Band high (%/week)", &pick(&mh), Some("+0.00;-0.00;0.00"))?;
        let rate = rng(TITLE, C_RATE, r0, r1);
        kv(ws, cx, TOP + 4, S_C, "Rate now (%/week)", &format!("={}", last_num(&rate)), Some("+0.00;-0.00;0.00"))?;
        let inb = last_num(&rng(TITLE, C_IN, r0, r1));
        kv(
            ws,
            cx,
            TOP + 5,
            S_C,
            "In band now",
            &format!("=IF(ISNUMBER({inb}),IF({inb}=1,\"yes\",\"no\"),\"\")"),
            None,
        )?;
        let (w, ff, lobs) = (cw.clone(), rng(TITLE, C_FF, r0, r1), cal_col(cx, "last_obs"));
        let first_w = format!("SUMIFS({w},{cd},MIN({lobs}))");
        kv(
            ws,
            cx,
            TOP + 6,
            S_C,
            "Start weight (lb)",
            &format!(
                "=IF(COUNT({w})=0,\"\",IF(ISNUMBER(Bod_Plan_start),IFERROR(1*INDEX({ff},MATCH(Bod_Plan_start,{cd},1)),{first_w}),{first_w}))"
            ),
            Some("0.0"),
        )?;
        let lost = last_num(&rng(TITLE, C_LOST, r0, r1));
        kv(ws, cx, TOP + 7, S_C, "Weight lost (%)", &format!("={lost}"), Some("0.0"))?;
        let lc = abs(S_C + 1, TOP + 7);
        kv(
            ws,
            cx,
            TOP + 8,
            S_C,
            "Reached Bod_Loss_line_pct",
            &format!("=IF(ISNUMBER({lc}),IF({lc}>=Bod_Loss_line_pct,\"yes\",\"not yet\"),\"\")"),
            None,
        )?;
        let streak = rng(TITLE, S_C + 5, W_HR + 1, W_HR + WEEKS);
        kv(ws, cx, TOP + 9, S_C, "In-band streak (weeks)", &format!("={}", last_num(&streak)), Some("0"))?;
        kpi(cx, "band_streak", "Weeks in a row inside the rate band", TITLE, TOP + 9, S_C + 1, Some("0"))?;
        let yes = abs(S_C + 1, TOP + 5);
        theme::cf_formula(ws, TOP + 5, S_C + 1, TOP + 5, S_C + 1, &format!("={yes}=\"yes\""), theme::OPTIMAL)?;
        theme::cf_formula(ws, TOP + 5, S_C + 1, TOP + 5, S_C + 1, &format!("={yes}=\"no\""), theme::OUT_OF_REF)?;

        // Modes table.
        section(ws, cx, M_HR - 1, S_C, "Bands by mode (%/week; gain Params are %/month * 12/52)")?;
        let band = |r: u32, hi: bool| -> String {
            match (MODES[(r - M_HR - 2) as usize], hi) {
                ("cut", false) => "=-Bod_Cut_hi_pct".into(),
                ("cut", true) => "=-Bod_Cut_lo_pct".into(),
                ("gain-beginner", false) => "=Bod_Gain_beg_lo*12/52".into(),
                ("gain-beginner", true) => "=Bod_Gain_beg_hi*12/52".into(),
                ("gain-intermediate", false) => "=Bod_Gain_int_lo*12/52".into(),
                ("gain-intermediate", true) => "=Bod_Gain_int_hi*12/52".into(),
                ("gain-advanced", false) => "=0".into(),
                ("gain-advanced", true) => "=Bod_Gain_adv_hi*12/52".into(),
                (_, false) => "=-Bod_Cut_lo_pct/2".into(),
                (_, true) => "=Bod_Cut_lo_pct/2".into(),
            }
        };
        let mcols = vec![
            Col::f("mode", |r| format!("=\"{}\"", MODES[(r - M_HR - 2) as usize])),
            Col::f("lo_pct_wk", |r| band(r, false)),
            Col::f("hi_pct_wk", |r| band(r, true)),
            Col::f("selected", |r| format!("=IF({}={mode},\"<<\",\"\")", at(S_C, r))),
        ];
        table(ws, cx, "tblBodRateModes", M_HR, S_C, &mcols, MODES.len() as u32)?;
        super::fmt_rows(ws, M_HR + 1, M_HR + 5, S_C + 1, S_C + 2, "+0.00;-0.00;0.00")?;
        let sm = at(S_C + 3, M_HR + 2);
        theme::cf_formula(ws, M_HR + 1, S_C, M_HR + 5, S_C + 3, &format!("=${sm}=\"<<\""), theme::BAND_REF)?;

        // Weekly table, oldest first; the last row is the week of the last log date.
        section(ws, cx, W_HR - 1, S_C, "Weeks (Monday start) up to the last log date")?;
        let wk = |r: u32| at(S_C, r);
        let wmean = |from: String| format!("IFERROR(AVERAGEIFS({cw},{cd},\">=\"&({from}),{cd},\"<\"&({from}+7)),\"\")");
        let wfirst = W_HR + 2;
        let wcols = vec![
            Col::f("week_of", |r| {
                let k = WEEKS - (r - (W_HR + 1));
                format!("=IF(COUNT({cd})=0,\"\",MAX({cd})-WEEKDAY(MAX({cd}),2)+1-7*{k})")
            }),
            Col::f("mean_lb", |r| format!("=IF(ISNUMBER({w}),{},\"\")", wmean(wk(r)), w = wk(r))),
            Col::f("prev_mean_lb", |r| format!("=IF(ISNUMBER({w}),{},\"\")", wmean(format!("{}-7", wk(r))), w = wk(r))),
            Col::f("rate_pct_wk", |r| {
                let (m, p) = (at(S_C + 1, r), at(S_C + 2, r));
                format!("=IF(AND(ISNUMBER({m}),ISNUMBER({p})),IF({p}>0,({m}/{p}-1)*100,\"\"),\"\")")
            }),
            Col::f("in_band", |r| {
                let x = at(S_C + 3, r);
                format!("=IF(AND(ISNUMBER({x}),ISNUMBER({blo})),IF(AND({x}>={blo},{x}<={bhi}),1,0),\"\")")
            }),
            Col::f("streak", |r| {
                let i = at(S_C + 4, r);
                if r == wfirst {
                    format!("=IF(N({i})=1,1,0)")
                } else {
                    format!("=IF(N({i})=1,{}+1,0)", at(S_C + 5, r - 1))
                }
            }),
        ];
        table(ws, cx, "tblBodRateWeeks", W_HR, S_C, &wcols, WEEKS)?;
        let (w0, w1) = (W_HR + 1, W_HR + WEEKS);
        super::date_rows(ws, cx, S_C, w0, w1)?;
        super::fmt_rows(ws, w0, w1, S_C + 1, S_C + 2, "0.0")?;
        super::fmt_rows(ws, w0, w1, S_C + 3, S_C + 3, "+0.00;-0.00;0.00")?;
        let wi = at(S_C + 4, w0 + 1);
        theme::cf_formula(ws, w0, S_C + 4, w1, S_C + 4, &format!("={wi}=1"), theme::OPTIMAL)?;
        theme::cf_formula(ws, w0, S_C + 4, w1, S_C + 4, &format!("={wi}=0"), theme::OUT_OF_REF)?;
        theme::cf_data_bar(ws, w0, S_C + 5, w1, S_C + 5, theme::BAND_OPT)?;

        // Charts.
        let mut chart = date_chart("Weekly rate (%/week) inside the guard-rail band");
        series(&mut chart, TITLE, "rate", r0, r1, C_RPLOT, false);
        series(&mut chart, TITLE, "band low", r0, r1, C_LO, false);
        series(&mut chart, TITLE, "band high", r0, r1, C_LO + 1, false);
        ws.insert_chart(w1 + 3, S_C, &chart)?;
        let mut chart = date_chart("Weight lost (%) vs the Bod_Loss_line_pct line");
        series(&mut chart, TITLE, "lost %", r0, r1, C_RPLOT + 1, false);
        series(&mut chart, TITLE, "loss line", r0, r1, C_LINE, false);
        ws.insert_chart(w1 + 21, S_C, &chart)?;
        set_widths(
            ws,
            &[
                12.0, 10.0, 10.0, 8.0, 10.0, 11.0, 8.0, 8.0, 8.0, 8.0, 9.0, 9.0, 9.0, 2.0, 24.0, 12.0, 12.0, 11.0, 8.0,
                7.0,
            ],
        )?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
