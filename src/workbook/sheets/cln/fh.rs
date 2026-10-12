//! Cln_FH (CLN-09): familial-hypercholesterolemia / Lp(a) pattern flag - LDL-C at or
//! above 190 or non-HDL at or above 220 mg/dL (DIP ch37:343) with a premature family
//! history of ASCVD (OUT ch14:161-163). The family history is an input table.

use rust_xlsxwriter::{DataValidation, Worksheet};

use super::ids::{HDL, LDL, TC};
use super::kit::{self, c, cf, need, Lab};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx};

pub const TITLE: &str = "Cln_FH";
pub const TABLE: &str = "tblViewClnFH";
pub const FAMILY: &str = "tblClnFamilyHx";

pub fn build(ws: &mut Worksheet, cx: &Cx) -> Result<()> {
    let (d, f) = (cx.d, cx.f);
    let lab = Lab::new(d);
    let row = kit::head(
        ws,
        cx,
        "FH / Lp(a) family-history pattern flag",
        &[
            "CLN-09 PLACEHOLDER [DX-RISK]: thresholds apply to untreated values (DIP ch37:343); the family-history table below is empty until filled in.",
            "Premature ASCVD in the family is the prompt to test Lp(a) once (OUT ch14:161-163).",
        ],
    )?;
    theme::awaiting(ws, row, "family history of premature ASCVD (enter relatives below; premature = YES/NO)", f)?;
    kit::dx_banner(ws, row + 1, "familial hypercholesterolemia, only where the flag column fills in", cx)?;
    let fr = row + 3;
    kit::section(ws, fr, "Family history (input)", cx)?;
    kit::input_table(ws, cx, FAMILY, fr + 1, 0, &["relative", "condition", "age_at_event", "premature", "note"])?;
    ws.add_data_validation(fr + 2, 3, fr + 200, 3, &DataValidation::new().allow_list_strings(&["YES", "NO"])?)?;
    let at = |m: &str| format!("={}", lab.at(m, "[Date]"));
    let yes = |cell: &str, p: &str| format!("=IF(ISNUMBER({cell}),IF({cell}>={p},\"YES\",\"NO\"),\"\")");
    let cols = vec![
        cf("Date", kit::panel_date(), &f.date),
        c("LDL", at(LDL)),
        c("TC", at(TC)),
        c("HDL", at(HDL)),
        c("non-HDL", need(&["[TC]", "[HDL]"], "[TC]-[HDL]")),
        c("LDL >= 190", yes("[LDL]", "Cln_FH_LDL")),
        c("non-HDL >= 220", yes("[non-HDL]", "Cln_FH_NonHDL")),
        c("Family history", format!("=IF(COUNTIF({FAMILY}[premature],\"YES\")>0,\"YES\",\"NO\")")),
        c(
            "Pattern flag",
            kit::dx_if(
                "AND([Family history]=\"YES\",OR([LDL >= 190]=\"YES\",[non-HDL >= 220]=\"YES\"))",
                "familial hypercholesterolemia with premature family ASCVD; Lp(a) is worth testing once (OUT ch14:161-163)",
            ),
        ),
    ];
    let t = kit::table(ws, TITLE, TABLE, fr + 5, 0, &cols, kit::n_panels(d))?;
    kit::cf_text(ws, &t, "Pattern flag", "Pattern", theme::OUT_OF_REF)?;
    kit::cf_text(ws, &t, "LDL >= 190", "YES", theme::OUT_OF_REF)?;
    kit::cf_text(ws, &t, "non-HDL >= 220", "YES", theme::OUT_OF_REF)?;
    set_widths(ws, &[11.0, 9.0, 9.0, 9.0, 9.0, 11.0, 13.0, 13.0, 60.0])?;
    Ok(())
}
