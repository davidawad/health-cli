//! Cln_Lipids, lower half: the Martin-Hopkins factor table (CLN-03), ApoB-years
//! exposure (CLN-06) and expected vs observed LDL change per intervention (CLN-07).

use rust_xlsxwriter::{Chart, ChartType, DataValidation, Formula, Worksheet};

use super::ids::LDL;
use super::kit::{self, c, cf, need, Lab};
use super::lipids::{MH_TABLE, TITLE};
use crate::error::Result;
use crate::infra::data::{MARKERS, SUPP};
use crate::infra::refs::RULE_HEADERS;
use crate::infra::theme;
use crate::workbook::{add_table, Cx, Data};

pub const EXPOSURE_TABLE: &str = "tblViewClnExposure";
pub const EXPECTED_TABLE: &str = "tblClnLdlExpected";
pub const OBSERVED_TABLE: &str = "tblViewClnLdlObserved";

/// Structured references into the Martin-Hopkins factor table.
pub struct MhFactor {
    pub tg_lo: String,
    pub tg_hi: String,
    pub nh_lo: String,
    pub nh_hi: String,
    pub factor: String,
}

impl MhFactor {
    pub fn ranges(_: &Data) -> Self {
        let r = |h: &str| format!("{MH_TABLE}[{h}]");
        Self { tg_lo: r("tg_lo"), tg_hi: r("tg_hi"), nh_lo: r("nonhdl_lo"), nh_hi: r("nonhdl_hi"), factor: r("factor") }
    }
}

/// Expected LDL effects quoted in DIP ch37 (rule-table schema).
const EXPECTED: &[(&str, &str, f64, &str, &str, &str)] = &[
    ("fiber", "Soluble fiber 3-12 g/day", -10.0, "mg/dL", "DIP ch37:327", "about 10 mg/dL TC/LDL-C reduction"),
    (
        "phytosterols",
        "Plant sterols/stanols 2 g/day",
        -10.0,
        "%",
        "DIP ch37:331",
        "-5% to -15% (midpoint shown); no gain above 3 g/day",
    ),
    ("pcsk9i", "PCSK9 inhibitor", -60.0, "%", "DIP ch37:387", "up to -60%"),
    ("mipomersen", "Mipomersen (HoFH)", -25.0, "%", "DIP ch37:345", "about -25%"),
    ("lomitapide", "Lomitapide (HoFH)", -40.0, "%", "DIP ch37:345", "about -40%"),
];

pub fn build(ws: &mut Worksheet, cx: &Cx, start: u32) -> Result<()> {
    let r = martin_hopkins(ws, cx, start)?;
    let r = exposure(ws, cx, r + 2)?;
    expected(ws, cx, r + 2)
}

fn martin_hopkins(ws: &mut Worksheet, cx: &Cx, r: u32) -> Result<u32> {
    kit::section(ws, r, "Martin-Hopkins factor table (CLN-03): enter the published TG x non-HDL grid", cx)?;
    theme::unverified(ws, r + 1, 0, "Martin 2013 JAMA 310:2061 (180-cell factor table, not entered)", cx.f)?;
    kit::input_table(
        ws,
        cx,
        MH_TABLE,
        r + 2,
        0,
        &["tg_lo", "tg_hi", "nonhdl_lo", "nonhdl_hi", "factor", "source_state"],
    )?;
    Ok(r + 3)
}

