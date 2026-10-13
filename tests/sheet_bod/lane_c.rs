//! Lane C tests: BOD-17..23 (`Bod_BMI`, `Bod_Protein`, `Bod_TDEE`, `Bod_LeadLag`,
//! `Bod_Macro`, `Bod_Dozen`, `Bod_Yield`): formulas and status lines on the built
//! workbook, then values after one LibreOffice recalculation against Rust references
//! computed from the fixture.

use calamine::{Data as Cell, Reader};
use tempfile::TempDir;

use crate::fixture::*;

const LB_PER_KG: f64 = 2.204_622_62;

/// Last weigh-in on or before day `i` (lb).
fn weight_ff(i: i64) -> Option<f64> {
    (0..=i).rev().find_map(weight)
}

/// Weight change per day on weigh-in day `i` since the previous weigh-in (Bod_Cal dw_day).
fn dw_day(i: i64) -> Option<f64> {
    let w = weight(i)?;
    let j = (0..i).rev().find(|j| weight(*j).is_some())?;
    Some((w - weight(j)?) / (i - j) as f64)
}

/// Pearson r of x[t] with dw_day[t+k] over pairs where both exist.
fn lag_r(x: impl Fn(i64) -> Option<f64>, k: i64) -> f64 {
    let pairs: Vec<(f64, f64)> = (0..DAYS - k).filter_map(|t| Some((x(t)?, dw_day(t + k)?))).collect();
    let n = pairs.len() as f64;
    let (mx, my) = (pairs.iter().map(|p| p.0).sum::<f64>() / n, pairs.iter().map(|p| p.1).sum::<f64>() / n);
    let sxy: f64 = pairs.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    let sxx: f64 = pairs.iter().map(|p| (p.0 - mx).powi(2)).sum();
    let syy: f64 = pairs.iter().map(|p| (p.1 - my).powi(2)).sum();
    sxy / (sxx * syy).sqrt()
}

fn text_of(c: &Cell) -> String {
    c.to_string()
}

#[test]
fn lane_c_sheets_have_their_formulas_and_status() {
    let dir = TempDir::new().unwrap();
    let p = build(dir.path());
    let mut wb = open(&p);
    let checks: [(&str, &str, &[&str]); 7] = [
        (
            "Bod_BMI",
            "BOD-17 BUILDABLE-NOW",
            &["Height_cm", "Bod_BMI_obese", "Bod_BMI_refeed", "SEARCH(\"waist\"", "Bod_Waist_f_cm"],
        ),
        (
            "Bod_Protein",
            "BOD-18 BUILDABLE-NOW",
            &["Protein_Mid_g_kg", "Bod_eGFR_kidney", "Bod_Protein_kidney_g_kg", "COUNTIFS(", "Bod_Protein_meal_g_lb"],
        ),
        ("Bod_TDEE", "BOD-19 BUILDABLE-NOW", &["Bod_kcal_per_kg", "AVERAGEIFS(", "Bod_TDEE_days", "Bod_lb_per_kg"]),
        (
            "Bod_LeadLag",
            "BOD-20 BUILDABLE-NOW",
            &[
                "SUMPRODUCT(Bod_LeadLag!$C$8:$C$127,Bod_LeadLag!$G$8:$G$127)",
                "SUMPRODUCT(Bod_LeadLag!$C$8:$C$126,Bod_LeadLag!$G$9:$G$127)",
                "SQRT(",
            ],
        ),
        ("Bod_Macro", "BOD-21 PLACEHOLDER", &["Bod_Atwater_fat", "Bod_Fat_floor_g_lb", "Bod_Fat_gain_hi", "AVERAGE("]),
        ("Bod_Dozen", "BOD-22 BUILDABLE-NOW", &["SUMIFS(", "MIN(", "INDEX("]),
        (
            "Bod_Yield",
            "BOD-23 BUILDABLE-NOW",
            &["COUNTIFS(Bod_Yield!$G$8:$G$11,\"<\"&G8,Bod_Yield!$H$8:$H$11,\"<\"&H8)", "RANK", "Data_Nutrition!"],
        ),
    ];
    for (sheet, status, want) in checks {
        let f = formulas(&mut wb, sheet);
        for w in want {
            if *w == "RANK" {
                assert!(f.contains("COUNTIFS(Bod_Yield!$G$8:$G$11,\"<\"&G8)+1"), "{sheet}: rank by COUNTIFS");
                continue;
            }
            assert!(f.contains(w), "{sheet} lacks {w}");
        }
        for bad in ["FILTER(", "QUERY(", "ARRAYFORMULA", "CORREL("] {
            assert!(!f.contains(bad), "{sheet} uses {bad}");
        }
        let t = sheet_text(&mut wb, sheet);
        assert!(t.contains(status), "{sheet}: status line {status}");
    }
    assert!(sheet_text(&mut wb, "Bod_Macro").contains("PLACEHOLDER - awaiting data"));
    assert!(sheet_text(&mut wb, "Bod_BMI").contains("UNVERIFIED - pending: NHLBI 1998"));

    // Rule tables: berries and other vegetables pending the image, the rest verified.
    let r = wb.worksheet_range("Bod_Dozen").unwrap();
    let rows: Vec<(String, String, String)> = (0..12)
        .map(|i| {
            (
                text_of(&at(&r, "id", "rule", i)),
                text_of(&at(&r, "id", "source_state", i)),
                text_of(&at(&r, "id", "source", i)),
            )
        })
        .collect();
    let total: f64 = (0..12).map(|i| num(&at(&r, "id", "value", i))).sum();
    assert_eq!(total, 24.0, "Daily Dozen servings add up to 24");
    for (rule, state, source) in &rows {
        if rule == "Berries" || rule == "Other vegetables" {
            assert_eq!(state, "UNVERIFIED", "{rule}");
            assert_eq!(source, "pending: GREG ch20 Figure 6 (image)");
        } else {
            assert_eq!(state, "verified", "{rule}");
            assert_eq!(source, "GREG ch20 (via longevity.md D5)");
        }
    }
    let b = wb.worksheet_range("Bod_BMI").unwrap();
    assert_eq!(text_of(&at(&b, "id", "source_state", 5)), "UNVERIFIED", "waist cut-off");
    assert_eq!(text_of(&at(&b, "id", "source_state", 2)), "verified", "obese bound");
}

