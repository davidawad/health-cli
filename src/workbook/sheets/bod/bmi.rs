//! BOD-17 `Bod_BMI` (clinical.md C4.3; BUILDABLE-NOW): body-mass index per calendar
//! day and the waist class.
//!
//! Row-aligned with `Bod_Cal`: `BMI = weight_kg / (Height_cm/100)^2` from the
//! carried-forward weight (blank until `Height_cm` is a positive number), the NIH class
//! from the `Bod_BMI_*` bounds (DIP ch164:239-245, verified) and a refeeding-risk flag
//! below `Bod_BMI_refeed` (DIP ch164:89), worded neutrally with `theme::dx_risk`.
//! Waist: `Bod_Waist_cm` when typed, else the `tblSizes` row whose name contains
//! "waist" (inches converted to cm), against the sex-specific cut-off
//! `Bod_Waist_m_cm` / `Bod_Waist_f_cm` (UNVERIFIED: NHLBI 1998). The bounds are listed
//! by name in the rule table `tblBodBmiRef`. KPI `K_BOD_bmi` = latest BMI.

use rust_xlsxwriter::{TableStyle, Worksheet};

use super::{at, cal_at, cal_head, head, kpi, kv, last_num, n_days, rng, section, table, Banner, Col, TOP};
use crate::error::Result;
use crate::infra::data::SIZES;
use crate::infra::refs::RULE_HEADERS;
use crate::infra::theme;
use crate::workbook::{add_table, set_widths, Cx, SheetModule};

pub const TITLE: &str = "Bod_BMI";
const HR: u32 = TOP + 1;
const C_KG: u16 = 3;
const C_BMI: u16 = 4;
const C_CLASS: u16 = 5;
const C_FLAG: u16 = 6;
const C_PLOT: u16 = 7;
/// Summary / waist / rule block column.
const SC: u16 = 12;

/// NIH class of the BMI in cell `x` ("" when `x` is not a number).
fn class(x: &str) -> String {
    format!(
        "IF(ISNUMBER({x}),IF({x}<Bod_BMI_under,\"underweight\",IF({x}<Bod_BMI_over,\"normal\",\
         IF({x}<Bod_BMI_obese,\"overweight\",IF({x}<Bod_BMI_obese3,\"obese\",\"obese class 3\")))),\"\")"
    )
}

/// A rule table (`refs::RULE_HEADERS`) at `(hr, c0)`. `value` is a formula when it starts
/// with `=`, else a number; an `UNVERIFIED` state gets the orange badge format.
pub(super) fn rule_table(ws: &mut Worksheet, cx: &Cx, name: &str, hr: u32, c0: u16, rows: &[[&str; 7]]) -> Result<()> {
    add_table(ws, name, hr, c0, RULE_HEADERS, rows.len(), TableStyle::Light1)?;
    for (i, row) in rows.iter().enumerate() {
        let r = hr + 1 + i as u32;
        for (k, v) in row.iter().enumerate() {
            let c = c0 + k as u16;
            if k == 2 {
                match v.strip_prefix('=') {
                    Some(_) => ws.write_formula(r, c, *v).map(|_| ())?,
                    None => ws.write_number(r, c, v.parse::<f64>().unwrap_or(0.0)).map(|_| ())?,
                }
            } else if k == 5 && *v == "UNVERIFIED" {
                ws.write_string_with_format(r, c, *v, &cx.f.unverified)?;
            } else if !v.is_empty() {
                ws.write_string(r, c, *v)?;
            }
        }
    }
    Ok(())
}

pub struct Bmi;

