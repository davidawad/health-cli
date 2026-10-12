# Quant / hedge-fund analytics -> Health Workbook (bead hx-rx3.3)

Angle: take every technique a quant uses to look at time series, portfolios, risk, attribution, regimes, factors, signal vs noise, backtests, and dashboards, and map it onto David's data.

## Honesty notes (read first)
- The library is thin on classical statistics (`~/Knowledge/Data/Books` is empty; `Data/Statistics` holds only I.J. Good). The richest sources are Paleologo, Kaufman, Chan, Faber, Bernstein, Duke and the FRI forecasting papers. I skimmed headings plus selected passages; I did not read any book cover to cover.
- Techniques come from the library (cited). Medical claims are NOT asserted here. Where a medical rationale needs a source I mark `[MED-VERIFY]`. Techniques that are standard but not in the library are marked `[EXT]` (e.g. reference change value, CUSUM).
- Data are tiny on the lab side (about 7 panels per marker) and moderate on the daily side (about 250 days of weight/kcal/protein). Almost every lab-side statistic is descriptive and carries a small-N warning. A recurring design rule from the sources: show the uncertainty, not just the point estimate (Paleologo Ch3 FAQ 3.2, Ch14 Insight 14.1).
- Path shorthand:
  - PAL = `~/Knowledge/Investing/Algotrading/Books/The Elements of Quantitative Investing by Giuseppe Paleologo/` (book Ch N = file ch(N+7).md: Ch2=ch09, Ch3=ch10, Ch4=ch11, Ch5=ch12, Ch6=ch13, Ch7=ch14, Ch8=ch15, Ch9=ch16, Ch10=ch17, Ch12=ch19, Ch13=ch20, Ch14=ch21)
  - KAU = `~/Knowledge/Investing/Algotrading/Books/Trading Systems and Methods - Kaufman/` (book Ch N = file ch(N+2).md: Ch2=ch04, Ch6=ch08, Ch7=ch09, Ch9=ch11, Ch10=ch12, Ch11=ch13, Ch13=ch15, Ch15=ch17, Ch17=ch19, Ch21=ch23, Ch22=ch24, Ch23=ch25, Ch24=ch26)
  - CHAN = `~/Knowledge/Investing/Algotrading/Books/Quantitative Trading - Chan/ch00.md` (one flat file; TOC at line ~825; drawdown ~L3169; data-snooping ~L3551 and L6013; Kelly appendix ~L809)
  - FABER = `.../Algotrading/Books/global-asset-allocation-faber/` and `.../Algotrading/Papers/faber-taa-2007/`
  - FO = `~/Knowledge/Investing/Public Markets/Books/The Front Office - Tom Costello/`
  - BERN = `.../Algotrading/Books/Against the Gods_ ... Bernstein/`
  - DUKE = `~/Knowledge/Investing/Public Markets/Books/Thinking in Bets by Annie Duke/`
  - FRI = `~/Knowledge/General/Forecasting/FRI-Good-Judgment/`
  - GOOD = `~/Knowledge/Data/Statistics/Papers/Rational Decisions by I. J. Good/`
- Effort: S = under an hour of Rust, M = half a day, L = a day or more.
- Excel feature assumptions (rust_xlsxwriter): tables, formulas incl. dynamic arrays, conditional formats (3-color scale, data bar, icon set, formula-based), sparklines, charts (line, scatter, column, stacked, bar, area, radar, combo), data validation. Data Tables (what-if) are not supported, so any "grid of parameters" idea is built with explicit formula grids.

## Master design pattern borrowed from quant shops
1. A raw `Data_*` layer (long format: date, marker, value, unit, lab) as Excel Tables.
2. A `Calc_*` layer (per-marker stats: baseline, SD, z, slope, RCV...) as formulas pointing at the Tables.
3. `Tearsheet` / `Dashboard` sheets that pull from Calc through one selector cell (data-validation dropdown) with INDEX/MATCH or XLOOKUP, so one page serves 56 markers (Front Office Ch13 "Facts and Evidence": allocators want evidence, drawdown first).
4. A `Checks` sheet (data-quality assertions) - see idea 89.

---

# THEME A - Normalization and baselines

### 1. Personal-baseline z-score per marker
- Shows: for each marker x panel, (value - personal mean) / personal SD; a heat grid (rows = 56 markers, columns = panels) so drift across years is visible in one screen. Complements reference/optimal ranges with "unusual for ME".
- Inputs: panel values (56 markers x ~7), no ranges needed.
- Excel: `=(v-AVERAGE(row))/STDEV.S(row)` (STDEV.S needs >=3 points; wrap in IF(COUNT<3,"")); 3-color scale conditional format centered at 0 (red-white-red absolute so both directions flagged). Use EWMA mean as a variant (idea 3).
- Why interesting: the same absolute value can be normal for the population and abnormal for the individual. Population reference intervals vs individual baselines is established in clinical chemistry `[MED-VERIFY]` (index of individuality, Fraser; not in library).
- Effort: S. Source: KAU Ch2 "Standardizing Risk and Return" (ch04.md); PAL Ch2.1 returns/excess (ch09.md).

### 2. Percentile-of-own-history and "rank among my panels"
- Shows: where today's value sits in own history (PERCENTRANK.INC) plus "best ever / worst ever" badge.
- Inputs: panel values.
- Excel: `PERCENTRANK.INC(range,x)`, icon set (traffic lights) conditional format; badge via `IF(x=MAX(...),"high")`.
- Why: instant orientation; analog of "percentage of positive years" and extreme tracking.
- Effort: S. Source: KAU Ch10 "Percentage of Positive Years" (ch12.md); KAU Ch2 median/mode/frequency distributions (ch04.md).

### 3. EWMA baseline with a half-life input
- Shows: exponentially weighted personal mean and SD, with `lambda` set by a single input cell (half-life in panels or days), so recent panels count more. Compare static vs EWMA z.
- Inputs: panel values; daily weight/kcal/protein.
- Excel: recursion `m_t = lambda*m_{t-1}+(1-lambda)*x_t` in a helper column; variance recursion the same; named cell `Half_Life`; `lambda = 0.5^(1/half_life)`.
- Why: separates stable markers from drifting ones; same machinery as GARCH/EWMA volatility estimators.
- Effort: S. Source: PAL Ch2 sec 2.3.1 "Muth's Original Model: EWMA", Insight 2.2 (ch09.md); KAU Ch7 "Exponential Smoothing" (ch09.md).

### 4. Distance-to-optimal "health score" (0-100) per marker, rolled into a composite
- Shows: each marker mapped to 0-100 by piecewise-linear distance from the optimal band (100 inside optimal, falling to 0 at the reference-range edge or beyond); then system roll-ups (lipids, thyroid, CBC, liver, glycemia) and an overall index over time.
- Inputs: 56 markers with reference low/high and optimal low/high; panel values.
- Excel: `=MAX(0,MIN(1,(v-ref_lo)/(opt_lo-ref_lo)))` style for each side with IFS; weights table (user-editable) -> SUMPRODUCT index; line chart of the index by panel; radar chart for the latest panel vs first panel.
- Why: a single trackable KPI, like an index level; mirrors how a fund tracks NAV.
- Effort: M. Source: KAU Ch2 "The Index / Constructing an Index" (ch04.md); PAL Ch9.6 "Aggregation: Signals versus Portfolios" (ch16.md).

### 5. Winsorize/clip outliers in calc layer
- Shows: toggle (data validation Y/N) that clips z beyond +/-3 for charts so one assay glitch does not blow the color scale.
- Inputs: panel values; z from idea 1.
- Excel: `=MEDIAN(-cap,z,cap)` with `cap` a named cell.
- Why: assay error and one-off events are the health analog of bad ticks.
- Effort: S. Source: PAL Ch6 sec 6.5 "Winsorization of Returns" (ch13.md); KAU Ch22 "Data Accuracy / Clean Data" (ch24.md).

### 6. Percent-change and annualized-rate-of-change between panels
- Shows: panel-to-panel % change and per-year rate (divide by elapsed years), plus log-change option so +50%/-33% symmetrical.
- Inputs: panel values + panel dates.
- Excel: `=LN(v2/v1)/((d2-d1)/365.25)`; heat map; data bars.
- Why: comparable velocity across panels spaced 6 months to 3 years apart (panels are irregularly spaced!).
- Effort: S. Source: PAL Ch2 sec 2.1.3 "Log Returns" (ch09.md); KAU Ch2 "Annualizing Returns" (ch04.md).

