//! BOD-11 `Bod_Osc`: stochastic "where in my range" oscillator (Q80).
//!
//! `%K = (weight_ff - MIN) / (MAX - MIN) * 100`, with MIN / MAX over the weigh-ins of
//! the trailing `Bod_Osc_days` calendar days (`INDEX:INDEX` windows over the
//! row-aligned `weight_lb` column), `%D` = mean of `%K` over `Bod_Osc_smooth` days.
//! Zones: above `Bod_Osc_hi` = top of the recent range, below `Bod_Osc_lo` = bottom.
//! Blank when the window has no weigh-in or no spread.

use rust_xlsxwriter::Worksheet;

use super::{
    abs, at, cal_head, head, kv, last_num, line_chart, n_days, plot_col, rng, section, table, Banner, Col, TOP,
};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Osc";
const HR: u32 = TOP + 1;
const C_W: u16 = 1;
const C_FF: u16 = 2;
const C_LO: u16 = 3;
const C_HI: u16 = 4;
const C_K: u16 = 5;
const C_D: u16 = 6;
const C_ZONE: u16 = 7;
const C_KPLOT: u16 = 8;
const C_DPLOT: u16 = 9;
const C_L80: u16 = 10;
const C_L20: u16 = 11;
const SC: u16 = 13;

pub struct Osc;

impl SheetModule for Osc {
    fn id(&self) -> &'static str {
        "bod-osc"
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
            "Stochastic where-in-my-range oscillator",
            "BOD-11",
            "Q80 | KAU Ch9 (stochastic %K / %D)",
            Banner::Note(
                "%K: where today's weight sits in the min-max of the last Bod_Osc_days days (0 = low, 100 = high).",
            ),
        )?;
        let n = n_days(cx);
        let first = HR + 2;
        let (r0, r1) = (HR + 1, HR + n);
        let col = |c: u16| rng(TITLE, c, r0, r1);
        let (wcol, kcol) = (col(C_W), col(C_K));

        section(ws, cx, TOP, 0, "Oscillator (one row per calendar day, Bod_Cal)")?;
        let win = move |src: &str, r: u32, days: &str| {
            let i = r - first + 1;
            format!("INDEX({src},MAX(1,{i}-{days}+1)):INDEX({src},{i})")
        };
        let mut cols = cal_head(HR);
        let (w1, w2, k1) = (wcol.clone(), wcol.clone(), kcol.clone());
        cols.push(Col::f("range_lo", move |r| {
            let x = win(&w1, r, "Bod_Osc_days");
            format!("=IF(COUNT({x})>0,MIN({x}),\"\")")
        }));
        cols.push(Col::f("range_hi", move |r| {
            let x = win(&w2, r, "Bod_Osc_days");
            format!("=IF(COUNT({x})>0,MAX({x}),\"\")")
        }));
        cols.push(Col::f("pct_k", |r| {
            let (f, lo, hi) = (at(C_FF, r), at(C_LO, r), at(C_HI, r));
            format!("=IF(AND(ISNUMBER({f}),ISNUMBER({lo}),ISNUMBER({hi})),IF({hi}>{lo},({f}-{lo})/({hi}-{lo})*100,\"\"),\"\")")
        }));
        cols.push(Col::f("pct_d", move |r| {
            let x = win(&k1, r, "Bod_Osc_smooth");
            format!("=IF(COUNT({x})>0,AVERAGE({x}),\"\")")
        }));
        cols.push(Col::f("zone", |r| {
            let k = at(C_K, r);
            format!("=IF(ISNUMBER({k}),IF({k}>Bod_Osc_hi,\"top\",IF({k}<Bod_Osc_lo,\"bottom\",\"mid\")),\"\")")
        }));
        cols.push(plot_col("k_plot", C_K));
        cols.push(plot_col("d_plot", C_D));
        cols.push(Col::f("line_hi", |_| "=Bod_Osc_hi".into()));
        cols.push(Col::f("line_lo", |_| "=Bod_Osc_lo".into()));
        table(ws, cx, "tblBodOsc", HR, 0, &cols, n)?;
        super::date_rows(ws, cx, 0, r0, r1)?;
        super::fmt_rows(ws, r0, r1, C_LO, C_HI, "0.0")?;
        super::fmt_rows(ws, r0, r1, C_K, C_D, "0")?;
        let k = at(C_K, r0 + 1);
        theme::cf_formula(ws, r0, C_K, r1, C_K, &format!("=AND(ISNUMBER({k}),{k}>Bod_Osc_hi)"), theme::OUT_OF_REF)?;
        theme::cf_formula(ws, r0, C_K, r1, C_K, &format!("=AND(ISNUMBER({k}),{k}<Bod_Osc_lo)"), theme::OPTIMAL)?;
        let z = at(C_ZONE, r0 + 1);
        theme::cf_formula(ws, r0, C_ZONE, r1, C_ZONE, &format!("={z}=\"top\""), theme::OUT_OF_REF)?;
        theme::cf_formula(ws, r0, C_ZONE, r1, C_ZONE, &format!("={z}=\"bottom\""), theme::OPTIMAL)?;

        // Summary.
        section(ws, cx, TOP, SC, "Summary")?;
        let zones = col(C_ZONE);
        let nk = format!("COUNT({kcol})");
        let rows: [(&str, String, &str); 7] = [
            ("%K today", format!("={}", last_num(&kcol)), "0"),
            ("%D today", format!("={}", last_num(&col(C_D))), "0"),
            ("Range low (lb)", format!("={}", last_num(&col(C_LO))), "0.0"),
            ("Range high (lb)", format!("={}", last_num(&col(C_HI))), "0.0"),
            ("Days with %K", format!("={nk}"), "0"),
            ("Share of days at the top", format!("=IF({nk}>0,COUNTIF({zones},\"top\")/{nk},\"\")"), "0%"),
            ("Share of days at the bottom", format!("=IF({nk}>0,COUNTIF({zones},\"bottom\")/{nk},\"\")"), "0%"),
        ];
        for (i, (label, f, nf)) in rows.iter().enumerate() {
            kv(ws, cx, TOP + 1 + i as u32, SC, label, f, Some(nf))?;
        }
        let kt = abs(SC + 1, TOP + 1);
        theme::cf_formula(
            ws,
            TOP + 1,
            SC + 1,
            TOP + 1,
            SC + 1,
            &format!("=AND(ISNUMBER({kt}),{kt}>Bod_Osc_hi)"),
            theme::OUT_OF_REF,
        )?;
        theme::cf_formula(
            ws,
            TOP + 1,
            SC + 1,
            TOP + 1,
            SC + 1,
            &format!("=AND(ISNUMBER({kt}),{kt}<Bod_Osc_lo)"),
            theme::OPTIMAL,
        )?;
        ws.write_string_with_format(
            TOP + 9,
            SC,
            "Top of the range on a cut = a bump to wait out, not a verdict; bottom = new low of the window.",
            &cx.f.note,
        )?;

        let mut chart = line_chart(
            TITLE,
            "Stochastic oscillator %K / %D",
            r0,
            r1,
            0,
            &[("%K", C_KPLOT), ("%D", C_DPLOT), ("top line", C_L80), ("bottom line", C_L20)],
        );
        chart.y_axis().set_min(0).set_max(100);
        ws.insert_chart(TOP + 11, SC, &chart)?;
        set_widths(ws, &[12.0, 10.0, 10.0, 9.0, 9.0, 7.0, 7.0, 8.0, 8.0, 8.0, 8.0, 8.0, 2.0, 28.0, 10.0])?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
