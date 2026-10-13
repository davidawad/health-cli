//! Lane B tests: BOD-09..15 (`Bod_Patterns`, `Bod_Revert`, `Bod_Osc`, `Bod_Fan`,
//! `Bod_Glide`, `Bod_Regime`, `Bod_Sens`): structure and formulas of each sheet, then one
//! LibreOffice recalculation checked against Rust references over `fixture::weight`.

use calamine::{Data as Cell, Range, Reader};

use crate::fixture::*;

const N: usize = DAYS as usize;
const LANE_B: [&str; 7] = ["Bod_Patterns", "Bod_Revert", "Bod_Osc", "Bod_Fan", "Bod_Glide", "Bod_Regime", "Bod_Sens"];

// ---------------------------------------------------------------- references

/// Observed weights by day.
fn obs() -> Vec<Option<f64>> {
    (0..DAYS).map(weight).collect()
}

/// Carried-forward weight (`Bod_Cal.weight_ff`).
fn ff() -> Vec<Option<f64>> {
    let mut last = None;
    obs()
        .into_iter()
        .map(|w| {
            if w.is_some() {
                last = w;
            }
            last
        })
        .collect()
}

/// `Bod_Cal.dw_day`: lb/day since the previous weigh-in, on weigh-in days.
fn dw_day() -> Vec<Option<f64>> {
    let (o, f) = (obs(), ff());
    let mut last: Option<usize> = None;
    (0..N)
        .map(|i| {
            let v = match (o[i], last) {
                (Some(w), Some(l)) => Some((w - f[i - 1].unwrap()) / (i - l) as f64),
                _ => None,
            };
            if o[i].is_some() {
                last = Some(i);
            }
            v
        })
        .collect()
}

fn mean(x: &[f64]) -> f64 {
    x.iter().sum::<f64>() / x.len() as f64
}

fn sd(x: &[f64]) -> f64 {
    let m = mean(x);
    (x.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (x.len() as f64 - 1.0)).sqrt()
}

/// Autocorrelation of centred `dw_day` (0 on days without a change), lags 1..=35.
fn acf() -> Vec<f64> {
    let d = dw_day();
    let m = mean(&d.iter().flatten().copied().collect::<Vec<_>>());
    let x: Vec<f64> = d.iter().map(|v| v.map_or(0.0, |v| v - m)).collect();
    let ss: f64 = x.iter().map(|v| v * v).sum();
    (1..=35).map(|k| (k..N).map(|i| x[i] * x[i - k]).sum::<f64>() / ss).collect()
}

/// AR(1) of deviations from the trailing 14-day mean over the last 90 days: (b, half-life).
fn ar1() -> (f64, f64) {
    let o = obs();
    let dev: Vec<Option<f64>> = (0..N)
        .map(|i| {
            let win: Vec<f64> = o[i.saturating_sub(13)..=i].iter().flatten().copied().collect();
            o[i].map(|w| w - mean(&win))
        })
        .collect();
    let pairs: Vec<(f64, f64)> =
        (1..N).filter(|&i| i + 1 > N - 90).filter_map(|i| Some((dev[i - 1]?, dev[i]?))).collect();
    let mx = mean(&pairs.iter().map(|p| p.0).collect::<Vec<_>>());
    let my = mean(&pairs.iter().map(|p| p.1).collect::<Vec<_>>());
    let sxy: f64 = pairs.iter().map(|(x, y)| (x - mx) * (y - my)).sum();
    let sxx: f64 = pairs.iter().map(|(x, _)| (x - mx).powi(2)).sum();
    let b = sxy / sxx;
    (b, 2f64.ln() / -b.ln())
}

/// Stochastic %K on day `i` over a 28-day window.
fn pct_k(i: usize) -> f64 {
    let win: Vec<f64> = obs()[i.saturating_sub(27)..=i].iter().flatten().copied().collect();
    let (lo, hi) = (win.iter().cloned().fold(f64::MAX, f64::min), win.iter().cloned().fold(f64::MIN, f64::max));
    (ff()[i].unwrap() - lo) / (hi - lo) * 100.0
}

