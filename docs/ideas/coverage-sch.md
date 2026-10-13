# Coverage: W7 SCH (schedules, routine, vaccines, prevention, visit pack)

Every SCH id of [MASTER.md](MASTER.md) section 3, the sheet that implements it and its
status in the generated workbook. Code: `src/workbook/sheets/sch/` (one module per sheet,
constants in `sch/params.rs`, rule tables in `sch/rules.rs`); tests: `tests/sheet_sch.rs`
(formula needles per idea, tables, names, charts, a LibreOffice recalculation after typing
inputs into the blue cells) plus unit tests in the modules.

Status as shipped: **BN** computes now with formulas over the data layer; **PH** full view
over an empty input with the `PLACEHOLDER - awaiting data` banner; **NS** view computes, a
constant or rule it relies on is `UNVERIFIED` with `pending: <reference>`.

| id | sheet | master status | shipped | what it reads | notes |
| --- | --- | --- | --- | --- | --- |
| SCH-01 | Sch_Vax, Ref_Vax | NS | NS | tblVax, Ref_Vax | Per vaccine: rule auto-matched by keyword (override in blue), series doses / gap / booster / live from `Ref_Vax`, dose count, first/last dose (`MINIFS`/`MAXIFS`), next due `EDATE(last, gap or booster)`, status current / due soon / overdue / series complete. 12-month ladder grid + chart. Dose ledger with live-vaccine spacing; planned doses checked against `Sch_Live_Gap_d` and an immune-globulin table (`Sch_Ig_Gap_months`, DIP ch148.md:87). All intervals UNVERIFIED pending the CDC adult schedule. KPIs `K_SCH_vax_overdue`, `K_SCH_vax_next_due`. |
| SCH-02 | Sch_Screen, Ref_Screen | NS | NS | DOB, Sex, tblLabs, tblPhys, Ref_Screen | 13 items; sex, start/stop age, interval (0 = once), framing, risk-only from `Ref_Screen`. Applies by `DATEDIF(DOB,Today)` and `Sex`; last done = later of a typed date and the item marker's last draw (`MAXIFS` on tblLabs); listed / done in tblPhys; next due `EDATE`; age-window chart. Attia practice cited (OUT ch22.md:31,37, ch14.md:163, ch28.md:29, ch35.md:19-25) and labelled "Attia practice - not guideline"; mainstream ages UNVERIFIED pending USPSTF. [DX-RISK] neutral line. KPI `K_SCH_screen_due`. |
| SCH-03 | Sch_Provider | BN | BN | tblPhys, tblProviders | Kind guessed from text (override), last visit typed, interval from `Sch_Int_<kind>_m` (or typed), next due, status; 12-month calendar with stacked chart. KPI `K_SCH_provider_overdue`. |
| SCH-04 | Sch_Monitor | BN (GATE-DATES) | PH | tblSupp start/stop, tblLabs | GATE-DATES failed (`tblSupp.start_date` / `stop_date` empty): supplement rows read "awaiting dates". Typed change rows compute now: window = change + weeks from the cited windows (lipids 4-12 wk DIP ch37.md:451, A1c DIP ch97.md:197, thyroid DIP ch98.md:345, verified Params), first draw in window by `MINIFS`, status done / missed / due now / upcoming. |
| SCH-05 | Sch_Retest | BN | BN | Calc_Marker, tblMarkers | CV, minimal meaningful change (`Sch_MMC_pct` until EFLM CVi/CVa), draws needed `((RCV_z*SD)/delta)^2`, personal-SD RCV proxy, inverse-volatility cadence (out-of-range shorter), next due, value of information `weight x (1+status) x staleness x (1+CV) / cost`, top-10 ranking + chart. KPI `K_SCH_retest_overdue`. |
| SCH-06 | Sch_PreDraw | BN (GATE-FIELDS) | BN | tblSupp (amount, unit), tblSuppIngredients, tblPanels, tblLabs | GATE-FIELDS passed (amount/unit exist). Biotin mcg per item (name or ingredient, mg -> mcg) vs `Sch_Biotin_mcg` (DIP ch98.md:233); planned draws: stop-by = draw - `Sch_Biotin_hold_h`, countdown, fasting hours vs `Sch_Fast_h` (KAT ch46.md:77); past panels: immunoassays on the panel and a biotin-in-window flag (dose dates typed; none in the store); prep checklist. |
| SCH-07 | Sch_Dosing | BN | BN | tblSupp | Editable slot map (time_of_day -> morning/noon/evening/bedtime), grid of doses for items taken, absorption rules by keyword (GREG ch35, HELMS ch02, longevity.md E11) -> food column and timing flag, compact INDEX/MATCH print list (print area, landscape), items-per-slot chart. Row-aligned grid read by SCH-08 and SCH-11. |
| SCH-08 | Sch_PillLoad | BN (GATE-DATES) | PH | tblSupp, Sch_Dosing | Burden computes now: doses/week from an editable frequency map, slots from Sch_Dosing, events and pills per day, complexity score (heuristic weights after KAT ch81.md:91), slot x kind crowding heatmap. Turnover per quarter (`COUNTIFS` on start/stop) waits on GATE-DATES, hence PH. KPIs `K_SCH_pills_day`, `K_SCH_complexity`. |
| SCH-09 | Sch_Gantt | BN (GATE-DATES) | PH | tblSupp start/stop, tblPanels | Offset and duration per item by kind (Rx / supplement / other) plus a bar per lab panel; invisible-offset stacked bar chart. Bars wait on start/stop dates (GATE-DATES). |
| SCH-10 | Sch_Refill | BN (GATE-FIELDS) | BN | tblSupp, tblProducts | GATE-FIELDS passed (price, units_per_year). Days per unit, run-out from a typed last purchase, reorder-by (`Sch_Refill_lead_d`), status at `Sch_Refill_warn_d`; 12-month cash forecast counting purchases per month (or the annual spend spread evenly until a purchase date is typed) + chart. KPI `K_SCH_refill_due`. |
| SCH-11 | Sch_Chelate | NS | NS | Sch_Dosing | 13 separation rules in the shared rule schema (`rule` = agent A, `unit` = agent B, `value` = hours, all UNVERIFIED pending Stockley's / labelling; mechanism KAT ch82, WHO ch00.md:1144). Slots of each agent by `COUNTIFS` on the grid, smallest clock gap over the 16 slot pairs (slot hours are Params), CONFLICT flag, chart. KPI `K_SCH_chelate_conflicts`. |
| SCH-12 | Sch_Morning | BN | BN | tblRoutine, tblSupp, Caffeine_HL | Step start = wake + blocking steps before it (`SUMPRODUCT`), night plan scheduled back from `Sch_Bed_h`, typed actual start/minutes -> slippage and overrun; day overlays: sunlight (WWS tip 11), supplement doses, caffeine cut-off (`Caffeine_HL x Sch_Caffeine_HLs`), last exercise (WWS tip 2), sleep; two Gantt charts. |
| SCH-13 | Sch_Visit, Sch_POMR, Sch_MedRec | BN | BN | Calc_Marker, tblMarkers, tblDaily, tblSupp | Visit: SOAP one-pager (S typed; O counts by status, latest panel and weight; flagged markers ranked by status then abs(z); A one neutral line per flag; P questions generated from flags, never-measured markers and the medication list; print area). POMR: typed problems with marker last value / date / trend, effectiveness, safety, adherence (DIP ch07.md:105,111), next monitoring; unlinked flagged markers suggested. MedRec: store vs reported, discrepancy and review age. KPI `K_SCH_medrec_open`. |
| SCH-14 | Sch_Low | PH | PH | tblSupp, DOB | Travel trips with days to trip, checklist due-by (`Sch_Travel_lead_d`) and readiness; stopping review per item taken from `Sch_Beers_age` (UNVERIFIED pending AGS Beers). Titles only in the library (DIP e17, ch29.md:21,39). |

Totals by id: BN 7 (SCH-03, 05, 06, 07, 10, 12, 13), PH 4 (SCH-04, 08, 09 by the date gate; SCH-14),
NS 3 (SCH-01, 02, 11) = 14. MASTER planned BN 10 / PH 1 / NS 3; the three GATE-DATES ideas degraded to PH
because `tblSupp.start_date` / `stop_date` are empty. Both GATE-FIELDS ideas (SCH-06, SCH-10) passed.

## Parameters

All SCH constants are `Params` rows registered by `sch/params.rs` (prefix `Sch_`), plus the infra seeds
`Caffeine_HL` and `RCV_z`. UNVERIFIED (pending): `Sch_Live_Gap_d`, `Sch_Ig_Gap_months` (CDC General Best
Practice Guidelines), `Sch_Beers_age` (AGS Beers Criteria). Verified with file + line: `Sch_Mon_*` (DIP ch37/97/98),
`Sch_Biotin_mcg`, `Sch_Biotin_hold_h` (DIP ch98.md:233), `Sch_Fast_h` (KAT ch46.md:77), `Sch_Sun_min`,
`Sch_Exercise_gap_h` (WWS ch21 via longevity.md E10). Everything else is `heuristic`. Vaccine intervals and
screening ages are rule rows in `Ref_Vax` / `Ref_Screen` with their own `source_state`.

## KPIs (10 of 10)

`K_SCH_vax_overdue`, `K_SCH_vax_next_due`, `K_SCH_screen_due`, `K_SCH_provider_overdue`, `K_SCH_retest_overdue`,
`K_SCH_pills_day`, `K_SCH_complexity`, `K_SCH_refill_due`, `K_SCH_chelate_conflicts`, `K_SCH_medrec_open`.

## Sheets that need labs

`Sch_Monitor`, `Sch_Retest`, `Sch_PreDraw`, `Sch_Visit`, `Sch_POMR` are built only when `biomarker` answered
(`available`); `Sch_Screen` and `Sch_Gantt` drop their lab columns / panel rows without it.
