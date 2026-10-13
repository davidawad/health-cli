//! Every constant the SCH sheets use, registered once per build by whichever SCH
//! sheet builds first (so any subset of sheets finds its names).
//!
//! States: values printed in a catalogued source (file + line) ship `verified`
//! with that citation; constants seen only outside the library are `UNVERIFIED`
//! with `pending: <reference>`; tunable choices (windows, weights, clock hours)
//! are `heuristic`. Vaccine intervals and screening ages are rule rows in
//! `Ref_Vax` / `Ref_Screen` ([`super::rules`]), not parameters.

use crate::error::Result;
use crate::infra::params::{Param, State};
use crate::workbook::Cx;

const CDC_BP: &str = "pending: CDC General Best Practice Guidelines for Immunization";

/// (name, value, unit, state, source, min, max, note).
type Row = (&'static str, f64, &'static str, State, &'static str, f64, f64, &'static str);

fn rows() -> Vec<Row> {
    use State::{Heuristic as H, Unverified as U, Verified as V};
    vec![
        // SCH-01 vaccines.
        ("Sch_Vax_Soon_d", 90.0, "days", H, "heuristic", 0.0, 365.0, "'due soon' window (clinical.md 10.1)"),
        (
            "Sch_Live_Gap_d",
            28.0,
            "days",
            U,
            CDC_BP,
            0.0,
            365.0,
            "minimum spacing of two live vaccines not given the same day",
        ),
        (
            "Sch_Ig_Gap_months",
            3.0,
            "months",
            U,
            CDC_BP,
            0.0,
            24.0,
            "immune globulin before a live vaccine; dose-related (DIP ch148.md:87)",
        ),
        // SCH-02 screening.
        ("Sch_Screen_Soon_d", 90.0, "days", H, "heuristic", 0.0, 365.0, "'due soon' window"),
        // SCH-03 providers.
        ("Sch_Prov_Soon_d", 30.0, "days", H, "heuristic", 0.0, 365.0, "'due soon' window"),
        ("Sch_Int_physical_m", 12.0, "months", H, "heuristic", 1.0, 120.0, "default interval: physical"),
        ("Sch_Int_dental_m", 6.0, "months", H, "heuristic", 1.0, 120.0, "default interval: dental"),
        ("Sch_Int_eye_m", 24.0, "months", H, "heuristic", 1.0, 120.0, "default interval: eye"),
        ("Sch_Int_specialist_m", 12.0, "months", H, "heuristic", 1.0, 120.0, "default interval: specialist"),
        ("Sch_Int_other_m", 12.0, "months", H, "heuristic", 1.0, 120.0, "default interval: other"),
        // SCH-04 monitoring windows after a therapy change.
        ("Sch_Mon_lipid_lo_wk", 4.0, "weeks", V, "DIP ch37.md:451", 0.0, 104.0, "repeat lipid panel 4-12 weeks"),
        ("Sch_Mon_lipid_hi_wk", 12.0, "weeks", V, "DIP ch37.md:451", 0.0, 104.0, "repeat lipid panel 4-12 weeks"),
        ("Sch_Mon_a1c_lo_wk", 9.0, "weeks", V, "DIP ch97.md:197", 0.0, 104.0, "A1c reflects 2-3 months"),
        ("Sch_Mon_a1c_hi_wk", 13.0, "weeks", V, "DIP ch97.md:197", 0.0, 104.0, "A1c reflects 2-3 months"),
        ("Sch_Mon_thyroid_lo_wk", 26.0, "weeks", V, "DIP ch98.md:345", 0.0, 104.0, "follow-up every 6-12 months"),
        ("Sch_Mon_thyroid_hi_wk", 52.0, "weeks", V, "DIP ch98.md:345", 0.0, 104.0, "follow-up every 6-12 months"),
        // SCH-05 retest cadence.
        (
            "Sch_MMC_pct",
            0.10,
            "of mean",
            H,
            "heuristic",
            0.01,
            1.0,
            "minimal meaningful change until CVi/CVa (EFLM) are filled",
        ),
        ("Sch_Retest_base_d", 180.0, "days", H, "heuristic", 7.0, 1825.0, "cadence of a marker at the reference CV"),
        ("Sch_Retest_min_d", 60.0, "days", H, "heuristic", 7.0, 1825.0, "shortest cadence"),
        ("Sch_Retest_max_d", 365.0, "days", H, "heuristic", 7.0, 1825.0, "longest cadence"),
        ("Sch_Retest_cv_ref", 0.10, "CV", H, "heuristic", 0.01, 1.0, "inverse-vol scaling: cadence x ref / CV"),
        ("Sch_Retest_out_d", 90.0, "days", H, "heuristic", 7.0, 1825.0, "cadence of a marker out of range"),
        ("Sch_Retest_Soon_d", 30.0, "days", H, "heuristic", 0.0, 365.0, "'due soon' window"),
        // SCH-06 pre-draw.
        (
            "Sch_Biotin_mcg",
            5000.0,
            "mcg/day",
            V,
            "DIP ch98.md:233",
            0.0,
            100000.0,
            ">5,000 mcg/day associated with major assay interference",
        ),
        ("Sch_Biotin_hold_h", 48.0, "h", V, "DIP ch98.md:233", 0.0, 168.0, "hold biotin 24 to 48 hours before a draw"),
        ("Sch_Fast_h", 10.0, "h", V, "KAT ch46.md:77", 0.0, 24.0, "lipids measured after a 10-hour fast"),
        ("Sch_Draw_h", 8.0, "clock h", H, "heuristic", 0.0, 24.0, "assumed draw time when none is typed"),
        // SCH-07 / 11 dosing slots (clock hours).
        ("Sch_Slot_morning_h", 8.0, "clock h", H, "heuristic", 0.0, 24.0, "morning slot"),
        ("Sch_Slot_noon_h", 12.0, "clock h", H, "heuristic", 0.0, 24.0, "noon slot"),
        ("Sch_Slot_evening_h", 18.0, "clock h", H, "heuristic", 0.0, 24.0, "evening slot"),
        ("Sch_Slot_bedtime_h", 22.0, "clock h", H, "heuristic", 0.0, 24.0, "bedtime slot"),
        // SCH-08 pill burden and complexity.
        ("Sch_CX_item", 1.0, "points", H, "heuristic", 0.0, 10.0, "per item taken (KAT ch81.md:91: fewer drugs)"),
        ("Sch_CX_slot", 1.0, "points", H, "heuristic", 0.0, 10.0, "per distinct dosing slot (fewer doses per day)"),
        ("Sch_CX_nondaily", 1.0, "points", H, "heuristic", 0.0, 10.0, "per item on a non-daily schedule"),
        ("Sch_CX_food", 0.5, "points", H, "heuristic", 0.0, 10.0, "per item with a food instruction"),
        ("Sch_Slot_crowd", 6.0, "pills", H, "heuristic", 1.0, 50.0, "a slot above this is crowded"),
        // SCH-09 Gantt.
        ("Sch_Gantt_panel_d", 3.0, "days", H, "heuristic", 1.0, 60.0, "bar width of a lab-panel marker"),
        // SCH-10 refill.
        ("Sch_Refill_lead_d", 7.0, "days", H, "heuristic", 0.0, 90.0, "order this many days before run-out"),
        ("Sch_Refill_warn_d", 14.0, "days", H, "heuristic", 0.0, 90.0, "reorder flag (clinical.md 6.5: <14 days)"),
        // SCH-12 routine.
        ("Sch_Wake_h", 6.5, "clock h", H, "heuristic", 0.0, 24.0, "wake time: the morning routine starts here"),
        ("Sch_Bed_h", 22.5, "clock h", H, "heuristic", 0.0, 24.0, "bedtime: the night routine ends here"),
        ("Sch_Sun_min", 30.0, "min", V, "WWS ch21 tip 11 (longevity.md E10)", 0.0, 240.0, "morning sunlight"),
        (
            "Sch_Exercise_gap_h",
            3.0,
            "h",
            V,
            "WWS ch21 tip 2 (longevity.md E10)",
            0.0,
            12.0,
            "no exercise later than 2-3 h before bed",
        ),
        (
            "Sch_Caffeine_HLs",
            2.0,
            "half-lives",
            H,
            "heuristic",
            0.0,
            10.0,
            "caffeine half-lives between last cup and bed",
        ),
        ("Sch_Slip_warn_min", 5.0, "min", H, "heuristic", 0.0, 120.0, "slippage flag"),
        // SCH-13 visit pack.
        ("Sch_POMR_monitor_d", 90.0, "days", H, "heuristic", 7.0, 1825.0, "next monitoring of a problem's marker"),
        (
            "Sch_MedRec_review_d",
            365.0,
            "days",
            H,
            "heuristic",
            7.0,
            1825.0,
            "a medication-list row older than this is due",
        ),
        // SCH-14 low priority.
        ("Sch_Travel_lead_d", 42.0, "days", H, "heuristic", 0.0, 365.0, "checklist items due this long before a trip"),
        (
            "Sch_Beers_age",
            65.0,
            "years",
            U,
            "pending: AGS Beers Criteria (2023)",
            0.0,
            120.0,
            "age the review applies from",
        ),
    ]
}

/// Register every SCH parameter unless an earlier SCH sheet already did.
pub fn ensure(cx: &Cx) -> Result<()> {
    let mut p = cx.shared.params.borrow_mut();
    if p.get("Sch_Vax_Soon_d").is_some() {
        return Ok(());
    }
    for (name, value, unit, state, source, min, max, note) in rows() {
        p.add(Param::num(name, value, unit, state, source).range(min, max).note(note))?;
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
    fn parameters_are_valid_unique_cited_and_in_range() {
        let mut set = ParamSet::default();
        crate::infra::params::seed(&mut set);
        for (name, value, unit, state, source, min, max, _) in rows() {
            assert!(name.starts_with("Sch_"), "{name}");
            assert!((min..=max).contains(&value), "{name}: {value} outside {min}..{max}");
            if state == State::Unverified {
                assert!(source.starts_with("pending: "), "{name}");
            }
            if state == State::Verified {
                assert!(source.contains(".md:") || source.contains("WWS"), "{name}: cite file and line");
            }
            set.add(Param::num(name, value, unit, state, source)).unwrap();
        }
        assert!(names().len() > 40);
    }
}
