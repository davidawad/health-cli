//! BOD-21 `Bod_Macro` (quant Q82; longevity.md D4; PLACEHOLDER): macro split against
//! the fat band of the current phase, with a drift line.
//!
//! `tblDaily` carries kcal and protein only, so carbs and fat are typed per day into
//! `tblBodMacroLog` (date, protein_g, carbs_g, fat_g). Each row computes kcal from the
//! Atwater factors (`Bod_Atwater_*`, 4/4/9 kcal/g), kcal per macro, fat % of kcal,
//! protein g/kg and fat g/lb from the `Bod_Cal` carried-forward weight on that date
//! (the latest weight on or before it), the fat floor check (`Bod_Fat_floor_g_lb`) and
//! the band check against the phase picked in the selector (cut: `Bod_Fat_cut_lo/hi`,
//! gain: `Bod_Fat_gain_lo/hi`; HELMS ch01:1493-1617), plus `drift7` = rolling mean of
//! fat % over the last 7 entries. Everything computes as soon as rows are typed.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{abs, at, cal_col, head, kv, list, rng, section, table, Banner, Col, LOG_ROWS, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Macro";
const HR: u32 = TOP + 1;
const C_P: u16 = 1;
const C_C: u16 = 2;
const C_F: u16 = 3;
const C_KCAL: u16 = 4;
const C_PK: u16 = 5;
const C_FK: u16 = 7;
const C_FP: u16 = 8;
const C_W: u16 = 9;
const C_FGL: u16 = 11;
const C_BAND: u16 = 13;
/// Entries in the drift mean.
const DRIFT: u32 = 7;
const SC: u16 = 16;

pub struct Macros;

