//! BOD-10 `Bod_Revert`: mean-reversion half-life of body weight (Q30).
//!
//! Each weigh-in's deviation from the trailing `Bod_Revert_ma`-day mean of the
//! weigh-ins (`dev`) is regressed on the previous day's deviation (AR(1)) over the
//! last `Bod_Revert_days` calendar days, using only pairs of consecutive weighed days:
//! `b = SUMPRODUCT(w,(x-mx),(y-my)) / SUMPRODUCT(w,(x-mx)^2)` with a 0/1 fit mask `w`,
//! intercept `a`, R^2, long-run mean `mu = a/(1-b)` and half-life `LN(2)/-LN(b)` days when
//! `0 < b < 1` ("no reversion" otherwise). A short half-life means bumps are water
//! noise; a long one, a real change. The deviation is charted with a +-1 SD band.

use rust_xlsxwriter::Worksheet;

use super::{abs, at, cal_head, head, kv, line_chart, n_days, plot_col, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Revert";
const HR: u32 = TOP + 1;
const C_W: u16 = 1;
const C_MA: u16 = 3;
const C_DEV: u16 = 4;
const C_PREV: u16 = 5;
const C_FIT: u16 = 6;
const C_X: u16 = 7;
const C_Y: u16 = 8;
const C_DPLOT: u16 = 9;
const C_HI: u16 = 10;
const C_LO: u16 = 11;
const SC: u16 = 13;

pub struct Revert;

impl SheetModule for Revert {
    fn id(&self) -> &'static str {
        "bod-revert"
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
            "AR(1) mean-reversion half-life",
            "BOD-10",
            "Q30 | CHAN Ch7 (half-life = ln2/-ln b); KAU Ch6",
            Banner::Note(
                "dev = weigh-in minus its trailing Bod_Revert_ma-day mean; AR(1) fit over the last Bod_Revert_days days.",
            ),
        )?;
        let n = n_days(cx);
        let first = HR + 2;
        let (r0, r1) = (HR + 1, HR + n);
        let col = |c: u16| rng(TITLE, c, r0, r1);
        let v = |k: u32| abs(SC + 1, TOP + k);
        let (nw, mx, my, b, a, sd) = (v(1), v(2), v(3), v(4), v(5), v(9));
        let wcol = col(C_W);

        section(ws, cx, TOP, 0, "Deviation from the trailing mean (one row per calendar day, Bod_Cal)")?;
        let mut cols = cal_head(HR);
        let wc = wcol.clone();
        cols.push(Col::f("trail_mean", move |r| {
            let i = r - first + 1;
            let win = format!("INDEX({wc},MAX(1,{i}-Bod_Revert_ma+1)):INDEX({wc},{i})");
            format!("=IF(COUNT({win})>0,AVERAGE({win}),\"\")")
        }));
        cols.push(Col::f("dev", |r| {
            let (w, m) = (at(C_W, r), at(C_MA, r));
            format!("=IF(AND(ISNUMBER({w}),ISNUMBER({m})),{w}-{m},\"\")")
        }));
        cols.push(Col::f("dev_prev", move |r| {
            if r == first {
                "=\"\"".into()
            } else {
                let p = at(C_DEV, r - 1);
                format!("=IF(ISNUMBER({p}),{p},\"\")")
            }
        }));
        cols.push(Col::f("in_fit", move |r| {
            let i = r - first + 1;
            let (d, p) = (at(C_DEV, r), at(C_PREV, r));
            format!("=IF(AND(ISNUMBER({d}),ISNUMBER({p}),{i}>{n}-Bod_Revert_days),1,0)")
        }));
        cols.push(Col::f("x0", |r| format!("=IF({}=1,{},0)", at(C_FIT, r), at(C_PREV, r))));
        cols.push(Col::f("y0", |r| format!("=IF({}=1,{},0)", at(C_FIT, r), at(C_DEV, r))));
        cols.push(plot_col("dev_plot", C_DEV));
        let s = sd.clone();
        cols.push(Col::f("plus_sd", move |_| format!("=IF(ISNUMBER({s}),{s},NA())")));
        let s = sd.clone();
        cols.push(Col::f("minus_sd", move |_| format!("=IF(ISNUMBER({s}),-{s},NA())")));
        table(ws, cx, "tblBodRevert", HR, 0, &cols, n)?;
        super::date_rows(ws, cx, 0, r0, r1)?;
        super::fmt_rows(ws, r0, r1, C_MA, C_MA, "0.0")?;
        super::fmt_rows(ws, r0, r1, C_DEV, C_PREV, "+0.00;-0.00;0.00")?;
        let d = at(C_DEV, r0 + 1);
        theme::cf_formula(
            ws,
            r0,
            C_DEV,
            r1,
            C_DEV,
            &format!("=AND(ISNUMBER({d}),ISNUMBER({sd}),ABS({d})>{sd})"),
            theme::OUT_OF_REF,
        )?;
        let f = at(C_FIT, r0 + 1);
        theme::cf_formula(ws, r0, C_FIT, r1, C_FIT, &format!("={f}=1"), theme::BAND_REF)?;

        // AR(1) fit.
        let (w, x, y) = (col(C_FIT), col(C_X), col(C_Y));
        section(ws, cx, TOP, SC, "AR(1): dev(t) = a + b * dev(t-1)")?;
        let sxy = format!("SUMPRODUCT({w},({x}-{mx}),({y}-{my}))");
        let sxx = format!("SUMPRODUCT({w},({x}-{mx})^2)");
        let syy = format!("SUMPRODUCT({w},({y}-{my})^2)");
        let rows: [(&str, String, &str); 9] = [
            ("Pairs in the fit", format!("=SUM({w})"), "0"),
            ("Mean dev(t-1) (lb)", format!("=IF({nw}>0,SUM({x})/{nw},\"\")"), "+0.000;-0.000"),
            ("Mean dev(t) (lb)", format!("=IF({nw}>0,SUM({y})/{nw},\"\")"), "+0.000;-0.000"),
            ("b (AR(1) coefficient)", format!("=IF({nw}>=3,IFERROR({sxy}/{sxx},\"\"),\"\")"), "0.000"),
            ("a (intercept, lb)", format!("=IF(ISNUMBER({b}),{my}-{b}*{mx},\"\")"), "+0.000;-0.000"),
            ("R^2", format!("=IF(ISNUMBER({b}),IFERROR({sxy}^2/({sxx}*{syy}),\"\"),\"\")"), "0.000"),
            (
                "Long-run mean mu = a/(1-b) (lb)",
                format!("=IF(ISNUMBER({b}),IF({b}<>1,{a}/(1-{b}),\"\"),\"\")"),
                "+0.00;-0.00",
            ),
            (
                "Half-life (days)",
                format!("=IF(ISNUMBER({b}),IF(AND({b}>0,{b}<1),LN(2)/-LN({b}),\"no reversion\"),\"\")"),
                "0.0",
            ),
            ("SD of dev in the fit (lb)", format!("=IF({nw}>=2,SQRT({syy}/({nw}-1)),\"\")"), "0.00"),
        ];
        for (i, (label, f, nf)) in rows.iter().enumerate() {
            kv(ws, cx, TOP + 1 + i as u32, SC, label, f, Some(nf))?;
        }
        ws.write_string_with_format(
            TOP + 11,
            SC,
            "b near 0: a bump is gone next day (water). b near 1: excursions persist (a real change).",
            &cx.f.note,
        )?;
        let hl = (TOP + 8, SC + 1);
        let hc = abs(hl.1, hl.0);
        theme::cf_formula(ws, hl.0, hl.1, hl.0, hl.1, &format!("=AND(ISNUMBER({hc}),{hc}<=7)"), theme::OPTIMAL)?;
        theme::cf_formula(ws, hl.0, hl.1, hl.0, hl.1, &format!("=AND(ISNUMBER({hc}),{hc}>7)"), theme::REF_OK)?;

        let mut chart = line_chart(
            TITLE,
            "Deviation from the trailing mean with +-1 SD",
            r0,
            r1,
            0,
            &[("dev (lb)", C_DPLOT), ("+1 SD", C_HI), ("-1 SD", C_LO)],
        );
        chart.set_width(680);
        ws.insert_chart(TOP + 13, SC, &chart)?;
        set_widths(ws, &[12.0, 10.0, 10.0, 10.0, 8.0, 9.0, 7.0, 8.0, 8.0, 9.0, 8.0, 8.0, 2.0, 30.0, 12.0])?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
