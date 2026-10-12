//! AGE-13 `Age_Zone2`: weekly zone 2 minutes against the 180-minute target,
//! rolling 4-week mean, streak, stacked by modality.
//!
//! GATE-FIELDS: `tblWorkout` is an exercise list (no date, duration or zone), so
//! sessions are logged here in `tblAgeCardio`; the view is a placeholder until rows arrive.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::{at, cell, head, kpi, list, rng, section, table, week_start, Banner, Col, LOG_ROWS, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Age_Zone2";
pub const MODALITIES: [&str; 6] = ["bike", "run", "row", "walk/ruck", "swim", "other"];
pub const ZONES: [&str; 4] = ["zone2", "below", "above", "intervals"];
const WEEKS: u32 = 26;
const W_HR: u32 = TOP + 1;
/// Weekly table columns: week, one per modality, total, target, met, rolling 4, streak.
const C_TOTAL: u16 = 1 + MODALITIES.len() as u16;
const C_MET: u16 = C_TOTAL + 2;
const C_ROLL: u16 = C_MET + 1;
const C_STREAK: u16 = C_ROLL + 1;
const LOG_C: u16 = C_STREAK + 2;
const LOG_HR: u32 = TOP + 1;

/// Zero-based row of the week `k` weeks back (k = 0 is the current week).
fn week_row(k: u32) -> u32 {
    W_HR + WEEKS - k
}

fn log(c: u16) -> String {
    rng(TITLE, LOG_C + c, LOG_HR + 1, LOG_HR + LOG_ROWS)
}

/// Every cardio minute (any zone) in the 7 days from `monday` (a formula expression).
pub fn cardio_minutes(monday: &str) -> String {
    format!("SUMIFS({m},{d},\">=\"&{monday},{d},\"<\"&({monday}+7))", m = log(2), d = log(0))
}

/// Zone 2 minutes of the last full week.
pub fn last_week_cell() -> String {
    cell(TITLE, week_row(1), C_TOTAL)
}

/// Rolling 4-week mean of zone 2 minutes, as of the last full week.
pub fn mean4_cell() -> String {
    cell(TITLE, week_row(1), C_ROLL)
}

/// Weeks (of the last `WEEKS`) that met the target.
pub fn weeks_met() -> String {
    format!("SUM({})", rng(TITLE, C_MET, W_HR + 1, W_HR + WEEKS))
}

pub struct Zone2;

