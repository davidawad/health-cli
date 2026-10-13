//! BOD-30 `Bod_SleepHyg` (L-E5, L-E6, L-E8, L-E9, L-E13, C11.5): sleep-hygiene scorecard,
//! timing gaps, sleep x glucose coupling, chronotype, STOP-BANG and PSQI slots, and a
//! sleep / recovery quick panel.
//!
//! * `tblBodHygiene`: YES/NO per night for Outlive's 10 rules (OUT ch60), score = YESes.
//! * A per-day table (28 days up to the last typed date) joins: score, sleep hours
//!   (`Bod_Sleep` log, else `tblWearable` of the next date), next-day HRV and efficiency,
//!   last-meal-to-bed gap (`Bod_Window` log, vs `Bod_Last_meal_gap_h`), last-caffeine gap
//!   (`Bod_Caffeine` log) and the next morning's first CGM reading (`Bod_Glycemic`).
//!   Pairwise r / slope by SUMPRODUCT with ISNUMBER masks.
//! * Supplement time-of-day flags from `tblSupp` (COUNTIFS on the evening slots).
//! * Chronotype card from the `Bod_Sleep` mean mid-sleep vs `Bod_Chrono_*_h`.
//! * STOP-BANG: 8 YES/NO items, count, neutral `theme::dx_risk` wording at
//!   `Bod_STOPBANG_flag` (UNVERIFIED, Chung 2008). PSQI: a typed slot.
//!
//! PLACEHOLDER: the scorecard and the logs it joins are empty until typed.

use rust_xlsxwriter::{DataValidation, DataValidationRule, Worksheet};

use super::sleep::{self, n_pairs, pearson, slope, wear, wear_on, yes_no};
use super::{at, caffeine, glycemic, head, kv, last_num, rng, section, table, window, Banner, Col, LOG_ROWS, TOP};
use crate::error::Result;
use crate::infra::data::SUPP;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_SleepHyg";
pub const RULES: [&str; 10] = [
    "no_alcohol",
    "no_food_3h",
    "no_screens_2h",
    "wind_down_1h",
    "sauna_or_bath",
    "cool_room_mid60sF",
    "fully_dark",
    "opportunity_8_9h",
    "fixed_wake",
    "no_clock_watch",
];
const STOPBANG: [&str; 8] = [
    "Snoring loudly",
    "Tired / sleepy in the day",
    "Observed stopped breathing",
    "High blood Pressure (treated or not)",
    "BMI > 35",
    "Age > 50",
    "Neck > 40 cm",
    "Gender male",
];
const HR: u32 = TOP + 2;
const DAYS: u32 = 28;
const PD_C: u16 = 13;
const SUM_C: u16 = PD_C + 9;

pub struct Hygiene;

impl SheetModule for Hygiene {
    fn id(&self) -> &'static str {
        "bod-hygiene"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        &["data-wearable", "data-supp", "bod-sleep", "bod-window", "bod-caffeine", "bod-glycemic"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "Sleep-hygiene scorecard, gaps, chronotype, STOP-BANG slot",
            "BOD-30",
            "L-E5, L-E6, L-E8, L-E9, L-E13, C11.5 | OUT ch60 (10 rules), ch58 l.25-27, l.55, ch49 l.13; WWS ch03, ch21 (12 tips)",
            Banner::Awaiting("nightly YES/NO scorecard in tblBodHygiene, plus the Bod_Sleep, Bod_Window, Bod_Caffeine, Bod_Glycemic logs and tblWearable"),
        )?;
        let (r0, r1) = (HR + 1, HR + LOG_ROWS);
        let hd = rng(TITLE, 0, r0, r1);
        let hs = rng(TITLE, 11, r0, r1);
        let (sd, md) = (sleep::log("date"), window::meal_log("date"));
        kv(
            ws,
            cx,
            TOP,
            0,
            "Anchor day",
            &format!("=IF(COUNT({hd},{sd},{md})=0,Today,MAX({hd},{sd},{md}))"),
            Some(theme::nf::DATE),
        )?;
        let anchor = super::abs(1, TOP);

        section(ws, cx, HR - 1, 0, "Scorecard (Outlive's 10 rules, YES/NO per night)")?;
        let mut cols = vec![Col::Date("date")];
        cols.extend(RULES.iter().map(|h| Col::In(h)));
        cols.push(Col::f("score", |r| {
            format!("=IF(ISNUMBER({}),COUNTIF({}:{},\"YES\"),\"\")", at(0, r), at(1, r), at(10, r))
        }));
        table(ws, cx, "tblBodHygiene", HR, 0, &cols, LOG_ROWS)?;
        yes_no(ws, r0, r1, 1, 10)?;
        theme::cf_data_bar(ws, r0, 11, r1, 11, theme::BAND_OPT)?;

