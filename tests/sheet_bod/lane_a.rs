//! Lane A tests: BOD-02..08 (`Bod_Rate`, `Bod_Dist`, `Bod_Signal`, `Bod_Bands`,
//! `Bod_Kalman`, `Bod_Slope`, `Bod_Cusum`). Structure and formulas on a built workbook;
//! values after a LibreOffice recalculation against Rust references of the same
//! recursions over the synthetic fixture.

use calamine::{Data as Cell, Reader};

use crate::fixture::*;

const N: usize = DAYS as usize;

fn ws() -> Vec<Option<f64>> {
    (0..DAYS).map(weight).collect()
}

/// Last weigh-in carried forward.
fn ff() -> Vec<Option<f64>> {
    let mut last = None;
    ws().into_iter()
        .map(|w| {
            last = w.or(last);
            last
        })
        .collect()
}

fn mean(x: &[f64]) -> f64 {
    x.iter().sum::<f64>() / x.len() as f64
}

fn sd(x: &[f64]) -> f64 {
    let m = mean(x);
    (x.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (x.len() - 1) as f64).sqrt()
}

/// KAMA as `Bod_Signal` computes it (n = 10, fast 2, slow 30): (er, kama) per day.
fn kama() -> Vec<(Option<f64>, Option<f64>)> {
    let (n, fast, slow) = (10usize, 2.0 / 3.0, 2.0 / 31.0);
    let f = ff();
    let absd: Vec<f64> = (0..N).map(|i| if i == 0 { 0.0 } else { (f[i].unwrap() - f[i - 1].unwrap()).abs() }).collect();
    let mut prev: Option<f64> = None;
    (0..N)
        .map(|i| {
            let er = (i >= n).then(|| {
                let change = (f[i].unwrap() - f[i - n].unwrap()).abs();
                let vol: f64 = absd[i + 1 - n..=i].iter().sum();
                if vol > 0.0 {
                    change / vol
                } else {
                    0.0
                }
            });
            let k = match (er, prev) {
                (Some(e), Some(p)) => p + (e * (fast - slow) + slow).powi(2) * (f[i].unwrap() - p),
                _ => f[i].unwrap(),
            };
            prev = Some(k);
            (er, Some(k))
        })
        .collect()
}

/// Local-level Kalman filter (q = 0.01, r = 1): (x, p, gain) per day.
fn kalman() -> Vec<(f64, f64, f64)> {
    let (q, r) = (0.01, 1.0);
    let w = ws();
    let (mut x, mut p) = (w[0].unwrap(), r);
    let mut out = vec![(x, p, 0.0)];
    for wi in &w[1..] {
        let pp = p + q;
        let k = if wi.is_some() { pp / (pp + r) } else { 0.0 };
        if let Some(v) = wi {
            x += k * (v - x);
        }
        p = (1.0 - k) * pp;
        out.push((x, p, k));
    }
    out
}

/// OLS slope (lb/week) and its standard error over the `len`-day window ending on day `i`.
fn ols(i: usize, len: usize) -> (f64, f64) {
    let pts: Vec<(f64, f64)> = (i + 1 - len..=i).filter_map(|d| weight(d as i64).map(|w| (d as f64, w))).collect();
    let n = pts.len() as f64;
    let (mx, my) = (pts.iter().map(|p| p.0).sum::<f64>() / n, pts.iter().map(|p| p.1).sum::<f64>() / n);
    let sxx: f64 = pts.iter().map(|p| (p.0 - mx).powi(2)).sum();
    let sxy: f64 = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    let syy: f64 = pts.iter().map(|p| (p.1 - my).powi(2)).sum();
    let b = sxy / sxx;
    let se = ((syy - b * b * sxx) / (n - 2.0) / sxx).sqrt();
    (7.0 * b, 7.0 * se)
}

fn has_all(f: &str, needles: &[&str], sheet: &str) {
    for x in needles {
        assert!(f.contains(x), "{sheet} formulas lack {x}");
    }
}