/// Fan drift and SD of daily change of weight_ff over the last 28 days.
fn fan_mu_sd() -> (f64, f64) {
    let f = ff();
    let d: Vec<f64> = (N - 28..N).map(|i| f[i].unwrap() - f[i - 1].unwrap()).collect();
    (mean(&d), sd(&d))
}

/// Max regain of weight_ff above the running minimum weigh-in.
fn max_regain() -> f64 {
    let (o, f) = (obs(), ff());
    let mut best = f64::MAX;
    (0..N)
        .map(|i| {
            if let Some(w) = o[i] {
                best = best.min(w);
            }
            f[i].unwrap() - best
        })
        .fold(0.0, f64::max)
}

/// Time-based EWMA one-step RMSE for `alpha`.
fn ewma_rmse(alpha: f64) -> f64 {
    let o = obs();
    let (mut lvl, mut last, mut se, mut k): (Option<f64>, usize, f64, f64) = (None, 0, 0.0, 0.0);
    for (i, w) in o.iter().enumerate() {
        let Some(w) = *w else { continue };
        lvl = Some(match lvl {
            Some(p) => {
                se += (w - p).powi(2);
                k += 1.0;
                p + (1.0 - (1.0 - alpha).powi((i - last) as i32)) * (w - p)
            }
            None => w,
        });
        last = i;
    }
    (se / k).sqrt()
}

/// Share of weigh-in days with |w - mean| > k * sd over a trailing `win`-day window (>= 3 weigh-ins).
fn boll_share(win: usize, k: f64) -> f64 {
    let o = obs();
    let (mut n, mut hit) = (0.0, 0.0);
    for i in 0..N {
        let Some(w) = o[i] else { continue };
        let x: Vec<f64> = o[(i + 1).saturating_sub(win)..=i].iter().flatten().copied().collect();
        if x.len() < 3 || sd(&x) == 0.0 {
            continue;
        }
        n += 1.0;
        if (w - mean(&x)).abs() / sd(&x) > k {
            hit += 1.0;
        }
    }
    hit / n
}

/// (row, col) of the first cell whose text is `label`.
fn pos(r: &Range<Cell>, label: &str) -> (u32, u32) {
    r.cells()
        .find(|(_, _, v)| matches!(v, Cell::String(s) if s == label))
        .map(|(row, c, _)| (row as u32, c as u32))
        .unwrap_or_else(|| panic!("no label {label}"))
}

// ---------------------------------------------------------------- structure

fn sheet(name: &str) -> (String, String) {
    let dir = tempfile::TempDir::new().unwrap();
    let p = build(dir.path());
    let mut wb = open(&p);
    (formulas(&mut wb, name), sheet_text(&mut wb, name))
}

#[test]
fn lane_b_patterns_has_shocks_weekdays_and_lagged_sumproducts() {
    let (f, t) = sheet("Bod_Patterns");
    assert!(t.contains("BOD-09 BUILDABLE-NOW"), "{t:.300}");
    assert!(f.contains("Bod_Cal!L"), "weekday comes from Bod_Cal");
    assert!(f.contains("AVERAGEIFS(") && f.contains("SUMPRODUCT(") && f.contains("SUMSQ("));
    assert!(f.contains("Bod_Shock_x") && f.contains("MATCH("));
    assert!(f.contains("NORM.S.INV(1-Alpha/2)"));
}

#[test]
fn lane_b_revert_fits_ar1_half_life() {
    let (f, t) = sheet("Bod_Revert");
    assert!(t.contains("BOD-10 BUILDABLE-NOW"));
    assert!(f.contains("LN(2)") && f.contains("Bod_Revert_days") && f.contains("Bod_Revert_ma"));
    assert!(f.contains("SUMPRODUCT("));
}

#[test]
fn lane_b_osc_uses_rolling_min_max() {
    let (f, t) = sheet("Bod_Osc");
    assert!(t.contains("BOD-11 BUILDABLE-NOW"));
    assert!(f.contains("MIN(INDEX(") && f.contains("MAX(INDEX(") && f.contains("Bod_Osc_days"));
}

