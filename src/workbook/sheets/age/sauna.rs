//! AGE-17 `Age_Sauna`: weekly sauna sessions with the Finnish-cohort
//! reference, and a post-meal walk log with the weekly share of meals walked.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{cell, head, kpi, list, rng, section, table, week_start, Banner, Col, LOG_ROWS, TOP};
use crate::error::Result;
use crate::infra::data::ROUTINE;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Age_Sauna";
const WEEKS: u32 = 12;
const W_HR: u32 = TOP + 3;
const S_C: u16 = 9;
const K_C: u16 = 14;
const MEALS: [&str; 4] = ["breakfast", "lunch", "dinner", "snack"];
/// Main meals per week (3 a day), the denominator of the walked share.
const MEALS_PER_WEEK: u32 = 21;

pub struct Sauna;

impl SheetModule for Sauna {
    fn id(&self) -> &'static str {
        "age-sauna"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Sauna sessions and post-meal walks",
            "AGE-17 (L-C18, L-C19) | LIFE l.4219-4229 (Finnish cohort, up to 7x/week); OUT ch28 l.35; GLU ch23; TW 2014400965445931450",
            Banner::Note("Cohort associations, not trial results: the reference line is context, not a target."),
        )?;
        let nr = ROUTINE.nrows(d);
        super::kv(
            ws,
            cx,
            TOP,
            0,
            "Sauna steps in tblRoutine",
            &format!("=COUNTIF({},\"*sauna*\")", ROUTINE.range_n(nr, "name")),
            None,
        )?;
        let (sd, sm) = (rng(TITLE, S_C, W_HR + 1, W_HR + LOG_ROWS), rng(TITLE, S_C + 1, W_HR + 1, W_HR + LOG_ROWS));
        let (kd, km) = (rng(TITLE, K_C, W_HR + 1, W_HR + LOG_ROWS), rng(TITLE, K_C + 2, W_HR + 1, W_HR + LOG_ROWS));
        let in_week = |range: &str, r: u32| format!("{range},\">=\"&$A{r},{range},\"<\"&($A{r}+7)");
        section(ws, cx, W_HR - 1, "Weeks (Monday start, oldest first)")?;
        let cols = [
            Col::f("week_of", |r| format!("={}", week_start(W_HR + WEEKS + 1 - r))),
            Col::f("sauna_sessions", |r| format!("=COUNTIFS({})", in_week(&sd, r))),
            Col::f("sauna_minutes", |r| format!("=SUMIFS({sm},{})", in_week(&sd, r))),
            Col::f("reference", |_| "=Age_Sauna_ref".into()),
            Col::f("walks", |r| format!("=COUNTIFS({})", in_week(&kd, r))),
            Col::f("walks_long_enough", |r| format!("=COUNTIFS({},{km},\">=\"&Age_Walk_min)", in_week(&kd, r))),
            Col::f("pct_meals_walked", |r| format!("=F{r}/{MEALS_PER_WEEK}")),
        ];
        table(ws, cx, "tblAgeSaunaWeeks", W_HR, 0, &cols, WEEKS)?;
        let (w0, w1) = (W_HR + 1, W_HR + WEEKS);
        for r in w0..=w1 {
            ws.set_cell_format(r, 0, &f.date)?;
            ws.set_cell_format(r, 6, &f.pct)?;
        }
        theme::cf_data_bar(ws, w0, 1, w1, 1, theme::BAND_BLUEPRINT)?;
        let last_full = w1 - 1;
        super::kv(
            ws,
            cx,
            TOP + 1,
            0,
            "Sauna sessions last full week",
            &format!("={}", cell(TITLE, last_full, 1)),
            None,
        )?;
        kpi(cx, "sauna_week", "Sauna sessions last full week", TITLE, TOP + 1, 1, None)?;
        let mut chart = Chart::new(ChartType::Column);
        chart
            .add_series()
            .set_name("sauna sessions")
            .set_categories((TITLE, w0, 0, w1, 0))
            .set_values((TITLE, w0, 1, w1, 1));
        let mut line = Chart::new(ChartType::Line);
        line.add_series().set_name("reference").set_categories((TITLE, w0, 0, w1, 0)).set_values((TITLE, w0, 3, w1, 3));
        chart.combine(&line);
        chart.title().set_name("Sauna sessions per week");
        chart.x_axis().set_num_format("mm-dd");
        chart.set_width(520).set_height(260);
        ws.insert_chart(w1 + 2, 0, &chart)?;

        ws.write_string_with_format(W_HR - 1, S_C, "Sauna log (typed)", &f.section)?;
        let cols = [Col::Date("date"), Col::In("minutes"), Col::In("temp_c"), Col::In("note")];
        table(ws, cx, "tblAgeSaunaLog", W_HR, S_C, &cols, LOG_ROWS)?;
        ws.write_string_with_format(W_HR - 1, K_C, "Post-meal walk log (typed)", &f.section)?;
        let cols = [Col::Date("date"), Col::In("meal"), Col::In("minutes"), Col::In("note")];
        table(ws, cx, "tblAgeWalkLog", W_HR, K_C, &cols, LOG_ROWS)?;
        list(ws, W_HR + 1, K_C + 1, W_HR + LOG_ROWS, &MEALS)?;
        set_widths(
            ws,
            &[30.0, 10.0, 10.0, 10.0, 8.0, 12.0, 12.0, 2.0, 2.0, 12.0, 8.0, 8.0, 20.0, 2.0, 12.0, 10.0, 8.0, 20.0],
        )?;
        Ok(())
    }
}