        // Per-day joins.
        section(ws, cx, HR - 1, PD_C, "Last 28 days (night of the date)")?;
        let p = |k: u16, r: u32| at(PD_C + k, r);
        let next = |r: u32| format!("({}+1)", p(0, r));
        let cols = vec![
            Col::f("day", |r| format!("={anchor}-{}", HR + 1 + DAYS - r)),
            Col::f("score", |r| format!("=IF(COUNTIFS({hd},{d})=0,\"\",SUMIFS({hs},{hd},{d}))", d = p(0, r))),
            Col::f("sleep_h", |r| {
                let (l, w) = (sleep::on_night("asleep_h", &p(0, r)), wear_on(cx, "sleep_hours", &next(r)));
                format!("=IF(ISNUMBER({l}),{l},{w})")
            }),
            Col::f("next_hrv", |r| format!("={}", wear_on(cx, "hrv", &next(r)))),
            Col::f("next_eff", |r| {
                let (w, l) = (wear_on(cx, "sleep_efficiency", &next(r)), sleep::on_night("efficiency", &p(0, r)));
                format!("=IF(ISNUMBER({w}),{w},{l})")
            }),
            Col::f("meal_gap_h", |r| format!("=IFERROR((Bod_Bedtime-{})*24,\"\")", window::last_meal(&p(0, r)))),
            Col::f("caffeine_gap_h", |r| format!("=IFERROR((Bod_Bedtime-{})*24,\"\")", caffeine::last_dose(&p(0, r)))),
            Col::f("first_cgm_next", |r| format!("={}", glycemic::first_reading(&next(r)))),
        ];
        table(ws, cx, "tblBodHygDays", HR, PD_C, &cols, DAYS)?;
        let (d0, d1) = (HR + 1, HR + DAYS);
        super::date_rows(ws, cx, PD_C, d0, d1)?;
        super::fmt_rows(ws, d0, d1, PD_C + 2, PD_C + 2, "0.00")?;
        super::fmt_rows(ws, d0, d1, PD_C + 4, PD_C + 4, "0%")?;
        super::fmt_rows(ws, d0, d1, PD_C + 5, PD_C + 6, "0.0")?;
        let g0 = p(5, d0 + 1);
        let short = format!("=AND(ISNUMBER({g0}),{g0}<Bod_Last_meal_gap_h)");
        theme::cf_formula(ws, d0, PD_C + 5, d1, PD_C + 5, &short, theme::OUT_OF_REF)?;
        let s0 = p(2, d0 + 1);
        let low = format!("=AND(ISNUMBER({s0}),{s0}<Bod_Sleep_min_h)");
        theme::cf_formula(ws, d0, PD_C + 2, d1, PD_C + 2, &low, theme::OUT_OF_REF)?;

        // Summary.
        let sc = SUM_C;
        let day = |k: u16| rng(TITLE, PD_C + k, d0, d1);
        let (ts, tk, tn) = (SUPP.range(cx.d, "time_of_day"), SUPP.range(cx.d, "taking"), SUPP.range(cx.d, "name"));
        let eve = |extra: &str| {
            format!("=COUNTIFS({ts},\"Before bed\",{tk},\"YES\"{extra})+COUNTIFS({ts},\"Night\",{tk},\"YES\"{extra})")
        };
        let rows: Vec<(&str, String, Option<&str>)> = vec![
            ("Nights scored (all)", format!("=COUNT({hd})"), None),
            ("Mean score (28 d)", format!("=IFERROR(AVERAGE({}),\"\")", day(1)), Some("0.0")),
            ("n (score, next-day HRV)", format!("={}", n_pairs(&day(1), &day(3))), None),
            ("r (score, next-day HRV)", format!("={}", pearson(&day(1), &day(3))), Some("+0.00;-0.00")),
            ("n (score, efficiency)", format!("={}", n_pairs(&day(1), &day(4))), None),
            ("r (score, efficiency)", format!("={}", pearson(&day(1), &day(4))), Some("+0.00;-0.00")),
            ("Mean last-meal-to-bed gap (h)", format!("=IFERROR(AVERAGE({}),\"\")", day(5)), Some("0.0")),
            (
                "Nights gap >= Bod_Last_meal_gap_h",
                format!("=IFERROR(COUNTIF({c},\">=\"&Bod_Last_meal_gap_h)/COUNT({c}),\"\")", c = day(5)),
                Some("0%"),
            ),
            ("Mean last-caffeine-to-bed gap (h)", format!("=IFERROR(AVERAGE({}),\"\")", day(6)), Some("0.0")),
            ("Supplements in Before bed / Night slots", eve(""), None),
            ("  of which caffeine-type", eve(&format!(",{tn},\"*caffeine*\"")), None),
            ("n (sleep h, next first CGM)", format!("={}", n_pairs(&day(2), &day(7))), None),
            ("Slope: mg/dL per extra hour of sleep", format!("={}", slope(&day(2), &day(7))), Some("+0.0;-0.0")),
            ("r (sleep h, next first CGM)", format!("={}", pearson(&day(2), &day(7))), Some("+0.00;-0.00")),
        ];
        section(ws, cx, HR - 1, sc, "Scorecard vs next day, gaps, coupling")?;
        for (i, (label, fm, nf)) in rows.iter().enumerate() {
            kv(ws, cx, HR + i as u32, sc, label, fm, *nf)?;
        }

