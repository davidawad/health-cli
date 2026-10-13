//! Every constant the BOD sheets use, registered once per build by whichever BOD
//! sheet builds first (so any subset of sheets finds its names).
//!
//! States: values the catalogues quote from a book with file + line ship `verified`
//! with that citation (`via <catalogue id>` says where the quote was taken from);
//! constants only seen in a secondary source are `UNVERIFIED` with
//! `pending: <reference>`; tunable choices (windows, thresholds of a detector) are
//! `heuristic`; values David types are `input`. The shared seeds `KAMA_fast`,
//! `KAMA_slow`, `Kalman_q`, `Kalman_r`, `CUSUM_k`, `CUSUM_h`, `Boll_K`, `Alpha`,
//! `Caffeine_HL` and `Protein_*_g_kg` (infra) are reused, not redefined.

use crate::error::Result;
use crate::infra::params::{Param, State};
use crate::workbook::Cx;

/// (name, value or None for a blank input, unit, state, source, note).
type Row = (&'static str, Option<f64>, &'static str, State, &'static str, &'static str);

fn rows() -> Vec<Row> {
    use State::{Heuristic as H, Input as I, Unverified as U, Verified as V};
    vec![
        // Units.
        ("Bod_lb_per_kg", Some(2.204_622_62), "lb/kg", V, "definition: 1 lb = 0.45359237 kg (exact)", ""),
        // BOD-01 weight dashboard.
        ("Bod_EMA_span", Some(7.0), "days", H, "heuristic", "EMA alpha per day = 2/(span+1); longevity.md G5"),
        (
            "Bod_Kcal_target",
            None,
            "kcal/day",
            I,
            "enter",
            "a target day also needs kcal at or under this (blank: ignored)",
        ),
        // BOD-02 rate guard-rails.
        (
            "Bod_Cut_lo_pct",
            Some(0.5),
            "%/week",
            V,
            "HELMS ch01 l.1090-1127 (via longevity.md D2)",
            "cut: lose at least",
        ),
        ("Bod_Cut_hi_pct", Some(1.0), "%/week", V, "HELMS ch01 l.1090-1127 (via longevity.md D2)", "cut: lose at most"),
        ("Bod_Gain_beg_lo", Some(1.0), "%/month", V, "HELMS ch01 l.1090-1127 (via longevity.md D2)", "beginner gain"),
        ("Bod_Gain_beg_hi", Some(1.5), "%/month", V, "HELMS ch01 l.1090-1127 (via longevity.md D2)", "beginner gain"),
        ("Bod_Gain_int_lo", Some(0.5), "%/month", V, "HELMS ch01 l.1090-1127 (via longevity.md D2)", "intermediate"),
        ("Bod_Gain_int_hi", Some(1.0), "%/month", V, "HELMS ch01 l.1090-1127 (via longevity.md D2)", "intermediate"),
        ("Bod_Gain_adv_hi", Some(0.5), "%/month", V, "HELMS ch01 l.1090-1127 (via longevity.md D2)", "advanced: up to"),
        ("Bod_Loss_line_pct", Some(5.0), "%", V, "DIP ch167.md:19 (via clinical.md C4.2)", "5% loss: measurable gains"),
        // BOD-04 KAMA, BOD-05 Bollinger, BOD-07 slope, BOD-08 CUSUM, BOD-09 shocks, BOD-11 oscillator.
        ("Bod_KAMA_n", Some(10.0), "days", H, "heuristic", "efficiency-ratio window (KAU Ch17)"),
        ("Bod_Boll_days", Some(20.0), "days", H, "heuristic", "Bollinger window over Bod_Cal days (KAU Ch7)"),
        ("Bod_Slope_short", Some(14.0), "days", H, "heuristic", "short rolling-slope window"),
        ("Bod_Slope_long", Some(28.0), "days", H, "heuristic", "long rolling-slope window"),
        ("Bod_CUSUM_base", Some(28.0), "days", H, "heuristic", "baseline days for the mean / sd of daily change"),
        ("Bod_Shock_x", Some(5.0), "x mean |change|", H, "heuristic", "MASTER BOD-09 (KAU Ch22)"),
        ("Bod_Osc_days", Some(28.0), "days", H, "heuristic", "stochastic range window (KAU Ch9)"),
        // BOD-10 / 12 / 13 reversion, fan, glide path.
        ("Bod_Revert_days", Some(90.0), "days", H, "heuristic", "AR(1) fit window"),
        ("Bod_Fan_hist", Some(28.0), "days", H, "heuristic", "days of daily change behind the fan's drift and sd"),
        ("Bod_Fan_days", Some(90.0), "days", H, "heuristic", "projection horizon"),
        ("Bod_Goal_lb", None, "lb", I, "enter", "glide-path goal weight"),
        ("Bod_Goal_date", None, "date", I, "enter", "glide-path goal date"),
        ("Bod_Plan_start", None, "date", I, "enter", "glide-path start (blank: first weigh-in)"),
        // BOD-16 wearable.
        ("Bod_ACWR_acute", Some(7.0), "days", H, "heuristic", "acute EWMA span"),
        ("Bod_ACWR_chronic", Some(28.0), "days", H, "heuristic", "chronic EWMA span"),
        ("Bod_ACWR_lo", Some(0.8), "ratio", U, "pending: Gabbett 2016 (Br J Sports Med 50:273)", "sweet-spot low"),
        ("Bod_ACWR_hi", Some(1.3), "ratio", U, "pending: Gabbett 2016 (Br J Sports Med 50:273)", "sweet-spot high"),
        ("Bod_Z_days", Some(30.0), "days", H, "heuristic", "z-band baseline window"),
        ("Bod_Z_band", Some(2.0), "sd", H, "heuristic", "z-band width"),
        // BOD-17 BMI and waist.
        ("Bod_BMI_under", Some(18.5), "kg/m2", V, "DIP ch164.md:239-245 (NIH classes, via clinical.md C4.3)", ""),
        ("Bod_BMI_over", Some(25.0), "kg/m2", V, "DIP ch164.md:239-245 (NIH classes, via clinical.md C4.3)", ""),
        ("Bod_BMI_obese", Some(30.0), "kg/m2", V, "DIP ch164.md:239-245 (NIH classes, via clinical.md C4.3)", ""),
        ("Bod_BMI_obese3", Some(40.0), "kg/m2", V, "DIP ch164.md:239-245 (NIH classes, via clinical.md C4.3)", ""),
        ("Bod_BMI_refeed", Some(18.0), "kg/m2", V, "DIP ch164.md:89 (via clinical.md C4.3)", "refeeding-risk note"),
        ("Bod_Waist_m_cm", Some(102.0), "cm", U, "pending: NHLBI 1998 obesity guidelines (waist risk)", "men"),
        ("Bod_Waist_f_cm", Some(88.0), "cm", U, "pending: NHLBI 1998 obesity guidelines (waist risk)", "women"),
        ("Bod_Waist_cm", None, "cm", I, "enter", "measured waist (blank: from tblSizes 'waist')"),
        // BOD-18 protein.
        ("Bod_eGFR_kidney", Some(60.0), "mL/min/1.73", U, "pending: KDIGO 2012 CKD guideline (G3a)", "ceiling applies"),
        ("Bod_Protein_kidney_g_kg", Some(0.8), "g/kg", U, "pending: KDIGO 2020 nutrition update", "reduced eGFR"),
        ("Bod_Protein_meal_g_lb", Some(0.25), "g/lb/meal", V, "OUT ch50 l.9-17 (via longevity.md D1)", "per meal"),
        ("Bod_Meals", Some(4.0), "meals/day", V, "OUT ch50 l.9-17 (via longevity.md D1)", ""),
        // BOD-19 TDEE.
        ("Bod_kcal_per_kg", Some(7700.0), "kcal/kg", H, "heuristic", "HELMS ch01 l.957: 3500 kcal/lb is a crude rule"),
        ("Bod_TDEE_days", Some(14.0), "days", H, "heuristic", "rolling window of the implied TDEE"),
        // BOD-21 macros.
        ("Bod_Fat_cut_lo", Some(15.0), "% kcal", V, "HELMS ch01 l.1493-1617 (via longevity.md D4)", "cut"),
        ("Bod_Fat_cut_hi", Some(25.0), "% kcal", V, "HELMS ch01 l.1493-1617 (via longevity.md D4)", "cut"),
        ("Bod_Fat_gain_lo", Some(20.0), "% kcal", V, "HELMS ch01 l.1493-1617 (via longevity.md D4)", "gain"),
        ("Bod_Fat_gain_hi", Some(30.0), "% kcal", V, "HELMS ch01 l.1493-1617 (via longevity.md D4)", "gain"),
        ("Bod_Fat_floor_g_lb", Some(0.25), "g/lb", V, "HELMS ch01 l.1493-1617 (via longevity.md D4)", "fat floor"),
        // BOD-24 glycemic.
        ("Bod_CGM_lo", Some(70.0), "mg/dL", V, "OUT ch49 l.3-15 (via longevity.md D13)", "time-in-range low"),
        ("Bod_CGM_hi", Some(110.0), "mg/dL", V, "OUT ch49 l.3-15 (via longevity.md D13)", "time-in-range high"),
        ("Bod_Spike_rise", Some(30.0), "mg/dL", V, "GLU ch11 (via longevity.md D12)", "post-meal rise = spike"),
        ("Bod_Fibre_g", Some(50.0), "g/day", V, "OUT ch49 l.3 (via longevity.md D14)", "fibre target"),
        // BOD-25 eating window.
        ("Bod_Window_ref_h", Some(8.0), "h", H, "heuristic", "16:8 reference only (LIFE l.3770-3776); no target"),
        // BOD-27 / 28 sleep.
        ("Bod_Sleep_min_h", Some(8.0), "h", V, "OUT ch60 rules 8-9 (via longevity.md E1)", "opportunity minimum"),
        ("Bod_Sleep_pref_h", Some(9.0), "h", V, "OUT ch60 rules 8-9 (via longevity.md E1)", "opportunity preferred"),
        ("Bod_Sleep_eff", Some(0.90), "fraction", V, "WWS ch06 (via longevity.md E2)", "efficiency good at or above"),
        ("Bod_Wake_SD_nights", Some(14.0), "nights", H, "heuristic", "window of the wake-time SD"),
        ("Bod_Wake_SD_max_min", Some(30.0), "min", H, "heuristic", "flag a wake-time SD above this"),
        ("Bod_Cycle_min", Some(90.0), "min", V, "WWS l.1192-1210 (via longevity.md E3)", "sleep cycle length"),
        // BOD-29 caffeine.
        ("Bod_Bedtime", Some(0.9375), "time", I, "enter", "habitual bedtime as a day fraction (22:30)"),
        ("Bod_Caffeine_bed_mg", Some(50.0), "mg", H, "heuristic", "flag caffeine still active at bedtime above this"),
        // BOD-30 sleep hygiene.
        ("Bod_Last_meal_gap_h", Some(3.0), "h", V, "OUT ch60 rule 2 (via longevity.md E6)", "no food before bed"),
        ("Bod_Screen_gap_h", Some(2.0), "h", V, "OUT ch60 (via longevity.md E5)", "no stimulating screens"),
        ("Bod_STOPBANG_flag", Some(3.0), "items", U, "pending: Chung 2008 (Anesthesiology 108:812)", "slot only"),
        // Lane A extras (BOD-02..08).
        ("Bod_ER_trend", Some(0.3), "ratio", H, "heuristic", "BOD-04: trending when the efficiency ratio is above"),
        ("Bod_Fat_tail_kurt", Some(1.0), "excess kurtosis", H, "heuristic", "BOD-03: flag fat tails above"),
        // Lane B extras (BOD-09..15).
        ("Bod_Revert_ma", Some(14.0), "days", H, "heuristic", "trailing mean the AR(1) deviations are taken from"),
        ("Bod_Osc_hi", Some(80.0), "%K", H, "heuristic", "top of the range above this (KAU Ch9)"),
        ("Bod_Osc_lo", Some(20.0), "%K", H, "heuristic", "bottom of the range below this (KAU Ch9)"),
        ("Bod_Osc_smooth", Some(3.0), "days", H, "heuristic", "%D = mean of %K over this many days"),
        // Lane C extras (BOD-17..23).
        ("Bod_Atwater_protein", Some(4.0), "kcal/g", V, "Atwater general factors (4/4/9 kcal/g)", "BOD-21 macros"),
        ("Bod_Atwater_carb", Some(4.0), "kcal/g", V, "Atwater general factors (4/4/9 kcal/g)", "BOD-21 macros"),
        ("Bod_Atwater_fat", Some(9.0), "kcal/g", V, "Atwater general factors (4/4/9 kcal/g)", "BOD-21 macros"),
        // Lane D extras (BOD-16, 24..30).
        ("Bod_Spike_note_n", Some(3.0), "spikes", H, "heuristic", "neutral pattern note from this many CGM spikes"),
        ("Bod_Arch_base_nights", Some(28.0), "nights", H, "heuristic", "personal stage-share baseline window"),
        ("Bod_Chrono_early_h", Some(3.0), "h after 00:00", H, "heuristic", "mean mid-sleep before this: morning type"),
        ("Bod_Chrono_late_h", Some(5.0), "h after 00:00", H, "heuristic", "mean mid-sleep after this: evening type"),
    ]
}

/// Register every BOD parameter unless an earlier BOD sheet already did.
pub fn ensure(cx: &Cx) -> Result<()> {
    let mut p = cx.shared.params.borrow_mut();
    if p.get("Bod_lb_per_kg").is_some() {
        return Ok(());
    }
    for (name, value, unit, state, source, note) in rows() {
        let param = match value {
            Some(v) => Param::num(name, v, unit, state, source).range(0.0, (v * 10.0).max(1.0)),
            None => Param::blank(name, unit, state, source),
        };
        p.add(param.note(note))?;
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
            let p = match value {
                Some(v) => Param::num(name, v, unit, state, source),
                None => Param::blank(name, unit, state, source),
            };
            set.add(p).unwrap();
            assert!(name.starts_with("Bod_"), "{name}");
            assert!(state != State::Verified || !source.is_empty(), "{name} cites its source");
        }
    }
}