impl SheetModule for Macros {
    fn id(&self) -> &'static str {
        "bod-macros"
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
            "Macro split vs targets and drift bands",
            "BOD-21",
            "Q82, L-D4 | HELMS ch01:1493-1617 (fat % bands, fat floor); FABER ch16; Atwater 4/4/9",
            Banner::Awaiting("daily carbs and fat: tblDaily has kcal and protein only; type macros in tblBodMacroLog"),
        )?;
        let s = TOP;
        let (mode, lo, hi) = (abs(SC + 1, s + 1), abs(SC + 1, s + 2), abs(SC + 1, s + 3));
        let (cd, cw) = (cal_col(cx, "date"), cal_col(cx, "weight_ff"));
        let first = HR + 2;
        section(ws, cx, TOP, 0, "Typed daily macros (blue cells); the rest are formulas")?;
        let (lo2, hi2) = (lo.clone(), hi.clone());
        let cols = vec![
            Col::Date("date"),
            Col::In("protein_g"),
            Col::In("carbs_g"),
            Col::In("fat_g"),
            Col::f("kcal", |r| {
                let (p, c, f) = (at(C_P, r), at(C_C, r), at(C_F, r));
                format!(
                    "=IF(COUNT({p},{c},{f})=0,\"\",Bod_Atwater_protein*N({p})+Bod_Atwater_carb*N({c})+Bod_Atwater_fat*N({f}))"
                )
            }),
            Col::f("protein_kcal", |r| format!("=IF(ISNUMBER({x}),Bod_Atwater_protein*{x},\"\")", x = at(C_P, r))),
            Col::f("carbs_kcal", |r| format!("=IF(ISNUMBER({x}),Bod_Atwater_carb*{x},\"\")", x = at(C_C, r))),
            Col::f("fat_kcal", |r| format!("=IF(ISNUMBER({x}),Bod_Atwater_fat*{x},\"\")", x = at(C_F, r))),
            Col::f("fat_pct", |r| {
                let (k, f) = (at(C_KCAL, r), at(C_FK, r));
                format!("=IF(AND(ISNUMBER({k}),ISNUMBER({f})),IF({k}>0,100*{f}/{k},\"\"),\"\")")
            }),
            Col::f("weight_lb", move |r| {
                let d = at(0, r);
                format!("=IF(ISNUMBER({d}),IFERROR(LOOKUP({d},{cd},{cw})+0,\"\"),\"\")")
            }),
            Col::f("protein_g_kg", |r| {
                let (p, w) = (at(C_P, r), at(C_W, r));
                format!("=IF(AND(ISNUMBER({p}),ISNUMBER({w})),IF({w}>0,{p}/({w}/Bod_lb_per_kg),\"\"),\"\")")
            }),
            Col::f("fat_g_lb", |r| {
                let (f, w) = (at(C_F, r), at(C_W, r));
                format!("=IF(AND(ISNUMBER({f}),ISNUMBER({w})),IF({w}>0,{f}/{w},\"\"),\"\")")
            }),
            Col::f("fat_floor", |r| {
                let g = at(C_FGL, r);
                format!("=IF(ISNUMBER({g}),IF({g}>=Bod_Fat_floor_g_lb,\"ok\",\"below floor\"),\"\")")
            }),
            Col::f("band", move |r| {
                let p = at(C_FP, r);
                format!(
                    "=IF(AND(ISNUMBER({p}),ISNUMBER({lo2}),ISNUMBER({hi2})),IF({p}<{lo2},\"below band\",\
                     IF({p}>{hi2},\"above band\",\"in band\")),\"\")"
                )
            }),
            Col::f("fat_pct_drift7", move |r| {
                let a = (r + 1).saturating_sub(DRIFT).max(first);
                format!("=IFERROR(AVERAGE({}:{}),\"\")", at(C_FP, a), at(C_FP, r))
            }),
        ];
        table(ws, cx, "tblBodMacroLog", HR, 0, &cols, LOG_ROWS)?;
        let (r0, r1) = (HR + 1, HR + LOG_ROWS);
        super::fmt_rows(ws, r0, r1, C_KCAL, C_FK, "0")?;
        super::fmt_rows(ws, r0, r1, C_FP, C_FP, "0.0")?;
        super::fmt_rows(ws, r0, r1, C_W + 1, C_FGL, "0.00")?;
        super::fmt_rows(ws, r0, r1, C_BAND + 1, C_BAND + 1, "0.0")?;
        let b = at(C_BAND, r0 + 1);
        theme::cf_formula(ws, r0, C_FP, r1, C_FP, &format!("={b}=\"in band\""), theme::OPTIMAL)?;
        theme::cf_formula(ws, r0, C_BAND, r1, C_BAND, &format!("={b}=\"in band\""), theme::OPTIMAL)?;
        theme::cf_formula(
            ws,
            r0,
            C_BAND,
            r1,
            C_BAND,
            &format!("=OR({b}=\"below band\",{b}=\"above band\")"),
            theme::OUT_OF_REF,
        )?;
        let fl = at(C_FGL + 1, r0 + 1);
        theme::cf_formula(ws, r0, C_FGL + 1, r1, C_FGL + 1, &format!("={fl}=\"below floor\""), theme::CRITICAL)?;

        // Selector and summary.
        section(ws, cx, s, SC, "Phase and summary")?;
        ws.write_string_with_format(s + 1, SC, "Phase (cut / gain)", &cx.f.bold)?;
        ws.write_string_with_format(s + 1, SC + 1, "cut", &cx.f.input)?;
        list(ws, s + 1, SC + 1, s + 1, &["cut", "gain"])?;
        kv(
            ws,
            cx,
            s + 2,
            SC,
            "Fat band low (% kcal)",
            &format!("=IF({mode}=\"gain\",Bod_Fat_gain_lo,Bod_Fat_cut_lo)"),
            Some("0"),
        )?;
        kv(
            ws,
            cx,
            s + 3,
            SC,
            "Fat band high (% kcal)",
            &format!("=IF({mode}=\"gain\",Bod_Fat_gain_hi,Bod_Fat_cut_hi)"),
            Some("0"),
        )?;
        let (kc, fp, band, pkg, fl) = (
            rng(TITLE, C_KCAL, r0, r1),
            rng(TITLE, C_FP, r0, r1),
            rng(TITLE, C_BAND, r0, r1),
            rng(TITLE, C_W + 1, r0, r1),
            rng(TITLE, C_FGL + 1, r0, r1),
        );
        kv(ws, cx, s + 4, SC, "Entries", &format!("=COUNT({kc})"), Some("0"))?;
        kv(ws, cx, s + 5, SC, "Mean fat (% kcal)", &format!("=IFERROR(AVERAGE({fp}),\"\")"), Some("0.0"))?;
        kv(
            ws,
            cx,
            s + 6,
            SC,
            "Entries in band (%)",
            &format!("=IFERROR(COUNTIFS({band},\"in band\")/COUNT({fp}),\"\")"),
            Some("0%"),
        )?;
        kv(ws, cx, s + 7, SC, "Mean protein (g/kg)", &format!("=IFERROR(AVERAGE({pkg}),\"\")"), Some("0.00"))?;
        kv(ws, cx, s + 8, SC, "Entries below fat floor", &format!("=COUNTIFS({fl},\"below floor\")"), Some("0"))?;
        ws.write_string_with_format(
            s + 10,
            SC,
            "Atwater general factors: protein 4, carbohydrate 4, fat 9 kcal/g (Params Bod_Atwater_*).",
            &cx.f.note,
        )?;

        let mut chart = Chart::new(ChartType::ColumnStacked);
        for (name, c) in [("protein kcal", C_PK), ("carbs kcal", C_PK + 1), ("fat kcal", C_PK + 2)] {
            chart.add_series().set_name(name).set_categories((TITLE, r0, 0, r1, 0)).set_values((TITLE, r0, c, r1, c));
        }
        chart.title().set_name("kcal by macro per logged day");
        chart.x_axis().set_num_format("yyyy-mm-dd");
        chart.set_width(720).set_height(320);
        ws.insert_chart(s + 12, SC, &chart)?;
        set_widths(
            ws,
            &[12.0, 10.0, 9.0, 8.0, 8.0, 12.0, 10.0, 9.0, 8.0, 10.0, 12.0, 9.0, 11.0, 11.0, 13.0, 2.0, 26.0, 10.0],
        )?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