#[test]
fn lane_a_sheets_have_their_formulas_tables_and_status() {
    let dir = tempfile::TempDir::new().unwrap();
    let p = build(dir.path());
    let mut wb = open(&p);
    let checks: [(&str, &str, &[&str]); 7] = [
        (
            "Bod_Rate",
            "BOD-02",
            &["AVERAGEIFS(", "Bod_Cut_hi_pct", "Bod_Gain_beg_lo*12/52", "Bod_Loss_line_pct", "Bod_Cal!"],
        ),
        ("Bod_Dist", "BOD-03", &["SKEW(", "KURT(", "NORM.DIST(", "COUNTIFS(", "CHOOSE(MATCH(", "PERCENTILE("]),
        ("Bod_Signal", "BOD-04", &["KAMA_fast", "KAMA_slow", "Bod_KAMA_n", "Bod_Weight!", "Bod_ER_trend"]),
        ("Bod_Bands", "BOD-05", &["Bod_Boll_days", "Boll_K", "STDEV.S(", "NA()"]),
        ("Bod_Kalman", "BOD-06", &["Kalman_q", "Kalman_r", "RCV_z", "Bod_Cal!"]),
        ("Bod_Slope", "BOD-07", &["Bod_Slope_short", "Bod_Slope_long", "SUMPRODUCT(", "T.INV.2T(", "Alpha"]),
        ("Bod_Cusum", "BOD-08", &["CUSUM_k", "CUSUM_h", "Bod_CUSUM_base", "MATCH(", "Bod_Weight!"]),
    ];
    for (sheet, id, needles) in checks {
        let f = formulas(&mut wb, sheet);
        has_all(&f, needles, sheet);
        for bad in ["FILTER(", "ARRAYFORMULA", "QUERY(", "FREQUENCY("] {
            assert!(!f.contains(bad), "{sheet} uses {bad}");
        }
        assert!(sheet_text(&mut wb, sheet).contains(&format!("{id} BUILDABLE-NOW")), "{sheet} status line");
    }
    // Tables, charts and KPIs.
    let names = wb.sheet_names();
    for (sheet, tables) in [
        ("Bod_Rate", &["tblBodRate", "tblBodRateModes", "tblBodRateWeeks"][..]),
        ("Bod_Dist", &["tblBodDistMoments", "tblBodDistHist", "tblBodDistSeries"]),
        ("Bod_Signal", &["tblBodSignal"]),
        ("Bod_Bands", &["tblBodBands"]),
        ("Bod_Kalman", &["tblBodKalman"]),
        ("Bod_Slope", &["tblBodSlope", "tblBodSlopeTest"]),
        ("Bod_Cusum", &["tblBodCusum", "tblBodCusumAlarms"]),
    ] {
        let xml = sheet_xml(&p, &names, sheet);
        assert!(xml.contains("<drawing "), "{sheet} has a chart");
        assert!(xml.contains("conditionalFormatting"), "{sheet} has conditional formats");
        assert!(xml.contains("<pane "), "{sheet} freezes panes");
        assert_eq!(xml.matches("<tablePart ").count(), tables.len(), "{sheet} tables");
    }
    let wbx = part(&p, "xl/workbook.xml");
    for k in ["K_BOD_band_streak", "K_BOD_slope_lb_wk", "K_BOD_cusum_alarms", "Bod_ER_trend", "Bod_Fat_tail_kurt"] {
        assert!(wbx.contains(k), "defined name {k}");
    }
}

