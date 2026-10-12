# Coverage: W4 AGE (aging clocks, longevity frameworks, fitness)

Every AGE id of [MASTER.md](MASTER.md) section 3, the sheet that implements it and its
status in the generated workbook. Code: `src/workbook/sheets/age/` (one module per sheet,
constants in `age/params.rs`); tests: `tests/sheet_age.rs` plus unit tests in the modules.

Status as shipped: **BN** full build over data on file or typed entries; **PH** full view over
an empty input with the `PLACEHOLDER - awaiting data` banner; **NS** view ships, a constant it
relies on is an `UNVERIFIED` Params row with `pending: <reference>`.

| id | sheet | master status | shipped | what it reads | notes |
| --- | --- | --- | --- | --- | --- |
| AGE-01 | Age_Pheno | PH | PH | tblPanels, tblLabs, Calc_Marker, DOB | Nine-input checker, xb, mortality M, PhenoAge, acceleration vs y = x. hs-CRP never measured: "lite" mode via `Assumed_CRP`. `PhenoAge_intercept` (infra seed) is blank + UNVERIFIED, so PhenoAge stays blank until it is typed. Weights verified (Levine 2018 Table 1). [DX-RISK] neutral wording. KPIs `K_AGE_phenoage`, `K_AGE_pheno_accel`. |
| AGE-02 | Age_Vendor | BN | BN | tblLabs (`biological-age`), DOB, Age_Pheno | Vendor age, chronological age, gap, PhenoAge on or before each draw. KPI `K_AGE_vendor_gap`. |
| AGE-03 | Age_PhenoWhatIf | PH | PH | Calc_Marker.last, Age_Pheno inputs | Override column (validated > 0), swap-one-marker rows, waterfall (hidden-base stack), non-additivity note. |
| AGE-04 | Age_Claims | BN | BN | tblAgeClaims (typed) | Pascoe and unnamed claims seeded from the bookmarked tweets; Johnson row blank. Labelled "different clock". |
| AGE-05 | Age_Epi | PH | PH | tblAgeEpi (typed), Age_Pheno | Horvath / Hannum / GrimAge / DunedinPACE slots plotted beside PhenoAge. |
| AGE-06 | Age_Hallmarks | PH | PH | tblSuppTags (h_*), tblSupp, tblEvidence, Ref_Hallmarks | Supplements x 9 matrix, $/yr per hallmark (split evenly), stance heat grid, GAP flag at <= `Age_Gap_max` taken. Tags empty, user-owned, MED-VERIFY. KPI `K_AGE_hallmark_gaps`. |
| AGE-07 | Age_SENS | PH | PH | tblSuppTags (s_*), tblSupp, tblEvidence, Ref_SENS | Same matrix over the 7 SENS categories; empty rows kept as honest answers. Category list UNVERIFIED (Ref_SENS). |
| AGE-08 | Age_Protocols | BN | BN | tblSupp, tblEvidence, tblDaily, tblWearable, Calc_Marker, Age_Zone2 | Sinclair list with dose ratios (LIFE l.10605-10662), multi-protocol matrix with a source note per filled cell and a "compare with" selector, longevity-drug watch. Blueprint column blank (no library source). |
| AGE-09 | Age_Horsemen | BN | BN | tblAgeHorsemen mapping (seeded, editable), Calc_Marker | Count, recent / stale / missing per horseman, stacked bar. KPI `K_AGE_horsemen_thin`. |
| AGE-10 | Age_DEXA | PH | PH | tblAgeDexa (typed), Height_cm | BMD, T/Z, lean mass, ALMI, FFMI, visceral fat; BMI last with caveat. |
| AGE-11 | Age_Decathlon | PH | PH | tblAgeDecathlon (13 seeded tasks + spare rows), DOB | required = target / (1 - decline)^(years / 10) (unit-tested: 30 lb at 80 -> 50 lb at 40 with 12%). Declines default to the infra seeds `Decline_*_pct_decade` (blank, UNVERIFIED). KPI `K_AGE_decathlon_pass`. |
| AGE-12 | Age_VO2 | PH | PH + NS | tblAgeVO2 (typed), tblWearable.rhr, DOB, Sex | Annual test card, Uth proxy (`Age_VO2_RHR_k` UNVERIFIED), projection to 100 with thresholds (OUT ch31/ch34), norm table empty and UNVERIFIED (FRIEND / Cooper), percentile bracket, hazard and elite-two-decades-younger lookups. KPI `K_AGE_vo2_latest`. |
| AGE-13 | Age_Zone2 | BN (GATE-FIELDS) | PH | tblAgeCardio (typed) | GATE-FIELDS failed: `tblWorkout` has no date / duration / zone (assumption A6), so sessions are logged in `tblAgeCardio`. Weekly minutes by modality, 180-min target, rolling 4 weeks, streak. KPI `K_AGE_z2_week`. |
| AGE-14 | Age_Z2Tools | NS | NS | DOB, tblDaily, tblAge4x4 / tblAgeDrift / tblAgeWkg (typed), Age_Zone2 | 70-85% of true max (OUT ch33); fallback `Age_HRmax_a/b` (208 - 0.7 x age) UNVERIFIED pending Tanaka 2001. W/kg bands, 4x4 gate, drift flag. |
| AGE-15 | Age_Strength | PH | PH | tblAgeStrengthTests (typed), tblDaily, Age_DEXA | Pillars with cited standards (OUT ch35), per-decade reduction user-owned, retirement-account projection, Radaelli chart (SCHO ch09 l.23). |
| AGE-16 | Age_Volume | BN (GATE-FIELDS) | PH | tblAgeLift (typed), tblWorkout names/muscles, Age_Zone2 | GATE-FIELDS failed (no sets / RIR / date in `tblWorkout`). Hard sets per muscle vs MEV / MAV / MRV (user-owned), mesocycle ramp, days hit, sessions/week, cardio : lifting ratio. KPI `K_AGE_sessions_week`. |
| AGE-17 | Age_Sauna | BN | BN | tblAgeSaunaLog / tblAgeWalkLog (typed), tblRoutine | Weekly sessions vs reference, post-meal walks and share of meals walked. KPI `K_AGE_sauna_week`. |
| AGE-18 | Age_Decade | BN | BN | tblAgeDecade / tblAgeRiskLedger (typed, seeded), Age_Decathlon, DOB | Decades with linked Decathlon task results; qualitative ledger with the non-additive banner. |

## Parameters

All AGE constants are `Params` rows registered by `age/params.rs` (prefix `Age_`), plus the infra seeds
`PhenoAge_*`, `Assumed_CRP`, `Decline_Strength_pct_decade`, `Decline_VO2_pct_decade`. UNVERIFIED (pending):
`Age_PA_t_months` (Levine 2018 Supplement 1), `Age_VO2_decline_post50_pct` (OUT ch34),
`Age_VO2_RHR_k` (Uth 2004), `Age_HRmax_a`, `Age_HRmax_b` (Tanaka 2001). Catalogued book statements
(file + line) ship `verified` with that citation; tunable choices are `heuristic`.

## KPIs (10 of 10)

`K_AGE_phenoage`, `K_AGE_pheno_accel`, `K_AGE_vendor_gap`, `K_AGE_hallmark_gaps`, `K_AGE_horsemen_thin`,
`K_AGE_decathlon_pass`, `K_AGE_vo2_latest`, `K_AGE_z2_week`, `K_AGE_sessions_week`, `K_AGE_sauna_week`.
