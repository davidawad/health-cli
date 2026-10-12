//! Cln_AnionGap (CLN-13): anion gap computed (Na - (Cl + HCO3)) vs reported, bands
//! from DIP ch75 (normal ~9, 3-11; > 17-20 elevated; > 20-25 organic acidosis likely)
//! and the albumin-adjusted gap (UNVERIFIED, Figge).

use rust_xlsxwriter::Worksheet;

use super::ids::{ALBUMIN, ANION_GAP, CHLORIDE, CO2, SODIUM};
use super::kit::{self, c, cf, need, Lab};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx};

pub const TITLE: &str = "Cln_AnionGap";
pub const TABLE: &str = "tblViewClnAnionGap";

pub fn build(ws: &mut Worksheet, cx: &Cx) -> Result<()> {
    let (d, f) = (cx.d, cx.f);
    let lab = Lab::new(d);
    let row = kit::head(
        ws,
        cx,
        "Anion gap: computed vs reported, with sourced bands",
        &[
            "CLN-13 BUILDABLE-NOW [DX-RISK]: AG = Na - (Cl + CO2); normal ~9, range 3-11 mEq/L, 'in excess of 17 to 20' elevated (DIP ch75:199-211); above 20-25 an organic acidosis is likely (ch75:243).",
            "Mild elevations cannot be attributed automatically (DIP ch75:243). Albumin-adjusted AG = AG + 2.5 x (4.4 - albumin) is UNVERIFIED (Figge 1998).",
        ],
    )?;
    kit::dx_banner(ws, row, "an elevated anion gap where the band reads 17-20 or above", cx)?;
    let at = |m: &str| format!("={}", lab.at(m, "[Date]"));
    let cols = vec![
        cf("Date", kit::panel_date(), &f.date),
        c("Sodium", at(SODIUM)),
        c("Chloride", at(CHLORIDE)),
        c("CO2", at(CO2)),
        c("AG reported", at(ANION_GAP)),
        cf("Albumin", at(ALBUMIN), &f.dec1),
        c("AG computed", need(&["[Sodium]", "[Chloride]", "[CO2]"], "[Sodium]-([Chloride]+[CO2])")),
        c("AG diff", need(&["[AG computed]", "[AG reported]"], "[AG computed]-[AG reported]")),
        c("AG", "=IF(ISNUMBER([AG computed]),[AG computed],[AG reported])"),
        c(
            "Band",
            "=IF(NOT(ISNUMBER([AG])),\"\",IF([AG]<Cln_AG_lo,\"below 3\",IF([AG]<=Cln_AG_hi,\"3-11 normal range\",IF([AG]<Cln_AG_elev,\"12-16\",IF([AG]<=Cln_AG_organic,\"17-20 elevated\",\"above 20\")))))",
        ),
        cf("AG albumin-adjusted", need(&["[AG]", "[Albumin]"], "[AG]+Cln_AG_alb_k*(Cln_AG_alb_ref-[Albumin])"), &f.dec1),
        c("line 3", "=Cln_AG_lo"),
        c("line 11", "=Cln_AG_hi"),
        c("line 17", "=Cln_AG_elev"),
        c("line 20", "=Cln_AG_organic"),
    ];
    let hr = row + 2;
    let t = kit::table(ws, TITLE, TABLE, hr, 0, &cols, kit::n_panels(d))?;
    theme::unverified(ws, hr - 1, t.col("AG albumin-adjusted"), "Figge 1998", f)?;
    kit::cf_text(ws, &t, "Band", "normal", theme::OPTIMAL)?;
    kit::cf_text(ws, &t, "Band", "12-16", theme::REF_OK)?;
    kit::cf_text(ws, &t, "Band", "elevated", theme::OUT_OF_REF)?;
    kit::cf_text(ws, &t, "Band", "above 20", theme::CRITICAL)?;
    ws.insert_chart(
        t.last + 2,
        0,
        &kit::line_chart(
            &t,
            "Anion gap with DIP ch75 bands",
            "Date",
            &["AG", "line 3", "line 11", "line 17", "line 20"],
            "mEq/L",
        ),
    )?;
    kit::kpi(
        cx,
        "anion_gap",
        "Latest anion gap (mEq/L)",
        &format!("LOOKUP(2,1/ISNUMBER({r}),{r})", r = t.range("AG")),
        None,
    )?;
    set_widths(ws, &[11.0])?;
    Ok(())
}
