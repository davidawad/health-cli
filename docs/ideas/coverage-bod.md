# Coverage: W5 BOD (body weight, nutrition, sleep, wearables)

Every BOD id of [MASTER.md](MASTER.md) section 3, the sheet that implements it and its
status in the generated workbook. Code: `src/workbook/sheets/bod/` (one module per sheet,
constants in `bod/params.rs`, the id -> sheet -> status list is `bod::COVERAGE`); tests:
`tests/sheet_bod.rs` (+ `tests/sheet_bod/`) and unit tests in the modules.

Status as shipped: **BUILDABLE-NOW** full build: formulas over the data layer (`Bod_Cal` /
`tblDaily`, `tblNutrition`, `tblSizes`, `tblLabs`) or a typed entry / calculator sheet;
**PLACEHOLDER** full view over an empty input (typed `tblBod*` log or the empty `tblWearable`)
with the `PLACEHOLDER - awaiting data` banner, computing as soon as rows arrive. Constants that
are not checked against their reference are `UNVERIFIED` Params rows with `pending: <reference>`.

Time series run over `Bod_Cal` (helper sheet, `bod/cal.rs`): one row per calendar day from the
first to the last `tblDaily` date, `AVERAGEIFS` by date, so windows and lags in rows are windows
and lags in days whatever gaps the log has. Weight is in lb (as `tblDaily`); `weight_kg` uses
`Bod_lb_per_kg`. Chart-only columns hold `#N/A` (`NA()`) on missing days so lines show gaps.

Assumption A1 (daily kcal / protein) is met: `tblDaily` carries `kcal` and `protein_g` from the
intake import, so BOD-18, BOD-19 and BOD-20 ship BUILDABLE-NOW (MASTER marked them PH pending A1).