#[test]
fn lane_b_fan_projects_sqrt_t_bands() {
    let (f, t) = sheet("Bod_Fan");
    assert!(t.contains("BOD-12 BUILDABLE-NOW"));
    assert!(t.contains("not a forecast"));
    assert!(f.contains("SQRT(") && f.contains("NORM.S.INV(0.975)") && f.contains("Bod_Fan_hist"));
    assert!(f.contains("Bod_Goal_lb"));
}

#[test]
fn lane_b_glide_has_plan_tracking_error_and_underwater() {
    let (f, t) = sheet("Bod_Glide");
    assert!(t.contains("BOD-13 BUILDABLE-NOW"));
    assert!(f.contains("STDEV.S(") && f.contains("MIN(") && f.contains("Bod_Goal_date"));
    assert!(f.contains("Bod_Cut_lo_pct"), "default plan from the cut band");
}

#[test]
fn lane_b_regime_is_a_placeholder_with_welch() {
    let (f, t) = sheet("Bod_Regime");
    assert!(t.contains("BOD-14 PLACEHOLDER"));
    assert!(t.contains("PLACEHOLDER - awaiting data"), "{t:.400}");
    assert!(f.contains("T.DIST.2T(") && f.contains("LOOKUP(2,1/") && f.contains("Data_Regimes!"));
    assert!(!f.contains("T.TEST("), "Welch t is built from cells, not an array T.TEST");
}

#[test]
fn lane_b_sens_has_explicit_grids() {
    let (f, t) = sheet("Bod_Sens");
    assert!(t.contains("BOD-15 BUILDABLE-NOW"));
    assert!(f.contains("STDEV.S(") && f.contains("COUNTIF(") && f.contains("SUMSQ(") && f.contains("RANK("));
    assert!(!f.contains("TABLE("), "no Data Tables");
}

// ---------------------------------------------------------------- recalculated

