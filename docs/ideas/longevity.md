# hx-rx3.5 — Longevity / Nutrition / Exercise / Sleep / Genetics: workbook feature catalog

Visualisation only, not medical advice. All ideas are views over David's own data (labs, daily log, supplements, nutrition items, routine, workouts, genome, optional Whoop).

## Source key (paths abbreviate to these aliases)

- OUT = `~/Knowledge/Medicine/Books/Peter Attia MD - Outlive_ The Science and Art of Longevity-Harmony (2023)/chNN.md`. File numbers are not book chapter numbers; I cite file + chapter title. Map: ch13 Can Our Ancient Genes Cope with Our Modern Diet? (metabolic health); ch14 Confronting Heart Disease; ch15 How to Reduce Cardiovascular Risk; ch16 Lipid-Lowering Meds; ch22 Early Detection; ch27 The Role of APOE e4; ch28 The Preventive Plan; ch30 The Most Powerful Longevity Drug (exercise); ch31 The Centenarian Decathlon; ch33 Zone 2; ch34 VO2 max; ch35 Strength; ch49 Lessons from CGM; ch50 Protein; ch58 Assessing Your Sleep; ch60 How to Improve Your Sleep.
- LIFE = `~/Knowledge/Medicine/Books/David A. Sinclair, Matthew D. LaPlante - Lifespan_ ... (2019)/ch00.md` (whole book in one file, PDF-derived; I cite line numbers, no page numbers exist).
- WWS = Walker, Why We Sleep. Chaptered epub copy `~/Knowledge/Medicine/Books/Why We Sleep_ Unlocking the Power of Sleep and Dreams - Matthew Walker/chNN.md` (ch03 Caffeine, Jet Lag, Melatonin; ch06 Changes in Sleep Across the Life Span; ch21 Appendix = Twelve Tips). Line numbers refer to the PDF copy `~/Knowledge/Medicine/Books/Matthew Walker PhD - Why We Sleep_ ... (2017)/ch00.md`.
- GREG = `~/Knowledge/Medicine/Nutrition/Books/How Not to Die - Greger/chNN.md` (ch20 Daily Dozen; ch30 Whole Grains; ch35 Appendix: Supplements).
- GLU = `~/Knowledge/Medicine/Nutrition/Books/Glucose Revolution - Inchauspé/chNN.md` (ch11 Underneath Our Skin; ch16-25 Hacks 1-10; ch22 = Hack 7 vinegar; ch23 = Hack 8 move).
- HELMS = `~/Knowledge/Medicine/Nutrition/Books/The Muscle and Strength Pyramid Nutrition - Helms Morgan Valdez/ch01.md` (Levels 1-2: energy, macros; text is mojibake-encoded, "�" = ".") and `ch02.md` (supplements, micronutrients).
- ISRA = `~/Knowledge/Medicine/Exercise/Books/Scientific Principles of Hypertrophy Training - Israetel/ch03.md, ch06.md`.
- SCHO = `~/Knowledge/Medicine/Exercise/Books/Science and Development of Muscle Hypertrophy (2nd ed.) - Schoenfeld/ch09.md, ch12.md`.
- ENDU = `~/Knowledge/Medicine/Exercise/Books/Endure ... - Hutchinson/ch09.md` (Conscious Quitter / Borg RPE), ch08 (Central Governor), ch15 (Heat).
- DEGR = `~/Knowledge/Medicine/Longevity/Books/Ending Aging - de Grey Rae/ch08.md` (Engineering Rejuvenation).
- TW = `~/Knowledge/Medicine/Twitter Bookmarks/<id>.md` (weak evidence; tweets, flagged as such).
- EXT = NOT in David's library. I verified PhenoAge in this session (links below); all other EXT citations are standard literature cited from memory and NOT verified here: flagged "EXT-unverified" so the builder checks them before shipping.
  - PhenoAge weights: Levine et al. 2018, Aging 10(4):573, Table 1, https://pmc.ncbi.nlm.nih.gov/articles/PMC5940111/ (verified: albumin g/L -0.0336; creatinine umol/L 0.0095; glucose mmol/L 0.1953; ln CRP mg/dL 0.0954; lymph % -0.0120; MCV fL 0.0268; RDW % 0.3306; ALP U/L 0.0019; WBC 1000/uL 0.0554; age 0.0804).
  - PhenoAge intercept/gamma/conversion constants (-19.907, gamma 0.0076927, 141.50225, 0.00553, 0.090165): public implementation README https://unpkg.com/mcp-phenoage-clock@1.0.1/README.md (paper's Supplement 1 not read; verify before shipping). That README uses ALP 0.00188 unrounded; paper table prints 0.0019.

Caveats that apply across the catalog: the library has no genetics book (`Medicine/Genetics/Books` is empty), so genotype cross-cuts lean on Outlive (APOE e4, Lp(a), homocysteine) plus EXT-unverified SNP knowledge. Neuroscience/Books is only Twitter bookmarks. Library has no Blueprint source; Blueprint targets come from David's own data.

Effort: S = a sheet of formulas, M = formulas + a chart or conditional formats, L = multi-sheet model, external lookup tables or heavy charting. All ideas are rust_xlsxwriter-feasible (formulas, tables, conditional formats, sparklines, charts, data validation); noted where Excel itself is the limit.

TOP-15 SHORTLIST marker: **[TOP15]**.

---

## A. Biological age and aging-clock views

**A1. PhenoAge calculator per panel [TOP15]** (effort M)
- Shows: Levine PhenoAge and age-acceleration (PhenoAge minus chronological age) for each of the ~7 panels 2018-2025, line chart vs the y=x diagonal.
- Inputs: albumin, creatinine, fasting glucose, hs-CRP, lymphocyte %, MCV, RDW, ALP, WBC, draw date, DOB. All in CBC+CMP+CRP; CRP is the one to check exists on every panel.
- Build: convert units in helper columns (albumin g/dL x10 = g/L; creatinine mg/dL x88.42 = umol/L; glucose mg/dL /18.016 = mmol/L; CRP mg/L /10 = mg/dL). xb = -19.907 -0.0336*alb +0.0095*cr +0.1953*glu +0.0954*LN(crp_mgdL) -0.0120*lym +0.0268*MCV +0.3306*RDW +0.00188*ALP +0.0554*WBC +0.0804*age. M = 1-EXP(-EXP(xb)*(EXP(120*0.0076927)-1)/0.0076927). PhenoAge = 141.50225 + LN(-0.00553*LN(1-M))/0.090165. Guard LN(CRP) with MAX(crp,0.01) (CRP of 0 breaks the log).
- Why: the single most recognised lab-only aging clock; turns the 7 panels into a trajectory. Original study: each +1 year of PhenoAge over chronological age associated with ~9% higher mortality risk (per the search summary of the paper; not read directly).
- Source: Levine 2018 (links above); implementation constants (README link above). Inputs coverage by lab: mine, no source needed.

**A2. PhenoAge input-completeness checker** (S)
- Shows: per panel, which of the 9 inputs are missing; PhenoAge cell shows "n/a" instead of a wrong number; fallback "PhenoAge-lite" without CRP flagged in a different colour.
- Inputs: same as A1. Build: COUNT over the 9 cells = 9 test, conditional format red on blanks. 
- Why: honest handling of older panels lacking hs-CRP. A NHANES analysis cited in the search results found no-CRP version correlates 0.99 with full (EXT-unverified, from search summary only). Source: search result summary of Frontiers/PMC papers; treat as hint.

