//! Cln_Uric (CLN-19): uric acid with the 6 mg/dL target and 7 mg/dL solubility
//! lines. Cln_NutStatus (CLN-24): the nutrition-status lab bundle with half-lives.
//! Both wait on markers that have never been measured.

use rust_xlsxwriter::{Formula, TableStyle, Worksheet};

use super::ids::{ALBUMIN, CRP, LYMPHOCYTES, PREALBUMIN, TRANSFERRIN, URIC};
use super::kit::{self, c, cf, Lab};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{add_table, set_widths, Cx};

pub const URIC_TITLE: &str = "Cln_Uric";
pub const URIC_TABLE: &str = "tblViewClnUric";
pub const TITLE: &str = "Cln_NutStatus";
pub const TABLE: &str = "tblViewClnNutStatus";

pub fn build_uric(ws: &mut Worksheet, cx: &Cx) -> Result<()> {
    let (d, f) = (cx.d, cx.f);
    let lab = Lab::new(d);
    let row = kit::head(
        ws,
        cx,
        "Uric acid with solubility and target lines",
        &[
            "CLN-19 PLACEHOLDER: above 7 mg/dL urate begins to exceed its solubility (DIP ch115:51); treat-to-target below 6 (ch115:21). Attia tests uric acid as an early metabolic signal (OUT ch13:150-164).",
            "Caveat: asymptomatic hyperuricemia generally needs no therapy (DIP ch115:107).",
        ],
    )?;
    theme::awaiting(ws, row, "uric acid (never measured)", f)?;
    let cols = vec![
        cf("Date", kit::panel_date(), &f.date),
        cf("Uric acid", format!("={}", lab.at(URIC, "[Date]")), &f.dec1),
        c("line 6 target", "=Cln_Urate_target"),
        c("line 7 solubility", "=Cln_Urate_sol"),
        c(
            "Position",
            "=IF(NOT(ISNUMBER([Uric acid])),\"\",IF([Uric acid]>Cln_Urate_sol,\"above solubility\",IF([Uric acid]>=Cln_Urate_target,\"6-7\",\"below 6\")))",
        ),
    ];
    let t = kit::table(ws, URIC_TITLE, URIC_TABLE, row + 2, 0, &cols, kit::n_panels(d))?;
    kit::cf_text(ws, &t, "Position", "above", theme::OUT_OF_REF)?;
    kit::cf_text(ws, &t, "Position", "below 6", theme::OPTIMAL)?;
    ws.insert_chart(
        t.last + 2,
        0,
        &kit::line_chart(&t, "Uric acid", "Date", &["Uric acid", "line 6 target", "line 7 solubility"], "mg/dL"),
    )?;
    set_widths(ws, &[11.0])?;
    Ok(())
}

/// (label, marker, half-life / role note).
const BUNDLE: &[(&str, &str, &str)] = &[
    ("Albumin", ALBUMIN, "long half-life (weeks): slow to reflect intake (DIP ch164:269-297)"),
    ("Total lymphocyte count", LYMPHOCYTES, "TLC < 1,200 cells/mm3: nonspecific malnutrition marker (DIP ch164:297)"),
    ("Prealbumin (transthyretin)", PREALBUMIN, "half-life 2-3 days: the most responsive (DIP ch164:269-297)"),
    ("Transferrin", TRANSFERRIN, "half-life 8-9 days (DIP ch164:269-297)"),
    ("hs-CRP", CRP, "read the others in its context: inflammation lowers them (DIP ch164:269-297)"),
];
const HEADS: &[&str] = &["Item", "Marker", "Half-life / role", "n", "Last value", "Last date", "Status", "Note"];

pub fn build(ws: &mut Worksheet, cx: &Cx) -> Result<()> {
    let (d, f) = (cx.d, cx.f);
    let lab = Lab::new(d);
    let row = kit::head(
        ws,
        cx,
        "Nutrition-status lab bundle",
        &["CLN-24 PLACEHOLDER: albumin and lymphocytes are measured; prealbumin, transferrin and hs-CRP are not. Labs must be read in the context of inflammation (DIP ch164:269-297)."],
    )?;
    theme::awaiting(ws, row, "prealbumin, transferrin, hs-CRP", f)?;
    let hr = row + 2;
    add_table(ws, TABLE, hr, 0, HEADS, BUNDLE.len(), TableStyle::Light9)?;
    for (i, (label, marker, note)) in BUNDLE.iter().enumerate() {
        let r = hr + 1 + i as u32;
        let x = r + 1;
        ws.write_string(r, 0, *label)?;
        ws.write_string(r, 1, *marker)?;
        ws.write_string(r, 2, *note)?;
        ws.write_formula(r, 3, Formula::new(format!("=IF({n}=\"\",0,{n})", n = lab.calc(marker, "n"))))?;
        // Lymphocytes are reported in 10^3/uL; the TLC note needs cells/mm3.
        let last = lab.calc(marker, "last");
        let v =
            if *marker == LYMPHOCYTES { format!("=IF(ISNUMBER({last}),{last}*1000,\"\")") } else { format!("={last}") };
        ws.write_formula_with_format(r, 4, Formula::new(v), &f.dec1)?;
        ws.write_formula_with_format(r, 5, Formula::new(format!("={}", lab.calc(marker, "last_date"))), &f.date)?;
        let st = lab.calc(marker, "status");
        ws.write_formula(
            r,
            6,
            Formula::new(format!(
                "=IF($D{x}=0,\"NOT TESTED\",IFERROR(CHOOSE({st}+1,\"optimal\",\"in reference\",\"out of reference\",\"beyond critical\"),\"\"))"
            )),
        )?;
        if *marker == LYMPHOCYTES {
            ws.write_formula(
                r,
                7,
                Formula::new(format!("=IF(AND(ISNUMBER($E{x}),$E{x}<Cln_TLC_low),\"below 1,200 cells/mm3\",\"\")")),
            )?;
        }
    }
    let (t0, t1) = (hr + 1, hr + BUNDLE.len() as u32);
    theme::cf_formula(ws, t0, 6, t1, 6, &format!("=$G{}=\"NOT TESTED\"", t0 + 1), theme::PLACEHOLDER)?;
    theme::cf_formula(ws, t0, 6, t1, 6, &format!("=$G{}=\"optimal\"", t0 + 1), theme::OPTIMAL)?;
    theme::cf_formula(ws, t0, 6, t1, 6, &format!("=$G{}=\"out of reference\"", t0 + 1), theme::OUT_OF_REF)?;
    set_widths(ws, &[26.0, 18.0, 62.0, 6.0, 11.0, 11.0, 16.0, 24.0])?;
    Ok(())
}
