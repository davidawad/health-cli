//! Cln_Iron (CLN-17): the iron quartet (ferritin, TSAT, MCV, RDW, Hb) per panel
//! with TSAT = iron / TIBC x 100 and bands from DIP ch124, MCV class against the
//! lab's reference, and a neutral pattern flag. Ferritin, iron and TIBC are missing.

use rust_xlsxwriter::Worksheet;

use super::ids::{FERRITIN, HEMOGLOBIN, IRON, MCV, RDW, TIBC};
use super::kit::{self, c, cf, need, Lab};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx};

pub const TITLE: &str = "Cln_Iron";
pub const TABLE: &str = "tblViewClnIron";

pub fn build(ws: &mut Worksheet, cx: &Cx) -> Result<()> {
    let (d, f) = (cx.d, cx.f);
    let lab = Lab::new(d);
    let row = kit::head(
        ws,
        cx,
        "Iron status: ferritin, TSAT, MCV, RDW, hemoglobin",
        &[
            "CLN-17 PLACEHOLDER [DX-RISK]: TSAT = serum iron / TIBC x 100; normal 20-50%, <= 15% typical of iron deficiency (DIP ch124:249-257); MCV micro / normo / macro (ch124:217); RDW (ch124:233).",
            "Ferritin is the best marker of stores but an acute-phase reactant: normal or high ferritin can mask deficiency during inflammation (DIP ch124:249-257).",
        ],
    )?;
    theme::awaiting(ws, row, "ferritin, serum iron and TIBC (hemoglobin, MCV and RDW are present)", f)?;
    kit::dx_banner(ws, row + 1, "iron deficiency, only where the pattern column fills in", cx)?;
    let at = |m: &str| format!("={}", lab.at(m, "[Date]"));
    let b = |m: &str, h: &str| format!("={}", lab.bound(m, h));
    let cols = vec![
        cf("Date", kit::panel_date(), &f.date),
        cf("Hemoglobin", at(HEMOGLOBIN), &f.dec1),
        cf("MCV", at(MCV), &f.dec1),
        cf("RDW", at(RDW), &f.dec1),
        c("Ferritin", at(FERRITIN)),
        c("Iron", at(IRON)),
        c("TIBC", at(TIBC)),
        cf("TSAT %", need(&["[Iron]", "[TIBC]"], "[Iron]/[TIBC]*100"), &f.dec1),
        c("MCV ref lo", b(MCV, "ref_lo")),
        c("MCV ref hi", b(MCV, "ref_hi")),
        c("RDW ref hi", b(RDW, "ref_hi")),
        c(
            "MCV class",
            "=IF(NOT(ISNUMBER([MCV])),\"\",IF(AND(ISNUMBER([MCV ref lo]),[MCV]<[MCV ref lo]),\"microcytic\",IF(AND(ISNUMBER([MCV ref hi]),[MCV]>[MCV ref hi]),\"macrocytic\",\"normocytic\")))",
        ),
        c("RDW flag", "=IF(AND(ISNUMBER([RDW]),ISNUMBER([RDW ref hi])),IF([RDW]>[RDW ref hi],\"above reference\",\"within\"),\"\")"),
        c(
            "TSAT band",
            "=IF(NOT(ISNUMBER([TSAT %])),\"\",IF([TSAT %]<=Cln_TSAT_low,\"<= 15%\",IF([TSAT %]<Cln_TSAT_lo,\"15-20%\",IF([TSAT %]<=Cln_TSAT_hi,\"20-50% normal\",\"> 50%\"))))",
        ),
        c(
            "Pattern",
            kit::dx_if(
                "AND(ISNUMBER([TSAT %]),[TSAT %]<=Cln_TSAT_low,[MCV class]=\"microcytic\")",
                "iron deficiency (TSAT <= 15% with microcytosis, DIP ch124)",
            ),
        ),
    ];
    let t = kit::table(ws, TITLE, TABLE, row + 3, 0, &cols, kit::n_panels(d))?;
    kit::cf_text(ws, &t, "TSAT band", "<= 15%", theme::OUT_OF_REF)?;
    kit::cf_text(ws, &t, "TSAT band", "normal", theme::OPTIMAL)?;
    kit::cf_text(ws, &t, "MCV class", "microcytic", theme::REF_OK)?;
    kit::cf_text(ws, &t, "MCV class", "macrocytic", theme::REF_OK)?;
    ws.insert_chart(
        t.last + 2,
        0,
        &kit::line_chart(&t, "Hemoglobin, MCV, RDW", "Date", &["Hemoglobin", "MCV", "RDW"], "value"),
    )?;
    ws.insert_chart(
        t.last + 2,
        8,
        &kit::line_chart(&t, "Ferritin and TSAT (awaiting data)", "Date", &["Ferritin", "TSAT %"], "value"),
    )?;
    set_widths(ws, &[11.0])?;
    Ok(())
}