#[test]
fn lane_b_recalculated_values_match_references() {
    let dir = tempfile::TempDir::new().unwrap();
    let Some(mut wb) = recalculated(dir.path()) else { return };
    let errs: Vec<String> =
        errors(&mut wb).into_iter().filter(|e| LANE_B.iter().any(|s| e.starts_with(&format!("{s}!")))).collect();
    assert!(errs.is_empty(), "formula errors after recalculation: {errs:?}");

    // BOD-09: weekday means, shocks, autocorrelation and the 7-day cycle.
    let p = wb.worksheet_range("Bod_Patterns").unwrap();
    let r = acf();
    for k in [1usize, 7, 14, 30] {
        close(num(&at(&p, "lag", "r", k as u32 - 1)), r[k - 1], &format!("acf lag {k}"));
    }
    assert_eq!(num(&right_of(&p, "Cycle: first significant peak (days)")), 7.0);
    for k in (1..=35).filter(|k| k % 7 != 0) {
        assert!(r[6] > r[k - 1], "lag 7 stands out over lag {k}");
    }
    let best = (1..=35).max_by(|a, b| r[a - 1].total_cmp(&r[b - 1])).unwrap();
    assert_eq!(num(&right_of(&p, "Strongest lag (days)")), best as f64);
    let d = dw_day();
    let absd: Vec<f64> = d.iter().flatten().map(|v| v.abs()).collect();
    let thr = 5.0 * mean(&absd);
    let shocks = absd.iter().filter(|v| **v > thr).count();
    close(num(&right_of(&p, "Shock threshold (lb/day)")), thr, "shock threshold");
    assert_eq!(num(&right_of(&p, "Shock days")), shocks as f64);
    for wd in 0..7usize {
        let x: Vec<f64> = (0..N).filter(|i| i % 7 == wd).filter_map(|i| d[i]).collect();
        close(num(&at(&p, "day_of_week", "mean_dw", wd as u32)), mean(&x), &format!("weekday {wd} mean"));
        close(num(&at(&p, "day_of_week", "sd_dw", wd as u32)), sd(&x), &format!("weekday {wd} sd"));
        assert_eq!(num(&at(&p, "day_of_week", "n", wd as u32)), x.len() as f64);
    }

    // BOD-10: AR(1) coefficient and half-life.
    let rv = wb.worksheet_range("Bod_Revert").unwrap();
    let (b, hl) = ar1();
    assert!(b > 0.0 && b < 1.0, "reference b {b}");
    close(num(&right_of(&rv, "b (AR(1) coefficient)")), b, "AR(1) b");
    close(num(&right_of(&rv, "Half-life (days)")), hl, "half-life");

    // BOD-11: %K within 0..100 and equal to the reference.
    let o = wb.worksheet_range("Bod_Osc").unwrap();
    for i in [10u32, 60, 80, 119] {
        let k = num(&at(&o, "date", "pct_k", i));
        assert!((0.0..=100.0).contains(&k), "%K day {i} = {k}");
        close(k, pct_k(i as usize), &format!("%K day {i}"));
    }

    // BOD-12: drift, SD and the median / 95% band at t = 30.
    let fan = wb.worksheet_range("Bod_Fan").unwrap();
    let (mu, s) = fan_mu_sd();
    close(num(&right_of(&fan, "Drift mu (lb/day)")), mu, "fan drift");
    close(num(&right_of(&fan, "SD of daily change (lb)")), s, "fan sd");
    let start = ff()[N - 1].unwrap();
    close(num(&at(&fan, "t", "median", 30)), start + 30.0 * mu, "median t=30");
    close(num(&at(&fan, "t", "hi95", 30)), start + 30.0 * mu + 1.959_964 * s * 30f64.sqrt(), "hi95 t=30");
    assert!(!matches!(at(&fan, "t", "t", 120), Cell::Float(_) | Cell::Int(_)), "blank beyond Bod_Fan_days");

    // BOD-13: default plan, regain after the +3 lb step.
    let g = wb.worksheet_range("Bod_Glide").unwrap();
    assert_eq!(num(&right_of(&g, "Goal plan in use (1) / default (0)")), 0.0);
    let w0 = weight(0).unwrap();
    close(num(&at(&g, "date", "plan", 119)), w0 * (1.0f64 - 0.005).powf(119.0 / 7.0), "default plan day 119");
    let mr = max_regain();
    assert!(mr > 2.5 && mr < 4.5, "reference regain {mr}");
    close(num(&right_of(&g, "Max regain above best (lb)")), mr, "max regain");

    // BOD-14: no phases typed: labels blank, the test asks for data.
    let rg = wb.worksheet_range("Bod_Regime").unwrap();
    assert!(text(&rg, pos(&rg, "p (two-sided)").0, pos(&rg, "p (two-sided)").1 + 1).starts_with("need"));
    assert_eq!(num(&at(&rg, "regime_label", "n_days", 0)), 0.0);

    // BOD-15: Bollinger share grid and EWMA RMSE per alpha.
    let sn = wb.worksheet_range("Bod_Sens").unwrap();
    let (gr, gc) = pos(&sn, "k \\ window (days)");
    let share = num(&sn.get_value((gr + 2, gc + 3)).cloned().unwrap()); // k = 2, window 20
    close(share, boll_share(20, 2.0), "breach share w20 k2");
    let (rr, rc) = pos(&sn, "RMSE (lb)");
    let alphas = [0.05, 0.1, 0.15, 0.2, 0.3, 0.4, 0.5];
    for (j, a) in alphas.iter().enumerate() {
        close(num(&sn.get_value((rr, rc + 1 + j as u32)).cloned().unwrap()), ewma_rmse(*a), &format!("RMSE {a}"));
    }
    let best = alphas.iter().copied().min_by(|a, b| ewma_rmse(*a).total_cmp(&ewma_rmse(*b))).unwrap();
    close(num(&right_of(&sn, "Best alpha (lowest RMSE)")), best, "best alpha");
}

#[test]
fn lane_b_empty_log_recalculates_without_errors() {
    let Some(so) = soffice() else {
        skip("soffice (LibreOffice) not installed: empty-log recalculation not checked");
        return;
    };
    let dir = tempfile::TempDir::new().unwrap();
    let mut d = fixture();
    d.intake.clear();
    if let Some(l) = d.labs.as_mut() {
        l.measurements.clear();
    }
    let p = dir.path().join("empty.xlsx");
    health_cli::workbook::build(&d, &[]).expect("workbook builds").save(&p).unwrap();
    let mut wb = recalc(&so, dir.path(), &p);
    let errs: Vec<String> =
        errors(&mut wb).into_iter().filter(|e| LANE_B.iter().any(|s| e.starts_with(&format!("{s}!")))).collect();
    assert!(errs.is_empty(), "formula errors with an empty log: {errs:?}");
    let g = wb.worksheet_range("Bod_Glide").unwrap();
    assert!(!matches!(right_of(&g, "Max regain above best (lb)"), Cell::Float(_)));
}

