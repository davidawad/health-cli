//! BOD-24 `Bod_Glycemic` (L-D12, L-D13, L-D14, C4.8): glycemic-hack compliance, a CGM
//! insight panel and fibre per day.
//!
//! * `tblBodHacks`: one row per day, YES/NO for Inchauspe's six hacks (GLU ch16-25),
//!   a daily score and the 4-week compliance %.
//! * `tblBodCGM`: typed / pasted readings (date, time, mg/dL, tag). Row formulas: the
//!   day's reference (latest pre-meal / fasting reading at or before this one, else the
//!   day's first reading), the rise above it, spike = rise > `Bod_Spike_rise`, and in
//!   range `Bod_CGM_lo..Bod_CGM_hi`. Panel: mean, SD, CV, % time in range (COUNTIFS),
//!   spikes, fasting mean.
//! * `tblBodFibre`: grams of fibre per day vs `Bod_Fibre_g`, per 1000 kcal using the
//!   `Bod_Cal` kcal of that date.
//!
//! A per-day table (28 days up to the last typed date) lines the three up and gives
//! the pairwise r of hack score vs spikes. Pattern wording is `theme::dx_risk`.
//! PLACEHOLDER: no CGM import or typed logs exist yet.

use rust_xlsxwriter::Worksheet;