impl SheetModule for Bmi {
    fn id(&self) -> &'static str {
        "bod-bmi"
    }
    fn title(&self) -> &'static str {
        TITLE
    }
    fn requires(&self) -> &'static [&'static str] {
        &["bod-cal", "data-daily", "data-sizes"]
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
        head(
            ws,
            cx,
            "BMI and waist classes, refeeding-risk note",
            "BOD-17",
            "C4.3 | DIP ch164:239-245 (NIH BMI classes), ch164:89 (refeeding BMI); waist cut-offs NHLBI 1998 (UNVERIFIED)",
            Banner::Unverified("NHLBI 1998 obesity guidelines (waist cut-offs)"),
        )?;
        let n = n_days(cx);
        section(ws, cx, TOP, 0, "BMI per calendar day (carried-forward weight, Height_cm from the Dashboard)")?;
        let mut cols = cal_head(HR);
        cols.push(Col::f("weight_kg", move |r| format!("={}", cal_at("weight_kg", HR, r))));
        cols.push(Col::f("bmi", move |r| {
            let k = at(C_KG, r);
            format!(
                "=IFERROR(IF(AND(ISNUMBER({k}),ISNUMBER(Height_cm)),IF(Height_cm>0,{k}/(Height_cm/100)^2,\"\"),\"\"),\"\")"
            )
        }));
        cols.push(Col::f("class", move |r| format!("={}", class(&at(C_BMI, r)))));
        cols.push(Col::f("refeed_flag", move |r| {
            let b = at(C_BMI, r);
            format!("=IF(ISNUMBER({b}),IF({b}<Bod_BMI_refeed,1,0),\"\")")
        }));
        cols.push(super::plot_col("bmi_plot", C_BMI));
        cols.push(Col::f("under_line", |_| "=Bod_BMI_under".into()));
        cols.push(Col::f("over_line", |_| "=Bod_BMI_over".into()));
        cols.push(Col::f("obese_line", |_| "=Bod_BMI_obese".into()));
        table(ws, cx, "tblBodBmi", HR, 0, &cols, n)?;
        let (r0, r1) = (HR + 1, HR + n);
        super::date_rows(ws, cx, 0, r0, r1)?;
        super::fmt_rows(ws, r0, r1, C_KG, C_BMI, "0.0")?;
        let tl = at(C_BMI, r0 + 1);
        for (cond, fill) in [
            (format!("{tl}<Bod_BMI_under"), theme::OUT_OF_REF),
            (format!("AND({tl}>=Bod_BMI_under,{tl}<Bod_BMI_over)"), theme::OPTIMAL),
            (format!("AND({tl}>=Bod_BMI_over,{tl}<Bod_BMI_obese)"), theme::REF_OK),
            (format!("{tl}>=Bod_BMI_obese"), theme::CRITICAL),
        ] {
            theme::cf_formula(ws, r0, C_BMI, r1, C_CLASS, &format!("=AND(ISNUMBER({tl}),{cond})"), fill)?;
        }

        // Summary.
        let (bmi, flag) = (rng(TITLE, C_BMI, r0, r1), rng(TITLE, C_FLAG, r0, r1));
        let s = TOP;
        section(ws, cx, s, SC, "Summary")?;
        kv(ws, cx, s + 1, SC, "Latest BMI (kg/m2)", &format!("={}", last_num(&bmi)), Some("0.0"))?;
        let latest = super::abs(SC + 1, s + 1);
        kv(ws, cx, s + 2, SC, "Class (NIH)", &format!("={}", class(&latest)), None)?;
        kv(ws, cx, s + 3, SC, "Height (cm)", "=IFERROR(IF(Height_cm>0,Height_cm,\"\"),\"\")", Some("0.0"))?;
        let note = theme::dx_risk("a refeeding-risk BMI (below Bod_BMI_refeed)");
        kv(
            ws,
            cx,
            s + 4,
            SC,
            "Refeeding-risk note",
            &format!("=IF(ISNUMBER({latest}),IF({latest}<Bod_BMI_refeed,\"{note}\",\"no refeeding-risk flag\"),\"\")"),
            None,
        )?;
        kv(ws, cx, s + 5, SC, "Days with BMI < Bod_BMI_refeed", &format!("=SUM({flag})"), Some("0"))?;
        kpi(cx, "bmi", "BMI (latest, kg/m2)", TITLE, s + 1, SC + 1, Some("0.0"))?;

        // Waist block.
        let w = s + 7;
        section(ws, cx, w, SC, "Waist vs sex-specific cut-off")?;
        theme::unverified(ws, w, SC + 2, "NHLBI 1998 obesity guidelines (waist cut-offs)", cx.f)?;
        let (names, vals, units) = (SIZES.range(cx.d, "name"), SIZES.range(cx.d, "value"), SIZES.range(cx.d, "unit"));
        let pick = |v: &str| format!("=IFERROR(LOOKUP(2,1/ISNUMBER(SEARCH(\"waist\",{names})),{v}),\"\")");
        kv(
            ws,
            cx,
            w + 1,
            SC,
            "Waist typed (Bod_Waist_cm)",
            "=IF(ISNUMBER(Bod_Waist_cm),Bod_Waist_cm,\"\")",
            Some("0.0"),
        )?;
        kv(ws, cx, w + 2, SC, "Waist in tblSizes", &pick(&vals), Some("0.0"))?;
        kv(ws, cx, w + 3, SC, "tblSizes unit", &pick(&units), None)?;
        let (typed, sz, unit) = (super::abs(SC + 1, w + 1), super::abs(SC + 1, w + 2), super::abs(SC + 1, w + 3));
        kv(
            ws,
            cx,
            w + 4,
            SC,
            "Waist used (cm)",
            &format!(
                "=IF(ISNUMBER({typed}),{typed},IF(ISNUMBER({sz}),IF(LEFT(LOWER(TRIM({unit})),2)=\"in\",{sz}*2.54,{sz}),\"\"))"
            ),
            Some("0.0"),
        )?;
        kv(ws, cx, w + 5, SC, "Sex (Params)", "=IF(Sex=\"\",\"\",Sex)", None)?;
        kv(
            ws,
            cx,
            w + 6,
            SC,
            "Cut-off (cm)",
            "=IF(LEFT(LOWER(Sex),1)=\"f\",Bod_Waist_f_cm,IF(LEFT(LOWER(Sex),1)=\"m\",Bod_Waist_m_cm,\"\"))",
            Some("0"),
        )?;
        let (wu, cut) = (super::abs(SC + 1, w + 4), super::abs(SC + 1, w + 6));
        kv(
            ws,
            cx,
            w + 7,
            SC,
            "Waist vs cut-off",
            &format!(
                "=IF(NOT(ISNUMBER({wu})),\"no waist on file\",IF(NOT(ISNUMBER({cut})),\"set Sex in Params\",\
                 IF({wu}>{cut},\"above cut-off\",\"at or below cut-off\")))"
            ),
            None,
        )?;
        kv(
            ws,
            cx,
            w + 8,
            SC,
            "Waist-to-height ratio",
            &format!("=IFERROR(IF(AND(ISNUMBER({wu}),Height_cm>0),{wu}/Height_cm,\"\"),\"\")"),
            Some("0.00"),
        )?;
        let wv = super::abs(SC + 1, w + 7);
        theme::cf_formula(ws, w + 7, SC + 1, w + 7, SC + 1, &format!("={wv}=\"above cut-off\""), theme::OUT_OF_REF)?;

        // Rule table: class bounds by name.
        let rr = w + 10;
        section(ws, cx, rr, SC, "Class bounds (values are the Params names)")?;
        let dip = "DIP ch164.md:239-245 (NIH classes, via clinical.md C4.3)";
        let nhlbi = "pending: NHLBI 1998 obesity guidelines (waist risk)";
        let rules: [[&str; 7]; 7] = [
            ["bmi-under", "underweight below", "=Bod_BMI_under", "kg/m2", dip, "verified", "Bod_BMI_under"],
            ["bmi-over", "overweight from", "=Bod_BMI_over", "kg/m2", dip, "verified", "Bod_BMI_over"],
            ["bmi-obese", "obese from", "=Bod_BMI_obese", "kg/m2", dip, "verified", "Bod_BMI_obese"],
            ["bmi-obese3", "obese class 3 from", "=Bod_BMI_obese3", "kg/m2", dip, "verified", "Bod_BMI_obese3"],
            [
                "bmi-refeed",
                "refeeding-risk note below",
                "=Bod_BMI_refeed",
                "kg/m2",
                "DIP ch164.md:89 (via clinical.md C4.3)",
                "verified",
                "Bod_BMI_refeed",
            ],
            ["waist-m", "waist cut-off, men", "=Bod_Waist_m_cm", "cm", nhlbi, "UNVERIFIED", "Bod_Waist_m_cm"],
            ["waist-f", "waist cut-off, women", "=Bod_Waist_f_cm", "cm", nhlbi, "UNVERIFIED", "Bod_Waist_f_cm"],
        ];
        rule_table(ws, cx, "tblBodBmiRef", rr + 1, SC, &rules)?;

        let chart = super::line_chart(
            TITLE,
            "BMI with class lines",
            r0,
            r1,
            0,
            &[
                ("BMI", C_PLOT),
                ("underweight below", C_PLOT + 1),
                ("overweight from", C_PLOT + 2),
                ("obese from", C_PLOT + 3),
            ],
        );
        ws.insert_chart(rr + 10, SC, &chart)?;
        set_widths(ws, &[12.0, 10.0, 10.0, 9.0, 8.0, 13.0, 10.0, 9.0, 10.0, 9.0, 10.0, 2.0, 30.0, 22.0])?;
        ws.set_freeze_panes(HR + 1, 1)?;
        Ok(())
    }
}
