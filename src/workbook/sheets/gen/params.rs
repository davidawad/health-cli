//! Every constant the GEN sheets use, registered once per build by whichever GEN
//! sheet builds first (so any subset of sheets finds its names).
//!
//! Tunable choices are `heuristic`; numbers asserted as fact without a checked
//! source are `UNVERIFIED` with `pending: <reference>` (blank when no defensible
//! value exists yet). The shared seed `Caffeine_HL` (infra) is reused, not redefined.

use crate::error::Result;
use crate::infra::params::{Param, State};
use crate::workbook::Cx;

use super::variants::CPIC;

/// Register the GEN parameters unless an earlier GEN sheet already did.
pub fn ensure(cx: &Cx) -> Result<()> {
    use State::{Heuristic as H, Unverified as U};
    let mut p = cx.shared.params.borrow_mut();
    if p.get("Gen_Retest_days").is_some() {
        return Ok(());
    }
    for prm in [
        Param::num("Gen_Retest_days", 365.0, "days", H, "heuristic")
            .range(30.0, 3650.0)
            .note("a genotype-triggered lab older than this is due again (GEN-05, GEN-11)"),
        Param::num("Gen_QC_NoCall_max", 0.05, "share", H, "heuristic")
            .range(0.0, 1.0)
            .note("a kit with a higher no-call rate is flagged (GEN-02)"),
        Param::num("Gen_QC_Het_lo", 0.15, "share", H, "heuristic")
            .range(0.0, 1.0)
            .note("heterozygous share of called sites below this is flagged (GEN-02)"),
        Param::num("Gen_QC_Het_hi", 0.40, "share", H, "heuristic")
            .range(0.0, 1.0)
            .note("heterozygous share above this is flagged: contamination or a bad call set (GEN-02)"),
        Param::num("Gen_Adverse_status", 2.0, "status 0-3", H, "heuristic")
            .range(1.0, 3.0)
            .note("Calc_Marker status at or above this counts as an observed abnormality (GEN-01, GEN-04)"),
        Param::num("Gen_HWE_rare", 0.01, "probability", H, "heuristic")
            .range(0.0, 0.5)
            .note("a call less likely than this under Hardy-Weinberg is flagged for re-check (GEN-08)"),
        Param::num("Gen_PGx_base", 2.0, "activity", H, "heuristic")
            .range(0.0, 4.0)
            .note("activity of two reference (fully functional) alleles"),
        Param::num("Gen_PGx_poor_max", 0.5, "activity", U, CPIC)
            .range(0.0, 2.0)
            .note("activity at or below this reads 'poor' (CPIC term standardisation, Caudle 2017)"),
        Param::num("Gen_PGx_normal", 2.0, "activity", U, CPIC)
            .range(0.0, 4.0)
            .note("activity equal to this reads 'normal'; above, 'increased'"),
        Param::blank("Gen_Caffeine_HL_slow", "h", U, "pending: SNPedia rs762551; Cornelis 2006 JAMA 295:1135")
            .range(1.0, 24.0)
            .note("caffeine half-life for a CYP1A2 C carrier; blank keeps Caffeine_HL (GEN-09)"),
        Param::num("Gen_Caffeine_dose_mg", 100.0, "mg", H, "heuristic")
            .range(0.0, 1000.0)
            .note("dose for the residual-caffeine curve (GEN-09)"),
        Param::num("Gen_MEQ_morning", 59.0, "MEQ score", U, "pending: Horne & Ostberg 1976 (Int J Chronobiol 4:97)")
            .range(16.0, 86.0)
            .note("morningness-eveningness score at or above this reads morning type (GEN-09)"),
        Param::num("Gen_MEQ_evening", 41.0, "MEQ score", U, "pending: Horne & Ostberg 1976 (Int J Chronobiol 4:97)")
            .range(16.0, 86.0)
            .note("score at or below this reads evening type (GEN-09)"),
    ] {
        p.add(prm)?;
    }
    Ok(())
}
