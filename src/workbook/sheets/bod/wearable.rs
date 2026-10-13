//! BOD-16 `Bod_Wearable` (Q78, Q79, L-E12, L-G15): the wearable block over `tblWearable`.
//!
//! Row-aligned with `Data_Wearable` (one row per export row): mirrors of date, HRV,
//! RHR, recovery, strain and sleep hours; strain EWMAs with spans `Bod_ACWR_acute` /
//! `Bod_ACWR_chronic` (`a = 2/(span+1)`) and their ratio (ACWR, band
//! `Bod_ACWR_lo..Bod_ACWR_hi`, UNVERIFIED Gabbett); HRV 7- and 28-day baselines
//! (AVERAGEIFS on date windows); HRV z against the previous `Bod_Z_days` days (mean
//! by AVERAGEIFS, SD by SUMPRODUCT sums) with `+-Bod_Z_band` bands; RHR vs its 28-day
//! baseline; and, per row, the previous evening's drinks (`Bod_Alcohol`), caffeine mg
//! (`Bod_Caffeine`) and last-meal-to-bed gap (`Bod_Window`). Weekly rollups add zone-2 /
//! cardio minutes from `Age_Zone2`; a matrix gives pairwise Pearson r (SUMPRODUCT with
//! ISNUMBER masks) of HRV / RHR / recovery / sleep vs those three habits.
//!
//! PLACEHOLDER: `tblWearable` is empty until a wearable export is imported.

use rust_xlsxwriter::Worksheet;

