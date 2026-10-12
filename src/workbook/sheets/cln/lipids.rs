//! Cln_Lipids: CLN-01..05 per panel (TG/HDL, non-HDL / TC:HDL / remnant, LDL
//! method cross-checks, ApoB-LDL concordance, ApoB target lines). CLN-06
//! (ApoB-years) and CLN-07 (expected vs observed) are in [`super::lipids_more`].

use rust_xlsxwriter::{Formula, Worksheet};

use super::ids::{APOB, CHOL_HDL, HDL, LDL, TC, TG, VLDL};
use super::kit::{self, c, cf, need, Lab};
use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{set_widths, Cx};

pub const TITLE: &str = "Cln_Lipids";
/// The per-panel table.
pub const TABLE: &str = "tblViewClnLipids";
/// Martin-Hopkins factor table (empty until someone enters the published grid).
pub const MH_TABLE: &str = "tblClnMHFactor";

pub fn build(ws: &mut Worksheet, cx: &Cx) -> Result<()> {
    let (d, f) = (cx.d, cx.f);
    let lab = Lab::new(d);
    let row = kit::head(
        ws,
        cx,
        "Lipids: ratios, LDL method cross-checks, ApoB concordance and exposure",
        &[
            "CLN-01 BUILDABLE-NOW: TG/HDL per panel with lines at 2 (should be below) and 1 (better yet) - OUT ch13:186.",
            "CLN-02 BUILDABLE-NOW [DX-RISK]: non-HDL = TC - HDL, TC:HDL vs the reported ratio, remnant = TC - HDL - LDL-C; pattern note at non-HDL >= 220 mg/dL (DIP ch37:343).",
            "CLN-03 NEEDS-SOURCE: Friedewald LDL-C = TC - HDL - TG/5 and VLDL = TG/5 (TG < 400) vs the lab; Martin-Hopkins over a factor table to enter. Constants UNVERIFIED on Params.",
            "CLN-04 BUILDABLE-NOW: ApoB vs LDL-C regression (SLOPE / INTERCEPT / RSQ), residual flag beyond Cln_Resid_SD sd, ApoB/LDL ratio vs own history (z).",
            "CLN-05 BUILDABLE-NOW: ApoB with Attia's 20-30 mg/dL goal (OUT ch16); the 100 / 70 lines are LDL-C guideline numbers, not ApoB.",
            "CLN-06 BUILDABLE-NOW and CLN-07 PLACEHOLDER: below the charts.",
        ],
    )?;
    kit::dx_banner(ws, row, "familial hypercholesterolemia, only where the FH note column fills in", cx)?;
    let mh = super::lipids_more::MhFactor::ranges(d);
    let at = |m: &str| format!("={}", lab.at(m, "[Date]"));
    let cols = vec![
        cf("Date", kit::panel_date(), &f.date),
        c("TC", at(TC)),
        c("HDL", at(HDL)),
        c("LDL", at(LDL)),
        c("TG", at(TG)),
        c("ApoB", at(APOB)),
        c("VLDL reported", at(VLDL)),
        c("TC:HDL reported", at(CHOL_HDL)),
        cf("TG/HDL", need(&["[TG]", "[HDL]"], "[TG]/[HDL]"), &f.dec2),
        c("line TG/HDL 1", "=Cln_TGHDL_best"),
        c("line TG/HDL 2", "=Cln_TGHDL_ok"),
        c("non-HDL", need(&["[TC]", "[HDL]"], "[TC]-[HDL]")),
        cf("TC:HDL", need(&["[TC]", "[HDL]"], "[TC]/[HDL]"), &f.dec2),
        cf("TC:HDL diff", need(&["[TC:HDL]", "[TC:HDL reported]"], "[TC:HDL]-[TC:HDL reported]"), &f.dec2),
        c("Remnant", need(&["[TC]", "[HDL]", "[LDL]"], "[TC]-[HDL]-[LDL]")),
        c(
            "FH pattern note",
            kit::dx_if(
                "AND(ISNUMBER([non-HDL]),[non-HDL]>=Cln_FH_NonHDL)",
                "familial hypercholesterolemia (non-HDL >= 220 mg/dL, DIP ch37:343)",
            ),
        ),
        cf(
            "Friedewald LDL",
            need(&["[TC]", "[HDL]", "[TG]"], "IF([TG]>=Cln_Friedewald_TG_max,\"\",[TC]-[HDL]-[TG]/Cln_Friedewald_div)"),
            &f.dec1,
        ),
        cf("Friedewald - lab", need(&["[Friedewald LDL]", "[LDL]"], "[Friedewald LDL]-[LDL]"), &f.dec1),
        cf("VLDL calc", need(&["[TG]"], "IF([TG]>=Cln_Friedewald_TG_max,\"\",[TG]/Cln_Friedewald_div)"), &f.dec1),
        cf("VLDL diff", need(&["[VLDL calc]", "[VLDL reported]"], "[VLDL calc]-[VLDL reported]"), &f.dec1),
        cf(
            "MH factor",
            need(
                &["[TG]", "[non-HDL]"],
                &format!(
                    "IF(SUMIFS({fa},{tl},\"<=\"&[TG],{th},\">\"&[TG],{nl},\"<=\"&[non-HDL],{nh},\">\"&[non-HDL])=0,\"\",SUMIFS({fa},{tl},\"<=\"&[TG],{th},\">\"&[TG],{nl},\"<=\"&[non-HDL],{nh},\">\"&[non-HDL]))",
                    fa = mh.factor,
                    tl = mh.tg_lo,
                    th = mh.tg_hi,
                    nl = mh.nh_lo,
                    nh = mh.nh_hi
                ),
            ),
            &f.dec1,
        ),
        cf("Martin-Hopkins LDL", need(&["[TC]", "[HDL]", "[TG]", "[MH factor]"], "[TC]-[HDL]-[TG]/[MH factor]"), &f.dec1),
        cf("ApoB/LDL", need(&["[ApoB]", "[LDL]"], "[ApoB]/[LDL]"), &f.dec2),
        cf(
            "ApoB/LDL z",
            need(&["[ApoB/LDL]"], "IF(COUNT([[ApoB/LDL]])<2,\"\",([ApoB/LDL]-AVERAGE([[ApoB/LDL]]))/STDEV([[ApoB/LDL]]))"),
            &f.dec2,
        ),
        cf(
            "ApoB fit",
            need(&["[LDL]"], "IF(COUNT([[ApoB]])<2,\"\",SLOPE([[ApoB]],[[LDL]])*[LDL]+INTERCEPT([[ApoB]],[[LDL]]))"),
            &f.dec1,
        ),
        cf("Residual", need(&["[ApoB]", "[ApoB fit]"], "[ApoB]-[ApoB fit]"), &f.dec1),
        c(
            "Residual flag",
            "=IF(AND(ISNUMBER([Residual]),COUNT([[Residual]])>2),IF(ABS([Residual])>Cln_Resid_SD*STDEV([[Residual]]),\"discordant\",\"\"),\"\")",
        ),
        c("line goal 20", "=Cln_ApoB_goal_lo"),
        c("line goal 30", "=Cln_ApoB_goal_hi"),
        c("line LDL-C 100 (not ApoB)", "=Cln_LDL_guide_hi"),
        c("line LDL-C 70 (not ApoB)", "=Cln_LDL_guide_hr"),
    ];
    let hr = row + 2;
    let t = kit::table(ws, TITLE, TABLE, hr, 0, &cols, kit::n_panels(d))?;
    for (h, pending) in [
        ("Friedewald LDL", "Friedewald 1972"),
        ("Martin-Hopkins LDL", "Martin 2013 JAMA 310:2061"),
        ("ApoB fit", "own-history regression, no external threshold"),
    ] {
        theme::unverified(ws, hr - 1, t.col(h), pending, f)?;
    }
    kit::cf_text(ws, &t, "Residual flag", "discordant", theme::OUT_OF_REF)?;
    kit::cf_text(ws, &t, "FH pattern note", "Pattern", theme::OUT_OF_REF)?;
    // Concordance summary.
    let s = t.last + 2;
    kit::section(ws, s, "ApoB ~ LDL-C concordance (CLN-04)", cx)?;
    let (apob, ldl) = (t.range("ApoB"), t.range("LDL"));
    for (i, (label, formula)) in [
        ("pairs", format!("=SUMPRODUCT(ISNUMBER({apob})*ISNUMBER({ldl}))")),
        ("slope", format!("=IFERROR(SLOPE({apob},{ldl}),\"\")")),
        ("intercept", format!("=IFERROR(INTERCEPT({apob},{ldl}),\"\")")),
        ("R squared", format!("=IFERROR(RSQ({apob},{ldl}),\"\")")),
        ("discordant panels", format!("=COUNTIF({},\"discordant\")", t.range("Residual flag"))),
    ]
    .iter()
    .enumerate()
    {
        let r = s + 1 + i as u32;
        ws.write_string(r, 0, *label)?;
        ws.write_formula_with_format(r, 1, Formula::new(formula), &f.dec2)?;
    }
    // Charts (CLN-01, CLN-05, CLN-04).
    let cr = s + 7;
    ws.insert_chart(
        cr,
        0,
        &kit::line_chart(&t, "TG/HDL (CLN-01)", "Date", &["TG/HDL", "line TG/HDL 1", "line TG/HDL 2"], "ratio"),
    )?;
    ws.insert_chart(
        cr,
        8,
        &kit::line_chart(
            &t,
            "ApoB with goal lines; LDL-C guideline lines are not ApoB (CLN-05)",
            "Date",
            &["ApoB", "line goal 20", "line goal 30", "line LDL-C 100 (not ApoB)", "line LDL-C 70 (not ApoB)"],
            "mg/dL",
        ),
    )?;
    ws.insert_chart(cr, 16, &kit::scatter(&t, "ApoB vs LDL-C (CLN-04)", "LDL", "ApoB", true))?;
    let latest = |h: &str| format!("LOOKUP(2,1/ISNUMBER({r}),{r})", r = t.range(h));
    kit::kpi(cx, "tg_hdl", "Latest TG/HDL ratio", &latest("TG/HDL"), Some("0.00"))?;
    kit::kpi(cx, "apob", "Latest ApoB (mg/dL)", &latest("ApoB"), None)?;
    super::lipids_more::build(ws, cx, cr + 17)?;
    set_widths(ws, &[11.0])?;
    ws.set_freeze_panes(hr + 1, 1)?;
    Ok(())
}
