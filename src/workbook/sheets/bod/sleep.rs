//! BOD-27 `Bod_Sleep` (L-E1, L-E2, L-E7): sleep opportunity, regularity, efficiency
//! and a 7-night debt ledger over a typed sleep log (`tblBodSleepLog`, one row per
//! night: bedtime, wake time, minutes asleep, awakenings).
//!
//! Row formulas: time in bed `MOD(wake-bed,1)*24` (crosses midnight), asleep hours,
//! efficiency = asleep / TIB (CF vs `Bod_Sleep_eff`), opportunity vs
//! `Bod_Sleep_min_h` / `Bod_Sleep_pref_h`, wake-time SD over the last
//! `Bod_Wake_SD_nights` logged nights (`STDEV.S` over an `INDEX:INDEX` window, minutes),
//! the 7-night ledger `SUM(Bod_Sleep_min_h - asleep)` and mid-sleep time. A summary
//! adds weekend drift, the `sleep_h` KPI and the `tblWearable` sleep fallback.
//!
//! PLACEHOLDER: the log and `tblWearable` are empty until nights are typed or a
//! wearable export arrives. This module also holds the helpers the other lane-D
//! sleep / wearable views share ([`pearson`], [`slope`], [`time_cells`], [`wear`]).

use rust_xlsxwriter::{DataValidation, DataValidationRule, Worksheet};

use super::{at, cell, head, input_time, kpi, kv, last_num, rng, section, table, Banner, Col, LOG_ROWS, TOP};
use crate::error::Result;
use crate::infra::data::WEARABLE;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Sleep";
const HR: u32 = TOP + 1;
const HEADS: &[&str] = &[
    "date",
    "bedtime",
    "wake",
    "minutes_asleep",
    "awakenings",
    "tib_h",
    "asleep_h",
    "efficiency",
    "opportunity",
    "wake_min",
    "wake_dow",
    "wake_sd_min",
    "shortfall_h",
    "debt_7n_h",
    "recovered",
    "mid_sleep",
    "mid_sleep_h",
    "tib_plot",
    "asleep_plot",
    "min_line",
];
const SUM_C: u16 = HEADS.len() as u16 + 1;

fn c(h: &str) -> u16 {
    HEADS.iter().position(|x| *x == h).unwrap_or_else(|| panic!("Bod_Sleep has no column {h}")) as u16
}

/// Absolute range of a sleep-log column (`date`, `bedtime`, `asleep_h`, `efficiency`, ...).
pub fn log(h: &str) -> String {
    rng(TITLE, c(h), HR + 1, HR + LOG_ROWS)
}

/// Mean of sleep-log column `h` on the night `night` (a date expression), or "".
pub fn on_night(h: &str, night: &str) -> String {
    format!("IFERROR(AVERAGEIFS({},{},{night}),\"\")", log(h), log("date"))
}

/// Habitual mid-sleep clock time (day fraction, "" until logged).
pub fn mid_sleep_cell() -> String {
    cell(TITLE, HR + 11, SUM_C + 1)
}

/// Mean mid-sleep in hours after midnight (negative before midnight), "" until logged.
pub fn mid_sleep_h_cell() -> String {
    cell(TITLE, HR + 10, SUM_C + 1)
}

// ------------------------------------------------------------ shared helpers

/// Absolute `Data_Wearable` range of column `h`.
pub fn wear(cx: &Cx, h: &str) -> String {
    WEARABLE.range(cx.d, h)
}

/// Mean `tblWearable` value of `h` on the date `day` (an expression), or "".
pub fn wear_on(cx: &Cx, h: &str, day: &str) -> String {
    format!("IFERROR(AVERAGEIFS({},{},{day}),\"\")", wear(cx, h), wear(cx, "date"))
}

fn sums(x: &str, y: &str) -> [String; 6] {
    let m = format!("ISNUMBER({x})*ISNUMBER({y})");
    [
        format!("SUMPRODUCT({m})"),
        format!("SUMPRODUCT({m},{x})"),
        format!("SUMPRODUCT({m},{y})"),
        format!("SUMPRODUCT({m},{x},{x})"),
        format!("SUMPRODUCT({m},{y},{y})"),
        format!("SUMPRODUCT({m},{x},{y})"),
    ]
}

/// Number of rows where both row-aligned ranges hold numbers.
pub fn n_pairs(x: &str, y: &str) -> String {
    format!("SUMPRODUCT(ISNUMBER({x})*ISNUMBER({y}))")
}