use super::sleep::{n_pairs, num_or_blank, pearson};
use super::{alcohol, at, caffeine, head, kv, last_num, rng, section, table, week_start, window, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::data::WEARABLE;
use crate::infra::theme;
use crate::workbook::sheets::age::zone2;
use crate::workbook::{col as letter, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Wearable";
const HR: u32 = TOP + 1;
const HEADS: &[&str] = &[
    "date",
    "hrv",
    "rhr",
    "recovery",
    "strain",
    "sleep_h",
    "strain_acute",
    "strain_chronic",
    "acwr",
    "hrv_7d",
    "hrv_28d",
    "z_mean",
    "z_sd",
    "hrv_z",
    "band_lo",
    "band_hi",
    "rhr_28d",
    "rhr_dev",
    "drinks_prev",
    "caffeine_mg_prev",
    "meal_gap_prev_h",
    "hrv_plot",
    "lo_plot",
    "hi_plot",
];
const SUM_C: u16 = HEADS.len() as u16 + 1;
const WEEKS: u32 = 12;

fn c(h: &str) -> u16 {
    HEADS.iter().position(|x| *x == h).unwrap_or_else(|| panic!("Bod_Wearable has no column {h}")) as u16
}

pub struct Wearable;

impl SheetModule for Wearable {
    fn id(&self) -> &'static str {
        "bod-wearable"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        &["data-wearable", "age-zone2", "bod-alcohol", "bod-caffeine", "bod-window"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "Wearable block: ACWR EWMAs, z-bands, baselines, CORREL matrix",
            "BOD-16",
            "Q78, Q79, L-E12, L-G15 | KAU Ch9 (EWMA); PAL Ch2; OUT ch58 l.25 (HRV among tracker inputs); correlation design mine",
            Banner::Awaiting("wearable export (Whoop/HRV/RHR/strain/sleep) into tblWearable"),
        )?;
        let n = WEARABLE.nrows(cx.d);
        let (r0, r1) = (HR + 1, HR + n);
        let first = HR + 2;
        let a = |h: &str, r: u32| at(c(h), r);
        let col = |h: &str| rng(TITLE, c(h), r0, r1);
        let mirror = |src: &'static str| {
            move |r: u32| num_or_blank(&format!("{}!{}{}", WEARABLE.sheet, letter(WEARABLE.col(src)), r - HR))
        };
        let ewma = |span: &'static str| {
            move |r: u32| {
                let s = at(c("strain"), r);
                if r == first {
                    return format!("=IF(ISNUMBER({s}),{s},\"\")");
                }
                let p = at(c(if span == "Bod_ACWR_acute" { "strain_acute" } else { "strain_chronic" }), r - 1);
                format!("=IF(ISNUMBER({s}),IF(ISNUMBER({p}),{p}+2/({span}+1)*({s}-{p}),{s}),{p})")
            }
        };
        let (dc, hc, rc) = (col("date"), col("hrv"), col("rhr"));
        let avg_win = |x: &str, r: u32, lo: &str, hi_op: &str| {
            let d = a("date", r);
            format!("=IF(ISNUMBER({d}),IFERROR(AVERAGEIFS({x},{dc},\">=\"&({d}-{lo}),{dc},\"{hi_op}\"&{d}),\"\"),\"\")")
        };
        let prev = |r: u32, f: String| format!("=IF(ISNUMBER({d}),{f},\"\")", d = a("date", r));
        let cols = vec![
            Col::f("date", mirror("date")),
            Col::f("hrv", mirror("hrv")),
            Col::f("rhr", mirror("rhr")),
            Col::f("recovery", mirror("recovery")),
            Col::f("strain", mirror("strain")),
            Col::f("sleep_h", mirror("sleep_hours")),
            Col::f("strain_acute", ewma("Bod_ACWR_acute")),
            Col::f("strain_chronic", ewma("Bod_ACWR_chronic")),
            Col::f("acwr", |r| {
                let (x, y) = (a("strain_acute", r), a("strain_chronic", r));
                format!("=IF(AND(ISNUMBER({x}),ISNUMBER({y})),IF({y}>0,{x}/{y},\"\"),\"\")")
            }),
            Col::f("hrv_7d", |r| avg_win(&hc, r, "6", "<=")),
            Col::f("hrv_28d", |r| avg_win(&hc, r, "27", "<=")),
            Col::f("z_mean", |r| avg_win(&hc, r, "Bod_Z_days", "<")),
            Col::f("z_sd", |r| {
                let d = a("date", r);
                let m = format!("ISNUMBER({dc})*ISNUMBER({hc})*({dc}>={d}-Bod_Z_days)*({dc}<{d})");
                let (k, s, ss) =
                    (format!("SUMPRODUCT({m})"), format!("SUMPRODUCT({m},{hc})"), format!("SUMPRODUCT({m},{hc},{hc})"));
                format!("=IF(ISNUMBER({d}),IF({k}<3,\"\",IFERROR(SQRT(MAX(0,({ss}-{s}^2/{k})/({k}-1))),\"\")),\"\")")
            }),
            Col::f("hrv_z", |r| {
                let (x, m, s) = (a("hrv", r), a("z_mean", r), a("z_sd", r));
                format!("=IF(AND(ISNUMBER({x}),ISNUMBER({m}),ISNUMBER({s})),IF({s}>0,({x}-{m})/{s},\"\"),\"\")")
            }),
            Col::f("band_lo", |r| {
                let (m, s) = (a("z_mean", r), a("z_sd", r));
                format!("=IF(AND(ISNUMBER({m}),ISNUMBER({s})),{m}-Bod_Z_band*{s},\"\")")
            }),
            Col::f("band_hi", |r| {
                let (m, s) = (a("z_mean", r), a("z_sd", r));
                format!("=IF(AND(ISNUMBER({m}),ISNUMBER({s})),{m}+Bod_Z_band*{s},\"\")")
            }),
            Col::f("rhr_28d", |r| avg_win(&rc, r, "28", "<")),
            Col::f("rhr_dev", |r| {
                let (x, b) = (a("rhr", r), a("rhr_28d", r));
                format!("=IF(AND(ISNUMBER({x}),ISNUMBER({b})),{x}-{b},\"\")")
            }),
            Col::f("drinks_prev", |r| prev(r, alcohol::drinks_on(&format!("({}-1)", a("date", r))))),
            Col::f("caffeine_mg_prev", |r| prev(r, caffeine::mg_on(&format!("({}-1)", a("date", r))))),
            Col::f("meal_gap_prev_h", |r| {
                let lm = window::last_meal(&format!("({}-1)", a("date", r)));
                prev(r, format!("IFERROR((Bod_Bedtime-{lm})*24,\"\")"))
            }),
            super::plot_col("hrv_plot", c("hrv")),
            super::plot_col("lo_plot", c("band_lo")),
            super::plot_col("hi_plot", c("band_hi")),
        ];
        table(ws, cx, "tblBodWearable", HR, 0, &cols, n)?;
        super::date_rows(ws, cx, 0, r0, r1)?;
        super::fmt_rows(ws, r0, r1, c("strain_acute"), c("acwr"), "0.00")?;
        super::fmt_rows(ws, r0, r1, c("hrv_7d"), c("rhr_dev"), "0.0")?;
        super::fmt_rows(ws, r0, r1, c("hrv_z"), c("hrv_z"), "+0.00;-0.00;0.00")?;
        super::fmt_rows(ws, r0, r1, c("meal_gap_prev_h"), c("meal_gap_prev_h"), "0.0")?;
        let ca = c("acwr");
        let x = a("acwr", r0 + 1);
        let out = format!("=AND(ISNUMBER({x}),OR({x}<Bod_ACWR_lo,{x}>Bod_ACWR_hi))");
        theme::cf_formula(ws, r0, ca, r1, ca, &out, theme::UNVERIFIED)?;
        let ok = format!("=AND(ISNUMBER({x}),{x}>=Bod_ACWR_lo,{x}<=Bod_ACWR_hi)");
        theme::cf_formula(ws, r0, ca, r1, ca, &ok, theme::OPTIMAL)?;
        theme::cf_icons(ws, r0, c("hrv_z"), r1, c("hrv_z"))?;
        theme::cf_three_color(ws, r0, c("rhr_dev"), r1, c("rhr_dev"), true)?;

        // Summary.
        let sc = SUM_C;
        section(ws, cx, TOP, sc, "Latest")?;
        let rows: Vec<(&str, String, Option<&str>)> = vec![
            ("Wearable days", format!("=COUNT({dc})"), None),
            ("HRV, latest", format!("={}", last_num(&hc)), Some("0")),
            ("HRV 7-day baseline", format!("={}", last_num(&col("hrv_7d"))), Some("0.0")),
            ("HRV 28-day baseline", format!("={}", last_num(&col("hrv_28d"))), Some("0.0")),
            ("HRV z, latest", format!("={}", last_num(&col("hrv_z"))), Some("+0.00;-0.00")),
            ("ACWR, latest", format!("={}", last_num(&col("acwr"))), Some("0.00")),
            (
                "ACWR band",
                format!(
                    "=IF(ISNUMBER({x}),IF({x}<Bod_ACWR_lo,\"below band\",IF({x}>Bod_ACWR_hi,\"above band\",\"in band\")),\"\")",
                    x = super::abs(sc + 1, TOP + 6)
                ),
                None,
            ),
            ("RHR vs 28-day baseline, latest", format!("={}", last_num(&col("rhr_dev"))), Some("+0.0;-0.0")),
            ("Strain EWMA alpha, acute", "=2/(Bod_ACWR_acute+1)".into(), Some("0.000")),
            ("Strain EWMA alpha, chronic", "=2/(Bod_ACWR_chronic+1)".into(), Some("0.000")),
        ];
        for (i, (label, fm, nf)) in rows.iter().enumerate() {
            kv(ws, cx, TOP + 1 + i as u32, sc, label, fm, *nf)?;
        }
        theme::unverified(
            ws,
            TOP + 11,
            sc,
            "Gabbett 2016 (Br J Sports Med 50:273): ACWR band Bod_ACWR_lo..Bod_ACWR_hi",
            f,
        )?;

        // Weekly rollup.
        let wr = TOP + 13;
        section(ws, cx, wr, sc, "Weeks (Monday start): strain vs cardio minutes (Age_Zone2 log)")?;
        let st = col("strain");
        let wk = |r: u32| at(sc, r);
        let inw = move |x: &str, r: u32| {
            format!("{x},{dc},\">=\"&{w},{dc},\"<\"&({w}+7)", w = wk(r), dc = rng(TITLE, 0, r0, r1))
        };
        let cols = vec![
            Col::f("week_of", |r| format!("={}", week_start(wr + 2 + WEEKS - r))),
            Col::f("strain_sum", |r| {
                format!("=IF(COUNTIFS({})=0,\"\",SUMIFS({}))", inw(&format!("{st},\">=0\""), r), inw(&st, r))
            }),
            Col::f("hrv_mean", |r| format!("=IFERROR(AVERAGEIFS({}),\"\")", inw(&hc, r))),
            Col::f("rhr_mean", |r| format!("=IFERROR(AVERAGEIFS({}),\"\")", inw(&rc, r))),
            Col::f("cardio_min", |r| format!("={}", zone2::cardio_minutes(&wk(r)))),
        ];
        table(ws, cx, "tblBodWearableWeeks", wr + 1, sc, &cols, WEEKS)?;
        let (w0, w1) = (wr + 2, wr + 1 + WEEKS);
        super::date_rows(ws, cx, sc, w0, w1)?;
        super::fmt_rows(ws, w0, w1, sc + 1, sc + 3, "0.0")?;
        let wcol = |k: u16| rng(TITLE, sc + k, w0, w1);
        kv(
            ws,
            cx,
            w1 + 1,
            sc,
            "r (weekly strain, cardio min)",
            &format!("={}", pearson(&wcol(1), &wcol(4))),
            Some("+0.00;-0.00"),
        )?;

        // Correlation matrix.
        let mr = w1 + 3;
        section(ws, cx, mr, sc, "Pearson r (pairs where both are numbers): metric vs the previous evening")?;
        let xs = [("drinks", "drinks_prev"), ("caffeine mg", "caffeine_mg_prev"), ("meal-to-bed h", "meal_gap_prev_h")];
        let ys = ["hrv", "rhr", "recovery", "sleep_h"];
        for (j, (label, _)) in xs.iter().enumerate() {
            ws.write_string_with_format(mr + 1, sc + 1 + j as u16, *label, &f.bold)?;
            ws.write_string_with_format(mr + 1, sc + 5 + j as u16, format!("n {label}"), &f.bold)?;
        }
        for (i, y) in ys.iter().enumerate() {
            let rr = mr + 2 + i as u32;
            ws.write_string_with_format(rr, sc, *y, &f.bold)?;
            for (j, (_, x)) in xs.iter().enumerate() {
                let (xr, yr) = (col(x), col(y));
                let nf = super::nf("+0.00;-0.00;0.00");
                ws.write_formula_with_format(rr, sc + 1 + j as u16, format!("={}", pearson(&xr, &yr)).as_str(), &nf)?;
                ws.write_formula(rr, sc + 5 + j as u16, format!("={}", n_pairs(&xr, &yr)).as_str())?;
            }
        }
        theme::cf_three_color(ws, mr + 2, sc + 1, mr + 5, sc + 3, true)?;

        let chart = super::line_chart(
            TITLE,
            "HRV with z-bands (previous Bod_Z_days days, +-Bod_Z_band sd)",
            r0,
            r1,
            0,
            &[("HRV", c("hrv_plot")), ("band low", c("lo_plot")), ("band high", c("hi_plot"))],
        );
        ws.insert_chart(mr + 8, sc, &chart)?;
        let mut w = vec![11.0, 7.0, 7.0, 8.0, 7.0, 7.0, 8.0, 8.0, 7.0, 8.0, 8.0, 8.0, 7.0, 7.0, 8.0, 8.0, 8.0, 8.0];
        w.extend([8.0, 9.0, 9.0, 7.0, 7.0, 7.0, 2.0, 30.0, 11.0, 11.0, 11.0, 11.0, 11.0, 11.0, 11.0]);
        set_widths(ws, &w)?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