use super::sleep::{n_pairs, pearson, time_cells, yes_no};
use super::{at, cal_col, head, kv, list, rng, section, table, Banner, Col, LOG_ROWS, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Glycemic";
pub const HACKS: [&str; 6] = [
    "veg_first",
    "savory_breakfast",
    "vinegar_before_carbs",
    "post_meal_walk",
    "dessert_not_snack",
    "carbs_with_clothes",
];
pub const TAGS: [&str; 5] = ["fasting", "pre-meal", "post-meal", "bedtime", "other"];
const HR: u32 = TOP + 3;
const CGM_C: u16 = 9;
const FIB_C: u16 = 18;
const PD_C: u16 = 21;
const DAYS: u32 = 28;
const SUM_C: u16 = PD_C + 14;

fn r_(c: u16) -> String {
    rng(TITLE, c, HR + 1, HR + LOG_ROWS)
}

/// Absolute range of a CGM-log column (`date`, `time`, `mg_dl`, `tag`, ...).
pub fn cgm(h: &str) -> String {
    let i = ["date", "time", "mg_dl", "tag", "ref_mg", "rise", "spike", "in_range"]
        .iter()
        .position(|x| *x == h)
        .unwrap_or_else(|| panic!("tblBodCGM has no column {h}"));
    r_(CGM_C + i as u16)
}

/// The first CGM reading (mg/dL) on `day` (a date expression), or "".
pub fn first_reading(day: &str) -> String {
    let (d, t, m) = (cgm("date"), cgm("time"), cgm("mg_dl"));
    format!("IF(COUNTIFS({d},{day},{m},\">=0\")=0,\"\",IFERROR(LOOKUP(2,1/(({d}={day})*({t}=MINIFS({t},{d},{day}))*ISNUMBER({m})),{m}),\"\"))")
}

pub struct Glycemic;

impl SheetModule for Glycemic {
    fn id(&self) -> &'static str {
        "bod-glycemic"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        &["bod-cal"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "Glycemic-hack log, CGM panel, fibre per day",
            "BOD-24",
            "L-D12, L-D13, L-D14, C4.8 | GLU ch11 (spike = rise > 30 mg/dL), ch16-25 (hacks; vinegar -8..30%, ch22); OUT ch49 l.3-15 (time in 70-110, fibre > 50 g)",
            Banner::Awaiting("daily hack checklist (tblBodHacks), CGM readings (tblBodCGM: date, time, mg/dL, tag), fibre g per day (tblBodFibre)"),
        )?;
        let (r0, r1) = (HR + 1, HR + LOG_ROWS);
        let (hd, hs) = (r_(0), r_(7));
        let (cd, cm, csp) = (cgm("date"), cgm("mg_dl"), cgm("spike"));
        let (fd, fg) = (r_(FIB_C), r_(FIB_C + 1));
        kv(
            ws,
            cx,
            TOP,
            0,
            "Anchor day",
            &format!("=IF(COUNT({hd},{cd},{fd})=0,Today,MAX({hd},{cd},{fd}))"),
            Some(theme::nf::DATE),
        )?;
        let anchor = super::abs(1, TOP);

        // (a) hack checklist.
        section(ws, cx, HR - 1, 0, "Glycemic hacks (YES/NO per day)")?;
        let mut cols = vec![Col::Date("date")];
        cols.extend(HACKS.iter().map(|h| Col::In(h)));
        cols.push(Col::f("score", |r| {
            format!("=IF(ISNUMBER({}),COUNTIF({}:{},\"YES\"),\"\")", at(0, r), at(1, r), at(6, r))
        }));
        table(ws, cx, "tblBodHacks", HR, 0, &cols, LOG_ROWS)?;
        yes_no(ws, r0, r1, 1, 6)?;
        theme::cf_data_bar(ws, r0, 7, r1, 7, theme::BAND_OPT)?;

        // (b) CGM log.
        section(ws, cx, HR - 1, CGM_C, "CGM readings (typed or pasted)")?;
        let (ct, cg) = (cgm("time"), cgm("tag"));
        let g = |k: u16, r: u32| at(CGM_C + k, r);
        let cols = vec![
            Col::Date("date"),
            Col::In("time"),
            Col::In("mg_dl"),
            Col::In("tag"),
            Col::f("ref_mg", |r| {
                let (d, t, m) = (g(0, r), g(1, r), g(2, r));
                format!(
                    "=IF(ISNUMBER({m}),IFERROR(LOOKUP(2,1/(({cd}={d})*({ct}<={t})*ISNUMBER({cm})*(({cg}=\"pre-meal\")+({cg}=\"fasting\"))),{cm}),{}),\"\")",
                    first_reading(&d)
                )
            }),
            Col::f("rise", |r| {
                let (m, b) = (g(2, r), g(4, r));
                format!("=IF(AND(ISNUMBER({m}),ISNUMBER({b})),{m}-{b},\"\")")
            }),
            Col::f("spike", |r| format!("=IF(ISNUMBER({x}),IF({x}>Bod_Spike_rise,1,0),\"\")", x = g(5, r))),
            Col::f("in_range", |r| {
                format!("=IF(ISNUMBER({m}),IF(AND({m}>=Bod_CGM_lo,{m}<=Bod_CGM_hi),1,0),\"\")", m = g(2, r))
            }),
        ];
        table(ws, cx, "tblBodCGM", HR, CGM_C, &cols, LOG_ROWS)?;
        time_cells(ws, r0, r1, CGM_C + 1)?;
        list(ws, r0, CGM_C + 3, r1, &TAGS)?;
        let m0 = g(2, r0 + 1);
        let out = format!("=AND(ISNUMBER({m0}),OR({m0}<Bod_CGM_lo,{m0}>Bod_CGM_hi))");
        theme::cf_formula(ws, r0, CGM_C + 2, r1, CGM_C + 2, &out, theme::OUT_OF_REF)?;
        let s0 = g(6, r0 + 1);
        theme::cf_formula(ws, r0, CGM_C + 6, r1, CGM_C + 6, &format!("={s0}=1"), theme::OUT_OF_REF)?;

        // (c) fibre log.
        section(ws, cx, HR - 1, FIB_C, "Fibre (typed g/day)")?;
        table(ws, cx, "tblBodFibre", HR, FIB_C, &[Col::Date("date"), Col::In("fibre_g")], LOG_ROWS)?;

        // Per-day table.
        section(ws, cx, HR - 1, PD_C, "Last 28 days")?;
        let (kd, kk) = (cal_col(cx, "date"), cal_col(cx, "kcal"));
        let p = |k: u16, r: u32| at(PD_C + k, r);
        let cols = vec![
            Col::f("day", |r| format!("={anchor}-{}", HR + 1 + DAYS - r)),
            Col::f("hack_score", |r| format!("=IF(COUNTIFS({hd},{d})=0,\"\",SUMIFS({hs},{hd},{d}))", d = p(0, r))),
            Col::f("cgm_n", |r| format!("=COUNTIFS({cd},{},{cm},\">=0\")", p(0, r))),
            Col::f("cgm_mean", |r| format!("=IF({}=0,\"\",AVERAGEIFS({cm},{cd},{}))", p(2, r), p(0, r))),
            Col::f("cgm_peak", |r| format!("=IF({}=0,\"\",MAXIFS({cm},{cd},{}))", p(2, r), p(0, r))),
            Col::f("spikes", |r| format!("=IF({}=0,\"\",SUMIFS({csp},{cd},{}))", p(2, r), p(0, r))),
            Col::f("tir", |r| {
                let (n, d) = (p(2, r), p(0, r));
                format!("=IF({n}=0,\"\",COUNTIFS({cd},{d},{cm},\">=\"&Bod_CGM_lo,{cm},\"<=\"&Bod_CGM_hi)/{n})")
            }),
            Col::f("fibre_g", |r| format!("=IF(COUNTIFS({fd},{d})=0,\"\",SUMIFS({fg},{fd},{d}))", d = p(0, r))),
            Col::f("kcal", |r| format!("=IFERROR(1*INDEX({kk},MATCH({},{kd},0)),\"\")", p(0, r))),
            Col::f("fibre_per_1000kcal", |r| {
                let (x, k) = (p(7, r), p(8, r));
                format!("=IF(AND(ISNUMBER({x}),ISNUMBER({k})),IF({k}>0,{x}/{k}*1000,\"\"),\"\")")
            }),
            Col::f("fibre_met", |r| format!("=IF(ISNUMBER({x}),IF({x}>=Bod_Fibre_g,1,0),\"\")", x = p(7, r))),
            super::plot_col("mean_plot", PD_C + 3),
            super::plot_col("peak_plot", PD_C + 4),
        ];
        table(ws, cx, "tblBodGlycemicDays", HR, PD_C, &cols, DAYS)?;
        let (d0, d1) = (HR + 1, HR + DAYS);
        super::date_rows(ws, cx, PD_C, d0, d1)?;
        super::fmt_rows(ws, d0, d1, PD_C + 3, PD_C + 4, "0")?;
        super::fmt_rows(ws, d0, d1, PD_C + 6, PD_C + 6, "0%")?;
        super::fmt_rows(ws, d0, d1, PD_C + 9, PD_C + 9, "0.0")?;
        theme::cf_three_color(ws, d0, PD_C + 5, d1, PD_C + 5, true)?;
        let fm = p(10, d0 + 1);
        theme::cf_formula(ws, d0, PD_C + 7, d1, PD_C + 7, &format!("={fm}=1"), theme::OPTIMAL)?;

        // Panel.
        let sc = SUM_C;
        let day = |k: u16| rng(TITLE, PD_C + k, d0, d1);
        let n = format!("COUNT({cm})");
        let wnd = format!("\">\"&({anchor}-28)");
        let rows: Vec<(&str, String, Option<&str>)> = vec![
            ("CGM readings", format!("={n}"), None),
            ("Mean glucose (mg/dL)", format!("=IFERROR(AVERAGE({cm}),\"\")"), Some("0")),
            ("SD (mg/dL)", format!("=IF({n}<2,\"\",STDEV.S({cm}))"), Some("0.0")),
            ("CV", format!("=IF({n}<2,\"\",IFERROR(STDEV.S({cm})/AVERAGE({cm}),\"\"))"), Some("0.0%")),
            (
                "% time in Bod_CGM_lo..Bod_CGM_hi",
                format!("=IF({n}=0,\"\",COUNTIFS({cm},\">=\"&Bod_CGM_lo,{cm},\"<=\"&Bod_CGM_hi)/{n})"),
                Some("0%"),
            ),
            ("Spikes (rise > Bod_Spike_rise)", format!("=SUM({csp})"), None),
            ("Fasting mean (mg/dL)", format!("=IFERROR(AVERAGEIFS({cm},{cg},\"fasting\"),\"\")"), Some("0")),
            ("Hack days logged", format!("=COUNT({hd})"), None),
            (
                "Hack compliance, last 4 weeks",
                format!("=IFERROR(SUMIFS({hs},{hd},{wnd})/(6*COUNTIFS({hd},{wnd})),\"\")"),
                Some("0%"),
            ),
            ("n days (hack score, spikes)", format!("={}", n_pairs(&day(1), &day(5))), None),
            ("r (hack score, spikes)", format!("={}", pearson(&day(1), &day(5))), Some("+0.00;-0.00")),
            ("Mean fibre (g/day, 28 d)", format!("=IFERROR(AVERAGE({}),\"\")", day(7)), Some("0")),
            ("Days at Bod_Fibre_g", format!("=IFERROR(SUM({c})/COUNT({c}),\"\")", c = day(10)), Some("0%")),
            ("Fibre per 1000 kcal (mean)", format!("=IFERROR(AVERAGE({}),\"\")", day(9)), Some("0.0")),
        ];
        section(ws, cx, HR - 1, sc, "CGM, hack and fibre panel")?;
        for (i, (label, fm, nf)) in rows.iter().enumerate() {
            kv(ws, cx, HR + i as u32, sc, label, fm, *nf)?;
        }
        let spikes = super::abs(sc + 1, HR + 5);
        let note = theme::dx_risk("repeated post-meal glucose rises above Bod_Spike_rise mg/dL");
        kv(
            ws,
            cx,
            HR + rows.len() as u32,
            sc,
            "Pattern note",
            &format!("=IF({spikes}>=Bod_Spike_note_n,\"{note}\",\"\")"),
            None,
        )?;
        ws.write_string_with_format(
            HR + rows.len() as u32 + 2,
            sc,
            "OUT ch49: 5-6 h of sleep vs 8 h raises the peak by about 10-20 mg/dL; aerobic work lowers glucose, lifting / HIIT raise it briefly.",
            &f.note,
        )?;
        let chart = super::line_chart(
            TITLE,
            "CGM mean and peak per day (mg/dL)",
            d0,
            d1,
            PD_C,
            &[("mean", PD_C + 11), ("peak", PD_C + 12)],
        );
        ws.insert_chart(HR + rows.len() as u32 + 4, sc, &chart)?;
        let mut w = vec![11.0, 9.0, 10.0, 10.0, 10.0, 10.0, 10.0, 7.0, 2.0];
        w.extend([11.0, 8.0, 8.0, 9.0, 8.0, 7.0, 7.0, 8.0, 2.0, 11.0, 8.0, 2.0]);
        w.extend([11.0, 10.0, 7.0, 9.0, 9.0, 7.0, 7.0, 8.0, 8.0, 10.0, 9.0, 8.0, 8.0, 2.0, 34.0, 10.0]);
        set_widths(ws, &w)?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