**A3. Marker contribution waterfall (which markers add/subtract years)** (M)
- Shows: for a chosen panel, each marker's contribution to xb relative to a reference healthy person, converted to approximate years (stacked/waterfall bar).
- Inputs: A1 inputs + reference vector (e.g. median healthy values, user-editable). Build: contribution_i = coef_i*(x_i - ref_i); approximate years = contribution / 0.090165 / ... use PhenoAge(x) - PhenoAge(ref) per marker by swapping one marker at a time (9 helper rows of the A1 formula). Waterfall = stacked column with hidden base series.
- Why: tells him what to fix (RDW and glucose carry the biggest per-unit weights: 0.3306 and 0.1953). Source: Table 1 above.

**A4. What-if age slider** (M)
- Shows: change one marker via input cell (data validation within plausible range) and see delta PhenoAge live.
- Build: input cells for each of the 9 markers defaulting to latest panel (=latest), override cells in a coloured column; A1 formula over override column. Data validation decimal between bounds.
- Why: tangible feel for e.g. "what if RDW 13.5 to 12.5". Source: A1.

**A5. 10-year mortality-score view (Gompertz M)** (S)
- Shows: M from A1 as a percentage by panel, plus bars. Note it is a model score, not a prediction for David.
- Source: README formula. 

**A6. Chronological vs biological age "gap" scoreboard with Johnson/Blueprint comparators** (S)
- Shows: David's PhenoAge delta over time next to Bryan Johnson claim and Pascoe claim as annotated reference lines (user-entered).
- Inputs: A1 plus claimed numbers from TW 1857431845576798654 (Dave Pascoe "DNA of a 38-year-old at 62, spends 98.5% less than Johnson"), TW 1803400497103249519 (40+ supplements, bio age 27 at 41). Source: those tweets (weak; clocks differ so label "different clock").

**A7. Hallmarks-of-aging coverage matrix [TOP15]** (M)
- Shows: 87 supplements (rows) x 9 hallmarks (columns, tick marks), then column totals = how many compounds target each hallmark, with a bar chart and a red flag where a hallmark has 0-1 interventions.
- Inputs: supplements table + a new column "hallmarks" (multi-select via 9 boolean columns or data-validation list per column). Hallmarks per Sinclair: genomic instability, telomere attrition, epigenome alterations, loss of proteostasis, deregulated nutrient sensing, mitochondrial dysfunction, cellular senescence, stem cell exhaustion, altered intercellular communication.
- Build: 9 boolean columns, SUMPRODUCT/COUNTIF totals; conditional format colour scale; stacked bar by research-stance. Data validation lists "Y/blank".
- Why: shows gaps and overconcentration in the stack. Source: LIFE ch00.md lines 1095-1118 (hallmark list) and 1170-1171 ("eight or nine hallmarks").

