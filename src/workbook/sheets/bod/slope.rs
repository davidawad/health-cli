//! BOD-07 `Bod_Slope`: rolling OLS slope of the weigh-ins on the day number over the
//! last `Bod_Slope_short` and `Bod_Slope_long` calendar days, in lb/week, with its
//! standard error and t, and an autocorrelation-corrected significance test
//! (KAU Ch6, Ch2; PAL Ch3 FAQ 3.2; quant.md Q25, Q28).
//!
//! Row-aligned with `Bod_Cal` (`k` = 1-based day index). Missing days are masked with
//! zero-filled helpers (`obs` 1/0, `xo` = obs*day_no, `yo` = obs*weight) so the sums over
//! a window W = INDEX(.,k-L+1):INDEX(.,k) count weigh-ins only:
//! * `n` = SUM(obs_W); slope = 7*(n*Sxy - Sx*Sy)/(n*Sxx - Sx^2) with Sxy = SUMPRODUCT(xo_W, yo_W)...;
//! * `se` = 7*SQRT(SSE/(n-2)/Sxx_c), SSE = Syy_c - b^2*Sxx_c (centred sums), `t` = slope/se.
//!
//! Whole series: OLS fit (a, b), residuals, lag-1 autocorrelation of the residuals on
//! consecutive days rho = SUM(e_t e_{t-1}) / SQRT(SUM e_t^2 * SUM e_{t-1}^2) (pairs only),
//! N_eff = n*(1-rho)/(1+rho) (rho floored at 0), t_adj = t*SQRT((1-rho)/(1+rho)), and a
//! verdict vs T.INV.2T(Alpha, N_eff-2).

use rust_xlsxwriter::Worksheet;

use super::signal::{date_chart, series, win};
use super::{
    abs, at, cal_at, cal_head, head, kpi, kv, last_num, n_days, plot_col, rng, section, table, Banner, Col, TOP,
};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Slope";
const HR: u32 = TOP + 1;
const C_W: u16 = 1;
const C_DAY: u16 = 3;
const C_OBS: u16 = 4;
const C_XO: u16 = 5;
const C_YO: u16 = 6;
/// First column of the short-window block (n, slope, se, t); the long block follows.
const C_SHORT: u16 = 7;
const C_LONG: u16 = 11;
const C_RES: u16 = 15;
const C_SPLOT: u16 = 16;
const S_C: u16 = 19;
/// Test table (short / long rows).
const T_HR: u32 = TOP + 6;

/// The window sums of zero-based columns over the `len`-day window ending on day `k`.
struct Win {
    x: String,
    y: String,
}

impl Win {
    fn new(xo: &str, yo: &str, k: u32, len: &str) -> Self {
        Self { x: win(xo, k, len), y: win(yo, k, len) }
    }
    /// Centred sum of products of two windows given the count cell `n`.
    fn c(&self, a: &str, b: &str, n: &str) -> String {
        format!("(SUMPRODUCT({a},{b})-SUM({a})*SUM({b})/{n})")
    }
    fn sxx(&self, n: &str) -> String {
        self.c(&self.x, &self.x, n)
    }
    fn sxy(&self, n: &str) -> String {
        self.c(&self.x, &self.y, n)
    }
    fn syy(&self, n: &str) -> String {
        self.c(&self.y, &self.y, n)
    }
}

pub struct Slope;

