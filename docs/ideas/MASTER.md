# MASTER implementation plan: Health Dashboard workbook (Rust, rust_xlsxwriter)

Merged from quant.md (Q1-Q95), clinical.md (C1.1-C11.6; the file header says 71 but 110 numbered sub-ideas exist) and longevity.md (L-A1..L-G18, 117). 322 raw ideas -> 163 master ideas + 2 absorbed into shared infra (Q90, C1.3). Every raw id is an alias in exactly one or more master rows (verified by script at the end).

## 0. Legend

Status
- BN = BUILDABLE-NOW: every input exists (labs, daily weight, supplements, products, nutrition, routine, workout, vaccines, physicals, genome CLI, evidence refs). Entry-form and calculator sheets (David types inputs, no stored data needed) count as BN.
- PH = PLACEHOLDER: build the full view over an empty input table plus an "awaiting data" banner (infra helper `awaiting()`), so it lights up when data arrives. Missing: hs-CRP, fasting insulin, Lp(a), homocysteine, vitamin D, uric acid, ferritin/iron/TIBC, FT4/FT3, uACR, any Whoop/HRV/sleep/VO2/CGM/DEXA, supplement mechanism/hallmark tags (David or an LLM fills, flagged unverified), daily kcal/protein intake log (see assumption A1).
- NS = NEEDS-SOURCE: a formula or threshold asserted as clinical fact is only EXT/EXT-unverified in the catalogs. Ship the view with the constant in a Params cell labelled UNVERIFIED (orange fill, source column = "pending: <named reference>"). Promote to BN only after someone verifies it against the named reference. Values that are real in the catalogs (library prose with file+line, or the verified Levine 2018 Table 1 weights) ship as cited. User-tunable heuristics (windows, caps, half-lives, weights) are Params with source "heuristic", not NS.

Tags: MED-VERIFY, EXT, EXT-unv (EXT-unverified), DX-RISK (neutral pattern wording only; "pattern consistent with... discuss with clinician"), TOP (on a source shortlist), GATE-DATES (needs supplement start/stop dates in tblSupp; if absent the sheet degrades to PH), GATE-FIELDS (needs product/supplement fields such as price, units per container, ingredient amounts; verify in health-cli schema).

Effort: S under an hour, M half a day, L a day or more.

Source keys are those of the three catalogs (KAU, PAL, CHAN, FABER, FO, BERN, DUKE, FRI, GOOD in quant.md; KAT, DIP, GG, OUT, EV, WHO, CIP in clinical.md; OUT, LIFE, WWS, GREG, GLU, HELMS, ISRA, SCHO, ENDU, DEGR, TW in longevity.md). Line refs live in the originals; implementers open them via the alias id.

## 1. Shared infrastructure (build FIRST, one agent, Wave 0)

Module `src/infra/`; no workstream edits these files. Sheet prefixes: `Data_`, `Calc_`, `Ref_`, `Params`.

1. Data tables (Excel Tables with stable names; long format, written by Rust from CLI JSON):
   - tblLabs (panel_date, panel_id, marker_id, value_raw, unit_raw, value (canonical), lab, method, fasting). Canonical-unit conversion absorbs C1.3 via Ref_Units (marker, unit_raw, factor, canonical_unit).
   - tblMarkers (marker_id, name, system, unit, ref_lo, ref_hi, opt_lo, opt_hi, blueprint_lo, blueprint_hi, dir = hi/lo/band, critical_lo, critical_hi (blank unless sourced), weight 1-5, cvi, cva (blank, unverified)). Covers all 56 present markers plus empty rows for MISSING markers (hs-CRP, insulin, Lp(a), homocysteine, vit D, uric acid, ferritin, FT4, uACR) so PH views and the completeness matrix can reference them.
   - tblPanels (panel_id, date, age_at_draw, lab, fasting, weight_at_panel from the `weight` marker, n_markers).
   - tblDaily (date, weight, kcal, protein_g, other; kcal/protein columns may be empty, assumption A1). tblWearable (date, hrv, rhr, recovery, strain, sleep_*) EMPTY.
   - tblSupp (87), tblProducts (45), tblNutrition (66), tblRoutine, tblWorkout, tblVax, tblPhys, tblSizes, tblResources, tblEvidence (kd refs + stance), tblGenome (rsid, gene, k1_genotype, k2_genotype, concordant), tblProfile (DOB, sex, height_cm, ancestry).
   - tblSuppTags (supp_id + 9 hallmark booleans + 7 SENS booleans + mechanism loadings 0-3 + indirectness/rung/evidence-grade entry columns). Empty, user-owned; PH sheets in SUP and AGE both read it (one tagging effort).
   - tblRegimes (derived: every supplement start/stop plus manual phase rows: cut, maintain, travel) used by LAB regime/event views and BOD regime stats.
   - tblSuppIngredients (long: supp_or_product, ingredient, amount, unit) derived if product data allows (GATE-FIELDS).
