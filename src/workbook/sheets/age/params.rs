//! Every constant the AGE sheets use, registered once per build by whichever AGE
//! sheet builds first (so any subset of sheets finds its names).
//!
//! States: values printed in a catalogued source (file + line, or Levine 2018
//! Table 1) ship `verified` with that citation; constants only seen in a
//! secondary source are `UNVERIFIED` with `pending: <reference>`; tunable choices
//! are `heuristic`. The shared seeds `PhenoAge_*`, `Decline_Strength_pct_decade`
//! and `Decline_VO2_pct_decade` (infra) are reused, not redefined.

use crate::error::Result;
use crate::infra::params::{Param, State};
use crate::workbook::Cx;

const LEVINE: &str = "Levine 2018 Table 1 (PMC5940111)";

/// (name, value, unit, state, source, note).
type Row = (&'static str, f64, &'static str, State, &'static str, &'static str);

fn rows() -> Vec<Row> {
    use State::{Heuristic as H, Unverified as U, Verified as V};
    vec![
        // AGE-01..03 PhenoAge: weights per Levine 2018 Table 1.
        ("Age_PA_w_albumin", -0.0336, "per g/L", V, LEVINE, ""),
        ("Age_PA_w_creatinine", 0.0095, "per umol/L", V, LEVINE, ""),
        ("Age_PA_w_glucose", 0.1953, "per mmol/L", V, LEVINE, ""),
        ("Age_PA_w_crp", 0.0954, "per ln(mg/dL)", V, LEVINE, ""),
        ("Age_PA_w_lymph", -0.0120, "per %", V, LEVINE, ""),
        ("Age_PA_w_mcv", 0.0268, "per fL", V, LEVINE, ""),
        ("Age_PA_w_rdw", 0.3306, "per %", V, LEVINE, ""),
        ("Age_PA_w_alp", 0.0019, "per U/L", V, LEVINE, "the mcp-phenoage-clock README uses the unrounded 0.00188"),
        ("Age_PA_w_wbc", 0.0554, "per 10^3/uL", V, LEVINE, ""),
        ("Age_PA_w_age", 0.0804, "per year", V, LEVINE, ""),
        (
            "Age_PA_t_months",
            120.0,
            "months",
            U,
            "pending: Levine 2018 Supplement 1",
            "mortality horizon in the Gompertz M",
        ),
        ("Age_PA_u_albumin", 10.0, "x", V, "SI conversion: g/dL x10 = g/L", "multiply"),
        ("Age_PA_u_creatinine", 88.42, "x", V, "SI conversion: creatinine 113.12 g/mol", "multiply mg/dL to umol/L"),
        ("Age_PA_u_glucose", 18.016, "/", V, "SI conversion: glucose 180.16 g/mol", "divide mg/dL to mmol/L"),
        ("Age_PA_u_crp", 10.0, "/", V, "SI conversion: mg/L /10 = mg/dL", "divide"),
        ("Age_PA_crp_floor", 0.01, "mg/dL", H, "heuristic", "CRP of 0 breaks the log (longevity.md L-A1)"),
        // AGE-06/07 tagging.
        (
            "Age_Gap_max",
            1.0,
            "interventions",
            H,
            "heuristic",
            "a hallmark or SENS row with at most this many is flagged",
        ),
        // AGE-09 four horsemen.
        ("Age_Stale_days", 365.0, "days", H, "heuristic", "an item older than this counts as stale"),
        // AGE-11 / 18 decathlon and decade planning.
        ("Age_Target_age", 100.0, "years", H, "heuristic", "the age the Centenarian Decathlon plans for"),
        // AGE-12 VO2 max.
        (
            "Age_VO2_decline_post50_pct",
            15.0,
            "%/decade",
            U,
            "pending: OUT ch34",
            "Decline_VO2_pct_decade applies before 50",
        ),
        ("Age_VO2_floor_m", 18.0, "ml/kg/min", V, "OUT ch34 (via longevity.md L-C2)", "independence floor, men"),
        ("Age_VO2_floor_f", 15.0, "ml/kg/min", V, "OUT ch34 (via longevity.md L-C2)", "independence floor, women"),
        ("Age_VO2_hike", 30.0, "ml/kg/min", V, "OUT ch34 l.24-26", "hiking a hilly trail"),
        ("Age_VO2_stairs", 32.0, "ml/kg/min", V, "OUT ch31 l.55", "90 steps in 2 min"),
        ("Age_VO2_marchand", 35.0, "ml/kg/min", V, "OUT ch34 (via longevity.md L-C2)", "Robert Marchand at 100+"),
        ("Age_VO2_train_gain", 6.0, "ml/kg/min", V, "OUT ch34 (via longevity.md L-C2)", "the 'with training' line"),
        ("Age_VO2_RHR_k", 15.3, "x", U, "pending: Uth 2004 (Eur J Appl Physiol 91:111)", "VO2 ~ k x HRmax/HRrest"),
        ("Age_VO2_test_days", 365.0, "days", V, "OUT ch34 l.14", "annual lab test"),
        // AGE-13/14 zone 2.
        ("Age_Z2_target_min", 180.0, "min/week", V, "OUT ch33", "about three hours per week"),
        ("Age_Z2_lo_pct", 0.70, "of HRmax", V, "OUT ch33 (via longevity.md L-C6)", "of the true maximum"),
        ("Age_Z2_hi_pct", 0.85, "of HRmax", V, "OUT ch33 (via longevity.md L-C6)", "of the true maximum"),
        ("Age_HRmax_a", 208.0, "bpm", U, "pending: Tanaka 2001 (JACC 37:153)", "fallback HRmax = a - b x age"),
        ("Age_HRmax_b", 0.7, "bpm/year", U, "pending: Tanaka 2001 (JACC 37:153)", ""),
        ("Age_Wkg_fit", 2.0, "W/kg", V, "OUT ch33 (watts/kg)", "reasonably fit"),
        ("Age_Wkg_very_fit", 3.0, "W/kg", V, "OUT ch33 (watts/kg)", "very fit"),
        ("Age_Wkg_pro", 4.0, "W/kg", V, "OUT ch33 (watts/kg)", "professional cyclists"),
        ("Age_4x4_gate_weeks", 22.0, "weeks", H, "heuristic", "OUT ch34 l.5 says 5-6 months of consistent zone 2"),
        ("Age_Drift_flag", 0.05, "fraction", H, "heuristic", "cardiac drift threshold (longevity.md L-C17: mine)"),
        // AGE-15 strength.
        ("Age_Carry_bw", 1.0, "x bodyweight", V, "OUT ch35 l.57-65", "farmer's carry total load for 1 min (men)"),
        ("Age_Carry_f_share", 0.75, "x men's load", V, "OUT ch35 l.57-65", "women: 75% of the men's carry"),
        ("Age_Ruck_bw", 0.25, "x bodyweight", V, "OUT ch35 l.73-77", "rucking load goal 25-33% of bodyweight"),
        ("Age_Hang_s_m", 120.0, "s", V, "OUT ch35 l.57-65", "dead hang at 40, men"),
        ("Age_Hang_s_f", 90.0, "s", V, "OUT ch35 l.57-65", "dead hang at 40, women"),
        ("Age_Train_keep", 0.5, "fraction", H, "heuristic", "share of the decline left when training"),
        // AGE-16 volume.
        ("Age_HardSet_RIR", 5.0, "RIR", V, "ISRA ch06 l.217-233", "a set counts when within this many reps of failure"),
        ("Age_Meso_step", 2.0, "sets/week", H, "heuristic", "weekly ramp of the mesocycle planner"),
        ("Age_Lift_sessions", 4.0, "per week", V, "OUT ch30 l.128", "Attia's own routine, not a prescription"),
        (
            "Age_Cardio_lift_max",
            0.5,
            "ratio",
            V,
            "HELMS ch01 l.1047-1061",
            "cardio time at most half of lifting time (dieting)",
        ),
        // AGE-17 sauna and walks.
        ("Age_Sauna_ref", 4.0, "per week", H, "heuristic", "Finnish cohort: up to 7x/week (LIFE l.4219-4229)"),
        ("Age_Walk_min", 10.0, "min", V, "GLU ch23 (Hack 8)", "post-meal walk length"),
    ]
}

/// Register every AGE parameter unless an earlier AGE sheet already did.
pub fn ensure(cx: &Cx) -> Result<()> {
    let mut p = cx.shared.params.borrow_mut();
    if p.get("Age_PA_w_albumin").is_some() {
        return Ok(());
    }
    for (name, value, unit, state, source, note) in rows() {
        let lim = if value < 0.0 { (value * 10.0, 0.0) } else { (0.0, (value * 10.0).max(1.0)) };
        p.add(Param::num(name, value, unit, state, source).range(lim.0, lim.1).note(note))?;
    }
    Ok(())
}

/// Every parameter name (for tests and the coverage doc).
pub fn names() -> Vec<&'static str> {
    rows().into_iter().map(|r| r.0).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::params::ParamSet;

    #[test]
    fn parameters_are_valid_unique_and_cited() {
        let mut set = ParamSet::default();
        crate::infra::params::seed(&mut set);
        for (name, value, unit, state, source, _) in rows() {
            set.add(Param::num(name, value, unit, state, source)).unwrap();
            assert!(name.starts_with("Age_"), "{name}");
            assert!(state != State::Verified || !source.is_empty(), "{name} cites its source");
        }
    }
}