**A8. Cost-per-hallmark [TOP15]** (S)
- Shows: monthly $ spent per hallmark (price/dose-frequency to $/month, split evenly across a supplement's hallmarks) as bar/treemap-substitute (sorted bar).
- Inputs: price, units per container, dose, frequency, hallmark flags from A7. Build: $/day = price/(units/(dose_units*freq)); SUMPRODUCT with flag columns divided by count of flags per row.
- Why: where the money goes vs where evidence is. Source: mine; hallmark list as A7.

**A9. Hallmark x research-stance heat grid** (S)
- Shows: count of supplements by research stance (e.g. strong/moderate/speculative) per hallmark; COUNTIFS; colour scale.
- Why: reveals "all my mitochondrial supplements are speculative". Source: David's stance column; hallmarks as A7.

**A10. Sinclair "What I do" protocol overlap** (S)
- Shows: row per item in Sinclair's personal list vs whether David takes it, dose ratio: NMN 1,000 mg/d, resveratrol 1,000 mg, metformin 1,000 mg, vitamin D, K2, aspirin 83 mg; skip one meal/day; low sugar/bread/pasta; blood tests every few months; lift/sauna/cold; BMI 23-25; stay cool.
- Build: lookup against supplements by name; dose ratio = David dose / Sinclair dose; conditional colour.
- Source: LIFE ch00.md lines 10605-10662 ("WHAT I DO"). Sinclair explicitly says it is not advice (lines 10607-10620). Useful as a "protocol library" tab with a source column per protocol (Sinclair, Attia, Greger, Blueprint, Helms).

**A11. Multi-protocol comparison matrix (Blueprint vs Sinclair vs Attia vs Greger vs Helms) [TOP15]** (M)
- Shows: rows = dimensions (protein g/kg, zone 2 min/wk, sleep hours, fasting window, alcohol, fibre g/day, supplements B12/D/omega-3/creatine, VO2 target...), columns = each source's stated number, plus David's actual value from his data and a delta.
- Build: static reference table (each cell carries source id in a note/comment via rust_xlsxwriter `write_comment`), David actual via formulas into other sheets, conditional format on delta.
- Source: per-cell citations: OUT ch50 (1.6 min, 2.2 ideal g/kg), OUT ch33 (3 h/wk zone 2), OUT ch60 (8-9 h opportunity), OUT ch49 (>50 g fibre), GREG ch35 (B12, D3 2000 IU), HELMS ch02 (EPA+DHA 2-3 g; creatine 0.04 g/kg), LIFE lines 10633-10636.

**A12. SENS damage-category map** (S)
- Shows: de Grey's categories of aging damage (chromosomal/nuclear mutations, mitochondrial mutations, intracellular junk, extracellular junk, cell loss, cell senescence/"zombies", extracellular crosslinks/glycation) mapped to David's interventions and "no available intervention" cells.
- Inputs: list of interventions with tag. Build: matrix w/ COUNTIF.
- Why: second taxonomy beside hallmarks; highlights where nothing consumer-available exists (an honest empty row). Source: DEGR ch08 (Engineering Rejuvenation) lines 66-94 show the chromosomal, mitochondrial, glycation, junk steps; the full seven-category list structure follows the book's chapters 5-12 titles (index.md: mitochondria, junk, spider webs/extracellular, AGE crosslinks, zombies, new cells, nuclear mutations). I did not read the full chapter text.

**A13. Intervention evidence ladder** (S)
- Shows: each supplement placed on rungs: human RCT outcome / human biomarker / animal only / mechanistic. Sinclair himself notes NMN/resveratrol evidence is mostly mouse (LIFE lines 4890-4906, 5016-5058) and that no antiaging therapy has long-term human clinical testing (lines 10613-10617).
- Build: data-validation dropdown for rung; pivot-like COUNTIFS; stacked bar.

---

## B. Medicine 3.0: cardiometabolic and the four horsemen

**B1. ApoB dashboard with target bands [TOP15]** (M)
- Shows: ApoB over time with bands: standard guideline LDL-C ceilings (100 mg/dL normal risk, 70 high risk are LDL-C, not ApoB) vs Attia's goal "as low as possible"; his aggressive target apoB ~20-30 mg/dL "about where it would be for a child".
- Inputs: ApoB (mg/dL) per panel; LDL-C.
- Build: line chart + two horizontal target lines (series of constants), conditional format on latest. Label that guideline LDL-C numbers and ApoB numbers are different units of risk.
- Why: ApoB is the first number Attia looks at ("my eyes immediately dart to apoB and Lp(a)"). Source: OUT ch15 (How to Reduce Cardiovascular Risk) paras 7-17, OUT ch16 para 3 (20-30 mg/dL target).

**B2. ApoB-to-LDL-C discordance** (S)
- Shows: ratio ApoB/LDL-C per panel; high ratio = more particles per unit cholesterol, flagged.
- Why: Attia argues particle count beats cholesterol-in-particle. Cut-offs are mine/derived, not a published threshold; show as "relative to his own history".
- Source: OUT ch15 line 7 (concept). Ratio: mine.

**B3. Exposure-over-time "ApoB-years" [TOP15]** (M)
- Shows: area under ApoB curve across the panels (mg/dL x years) vs the area at a "child-like" baseline (25 mg/dL) = excess lifetime exposure; cumulative line.
- Build: trapezoid sum with SUMPRODUCT over date differences /365.25; optional extrapolation back before 2018 flagged as assumption.
- Why: operationalises "the key factor is exposure to apoB-tagged particles over time". Source: OUT ch14 line 93; also ch15 line 59 (time course, 30-year NNT <7 vs 33-130 over 5 years).

**B4. TG:HDL ratio** (S)
- Shows: ratio per panel with lines at 2 and 1 (<2 good, <1 better).
- Source: OUT ch13 line 186.

**B5. Metabolic-syndrome criteria counter** (S)
- Shows: 5 criteria as pass/fail tiles: TG >150 mg/dL; HDL <40 (men)/<50 (women); waist >40 in (men)/>35 (women); fasting glucose >110 mg/dL as printed in the book; blood pressure (criterion 1, value not captured in my grep). Count of positives per panel.
- Note: Outlive prints >110 mg/dL for glucose; many guidelines use 100. Make threshold an editable cell with source in a comment.
- Source: OUT ch13 lines 53-65 (and lines 63-75: ~1/3 of obese are metabolically healthy; 20-40% of non-obese are not).

**B6. "Metabolically healthy?" vs BMI quadrant** (M)
- Shows: scatter of BMI (x) vs metabolic-syndrome count (y) over time; quadrants. Answers the tweet "BMI is wrong for me".
- Inputs: height, weight daily log, B5 counts. Source: OUT ch13 lines 47-75 (obesity vs metabolic dysfunction); TW 1769007456988860465 (BMI complaint).

**B7. Non-HDL, remnant cholesterol, and LDL-C method cross-check** (S)
- Shows: non-HDL = TC - HDL; remnant = TC - HDL - LDL-C; LDL by Friedewald (TC-HDL-TG/5) vs lab LDL.
- Source: ApoB-family concept OUT ch14 line 93, 61; Friedewald 1972 (EXT-unverified) and Martin-Hopkins 2013 JAMA 310:2061 (EXT-unverified).

**B8. Lp(a) one-time card** (S)
- Shows: Lp(a) value or "NOT TESTED" red tile; note: largely genetic, doesn't respond to diet/exercise, 20-30% of US population has elevated risk; test once. Link to LPA genotype (F4).
- Source: OUT ch14 lines 157-167; ch15 line 7.

**B9. Early-warning metabolic panel (uric acid, homocysteine, hs-CRP, ALT)** (S)
- Shows: four sparkline rows. Attia watches elevated uric acid, homocysteine, inflammation, mildly elevated ALT, plus TG:HDL and VLDL "years before textbook metabolic syndrome".
- Source: OUT ch13 line 186.

**B10. Glycemia triad: fasting glucose, A1c, fasting insulin** (M)
- Shows: three small multiples; glucose bands: ADA "normal" 60-100/100-126 vs "thriving" 72-85; A1c bands (Medicine 2.0 diagnoses diabetes at 6.5 percent, too late).
- Build: line charts + colour bands as stacked-area helper series; sparkline column.
- Source: GLU ch11 (fasting thriving range 72-85 mg/dL, post-meal rise <30 mg/dL); OUT ch13 line 182 (A1c 6.5 threshold critique). Fasting insulin availability: check the data (Attia names insulin as a metabolic marker, OUT ch15 line 17).

**B11. HOMA-IR and TyG index** (S)
- Shows: HOMA-IR = glucose(mg/dL)*insulin(uU/mL)/405; TyG = LN(TG*glucose/2). Needs fasting insulin; TyG needs only TG+glucose (available on every panel).
- Source: Matthews 1985 Diabetologia 28:412; Simental-Guerrero 2008 (both EXT-unverified). Motivation: OUT ch15 line 17 (insulin, visceral fat as metabolic markers).

**B12. Liver: ALT trend, FIB-4, De Ritis** (S)
- FIB-4 = (age*AST)/(platelets*SQRT(ALT)). Source: Sterling 2006 Hepatology 43:1317 (EXT-unverified); motivation OUT ch13 lines 43 & 186 (NAFLD/NASH progression; mildly elevated ALT).

**B13. Kidney: eGFR CKD-EPI 2021 with a creatine-supplement artifact flag [TOP15]** (M)
- Shows: eGFR per panel from creatinine, age, sex (race-free 2021 equation); flag when creatine monohydrate was in the stack at draw date, because creatine raises serum creatinine without changing kidney function; same flag on PhenoAge (creatinine coefficient 0.0095 means artifact inflates biological age).
- Build: eGFR = 142*MIN(Scr/k,1)^a*MAX(Scr/k,1)^-1.200*0.9938^age (*1.012 female). Flag via lookup of supplement start dates (creatine row) vs draw date.
- Source: Inker 2021 NEJM 385:1737 (EXT-unverified); creatine dose HELMS ch02 (0.018 g/lb/d = 0.04 g/kg/d); creatinine artifact: EXT-unverified general knowledge. The coupling to PhenoAge is mine and uses verified Table 1.

**B14. Thyroid gauge** (S)
- TSH/fT4/fT3 vs lab reference and optimal bands as bullet bars. Source: David's own reference+optimal columns (no book).

**B15. Marker report-card heatmap (56 markers x 7 panels) [TOP15]** (M)
- Shows: each cell coloured by position: below ref / in ref but outside optimal / optimal / above; plus "% of panels optimal" column and last-vs-first arrow.
- Build: helper numeric z = (value - opt_low)/(opt_high - opt_low) with 3-colour-scale conditional format or formula-based formats; sparklines per row.
- Why: the one-page "everything" view; Medicine 3.0 emphasis on tracking earlier than thresholds. Source: OUT ch13 line 182 (act before the diagnostic threshold).

**B16. Gap-closure priority queue** (M)
- Shows: top 10 markers ranked by weighted distance from optimal, weight editable (default: ApoB, Lp(a), fasting insulin/glucose/A1c, TG/HDL, hs-CRP higher).
- Build: LARGE/INDEX-MATCH ranking, weights from data-validated 1-5 cells. Source: weights justified by OUT ch15 line 7, line 17.

**B17. Slope and next-panel projection** (S)
- SLOPE/FORECAST.LINEAR over panel dates; "worsening for 2 consecutive panels" flag. Source: mine.

**B18. Four Horsemen coverage map [TOP15]** (M)
- Shows: Attia's four chronic-disease families (atherosclerosis; cancer; neurodegeneration; metabolic dysfunction/T2D) as columns; each tracked marker, habit or screening mapped to them, with a count and recency; shows the thinnest horseman (cancer screening has no marker in a blood panel).
- Build: mapping table + COUNTIFS; stacked bar; recency = TODAY()-last date.
- Source: OUT ch04-ch28 structure (Part II: heart disease ch14-16, cancer ch17-22, neurodegeneration ch23-28, metabolic ch13); "four horsemen" framing in the book's early chapters (not re-verified for the exact phrase).

**B19. Cancer screening calendar** (S)
- Shows: next-due dates for screenings David records (colonoscopy, skin, PSA/4Kscore etc.). Attia notes validated screening exists for only five cancers (lung for smokers, breast, prostate, colorectal, cervical); he treats screening as aggressive and net beneficial.
- Build: last-done + interval(user) = due; TODAY() conditional format. Intervals user-supplied (no numbers captured from book).
- Source: OUT ch22 lines 3, 15, 25.

**B20. DEXA sheet: BMD, lean mass, ALMI, visceral fat [TOP15]** (M)
- Shows: annual DEXA entries; ALMI (appendicular lean mass / height^2); visceral fat mass (g/lb); T/Z-score; hip fracture risk context.
- Why: Attia measures BMD yearly with DEXA, which also gives body fat and lean mass; a 40-year-old 200 lb man with 20% body fat and only 4.5 lb visceral fat is in the top 5% of CVD/T2D risk; lowest ALMI quartile had ~50% mortality over 12 years in a Chilean cohort (the sentence is truncated in my read: "approximately 50 percent of those in the lowest qua[rtile]...").
- Source: OUT ch35 lines 19-25 and 109; ch13 line 114.

**B21. Hearing and cognition annual slots** (S)
- Hearing loss associated with Alzheimer's through withdrawal; audiogram annually as a slot. Source: OUT ch28 line 29.

---

## C. Exercise: the Centenarian Decathlon, zone 2, VO2 max, strength

**C1. Centenarian Decathlon tracker [TOP15]** (L)
- Shows: a table of ~20 tasks (Attia's list of 10 + extras), target age, target performance, "capacity required today", current measured capacity, pass/fail, and a radar-like bar of % of required.
- Inputs: DOB, task list with target performance and capacity metric, current test result (date), decline rate per decade (8-17%/decade for strength, 10-15%/decade VO2).
- Build: required_now = target_at_age / (1 - decline)^((target_age - current_age)/10). Check vs book: 30 lb child at 80 requires ~50-55 lb now at 40 (30/(1-0.12)^4 = 50.0). Data validation on decline cells; conditional bars; bullet chart.
- Source: OUT ch31 (task list, 10 core items: hike 1.5 mi hilly trail, floor-to-stand with one arm, lift child, carry 2x5 lb groceries 5 blocks, 20 lb suitcase overhead, 30 s single-leg balance, sex, 4 flights stairs in 3 min, open a jar, 30 jump-rope skips; extras 11-20 incl. 5 pull-ups, 1 min dead hang, 90 steps in 2 min = VO2 32). Decline: 8-17%/decade strength (ch31 text), VO2 10%/decade, up to 15% after 50 (ch34).

**C2. VO2 max lifetime projection [TOP15]** (M)
- Shows: projected VO2 from current value to age 100 with 10%/decade decline (15% after 50), horizontal threshold lines: 18 (men)/15 (women) independence floor; ~30 hiking; 32 brisk stairs/90 steps in 2 min; 35 = threshold Robert Marchand reached at 100+.
- Build: cell per decade with IF(age<50,0.10,0.15); chart: line + constant-series thresholds; second line "if +6 ml/kg/min via training".
- Source: OUT ch34 lines 24-26 (hiking VO2 ~30; 32 at 50 declining toward 21 at 80), line 30 (50% mortality reduction bottom to next quartile), line ~32 (18/15 floor), line (+6 ml/kg/min = about 12 years younger); OUT ch31 line 55 (VO2 32 = 90 steps in 2 min).

**C3. VO2 max percentile to relative mortality lookup** (M)
- Shows: from VO2 value + age + sex, which bracket (least <20th, low, moderate, fit, high, elite top 2%) and the associated hazard ratio vs elite: least fit 20% = 4.09x; moderate ~ >2x; bottom quartile ~4x vs top quartile; 5x vs top 2.3%.
- Build: needs a norm table (age/sex percentile cut-offs from FRIEND/Cooper, EXT) as lookup sheet; hazard table from the 750k-veteran study text. Display relative-risk bar. Not additive with other risk views.
- Source: OUT ch30 lines 61-65 (JAMA 2018 120k people, JACC 2022 750k veterans: 4.09x). Norm table EXT (Figure 11 in book is an image; not extractable).

**C4. "Elite for sex, two decades younger" VO2 target** (S)
- Shows: target = elite (top 2%) VO2 for sex at age-20. Source: OUT ch34 lines 22 ("elite range ... then two decades younger"). Needs the norm table from C3.

**C5. Zone 2 weekly volume tracker [TOP15]** (M)
- Shows: weekly zone 2 minutes vs 180 target (3 h/wk, e.g. 4x45 min; 2x30 as entry), rolling 4-week mean, streak; stacked column by modality; data bars.
- Inputs: workout log with date, modality, minutes in zone 2 (Whoop zone 2 or HR-derived).
- Build: SUMIFS by ISO week; chart columns + target line; sparkline.
- Source: OUT ch33 lines (entry 2x30 min/wk; "about three hours per week ... minimum"; Attia himself four 1-hour sessions/week).

**C6. Zone 2 HR range calculator** (S)
- Shows: zone 2 HR = 70-85% of actual maximum HR (not estimated); fallback estimate HRmax = 208 - 0.7*age; talk test field; lactate 1.7-2.0 mmol/L if tested.
- Source: OUT ch33 (70-85% of true max; lactate 1.7-2.0 mmol; talk test); Tanaka 2001 JACC 37:153 (EXT-unverified).

**C7. Zone 2 power per kilogram progression** (M)
- Shows: watts/kg = average zone 2 watts / body weight (pull weight from daily log by date); bands 2 W/kg reasonably fit, 3 very fit, 4+ pro cyclists (zone 2 benchmark; sedentary metabolic syndrome ~1 W/kg).
- Build: scatter over time with band lines. Source: OUT ch33 (watts/kg paragraph; San Millan and Brooks study).

**C8. VO2 interval session log (4x4) and training-age gate** (S)
- Shows: sessions per week of 4 min hard / 4 min easy intervals; gate that unlocks "VO2 block" after ~5-6 months of consistent zone 2 for new exercisers; hill rucking counts.
- Source: OUT ch34 line 5 (5-6 months) and line 44 (4-min intervals, 3-8 min range).

**C9. Annual VO2 test card and cheap proxy (resting HR)** (S)
- Shows: lab-tested VO2 once a year; interim estimate = 15.3 * HRmax/HRrest (Uth 2004, Eur J Appl Physiol 91:111; EXT-unverified) from Whoop resting HR; drift flagged "estimate".
- Source: OUT ch34 line 14 (annual test); proxy EXT.

**C10. Strength pillars scorecard** (M)
- Shows: grip, carry, pull, hip-hinge with standards: farmer's carry half bodyweight in each hand (full BW total) for 1 min (men), 75% of that for women; dead hang >= 2 min men / 90 s women at 40, reduced each decade after 40; rucking load goal 25-33% BW.
- Build: per-decade reduction parameter (user-set; book gives no number); pass/fail; progress bars.
- Source: OUT ch35 lines 53, 57-65, 73, 77.

**C11. Muscle as retirement account** (M)
- Shows: current lean mass/ALMI, projected loss to age 80 (~40% less quad cross-section vs 25), strength 8-17%/decade, with and without training; "reserve" gauge. Falls: hip fracture 1-year mortality 15-36% over 65 (context line).
- Source: OUT ch35 lines 7, 25, 43; ch31.

**C12. Lifting frequency and heavy-day counter** (S)
- Shows: sessions/week vs Attia's own "heavy lifting four times per week" (his routine, not a prescription). Source: OUT ch30 line 128.

**C13. Hypertrophy volume landmarks per muscle group [TOP15]** (L)
- Shows: weekly hard sets per muscle (from workout list) against editable MEV / MAV / MRV; colours: below MEV red, MEV-MAV green, MAV-MRV amber, above MRV red.
- Build: workout table (exercise, muscle, sets, RIR, date) -> SUMIFS by week and muscle; per-muscle bullet charts; data validation on RIR (0-5; sets only count if within 5 RIR).
- Source: ISRA ch03 lines 51-55 (definitions), ch06 lines 217-233, 266-268, 301-307 (MRV, stimulus threshold: >30% 1RM, within 5 RIR). The book gives definitions, not numeric landmarks in my read; leave the numbers user-owned.

**C14. Mesocycle volume ramp planner** (M)
- Shows: weeks 1-6 sets/muscle ramping MEV to MRV then deload; actual vs plan.
- Build: linear interpolation formula + deload week; chart. Source: SCHO ch12 line 233 (systematic volume increase across a cycle) and ISRA ch06 lines 375-387 (rest days, recovery sessions when MRV approached too early).

**C15. Volume dose-response reference chart** (S)
- Radaelli: 1, 3, 5 sets per exercise over 6 months: elbow flexor effect sizes 0.10, 0.73, 1.10; extensors 0.05, 0.05, 2.33. Static reference chart to anchor "more sets = more growth until MRV".
- Source: SCHO ch09 line 23.

**C16. Frequency (muscle hits per week) counter** (S)
- Muscle-group frequency per week from workouts; volume-equated studies show little difference between split and total body; use as tidy heatmap. Source: SCHO ch09 lines 47-65.

**C17. Effort/RPE log and cardiac drift** (M)
- Shows: RPE (Borg) vs HR vs pace/power per zone 2 session; drift = HR second half / first half at same power; flags sessions drifting >5% (threshold mine).
- Source: ENDU ch09 lines 27-29 (Borg scale; perceived exertion as the governor); OUT ch33 (talk test). Whoop provides HR.

**C18. Sauna sessions** (S)
- Weekly dry-sauna session count with study reference: >2,300 Finnish men: up to 7x/week had ~2x lower heart disease, fatal heart attack and all-cause mortality vs 1x/week. Attia now takes dry sauna seriously for dementia/ASCVD.
- Source: LIFE ch00.md lines 4219-4229; OUT ch28 line 35.

**C19. Post-meal walk log** (S)
- Shows: yes/no + minutes after each main meal; weekly % of meals followed by 10-15 min walks; correlate with CGM spike. Evidence: Inchauspé hack 8; bookmarked tweet cites trials (post-meal beats pre-meal; 10-min post-meal beat a single 30-min walk by 12% in 3-h iAUC) (unverified secondhand).
- Source: GLU ch23 (Hack 8); TW 2014400965445931450.

**C20. Cardio-vs-lifting time budget** (S)
- Helms rule for dieters: total cardio no more than half of lifting time; low-impact; cap HIIT. Show actual ratio.
- Source: HELMS ch01 lines 1047-1061.

---

## D. Nutrition

**D1. Protein target bands vs daily log [TOP15]** (M)
- Shows: daily protein g/kg of bodyweight vs bands: RDA 0.8; Attia minimum 1.6; "good place to start" 2.2 g/kg (1 g/lb) for active people; safe upper 3.7 g/kg; per-meal ~0.25 g/lb x 4 meals; % of days at/above 1.6.
- Inputs: daily weight, protein (Nov 2023-Jul 2024 log). Build: ratio column, COUNTIFS for % days, line + 3 target lines, data bars. 
- Source: OUT ch50 lines 9-17; HELMS ch01 lines 1435-1467 (dieting protein: 0.72 g/lb studied; 1.1-1.3 g/lb "would not be harmful"). Tweet TW 2087208922822095017 gives protein 1 g/lb, fat 0.3 g/lb (weak).

**D2. Weight-change rate checker [TOP15]** (M)
- Shows: 7-day moving average of weight; weekly % bodyweight change vs recommended 0.5-1.0% loss/week (cut) and gain bands (beginner 1-1.5%/month, intermediate 0.5-1%, advanced up to 0.5%/month); streak of in-band weeks.
- Build: AVERAGE over 7 rows, weekly delta/weight, IF band, chart with band shading via stacked area helper.
- Source: HELMS ch01 lines 955 and 1090-1127.

**D3. Adaptive energy expenditure estimate** (M)
- Shows: implied TDEE = mean calories - (7700 kcal/kg * weekly weight change / 7); rolling, with the "3,500 kcal/lb" rule shown plus the book's footnote that the rule is crude (Hall 2011 cited by Helms).
- Source: HELMS ch01 line 957 (3500 kcal ~ 1 lb adipose) and footnote "Why is the 3500 kcal per pound weight loss rule wrong?" (Hall & Chow). Use as estimate only.

**D4. Macro split vs targets** (S)
- Shows: fat % calories in 15-25% (cut) or 20-30% (gain), fat floor ~0.25 g/lb; protein per kg; carbs remainder; per-day stacked column, target band.
- Source: HELMS ch01 lines 1493, 1515, 1617.

**D5. Daily Dozen checklist grid [TOP15]** (M)
- Shows: 12 categories x days; servings with data validation (0-N), per-category weekly %, total score out of 24 servings/day-ish, sparkline per category; compliance heatmap.
- Targets (Greger): beans 3; berries 1; other fruits 3; cruciferous 1 (30-80 g); greens 2 (cruciferous or not); other vegetables 2; flaxseed 1 tbsp ground; nuts/seeds 1 (30 g); herbs/spices 1 (1/4 tsp turmeric); whole grains 3; beverages 5 glasses (12 oz); exercise 90 min moderate or 40 min vigorous. Counts for berries and other vegetables come from Figure 6 (an image I could not read), so confirm those two.
- Source: GREG ch20 (serving sizes, 3 beans/fruit/grains, 5 beverages, exercise), ch30 (3 whole-grain servings), ch27 (flax tbsp).

**D6. Nutrition-items to Daily Dozen auto-mapper** (M)
- Shows: each nutrition item tagged with a Daily Dozen category and serving size; daily intake table sums via SUMIFS. Source: GREG ch20.

**D7. Nutrient coverage matrix (foods + supplements) [TOP15]** (L)
- Shows: nutrients (rows) x sources (foods, supplements) with total vs RDA/target; heat red where under 100% or over UL. Pre-seed with high-yield gap list: vitamin D, calcium, zinc, magnesium, iron flagged as the five deficiencies bodybuilders on diets show; B12 and iodine for plant-based diets.
- Inputs: nutrition items w/ micros (needs data), supplements w/ elemental amounts, RDA/UL table (EXT: NIH ODS).
- Source: HELMS ch02 line 9; GREG ch35 (B12 2,500 mcg/week or 250 mcg/day; over 65: 1,000 mcg/d; D3 2,000 IU/day with largest meal; iodine 150 mcg/day).

**D8. Supplement dose vs guideline checker** (M)
- Shows: for each reference-dose row (D3 2,000 IU Greger; D3 0.5-2 mcg/kg/d i.e. 20-80 IU/kg Helms; B12 250 mcg/d; EPA+DHA 2-3 g/d; creatine 0.04 g/kg/d; iodine 150 mcg), David's total (SUMIFS over supplements by ingredient) with under/ok/over flag.
- Source: HELMS ch02 (EFA 2-3 g; vit D3 0.225-0.9 mcg/lb; creatine 0.018 g/lb); GREG ch35.

**D9. Vitamin D dose x 25-OH-D lab loop** (S)
- Scatter: supplemental IU/d vs serum 25-OH-D at each panel; note Attia: higher vitamin D correlated with better memory in e4/e4 (no proof supplementing helps). Source: GREG ch35; HELMS ch02 line "bloodwork"; OUT ch28 line 37.

**D10. B12 / folate / homocysteine loop** (S)
- Dose in stack vs serum B12 and homocysteine; MTHFR genotype slot (F5). Source: GREG ch35 (B12); OUT ch28 line 37 (lowering homocysteine with B vitamins, optimising omega-3 in e4 carriers).

**D11. Omega-3 EPA+DHA sum and TG response** (S)
- SUM of EPA+DHA from fish-oil rows vs 2-3 g target; TG trend overlay. Source: HELMS ch02.

**D12. Glycemic-hack compliance + spike tally** (M)
- Shows: daily checkboxes for Inchauspé's hacks (veg first / green starter, savory breakfast, vinegar 1 tbsp before sweet or starchy, post-meal movement, dessert not snack, carbs with clothes) and optional CGM spike count: spike = post-meal rise >30 mg/dL, fasting 72-85 "thriving" (ADA normal is 60-100).
- Source: GLU ch11, ch16-25; vinegar reduces spike 8-30% (ch22).

**D13. CGM insight panel** (M)
- If CGM data is imported: mean glucose, SD, CV, time in 70-110, spikes; sleep 5-6 h vs 8 h = ~10-20 mg/dL higher peak and 5-10 mg/dL higher overall; refined carbs spike more; exercise: aerobic lowers, lifting/HIIT transiently raises.
- Source: OUT ch49 points 1-6 (lines 3-15).

**D14. Fibre per 1000 kcal and daily fibre** (S)
- Attia targets >50 g/day. Source: OUT ch49 line 3.

**D15. Yuka score vs macro quality scatter** (M)
- Nutrition items: x = Yuka score, y = protein per 100 kcal, bubble = price per serving; quadrant highlight "high protein, high Yuka, cheap". Source: David's nutrition table; protein density rationale OUT ch50.

**D16. Cost per gram protein ranking** (S)
- $/g protein for nutrition items and protein powders; SORT via helper rank; data bars. Whey isolate richer in amino acids than soy (OUT ch50 line 25). Source: mine + OUT ch50.

**D17. Eating-window and meal-skip tracker** (S)
- First/last meal time -> window length; days with a skipped meal. Reference: 16:8, 5:2, quarterly extended fast as practised by Attia (per Sinclair), Sinclair skips ~one meal daily. Attia himself presents time-restricted eating as "the case for and against" (ch52) so no target imposed.
- Source: LIFE lines 3770-3776 and 10641; OUT ch52 title.

**D18. Alcohol log vs sleep/HRV** (S)
- Drinks/day vs next-night sleep efficiency and HRV. Source: OUT ch60 rule 1 (alcohol impairs sleep more than any other factor); WWS Twelve Tips #4.

---

## E. Sleep and routine timing

**E1. Sleep opportunity and regularity [TOP15]** (M)
- Shows: bedtime/wake per night, time-in-bed vs the 8-hour minimum (9 preferred) opportunity, wake-time SD (fixed wake rule), weekend drift.
- Build: wake SD = STDEV over 14 nights (times as fractions); conditional format when TIB < 8 h; chart with target line.
- Source: OUT ch60 rules 8-9 (opportunity 8-9 h, fixed wake time even on weekends); WWS ch21 tip 1 ("if only one tip: keep the schedule; set a bedtime alarm").

**E2. Sleep efficiency tracker** (S)
- Efficiency = asleep/in-bed; >=90% good; teens ~95%; 70-80% typical in the 80s (1-1.5 h awake).
- Source: WWS ch06; PDF copy lines 2744-2756.

**E3. Sleep architecture view** (M)
- Stacked column of light/deep/REM minutes by night (Whoop); expected cycles = TIB/90; first-half-of-night deep vs late-night REM note; cumulative stage % vs a personal baseline (no population percentages captured in my read).
- Source: WWS PDF copy lines 1192-1210 (90-min cycles; deep NREM dominates early cycles, REM late); OUT ch58 line 25 (trackers estimate stage and duration "reasonably but not perfectly").

**E4. Caffeine clock [TOP15]** (M)
- Shows: remaining caffeine mg at bedtime = SUM(dose_i * 0.5^((bedtime - t_i)/half-life)); default half-life 6 h (range 5-7); bar of "mg still active at bedtime"; flag > threshold.
- Inputs: caffeine log (coffee/tea/pre-workout and caffeine-containing supplements with time of day from the supplements table).
- Build: SUMPRODUCT with 0.5^(...) over a log table; half-life in a data-validated cell (5-8); optional genotype-informed value (F11).
- Source: WWS ch03 (half-life 5-7 h; coffee at 7:30 pm leaves 50% at 1:30 am; full effect can take up to 8 h, WWS ch21 tip 3); PDF copy lines 775-781.

**E5. Sleep-hygiene scorecard** (S)
- Shows: yes/no nightly checkboxes from Outlive's 10 rules (no alcohol, no food 3 h before bed, no stimulating screens 2 h before, 1 h wind-down, sauna/hot bath before bed, room mid-60s F, fully dark, 8-9 h opportunity, fixed wake time, don't clock-watch) and Walker's 12 tips; score vs next-day sleep score/HRV (CORREL).
- Source: OUT ch60 (rules 1-10), WWS ch21 Appendix (12 tips; PDF lines 9812-9860).

**E6. Last-meal and last-supplement gap** (S)
- Shows: hours between last food and bedtime (>=3), last caffeine, last stimulating supplement; flags from the supplement time-of-day tags.
- Source: OUT ch60 rule 2; WWS tips 3, 5, 6 (medications/herbal remedies can disrupt sleep; move to early day).

**E7. Sleep debt ledger** (S)
- Shows: rolling 7-day deficit vs 8 h and a "recovered?" line; weekend sleep-in does not fully compensate.
- Source: WWS tip 1 (sleeping late on weekends won't fully make up). Threshold 8 h: WWS line 88; OUT ch60.

**E8. Sleep x glucose coupling** (M)
- Scatter of sleep hours vs next-morning fasting glucose / peak post-meal glucose (CGM); regression line (SLOPE). Attia: 5-6 h vs 8 h = +10-20 mg/dL peak. Source: OUT ch49 line 13.

**E9. Chronotype card and schedule fit** (S)
- Chronotype: morning ~40%, evening ~30%, intermediate ~30% of people; self-selected; compare habitual mid-sleep time with workout/work times. Source: WWS ch03; PDF lines 556-563. Genotype link F12.

**E10. Morning-routine Gantt [TOP15]** (M)
- Shows: routine steps (name, start, duration, category) as a Gantt via stacked bar (invisible start series); overlays: light exposure window, caffeine, supplements by time of day, exercise.
- Build: bar chart with first series no-fill; start as time fraction; category colours; secondary sheet "ideal night plan": sauna/bath, dark room, cool room. Include 30 min morning sunlight (WWS tip 11) and exercise no later than 2-3 h before bed (WWS tip 2).
- Source: David's routine sheet; WWS ch21 tips 2, 9, 11; OUT ch60 rules 5-7.

**E11. Supplement schedule grid with absorption rules** (M)
- Rows = 87 supplements; columns = hour blocks (data-validated time of day); flags: D3 with the largest meal (fat-soluble), creatine timing irrelevant long-term, stimulants away from bed. Cost-per-time-block bar.
- Source: GREG ch35 (D3 "ideally with the largest meal"); HELMS ch02 (creatine timing doesn't matter for long-term consumption).

**E12. Sleep sensors cross-check: HRV and RHR trend** (M)
- Overnight HRV and resting HR from Whoop with 7- and 28-day baselines; deviations highlight strain/illness/alcohol. Attia uses HRV among sleep-tracker inputs. Source: OUT ch58 line 25.

**E13. Incline-bed and apnea screening slot** (S)
- STOP-BANG items checklist (snore, tired, observed apnea, blood pressure; Attia says it correlates with formal testing) and a flag; tweets recommend inclined bed (weak evidence). Source: OUT ch58 line 55; TW 2104707480353959958 (single tweet, claims a review of 5 controlled trials, unverified).

---

## F. Genetics cross-cuts

The library contains no genetics text; usable anchors are Outlive (APOE e4, Lp(a), homocysteine) and Sinclair (sirtuins, epigenetic clock). Everything SNP-specific below is EXT-unverified: the builder should source each rsID and effect from SNPedia/ClinVar before display, and show Promethease magnitude/repute as given.

**F1. Genotype x biomarker x supplement cross-cut table [TOP15]** (L)
- Shows: one row per curated gene/SNP: gene, rsID, David's genotype (from 23andMe/WGS), what it modulates, the biomarker to watch, latest value and trend, the supplement/behaviour in his stack that relates, and a status tile (concordant/discordant/untested).
- Build: Excel table; genotype via lookup from the genome sheet (XLOOKUP/INDEX-MATCH on rsID); biomarker via lookup into the labs sheet by marker name and MAX(date); autofilter; conditional format on status.
- Seed rows: APOE (rs429358/rs7412) -> ApoB, fasting insulin/glucose, homocysteine (OUT ch27-28); LPA -> Lp(a) (OUT ch14); MTHFR C677T/A1298C -> homocysteine, folate, B12 vs methylfolate in stack; CYP1A2 rs762551 -> caffeine half-life; GC/CYP2R1/VDR -> 25-OH-D vs D3 dose; SLCO1B1 rs4149056 -> statin tolerance (relevant given OUT ch16 rosuvastatin discussion); 9p21 rs10757274 -> ASCVD risk context; FTO/MC4R -> weight response; TCF7L2 -> glucose; HFE -> ferritin/iron; ALDH2/ADH1B -> alcohol response; PER3/CLOCK -> chronotype.
- Source: OUT ch27, ch14, ch28; others EXT-unverified.

**F2. APOE decoder and e4 prevention checklist [TOP15]** (M)
- Shows: combine rs429358 and rs7412 into e2/e3/e4 diplotype using a 9-row lookup (TT/CC -> e3/e3, etc.); if e4, show Attia's e4-specific plan as a checklist tied to David's data: metabolic health first (glucose, insulin, TG:HDL), exercise (steady endurance) as "most powerful tool", Mediterranean-style diet and fatty fish, B vitamins to lower homocysteine, omega-3, vitamin D, dry sauna, hearing, sleep. Fact tile: a woman with one e4 copy is 4x as likely to develop Alzheimer's as a man with the same genotype.
- Build: INDEX/MATCH on concatenated genotypes; checklist rows reference live data (e.g. zone 2 min/week, homocysteine latest).
- Source: OUT ch27 lines 3-17; ch28 lines 7, 13, 29, 35, 37, 55. Allele-combination table EXT-unverified (standard, SNPedia rs429358/rs7412).

**F3. Lipid-genes versus measured ApoB** (M)
- Shows: annotated ApoB chart with genotype banners (APOB, LDLR, PCSK9, LPA, APOA5, CETP hits from Promethease). "Genetically high ceiling, or lifestyle?" Attia: some lack response to diet and need drugs; PCSK9 inhibitors lower Lp(a) ~30% (not shown to lower events).
- Source: OUT ch15 line 31, ch14 line 167; SNP effects EXT-unverified.

**F4. LPA genotype vs Lp(a) card** (S)
- Lp(a): value or "get tested once"; LPA SNP status; family history of premature heart attack slot (Attia: test if so). Source: OUT ch14 lines 157-167.

**F5. MTHFR to methylfolate/homocysteine panel** (S)
- Genotype + homocysteine + serum folate + B12 + the stack's folate form (folic acid vs methylfolate). Clinical value of MTHFR genotype is contested (EXT); keep the lab as arbiter. Source for homocysteine/B vitamins: OUT ch13 line 186, ch28 line 37.

**F6. Promethease browser** (M)
- Autofilter table of hits (SNP, gene, genotype, magnitude, repute, summary) plus a category column (cardio, metabolic, sleep, nutrient, longevity); category counts; "actionable-by-existing-biomarker" flag column pointing at F1. rust_xlsxwriter: table + autofilter; Excel has no slicer support there so use filter buttons and category counts via COUNTIFS.
- Source: David's Promethease export.

**F7. Pharmacogenomic flags vs supplement/drug list** (M)
- Shows: CYP/transport SNPs next to each supplement/drug that uses the enzyme; red flag where an inhibitor/inducer is in the stack. EXT-unverified: SLCO1B1 rs4149056 and simvastatin myopathy (Link 2008 NEJM 359:789).

**F8. Haplogroup identity card** (S)
- mtDNA and Y haplogroups as a static card with phylogeny path; tie-in note: mitochondria matter to aging (hallmark, LIFE lines 1040-1042) but haplogroup health claims are weak. Source: David's haplogroup output.

**F9. Genome QC sheet: array vs WGS concordance** (M)
- Counts of SNPs in both sources, concordance %, no-calls; checks on key rsIDs from F1; rendering conditional formats. Source: mine.

**F10. Genotype-triggered lab to-do** (S)
- Rule table: IF e4 present -> make sure insulin, homocysteine, ApoB, hs-CRP on next panel; IF high Lp(a) risk genotype -> Lp(a); IF 9p21 risk -> coronary calcium score discussion. Show "already measured? latest date".
- Source: OUT ch28 lines 7, 37; ch14 line 167; the rule list is mine.

**F11. Caffeine half-life personalisation** (S)
- Half-life parameter feeding E4: default 6 h; slow-metabolizer genotype default 8 h (editable); display source. EXT-unverified for the genotype effect; WWS gives 5-7 h average.

**F12. Chronotype genotype card** (S)
- PER3/CLOCK/RS57875989 etc. vs behavioural mid-sleep. EXT-unverified. WWS ch03 says chronotype is genetically determined (PDF lines 556-563 discuss types; the heritability claim is the book's).

**F13. Longevity-allele card** (S)
- FOXO3 rs2802292, APOE e2, CETP, IL6 etc. as an annotated card; honest label: small effects. Sinclair frames sirtuins/mTOR/AMPK as longevity genes (LIFE lines 1331-1396), not SNP-level. EXT-unverified.

**F14. Epigenetic-clock placeholder sheet** (S)
- Slots for a DNA-methylation clock test result (Horvath/GrimAge/DunedinPACE), plotted beside PhenoAge, since Sinclair centres the epigenome as the upstream cause of aging (LIFE lines 1182-1190, 2256-2260). Source: Sinclair lines cited.

---

## G. Cross-cuts, targets, costs, experiments

**G1. Blueprint-vs-me gap view [TOP15]** (M)
- Shows: per marker: latest value, Blueprint target, reference range, optimal range, direction-of-good (low/high/mid), absolute gap, % of way from reference edge to target, a bullet-chart bar, ranked by size of gap; top-10 "furthest from Blueprint".
- Build: Excel table; helper columns; conditional data bars; sparkline of history; sort via helper rank (rust_xlsxwriter writes formulas; dynamic arrays like SORTBY may need `write_dynamic_array_formula`).
- Source: David's Blueprint targets. Medicine 3.0 rationale OUT ch13 line 182.

**G2. Intervention timeline over biomarker lines** (L)
- Shows: each marker's line chart with vertical markers where a supplement/diet/training change started (needs start dates in the supplements table).
- Build: scatter series with 2 points per event (x=date, y=min/max) as vertical lines; event labels from a table.
- Source: mine; n-of-1 logic G3.

**G3. N-of-1 experiment ledger [TOP15]** (M)
- Shows: hypothesis, intervention, start/end, metric (any lab or Whoop series), before mean/SD, after mean/SD, delta, effect size (Cohen d), T.TEST p, verdict; one row per experiment.
- Source: mine; inspired by Inchauspé's vinegar self-test (GLU ch22) and the bookmarked self-experiments (TW 2106537633056969024). Statistical caveat displayed.

**G4. Outlive scorecard: ten tiles [TOP15]** (M)
- Shows: a one-page KPI strip with sparkline for each of: ApoB, TG:HDL, A1c/fasting glucose/insulin, hs-CRP, VO2 max, zone 2 min/week, grip/carry/hang, lean mass/ALMI, sleep hours+efficiency, body weight vs target; traffic-light vs thresholds from sections above.
- Source: assembled from OUT ch13-15, ch30-35, ch58-60.

**G5. Weight/calorie/protein daily dashboard (Nov 2023-Jul 2024)** (M)
- Shows: weight (daily dots + 7-day EMA), calories, protein per kg; weekly rate; sparkline per week; calendar heatmap of days hitting protein target.
- Build: EMA formula = prior*(1-a)+x*a; conditional formatting grid by week. Source: D1/D2.

**G6. Medicine 3.0 "marginal decade" planner** (M)
- Shows: lifespan vs healthspan; years David wants to be functional; "marginal decade" notion; link to C1 and C2.
- Source: OUT ch31 (Marginal Decade); ch11.

**G7. Evidence-weighted supplement quadrant** (M)
- Bubble chart: x = evidence rung (A13), y = $/month, bubble = number of hallmarks; labels; highlights "expensive and speculative". Source: mine + A7/A13.

**G8. Redundancy and stacked-dose detector** (M)
- Group supplements by active ingredient; sum amounts; compare to UL (EXT: NIH ODS) and to guideline doses (D8); flag duplicates and overlapping mechanisms.

**G9. Cost-per-day by time of day** (S)
- SUMIFS $/day by time-of-day tag; stacked bar; Bryan Johnson cost comparison (the tweet above puts Pascoe at 98.5% less). Source: TW 1857431845576798654 (claim, unverified).

**G10. Research-stance x source-protocol matrix** (S)
- Counts by (source protocol: Blueprint, Sinclair, Attia, Greger, Helms, other) x (stance). Detects "I take it because Johnson takes it, but the literature says speculative". Source: David's columns.

**G11. Mortality-risk ledger (qualitative)** (S)
- Table of protective-factor tiers with relative-risk statements as text: VO2 bottom quartile ~4x vs top quartile (OUT ch30 line 63); low muscle mass 40-50% greater mortality (ch30 line 87); top-third strength = ~48% lower mortality even in the bottom half of fitness (ch30 line 89); sauna up to 7x/week ~2x lower mortality vs once weekly (LIFE lines 4228-4230); sleep <6-7 h doubling cancer risk (WWS line 91; author claim). Mark: hazard ratios aren't additive; no composite score computed.

**G12. Longevity-drug watch sheet** (S)
- Static tracker for metformin, rapamycin, NMN/NR, resveratrol, statins/ezetimibe/PCSK9: current stance per source and whether in David's stack. Attia: rapamycin is the most interesting but dosing intermittent; Sinclair: metformin 1 g/day. Source: OUT ch12 lines 33, 109, 123; ch16; LIFE lines 4644-4734, 10633.

**G13. Biomarker "how often to retest" planner** (S)
- Each marker's retest cadence (user-set), last drawn, due date, "overdue" colour. Sinclair tests every few months (LIFE line 10646); Attia annual VO2 and DEXA.

**G14. LLM export sheet ("book on me")** (S)
- One flat sheet: demographics, last panel, trends, top gaps, genome flags, stack, goals, with a redaction column. Motivated by bookmarks of people giving LLMs their DNA + blood + wearables (TW 2019818347676983531) and privacy-layered approaches (TW 2106537633056969024).

**G15. Whoop panel: HRV, recovery, strain, sleep** (L)
- Shows: weekly rollups, 7/28-day baselines, strain vs zone minutes, HRV vs alcohol/caffeine/late meal (CORREL matrix with conditional colours), VO2 proxy (C9).
- Source: OUT ch58 line 25 (sensor inputs); correlation design mine.

**G16. Annual "year in review" sheet** (M)
- Per-year medians of key markers, VO2, weight, sleep, spend; delta vs prior; auto-generated narrative cells via concatenation.

**G17. Lean-mass-aware body composition** (S)
- FFMI = lean mass / height^2, waist-to-height, visceral fat from DEXA, BMI shown last with caveat. Source: OUT ch13 lines 47-75, 114; TW 1769007456988860465.

**G18. Dose-response scatter template (any supplement, any marker)** (M)
- Choose supplement and marker from data-validated dropdowns (INDEX/MATCH + dynamic chart ranges via named ranges); scatter + trend; R2 via RSQ. Source: mine.

---

## Top-15 shortlist (ordered by value/effort)

1. A1 PhenoAge calculator per panel (M)
2. B15 Marker report-card heatmap, 56 markers x 7 panels (M)
3. G1 Blueprint-vs-me gap view (M)
4. B1 ApoB dashboard with target bands (M)
5. C1 Centenarian Decathlon tracker (L)
6. C2 VO2 max lifetime projection (M)
7. D1 Protein bands vs daily log (M)
8. D2 Weight-change rate checker (M)
9. A7 Hallmarks coverage matrix + A8 cost-per-hallmark (M/S)
10. F1 Genotype x biomarker x supplement cross-cut table (L)
11. F2 APOE decoder + e4 prevention checklist (M)
12. C5 Zone 2 weekly volume tracker (M)
13. E4 Caffeine clock (M)
14. E10 Morning routine Gantt (M)
15. B13 eGFR + creatine artifact flag (M) and G4 Outlive scorecard (M) tie as 15th

Also high value but not shortlisted: D5 Daily Dozen grid, D7 nutrient coverage matrix, B3 ApoB-years, B18 Four horsemen coverage, G3 N-of-1 ledger, B20 DEXA sheet, C13 volume landmarks, A11 multi-protocol comparison.

## Data-availability risks to check first

- hs-CRP on all panels (PhenoAge); fasting insulin (HOMA-IR); Lp(a); homocysteine; uric acid; platelets/AST (FIB-4); DOB, sex, height.
- Supplement start dates (timeline, creatine artifact), active-ingredient amounts, hallmark tags, time-of-day tags.
- Whoop export schema; CGM source.
- Population norm tables (VO2 by age/sex; grip) are not in the library.

## Books and files I actually read from (skimmed, via grep and targeted reads, not cover to cover)

- Peter Attia, Outlive (2023): ch13, ch14, ch15, ch16, ch22, ch27, ch28, ch30, ch31, ch33, ch34, ch35, ch49, ch50, ch58, ch60; skimmed ch11-12 via grep; index.md for the chapter map.
- David Sinclair, Lifespan (2019): hallmark list (lines 1095-1182), sirtuin/TOR/AMPK intro (1331-1420), calorie restriction/fasting (3575-3776), sauna (4195-4232), metformin (4644-4734), resveratrol/NMN (4817-5058), "What I do" (10605-10662).
- Matthew Walker, Why We Sleep: caffeine (ch03; PDF lines 775-781), chronotypes (ch03; lines 556-563), sleep efficiency (ch06; lines 2744-2756), 90-minute cycles (lines 1192-1210), Twelve Tips (ch21; lines 9812-9860), intro claims (lines 88-94).
- Michael Greger, How Not to Die: ch20 Daily Dozen, ch30 whole grains, ch35 Supplements appendix.
- Inchauspé, Glucose Revolution: ch11, ch16, ch22 (vinegar), ch23 (movement).
- Helms/Morgan/Valdez, Muscle and Strength Pyramid Nutrition: ch01 (Levels 1-2: rates, protein, fat), ch02 (supplements/micronutrients).
- Israetel, Scientific Principles of Hypertrophy Training: ch03 glossary, ch06 fatigue management.
- Schoenfeld, Science and Development of Muscle Hypertrophy (2nd ed.): ch09 (volume, frequency), ch12 (programming).
- Hutchinson, Endure: index, ch09 (Borg RPE), ch08 headings.
- de Grey, Ending Aging: index, ch05/ch08 excerpts (SENS list not read in full).
- Twitter Bookmarks (Medicine): ~60 posts skimmed by first lines; used: 2014400965445931450, 1857431845576798654, 1803400497103249519, 2019818347676983531, 2106537633056969024, 1769007456988860465, 2087208922822095017, 2104707480353959958.
- Not read: Valdez Muscle and Strength Pyramids Training (indexed, not opened beyond headings), Biotechnology patents, Neuroscience (only bookmarks present), Genetics (folder empty), WHO Essential Medicines, Lehninger.
- Web (verified this session): Levine 2018 PhenoAge Table 1 (PMC5940111); PhenoAge constants from the mcp-phenoage-clock README.