### 7. Lab / assay-method change flags
- Shows: a column recording lab, method, unit and fasting status per panel; chart markers point-style by lab so you can see whether a step change coincides with a lab switch.
- Inputs: panel metadata (lab, units, fasting).
- Excel: data validation list for lab; chart series split by lab via helper columns; formula flag `=IF(lab<>prev_lab,"lab change","")`.
- Why: "point-in-time data" and data-sourcing hygiene: a discontinuity from a methodology change is not a real change.
- Effort: S. Source: PAL Ch8 sec 8.1.1 "Data Sourcing" and "Point-in-time data" (ch15.md); KAU Ch22 (ch24.md).

### 8. Unit/derived ratio engine ("spreads")
- Shows: derived markers computed live: TG/HDL, non-HDL (TC-HDL), ApoB/ApoA1 (if present), LDL-vs-ApoB discordance (z-score gap), BUN/creatinine, AST/ALT, NLR (neutrophil/lymphocyte from CBC), TC/HDL, Glucose x TG (TyG index = ln(TG*glucose/2)).
- Inputs: lipids, CMP, CBC, A1c.
- Excel: ordinary formulas into a `Derived` table with its own ref/optimal columns; discordance = z(LDL) - z(ApoB).
- Why: ratios and discordance behave like spreads in pairs trading: the relationship carries signal the legs hide. `[MED-VERIFY]` for each ratio's clinical meaning.
- Effort: M. Source: KAU Ch13 "Spreads and Arbitrage" (ch15.md); CHAN Ch7 "Stationarity and Cointegration" (L829).

---

# THEME B - Marker tear sheets (the "fund fact sheet" per marker)

### 9. One-page marker tear sheet with a selector dropdown (TOP)
- Shows: pick a marker -> current value vs ref/optimal band (chart with shaded bands), panel history line, z-score, percentile, slope, time-in-range, best/worst, next-expected value with interval, RCV flag, linked supplements started near its history, plus sparkline.
- Inputs: all panels; supplements start dates; marker ranges.
- Excel: dropdown (data validation list from marker table); INDEX/MATCH or `XLOOKUP`/dynamic `FILTER` to pull the row; chart references fixed helper range that re-evaluates; shaded bands built as stacked area series (ref low->optimal low, optimal band, optimal high->ref high).
- Why: allocators' "tear sheet" gives the same ten stats for every fund so comparisons are instant; same here, 56 markers one layout.
- Effort: M. Source: FO Ch13 "Facts and Evidence" (ch13.md); KAU Ch23 "Measuring Return and Risk" (ch25.md).

### 10. Fund-style "monthly returns table" -> marker x panel grid with year summary columns
- Shows: rows = markers, columns = panels, plus trailing columns: first value, last value, change, CAGR-style rate, min, max, #panels in range.
- Inputs: panel values.
- Excel: structured table with computed columns; sparkline column (line sparkline over panel columns) at the end of each row; color scale per row.
- Why: the standard fund monthly-returns grid, scannable in seconds.
- Effort: S. Source: FABER periodic table of returns discussion, `global-asset-allocation-faber/ch15.md` ("Comparison of the Strategies"); KAU Ch23 (ch25.md).

### 11. "Periodic table of returns" quilt of marker status
- Shows: per panel, each marker colored by status (low / optimal / high-normal / out-of-range) in a quilt, optionally sorted by rank within panel; the quilt makes the "which system is dragging" story visual.
- Inputs: panel values + ranges.
- Excel: formula status code (1-4) -> conditional format with custom colors via formula rules; sorting done in Rust at generation time (static) or by `SORTBY` dynamic array.
- Why: the allocation literature uses this to show regime-dependence of winners/losers.
- Effort: M. Source: `global-asset-allocation-faber/ch15.md` (periodic table of returns passage).

### 12. Time-in-optimal-range (hit rate)
- Shows: % of panels each marker was in the optimal band and in the reference band; whole-body "% of marker-panels in optimal" trend.
- Inputs: panel values + optimal/reference bounds.
- Excel: `=COUNTIFS(range,">="&lo,range,"<="&hi)/COUNT(range)`; data bars; per-system roll-up with SUMPRODUCT.
- Why: tracks consistency, not just latest value; same as "percentage of positive years/hit rate."
- Effort: S. Source: KAU Ch10 "Percentage of Positive Years" (ch12.md).

### 13. Drawdown from personal best (peak-to-trough, in z or %)
- Shows: for each marker, how far the latest panel sits from the best value ever reached (direction-aware: best = max for HDL/vit D, min for ApoB/LDL/A1c), and the longest stretch spent below the prior best ("drawdown duration").
- Inputs: panel values, direction flag per marker (higher-better / lower-better / target-band).
- Excel: running best `=IF(dir="hi",MAX($B2:B2),MIN($B2:B2))`; drawdown `=(best-v)/best*dirsign`; duration counter `=IF(v is new best,0,prev+Δdays)`; area chart of "underwater curve".
- Why: the underwater plot makes regression visible even when the latest number is still "in range".
- Effort: M. Source: CHAN drawdown/max-drawdown duration (ch00.md ~L3169-3197); KAU Ch23 "Average Maximum Retracement", "Time to Recovery" (ch25.md).

### 14. Ulcer Index for markers
- Shows: single number per marker = RMS of percentage drawdown from running best across panels; ranks markers by "how much anxiety they cause".
- Inputs: panel values.
- Excel: SUMSQ of drawdown column / COUNT, SQRT; bar chart sorted.
- Why: weights depth AND duration of regressions (a high ulcer index = long, deep slumps), unlike max-min alone. Kaufman's text explicitly ties this to investor anxiety.
- Effort: S. Source: KAU Ch23 "Ulcer Index" (ch25.md).

### 15. Time-to-recovery after an excursion out of range
- Shows: for each out-of-range episode: how many panels/days until back in range; median recovery time by marker; episodes table.
- Inputs: panel values + dates + ranges.
- Excel: flag column, episode id via cumulative counter, MINIFS/MAXIFS by episode, dates difference.
- Why: some markers snap back, some do not; helps decide how long to wait before retesting.
- Effort: M. Source: KAU Ch23 "Time to Recovery" (ch25.md); CHAN "maximum drawdown duration" (ch00.md ~L3187).

### 16. Linear-regression trend line with confidence bands and forecast
- Shows: per marker, OLS trend vs time (days), slope per year with standard error, R^2, projected next value at a chosen date, band = prediction interval; chart with data points, trendline, bands, ref/optimal bands.
- Inputs: panel values + dates.
- Excel: `SLOPE, INTERCEPT, RSQ, STEYX`; SE slope = `STEYX/SQRT(DEVSQ(x))`; band = fit +/- T.INV.2T(0.05,n-2)*STEYX*SQRT(1+1/n+(x-AVERAGE(x))^2/DEVSQ(x)); chart with 3 line series. Alternative: `LINEST(...,TRUE,TRUE)` for full stats array.
- Why: with n=7 intervals will be wide; showing that width is the point (is the slope distinguishable from zero?).
- Effort: M. Source: KAU Ch6 "Linear Regression", "Confidence Bands", "Forecast Oscillator" (ch08.md); PAL Ch3 FAQ 3.2 (SE of an estimate, ch10.md).

### 17. Signal-strength gauge: t-stat of trend
- Shows: `slope/SE` per marker with a verdict: "trend distinguishable from noise" vs "noise-like" (t vs T.INV threshold), shown as an icon set.
- Inputs: panel values + dates.
- Excel: `=slope/se`; `=IF(ABS(t)>T.INV.2T(.05,n-2),"signal","noise")`.
- Why: Sharpe ratio = t-statistic of the mean (iid); the same logic stops over-reading drift in 7 points.
- Effort: S. Source: PAL Ch3 sec 3.3 (Sharpe = t-stat, ch10.md).

### 18. Distribution view: histogram + moments of daily weight/kcal/protein
- Shows: mean, SD, skew, kurtosis (SKEW, KURT), histogram with normal overlay for daily series; flags fat tails (binge/skip days).
- Inputs: daily weight, kcal, protein (Nov 2023-Jul 2024).
- Excel: `FREQUENCY` for bins, column chart + scatter-smooth normal curve (`NORM.DIST`).
- Why: calories have fat tails; averages mislead. Same logic as non-normal return distributions.
- Effort: S. Source: KAU Ch2 "Moments of the Distribution: Mean, Variance, Skewness, Kurtosis", "Moments Using Excel" (ch04.md); PAL Ch2 "Stylized Facts" (ch09.md).

---

# THEME C - Signal vs noise (change detection)

