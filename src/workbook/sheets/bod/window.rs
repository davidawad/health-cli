//! BOD-25 `Bod_Window` (L-D17): eating window and meal skips over a typed meal log
//! (`tblBodMeals`: date, time, meal, skipped YES/NO).
//!
//! A per-day table for the 28 days up to the anchor (the last typed date, else
//! `Today`): first meal (`MINIFS`), last meal (`MAXIFS`), window length in hours,
//! meals eaten, meals skipped, and whether the window is within `Bod_Window_ref_h`.
//! `Bod_Window_ref_h` (16:8) is a reference line only: OUT ch52 presents time-restricted
//! eating as "the case for and against", so no target is imposed.
//!
//! PLACEHOLDER: the meal log is empty until meals are typed. `Bod_SleepHyg` reads the
//! log through [`last_meal`] for the last-meal-to-bed gap.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::sleep::{time_cells, yes_no};
use super::{at, head, kv, list, rng, section, table, Banner, Col, LOG_ROWS, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Window";
pub const MEALS: [&str; 4] = ["breakfast", "lunch", "dinner", "snack"];
const DAYS: u32 = 28;
const HR: u32 = TOP + 2;
const LOG_C: u16 = 10;
const LOG: &[&str] = &["date", "time", "meal", "skipped"];

/// Absolute range of a meal-log column (`date`, `time`, `meal`, `skipped`).
pub fn meal_log(h: &str) -> String {
    let i = LOG.iter().position(|x| *x == h).unwrap_or_else(|| panic!("tblBodMeals has no column {h}"));
    rng(TITLE, LOG_C + i as u16, HR + 1, HR + LOG_ROWS)
}

fn eaten(day: &str) -> String {
    let (d, t, s) = (meal_log("date"), meal_log("time"), meal_log("skipped"));
    format!("COUNTIFS({d},{day},{s},\"<>YES\",{t},\">=0\")")
}

/// Clock time of the last meal eaten on `day` (a date expression), or "".
pub fn last_meal(day: &str) -> String {
    let (d, t, s) = (meal_log("date"), meal_log("time"), meal_log("skipped"));
    format!("IF({}=0,\"\",MAXIFS({t},{d},{day},{s},\"<>YES\"))", eaten(day))
}

pub struct Window;

impl SheetModule for Window {
    fn id(&self) -> &'static str {
        "bod-window"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        head(
            ws,
            cx,
            "Eating-window and meal-skip tracker",
            "BOD-25",
            "L-D17 | LIFE l.3770-3776, l.10641 (16:8, skipped meals); OUT ch52 (TRE: the case for and against, no target)",
            Banner::Awaiting("meal times in tblBodMeals (date, time, meal, skipped YES/NO)"),
        )?;
        let (dl, tl, sl) = (meal_log("date"), meal_log("time"), meal_log("skipped"));
        kv(ws, cx, TOP, 0, "Anchor day", &format!("=IF(COUNT({dl})=0,Today,MAX({dl}))"), Some(theme::nf::DATE))?;
        let anchor = super::abs(1, TOP);
        section(ws, cx, HR - 1, 0, "Last 28 days (window = last meal - first meal eaten)")?;
        let a = |c: u16, r: u32| at(c, r);
        let cols = vec![
            Col::f("day", |r| format!("={anchor}-{}", HR + 1 + DAYS - r)),
            Col::f("meals_eaten", |r| format!("={}", eaten(&a(0, r)))),
            Col::f("skipped", |r| format!("=COUNTIFS({dl},{},{sl},\"YES\")", a(0, r))),
            Col::f("first_meal", |r| format!("=IF({}=0,\"\",MINIFS({tl},{dl},{},{sl},\"<>YES\"))", a(1, r), a(0, r))),
            Col::f("last_meal", |r| format!("={}", last_meal(&a(0, r)))),
            Col::f("window_h", |r| {
                let (f, l) = (a(3, r), a(4, r));
                format!("=IF(AND(ISNUMBER({f}),ISNUMBER({l})),({l}-{f})*24,\"\")")
            }),
            Col::f("within_ref", |r| format!("=IF(ISNUMBER({w}),IF({w}<=Bod_Window_ref_h,1,0),\"\")", w = a(5, r))),
            super::plot_col("window_plot", 5),
        ];
        table(ws, cx, "tblBodWindowDays", HR, 0, &cols, DAYS)?;
        let (r0, r1) = (HR + 1, HR + DAYS);
        super::date_rows(ws, cx, 0, r0, r1)?;
        super::fmt_rows(ws, r0, r1, 3, 4, "hh:mm")?;
        super::fmt_rows(ws, r0, r1, 5, 5, "0.0")?;
        theme::cf_data_bar(ws, r0, 5, r1, 5, theme::BAND_OPT)?;
        let sk = a(2, r0 + 1);
        theme::cf_formula(ws, r0, 2, r1, 2, &format!("={sk}>0"), theme::REF_OK)?;

        let sr = r1 + 2;
        let col = |c: u16| rng(TITLE, c, r0, r1);
        let avg = |c: u16| format!("=IFERROR(AVERAGE({}),\"\")", col(c));
        section(ws, cx, sr, 0, "Summary (last 28 days)")?;
        kv(ws, cx, sr + 1, 0, "Days with meals", &format!("=COUNTIF({},\">0\")", col(1)), None)?;
        kv(ws, cx, sr + 2, 0, "Mean first meal", &avg(3), Some("hh:mm"))?;
        kv(ws, cx, sr + 3, 0, "Mean last meal", &avg(4), Some("hh:mm"))?;
        kv(ws, cx, sr + 4, 0, "Mean window (h)", &avg(5), Some("0.0"))?;
        kv(
            ws,
            cx,
            sr + 5,
            0,
            "Mean meals eaten per logged day",
            &format!("=IFERROR(AVERAGEIF({c},\">0\"),\"\")", c = col(1)),
            Some("0.0"),
        )?;
        kv(ws, cx, sr + 6, 0, "Days with a skipped meal", &format!("=COUNTIF({},\">0\")", col(2)), None)?;
        kv(
            ws,
            cx,
            sr + 7,
            0,
            "% days within Bod_Window_ref_h",
            &format!("=IFERROR(SUM({c})/COUNT({c}),\"\")", c = col(6)),
            Some("0%"),
        )?;
        ws.write_string_with_format(
            sr + 8,
            0,
            "Reference only, not a target: Bod_Window_ref_h is the 16:8 pattern (LIFE l.3770-3776); OUT ch52 weighs TRE for and against.",
            &cx.f.note,
        )?;

        let mut chart = Chart::new(ChartType::Column);
        chart
            .add_series()
            .set_name("eating window (h)")
            .set_categories((TITLE, r0, 0, r1, 0))
            .set_values((TITLE, r0, 7, r1, 7));
        chart.title().set_name("Eating window per day");
        chart.x_axis().set_num_format("mm-dd");
        chart.show_na_as_empty_cell();
        chart.set_width(640).set_height(300);
        ws.insert_chart(sr + 10, 0, &chart)?;

        section(ws, cx, HR - 1, LOG_C, "Meal log (typed)")?;
        let cols = [Col::Date("date"), Col::In("time"), Col::In("meal"), Col::In("skipped")];
        table(ws, cx, "tblBodMeals", HR, LOG_C, &cols, LOG_ROWS)?;
        time_cells(ws, HR + 1, HR + LOG_ROWS, LOG_C + 1)?;
        list(ws, HR + 1, LOG_C + 2, HR + LOG_ROWS, &MEALS)?;
        yes_no(ws, HR + 1, HR + LOG_ROWS, LOG_C + 3, LOG_C + 3)?;
        set_widths(ws, &[12.0, 11.0, 8.0, 10.0, 10.0, 9.0, 10.0, 11.0, 2.0, 2.0, 12.0, 8.0, 10.0, 8.0])?;
        Ok(())
    }
}