impl SheetModule for Slope {
    fn id(&self) -> &'static str {
        "bod-slope"
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
            "Rolling 14/28-day OLS slope with t-stat and N_eff",
            "BOD-07",
            "Q25, Q28 | KAU Ch6 (linear regression slope), Ch2 (autocorrelation); PAL Ch3 FAQ 3.2",
            Banner::Note(
                "Slope of the weigh-ins over the last Bod_Slope_short / Bod_Slope_long days (lb/week). \
                 Daily weights are autocorrelated, so the test uses N_eff, not n.",
            ),
        )?;
        let n = n_days(cx);
        let (r0, r1) = (HR + 1, HR + n);
        let col = |c: u16| rng(TITLE, c, r0, r1);
        let (obs, xo, yo) = (col(C_OBS), col(C_XO), col(C_YO));
        let (a_all, b_all) = (abs(S_C + 1, TOP + 2), abs(S_C + 1, TOP + 1));
        section(ws, cx, TOP, 0, "Daily series (one row per calendar day, Bod_Cal)")?;
        let mut cols = cal_head(HR);
        cols.push(Col::f("day_no", |r| format!("={}", cal_at("day_no", HR, r))));
        cols.push(Col::f("obs", |r| format!("=IF(ISNUMBER({}),1,0)", at(C_W, r))));
        cols.push(Col::f("xo", |r| format!("=IF(ISNUMBER({}),{},0)", at(C_W, r), at(C_DAY, r))));
        cols.push(Col::f("yo", |r| format!("=IF(ISNUMBER({w}),{w},0)", w = at(C_W, r))));
        for (tag, len, c0) in [("short", "Bod_Slope_short", C_SHORT), ("long", "Bod_Slope_long", C_LONG)] {
            let (obs, xo, yo) = (obs.clone(), xo.clone(), yo.clone());
            cols.push(Col::f(if tag == "short" { "n_short" } else { "n_long" }, move |r| {
                let k = r - HR - 1;
                format!("=IF({k}<{len},\"\",SUM({}))", win(&obs, k, len))
            }));
            let (xo2, yo2) = (xo.clone(), yo.clone());
            cols.push(Col::f(if tag == "short" { "slope_short_wk" } else { "slope_long_wk" }, move |r| {
                let (w, nn) = (Win::new(&xo2, &yo2, r - HR - 1, len), at(c0, r));
                format!("=IF(N({nn})>=3,IFERROR(7*{}/{},\"\"),\"\")", w.sxy(&nn), w.sxx(&nn))
            }));
            cols.push(Col::f(if tag == "short" { "se_short" } else { "se_long" }, move |r| {
                let (w, nn, b) = (Win::new(&xo, &yo, r - HR - 1, len), at(c0, r), at(c0 + 1, r));
                format!(
                    "=IF(ISNUMBER({b}),IFERROR(7*SQRT(MAX(0,{syy}-({b}/7)^2*{sxx})/({nn}-2)/{sxx}),\"\"),\"\")",
                    syy = w.syy(&nn),
                    sxx = w.sxx(&nn)
                )
            }));
            cols.push(Col::f(if tag == "short" { "t_short" } else { "t_long" }, move |r| {
                let (b, s) = (at(c0 + 1, r), at(c0 + 2, r));
                format!("=IF(AND(ISNUMBER({b}),ISNUMBER({s})),IF({s}>0,{b}/{s},\"\"),\"\")")
            }));
        }
        cols.push(Col::f("resid0", |r| {
            let (o, y, d) = (at(C_OBS, r), at(C_YO, r), at(C_DAY, r));
            format!("=IF(AND({o}=1,ISNUMBER({b_all})),{y}-({a_all}+{b_all}*{d}),0)")
        }));
        cols.push(plot_col("short_plot", C_SHORT + 1));
        cols.push(plot_col("long_plot", C_LONG + 1));
        table(ws, cx, "tblBodSlope", HR, 0, &cols, n)?;
        super::date_rows(ws, cx, 0, r0, r1)?;
        for c0 in [C_SHORT, C_LONG] {
            super::fmt_rows(ws, r0, r1, c0 + 1, c0 + 2, "+0.00;-0.00;0.00")?;
            super::fmt_rows(ws, r0, r1, c0 + 3, c0 + 3, "+0.0;-0.0;0.0")?;
            let t = at(c0 + 3, r0 + 1);
            theme::cf_formula(ws, r0, c0 + 1, r1, c0 + 1, &format!("=AND(ISNUMBER({t}),{t}<=-2)"), theme::OPTIMAL)?;
            theme::cf_formula(ws, r0, c0 + 1, r1, c0 + 1, &format!("=AND(ISNUMBER({t}),{t}>=2)"), theme::OUT_OF_REF)?;
        }
        super::fmt_rows(ws, r0, r1, C_RES, C_RES, "+0.00;-0.00;0.00")?;

        // Whole-series fit and autocorrelation.
        section(ws, cx, TOP, S_C, "Whole series")?;
        let nn = format!("SUM({obs})");
        kv(
            ws,
            cx,
            TOP + 1,
            S_C,
            "OLS slope (lb/day)",
            &format!(
                "=IF({nn}>=3,IFERROR(({nn}*SUMPRODUCT({xo},{yo})-SUM({xo})*SUM({yo}))/({nn}*SUMPRODUCT({xo},{xo})-SUM({xo})^2),\"\"),\"\")"
            ),
            Some("+0.000;-0.000"),
        )?;
        kv(
            ws,
            cx,
            TOP + 2,
            S_C,
            "OLS intercept (lb at day 0)",
            &format!("=IF(ISNUMBER({b_all}),(SUM({yo})-{b_all}*SUM({xo}))/{nn},\"\")"),
            Some("0.0"),
        )?;
        let rho = if n >= 3 {
            let res = |a: u32, b: u32| rng(TITLE, C_RES, a, b);
            let ob = |a: u32, b: u32| rng(TITLE, C_OBS, a, b);
            let (et, ep, ot, op) = (res(r0 + 1, r1), res(r0, r1 - 1), ob(r0 + 1, r1), ob(r0, r1 - 1));
            format!("=IFERROR(SUMPRODUCT({et},{ep})/SQRT(SUMPRODUCT({op},{et},{et})*SUMPRODUCT({ot},{ep},{ep})),\"\")")
        } else {
            "=\"\"".to_string()
        };
        kv(ws, cx, TOP + 3, S_C, "Residual lag-1 autocorrelation (rho)", &rho, Some("0.00"))?;
        let rc = abs(S_C + 1, TOP + 3);
        let infl = format!("((1-MAX(0,{rc}))/(1+MAX(0,{rc})))");

        // Test table: one row per window.
        let tc = |c: u16, r: u32| at(S_C + c, r);
        let pick = |r: u32, c: u16| {
            let c0 = if r == T_HR + 2 { C_SHORT } else { C_LONG };
            last_num(&rng(TITLE, c0 + c, r0, r1))
        };
        let tcols = vec![
            Col::f("window", |r| if r == T_HR + 2 { "=\"short\"".into() } else { "=\"long\"".into() }),
            Col::f("days", |r| if r == T_HR + 2 { "=Bod_Slope_short".into() } else { "=Bod_Slope_long".into() }),
            Col::f("n", |r| format!("={}", pick(r, 0))),
            Col::f("slope_lb_wk", |r| format!("={}", pick(r, 1))),
            Col::f("t", |r| format!("={}", pick(r, 3))),
            Col::f("n_eff", |r| {
                let nc = tc(2, r);
                format!("=IF(AND(ISNUMBER({nc}),ISNUMBER({rc})),{nc}*{infl},\"\")")
            }),
            Col::f("t_adj", |r| {
                let t = tc(4, r);
                format!("=IF(AND(ISNUMBER({t}),ISNUMBER({rc})),{t}*SQRT({infl}),\"\")")
            }),
            Col::f("t_crit", |r| {
                let ne = tc(5, r);
                format!("=IF(N({ne})>2,T.INV.2T(Alpha,{ne}-2),\"\")")
            }),
            Col::f("verdict", |r| {
                let (t, c) = (tc(6, r), tc(7, r));
                format!(
                    "=IF(AND(ISNUMBER({t}),ISNUMBER({c})),IF(ABS({t})>{c},IF({t}<0,\"falling\",\"rising\"),\"flat (noise)\"),\"too few days\")"
                )
            }),
        ];
        section(ws, cx, T_HR - 1, S_C, "Latest slopes, autocorrelation-adjusted test (Alpha)")?;
        table(ws, cx, "tblBodSlopeTest", T_HR, S_C, &tcols, 2)?;
        super::fmt_rows(ws, T_HR + 1, T_HR + 2, S_C + 3, S_C + 3, "+0.00;-0.00;0.00")?;
        super::fmt_rows(ws, T_HR + 1, T_HR + 2, S_C + 4, S_C + 7, "0.00")?;
        let v = at(S_C + 8, T_HR + 2);
        for (word, fill) in
            [("falling", theme::OPTIMAL), ("rising", theme::OUT_OF_REF), ("flat (noise)", theme::REF_OK)]
        {
            theme::cf_formula(ws, T_HR + 1, S_C + 8, T_HR + 2, S_C + 8, &format!("={v}=\"{word}\""), fill)?;
        }
        kpi(cx, "slope_lb_wk", "Weight slope, long window (lb/week)", TITLE, T_HR + 2, S_C + 3, Some("+0.00;-0.00"))?;
        ws.write_string_with_format(
            T_HR + 4,
            S_C,
            "Green slope cell: t <= -2 (falling); amber: t >= 2. n_eff = n(1-rho)/(1+rho), rho floored at 0.",
            &cx.f.note,
        )?;

        let mut chart = date_chart("Rolling weight slope (lb/week)");
        series(&mut chart, TITLE, "short window", r0, r1, C_SPLOT, false);
        series(&mut chart, TITLE, "long window", r0, r1, C_SPLOT + 1, false);
        ws.insert_chart(T_HR + 6, S_C, &chart)?;
        let mut w = vec![12.0, 10.0, 10.0, 7.0, 5.0, 6.0, 7.0];
        w.extend([
            7.0, 9.0, 8.0, 7.0, 7.0, 9.0, 8.0, 7.0, 8.0, 9.0, 9.0, 2.0, 30.0, 9.0, 7.0, 10.0, 7.0, 7.0, 7.0, 7.0, 14.0,
        ]);
        set_widths(ws, &w)?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
