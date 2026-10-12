# Clinical / lab-medicine / pharmacology / evidence view catalog for the Health workbook (bead hx-rx3.4)

Scope: visualisation of David's own data. Nothing here is medical advice. Ideas marked **[DX-RISK]** drift toward diagnosing (staging, phenotype calls, "you have X"); build them as neutral pattern flags with wording like "pattern consistent with... discuss with clinician", never as a verdict.

## 0. How to read this / sourcing rules

Path abbreviations (all under `~/Knowledge`):

- **KAT** = `Stacks/R Medicine/RM Therapeutics. Pharmacology/RM300-666 Drugs and their actions/RM300 General works/Basic and Clinical Pharmacology 15th ed - Katzung Trevor/chNN.md` (Katzung chapter N is file ch(N+5): Ch3 PK = ch09, Ch5 pharmacogenomics = ch11, Ch35 lipids = ch46, Ch65 supplements = ch80, Ch67 interactions = ch82, Appendix vaccines = ch83).
- **DIP** = `Stacks/R Medicine/RM Therapeutics. Pharmacology/RM260-263 Chemotherapy/RM263 General special/Pharmacotherapy A Pathophysiologic Approach 11th ed - DiPiro Yee Posey/chNN.md` (Ch31 dyslipidemia = ch37, Ch61 CKD = ch67, Ch65 renal dosing = ch71, Ch69 acid-base = ch75, Ch91 diabetes = ch97, Ch92 thyroid = ch98, Ch109 gout = ch115, Ch118 anemias = ch124, Ch142 vaccines = ch148, Ch158 nutrition assessment = ch164, Ch161 obesity = ch167, e5 PK = ch11, e6 pharmacogenetics = ch12, e23 older adult = ch29, e59 kidney function = ch65, Ch1 patient care process = ch07).
- **GG** = `.../RM300 General works/Goodman and Gilman Pharmacological Basis of Therapeutics 13th ed - Brunton/chNN.md` (Ch7 pharmacogenetics = ch14, Ch33 dyslipidemias = ch42, Appendix II PK data = ch88).
- **OUT** = `Medicine/Books/Peter Attia MD - Outlive_ The Science and Art of Longevity-Harmony (2023)/chNN.md`.
- **EV** = `General/Evidence and Research Methodology/Papers/` (GRADE-1 = Guyatt 2011, GRADE-3 = Balshem 2011, RoB2 = Sterne 2019, AMSTAR2 = Shea 2017, IOA = Ioannidis 2005, BUT = Button 2013).
- **WHO** = `Medicine/Books/WHO Model List of Essential Medicines/ch00.md` (21st list, 2019).
- **CIP** = `Medicine/Reports/2026-10-03-ciprofloxacin-pregnancy.org`.
- **SR-papers** = the primary papers filed under `Stacks/R Medicine/` (vitamin D, ashwagandha, Bacopa, omega-3, NMN, nicotinamide, metformin, citicoline).

Honesty flag: the library is much thinner than the brief assumed. There is NO clinical chemistry / lab-medicine textbook, no Harrison's/UpToDate, no screening-guideline document, and the Katzung/DiPiro/GG tables were stripped to "formula"/images in the markdown (equations and many tables are lost; only prose survives). So:

- Where a number or formula is in the library prose, it is cited with chapter file and line.
- Where I use a standard formula/threshold that is NOT in the library, it is tagged **[EXT]** with the name of the standard source to verify. Those must be checked before shipping; they are not "from David's library".
- Page numbers are not available (markdown conversion); I give file + line number (`ch98.md:233`).

---

## 1. Lab-medicine statistics on serial panels (7 panels, 2018-2025)

### 1.1 Reference Change Value (critical difference) per marker
- Shows: for each marker, is the change between two consecutive panels bigger than analytic + biological noise? Green "real change", grey "within noise".
- Inputs: value_t1, value_t2 per marker; CV_analytic and CV_within-person (biological variation) per marker (a small lookup sheet).
- Excel: `=ABS(t2-t1)` vs `RCV = 2^0.5 * 1.96 * SQRT(CVa^2 + CVi^2)` in % (`=1.414*1.96*SQRT(CVa^2+CVi^2)`); `=IF(ABS((t2-t1)/t1*100)>RCV,"real","noise")`; conditional-format icon set.
- Why: stops over-reading 5% wiggles in ApoB, ALT, ferritin.
- Effort: M (needs the CVi table).
- Source: concept of serial testing and "no single test is 100% accurate; interpret trends" in OUT ch22.md:19-21; the RCV formula itself is **[EXT]** (Fraser, Harris; EFLM Biological Variation Database) - not in library. DIP ch11.md Key Concepts note clearance-dependent variability only for PK.

### 1.2 Delta check flag (impossible-jump detector)
- Shows: flags a value that jumped more than X% from the previous panel, as a "possible transcription/unit/lab-change error" before it is trusted.
- Inputs: previous and current value, per-marker delta limit (e.g. 50% for creatinine). 
- Excel: `=IF(ABS(cur/prev-1)>limit,"CHECK","")`, plus a "lab changed" column from the lab-name field.
- Why: 2018-2025 means multiple labs and units (mg/dL vs mmol/L).
- Effort: S.
- Source: delta-check practice **[EXT]**; assay interference can fake jumps (DIP ch98.md:233, see 5.1).

### 1.3 Unit and assay-method harmonisation sheet
- Shows: one canonical unit per marker, conversion factor, original unit, original lab, assay method; all charts read canonical.
- Inputs: raw value, raw unit, lab, method.
- Excel: `XLOOKUP(marker&unit, convTable)`; `=value*factor`; data validation list for units.
- Why: the book explicitly shows units both ways (mg/dL vs mmol/L: TSH mIU/L, LDL 190 mg/dL = 4.91 mmol/L; serum urate <6 mg/dL = <357 umol/L).
- Effort: S.
- Source: DIP ch37.md:343 (LDL-C 190 mg/dL = 4.91 mmol/L; non-HDL 220 = 5.69), DIP ch115.md:21 (6 mg/dL = 357 umol/L); also anion gap instrument note below.

### 1.4 Reference range vs optimal range vs "personal baseline" band chart
- Shows: line chart per marker with three shaded bands: lab reference range, David's "optimal" range (already in the data), and +-1 SD / personal baseline from his own panels.
- Inputs: ref_low/ref_high, opt_low/opt_high, values by date.
- Excel: stacked-area helper columns for bands + line series (rust_xlsxwriter combined chart); personal band `=AVERAGE(range)+-STDEV.S(range)`.
- Why: Attia's central argument is that population reference ranges are too permissive ("target LDL 100/70 is still too high"); showing both is the product thesis.
- Effort: M.
- Source: OUT ch15.md:9,15 (guideline targets vs "as low as possible"); DIP ch37.md:299 ("decision ... based on ASCVD risk and not merely plasma levels").

### 1.5 Time-in-range per marker
- Shows: % of panels in reference vs optimal range, per marker, as a bullet/bar column.
- Inputs: values, ranges.
- Excel: `=COUNTIFS(vals,">="&lo,vals,"<="&hi)/COUNT(vals)`; data bar.
- Why: single "how often am I in range" number, mirrors CGM "time in range".
- Effort: S.
- Source: concept transfer from CGM discussion OUT ch49.md; A1c as exposure over 2-3 months DIP ch97.md:197.

### 1.6 Out-of-range heatmap (markers x panel dates)
- Shows: 56 x 7 grid, colour = distance outside range in units of range width (or z).
- Inputs: values + ranges.
- Excel: helper grid `=(v-hi)/(hi-lo)` or `(lo-v)/(hi-lo)`; 3-colour-scale conditional format; frozen panes; group by organ system.
- Effort: S.
- Source: presentation method; pre-clinical drift rationale OUT ch13.md:186 (monitor early biomarkers before textbook thresholds).