### 19. Reference change value (RCV) / "is this change real?" flag (TOP) `[EXT]`
- Shows: for each marker and each panel-to-panel change: |change| vs a noise threshold built from the personal variability of that marker (SD of changes or CV) and, optionally, analytical variability; labelled "within noise" / "meaningful".
- Inputs: panel values; (optional) lab-stated analytical CV per assay if David supplies it.
- Excel: noise threshold = `1.96*SQRT(2)*SQRT(CVa^2+CVi^2)` (the standard RCV form, Fraser; not in library) `[MED-VERIFY]` - until CVa/CVi available use empirical `1.96*SQRT(2)*STDEV.S(diffs)/SQRT(2)` as a proxy; conditional format red/green on |change|/RCV.
- Why: most "improvements" across 7 panels may be inside noise; this is the biggest honesty feature for the workbook. Sharpe CIs in finance serve the same purpose.
- Effort: M. Source: technique `[EXT]`; the design philosophy comes from PAL Ch3 FAQ 3.2 and Ch14 Insight 14.1 "Reporting standard errors for attributions" (ch10.md, ch21.md).

### 20. Regression-to-the-mean warning (TOP)
- Shows: flag when a marker's "improvement" follows an extreme prior reading (high z at prior panel); estimates the portion of change expected by reversion alone = `(1-r)*(prior-mean)` with r = lag-1 autocorrelation of that marker (or pooled across markers).
- Inputs: panel values.
- Excel: `CORREL(x[1:n-1],x[2:n])` pooled over z-scored markers (stack in helper range); expected-reversion column; "net of reversion" change column.
- Why: you tend to start a protocol after a bad lab; part of the following improvement is statistical reversion, not the protocol. This is the single most important caveat for any event study (idea 32).
- Effort: M. Source: BERN Ch12 "Peapods and Perils" (Galton, regression to the mean; `Against the Gods.../ch12.md`); PAL Ch2 stylized facts (ch09.md).

### 21. Efficiency ratio (signal/noise) for daily weight
- Shows: ER over a rolling window = |net change over n days| / sum of |daily changes| (0 = pure noise, 1 = straight line); line chart plus color band.
- Inputs: daily weight.
- Excel: `=ABS(w_t - w_{t-n}) / SUMPRODUCT(ABS(w[t-n+1..t]-w[t-n..t-1]))` (helper column with abs daily diff, then SUM over window).
- Why: tells you whether the scale is currently showing a trend or water-weight noise; Kaufman's own adaptive-trend input.
- Effort: S. Source: KAU Ch17 "Kaufman's Adaptive Moving Average" (ch19.md).

### 22. Adaptive (KAMA) weight trend line
- Shows: a smoothing line that is fast when ER is high and slow when ER is low; overlay on raw daily weight.
- Inputs: daily weight.
- Excel: `SC = (ER*(fast-slow)+slow)^2`, `KAMA_t = KAMA_{t-1} + SC*(w_t - KAMA_{t-1})`; fast=2/(2+1), slow=2/(30+1) as named parameters; line chart.
- Why: avoids the lag of a fixed SMA during real change and the whipsaw during flat periods.
- Effort: M. Source: KAU Ch17 (ch19.md).

### 23. Personal Bollinger bands on daily weight/kcal/protein/HRV
- Shows: n-day SMA with +/-k SD bands; days breaching are highlighted; "band width" series (volatility of me).
- Inputs: daily series.
- Excel: `AVERAGE(OFFSET)` avoided - use fixed-size window formulas generated in Rust (`AVERAGE(B2:B21)` per row); STDEV.S window; line chart with 3 series + marker series for breaches via `IF(w>upper,w,NA())`.
- Why: standard way to define "unusual for me today" and to see compressing/expanding variability.
- Effort: S. Source: KAU Ch7 "Standard Deviation Moving Average" (ch09.md), Ch4 charting systems (ch06.md).

### 24. Local-level Kalman filter for weight (true weight vs noisy scale)
- Shows: filtered "true weight" and its uncertainty band, plus Kalman gain over time.
- Inputs: daily weight; two inputs: process noise q, measurement noise r (defaults estimated from data).
- Excel: `P_pred = P + q; K = P_pred/(P_pred+r); x = x + K*(w-x); P = (1-K)*P_pred`; chart with band.
- Why: the textbook "signal in noise" model; shows how much of day-to-day movement is hydration. PAL shows EWMA is the steady-state Kalman filter.
- Effort: M. Source: PAL Ch2 sec 2.4 "The Kalman Filter", Insight 2.2 (ch09.md); KAU Ch6 "Kalman Filters" (ch08.md).

### 25. Rolling slope (kg/week) with t-stat
- Shows: rolling 14/28-day OLS slope of weight (kg per week), with significance coloring, to see when loss started/stopped.
- Inputs: daily weight.
- Excel: `SLOPE(window_w, window_dates)*7`; t via `LINEST`; line + area chart.
- Why: replaces eyeballing; identifies plateau onset objectively.
- Effort: S. Source: KAU Ch6 "Using the Linear Regression Slope and Correlations" (ch08.md).

### 26. CUSUM change-point detector on weight/HRV/RHR `[EXT]`
- Shows: cumulative sum of deviations from a target/baseline that crosses a threshold when level shifts; flags the date of a regime change.
- Inputs: daily weight (or Whoop vitals).
- Excel: `S_t = MAX(0, S_{t-1} + (x_t - mu - k))` and a symmetric lower arm; threshold `h` as input; column flags.
- Why: detects sustained shifts quicker than looking at a moving average; supports the regime idea (31).
- Effort: S. Source: technique `[EXT]` (SPC); motivation from CHAN "Regime Switching" (TOC L825, ch00.md) and KAU Ch22 "Identifying Price Shocks" (ch24.md).

