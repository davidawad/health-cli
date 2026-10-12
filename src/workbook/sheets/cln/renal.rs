//! Cln_Renal (CLN-14): eGFR by CKD-EPI 2021 and creatinine clearance by
//! Cockcroft-Gault vs the reported eGFR, plus the creatine-supplement flag on
//! creatinine. Constants are UNVERIFIED on Params; the KDIGO grid (CLN-15) is in
//! [`super::renal_grid`].

use rust_xlsxwriter::Worksheet;

use super::ids::{CREATININE, EGFR};
use super::kit::{self, c, cf, need, Lab};
use crate::error::Result;
use crate::infra::data::{PANELS, SUPP};
use crate::infra::theme;
use crate::workbook::{col, set_widths, Cx};

pub const TITLE: &str = "Cln_Renal";
pub const TABLE: &str = "tblViewClnRenal";

fn ckd_epi(kappa: &str, alpha: &str, fem: bool) -> String {
    let x = format!("[Creatinine]/{kappa}");
    need(
        &["[Creatinine]", "[Age]"],
        &format!(
            "Cln_CKDEPI_A*MIN({x},1)^{alpha}*MAX({x},1)^Cln_CKDEPI_beta*Cln_CKDEPI_age^[Age]{}",
            if fem { "*Cln_CKDEPI_fem" } else { "" }
        ),
    )
}

pub fn build(ws: &mut Worksheet, cx: &Cx) -> Result<()> {
    let (d, f) = (cx.d, cx.f);
    let lab = Lab::new(d);
    let row = kit::head(
        ws,
        cx,
        "Kidney: eGFR CKD-EPI 2021, Cockcroft-Gault, creatine flag, KDIGO grid",
        &[
            "CLN-14 NEEDS-SOURCE [DX-RISK]: eGFR = 142 x min(Scr/k,1)^a x max(Scr/k,1)^-1.200 x 0.9938^age (x 1.012 female); coefficients UNVERIFIED (Inker 2021). Set Sex on Params; both columns are shown until then.",
            "Creatine raises serum creatinine without changing kidney function (mechanism EXT, unverified): the dated flag needs tblSupp start/stop dates (GATE-DATES); 'in current stack' reads the taking column.",
            "CLN-15 PLACEHOLDER [DX-RISK]: KDIGO eGFR x uACR grid below (DIP ch67:173), awaiting uACR.",
        ],
    )?;
    theme::awaiting(
        ws,
        row,
        "supplement start/stop dates for the dated creatine flag (tblSupp.start_date is empty)",
        f,
    )?;
    kit::dx_banner(
        ws,
        row + 1,
        "reduced kidney filtration only when computed and reported eGFR are both low on repeat draws",
        cx,
    )?;
    let ns = SUPP.nrows(d);
    let (names, taking, starts, stops) = (
        SUPP.range_n(ns, "name"),
        SUPP.range_n(ns, "taking"),
        SUPP.range_n(ns, "start_date"),
        SUPP.range_n(ns, "stop_date"),
    );
    let wc = col(PANELS.col("weight_at_panel"));
    let cols = vec![
        cf("Date", kit::panel_date(), &f.date),
        cf("Age", kit::panel_age(), &f.dec1),
        c("Sex", "=IF(Sex=\"\",\"\",LOWER(LEFT(Sex,1)))"),
        cf(
            "Weight kg",
            format!("=IF(ISNUMBER({s}!${wc}${{p}}),{s}!${wc}${{p}}*0.45359237,\"\")", s = PANELS.sheet),
            &f.dec1,
        ),
        cf("Creatinine", format!("={}", lab.at(CREATININE, "[Date]")), &f.dec2),
        c("eGFR reported", format!("={}", lab.at(EGFR, "[Date]"))),
        cf("eGFR if female", ckd_epi("Cln_CKDEPI_kappa_f", "Cln_CKDEPI_alpha_f", true), &f.int),
        cf("eGFR if male", ckd_epi("Cln_CKDEPI_kappa_m", "Cln_CKDEPI_alpha_m", false), &f.int),
        cf("eGFR CKD-EPI 2021", "=IF([Sex]=\"f\",[eGFR if female],IF([Sex]=\"m\",[eGFR if male],\"\"))", &f.int),
        cf("Calc - reported", need(&["[eGFR CKD-EPI 2021]", "[eGFR reported]"], "[eGFR CKD-EPI 2021]-[eGFR reported]"), &f.int),
        cf("eGFR used", "=IF(ISNUMBER([eGFR CKD-EPI 2021]),[eGFR CKD-EPI 2021],[eGFR reported])", &f.int),
        cf(
            "CrCl Cockcroft-Gault",
            need(
                &["[Age]", "[Weight kg]", "[Creatinine]"],
                "IF([Sex]=\"\",\"\",(140-[Age])*[Weight kg]*IF([Sex]=\"f\",Cln_CG_fem,1)/(Cln_CG_div*[Creatinine]))",
            ),
            &f.int,
        ),
        c(
            "Creatine in current stack",
            format!("=IF(COUNTIFS({names},\"*creatin*\",{taking},\"YES\")>0,\"YES (dates unknown)\",\"NO\")"),
        ),
        c(
            "Creatine before draw",
            format!(
                "=IF(COUNTIFS({names},\"*creatin*\",{starts},\"<=\"&[Date],{starts},\">0\")-COUNTIFS({names},\"*creatin*\",{starts},\"<=\"&[Date],{stops},\"<\"&([Date]-Cln_Creatine_days),{stops},\">0\")>0,\"creatine within Cln_Creatine_days: creatinine / eGFR may read high\",\"\")"
            ),
        ),
    ];
    let hr = row + 3;
    let t = kit::table(ws, TITLE, TABLE, hr, 0, &cols, kit::n_panels(d))?;
    for h in ["eGFR if female", "eGFR if male", "CrCl Cockcroft-Gault"] {
        theme::unverified(
            ws,
            hr - 1,
            t.col(h),
            if h.starts_with("CrCl") { "Cockcroft-Gault 1976" } else { "Inker 2021" },
            f,
        )?;
    }
    kit::cf_text(ws, &t, "Creatine in current stack", "YES", theme::REF_OK)?;
    kit::cf_text(ws, &t, "Creatine before draw", "creatine", theme::OUT_OF_REF)?;
    ws.insert_chart(
        t.last + 2,
        0,
        &kit::line_chart(
            &t,
            "eGFR: computed vs reported",
            "Date",
            &["eGFR reported", "eGFR if female", "eGFR if male"],
            "mL/min/1.73m2",
        ),
    )?;
    let egfr = format!("LOOKUP(2,1/ISNUMBER({r}),{r})", r = t.range("eGFR used"));
    kit::kpi(cx, "egfr", "Latest eGFR (CKD-EPI 2021 when Sex is set, else reported)", &egfr, Some("0"))?;
    super::renal_grid::build(ws, cx, t.last + 19, &egfr)?;
    set_widths(ws, &[11.0])?;
    Ok(())
}
