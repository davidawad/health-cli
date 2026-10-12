//! AGE-11 `Age_Decathlon`: Centenarian Decathlon tracker. Each task has a
//! target at the target age; the capacity required today is
//! `target / (1 - decline)^(years / 10)` (check: 30 lb at 80 -> ~50 lb at 40 with
//! 12%/decade). Declines default to the shared `Decline_*_pct_decade` parameters
//! (UNVERIFIED, blank until checked), so the required column stays empty until
//! those are typed or a row's decline is overridden.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{abs, as_fraction, at, head, kpi, list, rng, section, table, Banner, Col, AGE_NOW, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Age_Decathlon";
const HR: u32 = TOP + 5;
const ROWS: u32 = 20;
const KINDS: [&str; 4] = ["strength", "cardio", "balance", "other"];
/// (task, metric, unit, target at the target age, kind). OUT ch31 list (via longevity.md L-C1).
const TASKS: [(&str, &str, &str, Option<f64>, &str); 13] = [
    ("Hike 1.5 mi on a hilly trail", "VO2 max", "ml/kg/min", Some(30.0), "cardio"),
    ("Get up off the floor using one arm", "reps", "count", Some(1.0), "strength"),
    ("Pick up a young child from the floor", "load", "lb", Some(30.0), "strength"),
    ("Carry two 5-lb grocery bags five blocks", "load per hand", "lb", Some(5.0), "strength"),
    ("Lift a 20-lb suitcase overhead", "load", "lb", Some(20.0), "strength"),
    ("Balance on one leg, eyes open", "time", "s", Some(30.0), "balance"),
    ("Have sex", "able", "yes=1", None, "other"),
    ("Climb four flights of stairs in 3 min", "flights in 3 min", "count", Some(4.0), "cardio"),
    ("Open a jar", "able", "yes=1", None, "strength"),
    ("30 consecutive jump-rope skips", "skips", "count", Some(30.0), "cardio"),
    ("Pull-ups", "reps", "count", Some(5.0), "strength"),
    ("Dead hang", "time", "s", Some(60.0), "strength"),
    ("90 steps in 2 min (VO2 ~32)", "steps in 2 min", "count", Some(90.0), "cardio"),
];

/// Capacity required today for `target` at the target age after `years` of `decline` per decade.
pub fn required_formula(target: &str, decline: &str, years: &str) -> String {
    format!("IF(OR({target}=\"\",{decline}=\"\",{years}=\"\"),\"\",{target}/(1-{decline})^({years}/10))")
}

/// Task names column.
pub fn task_range() -> String {
    rng(TITLE, 0, HR + 1, HR + ROWS)
}

/// Result column (PASS / not yet), row-aligned with [`task_range`].
pub fn result_range() -> String {
    rng(TITLE, 12, HR + 1, HR + ROWS)
}

/// Pass count cell (for the decade planner).
pub fn pass_cell() -> String {
    super::cell(TITLE, TOP + 3, 1)
}

pub struct Decathlon;