### 1.7 Slope and projected crossing date
- Shows: linear trend per marker (SLOPE, per year) and the date the trend would cross the upper/lower reference limit; flags "crossing within 2 years".
- Inputs: dates, values.
- Excel: `=SLOPE(vals,dates)*365`, `=FORECAST.LINEAR`, `=lo_limit` crossing: `=(limit-INTERCEPT)/SLOPE`; sparkline per row.
- Why: Attia: disease declared at a threshold is a late event; watch trajectory ("A1c 6.5 ... may already be elevated risk").
- Effort: M.
- Source: OUT ch13.md:182-186; DIP ch97.md:199 ("Patients who have even minor increases in glucose or A1C over time ... likely progress").

### 1.8 Regression-to-the-mean / single-outlier guard
- Shows: when one panel is an outlier vs the others, suggest a repeat before acting.
- Inputs: values.
- Excel: `=ABS(last-MEDIAN(others))/MAD(others)`; flag > 3.
- Effort: S. Source: Attia "stack tests; no single test is 100% accurate" OUT ch22.md:19-21; Button "small samples -> unstable estimates" EV BUT (median power 21%).

### 1.9 Panel completeness matrix
- Shows: which of the 56 markers were measured on which panel (gaps: e.g. ApoB only on recent panels, Lp(a) never).
- Inputs: marker x panel presence.
- Excel: `=COUNTIF` grid; conditional format blanks; "overdue marker" = months since last measure.
- Why: Attia's "first draw" list = ApoB and Lp(a) (Lp(a) once in life); highlights never-measured markers.
- Effort: S. Source: OUT ch15.md:7, ch14.md:163.

---

## 2. Derived ratios and indices (computed from inputs David already has)

All are "calculated columns" in a Derived sheet, structured table with formulas.

### 2.1 Triglyceride / HDL ratio
- Shows: TG/HDL per panel, bands <1 (best), <2 (ok), >=2.
- Inputs: TG, HDL (mg/dL).
- Excel: `=TG/HDL`; bands via conditional format.
- Why: Attia's early metabolic-dysfunction proxy.
- Effort: S.
- Source: OUT ch13.md:186 ("ratio of triglycerides to HDL ... should be less than 2:1 or better yet less than 1:1").

### 2.2 Non-HDL cholesterol
- Shows: non-HDL per panel vs ApoB-derived expectation.
- Excel: `=TC-HDL`; FH flag if >=220 mg/dL at untreated.
- Source: DIP ch37.md:343 (non-HDL >=220 mg/dL suspicion of FH); definition of non-HDL = TC - HDL is **[EXT]** arithmetic.
- Effort: S. **[DX-RISK]** for the FH flag - label "pattern".

### 2.3 ApoB panel: concordance with LDL-C
- Shows: scatter ApoB vs LDL-C (and non-HDL) with regression; discordant panels highlighted (ApoB high while LDL-C looks fine).
- Inputs: ApoB, LDL-C, TC, HDL, TG.
- Excel: XY scatter + `SLOPE/RSQ`; residual column `=ApoB - (slope*LDL+intercept)`; flag |residual|>1 SD.
- Why: Attia: apoB (particle count) is more predictive than LDL-C (cholesterol content); his first two numbers.
- Effort: M.
- Source: OUT ch15.md:7; DIP ch37.md:191 (labs: elevated apoB), DIP ch37.md:107-109 (ApoB/ApoA1 lipoprotein biology); KAT ch46.md:39 (B-100 on VLDL, IDL, LDL).

