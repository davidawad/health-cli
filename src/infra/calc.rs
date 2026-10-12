//! The `Calc_*` layer: formulas only, no per-workstream logic. Views read
//! these instead of recomputing statistics.
//!
//! * `Calc_Labs`: one row per `tblLabs` row (same order): marker row lookup, the
//!   marker's bounds, in-reference / in-optimal flags and a status code.
//! * `Calc_Marker`: one row per `tblMarkers` row (same order): n, first / last
//!   date, last, previous, mean, sd, median, min, max, slope per year, direction
//!   sign, status code of the last draw, z of the last value, distance outside the
//!   range, share of draws in optimal / in reference, days since the last draw.
//! * `Calc_Panel`: one row per `tblPanels` row: n measured, composite score.
//!
//! Status code: 0 optimal, 1 reference-ok but not optimal, 2 out of reference,
//! 3 beyond critical (only when `critical_lo/hi` are filled in `tblMarkers`).
//! A marker with no optimal range counts as optimal when inside its reference.
//! Assumes at most one value per marker and day.
//!
//! Defined names set here: `N_Panels`, `Latest_Panel`, `Prev_Panel`.

use rust_xlsxwriter::{Formula, Worksheet};

use super::data::{LABS, MARKERS, PANELS};
use super::names::range_ref;
use crate::error::Result;
use crate::workbook::{set_widths, Cx, Data, SheetModule};

pub const LABS_TITLE: &str = "Calc_Labs";
pub const MARKER_TITLE: &str = "Calc_Marker";
pub const PANEL_TITLE: &str = "Calc_Panel";

pub const LABS_HEADERS: &[&str] =
    &["panel_id", "marker_id", "value", "mrow", "ref_lo", "ref_hi", "opt_lo", "opt_hi", "in_ref", "in_opt", "status"];
pub const MARKER_HEADERS: &[&str] = &[
    "marker_id",
    "n",
    "first_date",
    "last_date",
    "last",
    "previous",
    "mean",
    "sd",
    "median",
    "min",
    "max",
    "slope_per_year",
    "dir_sign",
    "status",
    "z_latest",
    "distance",
    "pct_in_opt",
    "pct_in_ref",
    "days_since_last",
];
pub const PANEL_HEADERS: &[&str] = &["panel_id", "date", "age", "n_measured", "pct_in_ref", "pct_in_opt"];

fn header(ws: &mut Worksheet, headers: &[&str], cx: &Cx) -> Result<()> {
    headers
        .iter()
        .enumerate()
        .try_for_each(|(i, h)| ws.write_string_with_format(0, i as u16, *h, &cx.f.header).map(|_| ()))?;
    ws.set_freeze_panes(1, 1)?;
    Ok(())
}

fn need_labs(d: &Data) -> bool {
    d.labs.is_some()
}

pub struct CalcLabs;

impl SheetModule for CalcLabs {
    fn id(&self) -> &'static str {
        "calc-labs"
    }
    fn title(&self) -> &'static str {
        LABS_TITLE
    }
    fn available(&self, d: &Data) -> bool {
        need_labs(d)
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let d = cx.d;
        header(ws, LABS_HEADERS, cx)?;
        let (nl, nm) = (LABS.nrows(d), MARKERS.nrows(d));
        let mid = MARKERS.range_n(nm, "marker_id");
        let col = |h: &str| MARKERS.range_n(nm, h);
        let pick = |h: &str, r: u32| {
            let rg = col(h);
            format!("=IFERROR(IF(ISNUMBER(INDEX({rg},$D{r})),INDEX({rg},$D{r}),\"\"),\"\")")
        };
        let rows = (LABS.rows)(d).len() as u32;
        for i in 0..rows {
            let (r, src) = (i + 2, i + 2);
            let t = LABS.sheet;
            ws.write_formula(i + 1, 0, Formula::new(format!("=IF({t}!B{src}=\"\",\"\",{t}!B{src})")))?;
            ws.write_formula(i + 1, 1, Formula::new(format!("={t}!C{src}")))?;
            ws.write_formula(i + 1, 2, Formula::new(format!("={t}!F{src}")))?;
            ws.write_formula(i + 1, 3, Formula::new(format!("=IFERROR(MATCH($B{r},{mid},0),\"\")")))?;
            for (c, h) in [(4u16, "ref_lo"), (5, "ref_hi"), (6, "opt_lo"), (7, "opt_hi")] {
                ws.write_formula(i + 1, c, Formula::new(pick(h, r)))?;
            }
            // In reference / optimal: blank when the marker has no such bounds.
            for (c, lo, hi) in [(8u16, "E", "F"), (9, "G", "H")] {
                ws.write_formula(
                    i + 1,
                    c,
                    Formula::new(format!(
                        "=IF(OR(NOT(ISNUMBER($C{r})),COUNT({lo}{r}:{hi}{r})=0),\"\",IF(AND(OR(NOT(ISNUMBER({lo}{r})),$C{r}>={lo}{r}),OR(NOT(ISNUMBER({hi}{r})),$C{r}<={hi}{r})),1,0))"
                    )),
                )?;
            }
            let (cl, ch) = (col("critical_lo"), col("critical_hi"));
            ws.write_formula(
                i + 1,
                10,
                Formula::new(format!(
                    "=IF(NOT(ISNUMBER($C{r})),\"\",IF(OR(AND(ISNUMBER(INDEX({cl},$D{r})),$C{r}<INDEX({cl},$D{r})),AND(ISNUMBER(INDEX({ch},$D{r})),$C{r}>INDEX({ch},$D{r}))),3,IF(I{r}=0,2,IF(J{r}=0,1,IF(OR(J{r}=1,I{r}=1),0,\"\")))))"
                )),
            )?;
        }
        let _ = nl;
        set_widths(ws, &[12.0, 24.0, 10.0, 6.0, 9.0, 9.0, 9.0, 9.0, 8.0, 8.0, 8.0])?;
        Ok(())
    }
}