impl SheetModule for Zone2 {
    fn id(&self) -> &'static str {
        "age-zone2"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "Zone 2 weekly volume",
            "AGE-13 (L-C5) | OUT ch33 (about 3 h/week; entry 2x30 min; Attia 4x1 h)",
            Banner::Awaiting(
                "cardio sessions: tblWorkout has no date/duration (GATE-FIELDS), log them in tblAgeCardio ->",
            ),
        )?;
        section(ws, cx, TOP, "Weeks (Monday start, oldest first): zone 2 minutes by modality")?;
        let (dl, ml, mins, zl) = (log(0), log(1), log(2), log(3));
        let mut cols = vec![Col::f("week_of", |r| format!("={}", week_start(W_HR + WEEKS + 1 - r)))];
        for m in MODALITIES {
            let (dl, ml, mins, zl) = (dl.clone(), ml.clone(), mins.clone(), zl.clone());
            cols.push(Col::f(m, move |r| {
                format!("=SUMIFS({mins},{dl},\">=\"&$A{r},{dl},\"<\"&($A{r}+7),{zl},\"zone2\",{ml},\"{m}\")")
            }));
        }
        cols.push(Col::f("total", |r| format!("=SUM({}:{})", at(1, r), at(C_TOTAL - 1, r))));
        cols.push(Col::f("target", |_| "=Age_Z2_target_min".into()));
        cols.push(Col::f("met", |r| format!("=IF({}>={},1,0)", at(C_TOTAL, r), at(C_TOTAL + 1, r))));
        cols.push(Col::f("rolling_4wk", |r| {
            let r0 = r.saturating_sub(3).max(W_HR + 2);
            format!("=AVERAGE({}:{})", at(C_TOTAL, r0), at(C_TOTAL, r))
        }));
        cols.push(Col::f("streak", |r| {
            if r == W_HR + 2 {
                format!("={}", at(C_MET, r))
            } else {
                format!("=IF({}=1,{}+1,0)", at(C_MET, r), at(C_STREAK, r - 1))
            }
        }));
        table(ws, cx, "tblAgeZone2Weeks", W_HR, 0, &cols, WEEKS)?;
        let (w0, w1) = (W_HR + 1, W_HR + WEEKS);
        for r in w0..=w1 {
            ws.set_cell_format(r, 0, &f.date)?;
            ws.set_cell_format(r, C_ROLL, &f.int)?;
        }
        theme::cf_data_bar(ws, w0, C_TOTAL, w1, C_TOTAL, theme::BAND_OPT)?;
        theme::cf_formula(ws, w0, C_MET, w1, C_MET, &format!("={}=1", at(C_MET, w0 + 1)), theme::OPTIMAL)?;
        theme::sparkline(ws, w1 + 1, C_TOTAL, &rng(TITLE, C_TOTAL, w0, w1))?;

        let sr = w1 + 3;
        super::kv(ws, cx, sr, 0, "Zone 2 minutes, last full week", &format!("={}", last_week_cell()), Some("0"))?;
        super::kv(ws, cx, sr + 1, 0, "Rolling 4-week mean", &format!("={}", mean4_cell()), Some("0"))?;
        super::kv(ws, cx, sr + 2, 0, "Current streak (weeks)", &format!("={}", cell(TITLE, w1, C_STREAK)), None)?;
        kpi(cx, "z2_week", "Zone 2 minutes last full week", TITLE, sr, 1, Some("0"))?;

        let mut chart = Chart::new(ChartType::ColumnStacked);
        for (i, m) in MODALITIES.iter().enumerate() {
            let c = 1 + i as u16;
            chart.add_series().set_name(*m).set_categories((TITLE, w0, 0, w1, 0)).set_values((TITLE, w0, c, w1, c));
        }
        let mut target = Chart::new(ChartType::Line);
        let tc = C_TOTAL + 1;
        target
            .add_series()
            .set_name("target")
            .set_categories((TITLE, w0, 0, w1, 0))
            .set_values((TITLE, w0, tc, w1, tc));
        chart.combine(&target);
        chart.title().set_name("Zone 2 minutes per week");
        chart.x_axis().set_num_format("mm-dd");
        chart.set_width(640).set_height(300);
        ws.insert_chart(sr + 4, 0, &chart)?;

        // Session log.
        ws.write_string_with_format(TOP, LOG_C, "Session log (typed)", &f.section)?;
        let cols = [
            Col::Date("date"),
            Col::In("modality"),
            Col::In("minutes"),
            Col::In("zone"),
            Col::In("avg_hr"),
            Col::In("avg_power_w"),
            Col::In("rpe"),
            Col::In("note"),
        ];
        table(ws, cx, "tblAgeCardio", LOG_HR, LOG_C, &cols, LOG_ROWS)?;
        list(ws, LOG_HR + 1, LOG_C + 1, LOG_HR + LOG_ROWS, &MODALITIES)?;
        list(ws, LOG_HR + 1, LOG_C + 3, LOG_HR + LOG_ROWS, &ZONES)?;
        let mut widths = vec![12.0; MODALITIES.len() + 1];
        widths.extend([8.0, 8.0, 6.0, 10.0, 7.0, 2.0, 12.0, 11.0, 9.0, 10.0, 8.0, 11.0, 6.0, 24.0]);
        set_widths(ws, &widths)?;
        Ok(())
    }
}
