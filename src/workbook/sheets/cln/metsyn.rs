//! Cln_MetSyn: CLN-21 metabolic-syndrome criteria counter per panel (TG, HDL and
//! glucose from labs; waist and blood pressure from a vitals input table) and
//! CLN-22 the BMI vs criteria-count quadrant scatter.

use rust_xlsxwriter::{Chart, ChartFormat, ChartLine, ChartType, Formula, Worksheet};

use super::ids::{GLUCOSE, HDL, TG};
use super::kit::{self, c, cf, Lab};
use crate::error::Result;
use crate::infra::data::PANELS;
use crate::infra::theme;
use crate::workbook::{col, set_widths, Cx};

pub const TITLE: &str = "Cln_MetSyn";
pub const TABLE: &str = "tblViewClnMetSyn";
pub const VITALS: &str = "tblClnVitals";

/// Value of vitals column `h` on the matched vitals date, blank when none.
fn vital(h: &str) -> String {
    format!(
        "=IF(OR([Vitals date]=\"\",COUNTIFS({VITALS}[date],[Vitals date],{VITALS}[{h}],\"<>\")=0),\"\",SUMIFS({VITALS}[{h}],{VITALS}[date],[Vitals date]))"
    )
}

/// 1 / 0 by sex-specific threshold, blank when the value or Sex is missing.
fn by_sex(cell: &str, op: &str, m: &str, f: &str) -> String {
    format!("=IF(AND(ISNUMBER({cell}),[Sex]<>\"\"),IF({cell}{op}IF([Sex]=\"f\",{f},{m}),1,0),\"\")")
}

