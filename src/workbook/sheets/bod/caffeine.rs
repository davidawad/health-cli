//! BOD-29 `Bod_Caffeine` (L-E4, L-F11): caffeine clock, mg still active at bedtime.
//!
//! Every dose decays as `mg*0.5^((Bod_Bedtime - time)*24/HL)`; doses after bedtime
//! are excluded. The half-life is an override cell (validated 5-8 h) else
//! `IFERROR(G_Caffeine_HL, Caffeine_HL)`: GEN-09 may publish a genotype value; the
//! name may not exist in a build, so `IFERROR` absorbs `#NAME?`.
//!
//! Blocks: the caffeine log `tblBodCaffeine` (date, time, source, mg) with a per-dose
//! mg-at-bed column; a per-day table for the 28 days up to the last logged date
//! (`SUMPRODUCT` with date / before-bedtime masks); the supplement stack from `tblSupp`
//! (taking = YES, name matching caffeine / coffee / tea / guarana, mg amounts, slot hour
//! from `Ref_TimeSlots`); a 100 mg decay curve. KPI `caffeine_bed`.
//!
//! PLACEHOLDER: the dose log is empty until typed; the stack block computes now.

use rust_xlsxwriter::{Chart, ChartType, DataValidation, DataValidationRule, Worksheet};

