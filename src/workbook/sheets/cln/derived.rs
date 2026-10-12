//! Cln_Derived (CLN-20): miscellaneous derived numbers per panel - NLR, PLR, total
//! lymphocyte count note, BUN:creatinine and A/G cross-checks against the reported
//! ratios, albumin-corrected calcium and the Mentzer index.

use rust_xlsxwriter::Worksheet;

use super::ids::{
    AG_RATIO, ALBUMIN, BUN, BUN_CR, CALCIUM, CREATININE, GLOBULIN, LYMPHOCYTES, MCV, NEUTROPHILS, PLATELETS, RBC,
    TOTAL_PROTEIN,
};
use super::kit::{self, c, cf, need, Lab};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx};

pub const TITLE: &str = "Cln_Derived";
pub const TABLE: &str = "tblViewClnDerived";

pub fn build(ws: &mut Worksheet, cx: &Cx) -> Result<()> {
    let (d, f) = (cx.d, cx.f);
    let lab = Lab::new(d);
    let row = kit::head(
        ws,
        cx,
        "Derived indices: NLR, PLR, TLC, BUN:Cr, A/G, corrected calcium, Mentzer",
        &[
            "CLN-20 NEEDS-SOURCE: NLR = neutrophils / lymphocytes, PLR = platelets / lymphocytes (EXT); TLC < 1,200 cells/mm3 is a nonspecific malnutrition marker (DIP ch164:297).",
            "BUN:Cr and A/G are recomputed and compared with the lab's own ratios (DIP ch164:83). Corrected Ca = Ca + 0.8 x (4 - albumin) and Mentzer = MCV / RBC (cut-off 13) are UNVERIFIED.",
            "Units assumed: lymphocytes and neutrophils 10^3/uL, albumin g/dL, calcium mg/dL.",
        ],
    )?;
    let at = |m: &str| format!("={}", lab.at(m, "[Date]"));
    let cols = vec![
        cf("Date", kit::panel_date(), &f.date),
        cf("Neutrophils", at(NEUTROPHILS), &f.dec2),
        cf("Lymphocytes", at(LYMPHOCYTES), &f.dec2),
        c("Platelets", at(PLATELETS)),
        cf("NLR", need(&["[Neutrophils]", "[Lymphocytes]"], "[Neutrophils]/[Lymphocytes]"), &f.dec2),
        cf("PLR", need(&["[Platelets]", "[Lymphocytes]"], "[Platelets]/[Lymphocytes]"), &f.dec1),
        c("TLC cells/mm3", need(&["[Lymphocytes]"], "[Lymphocytes]*1000")),
        c("TLC note", "=IF(AND(ISNUMBER([TLC cells/mm3]),[TLC cells/mm3]<Cln_TLC_low),\"below 1,200 (DIP ch164:297)\",\"\")"),
        c("BUN", at(BUN)),
        cf("Creatinine", at(CREATININE), &f.dec2),
        cf("BUN:Cr", need(&["[BUN]", "[Creatinine]"], "[BUN]/[Creatinine]"), &f.dec1),
        c("BUN:Cr reported", at(BUN_CR)),
        cf("BUN:Cr diff", need(&["[BUN:Cr]", "[BUN:Cr reported]"], "[BUN:Cr]-[BUN:Cr reported]"), &f.dec1),
        cf("Albumin", at(ALBUMIN), &f.dec1),
        cf("Globulin", at(GLOBULIN), &f.dec1),
        cf("Total protein", at(TOTAL_PROTEIN), &f.dec1),
        cf(
            "Globulin used",
            "=IF(ISNUMBER([Globulin]),[Globulin],IF(COUNT([Total protein],[Albumin])=2,[Total protein]-[Albumin],\"\"))",
            &f.dec1,
        ),
        cf("A/G", need(&["[Albumin]", "[Globulin used]"], "[Albumin]/[Globulin used]"), &f.dec2),
        cf("A/G reported", at(AG_RATIO), &f.dec2),
        cf("A/G diff", need(&["[A/G]", "[A/G reported]"], "[A/G]-[A/G reported]"), &f.dec2),
        cf("Calcium", at(CALCIUM), &f.dec1),
        cf(
            "Corrected Ca",
            need(&["[Calcium]", "[Albumin]"], "[Calcium]+Cln_CorrCa_k*(Cln_CorrCa_alb_ref-[Albumin])"),
            &f.dec2,
        ),
        cf("MCV", at(MCV), &f.dec1),
        cf("RBC", at(RBC), &f.dec2),
        cf("Mentzer", need(&["[MCV]", "[RBC]"], "[MCV]/[RBC]"), &f.dec1),
        c(
            "Mentzer note",
            "=IF(AND(ISNUMBER([Mentzer]),[Mentzer]<Cln_Mentzer_cut),\"below the Mentzer cut-off (UNVERIFIED); discuss with a clinician\",\"\")",
        ),
    ];
    let hr = row + 2;
    let t = kit::table(ws, TITLE, TABLE, hr, 0, &cols, kit::n_panels(d))?;
    for (h, p) in [("NLR", "standard texts (NLR)"), ("Corrected Ca", "Payne 1973"), ("Mentzer", "Mentzer 1973")] {
        theme::unverified(ws, hr - 1, t.col(h), p, f)?;
    }
    kit::cf_text(ws, &t, "TLC note", "below", theme::OUT_OF_REF)?;
    kit::cf_text(ws, &t, "Mentzer note", "below", theme::REF_OK)?;
    ws.insert_chart(t.last + 2, 0, &kit::line_chart(&t, "Neutrophil-to-lymphocyte ratio", "Date", &["NLR"], "ratio"))?;
    set_widths(ws, &[11.0])?;
    Ok(())
}