| id | sheet | status | master | what it reads | what it computes |
| --- | --- | --- | --- | --- | --- |
| BOD-01 | Bod_Weight | BUILDABLE-NOW | BN | Bod_Cal (tblDaily) | Weigh-in dots, time-based 7-day EMA (`Bod_EMA_span`, gap-aware `1-(1-a)^g`), weekly EMA rate, kcal, protein g/kg, target-day flag and a 26-week calendar heatmap. KPIs `K_BOD_weight_ema`, `K_BOD_rate_wk`. |
| BOD-02 | Bod_Rate | BUILDABLE-NOW | BN | Bod_Cal | 7-day mean vs previous 7 days -> %/week; mode selector (cut / gain bands from HELMS ch01, %/month converted to %/week / maintain); in-band flag; 26 weekly rows with streak; % lost vs `Bod_Loss_line_pct` (DIP ch167:19). KPI `K_BOD_band_streak`. |
| BOD-03 | Bod_Dist | BUILDABLE-NOW | BN | Bod_Cal | n, mean, STDEV.S, SKEW, KURT, median, min/max, P5/P95 for weight, daily change, kcal, protein; selected-series 20-bin COUNTIFS histogram with NORM.DIST expected counts; fat-tail flag (`Bod_Fat_tail_kurt`). |
| BOD-04 | Bod_Signal | BUILDABLE-NOW | BN | Bod_Cal, Bod_Weight EMA | Efficiency ratio over `Bod_KAMA_n` days, smoothing constant from `KAMA_fast/slow`, KAMA recursion, regime label (`Bod_ER_trend`). |
| BOD-05 | Bod_Bands | BUILDABLE-NOW | BN | Bod_Cal | Bollinger mid / sd / upper / lower over `Bod_Boll_days` (`Boll_K`), %b, bandwidth, breach markers (`NA()` gaps), breach counts. |
| BOD-06 | Bod_Kalman | BUILDABLE-NOW | BN | Bod_Cal | Local-level Kalman filter (`Kalman_q`, `Kalman_r` in lb^2): predicted P, gain, filtered weight, ±`RCV_z`·sqrt(P) band, innovations. |
| BOD-07 | Bod_Slope | BUILDABLE-NOW | BN | Bod_Cal | Rolling 14/28-day OLS slope (lb/week) by masked SUMPRODUCT sums, SE, t; residual lag-1 rho, N_eff, adjusted t vs `T.INV.2T(Alpha,N_eff-2)`. KPI `K_BOD_slope_lb_wk`. |
| BOD-08 | Bod_Cusum | BUILDABLE-NOW | BN | Bod_Cal, Bod_Weight EMA | Two-sided CUSUM (`CUSUM_k`, `CUSUM_h`) with reset; monitors `level` (weigh-in minus prior EMA, default: a step shows as a level shift) or `rate` (standardised daily change); alarm list via running counter + INDEX/MATCH. KPI `K_BOD_cusum_alarms`. |
| BOD-09 | Bod_Patterns | BUILDABLE-NOW | BN | Bod_Cal | Shock days (> `Bod_Shock_x` × mean abs change) with dated list; weekday mean/sd/n of change plus kcal/protein; autocorrelation lags 1-35 with Rust-generated shifted ranges and ±z/sqrt(n) bounds; first significant peak = cycle. |
| BOD-10 | Bod_Revert | BUILDABLE-NOW | BN | Bod_Cal | Deviation from a `Bod_Revert_ma`-day mean; AR(1) b, intercept, R2 over `Bod_Revert_days`; half-life LN(2)/-LN(b) or "no reversion". |
| BOD-11 | Bod_Osc | BUILDABLE-NOW | BN | Bod_Cal | Stochastic %K over `Bod_Osc_days` (window MIN/MAX), %D, top/bottom zones (`Bod_Osc_hi/lo`). |
| BOD-12 | Bod_Fan | BUILDABLE-NOW | BN | Bod_Cal | Drift and sd of daily change over `Bod_Fan_hist`; median ± z·sd·sqrt(t) at 50/80/95% to `Bod_Fan_days`; P(at/below goal); date the median reaches `Bod_Goal_lb`. Labelled analytic, not a forecast. |
| BOD-13 | Bod_Glide | BUILDABLE-NOW | BN | Bod_Cal, Params inputs | Plan line to `Bod_Goal_lb` / `Bod_Goal_date` (default: `Bod_Cut_lo_pct` %/week when goals are blank), deviation, running tracking error, best-so-far, underwater regain curve, days since best. |
| BOD-14 | Bod_Regime | PLACEHOLDER | BN (GATE-DATES) | tblBodRegimes (typed), tblRegimes, tblSupp dates | GATE-DATES failed (tblRegimes and tblSupp.start_date/stop_date empty). Per-day regime label, per-regime n / mean / sd of change, kcal, protein; Welch t, df, p = T.DIST.2T between two selected regimes (no T.TEST array). |
| BOD-15 | Bod_Sens | BUILDABLE-NOW | BN | Bod_Cal | Explicit grids: Bollinger window (10-42 d) × k (1.5-3) breach share; EWMA alpha 0.05-0.5 one-step RMSE with best alpha; 3-colour scales. |
| BOD-16 | Bod_Wearable | PLACEHOLDER | PH | tblWearable (empty), Bod_Alcohol / Bod_Caffeine / Bod_Window logs, Age_Zone2 | Acute/chronic strain EWMA and ACWR (`Bod_ACWR_lo/hi` UNVERIFIED, Gabbett 2016), HRV 7/28-day baselines and z-bands, RHR baseline, weekly strain vs zone-2 minutes, pairwise Pearson matrix vs alcohol / caffeine / last-meal gap. |
| BOD-17 | Bod_BMI | BUILDABLE-NOW | BN | Bod_Cal, Height_cm, tblSizes waist, Sex | BMI per day, NIH class (`Bod_BMI_*`, DIP ch164:239-245), refeeding note below `Bod_BMI_refeed`; waist vs sex cut-off (`Bod_Waist_*_cm` UNVERIFIED, NHLBI 1998), waist-to-height; rule table. KPI `K_BOD_bmi`. |
| BOD-18 | Bod_Protein | BUILDABLE-NOW | PH (A1) | Bod_Cal protein_g, Calc_Marker egfr | g/kg band (0.8/1.6/2.2/3.7, OUT ch50), at-target flag, run-length streaks (longest, current, count, mean), per-meal and daily targets; eGFR ceiling (`Bod_eGFR_kidney`, `Bod_Protein_kidney_g_kg` UNVERIFIED, KDIGO). KPI `K_BOD_protein_days_pct`. |
| BOD-19 | Bod_TDEE | BUILDABLE-NOW | PH (A1) | Bod_Cal kcal / energy_out, Bod_Weight EMA | Implied TDEE = mean kcal − `Bod_kcal_per_kg` × EMA kg change / days over `Bod_TDEE_days`; crude-rule footnote (HELMS ch01:957). KPI `K_BOD_tdee`. |
| BOD-20 | Bod_LeadLag | BUILDABLE-NOW | PH (A1) | Bod_Cal | Pairwise Pearson (masked SUMPRODUCT) of kcal and protein vs daily weight change for lags 0-14, n, ±2/sqrt(n) bound, strongest lag. |
| BOD-21 | Bod_Macro | PLACEHOLDER | PH | tblBodMacroLog (typed), Bod_Cal weight | tblDaily has no carbs/fat. Atwater kcal (`Bod_Atwater_*`), fat % vs cut/gain band (HELMS ch01), fat floor g/lb, protein g/kg, 7-entry drift. |
| BOD-22 | Bod_Dozen | BUILDABLE-NOW | BN | tblNutrition, typed grid and eat log | Greger rule table (berries and other vegetables UNVERIFIED pending GREG ch20 Figure 6), nutrition-item mapper, 28-day grid (typed + mapped SUMIFS), score out of 24, weekly % per category, sparklines. |
| BOD-23 | Bod_Yield | BUILDABLE-NOW | BN (GATE-FIELDS) | tblNutrition | GATE-FIELDS passes (price, protein, calories, yuka, portions). $/g protein, kcal/g protein, protein/100 kcal, dominance frontier (COUNTIFS), rank, two scatter charts. tblProducts has no macros. |
| BOD-24 | Bod_Glycemic | PLACEHOLDER | PH | tblBodHacks / tblBodCGM / tblBodFibre (typed), Bod_Cal kcal | Hack compliance, CGM mean / SD / CV / time in `Bod_CGM_lo`-`Bod_CGM_hi`, spikes (rise > `Bod_Spike_rise`), fibre vs `Bod_Fibre_g` and per 1000 kcal, hack-score vs spikes r; [DX-RISK] neutral note. |
| BOD-25 | Bod_Window | PLACEHOLDER | PH | tblBodMeals (typed) | First / last meal (MINIFS / MAXIFS), window hours, meals, skips per day, share within `Bod_Window_ref_h` (reference, no target). |
| BOD-26 | Bod_Alcohol | PLACEHOLDER | PH | tblBodAlcohol (typed), tblWearable, Bod_Sleep | Drinks vs next-night efficiency and HRV (pairwise r, n), dry vs drinking-night means. |
| BOD-27 | Bod_Sleep | PLACEHOLDER | PH | tblBodSleepLog (typed), tblWearable | Time in bed across midnight, efficiency vs `Bod_Sleep_eff`, opportunity vs 8-9 h, wake-time SD over `Bod_Wake_SD_nights`, weekend drift, 7-night debt ledger. KPI `K_BOD_sleep_h`. |
| BOD-28 | Bod_Arch | PLACEHOLDER | PH | tblBodStages (typed), tblWearable | Stage minutes and shares, expected cycles = TIB / `Bod_Cycle_min`, rolling baseline and deviation, stacked chart. |
| BOD-29 | Bod_Caffeine | PLACEHOLDER | PH | tblBodCaffeine (typed), tblSupp + Ref_TimeSlots | Half-life = override or IFERROR(`G_Caffeine_HL`, `Caffeine_HL`); mg active at `Bod_Bedtime` = SUMPRODUCT(mg·0.5^(dt/HL)); stack-based bedtime caffeine from tblSupp; decay curve. KPI `K_BOD_caffeine_bed`. |
| BOD-30 | Bod_SleepHyg | PLACEHOLDER | PH | tblBodHygiene / tblBodSTOPBANG (typed), BOD-24/25/27/29 logs, tblSupp | Outlive 10-rule scorecard vs next-day HRV / efficiency, last-meal and last-caffeine gaps, evening supplements, sleep × glucose slope, chronotype card, STOP-BANG slot (`Bod_STOPBANG_flag` UNVERIFIED, Chung 2008; neutral wording), PSQI slot, quick panel. |