#[test]
fn lane_a_sheets_recalculate_to_the_reference_values() {
    let dir = tempfile::TempDir::new().unwrap();
    let Some(mut wb) = recalculated(dir.path()) else { return };
    let errs = errors(&mut wb);
    assert!(errs.is_empty(), "formula errors after recalculation: {errs:?}");
    let w = ws();
    let last = N - 1;

    // BOD-02: 7-day means, rate, band, streak.
    let r = wb.worksheet_range("Bod_Rate").unwrap();
    let obs = |a: usize, b: usize| (a..=b).filter_map(|i| w[i]).collect::<Vec<_>>();
    let (m7, m7p) = (mean(&obs(last - 6, last)), mean(&obs(last - 13, last - 7)));
    close(num(&at(&r, "date", "mean7", last as u32)), m7, "mean7");
    close(num(&at(&r, "date", "mean7_prev", last as u32)), m7p, "mean7_prev");
    close(num(&at(&r, "date", "rate_pct_wk", last as u32)), (m7 / m7p - 1.0) * 100.0, "rate");
    close(num(&right_of(&r, "Band low (%/week)")), -1.0, "cut band low");
    close(num(&right_of(&r, "Band high (%/week)")), -0.5, "cut band high");
    close(num(&right_of(&r, "Start weight (lb)")), w[0].unwrap(), "start weight");
    let lost = (1.0 - ff()[last].unwrap() / w[0].unwrap()) * 100.0;
    close(num(&right_of(&r, "Weight lost (%)")), lost, "% lost");
    let streak = num(&right_of(&r, "In-band streak (weeks)"));
    assert!(streak >= 0.0 && streak.fract() == 0.0, "streak {streak}");

    // BOD-03: moments of weight and a histogram that counts every weigh-in.
    let d = wb.worksheet_range("Bod_Dist").unwrap();
    let all: Vec<f64> = w.iter().flatten().copied().collect();
    close(num(&at(&d, "series", "n", 0)), all.len() as f64, "n weigh-ins");
    close(num(&at(&d, "series", "mean", 0)), mean(&all), "mean weight");
    close(num(&at(&d, "series", "sd", 0)), sd(&all), "sd weight");
    close(num(&at(&d, "series", "min", 0)), all.iter().copied().fold(f64::MAX, f64::min), "min weight");
    let kcal: Vec<f64> = (0..DAYS).filter_map(intake).map(|x| x.0).collect();
    close(num(&at(&d, "series", "mean", 2)), mean(&kcal), "mean kcal");
    let counts: f64 = (0..20).map(|i| num(&at(&d, "bin", "count", i))).sum();
    close(counts, all.len() as f64, "histogram total");
    let expected: f64 = (0..20).map(|i| num(&at(&d, "bin", "expected", i))).sum();
    assert!(expected > 0.8 * all.len() as f64 && expected <= all.len() as f64, "normal overlay {expected}");

    // BOD-04: ER and KAMA, EMA alongside.
    let s = wb.worksheet_range("Bod_Signal").unwrap();
    let k = kama();
    for i in [10usize, 30, 79, 80, 100, last] {
        close(num(&at(&s, "date", "er", i as u32)), k[i].0.unwrap(), &format!("ER day {i}"));
        close(num(&at(&s, "date", "kama", i as u32)), k[i].1.unwrap(), &format!("KAMA day {i}"));
    }
    assert_eq!(at(&s, "date", "er", 5), Cell::String(String::new()), "ER blank before the window fills");
    close(num(&at(&s, "date", "ema", last as u32)), ema(7.0)[last].unwrap(), "EMA from Bod_Weight");
    close(num(&right_of(&s, "KAMA (lb)")), k[last].1.unwrap(), "KAMA summary");
    let regime = at_str(&right_of(&s, "Regime"));
    assert!(regime == "trending" || regime == "noisy", "{regime}");

    // BOD-05: Bollinger mid / sd over the last 20 days of weigh-ins.
    let b = wb.worksheet_range("Bod_Bands").unwrap();
    let win = obs(last - 19, last);
    close(num(&at(&b, "date", "mid", last as u32)), mean(&win), "Bollinger mid");
    close(num(&at(&b, "date", "upper", last as u32)), mean(&win) + 2.0 * sd(&win), "Bollinger upper");
    let breaches = num(&right_of(&b, "Breaches above")) + num(&right_of(&b, "Breaches below"));
    assert!(breaches >= 1.0, "the +3 lb step breaches the band");

    // BOD-06: Kalman recursion.
    let kf = wb.worksheet_range("Bod_Kalman").unwrap();
    let want = kalman();
    for i in [0usize, 1, 4, 13, 52, 80, last] {
        close(num(&at(&kf, "date", "x", i as u32)), want[i].0, &format!("Kalman x day {i}"));
        close(num(&at(&kf, "date", "p", i as u32)), want[i].1, &format!("Kalman P day {i}"));
    }
    close(num(&right_of(&kf, "True weight (lb)")), want[last].0, "Kalman summary");
    close(num(&right_of(&kf, "Band +- (lb, RCV_z sd)")), 1.96 * want[last].1.sqrt(), "Kalman band");

    // BOD-07: rolling OLS slope and its t.
    let sl = wb.worksheet_range("Bod_Slope").unwrap();
    for i in [40usize, 85, last] {
        let (b14, se14) = ols(i, 14);
        let (b28, se28) = ols(i, 28);
        close(num(&at(&sl, "date", "slope_short_wk", i as u32)), b14, &format!("slope14 day {i}"));
        close(num(&at(&sl, "date", "se_short", i as u32)), se14, &format!("se14 day {i}"));
        close(num(&at(&sl, "date", "slope_long_wk", i as u32)), b28, &format!("slope28 day {i}"));
        close(num(&at(&sl, "date", "t_long", i as u32)), b28 / se28, &format!("t28 day {i}"));
    }
    close(num(&at(&sl, "window", "slope_lb_wk", 1)), ols(last, 28).0, "KPI cell: latest long slope");
    let rho = num(&right_of(&sl, "Residual lag-1 autocorrelation (rho)"));
    assert!((-1.0..=1.0).contains(&rho), "rho {rho}");
    let verdict = at_str(&at(&sl, "window", "verdict", 1));
    assert!(["falling", "rising", "flat (noise)", "too few days"].contains(&verdict.as_str()), "{verdict}");

    // BOD-08: the +3 lb step at day 80 raises the first alarm within a few days.
    let c = wb.worksheet_range("Bod_Cusum").unwrap();
    let alarms = num(&right_of(&c, "Alarms"));
    assert!(alarms >= 1.0, "CUSUM alarms {alarms}");
    let first = num(&at(&c, "alarm_seq", "day_row", 0)) as i64 - 1;
    assert!((80..=84).contains(&first), "first alarm on day {first}");
    assert_eq!(at_str(&at(&c, "alarm_seq", "direction", 0)), "up");
    let start = num(&at(&c, "date", "date", 0));
    close(num(&at(&c, "alarm_seq", "date", 0)), start + first as f64, "alarm date");
}

fn at_str(v: &Cell) -> String {
    match v {
        Cell::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[test]
fn lane_a_sheets_recalculate_without_errors_on_an_empty_log() {
    let Some(so) = soffice() else {
        skip("soffice (LibreOffice) not installed: empty-log recalculation not checked");
        return;
    };
    let dir = tempfile::TempDir::new().unwrap();
    let mut data = fixture();
    data.intake.clear();
    data.labs = None;
    let mut book = health_cli::workbook::build(&data, &[]).expect("workbook builds without a log");
    let p = dir.path().join("empty.xlsx");
    book.save(&p).unwrap();
    let mut wb = recalc(&so, dir.path(), &p);
    let mine = ["Bod_Rate", "Bod_Dist", "Bod_Signal", "Bod_Bands", "Bod_Kalman", "Bod_Slope", "Bod_Cusum"];
    let errs: Vec<String> = errors(&mut wb).into_iter().filter(|e| mine.iter().any(|s| e.starts_with(s))).collect();
    assert!(errs.is_empty(), "formula errors on an empty log: {errs:?}");
    let c = wb.worksheet_range("Bod_Cusum").unwrap();
    close(num(&right_of(&c, "Alarms")), 0.0, "no alarms without data");
}
