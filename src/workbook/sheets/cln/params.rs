//! Constants of the clinical views, registered on the `Params` sheet.
//!
//! Values quoted in the library (catalog line refs) are `verified` with that
//! citation; standard formulas and thresholds the library lacks are `UNVERIFIED`
//! with the reference to check (`pending: ...`); tuning choices are `heuristic`.
//! Every Cln_* module calls [`ensure`]; the first call registers the set, so a
//! partial build (`--only cln-renal`) still defines the names its formulas use.

use crate::error::Result;
use crate::infra::params::{Param, State};
use crate::workbook::Cx;

fn v(name: &str, x: f64, unit: &str, src: &str) -> Param {
    Param::num(name, x, unit, State::Verified, src)
}
fn u(name: &str, x: f64, unit: &str, pending: &str) -> Param {
    Param::num(name, x, unit, State::Unverified, &format!("pending: {pending}"))
}
fn h(name: &str, x: f64, unit: &str) -> Param {
    Param::num(name, x, unit, State::Heuristic, "heuristic")
}

/// Every Cln_* parameter.
pub fn all() -> Vec<Param> {
    let ckd = "Inker 2021 NEJM 385:1737 (CKD-EPI 2021)";
    let ncep = "NCEP ATP III / AHA 2005 metabolic-syndrome criteria";
    let kdigo = "KDIGO 2012 CKD classification (cited by DIP ch67:173)";
    vec![
        // Lipids (CLN-01..07, CLN-09)
        v("Cln_TGHDL_best", 1.0, "ratio", "OUT ch13:186").note("TG/HDL below 1:1 'better yet'"),
        v("Cln_TGHDL_ok", 2.0, "ratio", "OUT ch13:186").note("TG/HDL should be below 2:1"),
        v("Cln_FH_LDL", 190.0, "mg/dL", "DIP ch37:343").note("LDL-C at or above: FH suspicion threshold"),
        v("Cln_FH_NonHDL", 220.0, "mg/dL", "DIP ch37:343").note("non-HDL at or above: FH suspicion threshold"),
        u("Cln_Friedewald_div", 5.0, "", "Friedewald 1972 (VLDL-C = TG/5, mg/dL)"),
        u("Cln_Friedewald_TG_max", 400.0, "mg/dL", "Friedewald 1972 (invalid at TG >= 400)"),
        h("Cln_Resid_SD", 1.0, "sd").range(0.5, 5.0).note("ApoB-vs-LDL residual flag width"),
        v("Cln_ApoB_goal_lo", 20.0, "mg/dL", "OUT ch16 para 3 (Attia target ~20-30)"),
        v("Cln_ApoB_goal_hi", 30.0, "mg/dL", "OUT ch16 para 3 (Attia target ~20-30)"),
        h("Cln_ApoB_baseline", 25.0, "mg/dL").note("child-like baseline: midpoint of OUT ch16 20-30"),
        v("Cln_LDL_guide_hi", 100.0, "mg/dL", "OUT ch15 paras 7-17 (guideline LDL-C, not ApoB)"),
        v("Cln_LDL_guide_hr", 70.0, "mg/dL", "OUT ch15 paras 7-17 (guideline LDL-C high risk, not ApoB)"),
        v("Cln_Post_Days", 28.0, "days", "DIP ch37:451 (repeat lipids 4-12 weeks after a change)"),
        // Glycemia and insulin (CLN-10..12)
        v("Cln_Glu_thrive_lo", 72.0, "mg/dL", "GLU ch11 (fasting 'thriving' 72-85)"),
        v("Cln_Glu_thrive_hi", 85.0, "mg/dL", "GLU ch11 (fasting 'thriving' 72-85)"),
        u("Cln_Glu_pre", 100.0, "mg/dL", "ADA Standards of Care (impaired fasting glucose)"),
        v("Cln_Glu_dx", 126.0, "mg/dL", "DIP ch97:23 (diagnostic cut-point)"),
        u("Cln_A1c_pre", 5.7, "%", "ADA Standards of Care (prediabetes A1c)"),
        v("Cln_A1c_dx", 6.5, "%", "DIP ch97:23; OUT ch13:182"),
        u("Cln_eAG_slope", 28.7, "mg/dL per %", "Nathan 2008 Diabetes Care (ADAG)"),
        u("Cln_eAG_icpt", 46.7, "mg/dL", "Nathan 2008 Diabetes Care (ADAG)"),
        u("Cln_TyG_div", 2.0, "", "Simental-Mendia 2008 (TyG = ln(TG*glucose/2))"),
        u("Cln_HOMA_div", 405.0, "", "Matthews 1985 Diabetologia 28:412 (mg/dL units)"),
        // Anion gap (CLN-13)
        v("Cln_AG_lo", 3.0, "mEq/L", "DIP ch75:199-211 (normal range 3-11)"),
        v("Cln_AG_typ", 9.0, "mEq/L", "DIP ch75:199-211 (normal ~9)"),
        v("Cln_AG_hi", 11.0, "mEq/L", "DIP ch75:199-211 (normal range 3-11)"),
        v("Cln_AG_elev", 17.0, "mEq/L", "DIP ch75:199-211 ('in excess of 17 to 20')"),
        v("Cln_AG_organic", 20.0, "mEq/L", "DIP ch75:243 ('exceeds 20 to 25')"),
        u("Cln_AG_alb_k", 2.5, "mEq/L per g/dL", "Figge 1998 (albumin-adjusted anion gap)"),
        u("Cln_AG_alb_ref", 4.4, "g/dL", "Figge 1998 (albumin-adjusted anion gap)"),
        // Renal (CLN-14, CLN-15)
        u("Cln_CKDEPI_A", 142.0, "", ckd),
        u("Cln_CKDEPI_kappa_f", 0.7, "mg/dL", ckd),
        u("Cln_CKDEPI_kappa_m", 0.9, "mg/dL", ckd),
        u("Cln_CKDEPI_alpha_f", -0.241, "", ckd),
        u("Cln_CKDEPI_alpha_m", -0.302, "", ckd),
        u("Cln_CKDEPI_beta", -1.2, "", ckd),
        u("Cln_CKDEPI_age", 0.9938, "per year", ckd),
        u("Cln_CKDEPI_fem", 1.012, "", ckd),
        u("Cln_CG_fem", 0.85, "", "Cockcroft & Gault 1976 Nephron 16:31"),
        u("Cln_CG_div", 72.0, "", "Cockcroft & Gault 1976 Nephron 16:31"),
        h("Cln_Creatine_days", 56.0, "days").range(0.0, 365.0).note("creatine before a draw (catalog C5.2: 4-8 weeks)"),
        u("Cln_KDIGO_G2", 90.0, "mL/min/1.73m2", kdigo),
        u("Cln_KDIGO_G3a", 60.0, "mL/min/1.73m2", kdigo),
        u("Cln_KDIGO_G3b", 45.0, "mL/min/1.73m2", kdigo),
        u("Cln_KDIGO_G4", 30.0, "mL/min/1.73m2", kdigo),
        u("Cln_KDIGO_G5", 15.0, "mL/min/1.73m2", kdigo),
        u("Cln_KDIGO_A2", 30.0, "mg/g", kdigo),
        u("Cln_KDIGO_A3", 300.0, "mg/g", kdigo),
        // Iron, thyroid, urate, nutrition, misc (CLN-17..20, CLN-24)
        v("Cln_TSAT_low", 15.0, "%", "DIP ch124:249-257 (<=15% typical of iron deficiency)"),
        v("Cln_TSAT_lo", 20.0, "%", "DIP ch124:249-257 (normal 20-50%)"),
        v("Cln_TSAT_hi", 50.0, "%", "DIP ch124:249-257 (normal 20-50%)"),
        v("Cln_Biotin_mcg", 5000.0, "mcg/day", "DIP ch98:233 (>5,000 mcg/day interferes)"),
        v("Cln_Biotin_hold_h", 48.0, "h", "DIP ch98:233 (hold 24-48 h before a draw)"),
        v("Cln_Urate_sol", 7.0, "mg/dL", "DIP ch115:51 (solubility limit)"),
        v("Cln_Urate_target", 6.0, "mg/dL", "DIP ch115:21 (treat-to-target)"),
        v("Cln_TLC_low", 1200.0, "cells/mm3", "DIP ch164:297 (nonspecific malnutrition marker)"),
        u("Cln_CorrCa_k", 0.8, "mg/dL per g/dL", "Payne 1973 BMJ (corrected calcium)"),
        u("Cln_CorrCa_alb_ref", 4.0, "g/dL", "Payne 1973 BMJ (corrected calcium)"),
        u("Cln_Mentzer_cut", 13.0, "", "Mentzer 1973 Lancet (MCV/RBC)"),
        // Metabolic syndrome and BMI (CLN-21, CLN-22)
        v("Cln_MetS_TG", 150.0, "mg/dL", "OUT ch13:53-65 (TG > 150)"),
        v("Cln_MetS_HDL_m", 40.0, "mg/dL", "OUT ch13:53-65 (HDL < 40 men)"),
        v("Cln_MetS_HDL_f", 50.0, "mg/dL", "OUT ch13:53-65 (HDL < 50 women)"),
        v("Cln_MetS_Glu", 110.0, "mg/dL", "OUT ch13:53-65 (as printed; AHA 2005 uses 100)"),
        v("Cln_MetS_Waist_m", 40.0, "in", "OUT ch13:53-65 (waist > 40 in men)"),
        v("Cln_MetS_Waist_f", 35.0, "in", "OUT ch13:53-65 (waist > 35 in women)"),
        u("Cln_MetS_SBP", 130.0, "mmHg", ncep),
        u("Cln_MetS_DBP", 85.0, "mmHg", ncep),
        u("Cln_MetS_n", 3.0, "criteria", ncep),
        u("Cln_BMI_cut", 25.0, "kg/m2", "WHO BMI classification (overweight)"),
        h("Cln_Vitals_days", 30.0, "days")
            .range(0.0, 365.0)
            .note("vitals within this many days of a draw count for it"),
        // Response loops (CLN-25)
        u("Cln_VitD_lo", 20.0, "ng/mL", "SR-papers vitamin D (stress-fracture trials)"),
        u("Cln_VitD_hi", 40.0, "ng/mL", "SR-papers vitamin D (stress-fracture trials)"),
    ]
}

/// Register the Cln_* parameters once per build.
pub fn ensure(cx: &Cx) -> Result<()> {
    let mut p = cx.shared.params.borrow_mut();
    if p.get("Cln_TGHDL_best").is_some() {
        return Ok(());
    }
    all().into_iter().try_for_each(|x| p.add(x))
}