## Parameters

All BOD constants are `Params` rows registered by `bod/params.rs` (prefix `Bod_`), plus the infra seeds
`KAMA_fast/slow`, `Kalman_q/r`, `CUSUM_k/h`, `Boll_K`, `Alpha`, `RCV_z`, `Caffeine_HL`, `Protein_*_g_kg`.
UNVERIFIED (pending): `Bod_ACWR_lo/hi` (Gabbett 2016), `Bod_Waist_m_cm/f_cm` (NHLBI 1998), `Bod_eGFR_kidney`
(KDIGO 2012), `Bod_Protein_kidney_g_kg` (KDIGO 2020), `Bod_STOPBANG_flag` (Chung 2008); Daily Dozen berries and
other-vegetables servings (GREG ch20 Figure 6) in `tblBodDozenRef`. Catalogued book values ship `verified` with
the citation and the catalogue they were taken from; windows and detector thresholds are `heuristic`; goals,
bedtime, kcal target and waist are `input`.

## KPIs (10 of 10)

`K_BOD_weight_ema`, `K_BOD_rate_wk`, `K_BOD_band_streak`, `K_BOD_slope_lb_wk`, `K_BOD_cusum_alarms`, `K_BOD_bmi`,
`K_BOD_protein_days_pct`, `K_BOD_tdee`, `K_BOD_sleep_h`, `K_BOD_caffeine_bed`.
