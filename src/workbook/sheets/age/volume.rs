//! AGE-16 `Age_Volume`: training volume. Weekly hard sets per muscle against
//! editable MEV / MAV / MRV landmarks, a mesocycle ramp planner, muscle
//! frequency, lifting sessions per week and the cardio : lifting time ratio.
//!
//! GATE-FIELDS: `tblWorkout` lists exercises without dates, sets or RIR, so sets
//! are logged here in `tblAgeLift` (exercise names offered from `tblWorkout`).

use rust_xlsxwriter::{Chart, ChartType, DataValidation, DataValidationRule, Formula, Worksheet};

use super::{abs, at, head, kpi, list, rng, section, table, week_start, zone2, Banner, Col, LOG_ROWS, TOP};
use crate::error::Result;
use crate::infra::data::WORKOUT;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Age_Volume";
const MUSCLES: [&str; 10] =
    ["chest", "back", "shoulders", "biceps", "triceps", "quads", "hamstrings", "glutes", "calves", "abs"];
const S_HR: u32 = TOP + 1;
const L_HR: u32 = S_HR + 9;
const M_HR: u32 = L_HR + MUSCLES.len() as u32 + 4;
const LOG_C: u16 = 13;
const LOG_HR: u32 = TOP + 1;

fn log(c: u16) -> String {
    rng(TITLE, LOG_C + c, LOG_HR + 1, LOG_HR + LOG_ROWS)
}

/// Hard sets for the muscle in `m` during the week starting `monday`.
fn sets_in_week(m: &str, monday: &str) -> String {
    format!("SUMIFS({h},{mu},{m},{d},\">=\"&{monday},{d},\"<\"&({monday}+7))", h = log(9), mu = log(2), d = log(0))
}

pub struct Volume;

