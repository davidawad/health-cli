//! BOD-28 `Bod_Arch` (L-E3): sleep architecture by night.
//!
//! Typed stage minutes per night (`tblBodStages`: light, deep, REM, awake) give the
//! asleep and in-bed minutes, stage shares, expected cycles = TIB / `Bod_Cycle_min`,
//! and a personal baseline: the mean deep / REM share over the previous
//! `Bod_Arch_base_nights` logged nights and tonight's deviation from it (no population
//! percentages: none were captured from the sources). A block reads
//! `tblWearable.sleep_deep_h` / `sleep_rem_h` when a wearable export arrives.
//! Trackers estimate stages "reasonably but not perfectly" (OUT ch58 l.25).
//!
//! PLACEHOLDER: the stage log and `tblWearable` are empty until typed / imported.

use rust_xlsxwriter::{Chart, ChartType, DataValidation, DataValidationRule, Worksheet};

use super::sleep::wear;
use super::{at, head, kv, last_num, rng, section, table, Banner, Col, LOG_ROWS, TOP};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Arch";
const HR: u32 = TOP + 1;
const HEADS: &[&str] = &[
    "date",
    "light_min",
    "deep_min",
    "rem_min",
    "awake_min",
    "asleep_min",
    "tib_min",
    "light_pct",
    "deep_pct",
    "rem_pct",
    "cycles",
    "deep_base",
    "deep_dev",
    "rem_base",
    "rem_dev",
];
const SUM_C: u16 = HEADS.len() as u16 + 1;

fn c(h: &str) -> u16 {
    HEADS.iter().position(|x| *x == h).unwrap_or_else(|| panic!("tblBodStages has no column {h}")) as u16
}

fn log(h: &str) -> String {
    rng(TITLE, c(h), HR + 1, HR + LOG_ROWS)
}

pub struct Arch;

