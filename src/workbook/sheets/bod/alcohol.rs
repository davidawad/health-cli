//! BOD-26 `Bod_Alcohol` (L-D18): drinks per evening vs the following night's sleep
//! efficiency and HRV.
//!
//! Each row of the typed log `tblBodAlcohol` (date = the evening, drinks, time of the
//! last drink) joins the next-night values: `tblWearable` HRV and sleep efficiency on
//! date+1 (a wearable row dated D describes the night ending on D), else the
//! `Bod_Sleep` log efficiency for the night of that date. The summary gives pairwise
//! Pearson r (SUMPRODUCT with ISNUMBER masks, see [`super::sleep::pearson`]) with n,
//! and mean HRV / efficiency on 0-drink vs 1+-drink nights. Log 0 on dry evenings so
//! they count as comparison nights.
//!
//! PLACEHOLDER: the log and `tblWearable` are empty until typed / imported.

use rust_xlsxwriter::{Chart, ChartFormat, ChartMarker, ChartMarkerType, ChartType, Worksheet};

use super::sleep::{n_pairs, on_night, pearson, time_cells, wear_on};
use super::{at, head, kv, rng, section, table, Banner, Col, LOG_ROWS, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Alcohol";
const HR: u32 = TOP + 1;
const HEADS: &[&str] = &[
    "date",
    "drinks",
    "last_drink",
    "note",
    "next_hrv",
    "wear_eff",
    "log_eff",
    "next_eff",
    "last_to_bed_h",
    "eff_plot",
];
const SUM_C: u16 = HEADS.len() as u16 + 1;

fn c(h: &str) -> u16 {
    HEADS.iter().position(|x| *x == h).unwrap_or_else(|| panic!("tblBodAlcohol has no column {h}")) as u16
}

/// Absolute range of an alcohol-log column.
pub fn log(h: &str) -> String {
    rng(TITLE, c(h), HR + 1, HR + LOG_ROWS)
}

/// Drinks logged on the evening `day` (a date expression), or "" when that evening is not logged.
pub fn drinks_on(day: &str) -> String {
    let (d, n) = (log("date"), log("drinks"));
    format!("IF(COUNTIFS({d},{day})=0,\"\",SUMIFS({n},{d},{day}))")
}

pub struct Alcohol;

impl SheetModule for Alcohol {
    fn id(&self) -> &'static str {
        "bod-alcohol"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        &["data-wearable", "bod-sleep"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        head(
            ws,
            cx,
            "Alcohol log vs next-night sleep and HRV",
            "BOD-26",
            "L-D18 | OUT ch60 rule 1 (alcohol impairs sleep more than any other factor); WWS ch21 tip 4",
            Banner::Awaiting("drinks per evening in tblBodAlcohol (0 on dry evenings) and next-night HRV / sleep efficiency in tblWearable or Bod_Sleep"),
        )?;
        section(ws, cx, TOP, 0, "Evenings (typed) joined to the following night")?;
        let a = |h: &str, r: u32| at(c(h), r);
        let guard = |r: u32, f: String| format!("=IF(ISNUMBER({}),{f},\"\")", a("date", r));
        let cols = vec![
            Col::Date("date"),
            Col::In("drinks"),
            Col::In("last_drink"),
            Col::In("note"),
            Col::f("next_hrv", |r| guard(r, wear_on(cx, "hrv", &format!("{}+1", a("date", r))))),
            Col::f("wear_eff", |r| guard(r, wear_on(cx, "sleep_efficiency", &format!("{}+1", a("date", r))))),
            Col::f("log_eff", |r| guard(r, on_night("efficiency", &a("date", r)))),
            Col::f("next_eff", |r| {
                let (w, l) = (a("wear_eff", r), a("log_eff", r));
                format!("=IF(ISNUMBER({w}),{w},IF(ISNUMBER({l}),{l},\"\"))")
            }),
            Col::f("last_to_bed_h", |r| {
                format!("=IF(ISNUMBER({t}),(Bod_Bedtime-{t})*24,\"\")", t = a("last_drink", r))
            }),
            super::plot_col("eff_plot", c("next_eff")),
        ];
        table(ws, cx, "tblBodAlcohol", HR, 0, &cols, LOG_ROWS)?;
        let (r0, r1) = (HR + 1, HR + LOG_ROWS);
        time_cells(ws, r0, r1, c("last_drink"))?;
        super::fmt_rows(ws, r0, r1, c("next_hrv"), c("next_hrv"), "0")?;
        super::fmt_rows(ws, r0, r1, c("wear_eff"), c("next_eff"), "0%")?;
        super::fmt_rows(ws, r0, r1, c("last_to_bed_h"), c("last_to_bed_h"), "0.0")?;
        let ce = c("next_eff");
        let e = a("next_eff", r0 + 1);
        theme::cf_formula(ws, r0, ce, r1, ce, &format!("=AND(ISNUMBER({e}),{e}<Bod_Sleep_eff)"), theme::OUT_OF_REF)?;
        theme::cf_data_bar(ws, r0, c("drinks"), r1, c("drinks"), theme::OUT_OF_REF)?;

        let sc = SUM_C;
        let (dr, hrv, eff) = (log("drinks"), log("next_hrv"), log("next_eff"));
        let mean_if = |x: &str, crit: &str| format!("=IFERROR(AVERAGEIFS({x},{dr},\"{crit}\"),\"\")");
        section(ws, cx, TOP, sc, "Drinks vs next night")?;
        let rows: Vec<(&str, String, Option<&str>)> = vec![
            ("Evenings logged", format!("=COUNT({})", log("date")), None),
            ("Mean drinks per logged evening", format!("=IFERROR(AVERAGE({dr}),\"\")"), Some("0.0")),
            ("n pairs (drinks, efficiency)", format!("={}", n_pairs(&dr, &eff)), None),
            ("r (drinks, next-night efficiency)", format!("={}", pearson(&dr, &eff)), Some("+0.00;-0.00")),
            ("n pairs (drinks, HRV)", format!("={}", n_pairs(&dr, &hrv)), None),
            ("r (drinks, next-night HRV)", format!("={}", pearson(&dr, &hrv)), Some("+0.00;-0.00")),
            ("Mean HRV, 0-drink nights", mean_if(&hrv, "0"), Some("0")),
            ("Mean HRV, 1+-drink nights", mean_if(&hrv, ">=1"), Some("0")),
            ("Mean efficiency, 0-drink nights", mean_if(&eff, "0"), Some("0%")),
            ("Mean efficiency, 1+-drink nights", mean_if(&eff, ">=1"), Some("0%")),
        ];
        for (i, (label, f, nf)) in rows.iter().enumerate() {
            kv(ws, cx, TOP + 1 + i as u32, sc, label, f, *nf)?;
        }
        ws.write_string_with_format(
            TOP + 12,
            sc,
            "OUT ch60 rule 1: alcohol impairs sleep more than any other factor. r is an association over your own nights, n shown; small n means little.",
            &cx.f.note,
        )?;

        let mut chart = Chart::new(ChartType::Scatter);
        chart
            .add_series()
            .set_name("next-night efficiency")
            .set_categories((TITLE, r0, c("drinks"), r1, c("drinks")))
            .set_values((TITLE, r0, c("eff_plot"), r1, c("eff_plot")))
            .set_format(ChartFormat::new().set_no_line())
            .set_marker(ChartMarker::new().set_type(ChartMarkerType::Circle).set_size(6));
        chart.title().set_name("Drinks vs next-night sleep efficiency");
        chart.x_axis().set_name("drinks");
        chart.y_axis().set_name("efficiency").set_num_format("0%");
        chart.show_na_as_empty_cell();
        chart.set_width(560).set_height(300);
        ws.insert_chart(TOP + 14, sc, &chart)?;
        set_widths(ws, &[11.0, 7.0, 10.0, 16.0, 9.0, 9.0, 9.0, 9.0, 12.0, 8.0, 2.0, 36.0, 10.0])?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
