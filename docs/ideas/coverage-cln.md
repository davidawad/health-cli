# Coverage: W2 clinical derived indices and flags (CLN)

Every CLN idea of [MASTER.md](MASTER.md) section 3 and where it lives. Code:
`src/workbook/sheets/cln/` (one module per sheet, helpers in `kit.rs`, parameters in
`params.rs`, marker slugs in `ids.rs`); tests: `tests/sheet_cln.rs` (which checks this
table against `cln::COVERAGE`).

| Idea | Sheet | Status | What is built |
| --- | --- | --- | --- |
| CLN-01 | Cln_Lipids | BUILDABLE-NOW | TG/HDL per panel with lines at 1 and 2 (chart); OUT ch13:186 |
| CLN-02 | Cln_Lipids | BUILDABLE-NOW | non-HDL, TC:HDL vs the reported ratio, remnant; FH pattern note at non-HDL >= 220 (DIP ch37:343) |
| CLN-03 | Cln_Lipids | NEEDS-SOURCE | Friedewald LDL-C and VLDL = TG/5 vs the lab; Martin-Hopkins over an empty factor table (`tblClnMHFactor`) |
| CLN-04 | Cln_Lipids | BUILDABLE-NOW | ApoB vs LDL-C SLOPE / INTERCEPT / RSQ, residual flag, ApoB/LDL z vs own history, scatter with trendline |
| CLN-05 | Cln_Lipids | BUILDABLE-NOW | ApoB chart with the 20-30 mg/dL goal (OUT ch16) and labelled LDL-C (not ApoB) guideline lines |
| CLN-06 | Cln_Lipids | BUILDABLE-NOW | ApoB-years: trapezoid exposure above a 25 mg/dL baseline, any marker via `Cln_Exposure_Marker`, flagged pre-history assumption |
| CLN-07 | Cln_Lipids | PLACEHOLDER | expected (DIP ch37 rule table `tblClnLdlExpected`) vs observed LDL change; GATE-DATES (`tblSupp.start_date` empty) |
| CLN-08 | Cln_RiskChecklist | PLACEHOLDER | eight priority markers: n, last date / value, NOT TESTED tiles, sparklines |
| CLN-09 | Cln_FH | PLACEHOLDER | FH / Lp(a) pattern flag over an empty family-history table (`tblClnFamilyHx`) |
| CLN-10 | Cln_Glycemia | BUILDABLE-NOW | glucose / A1c / insulin small multiples with bands; reported eAG primary, ADAG formula fills gaps |
| CLN-11 | Cln_Insulin | NEEDS-SOURCE | TyG per panel; HOMA-IR and QUICKI waiting on fasting insulin; constants UNVERIFIED |
| CLN-12 | Cln_Insulin | PLACEHOLDER | OGTT 0-120 min entry table (`tblClnOGTT`), peak minute, chart |
| CLN-13 | Cln_AnionGap | BUILDABLE-NOW | anion gap computed vs reported, DIP ch75 bands, albumin-adjusted gap (UNVERIFIED) |
| CLN-14 | Cln_Renal | NEEDS-SOURCE | CKD-EPI 2021 (both sexes until `Sex` is set) and Cockcroft-Gault vs reported eGFR; creatine flag (current stack now, dated flag GATE-DATES) |
| CLN-15 | Cln_Renal | PLACEHOLDER | KDIGO eGFR x uACR grid with the latest pair marked, awaiting uACR |
| CLN-16 | Cln_Liver | NEEDS-SOURCE | ALT trend vs reference, De Ritis, FIB-4 as a number only |
| CLN-17 | Cln_Iron | PLACEHOLDER | Hb / MCV / RDW present; ferritin, iron, TIBC, TSAT and pattern flag awaiting data |
| CLN-18 | Cln_Thyroid | BUILDABLE-NOW | TSH vs reference / optimal bands, TSH-FT4 scatter, biotin in current stack and per-draw (GATE-DATES) |
| CLN-19 | Cln_Uric | PLACEHOLDER | uric acid with 6 and 7 mg/dL lines and the asymptomatic caveat |
| CLN-20 | Cln_Derived | NEEDS-SOURCE | NLR, PLR, TLC < 1,200 note, BUN:Cr and A/G cross-checks, corrected calcium, Mentzer |
| CLN-21 | Cln_MetSyn | NEEDS-SOURCE | criteria counter per panel (labs + dated vitals table `tblClnVitals`), OUT ch13 thresholds, BP and 3-of-5 UNVERIFIED |
| CLN-22 | Cln_MetSyn | NEEDS-SOURCE | BMI (weight at panel, `Height_cm`) vs criteria-count quadrant scatter |
| CLN-23 | Cln_Systems | BUILDABLE-NOW | system tiles (markers, measured, optimal, out of range, % optimal, trend, sparkline), % optimal and burden grids, stacked chart |
| CLN-24 | Cln_NutStatus | PLACEHOLDER | albumin, TLC, prealbumin, transferrin, hs-CRP with half-lives and status |
| CLN-25 | Cln_Loops | PLACEHOLDER | pair selector (`tblClnLoopPairs`), dose at draw (GATE-DATES) vs marker, slope / R squared, charts |

KPIs (`K_CLN_*` on `Ref_KPI`): `tg_hdl`, `apob`, `apob_years`, `priority_tested`, `tyg`,
`metsyn`, `anion_gap`, `egfr`, `fib4`, `pct_optimal`.

Notes
* Constants are `Cln_*` rows on `Params`: values quoted in the library are `verified` with
  their catalog line; standard formulas the library lacks (CKD-EPI, Cockcroft-Gault,
  Friedewald, eAG, TyG, HOMA, corrected Ca, albumin-adjusted AG, Mentzer, KDIGO bounds,
  NCEP BP and 3-of-5, WHO BMI 25, vitamin D lines) are `UNVERIFIED` with `pending: <reference>`.
  The infra seeds `CKDEPI_k_*`, `FIB4_cut_*`, `HOMA_IR_divisor`, `TyG_scale` stay blank and unused.
* Marker slugs for markers not measured yet (`apob`, `insulin`, `lpa`, `hs-crp`, `ferritin`,
  `ft4`, ...) are assumptions in `ids.rs`; correct them there if biomarker-cli uses others.
* Charts plot a missing value (`""`) as zero in Excel line charts; tables are the reference.
