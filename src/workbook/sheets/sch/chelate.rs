//! SCH-11 `Sch_Chelate`: chelation / separation windows in the dosing grid. A rule
//! table in the shared schema (`rule` = agent A keyword, `unit` = agent B keyword,
//! `value` = hours apart, all UNVERIFIED: the library names the mechanism, KAT ch82
//! "bind or chelate", and one example, WHO ch00.md:1144, but no hours). For each rule
//! the slots holding A and B are read from `Sch_Dosing` (COUNTIFS on names x slot);
//! the smallest clock gap between an A slot and a B slot (slot hours are Params) is
//! compared with the rule: same slot or too close = CONFLICT.

use rust_xlsxwriter::{Chart, ChartType, Worksheet};

use super::dosing::{self, SLOTS, SLOT_HOURS};
use super::{at, head, kpi, kv, rng, section, Banner, TOP};
use crate::error::Result;
use crate::infra::refs::RULE_HEADERS;
use crate::infra::theme;
use crate::workbook::{add_table, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Sch_Chelate";
const PENDING: &str = "pending: Stockley's Drug Interactions / product labelling (separation hours)";
const RR: u32 = TOP + 4;
/// Agent A, agent B, hours, mechanism note.
const RULES: [(&str, &str, f64, &str); 13] = [
    ("iron", "calcium", 2.0, "PK-absorption: minerals compete / bind (KAT ch82)"),
    ("iron", "zinc", 2.0, "PK-absorption: minerals compete (KAT ch82)"),
    ("calcium", "zinc", 2.0, "PK-absorption: minerals compete (KAT ch82)"),
    ("calcium", "levothyroxine", 4.0, "PK-absorption: binds thyroid hormone (KAT ch82)"),
    ("iron", "levothyroxine", 4.0, "PK-absorption: binds thyroid hormone (KAT ch82)"),
    ("magnesium", "levothyroxine", 4.0, "PK-absorption: binds thyroid hormone (KAT ch82)"),
    ("calcium", "floxacin", 2.0, "PK-absorption: chelates quinolones (KAT ch82)"),
    ("magnesium", "floxacin", 2.0, "PK-absorption: chelates quinolones (KAT ch82)"),
    ("zinc", "floxacin", 2.0, "PK-absorption: chelates quinolones (KAT ch82)"),
    ("iron", "floxacin", 2.0, "PK-absorption: chelates quinolones (KAT ch82)"),
    ("calcium", "cycline", 2.0, "PK-absorption: chelates tetracyclines (KAT ch82)"),
    ("iron", "cycline", 2.0, "PK-absorption: chelates tetracyclines (KAT ch82)"),
    ("calcium", "ceftriaxone", 24.0, "do not administer with calcium (WHO ch00.md:1144)"),
];
/// No pair found: the gap sentinel.
const NONE: f64 = 99.0;

pub struct Chelate;

impl SheetModule for Chelate {
    fn id(&self) -> &'static str {
        "sch-chelate"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        &["sch-dosing"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        let (d, f) = (cx.d, cx.f);
        head(
            ws,
            cx,
            "Separation windows: minerals and chelating drugs in the dosing grid",
            "SCH-11 (C5.9) | KAT ch82.md (bind or chelate; absorption); WHO ch00.md:1144",
            Banner::Unverified("Stockley's Drug Interactions / product labelling (hours apart in every rule)"),
        )?;
        let n = RULES.len() as u32;
        let (r0, r1) = (RR + 1, RR + n);
        let mut headers: Vec<String> = RULE_HEADERS.iter().map(|h| (*h).to_string()).collect();
        headers.extend(SLOTS.iter().map(|s| format!("a_{s}")));
        headers.extend(SLOTS.iter().map(|s| format!("b_{s}")));
        headers.extend(["same_slot", "min_gap_h", "gap_shown", "conflict"].map(String::from));
        let hs: Vec<&str> = headers.iter().map(String::as_str).collect();
        section(ws, cx, RR - 1, 0, "Rules (rule = agent A keyword, unit = agent B keyword, value = hours apart)")?;
        add_table(ws, "tblSchChelate", RR, 0, &hs, n as usize, rust_xlsxwriter::TableStyle::Medium4)?;
        let names = dosing::name_range(d);
        let a0 = RULE_HEADERS.len() as u16;
        let b0 = a0 + SLOTS.len() as u16;
        let x0 = b0 + SLOTS.len() as u16;
        for (i, (a, b, h, note)) in RULES.iter().enumerate() {
            let r = r0 + i as u32;
            let x = r + 1;
            ws.write_string(r, 0, format!("CHL-{:02}", i + 1))?;
            ws.write_string_with_format(r, 1, *a, &f.input)?;
            ws.write_number_with_format(r, 2, *h, &f.input)?;
            ws.write_string_with_format(r, 3, *b, &f.input)?;
            ws.write_string(r, 4, PENDING)?;
            ws.write_string_with_format(r, 5, "UNVERIFIED", &f.unverified)?;
            ws.write_string(r, 6, *note)?;
            for k in 0..SLOTS.len() {
                let sr = dosing::slot_range(d, k);
                for (c0, kw) in [(a0, "B"), (b0, "D")] {
                    let fm = format!("=IF(COUNTIFS({names},\"*\"&{kw}{x}&\"*\",{sr},\"?*\")>0,1,0)");
                    ws.write_formula(r, c0 + k as u16, fm.as_str())?;
                }
            }
            let mask = |c0: u16, k: usize| at(c0 + k as u16, x);
            let same: Vec<String> = (0..SLOTS.len()).map(|k| format!("{}*{}", mask(a0, k), mask(b0, k))).collect();
            ws.write_formula(r, x0, format!("={}", same.join("+")).as_str())?;
            let mut gaps = Vec::new();
            for (i, hi) in SLOT_HOURS.iter().enumerate() {
                for (j, hj) in SLOT_HOURS.iter().enumerate() {
                    gaps.push(format!("IF(AND({}=1,{}=1),ABS({hi}-{hj}),{NONE})", mask(a0, i), mask(b0, j)));
                }
            }
            ws.write_formula(r, x0 + 1, format!("=MIN({})", gaps.join(",")).as_str())?;
            let g = at(x0 + 1, x);
            ws.write_formula(r, x0 + 2, format!("=IF({g}={NONE},0,{g})").as_str())?;
            ws.write_formula(r, x0 + 3, format!("=IF({g}={NONE},\"\",IF({g}<C{x},\"CONFLICT\",\"ok\"))").as_str())?;
        }
        let tl = at(x0 + 3, r0 + 1);
        theme::cf_formula(ws, r0, x0 + 3, r1, x0 + 3, &format!("={tl}=\"CONFLICT\""), theme::CRITICAL)?;
        theme::cf_formula(ws, r0, x0 + 3, r1, x0 + 3, &format!("={tl}=\"ok\""), theme::OPTIMAL)?;
        theme::cf_formula(ws, r0, a0, r1, x0 - 1, &format!("={}=1", at(a0, r0 + 1)), theme::OUT_OF_REF)?;

        let conf = rng(TITLE, x0 + 3, r0, r1);
        kv(ws, cx, TOP, 0, "Conflicts in the dosing grid", &format!("=COUNTIF({conf},\"CONFLICT\")"), None)?;
        kv(ws, cx, TOP + 1, 0, "Pairs both taken", &format!("=COUNTIF({conf},\"?*\")"), None)?;
        kpi(cx, "chelate_conflicts", "Separation conflicts in the dosing grid", TITLE, TOP, 1, None)?;

        let mut chart = Chart::new(ChartType::Bar);
        chart
            .add_series()
            .set_name("required hours")
            .set_categories((TITLE, r0, 1, r1, 1))
            .set_values((TITLE, r0, 2, r1, 2));
        chart
            .add_series()
            .set_name("current gap (0 = same slot or not both taken)")
            .set_categories((TITLE, r0, 1, r1, 1))
            .set_values((TITLE, r0, x0 + 2, r1, x0 + 2));
        chart.title().set_name("Required separation vs current gap (agent A shown)");
        chart.set_width(560).set_height(340);
        ws.insert_chart(r1 + 3, 0, &chart)?;
        set_widths(ws, &[9.0, 12.0, 7.0, 14.0, 34.0, 12.0, 40.0])?;
        Ok(())
    }
}