2. Calc layer (formulas only, no per-workstream logic): Calc_Marker (one row per marker: n, first/last date, last, previous, mean, sd, median, min, max, slope per year, dir sign, status code 0 optimal /1 ref-ok-not-optimal /2 out-of-ref /3 beyond-critical, z latest, distance score, pct-in-opt, pct-in-ref, days since last draw); Calc_Panel (per panel: age, n measured, composite score). All workstreams read these instead of recomputing.
3. Named ranges (via `define_name`): Today, DOB, Sex, Height_cm, Sel_Marker, Sel_Supp, Sel_Gene, Latest_Panel, Prev_Panel, N_Panels, plus every Params name below. Each workstream also publishes <=10 KPI cells named `K_<ws>_<name>` (stubs pre-created with "n/a") that GLB reads (contract, avoids cross-sheet coupling).
4. Params sheet (Q90): columns name, value, unit, state (verified / heuristic / UNVERIFIED), source, note; data-validation min/max on each. Seeds: Half_Life_days, Winsor_Cap, Boll_Window/Boll_K, KAMA_fast/slow, Kalman_q/r, CUSUM_k/h, Alpha, Kelly_Fraction, stance weights (Ref_Stance), RCV_z (1.96), CVa/CVi per marker (UNVERIFIED), Decline_Strength/VO2 per decade (OUT ch31/ch34), Caffeine_HL (6 h, WWS ch03; GEN-09 may override via `G_Caffeine_HL`), Protein bands 0.8/1.6/2.2/3.7 g/kg (OUT ch50), PhenoAge constants (weights verified: Levine 2018 Table 1; intercept/gamma/0.0076927/141.50225/0.00553/0.090165 UNVERIFIED), Assumed_CRP (blank), CKD-EPI coefficients (UNVERIFIED), FIB-4/HOMA/TyG/Friedewald/eAG/corrected-Ca constants (UNVERIFIED), delta-check limits (heuristic), metabolic-syndrome thresholds (UNVERIFIED except glucose as printed in OUT ch13), UL table (UNVERIFIED).
5. Ref lookups: Ref_Systems (marker->system), Ref_Units, Ref_Stance (stance->numeric weight), Ref_TimeSlots, Ref_Hallmarks (9, LIFE ch00 lines 1095-1118), Ref_SENS (7), Ref_Mechanisms, Ref_Direction. Rule tables (vaccine, screening, interaction, GRADE, UL/DRI, PGx allele) are owned by the consuming workstream but MUST follow the infra schema: id, rule, value, unit, source, source_state, note.
6. Style/theme module: palette (dataviz skill; light + dark friendly fills), header/table styles, number formats, status colours (optimal/ref-ok/out/critical, plus UNVERIFIED orange, PLACEHOLDER grey), conditional-format helpers (3-colour z scale, data bar, icon set, formula-based quilt), sparkline helper, stacked-area band helper (ref/opt/Blueprint bands), waterfall helper (hidden base series), Gantt helper (invisible-offset stacked bar), scatter-event-lines helper, `note_source(cell, source)` (rust_xlsxwriter Note), banners: `disclaimer()` ("Visualisation only - not medical advice"), `dx_risk()` wording, `awaiting(data_needed)`, `unverified()` badge; tab colour per workstream; freeze panes, widths, print areas.
7. Excel-version decision: target Excel 365 (XLOOKUP, FILTER, SORTBY, LET, dynamic arrays via `write_dynamic_array_formula`; enable the library's future-function prefix handling, verify API name). Data Tables (what-if) unsupported: every parameter grid is an explicit formula grid. Every workbook build runs the Checks sheet formulas in a LibreOffice headless recalculation test to catch #NAME?/#REF!.
8. Build harness: registry of `SheetModule { name, order, build(&mut Workbook, &Ctx) }`; one file per workstream directory so parallel agents never touch the same file; golden test that every declared sheet name and named range is unique (collision = build failure).
9. Data loader: `biomarker-cli`, `health-cli`, `genome` JSON -> typed structs; missing-input detector that feeds `awaiting()` and the Checks sheet.

### Assumptions to verify before Wave 1 (open facts, not blockers)
- A1: daily kcal and protein intake exist for 2023-11..2024-07 (quant and longevity catalogs assume ~250 days; the verified facts list only weight). Ideas needing intake are marked PH until confirmed; flip to BN if present.
- A2: tblSupp has start_date/stop_date (GATE-DATES). A3: tblSupp/tblProducts carry price, units per container, dose, frequency, time_of_day, source protocol, stance, ingredient amounts (GATE-FIELDS). A4: tblProfile has DOB, sex, height. A5: Rx drug list exists anywhere (SUP interaction matrix drug columns). A6: workout rows carry date, exercise, sets, reps/load, duration, optional RIR/muscle (AGE training views). A7: tblNutrition has macros, price, Yuka score. A8: physicals include BP/waist (CLN metabolic-syndrome counter).
- Unused-but-present inputs: sizes, resources (GLB-08 gives them a reference sheet), resources also feed Nav.

## 2. Workstreams (disjoint sheets and modules)

Dependency: Wave 0 infra -> Wave 1 (W1-W7 in parallel, each reads only Data_/Calc_/Ref_/Params and stubs) -> Wave 2 (W8 reads `K_*` contracts). Larger workstreams (W1, W5) can be split by lane with no shared files; lanes listed.

| WS | Name | Module dir | Sheets owned (all prefixed) | Ideas |
| --- | --- | --- | --- | --- |
| W1 | Lab tear sheets, stats and experiments | src/lab/ | Lane A (stats): Lab_Tear, Lab_Trend, Lab_Grid, Lab_Z, Lab_Rank, Lab_EWMA, Lab_Rate, Lab_Score, Lab_Gap, Lab_Range, Lab_Drawdown, Lab_Momentum, Lab_RCV, Lab_RTM, Lab_Delta, Lab_Complete, Lab_WalkFwd, Lab_VaR, Lab_Corr, Lab_PCA, Lab_WtAdj, Lab_Season, Lab_Guard. Lane B (experiments): Lab_Event, Lab_Regime, Lab_Confound, Lab_Dose, Lab_N1, Lab_Prereg, Lab_Journal, Lab_Hygiene, Lab_Brier, Lab_Quantile, Lab_Ledger, Lab_Bayes | LAB-01..35 |
| W2 | Clinical derived indices and flags | src/clinical/ | Cln_Lipids (CLN-01..07), Cln_RiskChecklist, Cln_Glycemia, Cln_Insulin, Cln_AnionGap, Cln_Renal, Cln_Liver, Cln_Iron, Cln_Thyroid, Cln_Uric, Cln_Derived, Cln_MetSyn, Cln_Systems, Cln_NutStatus, Cln_Loops, Cln_FH | CLN-01..25 |
| W3 | Supplement portfolio, cost, interactions, evidence | src/supplement/ | Sup_Book, Sup_Cost, Sup_Unit, Sup_Churn, Sup_Mech, Sup_DropOne, Sup_Value, Sup_Overlap, Sup_DRI, Sup_Audit, Sup_Interact, Sup_Regimen, Sup_LabDrug, Sup_PK, Sup_Cycle, Sup_Grade, Sup_Papers, Sup_Calc, Sup_Balance, Sup_Forest, Sup_Bubble, Sup_TrialDose, plus rule tables Ref_Interact, Ref_GRADE, Ref_UL | SUP-01..23 |
| W4 | Aging clocks, longevity frameworks and fitness | src/age/ | Age_Pheno, Age_Vendor, Age_PhenoWhatIf, Age_Claims, Age_Epi, Age_Hallmarks, Age_SENS, Age_Protocols, Age_Horsemen, Age_DEXA, Age_Decathlon, Age_VO2, Age_Zone2, Age_Z2Tools, Age_Strength, Age_Volume, Age_Sauna, Age_Decade | AGE-01..18 |
| W5 | Body, nutrition, sleep and wearable signals | src/body/ | Lane A (weight/nutrition): Bod_Weight, Bod_Rate, Bod_Dist, Bod_Signal, Bod_Bands, Bod_Kalman, Bod_Slope, Bod_Cusum, Bod_Patterns, Bod_Revert, Bod_Osc, Bod_Fan, Bod_Glide, Bod_Regime, Bod_Sens, Bod_BMI, Bod_Protein, Bod_TDEE, Bod_LeadLag, Bod_Macro, Bod_Dozen, Bod_Yield, Bod_Glycemic, Bod_Window, Bod_Alcohol. Lane B (sleep/wearable): Bod_Wearable, Bod_Sleep, Bod_Arch, Bod_Caffeine, Bod_SleepHyg | BOD-01..30 |
| W6 | Genetics cross-cuts | src/gen/ | Gen_CrossCut, Gen_QC, Gen_Exposure, Gen_Scorecard, Gen_APOE, Gen_Lipid, Gen_MTHFR, Gen_PGx, Gen_Caffeine, Gen_Cards, Gen_Todo, Ref_Allele | GEN-01..11 |
| W7 | Schedules, routine, vaccines, prevention, visit pack | src/schedule/ | Sch_Vax, Sch_Screen, Sch_Provider, Sch_Monitor, Sch_Retest, Sch_PreDraw, Sch_Dosing, Sch_PillLoad, Sch_Gantt, Sch_Refill, Sch_Chelate, Sch_Morning, Sch_Visit, Sch_POMR, Sch_MedRec, Sch_Low, plus rule tables Ref_Vax, Ref_Screen | SCH-01..14 |
| W8 | Global dashboard, Checks, navigation | src/global/ | Nav, Dash, Checks, Alerts, Changes, Scenario, YearReview, Export, Sources | GLB-01..08 |

Cross-workstream interfaces (read-only, by name): GEN-09 writes `G_Caffeine_HL`, BOD-29 reads it with fallback to Params!Caffeine_HL. W2 Cln_Loops, W1 Lab_Dose and W7 Sch_Monitor each recompute from Data_ tables rather than from each other. W8 reads only `K_*` names and Calc_Marker.

## 3. Master idea list

Columns: ID | aliases | summary | inputs | build notes | effort | status | tags | sources.

### W1 Lab tear sheets, stats and experiments (LAB)

| ID | Aliases | Summary | Inputs | Build notes | Eff | St | Tags | Sources |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| LAB-01 | Q9, C1.4 | Selector-driven one-page marker tear sheet: current vs ref/optimal/Blueprint bands, history line with personal +-1 SD band, z, percentile, slope, time-in-range, best/worst, sparkline, supplements started near history | tblLabs, tblMarkers, Calc_Marker, tblRegimes | DV list Sel_Marker; XLOOKUP/INDEX; stacked-area band helper + line chart; one layout serves 56 markers | M | BN | TOP | FO Ch13; KAU Ch23; OUT ch15:9,15; DIP ch37:299 |
| LAB-02 | Q16, Q17, C1.7, L-B17 | Trend per marker: OLS slope/yr with SE, R2, prediction band, t-stat verdict (signal vs noise), projected next value, projected date of crossing a ref limit, "worsening 2 panels" flag | tblLabs, dates | SLOPE/INTERCEPT/RSQ/STEYX, T.INV.2T band; band is wide at n=7 by design; icon set | M | BN | TOP | KAU Ch6; PAL Ch3 FAQ3.2; OUT ch13:182-186; DIP ch97:199 |
| LAB-03 | Q10, Q11, C1.6, L-B15 | Marker x panel grids: report-card heatmap (status 1-4 quilt), out-of-range distance heatmap, summary columns (first, last, change, rate, min, max, #in-range), sparkline column, system grouping | tblLabs, tblMarkers | status helper grid + formula conditional formats; SORTBY or Rust-side sort; frozen panes | M | BN | TOP | FABER ch15; KAU Ch23; OUT ch13:182,186 |
| LAB-04 | Q1, Q5 | Personal-baseline z-score heat grid (needs >=3 panels), winsorise toggle (cap +-3 via Params) | tblLabs | (v-mean)/STDEV.S; MEDIAN(-cap,z,cap); abs 3-colour scale | S | BN | TOP; MED-VERIFY (index of individuality, Fraser, not in library) | KAU Ch2; PAL Ch6.5 |
| LAB-05 | Q2 | Percentile of own history, "highest/lowest ever" badge | tblLabs | PERCENTRANK.INC; icon set | S | BN | | KAU Ch10, Ch2 |
| LAB-06 | Q3 | EWMA personal mean/SD with half-life input (time-based in days because panels are irregular), static vs EWMA z | tblLabs, Params | recursion helper columns; lambda=0.5^(dt/HL) | S | BN | | PAL Ch2.3.1; KAU Ch7 |
| LAB-07 | Q6 | Panel-to-panel % change, log change, annualised rate | tblLabs | LN(v2/v1)/((d2-d1)/365.25); data bars | S | BN | | PAL Ch2.1.3; KAU Ch2 |
| LAB-08 | Q4, Q65 | Distance-to-optimal score 0-100 per marker, system roll-ups, composite health index by panel (user weights vs inverse-vol weights), radar first vs latest | tblMarkers, tblLabs, Calc_Marker | piecewise formula with IFS; SUMPRODUCT index; line + radar | M | BN | TOP | KAU Ch2, Ch24; PAL Ch9.6; FABER ch08 |
| LAB-09 | L-G1, L-B16 | Blueprint-vs-me gap view and weighted priority queue: latest, Blueprint target, ref, optimal, gap, % of way, bullet bar, top-10 furthest, editable 1-5 weights | tblMarkers (Blueprint set), tblLabs | helper rank; SORTBY dynamic array; data bars; sparkline | M | BN | TOP | OUT ch13:182; ch15:7,17 |
| LAB-10 | Q12, C1.5 | Time-in-range (reference and optimal) per marker and whole-body trend | tblLabs, tblMarkers | COUNTIFS; data bars; system SUMPRODUCT | S | BN | | KAU Ch10; OUT ch49 (CGM analog) |
| LAB-11 | Q13, Q14, Q15 | Drawdown from personal best (direction-aware), Ulcer Index, out-of-range episodes and time-to-recovery | tblLabs, dir flag | running best, underwater area chart, episode ids + MINIFS | M | BN | | CHAN L3169-3197; KAU Ch23 |
| LAB-12 | Q41, Q42, Q43 | Trend-state arrows (latest vs mean of last k), dual-momentum leaderboard (relative rank + beyond-noise), persistence test (lag-1 autocorr of changes, trending vs reverting) | tblLabs | SIGN formulas; RANK.EQ; CORREL stacks | S | BN | | FABER taa-2007; moskowitz_ooi_pedersen_2012; carhart1997 |
| LAB-13 | Q19, C1.1 | Reference change value / "is this change real": per marker per interval, within noise vs meaningful | tblLabs, CVa/CVi table | RCV=1.414*1.96*SQRT(CVa^2+CVi^2); until CV table is verified use empirical proxy from SD of diffs (labelled); icon set | M | NS | TOP; EXT; MED-VERIFY | Fraser/Harris, EFLM Biological Variation DB (pending); PAL Ch3 FAQ3.2, Ch14 |
| LAB-14 | Q20, C1.8 | Regression-to-the-mean warning (pooled lag-1 r, expected reversion, net change) and single-outlier guard (MAD) | tblLabs | pooled CORREL over stacked z; ABS(last-MEDIAN)/MAD > Params | M | BN | TOP | BERN ch12; BUT; OUT ch22:19-21 |
| LAB-15 | Q7, C1.2 | Lab/assay/fasting change flags and delta-check (implausible jump) flag; chart markers by lab | tblLabs (lab, method, fasting) | IF(lab<>prev_lab); limit per marker in Params (heuristic) | S | BN | | PAL Ch8.1.1; DIP ch98:233 |
| LAB-16 | C1.9 | Panel completeness matrix and months-since-last-measure; never-measured markers highlighted | tblLabs, tblMarkers | COUNTIF grid; conditional format blanks | S | BN | | OUT ch15:7, ch14:163 |
| LAB-17 | Q35, Q74 | Walk-forward forecast scorecard (trend vs EWMA vs last-value, winner per marker) and ensemble forecast with extremizing and own-guess input | tblLabs | expanding-window SLOPE; MIN of abs errors; SUMPRODUCT | M | BN | | PAL Ch8.2.1; KAU Ch21; FRI Satopaa 2015 |
| LAB-18 | Q67 | Next-panel "value at risk": 95% plausible range and probability of leaving reference range, fan chart | tblLabs | PERCENTILE.INC of diffs or NORM.DIST | M | BN | | KAU Ch23 |
| LAB-19 | Q58 | Marker correlation matrix (Pearson and Spearman), significance mask using r_crit at n=7, drill-through scatter | tblLabs | CORREL generated per cell; rank helper block | M | BN | | KAU Ch6 |
| LAB-20 | Q62, Q63 | PCA scree on z-scored complete markers and effective-breadth score; state that <=6 components exist at n=7 | tblLabs | eigendecomposition in Rust, static values labelled "computed offline"; breadth =SUM(l)^2/SUMSQ(l) | L | BN | | PAL Ch7.2.5, Ch9 Insight 9.3 |
| LAB-21 | Q61, Q34 | Beta-to-weight and R2-by-weight per marker; FWL residuals (marker adjusted for weight) feeding event study | tblLabs incl. `weight` marker | SLOPE/RSQ; residual columns | M | BN | | PAL Ch4.7.3, Ch12 |
| LAB-22 | Q29 (lab part) | Draw-month table and seasonality caution (7 draws: months 12,10,1,11,3,12,1; no index, show only the table) | tblPanels | month extraction | S | BN | MED-VERIFY | KAU Ch10 |
| LAB-23 | Q36 | Multiple-comparisons guard: hypothesis count, expected false positives, Bonferroni threshold banner reused by LAB-19, LAB-24 | counts | named cells Guard_N, Guard_Alpha | S | BN | TOP | CHAN L3551,L6013; PAL Ch8 |
| LAB-24 | Q32 | Event study: marker change from last pre-start panel to first post-start panel per supplement, z against all-interval null, heat grid supplement x marker | tblSupp (start), tblLabs, LAB-21 residuals | long pair table (formulas); flag abs z>2; links to LAB-14 and LAB-23 | L | BN | TOP; GATE-DATES | KAU Ch2; PAL Ch14 |
| LAB-25 | Q31, C2.4, L-G2 | Regime timeline: interventions as shaded bands/vertical event lines over any marker or weight chart; lipid-intervention overlay with start labels | tblRegimes, tblLabs, tblDaily | 0/1 helper columns -> stacked area; scatter event lines; DV selector | M | BN | TOP; GATE-DATES | CHAN regime switching; DIP ch37:451 |
| LAB-26 | Q33 | Confounding map: count of other changes in each panel interval per start | tblSupp dates | COUNTIFS between panel dates | S | BN | GATE-DATES | PAL Ch14, Ch4 |
| LAB-27 | Q60, L-G18 | Dose-response: choose supplement and marker (DV), dose at each panel date, beta/R2/residual SD, scatter + trendline | tblSupp dose history, tblLabs | named dynamic ranges via INDEX; SLOPE/RSQ/STEYX | M | BN | GATE-DATES | PAL Ch12 Procedure 12.1 |
| LAB-28 | L-G3 | N-of-1 experiment ledger: before/after mean-SD, delta, Cohen d, T.TEST, verdict with caveat banner | entry + tblLabs/tblDaily | T.TEST; AVERAGEIFS by date window | M | BN | TOP | GLU ch22; TW 2106537633056969024 |
| LAB-29 | Q37 | Pre-registration table: hypothesis, marker, direction, minimum meaningful size in RCV units, review date; auto-outcome from next panel | entry + tblLabs | DV lists; first post-review panel lookup | M | BN | TOP | PAL Ch8.1.2; DUKE Ch3 |
| LAB-30 | Q38 | Decision journal: process vs outcome grades, 2x2 counts | entry | DV 1-5; COUNTIFS | S | BN | | DUKE Ch1-2 |
| LAB-31 | Q39 | Backtest-hygiene checklist (look-ahead, fasting match, survivorship) with % complete | entry | Y/N DV | S | BN | | PAL Ch8.1.2 |
| LAB-32 | Q72 | Prediction log with Brier score, calibration buckets, skill trend | entry | (p-o)^2; COUNTIFS buckets; scatter + 45-degree line | M | BN | | DUKE Ch2-3; FRI Himmelstein 2024 |
| LAB-33 | Q73 | Quantile forecasts (10/50/90) with pinball loss and coverage rate | entry + next panel | MAX(q*(y-f),(q-1)*(y-f)) | M | BN | | FRI Zhu 2024 |
| LAB-34 | Q75 | Evidence ledger with log-odds weights per supplement, posterior from prior (weights are David's judgment, labelled) | tblEvidence + entry | LN weights; SUMIFS; logistic | M | BN | | GOOD; hanson-lmsr |
| LAB-35 | Q76 | Bayesian "is it working" update after each panel from LAB-24 z and LAB-34 prior | LAB-24, LAB-34 | NORM.DIST likelihood ratio | M | BN | GATE-DATES | GOOD; DUKE Ch3 |

### W2 Clinical derived indices and flags (CLN)

| ID | Aliases | Summary | Inputs | Build notes | Eff | St | Tags | Sources |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CLN-01 | Q8, C2.1, L-B4 | TG/HDL per panel with lines at 2 and 1 | triglycerides, hdl-c | ratio + bands | S | BN | TOP; MED-VERIFY | OUT ch13:186 |
| CLN-02 | Q8, C2.2, C2.7 | Non-HDL (TC-HDL), TC:HDL ratio (cross-check vs reported chol-hdl-ratio), remnant (TC-HDL-LDL) | lipids | arithmetic; FH-pattern note at non-HDL>=220 (DX-RISK wording) | S | BN | DX-RISK | DIP ch37:343 |
| CLN-03 | C2.6, L-B7, Q8 | LDL method cross-check: Friedewald (TC-HDL-TG/5), VLDL=TG/5 (TG<400), Martin-Hopkins vs lab LDL-C; vldl-c is reported so show diff | lipids | constants in Params UNVERIFIED | S | NS | EXT; EXT-unv | Friedewald 1972; Martin-Hopkins JAMA 2013 (pending) |
| CLN-04 | Q8, C2.3, L-B2 | ApoB vs LDL-C/non-HDL concordance scatter, regression, residual flag, ApoB/LDL ratio vs own history | apob, ldl-c | SLOPE/RSQ; residual > 1 SD | M | BN | TOP; MED-VERIFY | OUT ch15:7; DIP ch37:107-109,191; KAT ch46:39 |
| CLN-05 | L-B1 | ApoB dashboard with target bands: guideline LDL-C lines (not ApoB, labelled), Attia 20-30 mg/dL goal, Blueprint band | apob | line + constant series | M | BN | TOP; MED-VERIFY | OUT ch15, ch16 |
| CLN-06 | Q69, C3.7, L-B3 | ApoB-years: trapezoid exposure vs 25 mg/dL baseline; generalised to any marker via selector | apob (and any), dates | running sum area chart; extrapolation before 2018 flagged as assumption | M | BN | TOP; MED-VERIFY | OUT ch14:93, ch15:57-59; KAU Ch23 |
| CLN-07 | C2.5, C2.4 | Expected vs observed % LDL change per intervention (fiber, phytosterols, PCSK9i etc. from DIP prose) | tblSupp dates, lipids | expected lookup table with sources | S | BN | GATE-DATES | DIP ch37:327,331,345,387 |
| CLN-08 | C3.5, L-B8, L-B9 | Attia priority-marker checklist (ApoB, Lp(a) once, TG/HDL, insulin, uric acid, homocysteine, ALT, hs-CRP): measured?, last date, sparklines, "NOT TESTED" tiles | tblMarkers, tblLabs | COUNTIF/MAXIFS; tiles | S | PH | TOP | OUT ch13:186, ch15:7, ch14:163; KAT ch46:169 |
| CLN-09 | C3.6 | FH / Lp(a) family-history pattern flag (LDL-C>=190 or non-HDL>=220 plus family history) | ldl-c, family history | IF rules | S | PH | DX-RISK | DIP ch37:343; OUT ch14:161-163 |
| CLN-10 | L-B10, C2.16 | Glycemia: glucose, A1c (and insulin when present) small multiples with ADA bands and 72-85 "thriving" band; eAG vs measured | glucose, hba1c, estimated-average-glucose | stacked-area bands; eAG formula 28.7*A1c-46.7 UNVERIFIED so reported eAG is primary | M | BN | DX-RISK; EXT-unv (formula) | GLU ch11; OUT ch13:182; DIP ch97:23,197 |
| CLN-11 | Q8, C2.14, L-B11 | TyG (needs TG, glucose: computable), HOMA-IR/QUICKI (insulin missing) | triglycerides, glucose, insulin | LN(TG*Glu/2); (Ins*Glu)/405 | S | NS | EXT-unv; DX-RISK | Matthews 1985; Simental-Guerrero 2008 (pending) |
| CLN-12 | C2.15 | OGTT glucose/insulin curve template (0-120 min) | entry | scatter | S | PH | | OUT ch13:188-190 |
| CLN-13 | C2.9, C2.10 | Anion gap computed vs reported with sourced bands (normal ~9, 3-11; >17-20 elevated); albumin-adjusted AG | sodium, chloride, co2, anion-gap, albumin | Na-(Cl+HCO3); adjusted term in Params UNVERIFIED | S | BN | DX-RISK | DIP ch75:199-211,243 |
| CLN-14 | C2.12, C5.2, L-B13 | eGFR CKD-EPI 2021 vs reported egfr, Cockcroft-Gault, creatine-supplement artifact flag on creatinine/eGFR (and PhenoAge) | creatinine, egfr, age, sex, tblSupp (creatine dates) | closed-form with Params coefficients UNVERIFIED | M | NS | TOP; EXT-unv; DX-RISK; GATE-DATES | Inker NEJM 2021 (pending); DIP ch65:19-25, ch67:173; HELMS ch02 |
| CLN-15 | C2.13 | uACR x eGFR KDIGO risk grid | uACR | 2D conditional-format grid | M | PH | DX-RISK | DIP ch67:173 |
| CLN-16 | Q8, C2.20, L-B12 | ALT trend, De Ritis (AST/ALT), FIB-4 (age, AST, ALT, platelets) shown as number only | alt, ast, platelets, age | formula in Params UNVERIFIED | S | NS | EXT-unv; DX-RISK | Sterling 2006 (pending); OUT ch13:186 |
| CLN-17 | C2.17, C8.4 | Iron quartet (ferritin, TSAT, MCV, RDW, Hb) with pattern flags; ferritin acute-phase caveat | hemoglobin, mcv, rdw (present); ferritin, iron, TIBC (missing) | TSAT=iron/TIBC*100 | M | PH | DX-RISK | DIP ch124:217-257 |
| CLN-18 | C2.22, L-B14, C8.6 | Thyroid: TSH bullet vs ref/optimal, FT4 axis scatter when present, biotin-in-last-48h annotation per panel | tsh, tblSupp (biotin mcg) | SUMIFS biotin; threshold 5000 mcg from DIP | M | BN | DX-RISK; GATE-FIELDS | DIP ch98:111,231-233,347 |
| CLN-19 | C2.21, C8.5 | Uric acid with 7 mg/dL solubility and 6 target lines plus asymptomatic caveat | uric acid (missing) | constant lines | S | PH | | DIP ch115:21,51,107; OUT ch13:150-164 |
| CLN-20 | C2.8, C2.11, C2.18, C2.19 | Misc derived: NLR, PLR, total lymphocyte count <1,200 note, BUN:Cr and A/G cross-check vs reported, corrected calcium, Mentzer index | cbc diff, bun, creatinine, calcium, albumin | formulas in Params UNVERIFIED | S | NS | EXT; EXT-unv | DIP ch164:297, ch164:83; Figge; standard texts (pending) |
| CLN-21 | C3.8, L-B5 | Metabolic-syndrome criteria counter per panel (TG, HDL, glucose from labs; waist, BP from physicals/sizes) | labs, tblPhys | thresholds editable with source comment (OUT prints glucose >110) | S | NS | DX-RISK; EXT | OUT ch13:53-65,186; NCEP ATP III/AHA (pending) |
| CLN-22 | L-B6 | BMI vs metabolic-count quadrant over time | CLN-21, weight, height | scatter + quadrants | M | NS | DX-RISK | OUT ch13:47-75; TW 1769007456988860465 |
| CLN-23 | C3.1, Q64 | Organ-system dashboard: tiles per system with N markers, % optimal, trend arrow, sparkline; burden decomposition by system over panels | Calc_Marker, Ref_Systems | COUNTIFS/SUMIFS; stacked columns | M | BN | TOP | DIP section structure; PAL Ch4.5.2 |
| CLN-24 | C4.6 | Nutrition-status lab bundle (albumin, lymphocytes present; prealbumin, transferrin, CRP missing) with half-life explainer | albumin, lymphocytes-abs | table + notes | S | PH | | DIP ch164:269-297 |
| CLN-25 | C8.1, C8.2, C8.3, L-D9, L-D10, L-D11 | Marker-supplement response loops: vitamin D dose vs 25-OH-D with 20/40 ng/mL lines, B12/folate vs homocysteine, EPA/DHA dose vs TG, iron, uric, iodine/biotin vs thyroid | tblSupp doses, labs | one parameterised panel per pair; scatter + regression | M | PH | GATE-DATES; MED-VERIFY | SR-papers (vit D stress fracture), KAT ch46:169; GREG ch35; HELMS ch02; OUT ch28:37 |

### W3 Supplement portfolio, cost, interactions, evidence (SUP)

| ID | Aliases | Summary | Inputs | Build notes | Eff | St | Tags | Sources |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| SUP-01 | Q44 | Position book: 87 supplements table with dose, frequency, time of day, price, $/yr, protocol, stance, start, status, weight % of spend | tblSupp | calculated columns; DV; data bars; autofilter | S | BN | GATE-FIELDS | KAU Ch24 |
| SUP-02 | Q45, Q46, L-G10 | Cost attribution by protocol and stance, waterfall, Pareto, HHI/Gini, protocol x stance matrix, "spend with no evidence" call-out | tblSupp | SUMIFS; hidden-base waterfall; sorted bar + cumulative line | M | BN | TOP | PAL Ch14, Ch4.5.1; KAU Ch24 |
| SUP-03 | Q53, C6.4, L-G9 | Spend KPIs: $/day, $/month, TER vs budget, by time-of-day, by category | tblSupp, tblProducts | SUM; KPI cells | S | BN | GATE-FIELDS | FABER ch16 |
| SUP-04 | Q54 | Unit economics: $/mg, $/serving, cheapest of 45 products per ingredient, savings if switched | tblProducts, tblSuppIngredients | MINIFS by ingredient | S | BN | GATE-FIELDS | KAU Ch13 |
| SUP-05 | Q56 | Churn ledger: cost of starts/stops (unused stock), washout weeks, cumulative waste | tblSupp dates, price | running total | S | BN | GATE-DATES | PAL Ch11 |
| SUP-06 | Q47 | Mechanism-exposure ("factor loadings") matrix with coverage radar and cost per mechanism | tblSuppTags | MMULT/SUMPRODUCT; heat map | L | PH | TOP; MED-VERIFY | PAL Ch4.1-4.2 |
| SUP-07 | Q51 | Drop-one analysis: saving and coverage lost per item, redundant flag | SUP-06 matrix | coverage minus loading | M | PH | | PAL Ch5, Ch4.5.2 |
| SUP-08 | Q49, Q50, Q52 | Benefit-per-dollar score with evidence shrinkage (Kelly fraction, stance factor), ranked bar, quadrant, efficient frontier with budget slider | stance, tags, marker gaps (Calc_Marker), $ | score=evidence_weight*relevance/cost; SORTBY; XLOOKUP at budget | M | PH | TOP; MED-VERIFY | PAL Ch9.5, Ch13.3; KAU Ch23; kelly-criterion papers |
| SUP-09 | Q48, L-G8 | Redundancy and stacked-dose detector by active ingredient across products | tblSuppIngredients | SUMIFS; duplicate flags (UL compare lives in SUP-10) | M | BN | GATE-FIELDS | PAL Ch4; KAU Ch24 |
| SUP-10 | C4.7, C6.6, L-D7, L-D8 | Micronutrient DRI/UL ledger (foods + supplements) and guideline-dose checker incl. WHO reference doses; Rx vs supplement vs WHO-essential tags | tblSuppIngredients, tblNutrition | SUMIFS; total/UL; UL table Params UNVERIFIED | L | NS | EXT; MED-VERIFY; GATE-FIELDS | DIP ch164:343,387; WHO ch00:4346-4420; GREG ch35; HELMS ch02; NIH ODS (pending) |
| SUP-11 | C3.3, C3.4, C6.9 | Indication-effectiveness-safety-adherence audit per supplement, medication-therapy-problem tags, marker-without-intervention and intervention-without-marker gap analysis | tblSupp, labs, tblEvidence | four computed lights; anti-join with COUNTIF | M | BN | | DIP ch07:105-119 |
| SUP-12 | C5.3, C5.4, C5.7, C5.8 | Supplement-drug interaction matrix seeded from Katzung (St John's wort, garlic/ginkgo/ginseng, CoQ10, glucosamine, melatonin, red yeast rice, niacin), mechanism taxonomy filter, statin map, vitamin-K-antagonist panel | tblSupp, Rx list (A5) | Ref_Interact rule table with source column; severity colours; drug columns PH if no Rx list | L | BN | TOP; MED-VERIFY | KAT ch80:77,109,143,175-177,243,289-297,321,355,489; ch82; DIP ch37 |
| SUP-13 | C5.10 | Drug/supplement-induced lab abnormality explainer (niacin/statin ALT, amiodarone-thyroid, topiramate) | labs, stack | rule table | L | NS | MED-VERIFY | KAT ch46:201; DIP ch75:225, ch98:231 |
| SUP-14 | C5.5, C5.6, C6.7 | Inducer/inhibitor onset-offset windows (7-14 d), steady-state/washout calculator (4 half-lives), half-life-based timing | tblSupp, half-lives | 0.5^(t/t1/2); Gantt CF | M | NS | EXT | KAT ch82:65, ch09:107-111; GG ch88; half-lives (pending) |
| SUP-15 | C6.8 | Cycling/discontinuation tracker (weeks on, onset windows e.g. Bacopa 12 wk, SJW 2-4 wk) | tblSupp dates | DATEDIF | S | BN | GATE-DATES | KAT ch80:247; SR-papers Bacopa |
| SUP-16 | C7.1, C7.2, C7.7, L-A13 | GRADE certainty badge per supplement-outcome (start 4/2, downgrades, upgrades, "+" stars), summary-of-findings rows, directness flags, intervention evidence ladder (RCT/biomarker/animal/mechanistic) | tblEvidence, tblSuppTags entry | CHOOSE/MAX; stacked bar | M | BN | TOP | EV GRADE-3 l.238-261, GRADE-1; LIFE l.4890-4906,5016-5058 |
| SUP-17 | C7.3, C7.4, C7.6, C7.12 | Paper-quality grids: RoB-2 five-domain traffic lights, AMSTAR-2 16 items with 7 critical, power badge, "how far to trust one paper" checklist | tblEvidence papers, entry | worst-domain formula; AMSTAR-2 rating formula | M | BN | | EV RoB2 l.93-97; AMSTAR2 l.478-520; BUT; Munafo/Simmons/Camerer |
| SUP-18 | C7.5, C7.10, C7.11 | Evidence calculators: Ioannidis PPV (with bias), NNT/NNH, sensitivity/specificity/PPV | entry | closed formulas | S | BN | | EV IOA; OUT ch15:57, ch22:19-21 |
| SUP-19 | C7.8 | Supporting vs warning papers balance bar per supplement, certainty-weighted | tblEvidence stance | COUNTIFS/SUMPRODUCT | M | BN | | SR-papers (incl. nicotinamide warning paper) |
| SUP-20 | C7.9 | Effect size + CI forest plot per supplement outcome with "absence of signal is not safety" note | entry | bar chart with custom error bars | M | BN | | CIP bottom line |
| SUP-21 | C7.13, L-G7 | Cost vs certainty bubble (x certainty/rung, y $/month, bubble pill count or hallmark count): "expensive and uncertain" quadrant | SUP-02, SUP-16 | bubble chart | M | BN | TOP | combines C6.4 + C7.1 |
| SUP-22 | C7.14 | Dose-in-trial vs dose-taken comparator ("no trial at this dose") | tblSupp, entry trial doses | ratio | M | BN | | KAT ch80:247; SR-papers |
| SUP-23 | hx-ft3.12 | Taking vs not (biomarker regimens vs store) and the per-person contraindication grid | biomarker `regimen ls`, `interaction grid --catalog` | static from biomarker JSON, store fallback | M | BN | | biomarker-cli docs/interventions.md |

### W4 Aging clocks, longevity frameworks, fitness (AGE)

| ID | Aliases | Summary | Inputs | Build notes | Eff | St | Tags | Sources |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| AGE-01 | L-A1, L-A2, L-A5, C8.8 | PhenoAge per panel with input-completeness checker (9 inputs), age acceleration vs y=x, mortality score M. hs-CRP missing on every panel: build full formula with CRP column empty and a "lite" mode driven by `Assumed_CRP` (blank = n/a) | albumin, creatinine, glucose, lymphocytes-pct, mcv, rdw, alp, wbc; hs-CRP MISSING; DOB | unit helpers (g/dL x10, mg/dL x88.42, /18.016); weights verified (Levine 2018 Table 1, PMC5940111); constants UNVERIFIED (README only) | M | PH | TOP; EXT-unv; DX-RISK | Levine 2018; mcp-phenoage-clock README |
| AGE-02 | NEW | Vendor `biological-age` marker vs chronological age over time and vs AGE-01 | biological-age, DOB | line + gap | S | BN | | biomarker-cli marker list |
| AGE-03 | L-A3, L-A4 | PhenoAge marker waterfall (swap-one-marker rows) and what-if input cells | AGE-01 inputs | waterfall helper; override column with DV | M | PH | EXT-unv | Levine Table 1 |
| AGE-04 | L-A6 | Comparator lines for Johnson/Pascoe claims (labelled "different clock", user-entered) | entry | annotated line | S | BN | weak (TW) | TW 1857431845576798654, 1803400497103249519 |
| AGE-05 | L-F14 | Epigenetic-clock placeholder (Horvath/GrimAge/DunedinPACE slots) beside PhenoAge | entry | table + chart | S | PH | | LIFE l.1182-1190,2256-2260 |
| AGE-06 | L-A7, L-A8, L-A9 | Hallmarks-of-aging coverage matrix (87 x 9), cost per hallmark, hallmark x stance heat grid, red flag where 0-1 interventions | tblSuppTags, tblSupp | SUMPRODUCT with flags; COUNTIFS | M | PH | TOP; MED-VERIFY | LIFE l.1095-1118,1170-1171 |
| AGE-07 | L-A12 | SENS damage-category map with honest empty rows | tblSuppTags | COUNTIF matrix | S | PH | EXT-unv | DEGR ch08 l.66-94 |
| AGE-08 | L-A10, L-A11, L-G12, C8.7 | Protocol library: Sinclair "what I do" overlap with dose ratios, multi-protocol comparison (Blueprint/Sinclair/Attia/Greger/Helms vs David actual with per-cell source notes), longevity-drug watch (metformin, rapamycin, NMN/NR, resveratrol, statins) | tblSupp, tblDaily, labs, static refs | XLOOKUP by name; Note per cell | M | BN | TOP | LIFE l.10605-10662; OUT ch50,33,60,49; GREG ch35; HELMS ch02; SR-papers |
| AGE-09 | L-B18 | Four-horsemen coverage map: markers, habits, screenings mapped to atherosclerosis/cancer/neurodegeneration/metabolic with count and recency | mapping entry, labs | COUNTIFS; stacked bar | M | BN | TOP | OUT Part II ch13-28 |
| AGE-10 | L-B20, L-G17 | DEXA sheet: BMD, lean mass, ALMI, FFMI, visceral fat, T/Z-score; BMI shown last with caveat | entry (DEXA absent) | ratio formulas | M | PH | TOP | OUT ch35:19-25,109; ch13:114 |
| AGE-11 | L-C1 | Centenarian Decathlon tracker: 20 tasks, target age, required capacity today = target/(1-decline)^(years/10), measured, pass/fail, bullet chart | DOB, entry tests | formula check: 30 lb at 80 -> ~50 lb at 40 | L | PH | TOP | OUT ch31, ch34 |
| AGE-12 | L-C2, L-C3, L-C4, L-C9, C11.6 | VO2 max: lifetime projection (10%/decade, 15% after 50, thresholds 18/15, 30, 32, 35), percentile and hazard-ratio lookup, elite-for-sex target, annual test card, resting-HR proxy | entry VO2, Whoop RHR | norm table and 15.3 proxy in Params UNVERIFIED | M | PH | TOP; EXT-unv | OUT ch34:5-30, ch30:61-65, ch31:55; Uth 2004 (pending) |
| AGE-13 | L-C5 | Zone 2 weekly minutes vs 180 target, rolling 4-wk mean, streak, stacked by modality | tblWorkout | SUMIFS by ISO week; target line | M | BN | TOP; GATE-FIELDS | OUT ch33 |
| AGE-14 | L-C6, L-C7, L-C8, L-C17 | Zone 2 HR range (70-85% of true max; 208-0.7*age fallback UNVERIFIED), W/kg progression, 4x4 session log and training-age gate, RPE/cardiac-drift log | entry, Whoop | formulas; scatter bands | M | NS | EXT-unv; PH inputs | OUT ch33, ch34:5,44; Tanaka 2001 (pending); ENDU ch09 |
| AGE-15 | L-C10, L-C11, L-C15 | Strength pillars scorecard, muscle-as-retirement-account projection, volume dose-response reference chart (Radaelli, static) | entry tests, lean mass | per-decade reduction in Params | M | PH | | OUT ch35:7,25,43,53-77; SCHO ch09:23 |
| AGE-16 | L-C12, L-C13, L-C14, L-C16, L-C20 | Training volume: weekly hard sets per muscle vs editable MEV/MAV/MRV bullets, mesocycle ramp planner, frequency counter, lifting sessions/week, cardio:lifting time ratio | tblWorkout (RIR, muscle map) | SUMIFS by week and muscle; landmark numbers user-owned | L | BN | TOP; GATE-FIELDS | ISRA ch03,ch06; SCHO ch09,ch12; HELMS ch01:1047-1061; OUT ch30:128 |
| AGE-17 | L-C18, L-C19 | Sauna weekly count with study reference; post-meal walk log | entry/tblRoutine | weekly COUNTIFS | S | BN | | LIFE l.4219-4229; GLU ch23; TW 2014400965445931450 |
| AGE-18 | L-G6, L-G11 | Marginal-decade planner and qualitative mortality-risk ledger (non-additive banner) | DOB, entry | text table | S | BN | | OUT ch31, ch30:61-89; LIFE l.4228; WWS l.91 |

### W5 Body, nutrition, sleep, wearable signals (BOD)

| ID | Aliases | Summary | Inputs | Build notes | Eff | St | Tags | Sources |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| BOD-01 | L-G5 | Daily weight dashboard: dots + 7-day EMA, weekly rate, kcal and protein/kg (when present), calendar heatmap of target days | tblDaily | EMA formula; CF grid by week | M | BN | TOP | L D1/D2 |
| BOD-02 | C4.1, C4.2, L-D2 | Weekly rate of change with guard-rails (0.5-1%/wk cut band, gain bands), % weight lost vs 5% line, in-band streak | tblDaily | 7-day mean deltas; stacked-area bands | M | BN | TOP | HELMS ch01:955,1090-1127; DIP ch167:19, ch164:89,229 |
| BOD-03 | Q18, Q3 (daily) | Distribution moments and histogram with normal overlay (weight; kcal/protein if A1) | tblDaily | SKEW/KURT/FREQUENCY | S | BN | | KAU Ch2; PAL Ch2 |
| BOD-04 | Q21, Q22 | Efficiency ratio and KAMA adaptive weight trend | tblDaily | ER window; SC recursion; Params fast/slow | M | BN | TOP | KAU Ch17 |
| BOD-05 | Q23 | Personal Bollinger bands (weight now; kcal/protein/HRV later), breach markers, band width | tblDaily | fixed-window formulas generated in Rust; NA() breach series | S | BN | TOP | KAU Ch7, Ch4 |
| BOD-06 | Q24 | Local-level Kalman filter true weight with band and gain | tblDaily, q, r | recursion | M | BN | | PAL Ch2.4; KAU Ch6 |
| BOD-07 | Q25, Q28 | Rolling 14/28-day slope kg/wk with t-stat and N_eff autocorrelation correction | tblDaily | SLOPE window; N_eff | S | BN | | KAU Ch6, Ch2; PAL Ch3 FAQ3.2 |
| BOD-08 | Q26 | CUSUM change-point detector | tblDaily | S+ and S- arms, h/k Params | S | BN | EXT | CHAN regime switching; KAU Ch22 |
| BOD-09 | Q27, Q29, Q81 | Shock days (>5x average change), weekday patterns, autocorrelation cycle detection (lags 1-35) | tblDaily | AVERAGEIFS by WEEKDAY; CORREL per lag | S | BN | | KAU Ch22, Ch15, Ch10, Ch11 |
| BOD-10 | Q30 | Mean-reversion half-life of weight | tblDaily | AR(1) via SLOPE; ln2/-ln(b) | S | BN | | CHAN Ch7; KAU Ch6 |
| BOD-11 | Q80 | Stochastic "where in my range" oscillator | tblDaily | MINIFS/MAXIFS | S | BN | | KAU Ch9 |
| BOD-12 | Q68 | Fan chart projection of weight (analytic or Rust-bootstrap static, labelled) | tblDaily | mean+z*sd*sqrt(t) | M | BN | | KAU Ch21; PAL Ch2 |
| BOD-13 | Q84, Q85 | Plan vs actual glide path with tracking error; weight underwater/regain curve | tblDaily, target | STDEV.S of deviations; running MIN | S | BN | | PAL Ch14; KAU Ch23; CHAN L3169 |
| BOD-14 | Q40 | Regime-conditioned stats (cut/maintain/travel/supplement on-off), Welch T.TEST | tblDaily, tblRegimes | AVERAGEIFS; T.TEST | S | BN | GATE-DATES | KAU Ch2; CHAN |
| BOD-15 | Q91 | Parameter sensitivity heat grids (Bollinger window x k, EWMA alpha) as explicit formula grids | tblDaily | 2-parameter grid; 3-colour | M | BN | | KAU Ch21 |
| BOD-16 | Q78, Q79, L-E12, L-G15, Q3 (daily) | Wearable block: 7/28-day EWMA acute:chronic ratio, 30-day z-bands, HRV/RHR baselines, strain vs zone minutes, HRV vs alcohol/caffeine CORREL matrix | tblWearable (empty) | two EWMA columns; icon set | L | PH | | KAU Ch9; PAL Ch2; OUT ch58:25 |
| BOD-17 | C4.3 | BMI and waist class bands, refeeding-risk BMI note | weight, height, waist (sizes?) | kg/m2; class lookup | S | BN | | DIP ch164:239-245,89 |
| BOD-18 | C3.9, C4.5, L-D1, Q83 | Protein g/kg bands (0.8, 1.6, 2.2, 3.7) with kidney-safe ceiling overlay (eGFR), % days at target, run-length streak statistics | tblDaily protein, weight, egfr | ratio column; COUNTIFS; run length | M | PH | TOP | OUT ch50:9-17; HELMS ch01:1435-1467; KAU Ch22 |
| BOD-19 | C4.4, L-D3 | Implied TDEE from calories vs weight change (7,700 kcal/kg heuristic with crude-rule footnote) | tblDaily kcal, weight | rolling formula | M | PH | EXT | HELMS ch01:957; DIP ch164:401-409 |
| BOD-20 | Q59 | Calorie/protein to weight-change lead-lag correlation (lags 0-14) | tblDaily | CORREL per lag; bar chart | S | PH | | KAU Ch2, Ch6 |
| BOD-21 | Q82, L-D4 | Macro split vs targets and drift bands | tblDaily, targets | stacked area; band CF | S | PH | | FABER ch16; HELMS ch01:1493-1617 |
| BOD-22 | L-D5, L-D6 | Daily Dozen grid (12 categories x days) with auto-mapper from nutrition items; confirm berries/other-vegetables counts (image in source) | entry, tblNutrition tags | DV counts; SUMIFS | M | BN | TOP | GREG ch20, ch27, ch30 |
| BOD-23 | Q86, L-D15, L-D16 | Food yield frontier ($/g protein vs kcal/g protein), Yuka vs protein-density bubble, cost per gram protein ranking | tblNutrition, tblProducts | dominance COUNTIFS; bubble | M | BN | GATE-FIELDS | KAU Ch23; OUT ch50:25 |
| BOD-24 | L-D12, L-D13, L-D14, C4.8 | Glycemic-hack log, CGM insight panel (mean, CV, time 70-110, spikes), fibre per day | entry, CGM (absent) | CF; CORREL | M | PH | | GLU ch11,16-25; OUT ch49 |
| BOD-25 | L-D17 | Eating-window and meal-skip tracker | entry | time arithmetic | S | PH | | LIFE l.3770-3776 |
| BOD-26 | L-D18 | Alcohol log vs next-night sleep and HRV | entry, wearable | CORREL | S | PH | | OUT ch60; WWS tip 4 |
| BOD-27 | L-E1, L-E2, L-E7 | Sleep opportunity vs 8-9 h, wake-time SD, efficiency (>=90%), 7-day debt ledger | wearable/entry | STDEV over 14 nights; CF | M | PH | TOP | OUT ch60 rules 8-9; WWS ch21, ch06 |
| BOD-28 | L-E3 | Sleep architecture stacked stages, expected cycles TIB/90 | wearable | stacked column | M | PH | | WWS l.1192-1210; OUT ch58:25 |
| BOD-29 | L-E4, L-F11 (consumer) | Caffeine clock: mg remaining at bedtime = SUM(dose*0.5^(dt/HL)), HL 6 h default, genotype override from GEN-09 | caffeine log, tblSupp time-of-day | SUMPRODUCT; DV 5-8 h | M | PH | TOP | WWS ch03, ch21 |
| BOD-30 | L-E5, L-E6, L-E8, L-E9, L-E13, C11.5 | Sleep-hygiene scorecard (Outlive 10 rules, Walker 12 tips) vs next-day metrics, last-meal/caffeine gap, sleep x glucose coupling, chronotype card, STOP-BANG slot, sleep and recovery quick panel | entry, wearable | CORREL; checklist DV | M | PH | | OUT ch60,58:25-27,55,ch49:13; WWS ch03,ch21 |

### W6 Genetics cross-cuts (GEN)

| ID | Aliases | Summary | Inputs | Build notes | Eff | St | Tags | Sources |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| GEN-01 | L-F1, L-F10 (partial) | Genotype x biomarker x supplement cross-cut table: gene, rsID, David genotype (k1/k2), modulates, biomarker latest/trend, related supplement, concordant/discordant/untested | tblGenome, labs, tblSupp | XLOOKUP on rsID; MAXIFS; seed APOE, LPA, MTHFR, CYP1A2, GC/VDR, SLCO1B1, 9p21, FTO, TCF7L2, HFE, ALDH2, PER3 | L | NS | TOP; EXT-unv | OUT ch27-28, ch14; SNPedia/ClinVar (pending) |
| GEN-02 | L-F9 | Genome QC: array (k1) vs WGS (k2) concordance, no-calls, key-rsID checks | tblGenome | COUNTIFS; CF | S | BN | | derived |
| GEN-03 | Q93, L-F6, C9.7 | Genome exposure report / annotation browser: variant x gene x system x magnitude x repute, system counts, filter, "bridges to a marker" flag | tblGenome + annotation export | table autofilter; COUNTIFS; annotation source must be cited per row | M | PH | MED-VERIFY | PAL Ch4, Ch6; EV GRADE principle |
| GEN-04 | Q77 | Model-vs-reality scorecard: predicted direction vs observed percentile, hit-rate | GEN-03 annotations, labs | INDEX/MATCH link table | L | PH | MED-VERIFY | PAL Ch5 |
| GEN-05 | L-F2, C9.4 | APOE decoder (rs429358 + rs7412 -> e2/e3/e4 via 9-row lookup) and e4 prevention checklist tied to live data | rs429358, rs7412 | INDEX/MATCH on concatenation; allele table UNVERIFIED | M | NS | TOP; EXT-unv; DX-RISK | OUT ch27:3-17, ch28:7-55; SNPedia (pending) |
| GEN-06 | L-F3, L-F4 | LPA genotype vs Lp(a) card ("NOT TESTED") and lipid genes vs measured ApoB banners | genotypes, Lp(a) missing, apob | card + annotated chart | M | PH | EXT-unv | OUT ch14:157-167, ch15:31 |
| GEN-07 | L-F5, C9.5, L-D10 (gene part) | MTHFR vs homocysteine/folate/B12 and folate form in stack; lab is arbiter | genotypes, homocysteine missing, tblSupp | table | S | PH | DX-RISK; EXT-unv | KAT ch46:169; OUT ch28:37 |
| GEN-08 | C9.1, C9.2, C9.3, L-F7 | Pharmacogene lookup per drug/supplement (CYP2D6/2C19/2C9, SLCO1B1, ABCG2, VKORC1, G6PD, DPYD...), predicted phenotype, inhibitor/inducer phenoconversion, Hardy-Weinberg frequency check | tblGenome, tblSupp, Rx list, allele freqs | XLOOKUP on rsid&genotype into Ref_Allele; chip star-alleles unreliable banner | L | NS | TOP; DX-RISK; EXT | KAT ch11:33,117-185, ch80:243,355, ch82; GG ch14; DIP ch12; gnomAD (pending) |
| GEN-09 | L-F11, L-F12, C9.6 | CYP1A2 caffeine half-life personalisation (writes `G_Caffeine_HL`), melatonin CYP1A2 note, chronotype genotype card | genotypes | default 6 h; slow-metabolizer override editable | S | NS | EXT-unv | WWS ch03; KAT ch80:355 |
| GEN-10 | L-F8, L-F13 | Haplogroup identity card and longevity-allele card (FOXO3, APOE e2, CETP, IL6) with "small effects" label | genome CLI | static cards | S | NS | EXT-unv | LIFE l.1040-1042,1331-1396 |
| GEN-11 | L-F10 | Genotype-triggered lab to-do (e4 -> insulin, homocysteine, ApoB, hs-CRP; LPA -> Lp(a); 9p21 -> CAC discussion) with "already measured?" | GEN-05/06, labs | rule table + MAXIFS | S | BN | | OUT ch28:7,37; ch14:167 |

### W7 Schedules, routine, vaccines, prevention, visit pack (SCH)

| ID | Aliases | Summary | Inputs | Build notes | Eff | St | Tags | Sources |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| SCH-01 | C10.1, C10.2, Q88 | Vaccine due-date engine with Ref_Vax interval rules as data, status current/due-90d/overdue, next-12-month ladder, live-vaccine/Ig guard | tblVax | EDATE; due-TODAY(); intervals UNVERIFIED (CDC schedule not in library) | M | NS | TOP; EXT | DIP ch148:17,23,87,113-115,165; CDC (pending) |
| SCH-02 | C10.4, L-B19, L-B21 | Screening schedule by age/sex (colonoscopy, lipid, A1c, PSA/mammography, skin, Lp(a) once, DEXA, cancer calendar, hearing/cognition) with Attia-practice vs guideline labels | tblProfile, tblPhys | DATEDIF; rule table | M | NS | TOP; DX-RISK; EXT | OUT ch22:3,15,25,31,37, ch14:163, ch28:29; DIP ch97:211; USPSTF (pending) |
| SCH-03 | C10.5 | Physical/provider calendar: physicals, specialists, dental, eye, last/next | tblPhys | date arithmetic | S | BN | | DIP ch07 |
| SCH-04 | C10.6 | Monitoring-interval engine: follow-up window after each therapy change (lipids 4-12 wk, A1c 2-3 mo, thyroid 6-12 mo) and whether it happened | tblSupp dates, labs | MINIFS date windows | M | BN | GATE-DATES | DIP ch37:451, ch97:197, ch98:345 |
| SCH-05 | Q70, Q71, L-G13 | Retest cadence by volatility and minimal meaningful change, value-of-information ranking for the next draw, due/overdue colouring | labs, RCV proxy, test cost (missing: weight 1) | n=((1.96*SD)/delta)^2; next_due | M | BN | | volatility-managed-portfolios; GOOD; LIFE l.10646; OUT |
| SCH-06 | C5.1, C5.11 | Pre-draw planner: last biotin-containing dose date and countdown per upcoming draw, per past panel "biotin in 48 h" flag, fasting 10 h rule, prep checklist | tblSupp (biotin mcg), draw dates | SUMIFS on draw_date-2; 5,000 mcg from DIP | M | BN | TOP; GATE-FIELDS | DIP ch98:233; KAT ch46:77; FDA 2017/2019 (EXT) |
| SCH-07 | C6.1, L-E11 | Dosing calendar grid (morning/noon/evening/bedtime/with-food) plus absorption rules (D3 with largest meal, stimulants away from bed), printable | tblSupp time_of_day | FILTER/XLOOKUP grid; print area | M | BN | TOP | KAT ch81:91; GREG ch35; HELMS ch02 |
| SCH-08 | C6.2, Q55 | Pill burden and regimen-complexity score, slot crowding heatmap, turnover per quarter trend | tblSupp | SUMPRODUCT; COUNTIFS | M | BN | GATE-DATES | KAT ch81:91,147-161; PAL Ch5.3.1; DIP ch29:21 |
| SCH-09 | C6.3 | Medication/supplement Gantt coloured by type with lab panel dates | tblSupp dates | invisible-offset stacked bar | M | BN | GATE-DATES | DIP ch07 |
| SCH-10 | C6.5, Q57 | Refill/run-out calendar and monthly cash-outflow forecast | tblProducts, last purchase | purchase + units/use; EOMONTH buckets | S | BN | GATE-FIELDS | KAT ch81:147 |
| SCH-11 | C5.9 | Chelation/separation windows in the dosing grid (same-slot conflicts) | tblSupp, rule table | COUNTIFS conflicts; hours UNVERIFIED | M | NS | EXT | KAT ch82; WHO ch00:1144 |
| SCH-12 | L-E10, Q87 | Morning-routine Gantt with planned vs actual slippage, supplement doses, light/caffeine/exercise overlays and a night plan | tblRoutine | stacked bar offset series | M | BN | TOP | FO ch17; WWS tips 2,9,11; OUT ch60 |
| SCH-13 | C11.1, C3.2, C11.3, C11.2 | Visit pack: SOAP one-pager auto-filled from latest panel and alerts, POMR problem list, questions-for-my-doctor generator, medication reconciliation | Calc_Marker, tblSupp, Alerts | XLOOKUP/TEXTJOIN; print layout | M | BN | | DIP ch07:105-119; KAT ch81 |
| SCH-14 | C10.7, C11.4 | Low priority: travel health checklist, geriatric/Beers stopping review (titles only in source) | entry | static | S | PH | weak | DIP e17 (title), ch29:21,39 |

### W8 Global dashboard, Checks, navigation (GLB)

| ID | Aliases | Summary | Inputs | Build notes | Eff | St | Tags | Sources |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| GLB-01 | Q92, L-G4 | Executive dashboard: KPI strip (health index, % optimal, alerts, spend/day, weight trend kg/wk, streak) and Outlive ten-tile scorecard with sparklines and traffic lights; PH tiles show "awaiting data" | `K_*` names, Calc_Marker | big-font cells, icon sets, sparklines | M | BN | TOP | FO Ch13; KAU Ch23; OUT ch13-15,30-35,58-60 |
| GLB-02 | Q89 | Checks sheet: missing dates, duplicates, unit consistency, ref range present per marker, sorted panels, formula-error counts (`SUMPRODUCT(--ISERROR)`), reconciliation totals, count of PH/NS sheets, single "ALL CHECKS PASS" | all tables | COUNTBLANK; ISERROR sweep | S | BN | TOP | KAU Ch22; PAL Ch8.1.2 |
| GLB-03 | Q66 | Tiered alerts (watch/action/critical) per marker plus live sorted alert list | Calc_Marker | status 0-3; FILTER | S | BN | MED-VERIFY (critical limits blank unless sourced) | KAU Ch23 |
| GLB-04 | Q94 | What changed since last panel: top movers in z and RCV units, new alerts, supplements started/stopped in interval | last two panels, tblSupp | SORTBY/TAKE; TEXTJOIN | M | BN | | PAL Ch14 |
| GLB-05 | Q95 | Scenario/stress sheet: shock a marker or weight and see derived ratios and alerts flip | Calc_Marker | shock input cells; before/after | S | BN | | KAU Ch22 |
| GLB-06 | L-G16 | Year-in-review: per-year medians of key markers, weight, spend; deltas; concatenated narrative cells | tblLabs, tblDaily, tblSupp | MEDIAN by year | M | BN | | derived |
| GLB-07 | L-G14 | LLM export "book on me": flat sheet of demographics, last panel, trends, gaps, genome flags, stack, goals, with redaction column | many | flat INDEX table | S | BN | privacy | TW 2019818347676983531, 2106537633056969024 |
| GLB-08 | NEW | Nav/index sheet with hyperlinks per workstream and status badges (BN/PH/NS), global disclaimer, provenance/Sources register, resources and sizes reference tables (inputs otherwise unused) | tblResources, tblSizes, module registry | internal links; badges from registry | S | BN | | health-cli store |

### Absorbed into shared infra (not separate rows)
- Q90 Parameter sheet (infra 4). C1.3 Unit/assay harmonisation sheet (infra 1, Ref_Units). Q7's lab/method/fasting columns live in tblLabs; LAB-15 only flags. C10.3 (vaccine storage) skipped as source says not needed.

## 4. Open source-verification items before NS can flip to BN
- EFLM Biological Variation Database / Fraser: CVa, CVi per marker (LAB-13, SCH-05).
- Levine 2018 Supplement 1: intercept, gamma, M and age conversion constants (AGE-01..03).
- Inker 2021 NEJM CKD-EPI coefficients (CLN-14); Sterling 2006 FIB-4 (CLN-16); Matthews 1985, Simental-Guerrero 2008 (CLN-11); Friedewald, Martin-Hopkins (CLN-03); Nathan 2008 eAG (CLN-10); Figge, corrected Ca (CLN-13, CLN-20).
- NIH ODS UL table (SUP-10); NCEP ATP III/AHA criteria (CLN-21); CDC adult schedule (SCH-01); USPSTF (SCH-02); SNPedia/ClinVar for every rsID and allele table (GEN-01, GEN-05, GEN-08..10); half-lives (SUP-14); Tanaka 2001, Uth 2004, FRIEND/Cooper norms (AGE-12, AGE-14); gnomAD frequencies (GEN-08).
- Follow-up ingest bead (from clinical.md): Tietz/Henry's, USPSTF A/B, CDC adult schedule, CPIC, EFLM biological variation.

## 5. Counts (script-verified: all 322 raw ids appear as aliases; Q90 and C1.3 absorbed into infra, C10.3 skipped)

Per status (163 master ideas): BN 106, PH 37, NS 20. 25 rows carry GATE-DATES or GATE-FIELDS (any status) and degrade to PH if the field check (A2/A3) fails: LAB 5, CLN 4, SUP 7, AGE 2, BOD 2, SCH 5.

Per workstream (total: BN / PH / NS):
- W1 Lab tear sheets, stats, experiments: 35 (34 / 0 / 1)
- W2 Clinical derived indices and flags: 25 (10 / 8 / 7)
- W3 Supplement portfolio, cost, interactions, evidence: 23 (17 / 3 / 3)
- W4 Aging clocks, longevity, fitness: 18 (8 / 9 / 1)
- W5 Body, nutrition, sleep, wearables: 30 (18 / 12 / 0)
- W6 Genetics cross-cuts: 11 (2 / 4 / 5)
- W7 Schedules, routine, vaccines, prevention: 14 (10 / 1 / 3)
- W8 Global dashboard, Checks, navigation: 8 (8 / 0 / 0)
