# Coverage: W8 GLB (global dashboard, Checks, navigation)

Every GLB id of [MASTER.md](MASTER.md) section 3, the sheet that implements it and its status in
the generated workbook. Code: `src/workbook/sheets/glb/` (one module per sheet, `Glb_*` parameters
in `glb/mod.rs`, coverage parsing in `glb/coverage.rs`); tests: `tests/sheet_glb.rs` (structure,
formulas, names, `--only` and no-lab builds) and `tests/sheet_glb/recalc.rs` (headless LibreOffice
recalculation with a typed Scenario shock: no formula errors, ALL CHECKS PASS, hand-checked numbers).

The GLB sheets open the workbook: `Nav` first, then `Dash`, `Alerts`, `Changes`, `Scenario`,
`YearReview`, `Checks`, `Export`, `Sources` and the hidden helper `Dash_Trend`.

Status as shipped: **BN** computes now with formulas over the KPI contract and the data layer;
**PH** full view over an empty input with the `PLACEHOLDER - awaiting data` banner; **NS** a constant
or rule it relies on is `UNVERIFIED` with `pending: <reference>`.

| id | sheet | master status | shipped | what it reads | what is built (Excel formulas) |
| --- | --- | --- | --- | --- | --- |
| GLB-01 | Dash, Dash_Trend | BN | BN | `K_*` names and `Params` only (Dash); `Calc_Panel`, `Calc_Labs`, `tblLabs`, `tblDaily`, `tblWearable` (Dash_Trend) | KPI strip: health index `100 x (Glb_HI_w_ref x (1 - K_LAB_pct_out) + (1 - Glb_HI_w_ref) x K_CLN_pct_optimal)`, % optimal, alerts (`K_GLB_alerts`, watch / critical in the detail line), spend per day (`K_SUP_usd_day`, light from `K_SUP_ter`), weight trend kg/week (`K_BOD_slope_lb_wk / Bod_lb_per_kg`, green when moving toward `Bod_Goal_lb`), streak (`K_BOD_band_streak`). Outlive ten-tile scorecard: ApoB (`Cln_ApoB_goal_*`, OUT ch16), TG:HDL (`Cln_TGHDL_best/ok`, OUT ch13), glycemia (TyG, no sourced cut-off so no light), hs-CRP (placeholder: never measured, no KPI), VO2 max (`Age_VO2_marchand`, sex floor, OUT ch34), zone 2 (`Age_Z2_target_min`, OUT ch33), strength (placeholder: grip / carry / hang tests), lean mass / ALMI (placeholder: DEXA), sleep (`Bod_Sleep_min_h`, OUT ch60), weight vs `Bod_Goal_lb`. Each tile: big-font value, detail line, traffic light (icon set on a 0 red / 1 amber / 2 green code, number rules), sparkline from `Dash_Trend` (8 last panels or weeks, `NA()` gaps). A tile whose KPI is not published or reads `n/a` shows "awaiting data". Board of every KPI of the build (`IFERROR(K_x,"n/a")`). Amber bands `Glb_Amber_pct`, `Glb_Index_green/amber`, `Glb_Alerts_amber`, `Glb_Streak_green`, `Glb_Weight_tol/amber` are heuristic. A test asserts no Dash formula contains a sheet reference. |
| GLB-02 | Checks | BN | BN | every table, `Calc_*`, `Params`, `Ref_KPI`, `Nav_Status`, every sheet | 28 assertions with severity block / warn / info: daily log blank and duplicate dates, missing days (warn); labs without a date, duplicate marker + date, raw value without canonical value (unit conversion), markers in more than one raw unit (warn), unknown markers, dates out of order; markers measured without a reference range (warn), duplicate marker ids; panels out of order; duplicate supplement names and taken supplements without a price (warn); reconciliation: `tblPanels.n_markers` and `Calc_Panel` vs lab rows, `Calc_Marker` draws vs lab rows, `K_DATA_n_measurements` vs `COUNT(value)`, `K_SUP_usd_yr` vs `SUMPRODUCT(taking x price x units)`, `12 x K_SUP_usd_month` vs `K_SUP_usd_yr`; Params outside min / max; counts of UNVERIFIED params, `n/a` KPIs, PH / NS / PH-or-NS views (`COUNTIF(Nav_Status,...)`). Formula-error sweep: one row per sheet (all but Checks) with `SUMPRODUCT(--ISERROR(range))-SUMPRODUCT(--ISNA(range))` over the sheet's measured used range and `#N/A` counted apart (chart columns use `NA()` on purpose). One cell: `ALL CHECKS PASS` or `CHECKS FAILING: n blocking`, plus the warning count. Nothing reads Checks (it would close a circular reference through the sweep); Dash links to it. |
| GLB-03 | Alerts | BN | BN (critical tier waits on sourced limits) | `Calc_Marker`, `tblMarkers` | Per marker: last value / date, tier from the `Calc_Marker` status (1 watch = left optimal, 2 action = out of reference, 3 critical = beyond `critical_lo/hi`), label, distance, z, days since, critical limits set; tier fills; counts; live sorted list (LARGE over a numeric sort key: tier, then distance and |z|) and an Excel 365 `TAKE(SORTBY(...))` spill. Critical limits stay blank unless sourced (MED-VERIFY banner). KPIs `K_GLB_alerts`, `K_GLB_watch`, `K_GLB_critical`. |
| GLB-04 | Changes | BN | BN + PH (supplement interval: GATE-DATES) | `tblPanels`, `tblLabs`, `Calc_Labs`, `Calc_Marker`, `tblMarkers`, `tblSupp`, `tblRegimes` | Latest / previous panel ids and dates (`Latest_Panel`, `Prev_Panel`), per marker value at both panels, change, % change, move in personal SDs, RCV % and move in RCV units (`RCV_z x sqrt(2) x sqrt(CVi^2 + CVa^2)`, blank until the CVs are typed: UNVERIFIED pending EFLM), status at both panels, new / resolved alert flags and `TEXTJOIN` lists; top movers up / down (`Glb_Top_n`) with LARGE and a `TAKE(SORTBY())` spill; supplements started / stopped and regime events between the panels (`TEXTJOIN`; waits on `tblSupp.start_date / stop_date`). KPIs `K_GLB_new_alerts`, `K_GLB_top_mover`. |
| GLB-05 | Scenario | BN | BN | `Calc_Marker`, `tblMarkers`, `tblDaily`, `Height_cm` | Inputs (blue): selected marker (drop-down of `tblMarkers`), shock %, weight shock %, plus a per-marker shock column. Shocked value, in-reference / in-optimal after, status after with the `Calc_Labs` rule (critical, out, ref-ok, optimal), flipped flag, alerts before / after. Derived ratios before / after: TG/HDL vs `Cln_TGHDL_ok`, non-HDL vs `Cln_FH_NonHDL`, TC/HDL, LDL/HDL, BMI from the latest logged weight vs `Bod_BMI_over`; flip flags. |
| GLB-06 | YearReview | BN | BN + PH (spend by year: GATE-DATES) | `tblPanels`, `tblLabs`, `tblDaily`, `tblWearable`, `tblSupp` | One row per calendar year of data: lab panels, days logged, median weight and delta, median and delta of ApoB, LDL-C, HDL-C, TG, glucose, HbA1c, hs-CRP (`MEDIAN(IF(YEAR(...)))` array formulas), median sleep, supplement spend prorated by start / stop dates (0 until dated), narrative sentence (`&`, `TEXT`); weight-by-year chart. |
| GLB-07 | Export | BN | BN | demographics names, `Calc_Marker`, `tblMarkers`, `K_*`, `tblSupp`, goal `Params` | Flat table section / item / value / unit / as-of / note: demographics (age, DOB, sex, height), latest value and status of every marker, slope per year, gaps (never measured / stale vs `Lab_Stale_days`), genome flags (`K_GEN_*`), the stack (taken items with dose and timing), goals (`Bod_Goal_lb`, `Bod_Goal_date`, `Cln_ApoB_goal_hi`, `Age_Z2_target_min`, `Age_Lift_sessions`, `Bod_Sleep_min_h`, `Age_Target_age`), every other KPI (formatted with its number format). Blue `redact` column (DOB defaults to YES) -> `[redacted]`; a copy-ready `line` per row and a `bundle` cell (`TEXTJOIN(CHAR(10))`, capped at 32,000 characters); privacy banner. |
| GLB-08 | Nav, Sources | BN | BN | module registry, `docs/ideas/coverage-*.md` (embedded at build time), `tblResources`, `tblSizes`, `Params` | Nav (first tab): global disclaimer; every sheet of the build grouped by workstream (W8, W1-W7, legacy views, shared data layer) as an internal hyperlink (hidden sheets listed, not linked) with its BN / PH / NS badge and idea ids parsed from the coverage tables (any dialect: `shipped` / `built as` / `status` columns, BUILDABLE-NOW / BUILT / PLACEHOLDER / NEEDS-SOURCE); per-workstream counts (`COUNTIFS`) with a link to each first sheet; defines `Nav_Status`. Sources: provenance of every `Data_*` table (origin, rows, first / last date, awaiting data), parameters by source state, catalog source keys, open source checks (MASTER section 4), resources (hyperlinks) and sizes reference tables from the data layer. |