#[test]
fn lane_c_sheets_recalculate_to_the_references() {
    let dir = TempDir::new().unwrap();
    let Some(mut wb) = recalculated(dir.path()) else { return };
    let errs = errors(&mut wb);
    assert!(errs.is_empty(), "formula errors after recalculation: {errs:?}");
    let last = DAYS - 1;

    // BOD-17: BMI from the carried-forward weight and 70 in.
    let b = wb.worksheet_range("Bod_BMI").unwrap();
    let bmi = |i: i64| weight_ff(i).unwrap() / LB_PER_KG / 1.778f64.powi(2);
    close(num(&right_of(&b, "Latest BMI (kg/m2)")), bmi(last), "latest BMI");
    close(num(&at(&b, "date", "bmi", 52)), bmi(52), "BMI on a skipped day");
    assert_eq!(text_of(&right_of(&b, "Class (NIH)")), "overweight");
    close(num(&right_of(&b, "Waist used (cm)")), 34.0 * 2.54, "waist from tblSizes");
    assert_eq!(text_of(&right_of(&b, "Waist vs cut-off")), "set Sex in Params");

    // BOD-18: % of logged days at target, streaks, kidney ceiling without eGFR.
    let p = wb.worksheet_range("Bod_Protein").unwrap();
    let pkg: Vec<Option<f64>> =
        (0..DAYS).map(|i| intake(i).map(|(_, g, _)| g / (weight_ff(i).unwrap() / LB_PER_KG))).collect();
    let logged = pkg.iter().flatten().count() as f64;
    let hit = pkg.iter().flatten().filter(|x| **x >= 1.6).count() as f64;
    close(num(&right_of(&p, "Days at target (% of logged)")), hit / logged, "% days at target");
    close(num(&right_of(&p, "Mean protein (g/kg)")), pkg.iter().flatten().sum::<f64>() / logged, "mean g/kg");
    let (mut cur, mut best) = (0.0f64, 0.0f64);
    for x in &pkg {
        if let Some(x) = x {
            cur = if *x >= 1.6 { cur + 1.0 } else { 0.0 };
        }
        best = best.max(cur);
    }
    close(num(&right_of(&p, "Longest streak (days)")), best, "longest streak");
    close(num(&right_of(&p, "Current streak (days)")), cur, "current streak");
    assert_eq!(text_of(&right_of(&p, "eGFR status")), "no eGFR on file");
    close(num(&right_of(&p, "Ceiling (g/kg)")), 3.7, "ceiling without eGFR");
    close(num(&at(&p, "date", "daily_target_g", last as u32)), 0.25 * 4.0 * weight_ff(last).unwrap(), "daily target");

    // BOD-19: implied TDEE over the last 14-day window from the 7-day EMA.
    let t = wb.worksheet_range("Bod_TDEE").unwrap();
    let e = ema(7.0);
    let tdee = |i: i64| {
        let k: Vec<f64> = (i - 13..=i).filter_map(|d| intake(d).map(|x| x.0)).collect();
        let mean = k.iter().sum::<f64>() / k.len() as f64;
        let dkg = (e[i as usize].unwrap() - e[(i - 14) as usize].unwrap()) / LB_PER_KG;
        mean - 7700.0 * dkg / 14.0
    };
    close(num(&right_of(&t, "Implied TDEE, latest (kcal/day)")), tdee(last), "latest TDEE");
    close(num(&at(&t, "date", "implied_tdee", 60)), tdee(60), "TDEE day 60");
    assert_eq!(text_of(&at(&t, "date", "implied_tdee", 13)), "", "no full window yet");

    // BOD-20: pairwise Pearson by lag.
    let l = wb.worksheet_range("Bod_LeadLag").unwrap();
    let kcal = |i: i64| intake(i).map(|x| x.0);
    let prot = |i: i64| intake(i).map(|x| x.1);
    for k in [0u32, 1, 7, 14] {
        close(num(&at(&l, "lag_days", "r_kcal", k)), lag_r(kcal, k as i64), &format!("r kcal lag {k}"));
        close(num(&at(&l, "lag_days", "r_prot", k)), lag_r(prot, k as i64), &format!("r protein lag {k}"));
    }
    let n0 = (0..DAYS).filter(|t| kcal(*t).is_some() && dw_day(*t).is_some()).count() as f64;
    close(num(&at(&l, "lag_days", "n_kcal", 0)), n0, "pairs at lag 0");
    close(num(&at(&l, "lag_days", "bound_kcal", 0)), 2.0 / n0.sqrt(), "bound at lag 0");
    let best = (0..15).max_by(|a, b| lag_r(kcal, *a).abs().total_cmp(&lag_r(kcal, *b).abs())).unwrap();
    close(num(&right_of(&l, "Strongest kcal lag (days)")), best as f64, "strongest kcal lag");

    // BOD-21: an empty log computes nothing and errs nowhere.
    let m = wb.worksheet_range("Bod_Macro").unwrap();
    close(num(&right_of(&m, "Entries")), 0.0, "no macro entries");
    close(num(&right_of(&m, "Fat band high (% kcal)")), 25.0, "cut band by default");

    // BOD-22: mapper suggestions and an empty grid scoring 0.
    let d = wb.worksheet_range("Bod_Dozen").unwrap();
    let want = ["Beans", "", "Berries", "Cruciferous vegetables"];
    for (i, w) in want.iter().enumerate() {
        assert_eq!(text_of(&at(&d, "item", "category_used", i as u32)), *w, "mapper row {i}");
    }
    close(num(&right_of(&d, "Score, latest day (of 24)")), 0.0, "empty grid");
    close(num(&at(&d, "date", "beans", 27)), 0.0, "grid totals are numbers");
    close(num(&at(&d, "date", "date", 27)), 45_292.0 + (DAYS - 1) as f64, "grid ends on the last log day");

    // BOD-23: $/g protein, rank and the dominance frontier (whey dominates all).
    let y = wb.worksheet_range("Bod_Yield").unwrap();
    let want =
        [(0.01, 220.0 / 15.0, 0.0, 2.0), (0.0048, 4.8, 1.0, 1.0), (0.3, 80.0, 0.0, 4.0), (0.025, 12.5, 0.0, 3.0)];
    for (i, (usd, kpg, front, rank)) in want.into_iter().enumerate() {
        let i = i as u32;
        close(num(&at(&y, "item", "usd_per_g_protein", i)), usd, "usd/g");
        close(num(&at(&y, "item", "kcal_per_g_protein", i)), kpg, "kcal/g");
        close(num(&at(&y, "item", "frontier", i)), front, "frontier");
        close(num(&at(&y, "item", "rank_usd_per_g", i)), rank, "rank");
    }
    assert_eq!(text_of(&right_of(&y, "Cheapest protein")), "Whey isolate");
}

