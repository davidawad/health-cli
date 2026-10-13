//! BOD-12 `Bod_Fan`: analytic fan chart of body weight (Q68). An analytic random walk
//! with drift, NOT a forecast.
//!
//! The drift `mu` and SD `sd` of the daily change of `weight_ff` (carried-forward
//! weight, `Bod_Cal`) over the last `Bod_Fan_hist` days are computed from a
//! row-aligned helper table. The projection table (t = 0..180 days, blank beyond
//! `Bod_Fan_days`) starts at the last `weight_ff`: median = start + mu*t, bands
//! median +- z*sd*SQRT(t) at z = NORM.S.INV(0.75 / 0.9 / 0.975) (50 / 80 / 95 %), the
//! probability of being at or past `Bod_Goal_lb` on day t, and the date the median
//! reaches the goal when one is set.

use rust_xlsxwriter::Worksheet;

use super::{
    abs, at, cal_head, head, kv, last_num, line_chart, n_days, plot_col, rng, section, table, Banner, Col, TOP,
};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Fan";
const HR: u32 = TOP + 1;
const C_FF: u16 = 2;
const C_DFF: u16 = 3;
const C_IN: u16 = 4;
const C_DZ: u16 = 5;
const SC: u16 = 7;
/// Projection table: t, date, median, 6 bands, p_at_goal, 7 plot columns.
const PC: u16 = SC + 3;
const P_HR: u32 = TOP + 19;
/// Projection rows beyond t = 0 (the horizon parameter is capped here).
pub const FAN_ROWS: u32 = 180;
/// (header, summary row `v(k)` of its z, sign).
const BANDS: [(&str, u32, i8); 6] =
    [("lo95", 11, -1), ("lo80", 10, -1), ("lo50", 9, -1), ("hi50", 9, 1), ("hi80", 10, 1), ("hi95", 11, 1)];

pub struct Fan;

