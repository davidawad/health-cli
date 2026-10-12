# Coverage: workstream LAB (`Lab_*` sheets, hx-rx3.12)

**The LAB-id column is open.** The catalog this workstream implements
(`MASTER.md`, section 3, 35 `LAB-*` ideas) was not in the build box: not in
the repo, the bead graph or the run inputs. So `docs/ideas/MASTER.md` is not
committed here, and no idea id is claimed. The table below lists what was
built, by capability. Once `MASTER.md` is in `docs/ideas/`, map each `LAB-*` id
to a row (or mark it missing) and update the status.

Status legend: **BUILT** (full view over the store and labs), **PLACEHOLDER**
(view with an empty input table and an awaiting-data banner), **NEEDS-SOURCE**
(inputs exist but stay blank and `UNVERIFIED` until a source is cited).

| LAB id | Capability | Sheet | Status |
|---|---|---|---|
| ? | Single-marker tear sheet: name, unit, reference interval, draws, first / last draw, latest, previous, change, % change | `Lab_Tear` | BUILT |
| ? | Descriptive stats for the selected marker: mean, SD, CV, min, max, z of latest, slope per year | `Lab_Tear` | BUILT |
| ? | Rolling mean and Bollinger band (`Boll_Window`, `Boll_K`), %B | `Lab_Tear` history | BUILT |
| ? | Winsorised z per draw (`Winsor_Cap`) | `Lab_Tear` history | BUILT |
| ? | Two-sided CUSUM shift detector (`CUSUM_k`, `CUSUM_h`), neutral "upward / downward shift" signal | `Lab_Tear` | BUILT |
| ? | EWMA over irregular draw dates (`Half_Life_days`) | `Lab_Tear` | BUILT |
| ? | Scalar Kalman filter estimate and variance (`Kalman_q`, `Kalman_r`) | `Lab_Tear` | BUILT |
| ? | Trend chart: value, EWMA, band, reference limits | `Lab_Tear` | BUILT |
| ? | Per-marker stats table (draws, dates, latest, mean, SD, CV, min, max, z, slope / yr, status) | `Lab_Stats` | BUILT |
| ? | Reference status (Low / High / OK) of the latest draw | `Lab_Stats` | BUILT |
| ? | Staleness: days since last draw vs `Lab_Stale_days` | `Lab_Stats` | BUILT |
| ? | Trend sparklines per marker | `Lab_Stats` | BUILT |
| ? | Last-two-draws change: delta, % change, days apart | `Lab_Change` | BUILT |
| ? | Delta check against `Delta_check_pct` (blank heuristic until set) | `Lab_Change` | BUILT (inactive until the parameter is set) |
| ? | Reference change value `RCV_z * sqrt(2) * sqrt(CVa^2 + CVi^2)` | `Lab_Change` | NEEDS-SOURCE (CVa / CVi: pending EFLM Biological Variation Database) |
| ? | Status quilt, marker x draw date (0 optimal / 1 ref / 2 out / 3 critical) | `Lab_Quilt` | BUILT (codes 0 and 3 need optimal / critical bounds) |
| ? | Optimal and critical bounds per marker | `Lab_Ranges` | NEEDS-SOURCE (blank, `UNVERIFIED`, `pending:` per row) |
| ? | N-of-1 experiments: baseline vs during, means, SDs, Welch t, df, p vs `Alpha` | `Lab_Exp` | PLACEHOLDER (GATE-DATES: no intervention start dates in the store) |
| - | Long lab table, `Lab_L_*` names (private adapter until hx-rx3.21's `tblLabs` lands) | `Lab_Long` (hidden) | BUILT |

## KPIs (`Ref_KPI`, 10 of 10 allowed)

`K_LAB_n_markers`, `K_LAB_n_draw_dates`, `K_LAB_last_draw`, `K_LAB_n_out`,
`K_LAB_pct_out`, `K_LAB_n_stale` (`Lab_Stats`); `K_LAB_n_beyond_rcv`,
`K_LAB_n_delta` (`Lab_Change`); `K_LAB_n_critical` (`Lab_Quilt`);
`K_LAB_n_exp_signal` (`Lab_Exp`).

## Parameters added (`Params`)

| Name | Value | State | Use |
|---|---|---|---|
| `Lab_Stale_days` | 365 | heuristic | stale flag on `Lab_Stats` |
| `Lab_Min_n_trend` | 3 | heuristic | fewest draws before a slope is shown |
| `Lab_Exp_Baseline_days` | 180 | heuristic | baseline window before an experiment's start |

The views also read seeded parameters: `Sel_Marker`, `Boll_Window`, `Boll_K`,
`Winsor_Cap`, `CUSUM_k`, `CUSUM_h`, `Half_Life_days`, `Kalman_q`, `Kalman_r`,
`RCV_z`, `Delta_check_pct`, `Alpha`. Per-marker clinical constants (optimal and
critical bounds, CVa, CVi) are rows on `Lab_Ranges`, not `Params`, because they
are one per marker. Each row is `UNVERIFIED` with a `pending:` source until
someone cites one.

## Not built

* The `Lab_Tear` chart plots the rows past the selected marker's last draw.
  Excel draws their `""` as zero. Avoiding that needs `NA()`, which the
  recalculation check rejects as a formula error.
* KAMA (`KAMA_fast`, `KAMA_slow` are seeded) has no view yet.