/// Pearson r over the rows where both ranges are numbers: SUMPRODUCT with ISNUMBER
/// masks (text such as "" counts as 0 in a SUMPRODUCT argument and is masked out),
/// so it works on formula columns where CORREL would see "". "" under 3 pairs.
pub fn pearson(x: &str, y: &str) -> String {
    let [n, sx, sy, sxx, syy, sxy] = sums(x, y);
    format!("IF({n}<3,\"\",IFERROR(({n}*{sxy}-{sx}*{sy})/SQRT(({n}*{sxx}-{sx}^2)*({n}*{syy}-{sy}^2)),\"\"))")
}

/// OLS slope of `y` on `x` over the numeric pairs ("" under 3 pairs).
pub fn slope(x: &str, y: &str) -> String {
    let [n, sx, sy, sxx, _, sxy] = sums(x, y);
    format!("IF({n}<3,\"\",IFERROR(({n}*{sxy}-{sx}*{sy})/({n}*{sxx}-{sx}^2),\"\"))")
}

/// Typed clock times on column `c`, zero-based rows `r0..=r1`: input fill, hh:mm, a 0..1 validation.
pub fn time_cells(ws: &mut Worksheet, r0: u32, r1: u32, c: u16) -> Result<()> {
    let f = input_time();
    for r in r0..=r1 {
        ws.write_blank(r, c, &f)?;
    }
    let dv = DataValidation::new()
        .allow_decimal_number(DataValidationRule::Between(0.0, 0.999_999))
        .set_input_message("clock time, e.g. 22:30")?;
    ws.add_data_validation(r0, c, r1, c, &dv)?;
    Ok(())
}

/// YES / NO drop-downs on zero-based columns `c0..=c1`, rows `r0..=r1`.
pub fn yes_no(ws: &mut Worksheet, r0: u32, r1: u32, c0: u16, c1: u16) -> Result<()> {
    for cc in c0..=c1 {
        super::list(ws, r0, cc, r1, &["YES", "NO"])?;
    }
    Ok(())
}

/// `x` when it is a number, else "" (a row-aligned mirror of a possibly blank cell).
pub fn num_or_blank(x: &str) -> String {
    format!("=IF(ISNUMBER({x}),{x},\"\")")
}

pub struct Sleep;