### 2.4 Lipid-lowering "dose-response" overlay
- Shows: LDL-C/ApoB over time with vertical event lines for statin / ezetimibe / niacin / fish oil / fiber / sterol starts (from the supplement table's start dates).
- Inputs: lipid values, intervention start/stop dates (supplements + Rx).
- Excel: scatter with extra series for events (date, ymax) as drop lines; annotation via data labels from a name column.
- Why: tests whether an intervention moved the marker.
- Effort: M.
- Source: DIP ch37.md:451 (repeat lipid panel 4-12 weeks after starting/adjusting therapy); expected effect sizes: fiber (3-12 g/day, ~10 mg/dL TC/LDL reduction, ch37.md:327), phytosterols 2 g/day -> LDL-C -5% to -15%, no extra above 3 g/day (ch37.md:331), PCSK9i up to -60% (ch37.md:387).

### 2.5 "Expected vs observed" LDL change after an intervention
- Shows: table: intervention, expected % change (from book), observed % change between pre/post panels, verdict.
- Excel: `=(post-pre)/pre`; lookup expected from an interventions table (values only where the book gives them: phytosterols 5-15%, PCSK9i ~60%, mipomersen ~25%, lomitapide ~40% [HoFH]).
- Source: DIP ch37.md:331, 345, 387. Effort: S.

### 2.6 Remnant / VLDL proxy
- Shows: TG/5 (Friedewald VLDL proxy) when TG <400.
- Source: **[EXT]** Friedewald 1972; Attia monitors VLDL (OUT ch13.md:186). Effort: S.

### 2.7 Castelli / TC:HDL ratio
- Excel `=TC/HDL`. Source **[EXT]** (Framingham). Effort: S. Low value vs ApoB; include only in the "traditional" lipid tab.

### 2.8 Neutrophil-to-lymphocyte ratio (NLR) and platelet-lymphocyte ratio
- Shows: NLR over time from the CBC differential.
- Excel `=Neut_abs/Lymph_abs`.
- Why: inflammation/stress index; also lymphocyte count < 1,200/mm3 as a nutrition marker.
- Source: total lymphocyte count <1,200 = nonspecific malnutrition marker DIP ch164.md:297; NLR itself **[EXT]**. Effort: S.

### 2.9 Anion gap (with sourced reference range)
- Excel: `=Na-(Cl+HCO3)` (K optional); bands: normal ~9, range 3-11 mEq/L; >17-20 elevated; >20-25 suggests organic acidosis.
- Why: CMP-derived, no extra test. Instrument-era note: older cut-off 12 is outdated.
- Source: DIP ch75.md:199-211 (definition; "normal SAG ~9, range 3 to 11"; "values in excess of 17 to 20"), ch75.md:243 ("when the SAG exceeds 20 to 25 a significant organic acidosis is likely"; caveat "mild elevations cannot be automatically attributed"). Effort: S. **[DX-RISK]**: display number + band only.

### 2.10 Albumin-adjusted anion gap
- Excel: `=AG+2.5*(4.4-Alb)`.
- Source: **[EXT]** (Figge). DIP ch75.md:243 only says protein/alkalemia alters gap. Effort: S.

### 2.11 BUN : creatinine ratio
- Excel `=BUN/Cr`. Source **[EXT]**; DIP ch164.md:83 lists BUN/SCr within nutrition labs. Effort: S.

### 2.12 Estimated GFR (CKD-EPI 2021) and creatinine clearance (Cockcroft-Gault)
- Shows: eGFR per panel; CKD category G1-G5 colour band; optional CG mL/min for drug-dosing context.
- Inputs: serum creatinine, age at panel date, sex (weight for CG); optional cystatin C.
- Excel: LET/LAMBDA closed-form CKD-EPI 2021 (`=142*MIN(Scr/k,1)^a*MAX(Scr/k,1)^-1.200*0.9938^age*(1.012 if F)`) with k, a by sex.
- Why: ties directly to creatine-supplement confounding (see 5.2) and to renal dose adjustment.
- Source: DIP ch65.md Key Concepts l.19-25 (GFR best single indicator; uACR critical; equations incorporate Scr, cystatin C, age, sex); DIP ch65.md:39 (reported eGFR vs Cockcroft-Gault used in FDA labels); DIP ch71.md:149 (estimating equations for dosing); CKD staging by eGFR AND uACR DIP ch67.md:173 (KDIGO). Equation coefficients: **[EXT]** (Inker 2021 NEJM) - the book's equation images were not preserved. Effort: M. **[DX-RISK]** for CKD stage labels.

### 2.13 Albumin:creatinine (uACR) risk-grid
- Shows: 2D KDIGO heat-map (eGFR row x uACR column) with David's point plotted.
- Source: DIP ch67.md:173; ch65.md l.19. Only if uACR exists in data. Effort: M. **[DX-RISK]**.

### 2.14 HOMA-IR / QUICKI / TyG index
- Excel: `=(FastInsulin*FastGlucose)/405` (mg/dL); `TyG=LN(TG*Glu/2)`.
- Inputs: fasting insulin + glucose (if insulin exists; if not, TyG uses TG+glucose only).
- Why: Attia: fasting/post-load insulin is "the canary in the coal mine"; HbA1c "not very specific by itself".
- Source: OUT ch13.md:186-190; formulas **[EXT]** (Matthews 1985; Simental-Guerrero 2008). Effort: S. **[DX-RISK]** (insulin resistance label).

### 2.15 OGTT insulin-curve plot (if he ever does the Attia test)
- Shows: glucose and insulin at 0/30/60/90/120 min after 75 g glucose; insulin peak at 30 min and persistence flagged.
- Source: OUT ch13.md:188-190 (75 g Glucola; measure glucose AND insulin every 30 min for 2 h; early insulin resistance shows dramatic 30-min rise, then stays elevated). Effort: S (template tab).

### 2.16 eAG from A1c, and A1c/FPG discordance
- Excel `=28.7*A1c-46.7`; compare to measured FPG.
- Source: **[EXT]** (Nathan 2008 ADAG). Diagnostic cut-points FPG >=126, A1c >=6.5% in DIP ch97.md:23; A1c = 2-3 month exposure ch97.md:197. Effort: S. **[DX-RISK]**.

### 2.17 Iron-status quartet with sourced bands
- Shows: ferritin, TSAT, MCV, RDW, Hb on one panel with "pattern" flags.
- Excel: `TSAT=SerumIron/TIBC*100` (formula stated in text); bands: TSAT normal 20-50%, <=15% typical IDA; ferritin = best marker of stores but is an acute-phase reactant (normal/high may mask deficiency in inflammation).
- Source: DIP ch124.md:249-257 (TSAT formula, bands, ferritin acute-phase caveat); MCV micro/macro/normocytic ch124.md:217; RDW ch124.md:233; reticulocyte index ch124.md:229. Effort: M. **[DX-RISK]**.

### 2.18 Mentzer index (MCV/RBC)
- **[EXT]** only. Skip unless thalassaemia trait is in the genome (HBB). Effort: S.

### 2.19 Corrected calcium
- `=Ca+0.8*(4-Alb)` **[EXT]**. Effort: S. Relevant to calcium+vitamin D supplements (see 5.x).

### 2.20 De Ritis (AST/ALT) and FIB-4
- `=AST/ALT`; `FIB4=(Age*AST)/(Plt*SQRT(ALT))`. **[EXT]** (Sterling 2006). Attia watches mildly elevated ALT as early metabolic sign (OUT ch13.md:186). Effort: S. **[DX-RISK]**: FIB-4 is a fibrosis screening score; show as a number only.

### 2.21 Uric acid vs gout/solubility lines
- Shows: serum urate with horizontal lines at 7 mg/dL (solubility limit) and treat-to-target 6 (and 5 for tophaceous).
- Source: DIP ch115.md:21 (target <6 mg/dL; lower for tophi), ch115.md:51 (>7 mg/dL begins to exceed solubility), ch115.md:107 (asymptomatic hyperuricemia generally needs no therapy -> include this caveat text). Attia tests uric acid because it drives fat storage/BP (OUT ch13.md:150-164). Effort: S.

### 2.22 Thyroid axis panel (TSH vs FT4 scatter) with biotin overlay
- Shows: TSH and FT4/T3 together, flagged when TSH below range with normal FT4/T3 ("subclinical pattern") and when biotin >5,000 mcg/day was in the stack in the preceding days (assay artefact possible).
- Source: DIP ch98.md:347 (subclinical hyperthyroidism definition), ch98.md:111 (TSH <0.1 mIU/L), ch98.md:233 (biotin >5,000 mcg/day interferes; false-high TT4/FT4/TT3, false-low TSH; hold 24-48 h). Effort: M. **[DX-RISK]**.

---

## 3. Organ-system and problem-oriented views

### 3.1 Organ-system panel dashboard
- Shows: tiles per system (cardiometabolic, glycemic, renal, hepatic, thyroid, haematologic/iron, inflammation, nutrients, bone/mineral) with N markers, % in optimal range, trend arrow.
- Inputs: marker->system mapping (lookup), values, ranges.
- Excel: `COUNTIFS` by system; sparkline columns; tile formatting.
- Source: organ-system organisation mirrors Pharmacotherapy section structure (cardiovascular Ch30-41, renal Ch59-65, endocrine Ch91-93, haematology Ch118) DIP index; the DiPiro chapters above for each system's labs. Effort: M.

### 3.2 Problem-oriented medical record (POMR) sheet
- Shows: one row per active "problem" with indication, current therapy (Rx + supplements), monitoring marker, last value, goal, next monitoring date.
- Inputs: problem list (e.g. dyslipidemia, vitamin D insufficiency), linked supplements, linked markers.
- Excel: structured table + XLOOKUP into the Labs/Supplements tables.
- Why: DiPiro's patient-care process is organised as a problem list; assess "indication, effectiveness, safety, adherence, in this order".
- Source: DIP ch07.md:105,111 (problem list; assess indication -> effectiveness -> safety -> adherence in this order), ch07.md:119 (most common medication-therapy problems: "needs additional therapy", "dose too low", "adherence"). Effort: M.

### 3.3 Indication-effectiveness-safety-adherence audit per supplement
- Shows: for each of the 87 supplements four traffic lights.
- Inputs: indication (text), effect marker + direction expected, safety flag (interaction count), adherence (dose log completeness).
- Excel: four computed columns + COUNTIFS; sort by worst.
- Source: DIP ch07.md:111. Effort: M.

### 3.4 "Marker has no matching intervention / intervention has no matching marker" gap analysis
- Shows: supplements whose rationale marker is never measured (e.g. Bacopa -> nothing to measure) and abnormal markers with no intervention.
- Excel: anti-join with `FILTER`/`COUNTIF`.
- Source: "needs additional therapy" and "unnecessary therapy" categories DIP ch07.md:119. Effort: S.

### 3.5 Cardiometabolic risk-enhancer checklist (non-score)
- Shows: checklist of Attia's priority numbers: ApoB, Lp(a) (once in a lifetime), TG/HDL, fasting insulin, uric acid, homocysteine, ALT, hsCRP, with measured/not-measured and last date.
- Source: OUT ch13.md:186; ch15.md:7; ch14.md:163; DIP ch37.md:191 (hsCRP, apoB), KAT ch46.md:169 (homocysteine; folic acid + B vitamins; betaine). Effort: S.

### 3.6 Familial-hypercholesterolemia / Lp(a) family-history flag
- Shows: flags when LDL-C >=190 or non-HDL >=220 untreated AND family history of premature ASCVD; prompts Lp(a).
- Source: DIP ch37.md:343 (thresholds; FH 24-fold higher MI risk <40); OUT ch14.md:161-163 (family history of premature heart disease -> Lp(a)). Effort: S. **[DX-RISK]**.

### 3.7 Lifetime apoB exposure ("area under apoB curve") chart
- Shows: cumulative apoB-years (trapezoid integral of ApoB/LDL over age) - Attia's point that risk scales with exposure over time.
- Excel: `=SUMPRODUCT((x2-x1)*(y1+y2)/2)`; line + area chart by age.
- Source: OUT ch15.md:57-59 ("risk is proportional to apoB exposure over time"; Sniderman 30-year model). Concept only; the integral is arithmetic. Effort: M.

### 3.8 Metabolic-syndrome criteria counter
- Shows: how many of 5 criteria (waist, TG, HDL, BP, glucose) are met; show "years before textbook definition" using TG/HDL.
- Source: concept in OUT ch13.md:186 (biomarkers show "many years before a patient would meet the textbook definition of metabolic syndrome"); the 5 criteria cut-offs **[EXT]** (NCEP ATP III/AHA). Effort: S. **[DX-RISK]**.

### 3.9 Kidney-safe protein ceiling
- Shows: daily protein vs bands: RDA 0.8 g/kg, Attia minimum 1.6, start 2.2 for active, upper limit 3.7 g/kg; overlay eGFR.
- Inputs: daily protein (g), weight, eGFR.
- Source: OUT ch50.md:9,13,17 (0.8 g/kg RDA; 1.6 minimum; ~2.2 g/kg start; 3.7 g/kg upper limit; "normal kidney function"). Effort: S. Direct fit: David logs daily protein Nov 2023-Jul 2024.

---

## 4. Weight, energy and nutrition views (daily weight/calories/protein)

### 4.1 Weekly rate of weight change with guard-rails
- Shows: 7-day rolling mean weight; rate in kg/week and % body weight/week; flags >1%/wk (muscle-loss risk).
- Excel: `=AVERAGE(OFFSET(...,-6,0,7,1))` or `AVERAGE(INDEX(...):INDEX(...))`; `=(avg_t - avg_t-7)/avg_t-7`; sparkline.
- Source: 5% weight loss improves BP, lipids, glucose DIP ch167.md:19; weight-loss targets framed as 5-10% DIP ch37.md:319; refeeding-syndrome risk when unintentional loss >5-10% DIP ch164.md:89; daily weight fluctuates with fluid DIP ch164.md:229. The 1%/week figure is **[EXT]** (Helms; Pyramid Nutrition book exists in `Medicine/Nutrition/Books/The Muscle and Strength Pyramid Nutrition - Helms Morgan Valdez` - not opened). Effort: M.

### 4.2 Cumulative % weight lost vs the "5% clinically meaningful" line
- Source: DIP ch167.md:19 (5% -> measurable BP/lipid/glucose gains), ch167.md:29 (stop drug if <5% at 12 weeks max dose). Effort: S.

### 4.3 BMI and waist band with sourced classes
- Excel: `=kg/m^2`; classes <18.5, 18.5-24.9, 25-29.9, 30-39.9, >=40; waist circumference as second risk axis; refeeding risk BMI <18.
- Source: DIP ch164.md:239-245 (BMI formula both unit systems; NIH classes; waist circumference measured below rib cage/above iliac crest), ch164.md:89. Effort: S.

### 4.4 Calorie balance and energy-model fit
- Shows: actual weight change vs predicted from logged calories (7,700 kcal/kg heuristic) -> back-calculated TDEE.
- Source: energy equations/kcal-per-kg methods DIP ch164.md:401-409 (kcal/kg and indirect calorimetry; examples); 7,700 kcal/kg **[EXT]**. Effort: M.

### 4.5 Protein per kg adherence and per-meal distribution
- Source: OUT ch50.md:13 (1.6-2.2 g/kg/day). Effort: S.

### 4.6 Nutrition-status lab bundle
- Shows: albumin, transferrin, prealbumin, total lymphocyte count, CRP together with their half-lives to explain sensitivity (prealbumin 2-3 d, transferrin 8-9 d, albumin long) and "interpret with CRP".
- Source: DIP ch164.md:269-297 (labs must be read in context of inflammation; transferrin half-life 8-9 d, prealbumin 2-3 d; TLC <1,200). Effort: S.

### 4.7 DRI / UL ledger for micronutrients from the supplement stack
- Shows: daily total of each nutrient (vitamin D, zinc, selenium, manganese, iodine, niacin, B6...) from all 87 supplements + diet vs RDA/UL; flag overshoot.
- Inputs: per-product nutrient amounts.
- Excel: `SUMIFS` across a long-format Product_Nutrient table; `=total/UL`; red above 100%.
- Source: DRI categories EAR/RDA/AI/UL defined DIP ch164.md:387; toxicity examples (manganese >500 mcg/day in long-term PN) ch164.md:343; selenium labs ch164.md:327; vitamin/mineral list in WHO ch00.md:4346 (ascorbic acid 50 mg, colecalciferol 400/1000 IU, nicotinamide 50 mg, pyridoxine 25 mg, riboflavin 5 mg, thiamine 50 mg, iodine, retinol). UL numeric values **[EXT]** (NIH ODS/IOM). Effort: L.

### 4.8 Glycemic-response / food-spike table (if foods have GI/GL)
- Source: Medicine/Nutrition/Books/Glucose Revolution - Inchauspe (not opened) and OUT ch49.md ("Lessons from CGM"). Effort: S. Weakly supported.

---

## 5. Supplement-drug and supplement-lab interactions (the high-value block)

### 5.1 Biotin-before-blood-draw washout planner
- Shows: for every upcoming lab date, the last-dose date for biotin-containing items and a countdown; per past panel, "biotin dose in the 48h before draw" flag so thyroid/other immunoassay results are annotated.
- Inputs: supplement table (biotin mcg/day, items containing biotin incl. multivitamins/hair formulas), lab dates.
- Excel: `=SUMIFS(biotin_mcg, supp, taking_on, draw_date-2)`; `IF(>5000,"hold 24-48 h")`; annotate TSH/FT4 cells.
- Why: false-high T4/T3, false-low TSH on competitive/sandwich assays.
- Source: DIP ch98.md:233 (">5,000 mcg/day associated with major interference ... hold 24 to 48 hours before"). Other assays affected (troponin, PTH, vitamin D, ferritin on streptavidin-biotin platforms) are **[EXT]** (FDA safety communication 2017/2019). Effort: M.

### 5.2 Creatine vs creatinine/eGFR confounder flag
- Shows: if a creatine product was active in the 4-8 weeks before a panel, add a footnote on creatinine/eGFR and prefer cystatin C if available.
- Source: eGFR equations use serum creatinine, optionally cystatin C DIP ch65.md l.25; the creatine-creatinine mechanism is **[EXT]** (library has no source; do not state as library fact). Effort: S.

### 5.3 Supplement-drug interaction matrix (87 x Rx)
- Shows: grid of supplements (rows) x drugs David takes/has taken (columns) with severity colour and mechanism code (PK-induction, PK-inhibition, PD-additive, chelation).
- Inputs: curated interaction table (supplement, object drug, mechanism, source).
- Excel: SUMPRODUCT/XLOOKUP into a matrix; 3-colour format; slicer-like data validation.
- Seed content with sourced entries: St John's wort -> PXR/CYP3A4/2C9/1A2 + P-gp induction, lowers digoxin, OCPs, cyclosporine, warfarin, theophylline, anticonvulsants (KAT ch80.md:243); garlic/ginkgo/ginseng antiplatelet + aspirin/NSAID/warfarin (ch80.md:109,143,175); CoQ10 may reduce warfarin effect (ch80.md:297); glucosamine raises INR (ch80.md:321); melatonin metabolised by CYP1A2, fluvoxamine inhibits; possible nifedipine BP rise (ch80.md:355); echinacea avoid in immunosuppression (ch80.md:77); ginseng + hypoglycemics/psychiatric drugs (ch80.md:177).
- Why: the book's own case question - "Are there any supplements that could increase bleeding risk with ibuprofen?" - is exactly this view.
- Source: KAT ch80.md:489 (worked case); KAT ch82.md (Ch67) mechanisms and Table 67-1 (table body lost in conversion). Effort: L.

### 5.4 Interaction mechanism taxonomy filter
- Shows: filter the matrix by mechanism: absorption (adsorption, chelation, pH, motility, transporters), distribution, metabolism (induction/inhibition), renal excretion, pharmacodynamic (additive/synergistic/antagonistic), combined toxicity.
- Source: KAT ch82.md section headings (PHARMACOKINETIC MECHANISMS / PHARMACODYNAMIC MECHANISMS / COMBINED TOXICITY). Effort: S.

### 5.5 Time-to-effect window for inducers/inhibitors
- Shows: for inducers (St John's wort, rifampin, etc.) the calendar bar 7-14 days to full effect and an equal or longer time to dissipate after stopping; for inhibitors "begins as soon as the tissue concentration is high enough".
- Excel: Gantt-style conditional format: `=AND(date>=start,date<=start+14)` etc.
- Source: KAT ch82.md:65 ("Enzyme induction ... maximal effects usually occur after 7-14 days and require an equal or longer time to dissipate; inhibition generally takes place more quickly"). Effort: M.

### 5.6 Washout / steady-state calculator per supplement
- Shows: time to ~94% steady state (4 half-lives) and to ~94% elimination after stopping; "wait N days before the blood draw".
- Excel: `=ROUNDUP(4*t_half,0)`; table of half-lives where known; chart of accumulation curve `=1-0.5^(t/t_half)`.
- Source: KAT ch09.md:107-111 (50% at 1 half-life, 75% at 2, >90% at 4); GG Appendix II PK data ch88.md (half-lives table; table body lost). Supplement half-lives **[EXT]**. Effort: M.

### 5.7 Statin x supplement map (red yeast rice, CoQ10, niacin, fiber, sterols, fish oil)
- Shows: warns that red yeast rice contains monacolin K = lovastatin (content varies >120-fold between OTC products; rhabdo/liver/renal case reports) and double-counts with a statin; CoQ10 meta-analysis shows no myopathy benefit; niacin lowers TG/LDL/Lp(a), raises HDL.
- Source: DIP ch37.md (Dietary Supplements subsection, red yeast rice ~l.333); KAT ch80.md:289 (CoQ10 and statin myopathy: no benefit); KAT ch46.md:243 (niacin effects); KAT ch11.md SLCO1B1 box. Effort: S.

### 5.8 Vitamin-K antagonist awareness panel
- Shows: all stack items that touch the warfarin pathway (CoQ10 vitamin-K-like, glucosamine, melatonin, SJW, ginkgo/garlic/ginseng) with direction.
- Source: KAT ch80.md:295-297, 319, 355, 489. Effort: S (subset of 5.3).

### 5.9 Chelation / separation windows in the dosing calendar
- Shows: items that bind or chelate others (minerals vs some antibiotics/thyroid drugs) get a "separate by >=X h" rule; calendar highlights violations in the same time-of-day slot.
- Excel: `COUNTIFS` of conflicting pairs in the same slot; conditional format.
- Source: KAT ch82.md (absorption: "bind or chelate", "alter gastric pH"; "reduction in extent of absorption is clinically important"); WHO ch00.md:1144 ("Do not administer with calcium" for ceftriaxone - an example). Specific separation hours **[EXT]**. Effort: M.

### 5.10 Drug-induced lab abnormality explainer
- Shows: for each abnormal marker, supplements/drugs in the stack documented to move it (e.g. niacin/statins -> ALT up to 3x ULN intermittent; amiodarone-thyroid; topiramate -> hyperchloremic acidosis; NSAID/ACEi/ARB with K).
- Source: KAT ch46.md:201 (aminotransferase up to 3x ULN, monitor if stable); DIP ch75.md:225 (topiramate); DIP ch98.md:231 (amiodarone/thyroid). Effort: L.

### 5.11 Lab-timing rules ("fasting 10 h", "draw before biotin")
- Shows: per-test prep checklist before the next draw.
- Source: lipids measured after a 10-hour fast KAT ch46.md:77; biotin DIP ch98.md:233; A1c needs no fast DIP ch97.md:197. Effort: S.

---

## 6. Dosing, scheduling and burden

### 6.1 Daily dosing calendar (time-of-day grid)
- Shows: rows = supplements/Rx, columns = morning / noon / evening / bedtime / with-food; cell = dose. One-page printable.
- Inputs: dose, frequency, time_of_day, with-food flag.
- Excel: `XLOOKUP`/`FILTER` driven grid; data validation list for time slots; print area set.
- Source: "directions for use ... simpler is better; fewer doses (and drugs) per day" KAT ch81.md:91; time-of-day data already in his supplement table. Effort: M.

### 6.2 Pill-burden and regimen-complexity score
- Shows: pills/day, dosing events/day, distinct time slots, by category; trend over time as stack grows; vs a threshold.
- Excel: `SUMPRODUCT(dose*units)`, `COUNTA(unique time slots)`; line chart by start/stop dates (needs active-date ranges).
- Why: the book says fewer doses and drugs per day improve adherence; noncompliance is a major cause of errors.
- Source: KAT ch81.md:91, 147-161 (four types of noncompliance); DIP ch29.md:21 (polypharmacy common in older adults; Beers). Effort: M.

### 6.3 Medication/supplement timeline (Gantt)
- Shows: bar per item from start to stop, coloured by Rx vs supplement vs protocol, with lab-panel dates as vertical lines.
- Excel: stacked bar chart with transparent offset series; or cell-fill conditional format on a date grid.
- Effort: M. Source: DIP ch07.md (medication history is part of the assessment); presentation method.

### 6.4 Cost per day / per month / per outcome-marker
- Shows: price/dose x frequency -> $/day; rank by cost; $ spent per item with weak evidence (join to evidence grade).
- Excel: `=price/units_per_bottle*units_per_day`; PivotTable-style `SUMIFS`.
- Why: pairs spend with evidence level (see 7.x) -> "$ on very-low-certainty evidence".
- Effort: S. Source: arithmetic; GRADE levels EV Balshem.

### 6.5 Refill / run-out calendar
- Excel: `=purchase_date + bottle_units/units_per_day`; conditional format when <14 days. Effort: S. Source: adherence KAT ch81.md:147.

### 6.6 Rx vs supplement vs WHO-essential classification
- Shows: tags each item "WHO Essential Medicine (vitamin/mineral)", "Rx", "supplement", and shows WHO form/strength (colecalciferol 400/1000 IU; ergocalciferol 50,000 IU; nicotinamide 50 mg; pyridoxine 25 mg; riboflavin 5 mg; thiamine 50 mg; ascorbic acid 50 mg; calcium 500 mg; iodine; retinol).
- Source: WHO ch00.md:4346-4420 (Section 27 Vitamins and minerals). Effort: S. Use as the "reference dose" column next to David's dose (ratio dose/WHO-list unit).

### 6.7 PK-driven timing suggestions (half-life -> BID vs QD)
- Source: KAT ch09.md (half-life determines dosing interval and accumulation); GG ch88.md. Effort: M, weak without half-life data.

### 6.8 Discontinuation / cycling tracker
- Shows: "weeks on" for cycled items (e.g. Bacopa onset 12 weeks; SJW onset 2-4 weeks; long-term benefit beyond 12 weeks unstudied).
- Source: Bacopa RCT 12-week design (SR-papers: "Randomized controlled trial of standardized Bacopa monniera extract in age-associated memory impairment"; "Effects of a Standardized Bacopa monnieri Extract ... Elderly", 12 weeks); KAT ch80.md:247 (SJW 2-4 week onset; >12 weeks unstudied). Effort: S.

### 6.9 Medication-therapy-problem tags
- Shows: tag each item with the DiPiro categories: needs additional therapy / unnecessary / wrong drug / dose too low / dose too high / adverse reaction / adherence.
- Source: DIP ch07.md:119. Effort: S (data validation list + pivot).

---

## 7. Evidence grading per supplement (GRADE, RoB 2, AMSTAR 2, statistics)

### 7.1 GRADE certainty badge per supplement x outcome
- Shows: High/Moderate/Low/Very low with the five downgrade checkboxes and three upgrade checkboxes; certainty computed.
- Inputs: study design of best evidence (RCT/observational), checkboxes: risk of bias, inconsistency, indirectness (e.g. animal-only), imprecision, publication bias; upgrades: large effect, dose-response, plausible confounding opposing effect.
- Excel: start = IF(RCT,4,2) (RCT starts "high", observational "low"); `=MAX(1,start - downgrades + upgrades)` -> CHOOSE("Very low","Low","Moderate","High"); stars (4 plus symbols = high, 3 = moderate, 2 = low, 1 = very low).
- Source: GRADE-3 Balshem Table (RCT starts high, observational low; downgrade: risk of bias, inconsistency, indirectness, imprecision, publication bias; upgrade: large effect, dose response, residual confounding; final categories High (4 plus) / Moderate / Low / Very low (1 plus)) ~l.238-261; GRADE-1 Guyatt evidence-profile / SoF structure Table 1. Effort: M.

### 7.2 Summary-of-findings table per supplement
- Shows: outcome, N studies, N participants, effect + CI, certainty. One row per outcome, laid out like a GRADE SoF.
- Source: GRADE-1 (Guyatt) "What is the difference between an EP and a SoFs table": EP has explicit quality judgments; SoF is the summary; one table per comparison with all patient-important outcomes. Effort: M.

### 7.3 Risk-of-bias traffic-light grid for his RCT papers
- Shows: papers x 5 RoB-2 domains with Low / Some concerns / High and an overall judgment.
- Domains: randomisation process; deviations from intended interventions; missing outcome data; measurement of outcome; selection of reported result.
- Excel: data validation list + conditional format; overall = worst domain (`=IF(COUNTIF(r,"High")>0,"High",IF(COUNTIF(r,"Some concerns")>0,"Some concerns","Low"))`).
- Source: RoB2 Sterne 2019: five domains, each judged low / some concerns / high, algorithm to overall judgment (l.93-97, 149, Table 1). Applicable papers he already has: Bacopa RCTs, ashwagandha RCTs, naturopathic anxiety RCT, citicoline RCT. Effort: M.

### 7.4 AMSTAR-2 confidence rating for the systematic reviews in his library
- Shows: 16 items yes/no, with the 7 critical items flagged (2 protocol registered, 4 search adequacy, 7 justification for excluded studies, 9 risk-of-bias of included studies, 11 appropriate meta-analytic methods, 13 RoB considered in interpretation, 15 publication bias) -> overall High / Moderate / Low / Critically low.
- Excel: `=IF(critical_flaws>1,"Critically low",IF(critical_flaws=1,"Low",IF(noncritical>1,"Moderate","High")))`.
- Source: AMSTAR2 Shea 2017 Box 1 (l.478-489) and overall rating scheme (l.497-520: no/one non-critical weakness = High; >1 non-critical = Moderate; one critical flaw = Low; >1 critical = Critically low). Applies to: ashwagandha SR, vitamin D & cognition SRs, vitamin D + calcium CV SR. Effort: M.

### 7.5 Ioannidis positive-predictive-value calculator
- Shows: PPV that a "significant" finding is true, from prior odds R, power (1-beta), alpha, bias u.
- Excel: `=((1-beta)*R)/(R-beta*R+alpha)`; with bias u: `=((1-beta)*R+u*beta*R)/(R+alpha-beta*R+u-u*alpha+u*beta*R)`; sliders via data validation; surface table of PPV by power and prior.
- Why: tells him how much a single supportive paper should move belief.
- Source: IOA 2005 (PPV formula = (1-b)R/(R-bR+a) in the "PPV" paragraph; Table 2 with bias u; Table 3 n-study formula). Effort: S.

### 7.6 Statistical-power sanity badge
- Shows: sample size vs a rule: tag small trials (e.g. ashwagandha adjunct n = 15/15; Bacopa elderly n = 54, 48 completed) as "underpowered risk".
- Source: BUT Button 2013 (median power 21% in neuroscience; excess-significance); trial sizes from the SR-papers. Effort: S.

### 7.7 Directness / indirectness flag (species, population, surrogate)
- Shows: "Animal only" for NMN maternal-obesity mouse study and nicotinamide mouse study; "Adolescent males" for citicoline; "young female Navy recruits" for stress fractures; "elderly" for Bacopa.
- Source: GRADE-3 indirectness as a downgrade factor; papers in SR-papers (NMN mice; nicotinamide mice; citicoline adolescent; vitamin D stress fractures nested case-control). Effort: S.

### 7.8 Supporting vs warning papers balance bar
- Shows: per supplement stacked bar of supporting and warning papers, weighted by certainty (or study type).
- Excel: `COUNTIFS` of paper table by supplement and stance; weighted `SUMPRODUCT`.
- Note: his library already contains a deliberate warning paper - "Long-term treatment with nicotinamide induces glucose intolerance and skeletal muscle lipotoxicity in chow-fed mice" - against NAD-precursor enthusiasm. Source: SR-papers. Effort: M.

### 7.9 Effect size + CI forest plot
- Shows: per supplement outcome, point estimate and CI on a forest plot (bar chart with error bars in Excel via custom error bars).
- Template: the Reports file already uses this structure (RR 1.01 [0.72-1.40]; 1.18 [0.87-1.60]; aOR 2.72 [2.27-3.27]) and the sentence "absence of detected signal is not proof of safety; a roughly 60% relative increase is not excluded".
- Source: CIP "Bottom line". Effort: M.

### 7.10 NNT / NNH calculator
- Shows: from control event rate and relative risk (or ARR) -> NNT; stack of statin examples.
- Excel: `=1/(CER*(1-RR))`; time-horizon toggle.
- Source: OUT ch15.md:57 (statin primary prevention 5-year NNT roughly 33-130; ~7 over a 30-year horizon per Sniderman). NNT definition arithmetic; Attia's example is the only sourced numbers. Effort: S.

### 7.11 Sensitivity / specificity / PPV for screening tests
- Shows: interactive 2x2: prevalence, sens, spec -> PPV, NPV, false positives per 1,000.
- Excel: `PPV=sens*prev/(sens*prev+(1-spec)*(1-prev))`.
- Source: OUT ch22.md:19-21 (mammography sens mid-80s, spec low 90s; at 1% prevalence PPV ~10%; stack tests). Effort: S.

### 7.12 "How far to trust one paper" panel (replication/power/bias)
- Shows: checklist per paper (preregistered? n? replicated? industry funded? RCT? animal?) -> score.
- Source: EV Munafo 2017 (manifesto), Simmons 2011 (false-positive psychology), Camerer 2018 (replicability), Button 2013 - read only as titles/abstract-level; Ioannidis 2005 corollaries. Effort: M. (Papers not read in depth.)

### 7.13 Evidence dashboard: cost vs certainty bubble
- Shows: x = GRADE certainty (1-4), y = $/month, bubble = pill count; top-left quadrant "expensive and uncertain".
- Source: combines 6.4 and 7.1. Effort: M.

### 7.14 Dose-in-trial vs dose-taken comparator
- Shows: trial dose (e.g. SJW 900 mg/day in 3 doses, 2-5% hyperforin; vitamin D 4,000 IU/d for >=40 ng/mL target; ashwagandha 120 mg/d in SSRI-adjunct RCT; Bacopa standardized extract) next to David's dose; ratio, and "no trial at this dose".
- Source: KAT ch80.md:247; SR-papers (vitamin D stress fractures: target >=40 ng/mL, ~4000 IU/day achievable; ashwagandha adjunct RCT 120 mg/day, n=15 per arm). Effort: M.

---

## 8. Biomarker-to-supplement response tracking

### 8.1 25(OH)D: dose-response and sourced thresholds
- Shows: 25(OH)D over time with lines at 20 ng/mL (below: roughly double stress-fracture risk) and 40 ng/mL (target in the stress-fracture study), overlay of daily vitamin D dose (IU) per period; regression of Delta level per 1,000 IU.
- Source: SR-papers "High serum 25-hydroxyvitamin D is associated with a low incidence of stress fractures" (<20 ng/mL vs >=40 ng/mL = double risk; target >=40 ng/mL; 4,000 IU/d); "Vitamin D Insufficiency among Free-Living Healthy Young Adults" (population distribution); "Vitamin D, cognition, and dementia" (threshold 50 nmol/L); Attia APOE note OUT ch28.md:37 (higher vitamin D correlated with better memory in e4/e4, causality unclear). Effort: M.

### 8.2 Homocysteine vs B-vitamin (folate, B6, B12) intake
- Source: KAT ch46.md:169 (homocysteine lowered by folic acid + B vitamins; betaine for severe), OUT ch28.md:37. Effort: S.

### 8.3 Omega-3 index / TG / EPA:DHA dose
- Source: DIP ch37.md:329 (oily fish 1-2 servings/week, AHA twice weekly; fish oil gives EPA/DHA); SR-papers "EPA but not DHA appears responsible ... depression" - split dose into EPA and DHA columns. Effort: S.

### 8.4 Ferritin/TSAT vs iron supplement
- Source: DIP ch124.md:249-257. Effort: S. **[DX-RISK]** (overload vs deficiency).

### 8.5 Uric acid vs fructose/purine/allopurinol/vitamin C stack
- Source: DIP ch115.md; OUT ch13.md:150-164. Effort: S.

### 8.6 Thyroid vs iodine/biotin/selenium/amiodarone-like exposures
- Source: DIP ch98.md:231 (iodine excess/amiodarone iodine 6 mg/day per 200 mg vs RDA 150 mcg), ch98.md:233. Effort: S.

### 8.7 Metformin / NAD precursors / longevity-stack markers
- Source: SR-papers (metformin review "from lilac to longevity"; NMN and nicotinamide mouse papers); Lifespan (Sinclair) epigenetic/biomarker aging discussion `Medicine/Books/David A. Sinclair... Lifespan.../ch00.md:2256, 3598` (only skimmed). Effort: S.

### 8.8 Biological-age composite from blood markers
- Shows: a score from albumin, creatinine, glucose, CRP, lymphocyte %, MCV, RDW, ALP, WBC (PhenoAge).
- Source: Lifespan ch00.md:3598 mentions biomarker-based biological age; the PhenoAge coefficients are **[EXT]** (Levine 2018). Effort: L. **[DX-RISK]** and scientifically contested - mark experimental.

---

## 9. Pharmacogenomic cross-reference (genome sheet x drug/supplement sheet)

### 9.1 Gene -> drug -> his genotype lookup
- Shows: for each drug/supplement he takes (or considered), the relevant pharmacogene, his genotype (23andMe/WGS), predicted phenotype (poor / intermediate / normal / rapid), and what the text says changes.
- Inputs: rsID genotypes; star-allele call table (hand-curated, since the 23andMe chip gives rsIDs); drug->gene map.
- Excel: `XLOOKUP(rsid&genotype, phenotypeTable)`; `TEXTJOIN` of affected drugs; colour by phenotype.
- Sourced content: CYP2D6 (>95% of phenotypes from 9 alleles; *3,*4,*5,*6 null; *10,*17,*41 reduced; *4 ~20% Europeans) KAT ch11.md:33; CYP2C19, CYP2B6, DPYD, UGT1A1, TPMT, G6PD (ch11.md headings); SLCO1B1 rs4149056 Val174Ala -> simvastatin exposure +221% in homozygotes (ch11.md:117-121); ABCG2 rs2231142 Q141K (~30% East Asians) ch11.md:123-127; OCT1 SLC22A1 ch11.md:129-135; CYP2C9 *2 (~30-40% less S-warfarin metabolism) and *3 + VKORC1 -1639G>A ch11.md:179-185; HLA hypersensitivity ch11.md:139-169. Parallel texts: GG ch14.md (Ch7 pharmacogenetics), DIP ch12.md (e6). Effort: L. **[DX-RISK]** - phenotype calls from consumer chips are unreliable for star alleles; label "informational, confirm with clinical PGx".

### 9.2 Hardy-Weinberg population-frequency check
- Shows: for each variant, expected genotype frequency in his ancestry (p^2, 2pq, q^2) and whether his genotype is common or rare.
- Excel: `=p^2`, `=2*p*(1-p)`, `=(1-p)^2`.
- Source: KAT ch11.md:33 (worked example: *4 ~20% in Europeans -> 4% homozygous, 32% heterozygous). Allele frequencies per population are in KAT Table 5-1 (table body lost) - pull from gnomAD **[EXT]**. Effort: S.

### 9.3 "Inducer/inhibitor x his metabolizer status" conflict detector
- Shows: if he is a poor metabolizer for CYP X and takes an inhibitor of CYP X (phenoconversion), show amplified exposure; if inducer present and normal metabolizer, show reduced exposure.
- Source: KAT ch82.md (metabolism section), KAT ch80.md:243 (SJW CYP3A4/2C9/1A2 induction), KAT ch80.md:355 (melatonin CYP1A2). Effort: L.

### 9.4 APOE / Lp(a) / lipid-gene risk cross-reference
- Shows: APOE genotype next to ApoB/lipids and the Attia prevention-plan items (APOE e4: sleep, exercise, homocysteine, omega-3, vitamin D discussion).
- Source: OUT ch27.md (Role of APOE e4), ch28.md:37 (preventive plan). LPA genotype (rs10455872, rs3798220) **[EXT]**. Effort: M. **[DX-RISK]**.

### 9.5 Homocysteine / MTHFR cross-reference
- Source: KAT ch46.md:169 (homocysteine, folate/B vitamins/betaine); MTHFR C677T effect size **[EXT]** - library has no MTHFR text; do NOT overstate (many Promethease MTHFR "risks" are weak). Effort: S.

### 9.6 Caffeine / melatonin / CYP1A2 view
- Source: KAT ch80.md:355 (melatonin metabolised by CYP1A2; fluvoxamine). CYP1A2 caffeine genotype **[EXT]**. Effort: S.

### 9.7 Promethease annotation triage
- Shows: filter his Promethease annotations to variants with a pharmacologic or lab-marker bridge (e.g. SLCO1B1, VKORC1, CYP2C9, G6PD, ABCG2/uric acid, LPA, APOE), with magnitude/repute columns -> view limited to actionable-in-this-workbook items.
- Source: KAT ch11.md gene list; EV GRADE principle (rate certainty of each association; most SNP-trait links are low certainty). Effort: M.

---

## 10. Preventive care, vaccines, screening schedules

### 10.1 Vaccine record with due-date engine
- Shows: each vaccine, last dose date, doses in series, next due, status (current / due in 90 d / overdue / series complete).
- Inputs: vaccine, date given, series position, interval rule per vaccine (booster interval, e.g. Td/Tdap 10 years, influenza annual).
- Excel: `=EDATE(last, months)`; `=IF(due<TODAY(),"overdue",IF(due<TODAY()+90,"due soon","ok"))`; conditional format; a Rules table sheet so intervals are data, not formulas.
- Source: concept - inactivated/subunit/toxoids need multiple doses and boosters DIP ch148.md:17; schedules updated annually (use current CDC) DIP ch148.md:23, 165; intervals themselves **[EXT]** (CDC schedule) because KAT Appendix Table A-2 body and the stripped tables are unreadable. Effort: M.

### 10.2 Live vs inactivated interaction guard
- Shows: warns when a live vaccine is planned within the Ig-interference window; pregnancy deferral for live vaccines.
- Source: DIP ch148.md:87 (Ig can inhibit live vaccine response, dose-related), ch148.md:113-115 (most vaccines fine in pregnancy; live vaccines deferred). Effort: S.

### 10.3 Vaccine storage-temperature note (not needed personally)
- Skip. (DIP ch148.md:97) - listed only because found.

### 10.4 Screening schedule by age and sex
- Shows: age timeline with recommended tests (colonoscopy, lipid panel, A1c/OGTT, PSA/mammography, skin check, Lp(a) once, DEXA) and last/next done.
- Inputs: DOB, sex, last-done dates, risk flags.
- Excel: `=DATEDIF(DOB,TODAY(),"y")`; rule table (test, start_age, interval_yrs, risk modifiers); due = `EDATE(last, 12*interval)`.
- Source (David's library only): Attia: reliable screening exists for 5 cancers: lung (smokers), breast, prostate, colorectal, cervical (OUT ch22.md:15); his practice: colonoscopy by age 40 and repeat q2-3 years depending on findings (ch22.md:31); Pap yearly (ch22.md:37); Lp(a) once per lifetime (ch14.md:163); diabetes screening at BMI >=25 (>=23 Asian-American) with another risk factor (DIP ch97.md:211). Mainstream intervals (USPSTF) **[EXT]** - the library has no guideline document; mark Attia's earlier-start recommendations as "Attia practice, not guideline". Effort: M. **[DX-RISK]** low if stated as schedule lookup.

### 10.5 Physical/provider calendar
- Shows: physicals, specialists, dental, eye, with last/next and provider contact; ties to pending screening items.
- Source: DIP ch07.md (patient care process = collect -> assess -> plan -> implement -> follow-up/monitor). Effort: S.

### 10.6 Monitoring-interval engine ("repeat lipid panel 4-12 weeks after a change")
- Shows: for each therapy change event, the earliest/latest date for the follow-up lab and whether it happened.
- Source: DIP ch37.md:451 (repeat lipid panel 4-12 weeks after starting or changing therapy); A1c every 2-3 months exposure DIP ch97.md:197; thyroid follow-up every 6-12 months after remission DIP ch98.md:345. Effort: M.

### 10.7 Travel health checklist
- Source: Pharmacotherapy e17 Travel Health (title only; not opened). Effort: S, weak.

---

## 11. Presentation patterns clinicians use (workbook structure)

### 11.1 SOAP-style "visit" sheet
- Shows: one-page printable for a physician visit: Subjective (symptoms/goals), Objective (latest labs + trends + weight), Assessment (flags), Plan (questions to ask, tests to request).
- Excel: all `XLOOKUP`/`TEXTJOIN` auto-populated from the latest panel; print layout.
- Source: DIP ch07.md (patient care process; problem list; documentation). Effort: M.

### 11.2 Medication reconciliation sheet
- Shows: the current list: name, dose, indication, prescriber, start date, last review.
- Source: DIP ch07.md (medication history/assessment); KAT ch81.md (Rational prescribing: elements of a prescription). Effort: S.

### 11.3 "Questions for my doctor" generator
- Shows: auto-list from flags (rising marker, unmeasured Lp(a), interaction found).
- Effort: S. Source: derived from other views.

### 11.4 Geriatric/Beers-style "stopping" review for long-term items
- Source: DIP ch29.md:21,39 (polypharmacy, Beers criteria); only relevant later in life; low priority. Effort: S.

### 11.5 Sleep and recovery quick panel
- Shows: sleep hours/regularity, PSQI score (last month), tracker stats (HRV, resting HR) if routine data exists.
- Source: OUT ch58.md:25-27 (trackers estimate duration/stage with reasonable accuracy; PSQI is best-validated questionnaire). Effort: S.

### 11.6 VO2max and fitness benchmarks
- Source: OUT ch34.md:7-9 (VO2max strongly correlated with longevity; testing available; recorded number). Effort: S. Only if fitness data exists.

---

## 12. Top-15 shortlist (impact for David x effort x data availability)

1. 5.1 Biotin washout planner + per-panel annotation (S-M; a sourced, concrete clinical trap).
2. 7.1 GRADE certainty badge per supplement (M; fits the 87 supplements and supporting/warning papers).
3. 5.3 Supplement-drug interaction matrix seeded from Katzung (L; biggest safety value).
4. 1.4 Reference vs optimal vs personal-baseline band charts (M; core of the dashboard).
5. 2.3 ApoB vs LDL-C concordance + 2.1 TG/HDL + 2.2 non-HDL (S-M; Attia's priority numbers).
6. 1.1 Reference Change Value / critical-difference flag (M).
7. 9.1 Pharmacogene lookup against his genotypes (L; use Katzung gene list).
8. 6.1 + 6.2 Dosing calendar and pill-burden score (M).
9. 8.1 25(OH)D dose-response with 20/40 ng/mL lines (M).
10. 4.1 Weekly weight-rate with guard-rails + 3.9 protein per kg (M; uses his daily data).
11. 10.1 Vaccine due-date engine (M).
12. 10.4 Screening-schedule by age (M).
13. 7.3/7.4 RoB-2 and AMSTAR-2 grids for the papers (M).
14. 2.17 Iron quartet + 2.22 thyroid axis with biotin overlay (M).
15. 6.4 + 7.13 Cost vs certainty bubble chart (M; "money on very-low-certainty evidence").

Honourable mentions: 1.7 projected crossing date, 3.2 problem-oriented list, 2.12 eGFR (needs EXT coefficients), 7.5 Ioannidis PPV calculator, 7.9 forest plot.

---

## 13. Cross-cutting build notes for rust_xlsxwriter

- Keep reference data (CV table, interaction table, allele/phenotype table, vaccine rules, screening rules, DRI/UL, GRADE levels) as separate "Rules" sheets populated from versioned CSV/JSON, never hard-coded in formulas, so every threshold carries its `source` column (file + line) from this catalog.
- Every formula-based flag should show its source in a cell comment/note (rust_xlsxwriter `Note`) and an `evidence` column; mark **[EXT]** rows visibly until verified.
- Prefer dynamic-array functions (`XLOOKUP`, `FILTER`, `LET`) only if the target Excel version supports them (Excel 365); otherwise INDEX/MATCH and SUMPRODUCT.
- Language guard: add a global "Visualisation only - not medical advice" banner and avoid diagnostic wording in all [DX-RISK] views.

---

## 14. Books/papers actually read from (and how deeply)

Read (targeted chapters, with grep of key sections):
- Katzung, Basic and Clinical Pharmacology 15e: Ch5 Pharmacogenomics (ch11), Ch3 PK (ch09, steady state passage), Ch35 Dyslipidemia (ch46, selected), Ch65 Dietary Supplements & Herbal Medications (ch80, mostly full), Ch67 Drug Interactions (ch82, full prose), Ch66 Rational Prescribing (ch81, compliance parts), Appendix vaccines (ch83, intro; tables stripped).
- DiPiro, Pharmacotherapy 11e: Ch31 Dyslipidemia (ch37), Ch69 Acid-Base (ch75), Ch92 Thyroid (ch98, selected), Ch118 Anemias (ch124, labs section), Ch109 Gout (ch115, key concepts), Ch158 Nutrition Assessment (ch164, selected), Ch161 Obesity (ch167, selected), Ch91 Diabetes (ch97, diagnosis/targets), Ch142 Vaccines (ch148, selected), Ch61 CKD (ch67, staging), e59 Kidney Function (ch65, key concepts only), Ch65 renal dosing (ch71, intro), e5 PK (ch11, key concepts), e23 Prescribing in Older Adult (ch29, headlines), Ch1 Patient Care Process (ch07, selected).
- Goodman & Gilman 13e: table of contents and Appendix II existence (ch88, intro); pharmacogenetics chapter not read in depth.
- Attia, Outlive: ch13 (metabolism/biomarkers), ch14-15 (lipids, apoB, Lp(a), NNT), ch22 (early detection, screening, sens/spec), ch28 (APOE prevention plan), ch34 (VO2max), ch50 (protein), ch58 (sleep assessment).
- WHO Model List of Essential Medicines 21st: Section 27 Vitamins and Minerals (ch00.md:4346-4420).
- Evidence papers: Guyatt 2011 GRADE-1, Balshem 2011 GRADE-3, Sterne 2019 RoB 2, Shea 2017 AMSTAR 2, Ioannidis 2005, Button 2013 (text extracted with pdftotext; key tables/formulas located).
- Supplement papers in Stacks/R Medicine (abstract/first-page level): vitamin D stress fractures; vitamin D insufficiency in young adults; vitamin D & cognition SRs; vitamin D+calcium CV SR; ashwagandha RCTs and SR; Bacopa RCTs; omega-3 papers (EPA vs DHA depression; athletics; schizophrenia working memory); citicoline RCT; NMN mouse; nicotinamide mouse; metformin review; naturopathic anxiety RCT.
- Report: Medicine/Reports/2026-10-03-ciprofloxacin-pregnancy.org (bottom-line section).

Only titles/indexes scanned (NOT read; no claims drawn from them): Lehninger Principles of Biochemistry 8e; Sinclair Lifespan (only two hits located); Walker Why We Sleep; de Grey Ending Aging; Fung The Obesity Code; Taubes Good Calories Bad Calories; Greger How Not to Die; Inchauspe Glucose Revolution; Helms Muscle & Strength Pyramid Nutrition; Schoenfeld/Israetel/Valdez/Hutchinson exercise books; van der Kolk; Sacks; RFK Jr Fauci books; Munafo/Simmons/Camerer/Brembs/ergonomics papers; the Medicine/Patents and Biotechnology folders; Headspace courses; Data/Statistics (only I.J. Good Rational Decisions listed; no clinical-statistics text).

Missing from the library (so these ideas lean on **[EXT]**): any clinical chemistry/lab-medicine text, biological-variation database, screening-guideline compendium (USPSTF/ACS), CDC immunization schedule, CPIC/PharmGKB tables, NIH ODS/IOM UL tables. Recommend a follow-up bead to ingest: Tietz or Henry's lab-medicine reference, USPSTF A/B recommendations, CDC adult schedule, CPIC guidelines, EFLM biological-variation database.