fn exposure(ws: &mut Worksheet, cx: &Cx, r: u32) -> Result<u32> {
    let (d, f) = (cx.d, cx.f);
    let lab = Lab::new(d);
    kit::section(ws, r, "ApoB-years (CLN-06): exposure above a baseline, trapezoid between draws; any marker", cx)?;
    let labels = [
        "Marker (pick any)",
        "Baseline (default Cln_ApoB_baseline for apob)",
        "ASSUMPTION: first value held since age (blank = none)",
        "Exposure above baseline (value x years)",
        "of which ASSUMPTION before the first draw",
    ];
    for (i, l) in labels.iter().enumerate() {
        ws.write_string(r + 1 + i as u32, 0, *l)?;
    }
    ws.write_string_with_format(r + 1, 1, "apob", &f.input)?;
    let nm = MARKERS.nrows(d);
    ws.add_data_validation(
        r + 1,
        1,
        r + 1,
        1,
        &DataValidation::new().allow_list_formula(Formula::new(format!("={}", MARKERS.range_n(nm, "marker_id")))),
    )?;
    kit::name_cell(cx, "Cln_Exposure_Marker", TITLE, r + 1, 1)?;
    ws.write_formula_with_format(
        r + 2,
        1,
        Formula::new("=IF(Cln_Exposure_Marker=\"apob\",Cln_ApoB_baseline,0)"),
        &f.input,
    )?;
    kit::name_cell(cx, "Cln_Exposure_Base", TITLE, r + 2, 1)?;
    ws.write_blank(r + 3, 1, &f.input)?;
    kit::name_cell(cx, "Cln_Exposure_FromAge", TITLE, r + 3, 1)?;
    let cols = vec![
        cf("Date", kit::panel_date(), &f.date),
        c("Value", format!("={}", lab.at_x("Cln_Exposure_Marker", "[Date]"))),
        cf(
            "Prev date",
            "=IF(NOT(ISNUMBER([Value])),\"\",IFERROR(LOOKUP(2,1/ISNUMBER([<Value]),[<Date]),\"\"))",
            &f.date,
        ),
        c("Prev value", "=IF(NOT(ISNUMBER([Value])),\"\",IFERROR(LOOKUP(2,1/ISNUMBER([<Value]),[<Value]),\"\"))"),
        cf("Years", need(&["[Value]", "[Prev value]"], "([Date]-[Prev date])/365.25"), &f.dec2),
        cf("Excess", need(&["[Years]"], "[Years]*(([Value]+[Prev value])/2-Cln_Exposure_Base)"), &f.dec1),
        cf("Cumulative", "=IF(ISNUMBER([Value]),SUM([^Excess]),\"\")", &f.dec1),
    ];
    let t = kit::table(ws, TITLE, EXPOSURE_TABLE, r + 7, 0, &cols, kit::n_panels(d))?;
    let (dates, vals, cum) = (t.range("Date"), t.range("Value"), t.range("Cumulative"));
    let first = format!("MINIFS({dates},{vals},\">-1E+99\")");
    let extra = format!(
        "=IFERROR(IF(ISNUMBER(Cln_Exposure_FromAge),(({first}-DOB)/365.25-Cln_Exposure_FromAge)*(SUMIFS({vals},{dates},{first})-Cln_Exposure_Base),0),0)"
    );
    ws.write_formula_with_format(r + 5, 1, Formula::new(extra), &f.dec1)?;
    ws.write_formula_with_format(
        r + 4,
        1,
        Formula::new(format!("=IFERROR(LOOKUP(2,1/ISNUMBER({cum}),{cum}),0)+B{}", r + 6)),
        &f.dec1,
    )?;
    ws.write_string_with_format(r + 5, 2, "ASSUMPTION - not measured", &f.unverified)?;
    let mut ch = Chart::new(ChartType::Area);
    ch.add_series().set_name("Cumulative").set_categories(t.area("Date")).set_values(t.area("Cumulative"));
    ch.title().set_name("Cumulative exposure above baseline (CLN-06)");
    ch.x_axis().set_num_format(theme::nf::DATE);
    ch.legend().set_hidden();
    ch.set_width(560).set_height(260);
    ws.insert_chart(r + 1, 9, &ch)?;
    kit::kpi(
        cx,
        "apob_years",
        "ApoB-years above baseline (Cln_Lipids selector)",
        &format!("{TITLE}!$B${}", r + 5),
        Some("0.0"),
    )?;
    Ok(t.last.max(r + 15))
}