pub struct CalcMarker;

impl SheetModule for CalcMarker {
    fn id(&self) -> &'static str {
        "calc-marker"
    }
    fn title(&self) -> &'static str {
        MARKER_TITLE
    }
    fn available(&self, d: &Data) -> bool {
        need_labs(d)
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let d = cx.d;
        header(ws, MARKER_HEADERS, cx)?;
        let nl = LABS.nrows(d);
        let (lm, ld, lv) = (LABS.range_n(nl, "marker_id"), LABS.range_n(nl, "panel_date"), LABS.range_n(nl, "value"));
        let cr = |c: &str| crate::workbook::range(LABS_TITLE, labs_col(c), 2, 1 + nl);
        let (c_inref, c_inopt, c_status) = (cr("in_ref"), cr("in_opt"), cr("status"));
        let mk = |h: &str, r: u32| format!("{}!${}${r}", MARKERS.sheet, crate::workbook::col(MARKERS.col(h)));
        let rows = (MARKERS.rows)(d).len() as u32;
        for i in 0..rows {
            let r = i + 2;
            let a = format!("$A{r}");
            let t = MARKERS.sheet;
            ws.write_formula(i + 1, 0, Formula::new(format!("={t}!A{r}")))?;
            ws.write_formula(i + 1, 1, Formula::new(format!("=COUNTIFS({lm},{a})")))?;
            ws.write_formula(i + 1, 2, Formula::new(format!("=IF($B{r}=0,\"\",MINIFS({ld},{lm},{a}))")))?;
            ws.write_formula(i + 1, 3, Formula::new(format!("=IF($B{r}=0,\"\",MAXIFS({ld},{lm},{a}))")))?;
            ws.write_formula(i + 1, 4, Formula::new(format!("=IF($B{r}=0,\"\",SUMIFS({lv},{lm},{a},{ld},$D{r}))")))?;
            ws.write_formula(
                i + 1,
                5,
                Formula::new(format!(
                    "=IF($B{r}<2,\"\",SUMIFS({lv},{lm},{a},{ld},MAXIFS({ld},{lm},{a},{ld},\"<\"&$D{r})))"
                )),
            )?;
            ws.write_formula(i + 1, 6, Formula::new(format!("=IF($B{r}=0,\"\",AVERAGEIFS({lv},{lm},{a}))")))?;
            ws.write_array_formula(
                i + 1,
                7,
                i + 1,
                7,
                Formula::new(format!("=IF($B{r}<2,\"\",STDEV.S(IF({lm}={a},{lv})))")),
            )?;
            ws.write_array_formula(
                i + 1,
                8,
                i + 1,
                8,
                Formula::new(format!("=IF($B{r}=0,\"\",MEDIAN(IF({lm}={a},{lv})))")),
            )?;
            ws.write_formula(i + 1, 9, Formula::new(format!("=IF($B{r}=0,\"\",MINIFS({lv},{lm},{a}))")))?;
            ws.write_formula(i + 1, 10, Formula::new(format!("=IF($B{r}=0,\"\",MAXIFS({lv},{lm},{a}))")))?;
            ws.write_array_formula(
                i + 1,
                11,
                i + 1,
                11,
                Formula::new(format!(
                    "=IF($B{r}<2,\"\",IFERROR(SLOPE(IF({lm}={a},{lv}),IF({lm}={a},{ld}))*365.25,\"\"))"
                )),
            )?;
            ws.write_formula(
                i + 1,
                12,
                Formula::new(format!("=IF({d}=\"hi\",1,IF({d}=\"lo\",-1,IF({d}=\"band\",0,\"\")))", d = mk("dir", r))),
            )?;
            ws.write_formula(
                i + 1,
                13,
                Formula::new(format!(
                    "=IF($B{r}=0,\"\",IF(COUNTIFS({lm},{a},{ld},$D{r},{c_status},\">=0\")=0,\"\",SUMIFS({c_status},{lm},{a},{ld},$D{r})))"
                )),
            )?;
            ws.write_formula(
                i + 1,
                14,
                Formula::new(format!(
                    "=IF(OR($B{r}<2,NOT(ISNUMBER($H{r}))),\"\",IF($H{r}=0,\"\",($E{r}-$G{r})/$H{r}))"
                )),
            )?;
            let (lo, hi) = (mk("ref_lo", r), mk("ref_hi", r));
            ws.write_formula(
                i + 1,
                15,
                Formula::new(format!(
                    "=IF(OR($B{r}=0,COUNT({lo},{hi})=0),\"\",IF(AND(ISNUMBER({lo}),$E{r}<{lo}),({lo}-$E{r})/IF(ISNUMBER({hi}),{hi}-{lo},ABS({lo})),IF(AND(ISNUMBER({hi}),$E{r}>{hi}),($E{r}-{hi})/IF(ISNUMBER({lo}),{hi}-{lo},ABS({hi})),0)))"
                )),
            )?;
            for (c, rng) in [(16u16, &c_inopt), (17, &c_inref)] {
                ws.write_formula(
                    i + 1,
                    c,
                    Formula::new(format!("=IF(COUNTIFS({lm},{a},{rng},\">=0\")=0,\"\",AVERAGEIFS({rng},{lm},{a}))")),
                )?;
            }
            ws.write_formula(i + 1, 18, Formula::new(format!("=IF($B{r}=0,\"\",TODAY()-$D{r})")))?;
        }
        set_widths(
            ws,
            &[24.0, 6.0, 11.0, 11.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 12.0, 8.0, 7.0, 9.0, 9.0, 10.0, 10.0, 12.0],
        )?;
        let fmt = &cx.f.date;
        ws.set_column_format(2, fmt)?;
        ws.set_column_format(3, fmt)?;
        Ok(())
    }
}