### 27. Price-shock days: unusually large single-day moves
- Shows: days where |daily change| exceeds N x average true range (rolling mean abs change); list with kcal/protein of day before (salt/binge flags).
- Inputs: daily weight, kcal.
- Excel: abs diff column, rolling mean, ratio >5 rule (Kaufman's own 5x), filter table.
- Why: lets you exclude shock days from trend fits (or investigate them).
- Effort: S. Source: KAU Ch22 "Analyzing the Price Shocks / Identifying Price Shocks" (ch24.md).

### 28. Effective sample size correction for autocorrelated daily data
- Shows: lag-1 autocorrelation of weight, N_eff = N(1-rho)/(1+rho), and inflated standard errors for every daily-series statistic.
- Inputs: daily series.
- Excel: `CORREL(w[2:n],w[1:n-1])`; adjust SE in the slope stats of idea 25.
- Why: daily weight is strongly autocorrelated; naive t-stats overstate significance. PAL gives the autocorrelation adjustment to the Sharpe SE.
- Effort: S. Source: PAL Ch3 FAQ 3.2 (ch10.md); KAU Ch2 "Autocorrelation" (ch04.md).

### 29. Weekday / calendar patterns (seasonality)
- Shows: average weight change, kcal, protein by weekday; month-of-year view of labs (vitamin D, lipids vs season); percent-of-weeks "up" by weekday.
- Inputs: daily series with dates; panel dates.
- Excel: `AVERAGEIFS` with `WEEKDAY` helper; column chart; seasonal index = group mean / overall mean (method of link relatives/ratio to average).
- Why: weekend effect on weight/calories is typical; seasonal labs may mislead comparisons across panels drawn in different months `[MED-VERIFY]`.
- Effort: S. Source: KAU Ch15 "Weekday, Weekend, and Reversal Patterns" (ch17.md), Ch10 "Seasonality and Calendar Patterns" incl. "Method of Yearly Averages", "Link Relatives" (ch12.md).

### 30. Mean-reversion half-life of weight (set point)
- Shows: AR(1) fit of weekly weight deviations from personal mean; half-life = ln(2)/(-ln(b)) days; "how long a bump persists".
- Inputs: daily/weekly weight.
- Excel: regress `Δw_t` on `w_{t-1}-mean` using `SLOPE`; half-life formula; chart of deviations.
- Why: gives a number for how sticky weight excursions are; short half-life -> water noise, long -> real change.
- Effort: S. Source: CHAN Ch7 "Stationarity and Cointegration" and Example 3.7 mean-reverting model (ch00.md ~L6767); KAU Ch6 ARIMA/mean-reverting indicator (ch08.md).

---

# THEME D - Regimes, interventions, event studies, backtesting

### 31. Regime timeline: interventions as shaded bands over marker charts (TOP)
- Shows: Gantt-like strip of supplement/protocol/diet regimes (start-stop) under a marker or weight chart; vertical bands mark e.g. "Blueprint stack on", "cut phase", "travel".
- Inputs: supplement start/stop dates, diet-phase dates, panel dates, daily series.
- Excel: regime table; for each date a helper column with 0/1 per regime -> stacked area/column on secondary axis with 100% transparency colors; or stacked bar chart for Gantt.
- Why: the cheapest way to let the eye connect interventions to outcomes (and see confounding overlap).
- Effort: M. Source: CHAN "Regime Switching" (TOC L825; ch00.md); PAL Ch13 dynamic risk allocation concept (ch20.md).

### 32. Event study: marker change around supplement start (TOP)
- Shows: for each supplement with a recorded start date: marker values at the last pre-start panel vs first post-start panel; change, change in z, vs the marker's typical panel-to-panel change (null distribution from all other intervals), and a verdict "outside normal movement?".
- Inputs: supplement start dates + dose; panel dates/values.
- Excel: per supplement x marker long table (Rust-generated pairs, but values are formulas); null SD = STDEV of all consecutive-panel diffs; z_event = change/SD; flag |z|>2; heat grid supplement x marker.
- Why: converts the 87-supplement list into hypotheses with a check against noise; marker-by-marker "abnormal change". Combine with idea 20 to net out reversion, and with idea 36 for multiple-comparison control.
- Effort: L. Source: KAU Ch2 "2-Sample t-Test" (ch04.md); PAL Ch14 attribution with errors (ch21.md); event-study format `[EXT]`.

### 33. Overlap detection: which interventions changed in the same window (confounding map)
- Shows: matrix of start dates between panels: number of other changes (supplements, diet, training, meds) co-occurring with each start; a confounding score.
- Inputs: start/stop dates for all supplements/protocols.
- Excel: `COUNTIFS` on start-date between panel dates; conditional format.
- Why: if 15 things changed between two panels, attribution is meaningless; this makes that visible instead of implicit.
- Effort: S. Source: PAL Ch14 "Two Paradoxes" / attribution errors (ch21.md); PAL Ch4 Frisch-Waugh-Lovell (ch11.md).

### 34. Control-for-weight adjustment (FWL residualization)
- Shows: marker values adjusted for weight change (residual of marker ~ weight at panel time) before judging a supplement.
- Inputs: panel values + weight at panel date (from daily series).
- Excel: `LINEST` or `SLOPE/INTERCEPT` per marker, residual column, then use residual in the event study.
- Why: weight loss itself moves many markers; subtracting that gives the "idiosyncratic" part, exactly like removing market beta.
- Effort: M. Source: PAL Ch4 sec 4.7.3 Frisch-Waugh-Lovell, "Stagewise linear regression" (ch11.md); PAL Ch12 Hedging (ch19.md).

### 35. Walk-forward forecast scorecard
- Shows: for each marker, predict panel k from panels < k via trend (idea 16), EWMA (idea 3), and "last value"; score by absolute error; which model won per marker.
- Inputs: panel values + dates.
- Excel: expanding-window SLOPE/INTERCEPT; error columns; summary with AVERAGE of abs errors; winner via INDEX/MATCH of MIN.
- Why: tells David whether trend lines have ever predicted anything; same discipline as out-of-sample testing.
- Effort: M. Source: PAL Ch8 sec 8.2.1 "Cross-Validation and Walk-Forward" (ch15.md); KAU Ch21 "Step-Forward Testing" (ch23.md).

### 36. Multiple-comparisons / data-snooping guard on every "discovery" view
- Shows: banner on the correlation matrix and event-study grid: number of hypotheses tested (e.g. 87 x 56 = 4,872), expected false positives at alpha = 5%, Bonferroni-adjusted threshold; cells passing it highlighted differently.
- Inputs: counts of supplements/markers.
- Excel: `=0.05*COUNT(cells)`; `=0.05/COUNT` ; formula-based conditional format layer.
- Why: with thousands of pairings some will "work" by chance; the library flags this as the key failure of backtests.
- Effort: S. Source: CHAN "Does the Strategy Suffer from Data-Snooping Bias?" (ch00.md ~L3551, L6013); PAL Ch8 "Rademacher Anti-Serum" (ch15.md).

### 37. Pre-registration sheet for each new intervention
- Shows: form row: intervention, start date, hypothesis (marker, direction, minimum meaningful size in RCV units), review date, theory/mechanism, confidence %; after the next panel it auto-fills outcome and a hit/miss.
- Inputs: new rows entered by David; panels thereafter.
- Excel: Table with data validation lists (marker names, up/down), formula for outcome pulling the first post-review panel, `IF` for hit.
- Why: "have a theory and pre-register before the backtest", the only reliable defense against storytelling after the fact.
- Effort: M. Source: PAL Ch8 sec 8.1.2 "Have a theory (if you can)" (ch15.md); DUKE Ch3 "Bet to Learn" (ch09.md in `Thinking in Bets by Annie Duke`).

### 38. Decision journal separating outcome from process ("resulting" detector)
- Shows: per decision (start/stop supplement, change diet): evidence quality, expected effect, probability, what happened, then two separate grades: decision quality vs outcome luck.
- Inputs: journal entries.
- Excel: Table + data validation 1-5 grades; 2x2 matrix counts via COUNTIFS (good decision/good outcome, etc.).
- Why: Duke's central point: judging decisions by results is the main error.
- Effort: S. Source: DUKE Ch1 "Life Is Poker, Not Chess", Ch2 "Wanna Bet?" (ch07-ch08.md).

### 39. Backtest-hygiene checklist sheet (look-ahead / survivorship for health data)
- Shows: checklist: did the protocol start before the lab? is the panel fasting-matched? did I drop supplements I quit (survivorship)? includes failed supplements in cost stats?
- Inputs: manual.
- Excel: check boxes via data validation Y/N; summary % complete.
- Why: survivorship bias (only counting supplements that "worked") is exactly the failure the library enumerates.
- Effort: S. Source: PAL Ch8 sec 8.1.2 "Survivorship bias", "Data leakage", "Missingness" (ch15.md).

### 40. Regime-conditioned statistics
- Shows: mean/SD of weight change, kcal, protein split by regime (cut, maintain, travel, supplement on/off).
- Inputs: daily series + regime table.
- Excel: `AVERAGEIFS` with regime helper column; clustered bars; Welch t via formulas for two chosen regimes (`T.TEST`).
- Why: quantifies effects of phases without needing a model.
- Effort: S. Source: KAU Ch2 "2-Sample t-Test" (ch04.md); CHAN regime switching (ch00.md TOC L825).

### 41. Trend filter state per marker (10-panel-SMA-style, Faber)
- Shows: simple trend state: latest value vs mean of last k panels; state = "improving/worsening/flat" direction-aware, shown as arrow icon.
- Inputs: panel values + direction flags.
- Excel: `=SIGN(v - AVERAGE(last_k))*dir`; icon set arrows.
- Why: the simplest rule-based filter, as in Faber's 10-month SMA tactical allocation: a rule robust because it is simple.
- Effort: S. Source: FABER `faber-taa-2007.md` ("Buy when monthly price > 10-month SMA"); KAU Ch8 trend systems (ch10.md).

### 42. Dual momentum ranking of markers (absolute + relative)
- Shows: rank markers by improvement (relative momentum) and require improvement beyond noise (absolute momentum vs RCV); a leaderboard of "most improved" and "most deteriorated".
- Inputs: panel values.
- Excel: z-change column; rank with `RANK.EQ`; filter column by RCV; sorted dynamic array (`SORTBY`).
- Why: separates "relatively better than my other markers" from "actually moved".
- Effort: S. Source: `Algotrading/Papers/relative-strength-strategies-antonacci2012/`; `moskowitz_ooi_pedersen_2012_time_series_momentum.md`.

### 43. Persistence test: do improvements continue or reverse?
- Shows: lag-1 autocorrelation of panel-to-panel changes per marker and pooled; label "trending" (>0) vs "mean-reverting" (<0).
- Inputs: panel values.
- Excel: diffs, `CORREL` of lagged diffs; pooled with stacked range.
- Why: tells David whether trend-following extrapolation makes sense for a given marker; the same split as momentum vs mean-reversion strategies.
- Effort: S. Source: `Public Markets/Papers/persistence-mutual-fund-carhart1997/`; `Algotrading/Papers/time-series-momentum-moskowitz-ooi-pedersen/`; CHAN "Mean-Reverting versus Momentum Strategies" (ch00.md TOC ~L821).

---

# THEME E - Supplements as a portfolio

### 44. Holdings table = position book
- Shows: one Excel Table row per supplement: name, dose, unit, frequency, time of day, price, $/yr, source protocol, research stance, start date, status; slicers via autofilter; weight column = % of total annual spend.
- Inputs: the 87-supplement data.
- Excel: Table with calculated columns, `=[@[$/yr]]/SUM([$/yr])`, data validation for stance/protocol/time of day, data bars on weight.
- Why: foundation for all portfolio views below; the stack is literally a book of positions with a cost basis.
- Effort: S. Source: KAU Ch24 "Types of Portfolio Models" (ch26.md).

### 45. Cost attribution by source protocol (waterfall/treemap-ish) (TOP)
- Shows: total $/yr split by protocol (Blueprint, own research, doctor, other) and by research stance (strong / mixed / weak / none); waterfall of "total cost -> by protocol -> by item"; "spend with no evidence" call-out.
- Inputs: $/yr, source protocol, research stance.
- Excel: `SUMIFS`; stacked column "waterfall" (invisible base series) and doughnut/bar; Pareto bar (sorted) with cumulative line.
- Why: performance attribution applied to cost; makes visible how much money rides on weak evidence.
- Effort: M. Source: PAL Ch14 "Ex-Post Performance Attribution" (ch21.md) and Ch4 sec 4.5.1 "Performance Attribution" (ch11.md).

### 46. Pareto / concentration: top-N spend, HHI, Lorenz
- Shows: cumulative spend curve (top 10 items = x% of spend), Herfindahl index, Gini; chart.
- Inputs: $/yr.
- Excel: sorted via `SORTBY`; `SUMPRODUCT(share^2)`; area chart.
- Why: tells which few items drive cost; same analytics as position concentration.
- Effort: S. Source: KAU Ch24 "Too Much Diversification" (ch26.md); PAL Ch9 Insight 9.3 (ch16.md).

### 47. Mechanism-exposure ("factor loadings") matrix (TOP)
- Shows: matrix supplement x mechanism/target (e.g., lipid lowering, glycemia, sleep, inflammation, cognition, mitochondria, hormones) with 0/1 or 0-3 loadings; column sums = total exposure per mechanism, cost per mechanism, count of redundant items; radar chart of "coverage".
- Inputs: manually tagged loadings per supplement (David or an LLM-fill, flagged as unverified).
- Excel: `SUMPRODUCT(loadings, dose_flag)`, `MMULT` for mechanism x cost, conditional-format heat map; stacked bar.
- Why: the factor-model view: positions are bundles of exposures; redundancy and gaps appear. `[MED-VERIFY]` tags must be sourced per supplement.
- Effort: L (data labelling) / M (Excel). Source: PAL Ch4 sec 4.1-4.2 "Factor Models", "Superposition of Effects" (ch11.md); PAL Ch4.5.2 risk decomposition.

### 48. Redundancy / overlap by ingredient across products (45 products x supplements)
- Shows: ingredient x product matrix, duplicate-active flags, summed daily dose per ingredient vs upper limit if provided.
- Inputs: product ingredient list/dose (nutrition items and 45 products), per-ingredient daily limits if sourced.
- Excel: `SUMIFS` across a long ingredient table; `IF(total>limit,"over")`.
- Why: stacked sources double-count the same active (typical when combining a multivitamin plus singles). `[MED-VERIFY]` limits need an authority.
- Effort: M. Source: PAL Ch4 "Superposition of Effects" (ch11.md); KAU Ch24 diversification vs overlap (ch26.md).

### 49. Benefit-per-dollar ("Sharpe per $") score (TOP)
- Shows: for each supplement: (evidence-weighted expected effect on David's markers) / ($/yr); ranked bar; quadrant scatter (cost x evidence) with bubble = pill burden.
- Inputs: $/yr, research stance (mapped to numeric evidence weight), mechanism loadings, relevant marker gap from optimal (idea 4).
- Excel: `score = evidence_weight * relevance / cost`; relevance = SUMPRODUCT(loading, marker_gap_z); scatter/bubble chart; data labels.
- Why: forces explicit assumptions into named cells; same as ranking strategies by information ratio. Medical effect sizes must be sourced `[MED-VERIFY]`; the workbook only holds weights David sets.
- Effort: M. Source: PAL Ch9 sec 9.5 "Drivers of Information Ratio" and Ch14.4 fundamental law (ch16.md, ch21.md); KAU Ch23 ratio list (ch25.md).

### 50. Evidence-weight shrinkage ("fractional Kelly" for conviction)
- Shows: a conviction multiplier that scales the claimed effect down by uncertainty (e.g., stance weak -> 25% of claimed), feeding idea 49; sensitivity slider.
- Inputs: stance, optional reported effect size.
- Excel: named cell `Kelly_Fraction`; `adj = claimed*fraction*stance_factor`; data validation list of stances.
- Why: when edge is uncertain the optimal bet is smaller; the book shows that "all reasonable investors use fractional Kelly without knowing."
- Effort: S. Source: PAL Ch13 sec 13.3 "Fractional Kelly Strategy", Example 13.6 uncertain expected return, Insight 13.2 (ch20.md); `Algotrading/Papers/kelly-criterion-blackjack-thorp/`.

### 51. Drop-one analysis (marginal contribution to cost and coverage)
- Shows: for each supplement: money saved if removed; mechanism coverage lost (exposure remaining without it); items whose removal loses zero unique coverage flagged "redundant".
- Inputs: mechanism matrix, $/yr.
- Excel: coverage per mechanism excluding item `=SUM(col)-loading`; flag where any mechanism coverage hits 0; saving = $/yr.
- Why: marginal-contribution-to-risk analog; hunts for free savings.
- Effort: M. Source: PAL Ch5 "Evaluating risk/decomposition" and Ch4 sec 4.5.2 (ch11.md, ch12.md).

### 52. Efficient frontier: cumulative cost vs cumulative benefit
- Shows: supplements sorted by benefit/$; cumulative cost x cumulative coverage curve; knee marks where marginal benefit per $ collapses; vertical line at current spend, slider "budget".
- Inputs: idea 49 scores, $/yr.
- Excel: `SORTBY`, running sum, scatter with line; budget cell with `XLOOKUP` for coverage at budget.
- Why: the "efficient frontier" of the stack; answers "what's my best stack for $X".
- Effort: M. Source: KAU Ch23 "The Efficient Frontier" (ch25.md), Ch24 "Finding Optimal Portfolio Allocation using Excel's Solver" (ch26.md).

### 53. Total expense ratio and fee-drag view
- Shows: total annual spend (supplements + products + tests), per-month and per-day cost; "TER" = spend as % of stated budget/income; trend by year if price history exists.
- Inputs: $/yr, products cost, panel cost if known.
- Excel: SUMs; KPI cards via big-font cells; line of cost per day.
- Why: fee drag compounds, and the Faber implementation chapter treats fees explicitly.
- Effort: S. Source: FABER `global-asset-allocation-faber/ch16.md` ("Implementation: ETFs, Fees, Taxes, Advisors").

### 54. Unit economics: cost per unit of active and per day of coverage
- Shows: $/mg, $/serving, $/day; best-value alternative among the 45 products for the same ingredient; savings if switched.
- Inputs: product price, size, servings, dose.
- Excel: formulas; `MINIFS` by ingredient; data bars.
- Why: pure-arbitrage view of the shelf (cheapest source of the same exposure).
- Effort: S. Source: KAU Ch13 "Spreads and Arbitrage" (ch15.md) (analogy only).

### 55. Pill-burden / "turnover" and schedule heatmap
- Shows: matrix time-of-day x supplement; pills per slot; count of changes per quarter (stack turnover); highlight crowded slots.
- Inputs: frequency, time of day, start/stop dates.
- Excel: `COUNTIFS` by slot; changes count by `COUNTIFS` on start/stop; heat map.
- Why: complexity is a cost that hits adherence; turnover is a cost in portfolio terms.
- Effort: S. Source: PAL Ch5 sec 5.3.1 "Model Turnover" (ch12.md), Ch3.4 "Capacity" (ch10.md).

### 56. Transaction-cost ledger of starting/stopping
- Shows: each start/stop has a $ cost (unused inventory) and a "washout" (cannot read effect until X weeks); cumulative waste.
- Inputs: start/stop dates, price, stock on hand.
- Excel: formulas; running total line.
- Why: churn cost is invisible otherwise.
- Effort: S. Source: PAL Ch11 "Market-Impact-Aware Portfolio Management" (ch18.md).

### 57. Renewal and runway calendar
- Shows: when each product runs out (stock / daily use), reorder date, monthly cash outflow forecast chart.
- Inputs: product size, daily use, last purchase date, price.
- Excel: `=purchase_date + servings/daily_use`; `EOMONTH` buckets; stacked column by month; conditional format on days-to-run-out.
- Why: cash-flow forecast like a fund's capital-call schedule.
- Effort: S. Source: `Investing/Accounting/Books/Private Equity Accounting - Stefanova/` (cash-flow schedules; not read in depth).

---

# THEME F - Correlation, factors, dimensionality

### 58. Marker correlation matrix (with significance masking)
- Shows: Spearman/Pearson matrix across panels (56 x 56) with color scale; cells where |r| fails a significance threshold given n=7 greyed; click-through to scatter via selector.
- Inputs: panel values.
- Excel: `CORREL` generated cell-by-cell; Spearman via rank helper block; mask using `ABS(r) < r_crit` where `r_crit = t/SQRT(n-2+t^2)`.
- Why: with 7 points |r| must exceed about 0.75 to be nominally significant, and idea 36 must apply. Showing the mask prevents false stories.
- Effort: M. Source: KAU Ch6 "Linear Correlation", "Spearman's Correlation", "Confidence Bands" (ch08.md).

### 59. Daily-data correlation and lead-lag (calories/protein -> weight)
- Shows: cross-correlation of kcal(t-k) with weight change(t) for k = 0..14; bar chart by lag; best lag.
- Inputs: daily kcal, protein, weight.
- Excel: `CORREL(OFFSET-free windows)` generated per lag in Rust; bar chart.
- Why: finds the real lag between intake and scale; a standard lead-lag test.
- Effort: S. Source: KAU Ch2 and Ch6 "Autocorrelation" (ch04.md, ch08.md).

### 60. Dose-response beta (hedge-ratio style)
- Shows: for a marker with a time-varying dose of an intervention across panels: slope of marker on dose (beta), R^2, residual SD.
- Inputs: dose history of the supplement at each panel date; panel values.
- Excel: `SLOPE, RSQ, STEYX` on the 7 pairs; chart with trendline.
- Why: the closest thing to dose-response evidence in the data; hedge-ratio regression.
- Effort: M. Source: PAL Ch12 "Hedging" Procedure 12.1 simple single-asset hedging (ch19.md).

### 61. Beta-to-weight and R^2-by-weight for every marker
- Shows: per marker: sensitivity to weight (marker change per kg) and share of variance explained by weight; ranks "weight-sensitive" markers.
- Inputs: panel values + weight at panel dates.
- Excel: `SLOPE`, `RSQ`.
- Why: tells which labs mostly follow body weight (no need for supplement explanations).
- Effort: S. Source: PAL Ch4 "Alpha spanned and alpha orthogonal" (ch11.md); `frazzini_pedersen_2014_betting_against_beta.md` (beta concept).

### 62. PCA of the marker panel ("health factors") and scree plot
- Shows: eigenvalues of the z-scored marker covariance across panels, scree chart, "effective number of independent markers", loadings of first components.
- Inputs: z-scored panels.
- Excel: precompute in Rust (eigendecomposition) and write static values with a "computed offline" label; or MMULT power iteration formulas (small). Note: with n=7 panels, at most 6 non-zero components: state that.
- Why: shows how many independent dimensions the 56 markers really carry; breadth drives information ratio.
- Effort: L. Source: PAL Ch7 "Statistical Factor Models", sec 7.2.5 "Choosing the Number of Factors" (ch14.md); PAL Ch9.5 (ch16.md).

### 63. Effective-breadth score = (sum lambda)^2 / sum lambda^2
- Shows: single number of effective independent markers, plus the same for supplements' mechanism matrix.
- Inputs: correlation matrix / eigenvalues.
- Excel: from idea 62 outputs, `=SUM(l)^2/SUMSQ(l)`.
- Why: tests whether the dashboard's 56 markers are really 8 stories.
- Effort: S (given 62). Source: PAL Ch9 Insight 9.3 "Asset correlations, dispersion, and limits to performance" (ch16.md).

### 64. System-level burden decomposition
- Shows: "variance/burden" of out-of-optimal distance by system (lipids, glycemia, thyroid, CBC, CMP) over panels; stacked column showing where burden sits; contributions sum to total.
- Inputs: distance scores from idea 4 + system tags.
- Excel: `SUMIFS` by system; stacked columns across panels.
- Why: risk decomposition applied to health: "what part of my gap is lipids."
- Effort: S. Source: PAL Ch4 sec 4.5.2 "Risk Management: Forecast and Decomposition" (ch11.md).

### 65. Composite "quality" index with risk-parity (inverse-vol) weights
- Shows: composite from z-scored markers, weighted by user choices or inverse volatility so noisy markers do not dominate; track across panels; compare equal-weight vs inverse-vol.
- Inputs: z-scores and their SDs.
- Excel: `weights = (1/sd)/SUM(1/sd)`; `SUMPRODUCT`; line chart of both.
- Why: composite signals are standard (QMJ); inverse-vol weighting is the risk parity logic.
- Effort: M. Source: `asness_frazzini_pedersen_2019_quality_minus_junk.md`; FABER `global-asset-allocation-faber/ch08.md` (risk parity); KAU Ch24 "Equal-Risk Weighting" (ch26.md).

---

# THEME G - Risk, extremes, action thresholds

### 66. Tiered alert levels ("stop-loss" bands)
- Shows: per marker: watch (leaves optimal), action (leaves reference), critical (user-defined hard line); cell color escalates; list of currently triggered alerts sorted by tier.
- Inputs: optimal and reference bounds plus optional critical limits `[MED-VERIFY]`.
- Excel: formula status 0-3; conditional format; `FILTER` for the alert list.
- Why: stop-loss levels in risk control, defined in advance, remove improvisation.
- Effort: S. Source: KAU Ch23 "Individual Trade Risk / Stop-Losses" (ch25.md).

### 67. "Value at risk" for the next panel
- Shows: 95% range of plausible next value per marker from historical changes (or parametric mean + z*sd), and probability of leaving the reference range.
- Inputs: panel values.
- Excel: `PERCENTILE.INC` of diffs; or `NORM.DIST`; probability = `1-NORM.DIST(ref_hi, last+drift, sd, TRUE)`; fan chart.
- Why: an actionable "what to expect next panel", with honest width.
- Effort: M. Source: KAU Ch23 "Value at Risk", "A Simpler Way to Calculate Value at Risk" (ch25.md); `Modelling Extremal Events - Mikosch/` (tail concepts, not read in depth).

### 68. Monte-Carlo-style fan chart for weight
- Shows: projected weight over next 60-90 days with 50/80/95% bands from bootstrapped daily changes, under chosen drift.
- Inputs: daily weight; target date.
- Excel: analytic fan (mean + z*sd*sqrt(t)) in formulas; or precomputed percentile paths from Rust bootstrap, written as values (labelled).
- Why: honest projection instead of one line.
- Effort: M. Source: KAU Ch21 "Monte Carlo Sampling" (ch23.md); PAL Ch2 (ch09.md).

### 69. Cumulative-exposure area under curve (e.g., ApoB-years)
- Shows: trapezoid-integrated area under the curve of a marker across years (cumulative exposure), versus an optimal-level baseline; running total chart.
- Inputs: panel values + dates (2018-2025).
- Excel: `=(v1+v2)/2*(d2-d1)/365.25` running sum; area chart.
- Why: treats exposure as an integral rather than a snapshot. Whether cumulative exposure is clinically meaningful for a given marker needs a source `[MED-VERIFY]`.
- Effort: S. Source: KAU Ch23 equity-curve based measures (ch25.md); concept of integral under equity curve `[EXT]`.

### 70. Retest cadence by volatility ("vol-managed testing")
- Shows: recommended re-test interval per marker from its volatility and how big a change would matter; sorted by "retest first."
- Inputs: marker SD of changes, minimal meaningful change (RCV).
- Excel: `n_needed = ((1.96)*SD/delta)^2` as a power calculation; `next_due = last_date + interval`; conditional format on days overdue.
- Why: scale effort to uncertainty; volatility-managed portfolios apply the same inverse-vol scaling.
- Effort: M. Source: `Algotrading/Papers/volatility-managed-portfolios-moreira-muir/`; KAU Ch23 "Position Sizing" (ch25.md).

### 71. Value-of-information ranking for the next blood draw
- Shows: for each marker: (decision relevance x current uncertainty) / test cost -> which to include in the next panel.
- Inputs: marker cost (if known), uncertainty (SD/width of prediction interval), relevance weight.
- Excel: formula ranking; bar chart.
- Why: expected-utility reasoning says buy the information with the best expected payoff per cost.
- Effort: S. Source: GOOD (I.J. Good, expected utility / value of information; `Rational Decisions by I. J. Good.md`).

---

# THEME H - Forecasting, calibration, evidence

### 72. Prediction log with Brier score and calibration table (TOP)
- Shows: David logs probability forecasts ("ApoB < X at next panel": 70%); after the outcome the sheet computes Brier score, calibration buckets (predicted vs realized), and a trend of skill.
- Inputs: forecast entries.
- Excel: `(p-outcome)^2`, bucket via `COUNTIFS`, scatter of calibration with 45-degree line; running mean.
- Why: turns health beliefs into checkable bets; Duke's "bet to learn," and FRI scoring.
- Effort: M. Source: DUKE Ch2-3 (ch08-ch09.md); FRI `Himmelstein-2024-Forecasting-Proficiency-Test.pdf`, `Mellers-2017-How-Generalizable-Is-Good-Judgment.pdf`.

### 73. Quantile forecasts for next panel with pinball loss
- Shows: David enters 10/50/90% quantiles for chosen markers; scored later with pinball loss and coverage rate (did truth land inside 80% interval ~80% of time).
- Inputs: quantile entries; next panel values.
- Excel: formula coverage; pinball loss `=MAX(q*(y-f),(q-1)*(y-f))`.
- Why: the FRI psychometrics paper found quantile elicitation measured skill with lower error than binned probabilities.
- Effort: M. Source: FRI `Zhu-2024-Psychometrics-Probability-Quantile-Forecasts.pdf`.

### 74. Ensemble forecast with extremizing
- Shows: next-panel forecast as weighted average of models (trend, EWMA, last value, David's own guess) plus optional extremizing factor; shows how weights perform historically.
- Inputs: model forecasts (ideas 16, 3), own forecast.
- Excel: `SUMPRODUCT`; extremize `=mean+a*(avg-mean)` with `a>1`.
- Why: the paper argues weighted averages should be pushed away from the marginal mean; only worthwhile if models carry partly independent information.
- Effort: M. Source: FRI `Satopaa-2015-Combining-Extremizing-Real-Valued-Forecasts.pdf`.

### 75. Evidence ledger with log-odds ("weight of evidence") per supplement
- Shows: each supplement has evidence items (RCT, meta-analysis, mechanistic, anecdote) with a log-likelihood-ratio weight; sum -> posterior probability that it helps given a prior; ladder chart.
- Inputs: manual evidence entries with sourced citations; research stance.
- Excel: `LN` weights, `SUMIFS`, posterior `=1/(1+EXP(-(prior_logodds+sum)))`.
- Why: I.J. Good's weight of evidence makes updating additive and auditable. Weights are David's judgment and must be labelled; no medical claims are encoded by the tool.
- Effort: M. Source: GOOD `Rational Decisions by I. J. Good.md` (weight of evidence/Bayes factor discussion); `Data/Prediction Markets/Papers/hanson-lmsr-original-ssrn.md` (scoring rules).

### 76. Bayesian update of "is it working" after each panel
- Shows: posterior probability that a supplement shifted the marker (vs null) after each post-start panel, using idea 32's z as a likelihood ratio.
- Inputs: event-study z, prior from idea 75.
- Excel: normal-likelihood ratio formula with `NORM.DIST`.
- Why: shows belief changing gradually instead of binary verdicts.
- Effort: M. Source: GOOD (weight of evidence); DUKE Ch3 (ch09.md).

### 77. Model-vs-reality scorecard for genome predictions
- Shows: for variants with a stated predicted effect on a marker (Promethease annotation), compare predicted direction/risk vs observed percentile in labs; hit-rate of "my genome predicts my labs".
- Inputs: genome annotation table (rsID, gene, predicted effect, magnitude, repute), panel data.
- Excel: INDEX/MATCH link table (variant -> marker), concordance formula, hit-rate KPI.
- Why: evaluating a risk model against realized outcomes is the whole of risk-model validation. Variant-effect claims need sources `[MED-VERIFY]`.
- Effort: L. Source: PAL Ch5 "Evaluating Risk" (predicted vs realized; ch12.md).

---

# THEME I - Wearables (Whoop), training load, oscillators

### 78. Acute:chronic ratio of HRV/strain via two EWMAs
- Shows: 7-day vs 28-day EWMA of HRV/RHR/strain/sleep; ratio and band; flag deviation.
- Inputs: Whoop daily series (future).
- Excel: two EWMA columns; ratio; conditional format.
- Why: fast vs slow baselines = MACD logic; a robust "am I deviating from baseline" signal.
- Effort: S. Source: KAU Ch9 "Moving Average Convergence/Divergence (MACD)" (ch11.md); PAL Ch2 EWMA (ch09.md).

### 79. Personal-baseline bands for Whoop metrics (z over trailing 30 days)
- Shows: z-score of today vs trailing window, color-coded, with streak counters.
- Inputs: Whoop series.
- Excel: rolling AVERAGE/STDEV windows; icon set.
- Why: same as idea 23 but for recovery vitals.
- Effort: S. Source: KAU Ch7 (ch09.md).

### 80. Stochastic-style "where in my range" oscillator
- Shows: (x - min_n)/(max_n - min_n) in 0-100 for weight/HRV; overbought/oversold shading.
- Inputs: daily series.
- Excel: `MINIFS/MAXIFS` or fixed windows; line + threshold lines.
- Why: simple range-position feature to compare metrics on one 0-100 axis.
- Effort: S. Source: KAU Ch9 "Stochastics" (ch11.md).

### 81. Cycle detection (weekly/monthly) via autocorrelation spectrum
- Shows: autocorrelation at lags 1-35 days for weight/HRV, peak lag labelled (e.g., 7 = weekly, ~28 = monthly).
- Inputs: daily series.
- Excel: `CORREL` per lag (generated), bar chart.
- Why: finds periodicity (weekend effect, training cycles) without FFT.
- Effort: S. Source: KAU Ch11 "Cycle Analysis" (ch13.md); Ch2 autocorrelation (ch04.md).

---

# THEME J - Nutrition, macros, routine, vaccines

### 82. Macro "allocation" drift and rebalancing bands
- Shows: target macro split (protein/carb/fat or g/kg) vs actual daily, drift from target, days outside a +/-band, weekly rolling.
- Inputs: daily kcal, protein; macro targets; nutrition items.
- Excel: formulas; conditional format when drift > band; stacked area of actual allocation.
- Why: asset-allocation drift and rebalance bands in direct form.
- Effort: S. Source: `global-asset-allocation-faber/ch16.md` (implementation/rebalancing); KAU Ch24 allocation (ch26.md).

### 83. Adherence streak statistics ("theory of runs")
- Shows: longest run of protein-target days, number of runs, expected run length given hit rate p (=1/(1-p)), with probability of observed streak under random sequence.
- Inputs: daily protein vs target.
- Excel: run-length column; `MAX`, `COUNTIFS`; expected `=1/(1-p)`.
- Why: separates "habit" from luck; Kaufman's chapter formalizes runs.
- Effort: S. Source: KAU Ch22 "Gambling Techniques: The Theory of Runs" (ch24.md).

### 84. Plan vs actual glide path with tracking error
- Shows: planned weight trajectory (target rate) vs actual; tracking error = SD of deviations; information-ratio-like "return vs plan / TE".
- Inputs: daily weight, target date/weight.
- Excel: planned line `=start - rate*days`; deviation column; `STDEV.S`; chart.
- Why: tracking error vs benchmark = standard monitoring tool.
- Effort: S. Source: PAL Ch14 attribution/benchmark framing (ch21.md); KAU Ch23 "Calculating the Information Ratio" (ch25.md).

### 85. Weight "equity curve" and underwater (regain) chart
- Shows: weight vs running minimum (for a loss phase) with underwater regain %, duration of regains.
- Inputs: daily weight.
- Excel: running `MIN`, regain % column, area chart.
- Why: gives objective definition of "relapse" episodes.
- Effort: S. Source: CHAN drawdown (ch00.md ~L3169); KAU Ch23 "Average Maximum Retracement" (ch25.md).

### 86. Food "yield" frontier (protein per kcal vs per $)
- Shows: scatter of nutrition items: x = $/g protein, y = kcal/g protein; frontier of dominating foods; table of efficient picks.
- Inputs: nutrition items with macros and price.
- Excel: formulas; scatter with data labels; flag Pareto-optimal via `COUNTIFS` dominance test.
- Why: Pareto efficiency analogue to risk/return scatter.
- Effort: S. Source: KAU Ch23 "The Efficient Frontier" (ch25.md).

### 87. Morning routine as a trade timeline with slippage
- Shows: step durations (planned vs actual), cumulative time, supplement doses tied to steps; Gantt chart.
- Inputs: morning routine steps with durations.
- Excel: stacked bar Gantt; sum of planned vs actual; conditional format on overrun.
- Why: the Timeline of a Trade chapter is a template for decomposing latency.
- Effort: S. Source: FO ch17 "The Timeline of a Trade" (ch17.md).

### 88. Vaccine/booster maturity ladder
- Shows: ladder of vaccines by due date, days to due, overdue highlight, next-12-month schedule.
- Inputs: vaccines with dates and intervals (intervals need a source `[MED-VERIFY]`).
- Excel: `=EDATE(last, interval)`; `=due-TODAY()` icon set; timeline bar.
- Why: bond-ladder logic for expirations; ensures no cliff.
- Effort: S. Source: concept only; `Investing/Bonds/Books/` not read.

---

# THEME K - Data integrity and presentation

### 89. `Checks` sheet: assertion dashboard
- Shows: red/green checks: no missing dates in daily series, duplicates, units consistent, ref range present for each marker, panel dates sorted, formulas error-free (`ISERROR` count), totals reconcile (sum by category = grand total).
- Inputs: all data.
- Excel: `COUNTBLANK`, `SUMPRODUCT(--ISERROR(range))`, reconciliation formulas; one KPI "ALL CHECKS PASS".
- Why: any quant shop reconciles before reading numbers; avoids trusting a broken formula.
- Effort: S. Source: KAU Ch22 "Data Accuracy / Clean Data / Final Steps Before Launch" (ch24.md); PAL Ch8.1.2 "Avoidable mistakes" (ch15.md).

### 90. Parameter sheet (single typed defaults object)
- Shows: one `Params` sheet with all thresholds (half-life, window, k, alpha, Kelly fraction, evidence weights) as named cells; every formula references names.
- Inputs: defaults.
- Excel: named ranges via `define_name`; data validation min/max.
- Why: makes sensitivity analysis one-cell edits and avoids magic numbers.
- Effort: S. Source: PAL Ch8.1.2 "Define the backtesting protocol beforehand" (ch15.md).

### 91. Parameter sensitivity heat grid (window x k, alpha x window)
- Shows: grid of an outcome (e.g., breach count for Bollinger window x k; smoothing error for alpha) to choose robust defaults; plateaus preferred to sharp peaks.
- Inputs: daily series.
- Excel: explicit formula grid (no Data Tables); 3-color scale.
- Why: "look for continuity in 2-parameter tests" instead of picking the optimum.
- Effort: M. Source: KAU Ch21 "Visualizing and Interpreting Test Results", "Heat Maps", "Seeing Continuity in 2-Parameter Tests" (ch23.md).

### 92. Executive dashboard KPI strip (cards + 6 sparklines)
- Shows: top row: health index (idea 4), % markers in optimal, # alerts, spend/day, weight trend kg/week, adherence streak; below: sparklines for each.
- Inputs: outputs of earlier ideas.
- Excel: large-font merged cells, sparklines, icon sets, one chart.
- Why: the front page a PM reads first: NAV, risk, exposure, flows.
- Effort: M. Source: FO Ch13 (ch13.md); KAU Ch23 (ch25.md).

### 93. Genome exposure report ("risk factor exposures")
- Shows: table variant x gene x system x effect/magnitude/repute; counts by system; link to relevant markers (idea 77); filter dropdown by system and confidence.
- Inputs: 23andMe + WGS variants, Promethease.
- Excel: Table with autofilter, data validation for repute, `COUNTIFS` summary, bar chart.
- Why: a factor-exposure report is the portfolio-level view of many small bets. All clinical interpretation must come from sourced annotations `[MED-VERIFY]`.
- Effort: M. Source: PAL Ch4 "Factor Models" exposures (ch11.md), Ch6 "A Tour of Factors" (ch13.md).

### 94. "What changed since last panel" diff sheet
- Shows: top movers up/down in z and in RCV units; new alerts; supplements started/stopped in the interval.
- Inputs: last two panels + supplement dates.
- Excel: `SORTBY`, `TAKE` (or LARGE/SMALL) dynamic arrays; text via `TEXTJOIN`.
- Why: the daily "what moved" email of a PM desk.
- Effort: M. Source: PAL Ch14 "Ex-Post Performance Attribution" (ch21.md).

### 95. Scenario / stress sheet
- Shows: user adds +/- X% shock to a marker (or to weight) and sees which derived ratios and alerts flip.
- Inputs: current values, shock cells.
- Excel: shock input cells, recomputed derived metrics, before/after table.
- Why: stress tests probe thresholds.
- Effort: S. Source: KAU Ch22 "Extreme Events" (ch24.md).

---

# TOP-15 SHORTLIST (highest value per effort, honest about small N)
1. Idea 9 - Selector-driven marker tear sheet (one layout, 56 markers)
2. Idea 4 - Distance-to-optimal score and composite health index
3. Idea 1 - Personal-baseline z-score heat grid
4. Idea 19 - Reference change value / "is this change real?" flag
5. Idea 20 - Regression-to-the-mean warning
6. Idea 32 - Event study: marker change around supplement start
7. Idea 31 - Regime timeline overlay (interventions over charts)
8. Idea 45 - Cost attribution by protocol and research stance
9. Idea 47 - Mechanism-exposure ("factor loadings") matrix
10. Idea 49 - Benefit-per-dollar score (with Idea 50 shrinkage)
11. Idea 13/14 - Drawdown-from-best and Ulcer Index for markers
12. Idea 21-23 - Weight signal stack: efficiency ratio, adaptive trend, Bollinger bands
13. Idea 37 - Pre-registration sheet (with 72 Brier log as follow-on)
14. Idea 36 - Multiple-comparison guard on correlations and event study
15. Idea 89 - Checks sheet

Cheap runners-up: 12 (time-in-range), 16-17 (trend with t-stat), 44 (position book), 66 (alert tiers), 70 (retest cadence), 52 (efficient frontier), 92 (KPI strip).

# Books/papers I actually read from
Depth key: [TOC+sections] = read table of contents and heading outline plus selected paragraphs; [abstract] = abstract/first page only; [TOC] = outline only.
- The Elements of Quantitative Investing - Paleologo [TOC+sections: Ch2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14 outlines; passages from Ch2.3 (EWMA), Ch3 (Sharpe, CI), Ch8.1 (research hygiene), Ch14.2 (attribution paradoxes)]
- Trading Systems and Methods - Kaufman [TOC+sections: Ch2, 6, 7, 9, 10, 11, 15, 17, 21, 22, 23, 24; passages on Ulcer Index, price shocks, KAMA]
- Quantitative Trading - Chan [TOC; keyword hits for drawdown, Sharpe, data-snooping, regime switching, mean reversion; one passage on Sharpe vs leverage]
- Global Asset Allocation - Faber [TOC; keyword hit on periodic table of returns]; A Quantitative Approach to Tactical Asset Allocation - Faber 2007 [keyword hits: 10-month SMA, drawdown]
- Against the Gods - Bernstein [TOC; Ch12 Galton/regression-to-the-mean passages]
- Thinking in Bets - Duke [TOC only; relied on chapter titles; one grep]
- The Front Office - Costello [TOC; ch13 Facts and Evidence and ch15 Efficiency and Strategy Design opening paragraphs]
- Papers (abstracts only): Moskowitz-Ooi-Pedersen time-series momentum, Moreira-Muir volatility-managed portfolios, Daniel-Moskowitz momentum crashes, Frazzini-Pedersen betting against beta, Asness-Frazzini-Pedersen quality minus junk, Gupta-Kelly factor momentum; Carhart 1997 persistence (existence only); Thorp Kelly (existence); Antonacci relative strength (existence)
- Forecasting (first two pages): Satopaa-Ungar extremizing 2015, Zhu et al. 2024 psychometrics of probability and quantile forecasts, Himmelstein et al. 2024 Forecasting Proficiency Test, Mellers et al. 2017 generalizability of good judgment
- I.J. Good - Rational Decisions [keyword hits only: expected utility]
- Not read / only listed: Hull, Natenberg, Bennett, Haug, Lewis, Shreve, Baxter, Tankov, Mikosch (TOC only, no health mapping), Damodaran, Graham, Private Equity Accounting - Stefanova, Hanson LMSR, Algorithmic Game Theory, Prediction-market and real-estate folders. `Data/Books` is empty; `Data/Statistics` contains only the Good paper; `Investing/Trading/Books` and `Investing/Bonds/Books` were not opened.

# Gaps worth a follow-up (not covered by this library)
- Clinical biostatistics (reference change value, biological variation, index of individuality, intraindividual CV) - needs external sources; idea 19 and 1 depend on them.
- Pharmacology/nutrient upper limits and interactions (ideas 48, 66, 88).
- Any proper time-series package for change-point detection (idea 26 is `[EXT]`).