fn expected(ws: &mut Worksheet, cx: &Cx, r: u32) -> Result<()> {
    let (d, f) = (cx.d, cx.f);
    let lab = Lab::new(d);
    kit::section(ws, r, "Expected vs observed LDL-C change per intervention (CLN-07)", cx)?;
    theme::awaiting(
        ws,
        r + 1,
        "supplement start/stop dates (tblSupp.start_date / stop_date are empty: GATE-DATES); type the tblSupp name of each intervention",
        f,
    )?;
    add_table(ws, EXPECTED_TABLE, r + 2, 0, RULE_HEADERS, EXPECTED.len(), rust_xlsxwriter::TableStyle::Light1)?;
    for (i, (id, rule, value, unit, source, note)) in EXPECTED.iter().enumerate() {
        let x = r + 3 + i as u32;
        ws.write_string(x, 0, *id)?;
        ws.write_string(x, 1, *rule)?;
        ws.write_number(x, 2, *value)?;
        ws.write_string(x, 3, *unit)?;
        ws.write_string(x, 4, *source)?;
        ws.write_string(x, 5, "verified")?;
        ws.write_string(x, 6, *note)?;
    }
    let ns = SUPP.nrows(d);
    let (names, starts) = (SUPP.range_n(ns, "name"), SUPP.range_n(ns, "start_date"));
    let (lm, ld, lv) = lab.ranges();
    let start = format!("INDEX({starts},MATCH([Supplement],{names},0))");
    let cols = vec![
        c("id", format!("=INDEX({EXPECTED_TABLE}[id],{{p}}-1)")),
        c("Supplement", ""),
        cf("Start", format!("=IFERROR(IF({start}>0,{start},\"\"),\"\")"), &f.date),
        cf(
            "Pre date",
            format!("=IF(NOT(ISNUMBER([Start])),\"\",IF(MAXIFS({ld},{lm},\"{LDL}\",{ld},\"<\"&[Start])=0,\"\",MAXIFS({ld},{lm},\"{LDL}\",{ld},\"<\"&[Start])))"),
            &f.date,
        ),
        c("Pre LDL", need(&["[Pre date]"], &format!("SUMIFS({lv},{lm},\"{LDL}\",{ld},[Pre date])"))),
        cf(
            "Post date",
            format!("=IF(NOT(ISNUMBER([Start])),\"\",IF(MINIFS({ld},{lm},\"{LDL}\",{ld},\">=\"&([Start]+Cln_Post_Days))=0,\"\",MINIFS({ld},{lm},\"{LDL}\",{ld},\">=\"&([Start]+Cln_Post_Days))))"),
            &f.date,
        ),
        c("Post LDL", need(&["[Post date]"], &format!("SUMIFS({lv},{lm},\"{LDL}\",{ld},[Post date])"))),
        cf("Observed %", need(&["[Pre LDL]", "[Post LDL]"], "([Post LDL]-[Pre LDL])/[Pre LDL]*100"), &f.dec1),
        cf("Observed mg/dL", need(&["[Pre LDL]", "[Post LDL]"], "[Post LDL]-[Pre LDL]"), &f.dec1),
        c("Expected", format!("=INDEX({EXPECTED_TABLE}[value],{{p}}-1)")),
        c("Unit", format!("=INDEX({EXPECTED_TABLE}[unit],{{p}}-1)")),
        cf("Observed - expected", "=IFERROR(IF([Unit]=\"%\",[Observed %]-[Expected],[Observed mg/dL]-[Expected]),\"\")", &f.dec1),
    ];
    let t = kit::table(ws, TITLE, OBSERVED_TABLE, r + 4 + EXPECTED.len() as u32, 0, &cols, EXPECTED.len() as u32)?;
    for i in 0..EXPECTED.len() as u32 {
        ws.write_blank(t.first + i, t.col("Supplement"), &f.input)?;
    }
    Ok(())
}