/// Zero-based column of a `Calc_Labs` header.
fn labs_col(h: &str) -> u16 {
    LABS_HEADERS.iter().position(|x| *x == h).unwrap_or_else(|| panic!("no Calc_Labs column {h}")) as u16
}

pub struct CalcPanel;

impl SheetModule for CalcPanel {
    fn id(&self) -> &'static str {
        "calc-panel"
    }
    fn title(&self) -> &'static str {
        PANEL_TITLE
    }
    fn available(&self, d: &Data) -> bool {
        need_labs(d)
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let d = cx.d;
        header(ws, PANEL_HEADERS, cx)?;
        let (nl, np) = (LABS.nrows(d), PANELS.nrows(d));
        let lp = LABS.range_n(nl, "panel_id");
        let cr = |c: &str| crate::workbook::range(LABS_TITLE, labs_col(c), 2, 1 + nl);
        let (c_inref, c_inopt) = (cr("in_ref"), cr("in_opt"));
        let rows = (PANELS.rows)(d).len() as u32;
        for i in 0..rows {
            let r = i + 2;
            let t = PANELS.sheet;
            ws.write_formula(i + 1, 0, Formula::new(format!("={t}!A{r}")))?;
            ws.write_formula_with_format(i + 1, 1, Formula::new(format!("={t}!B{r}")), &cx.f.date)?;
            ws.write_formula(i + 1, 2, Formula::new(format!("={t}!C{r}")))?;
            ws.write_formula(i + 1, 3, Formula::new(format!("=COUNTIFS({lp},$A{r})")))?;
            for (c, rng) in [(4u16, &c_inref), (5, &c_inopt)] {
                ws.write_formula_with_format(
                    i + 1,
                    c,
                    Formula::new(format!(
                        "=IF(COUNTIFS({lp},$A{r},{rng},\">=0\")=0,\"\",AVERAGEIFS({rng},{lp},$A{r}))"
                    )),
                    &cx.f.pct,
                )?;
            }
        }
        set_widths(ws, &[12.0, 11.0, 8.0, 11.0, 11.0, 11.0])?;
        let mut names = cx.shared.names.borrow_mut();
        let dates = range_ref(PANELS.sheet, 1, PANELS.col("date"), np, PANELS.col("date"));

        names.define("N_Panels", &format!("COUNT({dates})"), "calc")?;
        names.define("Latest_Panel", &format!("IF(COUNT({dates})=0,\"\",MAX({dates}))"), "calc")?;
        names.define("Prev_Panel", &format!("IF(COUNT({dates})<2,\"\",LARGE({dates},2))"), "calc")?;
        let mut k = cx.shared.kpis.borrow_mut();
        k.publish("DATA", "n_panels", "Lab panels", Some("N_Panels"), None)?;
        k.publish("DATA", "latest_panel", "Latest lab panel", Some("Latest_Panel"), Some("yyyy-mm-dd"))?;
        k.publish(
            "DATA",
            "n_measurements",
            "Lab measurements",
            Some(&format!("COUNT({})", LABS.range_n(nl, "value"))),
            None,
        )?;
        Ok(())
    }
}

pub fn modules() -> Vec<Box<dyn SheetModule>> {
    vec![Box::new(CalcLabs), Box::new(CalcMarker), Box::new(CalcPanel)]
}
