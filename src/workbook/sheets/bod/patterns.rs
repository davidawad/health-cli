//! BOD-09 `Bod_Patterns`: shock days, weekday patterns and cycle detection in the
//! daily weight change (Q27, Q29, Q81).
//!
//! * Shock days: `|dw_day|` (lb/day since the previous weigh-in, `Bod_Cal`) above
//!   `Bod_Shock_x` times its mean; a running counter numbers the shocks and an
//!   `INDEX/MATCH` list shows their dates.
//! * Weekday table (Mon..Sun): `AVERAGEIFS` of `dw_day` by `Bod_Cal` weekday, the count
//!   and a conditional SD (`SUMPRODUCT`), plus mean kcal and protein by weekday.
//! * Autocorrelation of `dw_day` (centred, 0 on days without a change) for lags
//!   1..35 over Rust-generated shifted ranges:
//!   `r_k = SUMPRODUCT(x[1+k..n], x[1..n-k]) / SUMSQ(x)`, bounds `+-z(1-Alpha/2)/SQRT(n)`.
//!   The cycle is the first significant positive local peak (a weekly rhythm shows as
//!   lag 7, with echoes at 14, 21, ...); the strongest lag is flagged too.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{abs, at, cal_at, cal_col, cal_head, head, kv, n_days, plot_col, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Patterns";
const HR: u32 = TOP + 1;
const C_DATE: u16 = 0;
const C_DW: u16 = 3;
const C_WD: u16 = 4;
const C_ABS: u16 = 5;
const C_SHOCK: u16 = 6;
const C_SN: u16 = 7;
const C_X: u16 = 8;
const C_DW0: u16 = 9;
const C_ISN: u16 = 10;
/// Summary block (labels; values one column right).
const SC: u16 = 12;
const SH_HR: u32 = TOP + 12;
const SHOCKS: u32 = 20;
const WD_HR: u32 = SH_HR + SHOCKS + 3;
/// Autocorrelation table.
const AC: u16 = 21;
const LAGS: u32 = 35;
const CH: u16 = 30;
const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

pub struct Patterns;