impl SheetModule for Fan {
    fn id(&self) -> &'static str {
        "bod-fan"
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
            "Analytic fan-chart projection mean + z*sd*sqrt(t)",
            "BOD-12",
            "Q68 | KAU Ch21; PAL Ch2 (random walk with drift)",
            Banner::Note("Analytic random walk with drift, not a forecast: bands only say how far noise alone could carry the weight."),
        )?;
        let n = n_days(cx);
        let first = HR + 2;
        let (r0, r1) = (HR + 1, HR + n);
        let col = |c: u16| rng(TITLE, c, r0, r1);
        let v = |k: u32| abs(SC + 1, TOP + k);
        let (last, start, nh, mu, sd) = (v(1), v(2), v(3), v(4), v(5));

        section(ws, cx, TOP, 0, "Daily change of weight_ff (Bod_Cal)")?;
        let mut cols = cal_head(HR);
        cols.push(Col::f("dff", move |r| {
            if r == first {
                "=\"\"".into()
            } else {
                let (f, p) = (at(C_FF, r), at(C_FF, r - 1));
                format!("=IF(AND(ISNUMBER({f}),ISNUMBER({p})),{f}-{p},\"\")")
            }
        }));
        cols.push(Col::f("in_hist", move |r| {
            let i = r - first + 1;
            format!("=IF(AND(ISNUMBER({}),{i}>{n}-Bod_Fan_hist),1,0)", at(C_DFF, r))
        }));
        cols.push(Col::f("dz", |r| format!("=IF({}=1,{},0)", at(C_IN, r), at(C_DFF, r))));
        table(ws, cx, "tblBodFanHist", HR, 0, &cols, n)?;
        super::date_rows(ws, cx, 0, r0, r1)?;
        super::fmt_rows(ws, r0, r1, C_DFF, C_DFF, "+0.0;-0.0;0.0")?;
        let h = at(C_IN, r0 + 1);
        theme::cf_formula(ws, r0, C_IN, r1, C_IN, &format!("={h}=1"), theme::BAND_REF)?;

        // Drift, SD and the goal crossing.
        let (dates, ff, inh, dz) = (col(0), col(C_FF), col(C_IN), col(C_DZ));
        section(ws, cx, TOP, SC, "Random walk with drift")?;
        let goal = "Bod_Goal_lb";
        let rows: [(&str, String, &str); 12] = [
            ("Last day of the log", format!("=IF(COUNT({dates})=0,\"\",MAX({dates}))"), "yyyy-mm-dd"),
            ("Start level: last weight_ff (lb)", format!("={}", last_num(&ff)), "0.0"),
            ("Days of change in the history", format!("=SUM({inh})"), "0"),
            ("Drift mu (lb/day)", format!("=IF({nh}>0,SUM({dz})/{nh},\"\")"), "+0.000;-0.000"),
            (
                "SD of daily change (lb)",
                format!("=IF({nh}>1,SQRT(SUMPRODUCT({inh},({dz}-{mu})^2)/({nh}-1)),\"\")"),
                "0.000",
            ),
            ("Drift per week (lb)", format!("=IF(ISNUMBER({mu}),7*{mu},\"\")"), "+0.00;-0.00"),
            ("Goal (Bod_Goal_lb)", format!("=IF(ISNUMBER({goal}),{goal},\"not set\")"), "0.0"),
            (
                "Median reaches the goal on",
                format!(
                    "=IF(NOT(ISNUMBER({goal})),\"set Bod_Goal_lb\",IF(AND(ISNUMBER({mu}),ISNUMBER({start}),ISNUMBER({last})),IF({goal}={start},{last},IF({mu}=0,\"no drift\",IF(({goal}-{start})/{mu}>0,{last}+ROUNDUP(({goal}-{start})/{mu},0),\"not on the current drift\"))),\"\"))"
                ),
                "yyyy-mm-dd",
            ),
            ("z 50% band", "=NORM.S.INV(0.75)".into(), "0.000"),
            ("z 80% band", "=NORM.S.INV(0.9)".into(), "0.000"),
            ("z 95% band", "=NORM.S.INV(0.975)".into(), "0.000"),
            ("Projection rows shown", format!("=MIN(Bod_Fan_days,{FAN_ROWS})"), "0"),
        ];
        for (i, (label, f, nf)) in rows.iter().enumerate() {
            kv(ws, cx, TOP + 1 + i as u32, SC, label, f, Some(nf))?;
        }
        ws.write_string_with_format(
            TOP + 14,
            SC,
            format!("Analytic random walk with drift, not a forecast. Horizon capped at {FAN_ROWS} days."),
            &cx.f.note,
        )?;

        // Projection table.
        section(ws, cx, P_HR - 1, PC, "Projection (t days after the last day of the log)")?;
        let rows_n = FAN_ROWS + 1;
        let p_first = P_HR + 2;
        let c_med = PC + 2;
        let mut pc = vec![
            Col::f("t", |r| {
                let k = r - p_first;
                format!("=IF(AND({k}<=Bod_Fan_days,ISNUMBER({start}),ISNUMBER({last})),{k},\"\")")
            }),
            Col::f("date", |r| format!("=IF(ISNUMBER({t}),{last}+{t},\"\")", t = at(PC, r))),
            Col::f("median", |r| {
                format!("=IF(AND(ISNUMBER({t}),ISNUMBER({mu})),{start}+{mu}*{t},\"\")", t = at(PC, r))
            }),
        ];
        for (name, zr, sign) in BANDS {
            let z = v(zr);
            let op = if sign < 0 { "-" } else { "+" };
            let sd = sd.clone();
            pc.push(Col::f(name, move |r| {
                let (m, t) = (at(c_med, r), at(PC, r));
                format!("=IF(AND(ISNUMBER({m}),ISNUMBER({sd})),{m}{op}{z}*{sd}*SQRT({t}),\"\")")
            }));
        }
        pc.push(Col::f("p_at_goal", |r| {
            let (m, t) = (at(c_med, r), at(PC, r));
            let zz = format!("({goal}-{m})/({sd}*SQRT({t}))");
            format!(
                "=IF(AND(ISNUMBER({m}),ISNUMBER({goal}),ISNUMBER({sd})),IF(AND({t}>0,{sd}>0),IF({goal}<={start},NORM.S.DIST({zz},TRUE),1-NORM.S.DIST({zz},TRUE)),\"\"),\"\")"
            )
        }));
        let heads = ["median_plot", "lo95_plot", "lo80_plot", "lo50_plot", "hi50_plot", "hi80_plot", "hi95_plot"];
        for (j, h) in heads.into_iter().enumerate() {
            pc.push(plot_col(h, c_med + j as u16));
        }
        table(ws, cx, "tblBodFan", P_HR, PC, &pc, rows_n)?;
        let (p0, p1) = (P_HR + 1, P_HR + rows_n);
        super::date_rows(ws, cx, PC + 1, p0, p1)?;
        super::fmt_rows(ws, p0, p1, c_med, c_med + 6, "0.0")?;
        super::fmt_rows(ws, p0, p1, PC + 9, PC + 9, "0%")?;
        theme::cf_data_bar(ws, p0, PC + 9, p1, PC + 9, theme::BAND_OPT)?;
        let m = at(c_med, p0 + 1);
        theme::cf_formula(
            ws,
            p0,
            c_med,
            p1,
            c_med,
            &format!(
                "=AND(ISNUMBER({m}),ISNUMBER({goal}),ISNUMBER({start}),IF({goal}<={start},{m}<={goal},{m}>={goal}))"
            ),
            theme::OPTIMAL,
        )?;

        let series: Vec<(&str, u16)> = ["median", "95% low", "80% low", "50% low", "50% high", "80% high", "95% high"]
            .into_iter()
            .enumerate()
            .map(|(j, s)| (s, PC + 10 + j as u16))
            .collect();
        let chart = line_chart(TITLE, "Fan: random walk with drift (not a forecast)", p0, p1, PC + 1, &series);
        ws.insert_chart(TOP, PC, &chart)?;
        set_widths(ws, &[12.0, 10.0, 10.0, 8.0, 8.0, 8.0, 2.0, 32.0, 12.0, 2.0, 6.0, 12.0])?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