#[test]
fn lane_c_sheets_recalculate_without_errors_on_an_empty_store() {
    let Some(so) = soffice() else {
        skip("soffice (LibreOffice) not installed: empty-store recalculation not checked");
        return;
    };
    let dir = TempDir::new().unwrap();
    let mut data = fixture();
    data.intake.clear();
    data.nutrition.clear();
    data.sizes.clear();
    data.labs = None;
    let mut book = health_cli::workbook::build(&data, &[]).expect("workbook builds");
    let p = dir.path().join("empty.xlsx");
    book.save(&p).unwrap();
    let mut wb = recalc(&so, dir.path(), &p);
    let errs: Vec<String> = errors(&mut wb)
        .into_iter()
        .filter(|e| {
            ["Bod_BMI", "Bod_Protein", "Bod_TDEE", "Bod_LeadLag", "Bod_Macro", "Bod_Dozen", "Bod_Yield"]
                .iter()
                .any(|s| e.starts_with(&format!("{s}!")))
        })
        .collect();
    assert!(errs.is_empty(), "formula errors on an empty store: {errs:?}");
    let p = wb.worksheet_range("Bod_Protein").unwrap();
    assert_eq!(text_of(&right_of(&p, "Latest eGFR (mL/min/1.73m2)")), "no eGFR on file");
    let y = wb.worksheet_range("Bod_Yield").unwrap();
    close(num(&right_of(&y, "Items with $/g protein")), 0.0, "no nutrition items");
}