impl SheetModule for Arch {
    fn id(&self) -> &'static str {
        "bod-arch"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        &["data-wearable"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        head(
            ws,
            cx,
            "Sleep architecture stages, expected cycles",
            "BOD-28",
            "L-E3 | WWS l.1192-1210 (90-min cycles; deep NREM early, REM late); OUT ch58 l.25 (trackers: reasonable, not perfect)",
            Banner::Awaiting("stage minutes per night in tblBodStages, or sleep_deep_h / sleep_rem_h in tblWearable"),
        )?;
        section(ws, cx, TOP, 0, "Stages per night (typed minutes)")?;
        let k0 = HR + 2;
        let a = |h: &str, r: u32| at(c(h), r);
        let share = |h: &'static str| {
            move |r: u32| {
                let (x, s) = (at(c(h), r), at(c("asleep_min"), r));
                format!("=IF(AND(ISNUMBER({s}),ISNUMBER({x})),IF({s}>0,{x}/{s},\"\"),\"\")")
            }
        };
        let base = |h: &'static str| {
            move |r: u32| {
                let k = r + 1 - k0;
                if k == 1 {
                    return "=\"\"".to_string();
                }
                let col = log(h);
                let w = format!("INDEX({col},MAX(1,{k}-Bod_Arch_base_nights)):INDEX({col},{})", k - 1);
                format!("=IF(ISNUMBER({}),IF(COUNT({w})=0,\"\",AVERAGE({w})),\"\")", at(c(h), r))
            }
        };
        let dev = |h: &'static str, b: &'static str| {
            move |r: u32| {
                let (x, y) = (at(c(h), r), at(c(b), r));
                format!("=IF(AND(ISNUMBER({x}),ISNUMBER({y})),{x}-{y},\"\")")
            }
        };
        let cols = vec![
            Col::Date("date"),
            Col::In("light_min"),
            Col::In("deep_min"),
            Col::In("rem_min"),
            Col::In("awake_min"),
            Col::f("asleep_min", |r| {
                let (l, m) = (a("light_min", r), a("rem_min", r));
                format!("=IF(COUNT({l}:{m})=0,\"\",SUM({l}:{m}))")
            }),
            Col::f("tib_min", |r| {
                format!("=IF(ISNUMBER({s}),{s}+N({w}),\"\")", s = a("asleep_min", r), w = a("awake_min", r))
            }),
            Col::f("light_pct", share("light_min")),
            Col::f("deep_pct", share("deep_min")),
            Col::f("rem_pct", share("rem_min")),
            Col::f("cycles", |r| format!("=IF(ISNUMBER({t}),{t}/Bod_Cycle_min,\"\")", t = a("tib_min", r))),
            Col::f("deep_base", base("deep_pct")),
            Col::f("deep_dev", dev("deep_pct", "deep_base")),
            Col::f("rem_base", base("rem_pct")),
            Col::f("rem_dev", dev("rem_pct", "rem_base")),
        ];
        table(ws, cx, "tblBodStages", HR, 0, &cols, LOG_ROWS)?;
        let (r0, r1) = (HR + 1, HR + LOG_ROWS);
        super::fmt_rows(ws, r0, r1, c("light_pct"), c("rem_pct"), "0%")?;
        super::fmt_rows(ws, r0, r1, c("cycles"), c("cycles"), "0.0")?;
        super::fmt_rows(ws, r0, r1, c("deep_base"), c("rem_dev"), "+0.0%;-0.0%;0.0%")?;
        for h in ["deep_dev", "rem_dev"] {
            theme::cf_three_color(ws, r0, c(h), r1, c(h), false)?;
        }
        theme::cf_data_bar(ws, r0, c("cycles"), r1, c("cycles"), theme::BAND_OPT)?;
        let minutes = DataValidation::new()
            .allow_decimal_number(DataValidationRule::Between(0.0, 900.0))
            .set_input_message("minutes in this stage")?;
        ws.add_data_validation(r0, c("light_min"), r1, c("awake_min"), &minutes)?;

        let sc = SUM_C;
        let mean = |h: &str| format!("=IFERROR(AVERAGE({}),\"\")", log(h));
        section(ws, cx, TOP, sc, "Summary (stage log)")?;
        let rows: Vec<(&str, String, Option<&str>)> = vec![
            ("Nights logged", format!("=COUNT({})", log("asleep_min")), None),
            ("Mean light share", mean("light_pct"), Some("0%")),
            ("Mean deep share", mean("deep_pct"), Some("0%")),
            ("Mean REM share", mean("rem_pct"), Some("0%")),
            ("Mean expected cycles (TIB / Bod_Cycle_min)", mean("cycles"), Some("0.0")),
            ("Deep share vs baseline, latest", format!("={}", last_num(&log("deep_dev"))), Some("+0.0%;-0.0%")),
            ("REM share vs baseline, latest", format!("={}", last_num(&log("rem_dev"))), Some("+0.0%;-0.0%")),
        ];
        for (i, (label, f, nf)) in rows.iter().enumerate() {
            kv(ws, cx, TOP + 1 + i as u32, sc, label, f, *nf)?;
        }
        let wr = TOP + 9;
        section(ws, cx, wr, sc, "Wearable stages (tblWearable)")?;
        let (dp, rm, sh) = (wear(cx, "sleep_deep_h"), wear(cx, "sleep_rem_h"), wear(cx, "sleep_hours"));
        let share_w = |x: &str| {
            let m = format!("ISNUMBER({x})*ISNUMBER({sh})");
            format!("=IFERROR(SUMPRODUCT({m},{x})/SUMPRODUCT({m},{sh}),\"\")")
        };
        kv(ws, cx, wr + 1, sc, "Nights with stages", &format!("=COUNT({dp})"), None)?;
        kv(ws, cx, wr + 2, sc, "Mean deep (h)", &format!("=IFERROR(AVERAGE({dp}),\"\")"), Some("0.00"))?;
        kv(ws, cx, wr + 3, sc, "Mean REM (h)", &format!("=IFERROR(AVERAGE({rm}),\"\")"), Some("0.00"))?;
        kv(ws, cx, wr + 4, sc, "Deep share of sleep", &share_w(&dp), Some("0%"))?;
        kv(ws, cx, wr + 5, sc, "REM share of sleep", &share_w(&rm), Some("0%"))?;
        kv(
            ws,
            cx,
            wr + 6,
            sc,
            "Expected cycles (mean sleep h / Bod_Cycle_min)",
            &format!("=IFERROR(AVERAGE({sh})*60/Bod_Cycle_min,\"\")"),
            Some("0.0"),
        )?;
        ws.write_string_with_format(
            wr + 8,
            sc,
            "Deep NREM dominates the early cycles, REM the late ones (WWS l.1192-1210): a short night cuts REM most. Trackers estimate stages reasonably but not perfectly (OUT ch58 l.25).",
            &cx.f.note,
        )?;

        let mut chart = Chart::new(ChartType::ColumnStacked);
        for h in ["light_min", "deep_min", "rem_min", "awake_min"] {
            chart.add_series().set_name(h.trim_end_matches("_min")).set_categories((TITLE, r0, 0, r1, 0)).set_values((
                TITLE,
                r0,
                c(h),
                r1,
                c(h),
            ));
        }
        chart.title().set_name("Sleep stages per night (min)");
        chart.x_axis().set_num_format("mm-dd");
        chart.set_width(720).set_height(320);
        ws.insert_chart(wr + 10, sc, &chart)?;
        let mut w = vec![11.0, 9.0, 9.0, 9.0, 9.0, 10.0, 8.0, 9.0, 9.0, 8.0, 7.0, 9.0, 9.0, 9.0, 9.0];
        w.extend([2.0, 40.0, 10.0]);
        set_widths(ws, &w)?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