pub fn build(ws: &mut Worksheet, cx: &Cx) -> Result<()> {
    let (d, f) = (cx.d, cx.f);
    let lab = Lab::new(d);
    let row = kit::head(
        ws,
        cx,
        "Metabolic-syndrome criteria counter and BMI quadrant",
        &[
            "CLN-21 NEEDS-SOURCE [DX-RISK]: TG > 150, HDL < 40 men / < 50 women, waist > 40 in men / > 35 in women, glucose > 110 as printed in OUT ch13:53-65 (AHA 2005 uses 100); BP 130/85 and the 3-of-5 rule are UNVERIFIED (NCEP ATP III).",
            "Sex-specific criteria need Sex on Params. Waist and blood pressure come from the vitals table below (the physicals have no values): the latest entry within Cln_Vitals_days before a draw counts.",
            "CLN-22 NEEDS-SOURCE [DX-RISK]: BMI (weight at panel, Height_cm) vs criteria met; quadrant lines at BMI 25 (UNVERIFIED, WHO) and 3 criteria. Biomarkers can move years before the textbook definition (OUT ch13:186).",
        ],
    )?;
    theme::awaiting(ws, row, "waist and blood pressure (enter dated vitals below; tblPhys has no measurements)", f)?;
    kit::dx_banner(ws, row + 1, "metabolic syndrome, only where 3 or more criteria are met", cx)?;
    let vr = row + 3;
    kit::section(ws, vr, "Vitals (input)", cx)?;
    kit::input_table(ws, cx, VITALS, vr + 1, 0, &["date", "waist_in", "bp_sys", "bp_dia", "note"])?;
    ws.set_cell_format(vr + 2, 0, &f.date)?;
    let wc = col(PANELS.col("weight_at_panel"));
    let at = |m: &str| format!("={}", lab.at(m, "[Date]"));
    let crit = |cell: &str, cond: &str| format!("=IF(ISNUMBER({cell}),IF({cond},1,0),\"\")");
    let vd = format!(
        "=IF(MAXIFS({VITALS}[date],{VITALS}[date],\"<=\"&[Date],{VITALS}[date],\">=\"&([Date]-Cln_Vitals_days))=0,\"\",MAXIFS({VITALS}[date],{VITALS}[date],\"<=\"&[Date],{VITALS}[date],\">=\"&([Date]-Cln_Vitals_days)))"
    );
    let cols = vec![
        cf("Date", kit::panel_date(), &f.date),
        c("Sex", "=IF(Sex=\"\",\"\",LOWER(LEFT(Sex,1)))"),
        c("TG", at(TG)),
        c("HDL", at(HDL)),
        c("Glucose", at(GLUCOSE)),
        cf("Vitals date", vd, &f.date),
        c("Waist in", vital("waist_in")),
        c("SBP", vital("bp_sys")),
        c("DBP", vital("bp_dia")),
        c("TG crit", crit("[TG]", "[TG]>Cln_MetS_TG")),
        c("HDL crit", by_sex("[HDL]", "<", "Cln_MetS_HDL_m", "Cln_MetS_HDL_f")),
        c("Glucose crit", crit("[Glucose]", "[Glucose]>Cln_MetS_Glu")),
        c("Waist crit", by_sex("[Waist in]", ">", "Cln_MetS_Waist_m", "Cln_MetS_Waist_f")),
        c(
            "BP crit",
            "=IF(COUNT([SBP],[DBP])=0,\"\",IF(OR(AND(ISNUMBER([SBP]),[SBP]>=Cln_MetS_SBP),AND(ISNUMBER([DBP]),[DBP]>=Cln_MetS_DBP)),1,0))",
        ),
        c("Met", "=SUM([TG crit],[HDL crit],[Glucose crit],[Waist crit],[BP crit])"),
        c("Assessed", "=COUNT([TG crit],[HDL crit],[Glucose crit],[Waist crit],[BP crit])"),
        c("Pattern", kit::dx_if("AND([Assessed]>0,[Met]>=Cln_MetS_n)", "metabolic syndrome (3 or more criteria met)")),
        cf(
            "Weight kg",
            format!("=IF(ISNUMBER({s}!${wc}${{p}}),{s}!${wc}${{p}}*0.45359237,\"\")", s = PANELS.sheet),
            &f.dec1,
        ),
        cf("BMI", "=IFERROR(IF(AND(ISNUMBER([Weight kg]),ISNUMBER(Height_cm)),[Weight kg]/(Height_cm/100)^2,\"\"),\"\")", &f.dec1),
        c(
            "Quadrant",
            "=IF(OR(NOT(ISNUMBER([BMI])),[Assessed]=0),\"\",IF([BMI]>=Cln_BMI_cut,\"BMI >= 25\",\"BMI < 25\")&\" / \"&IF([Met]>=Cln_MetS_n,\">= 3 criteria\",\"< 3 criteria\"))",
        ),
    ];
    let hr = vr + 5;
    let t = kit::table(ws, TITLE, TABLE, hr, 0, &cols, kit::n_panels(d))?;
    theme::cf_formula(
        ws,
        t.first,
        t.col("TG crit"),
        t.last,
        t.col("BP crit"),
        &format!("={}{}=1", col(t.col("TG crit")), t.first + 1),
        theme::OUT_OF_REF,
    )?;
    kit::cf_text(ws, &t, "Pattern", "Pattern", theme::OUT_OF_REF)?;
    // Quadrant lines (CLN-22): x = BMI cut from 0 to 5 criteria; y = 3 criteria across BMI 15-40.
    let q = t.last + 2;
    ws.write_string_with_format(q, 0, "Quadrant lines (chart helper)", &f.note)?;
    for (i, (x, y)) in
        [("=Cln_BMI_cut", "0"), ("=Cln_BMI_cut", "5"), ("15", "=Cln_MetS_n"), ("40", "=Cln_MetS_n")].iter().enumerate()
    {
        let r = q + 1 + i as u32;
        for (k, v) in [(0u16, x), (1, y)] {
            match v.strip_prefix('=') {
                Some(_) => ws.write_formula(r, k, Formula::new(*v)).map(|_| ())?,
                None => ws.write_number(r, k, v.parse::<f64>().unwrap_or_default()).map(|_| ())?,
            }
        }
    }
    let mut ch = Chart::new(ChartType::Scatter);
    ch.add_series().set_name("panels").set_categories(t.area("BMI")).set_values(t.area("Met"));
    for (name, r0) in [("BMI cut", q + 1), ("criteria cut", q + 3)] {
        ch.add_series()
            .set_name(name)
            .set_categories((TITLE, r0, 0, r0 + 1, 0))
            .set_values((TITLE, r0, 1, r0 + 1, 1))
            .set_format(ChartFormat::new().set_line(ChartLine::new().set_color("7F7F7F")));
    }
    ch.title().set_name("BMI vs metabolic criteria met (CLN-22)");
    ch.x_axis().set_name("BMI kg/m2");
    ch.y_axis().set_name("criteria met").set_min(0).set_max(5);
    ch.set_width(520).set_height(320);
    ws.insert_chart(q, 3, &ch)?;
    let met = t.range("Met");
    let asd = t.range("Assessed");
    kit::kpi(
        cx,
        "metsyn",
        "Metabolic-syndrome criteria met (latest assessed panel)",
        &format!("LOOKUP(2,1/({asd}>0),{met})"),
        None,
    )?;
    set_widths(ws, &[11.0])?;
    Ok(())
}