impl SheetModule for Patterns {
    fn id(&self) -> &'static str {
        "bod-patterns"
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
            "Shock days, weekday means, autocorrelation lags 1-35",
            "BOD-09",
            "Q27, Q29, Q81 | KAU Ch22 (shocks), Ch15 (weekday), Ch10-11 (autocorrelation)",
            Banner::Note(
                "dw_day = lb/day since the previous weigh-in (Bod_Cal). Shock: |dw_day| > Bod_Shock_x x mean.",
            ),
        )?;
        let n = n_days(cx);
        let first = HR + 2;
        let (r0, r1) = (HR + 1, HR + n);
        let col = |c: u16| rng(TITLE, c, r0, r1);
        let v = |k: u32| abs(SC + 1, TOP + k);
        let (mean_abs, thr, n_shock, n_dw, m, bound, cycle) = (v(1), v(2), v(3), v(4), v(5), v(6), v(7));

        section(ws, cx, TOP, 0, "Daily change (one row per calendar day, Bod_Cal)")?;
        let mut cols = cal_head(HR);
        cols.push(Col::f("dw_day", move |r| format!("={}", cal_at("dw_day", HR, r))));
        cols.push(Col::f("weekday", move |r| format!("={}", cal_at("weekday", HR, r))));
        cols.push(Col::f("abs_dw", |r| format!("=IF(ISNUMBER({d}),ABS({d}),\"\")", d = at(C_DW, r))));
        let t = thr.clone();
        cols.push(Col::f("shock", move |r| {
            let a = at(C_ABS, r);
            format!("=IF(ISNUMBER({a}),IF(AND(ISNUMBER({t}),{a}>{t}),1,0),\"\")")
        }));
        cols.push(Col::f("shock_n", move |r| {
            let s = format!("IF({}=1,1,0)", at(C_SHOCK, r));
            if r == first {
                format!("={s}")
            } else {
                format!("={}+{s}", at(C_SN, r - 1))
            }
        }));
        let mm = m.clone();
        cols.push(Col::f("acf_x", move |r| {
            let d = at(C_DW, r);
            format!("=IF(AND(ISNUMBER({d}),ISNUMBER({mm})),{d}-{mm},0)")
        }));
        cols.push(Col::f("dw0", |r| format!("=IF(ISNUMBER({d}),{d},0)", d = at(C_DW, r))));
        cols.push(Col::f("has_dw", |r| format!("=IF(ISNUMBER({}),1,0)", at(C_DW, r))));
        table(ws, cx, "tblBodPatterns", HR, 0, &cols, n)?;
        super::date_rows(ws, cx, C_DATE, r0, r1)?;
        super::fmt_rows(ws, r0, r1, C_DW, C_DW, "+0.00;-0.00;0.00")?;
        super::fmt_rows(ws, r0, r1, C_ABS, C_ABS, "0.00")?;
        super::fmt_rows(ws, r0, r1, C_X, C_X, "+0.000;-0.000;0.000")?;
        let tl = at(C_SHOCK, r0 + 1);
        theme::cf_formula(ws, r0, C_SHOCK, r1, C_SHOCK, &format!("={tl}=1"), theme::CRITICAL)?;
        theme::cf_data_bar(ws, r0, C_ABS, r1, C_ABS, theme::BAND_BLUEPRINT)?;

        // Summary.
        let (dwr, lag_r, r_r, peak_r) = (
            col(C_DW),
            rng(TITLE, AC, HR + 1, HR + LAGS),
            rng(TITLE, AC + 1, HR + 1, HR + LAGS),
            rng(TITLE, AC + 5, HR + 1, HR + LAGS),
        );
        section(ws, cx, TOP, SC, "Summary")?;
        let rows: [(&str, String, &str); 9] = [
            (
                "Mean |daily change| (lb/day)",
                format!("=IF(COUNT({dwr})>0,SUMPRODUCT(ABS({}))/COUNT({dwr}),\"\")", col(C_DW0)),
                "0.000",
            ),
            ("Shock threshold (lb/day)", format!("=IF(ISNUMBER({mean_abs}),Bod_Shock_x*{mean_abs},\"\")"), "0.000"),
            ("Shock days", format!("=SUM({})", col(C_SHOCK)), "0"),
            ("Days with a daily change", format!("=COUNT({dwr})"), "0"),
            ("Mean daily change (lb/day)", format!("=IFERROR(AVERAGE({dwr}),\"\")"), "+0.000;-0.000"),
            ("ACF bound (+/-)", format!("=IF({n_dw}>0,NORM.S.INV(1-Alpha/2)/SQRT({n_dw}),\"\")"), "0.000"),
            (
                "Cycle: first significant peak (days)",
                format!("=IFERROR(INDEX({lag_r},MATCH(1,{peak_r},0)),\"none\")"),
                "0",
            ),
            (
                "Strongest lag (days)",
                format!("=IF(COUNT({r_r})=0,\"\",IFERROR(INDEX({lag_r},MATCH(MAX({r_r}),{r_r},0)),\"\"))"),
                "0",
            ),
            ("r at the strongest lag", format!("=IF(COUNT({r_r})=0,\"\",MAX({r_r}))"), "0.000"),
        ];
        for (i, (label, f, nf)) in rows.iter().enumerate() {
            kv(ws, cx, TOP + 1 + i as u32, SC, label, f, Some(nf))?;
        }

        // Shock list: the k-th shock is the first row where the running counter reaches k.
        section(ws, cx, SH_HR - 1, SC, "Shock days (first 20)")?;
        let (dates, sn, wl) = (col(C_DATE), col(C_SN), col(1));
        let shock_cols = vec![
            Col::f("shock_no", |r| format!("={}", r - SH_HR - 1)),
            Col::f("shock_date", |r| {
                let k = at(SC, r);
                format!("=IF({k}<={n_shock},IFERROR(INDEX({dates},MATCH({k},{sn},0)),\"\"),\"\")")
            }),
            Col::f("dw_day", |r| {
                let k = at(SC, r);
                format!("=IF(ISNUMBER({}),INDEX({dwr},MATCH({k},{sn},0)),\"\")", at(SC + 1, r))
            }),
            Col::f("weight_lb", |r| {
                let k = at(SC, r);
                format!("=IF(ISNUMBER({}),INDEX({wl},MATCH({k},{sn},0)),\"\")", at(SC + 1, r))
            }),
        ];
        table(ws, cx, "tblBodShocks", SH_HR, SC, &shock_cols, SHOCKS)?;
        super::date_rows(ws, cx, SC + 1, SH_HR + 1, SH_HR + SHOCKS)?;
        super::fmt_rows(ws, SH_HR + 1, SH_HR + SHOCKS, SC + 2, SC + 2, "+0.00;-0.00;0.00")?;

        // Weekday table.
        section(ws, cx, WD_HR - 1, SC, "Weekday pattern (Bod_Cal weekday, 1 = Monday)")?;
        let (wd, isn, dw0) = (col(C_WD), col(C_ISN), col(C_DW0));
        let (kcal, prot, cwd) = (cal_col(cx, "kcal"), cal_col(cx, "protein_g"), cal_col(cx, "weekday"));
        let wd_cols = vec![
            Col::f("day_of_week", |r| format!("=\"{}\"", DAYS[(r - WD_HR - 2) as usize])),
            Col::f("wd", |r| format!("={}", r - WD_HR - 1)),
            Col::f("n", |r| format!("=SUMPRODUCT(({wd}={})*{isn})", at(SC + 1, r))),
            Col::f("mean_dw", |r| {
                format!("=IF({}>0,IFERROR(AVERAGEIFS({dwr},{wd},{}),\"\"),\"\")", at(SC + 2, r), at(SC + 1, r))
            }),
            Col::f("sd_dw", |r| {
                let (k, c, mu) = (at(SC + 1, r), at(SC + 2, r), at(SC + 3, r));
                format!("=IF(AND({c}>1,ISNUMBER({mu})),SQRT(SUMPRODUCT(({wd}={k})*{isn}*({dw0}-{mu})^2)/({c}-1)),\"\")")
            }),
            Col::f("mean_kcal", |r| format!("=IFERROR(AVERAGEIFS({kcal},{cwd},{}),\"\")", at(SC + 1, r))),
            Col::f("mean_protein_g", |r| format!("=IFERROR(AVERAGEIFS({prot},{cwd},{}),\"\")", at(SC + 1, r))),
            plot_col("mean_dw_plot", SC + 3),
        ];
        table(ws, cx, "tblBodWeekday", WD_HR, SC, &wd_cols, 7)?;
        let (w0, w1) = (WD_HR + 1, WD_HR + 7);
        super::fmt_rows(ws, w0, w1, SC + 3, SC + 4, "+0.000;-0.000;0.000")?;
        super::fmt_rows(ws, w0, w1, SC + 5, SC + 6, "0")?;
        theme::cf_three_color(ws, w0, SC + 3, w1, SC + 3, true)?;
        theme::cf_data_bar(ws, w0, SC + 5, w1, SC + 5, theme::BAND_REF)?;

        // Autocorrelation table (lags 1..LAGS), ranges generated here from the row count.
        section(ws, cx, TOP, AC, "Autocorrelation of dw_day (cycle detection)")?;
        let x_all = col(C_X);
        let acf_cols = vec![
            Col::f("lag", |r| format!("={}", r - HR - 1)),
            Col::f("r", |r| {
                let k = r - HR - 1;
                if n < k + 3 {
                    return "=\"\"".into();
                }
                let (a, b) = (rng(TITLE, C_X, r0 + k, r1), rng(TITLE, C_X, r0, r1 - k));
                format!("=IF({n_dw}-{k}<3,\"\",IFERROR(SUMPRODUCT({a},{b})/SUMSQ({x_all}),\"\"))")
            }),
            Col::f("lo", |_| format!("=IF(ISNUMBER({bound}),-{bound},\"\")")),
            Col::f("hi", |_| format!("=IF(ISNUMBER({bound}),{bound},\"\")")),
            Col::f("significant", |r| {
                let x = at(AC + 1, r);
                format!("=IF(AND(ISNUMBER({x}),ISNUMBER({h})),IF(ABS({x})>{h},1,0),\"\")", h = at(AC + 3, r))
            }),
            Col::f("peak", |r| {
                if r == HR + 2 {
                    return "=0".into();
                }
                let (x, p, s) = (at(AC + 1, r), at(AC + 1, r - 1), at(AC + 4, r));
                let next = if r == HR + 1 + LAGS {
                    "TRUE".to_string()
                } else {
                    let q = at(AC + 1, r + 1);
                    format!("IF(ISNUMBER({q}),{x}>={q},TRUE)")
                };
                format!("=IF(AND(ISNUMBER({x}),ISNUMBER({p})),IF(AND({s}=1,{x}>0,{x}>{p},{next}),1,0),0)")
            }),
            Col::f("flag", |r| {
                let (l, x) = (at(AC, r), at(AC + 1, r));
                format!(
                    "=TRIM(IF({l}={cycle},\"cycle \",\"\")&IF(AND(ISNUMBER({x}),COUNT({r_r})>0),IF(AND({x}=MAX({r_r}),{x}>0),\"strongest\",\"\"),\"\"))"
                )
            }),
            plot_col("r_plot", AC + 1),
        ];
        table(ws, cx, "tblBodACF", HR, AC, &acf_cols, LAGS)?;
        let (a0, a1) = (HR + 1, HR + LAGS);
        super::fmt_rows(ws, a0, a1, AC + 1, AC + 3, "+0.000;-0.000;0.000")?;
        let s = at(AC + 4, a0 + 1);
        theme::cf_formula(ws, a0, AC + 1, a1, AC + 1, &format!("={s}=1"), theme::OPTIMAL)?;
        let fl = at(AC + 6, a0 + 1);
        theme::cf_formula(ws, a0, AC + 6, a1, AC + 6, &format!("=LEN({fl})>0"), theme::REF_OK)?;

        // Charts: weekday means and the correlogram with its significance bounds.
        let mut wchart = Chart::new(ChartType::Column);
        wchart.add_series().set_name("mean dw_day (lb/day)").set_categories((TITLE, w0, SC, w1, SC)).set_values((
            TITLE,
            w0,
            SC + 7,
            w1,
            SC + 7,
        ));
        wchart.title().set_name("Mean daily change by weekday");
        wchart.show_na_as_empty_cell();
        wchart.set_width(560).set_height(280);
        ws.insert_chart(TOP, CH, &wchart)?;
        let mut acf = Chart::new(ChartType::Column);
        acf.add_series().set_name("r (autocorrelation)").set_categories((TITLE, a0, AC, a1, AC)).set_values((
            TITLE,
            a0,
            AC + 7,
            a1,
            AC + 7,
        ));
        let mut bounds = Chart::new(ChartType::Line);
        for (name, c) in [("-bound", AC + 2), ("+bound", AC + 3)] {
            bounds
                .add_series()
                .set_name(name)
                .set_categories((TITLE, a0, AC, a1, AC))
                .set_values((TITLE, a0, c, a1, c));
        }
        acf.combine(&bounds);
        acf.title().set_name("Autocorrelation of daily change, lags 1-35 days");
        acf.show_na_as_empty_cell();
        acf.set_width(560).set_height(300);
        ws.insert_chart(TOP + 16, CH, &acf)?;

        set_widths(ws, &[12.0, 10.0, 10.0, 9.0, 8.0, 8.0, 7.0, 8.0, 9.0, 8.0, 7.0, 2.0, 30.0, 11.0, 9.0, 9.0])?;
        ws.set_column_width(AC + 6, 16)?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