use super::sleep::time_cells;
use super::{at, head, kpi, kv, list, rng, section, table, Banner, Col, LOG_ROWS, TOP};
use crate::error::Result;
use crate::infra::data::SUPP;
use crate::infra::refs::REF_TIME_SLOTS;
use crate::infra::theme;
use crate::workbook::{col as letter, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_Caffeine";
pub const SOURCES: [&str; 5] = ["coffee", "tea", "pre-workout", "supplement", "other"];
const DAYS: u32 = 28;
const HR: u32 = TOP + 10;
const STACK_C: u16 = 10;
const LOG_C: u16 = 19;
const LOG: &[&str] = &["date", "time", "source", "mg", "mg_at_bed"];

/// Absolute range of a caffeine-log column (`date`, `time`, `source`, `mg`, `mg_at_bed`).
pub fn log(h: &str) -> String {
    let i = LOG.iter().position(|x| *x == h).unwrap_or_else(|| panic!("tblBodCaffeine has no column {h}"));
    rng(TITLE, LOG_C + i as u16, HR + 1, HR + LOG_ROWS)
}

/// Total mg logged on `day` (a date expression), or "" when nothing is logged.
pub fn mg_on(day: &str) -> String {
    let (d, m) = (log("date"), log("mg"));
    format!("IF(COUNTIFS({d},{day})=0,\"\",SUMIFS({m},{d},{day}))")
}

/// Clock time of the last dose logged on `day`, or "".
pub fn last_dose(day: &str) -> String {
    let (d, t) = (log("date"), log("time"));
    format!("IF(COUNTIFS({d},{day},{t},\">=0\")=0,\"\",MAXIFS({t},{d},{day}))")
}

/// The half-life in use (h).
fn hl() -> String {
    super::abs(1, TOP + 4)
}

pub struct Caffeine;

impl SheetModule for Caffeine {
    fn id(&self) -> &'static str {
        "bod-caffeine"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        &["data-supp", "ref-timeslots"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let f = cx.f;
        head(
            ws,
            cx,
            "Caffeine clock: mg active at bedtime",
            "BOD-29",
            "L-E4, L-F11 | WWS ch03 (half-life 5-7 h; 7:30 pm coffee leaves 50% at 1:30 am), WWS ch21 tip 3; genotype override from GEN-09",
            Banner::Awaiting("caffeine doses in tblBodCaffeine (date, time, source, mg); the supplement-stack block reads tblSupp now"),
        )?;
        let hl = hl();
        section(ws, cx, TOP, 0, "Half-life")?;
        ws.write_string_with_format(TOP + 1, 0, "Half-life override (h, 5-8)", &f.bold)?;
        ws.write_blank(TOP + 1, 1, &f.input)?;
        let dv = DataValidation::new()
            .allow_decimal_number(DataValidationRule::Between(5.0, 8.0))
            .set_input_message("blank: genotype value (G_Caffeine_HL) if present, else Caffeine_HL")?;
        ws.add_data_validation(TOP + 1, 1, TOP + 1, 1, &dv)?;
        let ov = super::abs(1, TOP + 1);
        kv(ws, cx, TOP + 2, 0, "Genotype half-life (G_Caffeine_HL)", "=IFERROR(G_Caffeine_HL,\"\")", Some("0.0"))?;
        kv(ws, cx, TOP + 3, 0, "Default half-life (Caffeine_HL)", "=Caffeine_HL", Some("0.0"))?;
        let used = format!("=IF(ISNUMBER({ov}),{ov},IFERROR(G_Caffeine_HL,Caffeine_HL))");
        kv(ws, cx, TOP + 4, 0, "Half-life used (h)", &used, Some("0.0"))?;
        kv(ws, cx, TOP + 5, 0, "Bedtime (Bod_Bedtime)", "=Bod_Bedtime", Some("hh:mm"))?;
        kv(
            ws,
            cx,
            TOP + 6,
            0,
            "Half-life source",
            &format!("=IF(ISNUMBER({ov}),\"override\",IFERROR(IF(ISNUMBER(G_Caffeine_HL),\"genotype (GEN-09)\",\"Caffeine_HL\"),\"Caffeine_HL (WWS ch03)\"))"),
            None,
        )?;

        // Per-day table.
        let (dl, tl, ml) = (log("date"), log("time"), log("mg"));
        kv(ws, cx, TOP + 7, 0, "Anchor day", &format!("=IF(COUNT({dl})=0,Today,MAX({dl}))"), Some(theme::nf::DATE))?;
        let anchor = super::abs(1, TOP + 7);
        section(ws, cx, HR - 1, 0, "Last 28 days")?;
        let cols = vec![
            Col::f("day", |r| format!("={anchor}-{}", HR + 1 + DAYS - r)),
            Col::f("doses", |r| format!("=COUNTIFS({dl},{})", at(0, r))),
            Col::f("total_mg", |r| format!("=SUMIFS({ml},{dl},{})", at(0, r))),
            Col::f("last_dose", |r| format!("={}", last_dose(&at(0, r)))),
            Col::f("last_to_bed_h", |r| format!("=IF(ISNUMBER({t}),(Bod_Bedtime-{t})*24,\"\")", t = at(3, r))),
            Col::f("mg_at_bed", |r| {
                format!(
                    "=IF({n}=0,\"\",SUMPRODUCT(({dl}={d})*ISNUMBER({tl})*({tl}<=Bod_Bedtime),{ml},0.5^((Bod_Bedtime-{tl})*24/{hl})))",
                    n = at(1, r),
                    d = at(0, r)
                )
            }),
            Col::f("flag", |r| {
                format!("=IF(ISNUMBER({m}),IF({m}>Bod_Caffeine_bed_mg,\"over\",\"\"),\"\")", m = at(5, r))
            }),
        ];
        table(ws, cx, "tblBodCaffeineDays", HR, 0, &cols, DAYS)?;
        let (r0, r1) = (HR + 1, HR + DAYS);
        super::date_rows(ws, cx, 0, r0, r1)?;
        super::fmt_rows(ws, r0, r1, 3, 3, "hh:mm")?;
        super::fmt_rows(ws, r0, r1, 4, 5, "0.0")?;
        let m0 = at(5, r0 + 1);
        theme::cf_formula(
            ws,
            r0,
            5,
            r1,
            5,
            &format!("=AND(ISNUMBER({m0}),{m0}>Bod_Caffeine_bed_mg)"),
            theme::OUT_OF_REF,
        )?;
        theme::cf_data_bar(ws, r0, 2, r1, 2, theme::BAND_OPT)?;

        // Supplement stack (row-aligned with tblSupp).
        let ns = SUPP.nrows(cx.d);
        let sv = |h: &str, r: u32| format!("{}!{}{}", SUPP.sheet, letter(SUPP.col(h)), r - HR);
        let (slot_r, hour_r) = (REF_TIME_SLOTS.range(cx.d, "slot"), REF_TIME_SLOTS.range(cx.d, "start_hour"));
        section(ws, cx, HR - 1, STACK_C, "Supplement stack (tblSupp, taking = YES, caffeine-type names, mg)")?;
        let sc = |k: u16, r: u32| at(STACK_C + k, r);
        let cols = vec![
            Col::f("name", |r| format!("=\"\"&{}", sv("name", r))),
            Col::f("taking", |r| format!("=\"\"&{}", sv("taking", r))),
            Col::f("caffeinated", |r| {
                let n = sc(0, r);
                format!(
                    "=IF(AND({t}=\"YES\",OR(ISNUMBER(SEARCH(\"caffeine\",{n})),ISNUMBER(SEARCH(\"coffee\",{n})),ISNUMBER(SEARCH(\" tea\",\" \"&{n})),ISNUMBER(SEARCH(\"guarana\",{n})))),1,0)",
                    t = sc(1, r)
                )
            }),
            Col::f("mg", |r| {
                let amt = sv("amount", r);
                format!(
                    "=IF(AND({c}=1,LOWER(\"\"&{u})=\"mg\",ISNUMBER({amt})),{amt},\"\")",
                    c = sc(2, r),
                    u = sv("unit", r)
                )
            }),
            Col::f("slot", |r| format!("=\"\"&{}", sv("time_of_day", r))),
            Col::f("start_h", |r| format!("=IFERROR(INDEX({hour_r},MATCH({},{slot_r},0)),\"\")", sc(4, r))),
            Col::f("mg_at_bed", |r| {
                let (m, h) = (sc(3, r), sc(5, r));
                format!(
                    "=IF(AND(ISNUMBER({m}),ISNUMBER({h})),IF({h}/24<=Bod_Bedtime,{m}*0.5^((Bod_Bedtime-{h}/24)*24/{hl}),0),\"\")"
                )
            }),
        ];
        table(ws, cx, "tblBodCaffeineStack", HR, STACK_C, &cols, ns)?;
        super::fmt_rows(ws, r0, HR + ns, STACK_C + 6, STACK_C + 6, "0.0")?;
        let stack = rng(TITLE, STACK_C + 6, r0, HR + ns);

        // Summary.
        let sr = r1 + 2;
        section(ws, cx, sr, 0, "At bedtime")?;
        kv(ws, cx, sr + 1, 0, "Supplement stack, mg at bedtime", &format!("=SUM({stack})"), Some("0.0"))?;
        let latest = at(5, r1 + 1);
        kv(
            ws,
            cx,
            sr + 2,
            0,
            "Latest logged day, mg at bedtime",
            &format!("=IF(ISNUMBER({latest}),{latest},\"\")"),
            Some("0.0"),
        )?;
        let (s1, s2) = (super::abs(1, sr + 1), super::abs(1, sr + 2));
        kv(ws, cx, sr + 3, 0, "Total at bedtime (stack + latest day)", &format!("={s1}+N({s2})"), Some("0.0"))?;
        kv(
            ws,
            cx,
            sr + 4,
            0,
            "Flag",
            &format!("=IF({}>Bod_Caffeine_bed_mg,\"above Bod_Caffeine_bed_mg\",\"ok\")", super::abs(1, sr + 3)),
            None,
        )?;
        kv(
            ws,
            cx,
            sr + 5,
            0,
            "Days over the flag (28 d)",
            &format!("=COUNTIF({},\"over\")", rng(TITLE, 6, r0, r1)),
            None,
        )?;
        ws.write_string_with_format(
            sr + 6,
            0,
            "Do not log stack doses again in the dose log (they are counted above).",
            &f.note,
        )?;
        kpi(cx, "caffeine_bed", "Caffeine active at bedtime (mg)", TITLE, sr + 3, 1, Some("0"))?;

        // Decay curve of a 100 mg dose.
        let dr = HR + ns.max(1) + 3;
        section(ws, cx, dr - 1, STACK_C, "Decay of a 100 mg dose")?;
        let cols = vec![
            Col::f("hours_after", |r| format!("={}", r - dr - 2)),
            Col::f("mg_left", |r| format!("=100*0.5^({}/{hl})", at(STACK_C, r))),
        ];
        table(ws, cx, "tblBodCaffeineDecay", dr, STACK_C, &cols, 25)?;
        super::fmt_rows(ws, dr + 1, dr + 25, STACK_C + 1, STACK_C + 1, "0.0")?;

        let mut bars = Chart::new(ChartType::Column);
        bars.add_series()
            .set_name("mg at bedtime")
            .set_categories((TITLE, r0, 0, r1, 0))
            .set_values((TITLE, r0, 5, r1, 5));
        bars.title().set_name("Caffeine still active at bedtime (mg)");
        bars.x_axis().set_num_format("mm-dd");
        bars.set_width(600).set_height(280);
        ws.insert_chart(sr + 8, 0, &bars)?;
        let mut curve = Chart::new(ChartType::Line);
        curve
            .add_series()
            .set_name("mg left of 100 mg")
            .set_categories((TITLE, dr + 1, STACK_C, dr + 25, STACK_C))
            .set_values((TITLE, dr + 1, STACK_C + 1, dr + 25, STACK_C + 1));
        curve.title().set_name("Decay of 100 mg (hours after the dose)");
        curve.set_width(520).set_height(280);
        ws.insert_chart(dr + 27, STACK_C, &curve)?;

        // Dose log.
        section(ws, cx, HR - 1, LOG_C, "Caffeine log (typed)")?;
        let cols = vec![
            Col::Date("date"),
            Col::In("time"),
            Col::In("source"),
            Col::In("mg"),
            Col::f("mg_at_bed", |r| {
                let (t, m) = (at(LOG_C + 1, r), at(LOG_C + 3, r));
                format!(
                    "=IF(AND(ISNUMBER({t}),ISNUMBER({m})),IF({t}<=Bod_Bedtime,{m}*0.5^((Bod_Bedtime-{t})*24/{hl}),0),\"\")"
                )
            }),
        ];
        table(ws, cx, "tblBodCaffeine", HR, LOG_C, &cols, LOG_ROWS)?;
        time_cells(ws, HR + 1, HR + LOG_ROWS, LOG_C + 1)?;
        list(ws, HR + 1, LOG_C + 2, HR + LOG_ROWS, &SOURCES)?;
        super::fmt_rows(ws, HR + 1, HR + LOG_ROWS, LOG_C + 4, LOG_C + 4, "0.0")?;
        let mut w = vec![34.0, 10.0, 9.0, 9.0, 11.0, 10.0, 8.0, 2.0, 2.0, 2.0];
        w.extend([24.0, 8.0, 11.0, 8.0, 11.0, 8.0, 10.0, 2.0, 2.0, 12.0, 8.0, 12.0, 8.0, 10.0]);
        set_widths(ws, &w)?;
        Ok(())
    }
}