        // Chronotype card.
        let cr = HR + rows.len() as u32 + 1;
        section(ws, cx, cr, sc, "Chronotype card (Bod_Sleep mid-sleep)")?;
        kv(ws, cx, cr + 1, sc, "Habitual mid-sleep", &format!("={}", sleep::mid_sleep_cell()), Some("hh:mm"))?;
        let h = sleep::mid_sleep_h_cell();
        kv(
            ws,
            cx,
            cr + 2,
            sc,
            "Chronotype (heuristic)",
            &format!("=IF(ISNUMBER({h}),IF({h}<Bod_Chrono_early_h,\"morning\",IF({h}>Bod_Chrono_late_h,\"evening\",\"intermediate\")),\"\")"),
            None,
        )?;
        ws.write_string_with_format(
            cr + 3,
            sc,
            "WWS ch03: morning ~40%, evening ~30%, intermediate ~30% of people.",
            &f.note,
        )?;

        // Quick panel (C11.5).
        let qr = cr + 5;
        section(ws, cx, qr, sc, "Sleep and recovery quick panel")?;
        let asleep = sleep::log("asleep_h");
        kv(ws, cx, qr + 1, sc, "Sleep h, latest logged", &format!("={}", last_num(&asleep)), Some("0.00"))?;
        kv(
            ws,
            cx,
            qr + 2,
            sc,
            "Wearable sleep h, latest",
            &format!("={}", last_num(&wear(cx, "sleep_hours"))),
            Some("0.00"),
        )?;
        kv(ws, cx, qr + 3, sc, "HRV, latest", &format!("={}", last_num(&wear(cx, "hrv"))), Some("0"))?;
        kv(ws, cx, qr + 4, sc, "RHR, latest", &format!("={}", last_num(&wear(cx, "rhr"))), Some("0"))?;
        kv(
            ws,
            cx,
            qr + 5,
            sc,
            "Wake-time SD, latest (min)",
            &format!("={}", last_num(&sleep::log("wake_sd_min"))),
            Some("0"),
        )?;
        ws.write_string_with_format(qr + 6, sc, "PSQI global score (0-21, typed)", &f.bold)?;
        ws.write_blank(qr + 6, sc + 1, &f.input)?;
        let dv = DataValidation::new().allow_whole_number(DataValidationRule::Between(0, 21));
        ws.add_data_validation(qr + 6, sc + 1, qr + 6, sc + 1, &dv)?;
        ws.write_string_with_format(qr + 7, sc, "PSQI date", &f.bold)?;
        ws.write_blank(qr + 7, sc + 1, &super::input_date())?;
        ws.write_string_with_format(
            qr + 8,
            sc,
            "OUT ch58 l.25-27: trackers estimate duration and stages reasonably; PSQI is the best-validated questionnaire.",
            &f.note,
        )?;

        // STOP-BANG slot.
        let br = qr + 10;
        section(ws, cx, br, sc, "STOP-BANG screening slot (YES/NO)")?;
        let cols = [Col::In("item"), Col::In("answer")];
        table(ws, cx, "tblBodSTOPBANG", br + 1, sc, &cols, STOPBANG.len() as u32)?;
        for (i, item) in STOPBANG.iter().enumerate() {
            ws.write_string(br + 2 + i as u32, sc, *item)?;
        }
        let (b0, b1) = (br + 2, br + 1 + STOPBANG.len() as u32);
        yes_no(ws, b0, b1, sc + 1, sc + 1)?;
        let ans = rng(TITLE, sc + 1, b0, b1);
        kv(ws, cx, b1 + 1, sc, "STOP-BANG YES count", &format!("=COUNTIF({ans},\"YES\")"), None)?;
        let note = theme::dx_risk("obstructive sleep apnoea risk on a screening questionnaire");
        kv(
            ws,
            cx,
            b1 + 2,
            sc,
            "Flag",
            &format!("=IF(COUNTA({ans})=0,\"\",IF({}>=Bod_STOPBANG_flag,\"{note}\",\"\"))", super::abs(sc + 1, b1 + 1)),
            None,
        )?;
        theme::unverified(
            ws,
            b1 + 3,
            sc,
            "Chung 2008 (Anesthesiology 108:812): STOP-BANG items and the Bod_STOPBANG_flag cut-off",
            f,
        )?;
        ws.write_string_with_format(
            b1 + 4,
            sc,
            "OUT ch58 l.55: STOP-BANG correlates with formal testing; an inclined bed is weak evidence (one tweet, unverified).",
            &f.note,
        )?;

        let chart = super::line_chart(
            TITLE,
            "Sleep hours and hygiene score per night",
            d0,
            d1,
            PD_C,
            &[("score", PD_C + 1), ("sleep h", PD_C + 2)],
        );
        ws.insert_chart(b1 + 6, sc, &chart)?;
        let mut w = vec![11.0];
        w.extend([7.5; 10]);
        w.extend([6.0, 2.0, 11.0, 7.0, 8.0, 8.0, 8.0, 10.0, 11.0, 11.0, 2.0, 36.0, 10.0]);
        set_widths(ws, &w)?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