impl SheetModule for Sleep {
    fn id(&self) -> &'static str {
        "bod-sleep"
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
            "Sleep opportunity, wake-time SD, efficiency, 7-day debt",
            "BOD-27",
            "L-E1, L-E2, L-E7 | OUT ch60 rules 8-9 (8-9 h opportunity, fixed wake time); WWS ch21 tip 1; WWS ch06 (efficiency >= 90%)",
            Banner::Awaiting("nightly sleep log in tblBodSleepLog (bedtime, wake time, minutes asleep) or wearable sleep_hours / sleep_efficiency in tblWearable"),
        )?;
        section(ws, cx, TOP, 0, "Sleep log (one row per night; date = the evening the night starts)")?;
        let k0 = HR + 2; // Excel row of the first body row
        let a = |h: &str, r: u32| at(c(h), r);
        let win = |h: &'static str, n: &'static str, r: u32| {
            let (col, k) = (log(h), r + 1 - k0);
            format!("INDEX({col},MAX(1,{k}-{n}+1)):INDEX({col},{k})")
        };
        let cols = vec![
            Col::Date("date"),
            Col::In("bedtime"),
            Col::In("wake"),
            Col::In("minutes_asleep"),
            Col::In("awakenings"),
            Col::f("tib_h", |r| {
                let (b, w) = (a("bedtime", r), a("wake", r));
                format!("=IF(AND(ISNUMBER({b}),ISNUMBER({w})),MOD({w}-{b},1)*24,\"\")")
            }),
            Col::f("asleep_h", |r| format!("=IF(ISNUMBER({m}),{m}/60,\"\")", m = a("minutes_asleep", r))),
            Col::f("efficiency", |r| {
                let (s, t) = (a("asleep_h", r), a("tib_h", r));
                format!("=IF(AND(ISNUMBER({s}),ISNUMBER({t})),IF({t}>0,{s}/{t},\"\"),\"\")")
            }),
            Col::f("opportunity", |r| {
                let t = a("tib_h", r);
                format!("=IF(ISNUMBER({t}),IF({t}<Bod_Sleep_min_h,\"short\",IF({t}<Bod_Sleep_pref_h,\"8h+\",\"9h+\")),\"\")")
            }),
            Col::f("wake_min", |r| format!("=IF(ISNUMBER({w}),{w}*1440,\"\")", w = a("wake", r))),
            Col::f("wake_dow", |r| format!("=IF(ISNUMBER({d}),WEEKDAY({d}+1,2),\"\")", d = a("date", r))),
            Col::f("wake_sd_min", |r| {
                let w = win("wake_min", "Bod_Wake_SD_nights", r);
                format!("=IF(ISNUMBER({}),IF(COUNT({w})>=2,STDEV.S({w}),\"\"),\"\")", a("wake_min", r))
            }),
            Col::f("shortfall_h", |r| format!("=IF(ISNUMBER({s}),Bod_Sleep_min_h-{s},\"\")", s = a("asleep_h", r))),
            Col::f("debt_7n_h", |r| {
                format!("=IF(ISNUMBER({}),SUM({}),\"\")", a("shortfall_h", r), win("shortfall_h", "7", r))
            }),
            Col::f("recovered", |r| {
                format!("=IF(ISNUMBER({d}),IF({d}<=0,\"YES\",\"NO\"),\"\")", d = a("debt_7n_h", r))
            }),
            Col::f("mid_sleep", |r| {
                let (b, t) = (a("bedtime", r), a("tib_h", r));
                format!("=IF(ISNUMBER({t}),MOD({b}+{t}/48,1),\"\")")
            }),
            Col::f("mid_sleep_h", |r| {
                format!("=IF(ISNUMBER({m}),(MOD({m}+0.5,1)-0.5)*24,\"\")", m = a("mid_sleep", r))
            }),
            super::plot_col("tib_plot", c("tib_h")),
            super::plot_col("asleep_plot", c("asleep_h")),
            Col::f("min_line", |r| format!("=IF(ISNUMBER({}),Bod_Sleep_min_h,NA())", a("date", r))),
        ];
        table(ws, cx, "tblBodSleepLog", HR, 0, &cols, LOG_ROWS)?;
        let (r0, r1) = (HR + 1, HR + LOG_ROWS);
        time_cells(ws, r0, r1, c("bedtime"))?;
        time_cells(ws, r0, r1, c("wake"))?;
        super::fmt_rows(ws, r0, r1, c("tib_h"), c("asleep_h"), "0.00")?;
        super::fmt_rows(ws, r0, r1, c("efficiency"), c("efficiency"), "0%")?;
        super::fmt_rows(ws, r0, r1, c("wake_min"), c("wake_min"), "0")?;
        super::fmt_rows(ws, r0, r1, c("wake_sd_min"), c("wake_sd_min"), "0")?;
        super::fmt_rows(ws, r0, r1, c("shortfall_h"), c("debt_7n_h"), "+0.0;-0.0;0.0")?;
        super::fmt_rows(ws, r0, r1, c("mid_sleep"), c("mid_sleep"), "hh:mm")?;
        super::fmt_rows(ws, r0, r1, c("mid_sleep_h"), c("min_line"), "0.00")?;
        let tl = |h: &str| at(c(h), r0 + 1);
        let (e, t, s, d) = (tl("efficiency"), tl("tib_h"), tl("wake_sd_min"), tl("debt_7n_h"));
        let ce = c("efficiency");
        theme::cf_formula(ws, r0, ce, r1, ce, &format!("=AND(ISNUMBER({e}),{e}<Bod_Sleep_eff)"), theme::OUT_OF_REF)?;
        theme::cf_formula(ws, r0, ce, r1, ce, &format!("=AND(ISNUMBER({e}),{e}>=Bod_Sleep_eff)"), theme::OPTIMAL)?;
        let ct = c("tib_h");
        theme::cf_formula(ws, r0, ct, r1, ct, &format!("=AND(ISNUMBER({t}),{t}<Bod_Sleep_min_h)"), theme::OUT_OF_REF)?;
        let cs = c("wake_sd_min");
        let sd_rule = format!("=AND(ISNUMBER({s}),{s}>Bod_Wake_SD_max_min)");
        theme::cf_formula(ws, r0, cs, r1, cs, &sd_rule, theme::OUT_OF_REF)?;
        let cd = c("debt_7n_h");
        theme::cf_formula(ws, r0, cd, r1, cd, &format!("=AND(ISNUMBER({d}),{d}>0)"), theme::REF_OK)?;

        // Summary.
        let (sc, dates) = (SUM_C, log("date"));
        let last7 = |h: &str| format!("=IFERROR(AVERAGEIFS({},{dates},\">\"&(MAX({dates})-7)),\"\")", log(h));
        section(ws, cx, TOP, sc, "Summary (last 7 logged nights)")?;
        let rows: Vec<(&str, String, Option<&str>)> = vec![
            ("Nights logged", format!("=COUNT({dates})"), None),
            ("Mean asleep (h)", last7("asleep_h"), Some("0.00")),
            ("Mean time in bed (h)", last7("tib_h"), Some("0.00")),
            ("Mean efficiency", last7("efficiency"), Some("0%")),
            ("Nights in bed < Bod_Sleep_min_h", format!("=COUNTIFS({},\"<\"&Bod_Sleep_min_h)", log("tib_h")), None),
            ("Wake-time SD, latest (min)", format!("={}", last_num(&log("wake_sd_min"))), Some("0")),
            (
                "Weekend drift: Sat/Sun wake minus Mon-Fri (min)",
                format!(
                    "=IFERROR(AVERAGEIFS({w},{d},\">=6\")-AVERAGEIFS({w},{d},\"<=5\"),\"\")",
                    w = log("wake_min"),
                    d = log("wake_dow")
                ),
                Some("+0;-0;0"),
            ),
            ("7-night debt, latest (h)", format!("={}", last_num(&log("debt_7n_h"))), Some("+0.0;-0.0;0.0")),
            ("Recovered?", format!("=IF(ISNUMBER({x}),IF({x}<=0,\"YES\",\"NO\"),\"\")", x = abs_s(HR + 7)), None),
            (
                "Wake SD flag",
                format!("=IF(ISNUMBER({x}),IF({x}>Bod_Wake_SD_max_min,\"irregular\",\"ok\"),\"\")", x = abs_s(HR + 5)),
                None,
            ),
            (
                "Mid-sleep, mean (h after midnight)",
                format!("=IFERROR(AVERAGE({}),\"\")", log("mid_sleep_h")),
                Some("0.00"),
            ),
            (
                "Mid-sleep, mean (clock)",
                format!("=IF(ISNUMBER({x}),MOD({x}/24,1),\"\")", x = abs_s(HR + 10)),
                Some("hh:mm"),
            ),
        ];
        for (i, (label, f, nf)) in rows.iter().enumerate() {
            kv(ws, cx, TOP + 1 + i as u32, sc, label, f, *nf)?;
        }
        kpi(cx, "sleep_h", "Mean sleep (h), last 7 logged nights", TITLE, TOP + 2, sc + 1, Some("0.0"))?;

        let wr = TOP + 15;
        section(ws, cx, wr, sc, "Wearable fallback (tblWearable, last 7 days)")?;
        let (wd, wh, we) = (wear(cx, "date"), wear(cx, "sleep_hours"), wear(cx, "sleep_efficiency"));
        let w7 = |x: &str| format!("=IFERROR(AVERAGEIFS({x},{wd},\">\"&(MAX({wd})-7)),\"\")");
        kv(ws, cx, wr + 1, sc, "Wearable nights", &format!("=COUNT({wh})"), None)?;
        kv(ws, cx, wr + 2, sc, "Wearable sleep (h)", &w7(&wh), Some("0.00"))?;
        kv(ws, cx, wr + 3, sc, "Wearable efficiency", &w7(&we), Some("0%"))?;
        kv(
            ws,
            cx,
            wr + 4,
            sc,
            "Best available sleep (h)",
            &format!("=IF(ISNUMBER({l}),{l},{w})", l = abs_s(HR + 1), w = super::abs(sc + 1, wr + 2)),
            Some("0.00"),
        )?;

        let chart = super::line_chart(
            TITLE,
            "Time in bed and asleep vs the 8 h opportunity",
            r0,
            r1,
            0,
            &[("time in bed (h)", c("tib_plot")), ("asleep (h)", c("asleep_plot")), ("Bod_Sleep_min_h", c("min_line"))],
        );
        ws.insert_chart(wr + 7, sc, &chart)?;
        let mut w =
            vec![11.0, 8.0, 8.0, 9.0, 8.0, 7.0, 8.0, 9.0, 10.0, 8.0, 8.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 7.0, 7.0, 7.0];
        w.extend([2.0, 40.0, 10.0]);
        set_widths(ws, &w)?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}

/// Absolute reference to the summary value of zero-based row `r` (its value column).
fn abs_s(r: u32) -> String {
    super::abs(SUM_C + 1, r)
}
