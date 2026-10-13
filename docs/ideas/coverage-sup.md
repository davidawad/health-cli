# Coverage: workstream W3 (SUP) - supplement portfolio, cost, interactions, evidence

Every SUP idea of [MASTER.md](MASTER.md) section 3 and the sheet that implements it.
Code: `src/workbook/sheets/sup/` (one registry line in `sheets/mod.rs`). Tests: `tests/sheet_sup.rs`.

Status as built: **BN** full build over present data; **PH** placeholder (full view over an empty
input, `PLACEHOLDER - awaiting data` banner); **NS** needs source (view ships, constants
UNVERIFIED with `pending: <reference>`); **BN+PH** part works now, part waits on data.

| id | MASTER status | built as | sheet(s) | what is built | waiting on |
| --- | --- | --- | --- | --- | --- |
| SUP-01 | BN, GATE-FIELDS | BN | `Sup_Book` | position book row-aligned with `tblSupp`: dose, doses/wk, daily dose, price, $/yr/mo/day, protocols, n refs, net stance (Ref_Stance), status, weight % (data bars), weeks on | start/stop dates (columns present, empty) |
| SUP-02 | BN | BN | `Sup_Cost` A-C, E | $/yr by protocol (equal split), by stance, protocol x stance matrix, waterfall (hidden base), Pareto + cumulative line, HHI, Gini, top-10 share, "spend with no evidence" call-out | - |
| SUP-03 | BN, GATE-FIELDS | BN | `Sup_Cost` D | $/yr, $/month, $/day, products in use, total, TER vs `Sup_Budget_yr`, by time of day, by kind | `Sup_Budget_yr` (input) |
| SUP-04 | BN, GATE-FIELDS | BN+PH | `Sup_Unit` | $/dose, amount/yr, $/mg (mg/g/mcg); cheapest source per ingredient (MINIFS) and saving if switched | `tblSuppIngredients` amounts |
| SUP-05 | BN, GATE-DATES | PH | `Sup_Churn` | weekly cost, unused stock at a stop (`Sup_Stock_wk`), washout (`Sup_Washout_wk`), running cumulative waste, scatter | `tblSupp.start_date` / `stop_date` |
| SUP-06 | PH | PH | `Sup_Mech` | loadings matrix from `tblSuppTags.m_*` x active, exposure, items loading, covered, cost per mechanism, heat map, radar | `tblSuppTags` loadings |
| SUP-07 | PH | PH | `Sup_DropOne` | saving, mechanisms lost (`Sup_Cover_Min`), coverage left, redundant flag, total saving from redundant items | `tblSuppTags` loadings |
| SUP-08 | PH | PH | `Sup_Value` | stance factor, certainty, evidence weight = `Kelly_Fraction` x certainty/4 x stance factor, relevance, score per $100, rank, quadrant, efficient frontier with budget slider, ranked bar + frontier charts | tags + GRADE rows |
| SUP-09 | BN, GATE-FIELDS | BN+PH | `Sup_Overlap` | stacked-dose detector by ingredient (SUMIFS, n sources, flag); name overlap (listed twice / also a product) works now | `tblSuppIngredients` |
| SUP-10 | NS | NS | `Sup_DRI`, `Ref_UL` | UL ledger (supplements + food input) with % of UL and near/over flags (`Sup_UL_Warn`); WHO reference-strength table (`tblSupWHO`) and Rx / nutrient / supplement / WHO-essential tag checker with dose ratio | UL values UNVERIFIED pending NIH ODS / IOM; ingredient amounts |
| SUP-11 | BN | BN | `Sup_Audit` | four lights (indication, effectiveness, safety, adherence), worst light, DiPiro MTP tag + manual MTP drop-down, entry table supplement -> marker, "intervention without marker" and "marker without intervention" (Calc_Marker status >= 2) | supplement -> marker links (entry) |
| SUP-12 | BN | BN (rules UNVERIFIED) | `Sup_Interact`, `Ref_Interact` | supplement x drug-class matrix (severity colours), mechanism filter `Sup_Mech_Filter`, Rx class list (prefilled with Rx items), "on Rx list?" column flags, statin map, warfarin (VKA) panel | Rx classes typed (A5); MED-VERIFY of rows |
| SUP-13 | NS | NS | `Sup_LabDrug` | rule table `tblSupLabDrug` (niacin/statin/red yeast -> ALT/AST, amiodarone -> TSH, topiramate -> CO2/chloride, biotin -> TSH), in-stack check, latest value vs reference from Calc_Marker, x ULN, neutral DX-RISK wording | rules UNVERIFIED (MED-VERIFY); `Sup_ALT_ULN_x` UNVERIFIED |
| SUP-14 | NS | NS | `Sup_PK` | half-life input, inducer flag from Ref_Interact, steady state / washout (`Sup_SS_HalfLives`, KAT ch09), induction onset/offset (`Sup_Inducer_Onset_d`, KAT ch82:65), BID/QD timing (`Sup_BID_HL_h`), wait before draw, 0-28 day elimination grid, accumulation chart | supplement half-lives (pending: GG ch88 / EXT) |
| SUP-15 | BN, GATE-DATES | PH | `Sup_Cycle` | weeks on (DATEDIF), onset / studied windows (`tblSupOnset`: Bacopa 12 wk, SJW 2-4 wk, SJW > 12 wk unstudied), status | start/stop dates |
| SUP-16 | BN | BN | `Sup_Grade`, `Ref_GRADE` | GRADE entry per supplement x outcome: start 4/2, five downgrades, three upgrades, certainty, label, plus-stars, directness flag, SoF columns, certainty distribution + evidence ladder (`tblSuppTags.rung`) chart | entry |
| SUP-17 | BN | BN | `Sup_Papers` | one row per `tblEvidence` paper (+ spare): RoB 2 five domains + worst-domain overall, AMSTAR-2 16 items (7 critical) + rating, power from n and d (`Alpha`, `Sup_Power_Target`), trust checklist score of 6 | entry |
| SUP-18 | BN | BN | `Sup_Calc` | Ioannidis PPV (no bias, bias u) + PPV surface, NNT / NNH / RR / RRR, screening PPV / NPV / FP and FN per 1,000 | - |
| SUP-19 | BN | BN | `Sup_Balance` | supporting / warning / neutral counts per supplement, certainty-weighted, net, stacked bar | certainty grows with GRADE rows |
| SUP-20 | BN | BN | `Sup_Forest` | entry table (measure, effect, CI), null value by measure, custom x error bars scatter (forest plot), "absence of signal is not safety" reading | entry |
| SUP-21 | BN | BN | `Sup_Bubble` | certainty vs $/month, bubble size column (hallmark count else pills/day), "expensive and uncertain" quadrant; scatter chart (rust_xlsxwriter 0.99 has no bubble type) | - |
| SUP-22 | BN | BN | `Sup_TrialDose` | daily dose vs trial dose (`tblSupTrialDose`: SJW 900 mg, vitamin D 4,000 IU, ashwagandha 120 mg) + override column, ratio, "no trial at this dose" (`Sup_Trial_Tol`) | - |
| SUP-23 | BN (new) | BN+PH | `Sup_Regimen` | taking vs not per supplement (biomarker `regimen ls` status next to the store's, agree/DIFFERS, start/stop, days, dose), biomarker regimens with no store row, contraindication grid from `biomarker interaction grid -p <person> --catalog` (kind per cell, coloured) and edges outside the grid | biomarker-cli >= 0.7.1 (else store taking and a grid placeholder); start dates (hx-ft3.12) |

Rule tables owned here (shared schema `id, rule, value, unit, source, source_state, note`):
`Ref_Interact` (22 rows, UNVERIFIED, KAT ch80 / ch46 / DIP ch37 citations), `Ref_GRADE` (14 rows,
GRADE-3 / GRADE-1), `Ref_UL` (18 rows, UNVERIFIED pending NIH ODS / IOM), plus on-sheet
`tblSupWHO`, `tblSupLabDrug`, `tblSupOnset`, `tblSupTrialDose`.

KPIs (`K_SUP_*`, 10): `n_active`, `usd_yr`, `usd_month`, `usd_day`, `ter`, `hhi`,
`usd_no_evidence`, `n_interactions`, `n_audit_red`, `usd_low_certainty`.

Parameters (`Params`): `Sup_Budget_yr` (input), `Sup_Stock_wk`, `Sup_Washout_wk`, `Sup_Cover_Min`,
`Sup_Default_Certainty`, `Sup_UL_Warn`, `Sup_BID_HL_h`, `Sup_Power_Target`, `Sup_Trial_Tol`
(heuristic), `Sup_Inducer_Onset_d` (verified, KAT ch82.md:65), `Sup_SS_HalfLives` (verified,
KAT ch09.md:107-111), `Sup_ALT_ULN_x` (UNVERIFIED, pending KAT ch46.md:201). Reused seeds:
`Kelly_Fraction`, `Alpha`. Defined name: `Sup_Mech_Filter` (input cell on `Sup_Interact`).

Caveat: entry forms (GRADE, RoB 2 / AMSTAR-2, forest rows, half-lives, Rx classes, trial overrides,
supplement -> marker links) live in the generated workbook; regenerating it resets them until the
store grows tables for them.
