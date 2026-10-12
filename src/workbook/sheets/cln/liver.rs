//! Cln_Liver (CLN-16): ALT trend against its reference, De Ritis ratio (AST/ALT)
//! and FIB-4 = age x AST / (platelets x sqrt(ALT)), shown as numbers only.

use rust_xlsxwriter::Worksheet;

use super::ids::{ALT, AST, PLATELETS};
use super::kit::{self, c, cf, need, Lab};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx};

pub const TITLE: &str = "Cln_Liver";
pub const TABLE: &str = "tblViewClnLiver";

pub fn build(ws: &mut Worksheet, cx: &Cx) -> Result<()> {
    let (d, f) = (cx.d, cx.f);
    let lab = Lab::new(d);
    let row = kit::head(
        ws,
        cx,
        "Liver: ALT trend, De Ritis ratio, FIB-4",
        &[
            "CLN-16 NEEDS-SOURCE [DX-RISK]: Attia watches mildly elevated ALT as an early metabolic sign (OUT ch13:186). FIB-4 (Sterling 2006) and De Ritis are UNVERIFIED formulas.",
            "FIB-4 is a fibrosis screening score: it is shown as a number only, without cut-offs or labels. FIB-4 needs DOB (age at draw).",
        ],
    )?;
    kit::dx_banner(
        ws,
        row,
        "liver disease only with imaging and a clinician's reading; FIB-4 here is a number, not a stage",
        cx,
    )?;
    let at = |m: &str| format!("={}", lab.at(m, "[Date]"));
    let cols = vec![
        cf("Date", kit::panel_date(), &f.date),
        cf("Age", kit::panel_age(), &f.dec1),
        c("ALT", at(ALT)),
        c("AST", at(AST)),
        c("Platelets", at(PLATELETS)),
        cf("De Ritis AST/ALT", need(&["[AST]", "[ALT]"], "[AST]/[ALT]"), &f.dec2),
        cf(
            "FIB-4",
            need(&["[Age]", "[AST]", "[Platelets]", "[ALT]"], "[Age]*[AST]/([Platelets]*SQRT([ALT]))"),
            &f.dec2,
        ),
        c("line ALT ref high", format!("={}", lab.bound(ALT, "ref_hi"))),
    ];
    let hr = row + 2;
    let t = kit::table(ws, TITLE, TABLE, hr, 0, &cols, kit::n_panels(d))?;
    theme::unverified(ws, hr - 1, t.col("De Ritis AST/ALT"), "standard texts (De Ritis 1957)", f)?;
    theme::unverified(ws, hr - 1, t.col("FIB-4"), "Sterling 2006 Hepatology 43:1317", f)?;
    ws.insert_chart(
        t.last + 2,
        0,
        &kit::line_chart(&t, "ALT with the lab's upper reference", "Date", &["ALT", "line ALT ref high"], "U/L"),
    )?;
    ws.insert_chart(t.last + 2, 8, &kit::line_chart(&t, "FIB-4 (number only)", "Date", &["FIB-4"], "FIB-4"))?;
    kit::kpi(
        cx,
        "fib4",
        "Latest FIB-4 (number only)",
        &format!("LOOKUP(2,1/ISNUMBER({r}),{r})", r = t.range("FIB-4")),
        Some("0.00"),
    )?;
    set_widths(ws, &[11.0])?;
    Ok(())
}