Totals by id: BN 8 (GLB-01..08); GLB-04 and GLB-06 carry a PH part behind GATE-DATES (`tblSupp`
start / stop dates are empty). MASTER planned BN 8 / PH 0 / NS 0.

## KPIs and names

`K_GLB_alerts`, `K_GLB_watch`, `K_GLB_critical` (Alerts), `K_GLB_new_alerts`, `K_GLB_top_mover`
(Changes). Defined name `Nav_Status` (Nav badge column, read by Checks).

## Parameters

All heuristic (`Glb_*`, registered by Nav, which builds ahead of `Params`): `Glb_Amber_pct`,
`Glb_HI_w_ref`, `Glb_Index_green`, `Glb_Index_amber`, `Glb_Alerts_amber`, `Glb_Streak_green`,
`Glb_Weight_amber`, `Glb_Weight_tol`, `Glb_Top_n`. Reused: `Cln_ApoB_goal_lo/hi`, `Cln_TGHDL_best/ok`,
`Cln_FH_NonHDL`, `Age_VO2_marchand`, `Age_VO2_floor_m/f`, `Age_Z2_target_min`, `Bod_Sleep_min_h`,
`Bod_Goal_lb`, `Bod_lb_per_kg`, `Bod_BMI_over`, `RCV_z`, `Lab_Stale_days`.

## Build notes

* Phases: Nav is phase 1 (always built, registers `Glb_*`); Alerts and Changes publish KPIs in
  phase 0; Dash, Export, Checks and Dash_Trend build in phase 2, after `Ref_KPI` defined every
  KPI name. `--only` builds bring Nav along (and Dash brings Dash_Trend); GLB sheets test
  `Shared::has` before reading another sheet, so narrower builds never reference a missing sheet.
* Checks needs each sheet's used range; rust_xlsxwriter keeps dimensions private, so the build
  serialises once, reads every sheet's `<dimension>` back and then fills the sweep formulas
  (`SheetModule::finishes` / `finish`).
* Excel 365 spills use `TAKE` / `SORTBY` (no `FILTER`); every list also has a classic LARGE /
  INDEX / MATCH version that works in any Excel and in LibreOffice.