/// Copy the workbook at `src` to `dst`, typing values into blank cells of one sheet part:
/// (cell, value): a value that parses as a number is written as one, anything else as text.
fn type_cells(src: &std::path::Path, dst: &std::path::Path, part: &str, cells: &[(&str, &str)]) {
    use std::io::{Read, Write};
    let mut z = zip::ZipArchive::new(std::fs::File::open(src).unwrap()).unwrap();
    let mut w = zip::ZipWriter::new(std::fs::File::create(dst).unwrap());
    let opts = zip::write::SimpleFileOptions::default();
    for i in 0..z.len() {
        let mut f = z.by_index(i).unwrap();
        let name = f.name().to_string();
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).unwrap();
        if name == part {
            let mut xml = String::from_utf8(buf).unwrap();
            for (cell, v) in cells {
                let open = format!("<c r=\"{cell}\" s=\"");
                let a = xml.find(&open).unwrap_or_else(|| panic!("no blank cell {cell}"));
                let b = a + xml[a..].find("/>").unwrap();
                let style = &xml[a + open.len()..b - 1];
                let new = match v.parse::<f64>() {
                    Ok(_) => format!("<c r=\"{cell}\" s=\"{style}\"><v>{v}</v></c>"),
                    Err(_) => format!("<c r=\"{cell}\" s=\"{style}\" t=\"inlineStr\"><is><t>{v}</t></is></c>"),
                };
                xml.replace_range(a..b + 2, &new);
            }
            buf = xml.into_bytes();
        }
        w.start_file(name, opts).unwrap();
        w.write_all(&buf).unwrap();
    }
    w.finish().unwrap();
}

#[test]
fn lane_b_regime_with_typed_phases_runs_welch() {
    let Some(so) = soffice() else {
        skip("soffice (LibreOffice) not installed: regime Welch test not checked");
        return;
    };
    let dir = tempfile::TempDir::new().unwrap();
    let p = build(dir.path());
    let names = open(&p).sheet_names();
    let i = names.iter().position(|n| n == "Bod_Regime").unwrap();
    let part = format!("xl/worksheets/sheet{}.xml", i + 1);
    // cut from day 0 to day 49, maintain from day 50, open-ended.
    let typed = dir.path().join("typed.xlsx");
    type_cells(
        &p,
        &typed,
        &part,
        &[("A8", "cut"), ("B8", "45292"), ("C8", "45341"), ("A9", "maintain"), ("B9", "45342")],
    );
    let mut wb = recalc(&so, dir.path(), &typed);
    let errs: Vec<String> = errors(&mut wb).into_iter().filter(|e| e.starts_with("Bod_Regime!")).collect();
    assert!(errs.is_empty(), "{errs:?}");
    let r = wb.worksheet_range("Bod_Regime").unwrap();
    assert_eq!(num(&at(&r, "regime_label", "n_days", 0)), 50.0);
    assert_eq!(num(&at(&r, "regime_label", "n_days", 1)), 70.0);
    let d = dw_day();
    let a: Vec<f64> = d[..50].iter().flatten().copied().collect();
    let b: Vec<f64> = d[50..].iter().flatten().copied().collect();
    let (va, vb) = (sd(&a).powi(2) / a.len() as f64, sd(&b).powi(2) / b.len() as f64);
    let t = (mean(&a) - mean(&b)) / (va + vb).sqrt();
    let df = (va + vb).powi(2) / (va * va / (a.len() as f64 - 1.0) + vb * vb / (b.len() as f64 - 1.0));
    close(num(&right_of(&r, "Welch t")), t, "Welch t");
    close(num(&right_of(&r, "df (Welch-Satterthwaite)")), df, "Welch df");
    let pv = num(&right_of(&r, "p (two-sided)"));
    assert!(pv > 0.0 && pv < 1.0, "p {pv}");
}