impl SheetModule for Decathlon {
    fn id(&self) -> &'static str {
        "age-decathlon"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "Centenarian Decathlon",
            "AGE-11 (L-C1) | OUT ch31 (task list), ch34 (VO2 decline); decline 8-17%/decade strength per ch31 text",
            Banner::Awaiting("your test results, and Decline_Strength_pct_decade / Decline_VO2_pct_decade (UNVERIFIED, blank in Params)"),
        )?;
        section(ws, cx, TOP, "You")?;
        super::kv(ws, cx, TOP + 1, 0, "Age now", &format!("={AGE_NOW}"), Some("0.0"))?;
        super::kv(ws, cx, TOP + 2, 0, "Default target age", "=Age_Target_age", None)?;
        let age = abs(1, TOP + 1);
        let n = ROWS;
        let (r0, r1) = (HR + 1, HR + n);
        let cols = [
            Col::In("task"),
            Col::In("metric"),
            Col::In("unit"),
            Col::In("kind"),
            Col::In("target_at_age"),
            Col::f("target_age", |_| "=Age_Target_age".into()),
            Col::f("years_to_target", |r| format!("=IF(ISNUMBER({age}),MAX(0,F{r}-{age}),\"\")")),
            Col::f("decline_per_decade", |r| {
                format!(
                    "=IF(D{r}=\"cardio\",{},{})",
                    as_fraction("Decline_VO2_pct_decade"),
                    as_fraction("Decline_Strength_pct_decade")
                )
            }),
            Col::f("required_now", |r| format!("={}", required_formula(&at(4, r), &at(7, r), &at(6, r)))),
            Col::In("measured"),
            Col::Date("test_date"),
            Col::f("pct_of_required", |r| format!("=IF(AND(ISNUMBER(J{r}),ISNUMBER(I{r}),N(I{r})>0),J{r}/I{r},\"\")")),
            Col::f("result", |r| {
                format!("=IF(OR(J{r}=\"\",NOT(ISNUMBER(I{r}))),\"\",IF(J{r}>=I{r},\"PASS\",\"not yet\"))")
            }),
        ];
        table(ws, cx, "tblAgeDecathlon", HR, 0, &cols, n)?;
        for (i, (task, metric, unit, target, kind)) in TASKS.iter().enumerate() {
            let r = r0 + i as u32;
            ws.write_string_with_format(r, 0, *task, &f.input)?;
            ws.write_string_with_format(r, 1, *metric, &f.input)?;
            ws.write_string_with_format(r, 2, *unit, &f.input)?;
            ws.write_string_with_format(r, 3, *kind, &f.input)?;
            if let Some(t) = target {
                ws.write_number_with_format(r, 4, *t, &f.input)?;
            }
        }
        for r in r0..=r1 {
            ws.set_cell_format(r, 7, &f.pct)?;
            ws.set_cell_format(r, 8, &f.dec1)?;
            ws.set_cell_format(r, 11, &f.pct)?;
        }
        theme::note_source(ws, HR, 0, "OUT ch31: 10 core tasks plus extras (pull-ups, dead hang, 90 steps in 2 min)")?;
        theme::note_source(
            ws,
            HR,
            7,
            "Overwrite per row; defaults are Decline_VO2_pct_decade (cardio) or Decline_Strength_pct_decade",
        )?;
        list(ws, r0, 3, r1, &KINDS)?;
        theme::cf_data_bar(ws, r0, 11, r1, 11, theme::BAND_OPT)?;
        theme::cf_formula(ws, r0, 12, r1, 12, &format!("={}=\"PASS\"", at(12, r0 + 1)), theme::OPTIMAL)?;
        theme::cf_formula(ws, r0, 12, r1, 12, &format!("={}=\"not yet\"", at(12, r0 + 1)), theme::OUT_OF_REF)?;
        let res = rng(TITLE, 12, r0, r1);
        super::kv(ws, cx, TOP + 3, 0, "Tasks passed", &format!("=COUNTIF({res},\"PASS\")"), None)?;
        kpi(cx, "decathlon_pass", "Centenarian Decathlon tasks passed", TITLE, TOP + 3, 1, None)?;
        let mut chart = Chart::new(ChartType::Bar);
        chart
            .add_series()
            .set_name("% of capacity required today")
            .set_categories((TITLE, r0, 0, r1, 0))
            .set_values((TITLE, r0, 11, r1, 11));
        chart.title().set_name("Measured vs required today (100% = on track)");
        chart.x_axis().set_reverse();
        chart.legend().set_hidden();
        chart.set_width(620).set_height(420);
        ws.insert_chart(TOP, 14, &chart)?;
        set_widths(ws, &[36.0, 14.0, 10.0, 9.0, 11.0, 10.0, 10.0, 11.0, 11.0, 10.0, 12.0, 11.0, 9.0])?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_capacity_matches_the_book_check() {
        // 30 lb at 80 with 12%/decade -> ~50 lb at 40 (OUT ch31 via longevity.md L-C1).
        let required = 30.0 / (1.0_f64 - 0.12).powf(40.0 / 10.0);
        assert!((required - 50.0).abs() < 0.1, "{required}");
        assert_eq!(required_formula("E7", "H7", "G7"), "IF(OR(E7=\"\",H7=\"\",G7=\"\"),\"\",E7/(1-H7)^(G7/10))");
    }
}
