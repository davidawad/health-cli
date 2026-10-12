//! Cln_Insulin: CLN-11 TyG per panel (computable now), HOMA-IR and QUICKI (need
//! fasting insulin); CLN-12 an OGTT entry template (0-120 min glucose and insulin).

use rust_xlsxwriter::{Chart, ChartType, Formula, TableStyle, Worksheet};

use super::ids::{GLUCOSE, INSULIN, TG};
use super::kit::{self, c, cf, need, Lab};
use crate::error::Result;
use crate::infra::data::PANELS;
use crate::infra::theme;
use crate::workbook::{add_table, col, set_widths, Cx};

pub const TITLE: &str = "Cln_Insulin";
pub const TABLE: &str = "tblViewClnInsulin";
pub const OGTT: &str = "tblClnOGTT";

pub fn build(ws: &mut Worksheet, cx: &Cx) -> Result<()> {
    let (d, f) = (cx.d, cx.f);
    let lab = Lab::new(d);
    let row = kit::head(
        ws,
        cx,
        "Insulin sensitivity indices and OGTT template",
        &[
            "CLN-11 NEEDS-SOURCE [DX-RISK]: TyG = ln(TG x glucose / 2) (Simental-Mendia 2008), HOMA-IR = insulin x glucose / 405 (Matthews 1985), QUICKI = 1 / (log insulin + log glucose) (Katz 2000): all UNVERIFIED; numbers only, no labels.",
            "CLN-12 PLACEHOLDER: OGTT template - 75 g glucose, glucose and insulin every 30 min for 2 h (OUT ch13:188-190).",
        ],
    )?;
    theme::awaiting(ws, row, "fasting insulin (HOMA-IR and QUICKI stay blank until it is measured)", f)?;
    let fc = col(PANELS.col("fasting"));
    let at = |m: &str| format!("={}", lab.at(m, "[Date]"));
    let cols = vec![
        cf("Date", kit::panel_date(), &f.date),
        c("Fasting", format!("=IF({s}!${fc}${{p}}=\"\",\"\",{s}!${fc}${{p}})", s = PANELS.sheet)),
        c("TG", at(TG)),
        c("Glucose", at(GLUCOSE)),
        c("Insulin", at(INSULIN)),
        cf("TyG", need(&["[TG]", "[Glucose]"], "LN([TG]*[Glucose]/Cln_TyG_div)"), &f.dec2),
        cf("HOMA-IR", need(&["[Insulin]", "[Glucose]"], "[Insulin]*[Glucose]/Cln_HOMA_div"), &f.dec2),
        cf("QUICKI", need(&["[Insulin]", "[Glucose]"], "1/(LOG10([Insulin])+LOG10([Glucose]))"), &f.dec2),
    ];
    let hr = row + 2;
    let t = kit::table(ws, TITLE, TABLE, hr, 0, &cols, kit::n_panels(d))?;
    for (h, p) in [("TyG", "Simental-Mendia 2008"), ("HOMA-IR", "Matthews 1985"), ("QUICKI", "Katz 2000")] {
        theme::unverified(ws, hr - 1, t.col(h), p, f)?;
    }
    ws.insert_chart(t.last + 2, 0, &kit::line_chart(&t, "TyG index per panel", "Date", &["TyG"], "TyG"))?;
    kit::kpi(
        cx,
        "tyg",
        "Latest TyG index",
        &format!("LOOKUP(2,1/ISNUMBER({r}),{r})", r = t.range("TyG")),
        Some("0.00"),
    )?;

    // CLN-12: OGTT template.
    let o = t.last + 19;
    kit::section(ws, o, "OGTT curve (CLN-12): type the values of one test", cx)?;
    theme::awaiting(ws, o + 1, "an OGTT with glucose and insulin (none recorded)", f)?;
    ws.write_string(o + 2, 0, "Test date")?;
    ws.write_blank(o + 2, 1, &f.input)?;
    let heads = ["minute", "glucose", "insulin"];
    add_table(ws, OGTT, o + 3, 0, &heads, 5, TableStyle::Light1)?;
    for (i, m) in [0.0, 30.0, 60.0, 90.0, 120.0].iter().enumerate() {
        let r = o + 4 + i as u32;
        ws.write_number(r, 0, *m)?;
        ws.write_blank(r, 1, &f.input)?;
        ws.write_blank(r, 2, &f.input)?;
    }
    let s = o + 10;
    for (i, (label, formula)) in [
        ("Insulin peak at minute", format!("=IF(COUNT({OGTT}[insulin])=0,\"\",INDEX({OGTT}[minute],MATCH(MAX({OGTT}[insulin]),{OGTT}[insulin],0)))")),
        ("Glucose at 120 min", format!("=IFERROR(IF(COUNTIFS({OGTT}[minute],120,{OGTT}[glucose],\"<>\")=0,\"\",SUMIFS({OGTT}[glucose],{OGTT}[minute],120)),\"\")")),
        ("Insulin 120 / 0 min", format!("=IFERROR(SUMIFS({OGTT}[insulin],{OGTT}[minute],120)/SUMIFS({OGTT}[insulin],{OGTT}[minute],0),\"\")")),
    ]
    .iter()
    .enumerate()
    {
        ws.write_string(s + i as u32, 0, *label)?;
        ws.write_formula_with_format(s + i as u32, 1, Formula::new(formula), &f.dec1)?;
    }
    ws.write_string_with_format(
        s + 3,
        0,
        "Early insulin resistance shows as a steep rise at 30 min that stays high (OUT ch13:188-190).",
        &f.note,
    )?;
    let mut ch = Chart::new(ChartType::ScatterStraightWithMarkers);
    for (k, name) in [(1u16, "glucose"), (2, "insulin")] {
        let ser = ch.add_series().set_name(name).set_categories((TITLE, o + 4, 0, o + 8, 0)).set_values((
            TITLE,
            o + 4,
            k,
            o + 8,
            k,
        ));
        if k == 2 {
            ser.set_secondary_axis(true);
        }
    }
    ch.title().set_name("OGTT: glucose (left) and insulin (right) by minute");
    ch.x_axis().set_name("minute");
    ch.set_width(520).set_height(280);
    ws.insert_chart(o + 2, 5, &ch)?;
    set_widths(ws, &[24.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0])?;
    Ok(())
}