impl SheetModule for Volume {
    fn id(&self) -> &'static str {
        "age-volume"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Training volume: hard sets per muscle vs landmarks",
            "AGE-16 (L-C12, L-C13, L-C14, L-C16, L-C20) | ISRA ch03, ch06; SCHO ch09, ch12; HELMS ch01 l.1047-1061; OUT ch30 l.128",
            Banner::Awaiting("lifting sets: tblWorkout has no date/sets/RIR (GATE-FIELDS), log them in tblAgeLift ->"),
        )?;
        let (dates, first, minutes) = (log(0), log(10), log(8));

        // Weekly summary.
        section(ws, cx, TOP, "This week and last")?;
        let (w0, w1) = (week_start(0), week_start(1));
        let sessions = |w: &str| format!("SUMIFS({first},{dates},\">=\"&{w},{dates},\"<\"&({w}+7))");
        let lift_min = |w: &str| format!("SUMIFS({minutes},{dates},\">=\"&{w},{dates},\"<\"&({w}+7))");
        let rows: [(&str, String, String, Option<&str>); 4] = [
            ("Lifting sessions (vs Age_Lift_sessions)", sessions(&w0), sessions(&w1), None),
            ("Lifting minutes", lift_min(&w0), lift_min(&w1), Some("0")),
            ("Cardio minutes (Age_Zone2 log)", zone2::cardio_minutes(&w0), zone2::cardio_minutes(&w1), Some("0")),
            ("Cardio : lifting ratio (max Age_Cardio_lift_max)", String::new(), String::new(), Some("0.00")),
        ];
        ws.write_string_with_format(S_HR, 1, "this week", &f.header)?;
        ws.write_string_with_format(S_HR, 2, "last week", &f.header)?;
        for (i, (label, a, b, nf)) in rows.iter().enumerate() {
            let r = S_HR + 1 + i as u32;
            ws.write_string_with_format(r, 0, *label, &f.bold)?;
            for (c, formula) in [(1u16, a), (2, b)] {
                let fx = if formula.is_empty() {
                    format!("=IF(N({l})=0,\"\",{c2}/{l})", l = at(c, S_HR + 3), c2 = at(c, S_HR + 4))
                } else {
                    format!("={formula}")
                };
                match nf {
                    Some(n) => ws.write_formula_with_format(
                        r,
                        c,
                        fx.as_str(),
                        &rust_xlsxwriter::Format::new().set_num_format(*n),
                    )?,
                    None => ws.write_formula(r, c, fx.as_str())?,
                };
            }
        }
        theme::cf_formula(
            ws,
            S_HR + 4,
            1,
            S_HR + 4,
            2,
            &format!("=AND(ISNUMBER(B{0}),B{0}>Age_Cardio_lift_max)", S_HR + 5),
            theme::OUT_OF_REF,
        )?;
        kpi(cx, "sessions_week", "Lifting sessions last full week", TITLE, S_HR + 1, 2, None)?;

        // Landmarks.
        section(
            ws,
            cx,
            L_HR - 1,
            "Landmarks per muscle (MEV / MAV / MRV are yours to set; ISRA gives definitions, not numbers)",
        )?;
        let (mu, dd) = (log(2), log(0));
        let cols = [
            Col::In("muscle"),
            Col::In("mev"),
            Col::In("mav"),
            Col::In("mrv"),
            Col::f("this_week", |r| format!("={}", sets_in_week(&format!("$A{r}"), &w0))),
            Col::f("last_week", |r| format!("={}", sets_in_week(&format!("$A{r}"), &w1))),
            Col::f("avg_4wk", |r| {
                let s: Vec<String> = (1..=4).map(|k| sets_in_week(&format!("$A{r}"), &week_start(k))).collect();
                format!("=({})/4", s.join("+"))
            }),
            Col::f("status", |r| {
                format!("=IF(COUNT(B{r}:D{r})<3,\"set landmarks\",IF(G{r}<B{r},\"below MEV\",IF(G{r}<=C{r},\"MEV-MAV\",IF(G{r}<=D{r},\"MAV-MRV\",\"above MRV\"))))")
            }),
            Col::f("days_hit_this_week", |r| {
                format!("=SUMPRODUCT(({mu}=$A{r})*({dd}>={w0})*({dd}<{w0}+7)/(COUNTIFS({mu},{mu},{dd},{dd})+({mu}<>$A{r})))")
            }),
        ];
        table(ws, cx, "tblAgeLandmarks", L_HR, 0, &cols, MUSCLES.len() as u32)?;
        let (l0, l1) = (L_HR + 1, L_HR + MUSCLES.len() as u32);
        for (i, m) in MUSCLES.iter().enumerate() {
            ws.write_string(l0 + i as u32, 0, *m)?;
            ws.set_cell_format(l0 + i as u32, 6, &f.dec1)?;
        }
        for (status, fill) in [
            ("below MEV", theme::OUT_OF_REF),
            ("MEV-MAV", theme::OPTIMAL),
            ("MAV-MRV", theme::REF_OK),
            ("above MRV", theme::CRITICAL),
        ] {
            theme::cf_formula(ws, l0, 7, l1, 7, &format!("={}=\"{status}\"", at(7, l0 + 1)), fill)?;
        }
        let mut chart = Chart::new(ChartType::Bar);
        for (name, c) in [("avg hard sets / week", 6u16), ("MEV", 1), ("MRV", 3)] {
            chart.add_series().set_name(name).set_categories((TITLE, l0, 0, l1, 0)).set_values((TITLE, l0, c, l1, c));
        }
        chart.title().set_name("Hard sets per muscle vs landmarks");
        chart.x_axis().set_reverse();
        chart.set_width(520).set_height(320);
        ws.insert_chart(M_HR + MUSCLES.len() as u32 + 2, 0, &chart)?;

        // Mesocycle planner.
        section(
            ws,
            cx,
            M_HR - 1,
            "Mesocycle ramp: MEV + Age_Meso_step sets per week, capped at MRV; week 6 deloads to half of MEV",
        )?;
        let mut cols = vec![Col::f("muscle", |r| format!("=A{}", r - M_HR + L_HR))];
        let wk = ["week1", "week2", "week3", "week4", "week5", "deload"];
        for (k, h) in wk.iter().enumerate() {
            cols.push(Col::f(h, move |r| {
                let lr = r - M_HR + L_HR;
                if k == 5 {
                    format!("=IF(ISNUMBER(B{lr}),ROUND(B{lr}/2,0),\"\")")
                } else {
                    format!("=IF(AND(ISNUMBER(B{lr}),ISNUMBER(D{lr})),MIN(D{lr},B{lr}+Age_Meso_step*{k}),\"\")")
                }
            }));
        }
        table(ws, cx, "tblAgeMeso", M_HR, 0, &cols, MUSCLES.len() as u32)?;

        // Lift log.
        ws.write_string_with_format(
            TOP,
            LOG_C,
            "Lift log (typed): a set counts as hard within Age_HardSet_RIR reps of failure (blank RIR counts)",
            &f.section,
        )?;
        let nw = WORKOUT.nrows(d);
        let (wn, wm) = (WORKOUT.range_n(nw, "name"), WORKOUT.range_n(nw, "muscles"));
        let first_row = LOG_HR + 2;
        let cols = [
            Col::Date("date"),
            Col::In("exercise"),
            Col::In("muscle"),
            Col::f("workout_muscles", |r| {
                format!("=IF({e}=\"\",\"\",IFERROR(INDEX({wm},MATCH({e},{wn},0)),\"\"))", e = at(LOG_C + 1, r))
            }),
            Col::In("sets"),
            Col::In("reps"),
            Col::In("load"),
            Col::In("rir"),
            Col::In("minutes"),
            Col::f("hard_sets", |r| {
                let (s, rir) = (at(LOG_C + 4, r), at(LOG_C + 7, r));
                format!("=IF(NOT(ISNUMBER({s})),0,IF(OR({rir}=\"\",N({rir})<=Age_HardSet_RIR),{s},0))")
            }),
            Col::f("first_of_day", |r| {
                let dcell = at(LOG_C, r);
                format!("=IF({dcell}=\"\",0,IF(COUNTIF({}:{dcell},{dcell})=1,1,0))", abs(LOG_C, first_row - 1))
            }),
        ];
        table(ws, cx, "tblAgeLift", LOG_HR, LOG_C, &cols, LOG_ROWS)?;
        let (g0, g1) = (LOG_HR + 1, LOG_HR + LOG_ROWS);
        ws.add_data_validation(
            g0,
            LOG_C + 1,
            g1,
            LOG_C + 1,
            &DataValidation::new()
                .allow_list_formula(Formula::new(format!("={wn}")))
                .set_error_style(rust_xlsxwriter::DataValidationErrorStyle::Information),
        )?;
        list(ws, g0, LOG_C + 2, g1, &MUSCLES)?;
        ws.add_data_validation(
            g0,
            LOG_C + 7,
            g1,
            LOG_C + 7,
            &DataValidation::new().allow_whole_number(DataValidationRule::Between(0, 10)),
        )?;
        set_widths(ws, &[40.0, 10.0, 10.0, 8.0, 10.0, 10.0, 9.0, 13.0, 12.0])?;
        Ok(())
    }
}
