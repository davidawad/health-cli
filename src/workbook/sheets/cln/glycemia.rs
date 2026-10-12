//! Cln_Glycemia (CLN-10): fasting glucose, A1c and insulin per panel with bands
//! (72-85 'thriving', 100 / 126 mg/dL, A1c 5.7 / 6.5 %), and eAG: the lab's reported
//! value is primary; the ADAG formula (UNVERIFIED) fills in when none is reported.

use rust_xlsxwriter::Worksheet;

use super::ids::{A1C, EAG, GLUCOSE, INSULIN};
use super::kit::{self, c, cf, need, Lab};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx};

pub const TITLE: &str = "Cln_Glycemia";
pub const TABLE: &str = "tblViewClnGlycemia";

pub fn build(ws: &mut Worksheet, cx: &Cx) -> Result<()> {
    let (d, f) = (cx.d, cx.f);
    let lab = Lab::new(d);
    let row = kit::head(
        ws,
        cx,
        "Glycemia: glucose, A1c and insulin with bands; eAG reported vs computed",
        &[
            "CLN-10 BUILDABLE-NOW [DX-RISK]: bands 72-85 mg/dL 'thriving' (GLU ch11), 126 mg/dL and A1c 6.5% (DIP ch97:23); 100 mg/dL and 5.7% are UNVERIFIED (ADA).",
            "eAG: the reported value is primary; eAG = 28.7 x A1c - 46.7 is UNVERIFIED (Nathan 2008) and only fills gaps. eAG - glucose shows A1c / fasting-glucose discordance.",
        ],
    )?;
    theme::awaiting(ws, row, "fasting insulin (the insulin column and chart stay empty until it is measured)", f)?;
    kit::dx_banner(ws, row + 1, "diabetes-range glucose or A1c, only where the pattern column fills in", cx)?;
    let at = |m: &str| format!("={}", lab.at(m, "[Date]"));
    let cols = vec![
        cf("Date", kit::panel_date(), &f.date),
        c("Glucose", at(GLUCOSE)),
        cf("A1c", at(A1C), &f.dec1),
        c("Insulin", at(INSULIN)),
        cf("eAG reported", at(EAG), &f.dec1),
        cf("eAG computed", need(&["[A1c]"], "Cln_eAG_slope*[A1c]-Cln_eAG_icpt"), &f.dec1),
        cf("eAG used", "=IF(ISNUMBER([eAG reported]),[eAG reported],[eAG computed])", &f.dec1),
        c(
            "eAG source",
            "=IF(ISNUMBER([eAG reported]),\"reported\",IF(ISNUMBER([eAG computed]),\"computed (UNVERIFIED)\",\"\"))",
        ),
        cf("eAG - glucose", need(&["[eAG used]", "[Glucose]"], "[eAG used]-[Glucose]"), &f.dec1),
        c(
            "Glucose band",
            "=IF(NOT(ISNUMBER([Glucose])),\"\",IF([Glucose]>=Cln_Glu_dx,\">= 126\",IF([Glucose]>=Cln_Glu_pre,\"100-125\",IF(AND([Glucose]>=Cln_Glu_thrive_lo,[Glucose]<=Cln_Glu_thrive_hi),\"72-85 thriving\",\"below 100\"))))",
        ),
        c("line 72", "=Cln_Glu_thrive_lo"),
        c("line 85", "=Cln_Glu_thrive_hi"),
        c("line 100", "=Cln_Glu_pre"),
        c("line 126", "=Cln_Glu_dx"),
        c("line A1c 5.7", "=Cln_A1c_pre"),
        c("line A1c 6.5", "=Cln_A1c_dx"),
        c(
            "Pattern",
            kit::dx_if(
                "OR(AND(ISNUMBER([Glucose]),[Glucose]>=Cln_Glu_dx),AND(ISNUMBER([A1c]),[A1c]>=Cln_A1c_dx))",
                "glucose or A1c at or above the diagnostic cut-points (DIP ch97:23)",
            ),
        ),
    ];
    let hr = row + 3;
    let t = kit::table(ws, TITLE, TABLE, hr, 0, &cols, kit::n_panels(d))?;
    theme::unverified(ws, hr - 1, t.col("eAG computed"), "Nathan 2008 (ADAG)", f)?;
    kit::cf_text(ws, &t, "Glucose band", "thriving", theme::OPTIMAL)?;
    kit::cf_text(ws, &t, "Glucose band", "100-125", theme::OUT_OF_REF)?;
    kit::cf_text(ws, &t, "Glucose band", ">= 126", theme::CRITICAL)?;
    let cr = t.last + 2;
    ws.insert_chart(
        cr,
        0,
        &kit::line_chart(
            &t,
            "Fasting glucose",
            "Date",
            &["Glucose", "line 72", "line 85", "line 100", "line 126"],
            "mg/dL",
        ),
    )?;
    ws.insert_chart(cr, 8, &kit::line_chart(&t, "A1c", "Date", &["A1c", "line A1c 5.7", "line A1c 6.5"], "%"))?;
    ws.insert_chart(cr, 16, &kit::line_chart(&t, "Fasting insulin (awaiting data)", "Date", &["Insulin"], "uIU/mL"))?;
    set_widths(ws, &[11.0])?;
    Ok(())
}
