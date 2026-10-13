# Adding a sheet or view to the workbook

`health sheet` builds the workbook from a registry of modules in
`src/workbook/sheets/`: one module per sheet or view, nothing shared between
modules except the data and the formats in [`Cx`](../src/workbook/mod.rs).

1. Create `src/workbook/sheets/<name>.rs`:

   ```rust
   pub const TITLE: &str = "My View";          // tab name, <= 31 chars
   pub struct MyView;
   impl SheetModule for MyView {
       fn id(&self) -> &'static str { "my-view" }   // `health sheet --only my-view`
       fn title(&self) -> &'static str { TITLE }
       fn available(&self, d: &Data) -> bool { true } // e.g. d.labs.is_some()
       fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> { ... }
   }
   ```

2. Declare `pub mod <name>;` and add `Box::new(<name>::MyView)` to
   `registry()` in `src/workbook/sheets/mod.rs` (the registry order is the tab order).
   A unit test there checks ids and titles are unique and valid for Excel.
   A workstream with several sheets keeps them in `sheets/<prefix>/` with its own
   `mod.rs` exposing `modules()`, and adds one `reg.extend(<prefix>::modules());` line
   (see `sheets/lab/`).

3. Rules of the house:
   * **Excel-native formulas only.** No `FILTER`, `QUERY`, `ARRAYFORMULA`,
     `IMPORTRANGE` or `__xludf.DUMMYFUNCTION` (a test scans every formula).
     `LOOKUP(2,1/cond,range)`, `SUMIFS`, `COUNTIFS`, `INDEX/MATCH`,
     `SUMPRODUCT` cover what Google's functions were used for.
   * **Inputs are values, derived numbers are formulas** (`$/yr`, ages, BMI,
     totals). The store never holds a derived value.
   * **Real tables** via `add_table` (unique `tblName`), data validation for
     closed vocabularies (lists live on the hidden `Lists` sheet), conditional
     formats for thresholds, charts for trends.
   * **Cross-sheet references go through the owner's `pub fn`/`const`**
     (`supplements::cost_range(d)`, `weight_age::weight_range(d)`), never a
     hard-coded `Sheet!C7`, so layouts can move.
   * New data comes from the store (`Data`, loaded in `workbook::Data::load`)
     or from a sibling tool's JSON (`workbook::labs`); extend those, not the module.
   * Add an assertion to `tests/sheet.rs` (values, formulas, XML parts).

Views from the ideas catalog (`docs/ideas/`) are implemented this way.

## Shared infrastructure (`src/infra/`)

Modules reach it through `cx.shared` (registries) and `crate::infra::theme` (helpers).
Nothing here is edited by a view's author.

* **Registry and phases.** `SheetModule::phase()` 0 builds first; phase 1 (`Params`,
  `Ref_KPI`) builds last so it lists everything the others registered. Tab order is
  registry order. Duplicate sheet titles (case-insensitive) fail the build.
* **Named ranges.** `cx.shared.names.borrow_mut().define("Sel_X", &names::cell_ref("Lab_Tear", 3, 1), "lab")?`
  Names must be legal and must not look like cells (`K1`); a collision fails the build and
  names both owners. Standard names: `Today`, `DOB`, `Height_in`, `Height_cm`, every
  parameter, every KPI.
* **Parameters.** `cx.shared.params.borrow_mut().add(Param::num("Lab_Alpha", 0.05, "", State::Heuristic, "heuristic").range(0.0, 0.5))?`
  The `Params` sheet (`name, value, unit, state, source, note, min, max`) gets one row per
  parameter, a defined name at its value cell, validation for min/max and a state colour.
  States: `verified` (cite the source), `heuristic`, `UNVERIFIED` (source must read
  `pending: <named reference>`; the build rejects an uncited one), `input`. Use the name in
  formulas (`=Half_Life_days`), never the address. Seeds are in `infra/params.rs`.
* **KPIs.** `cx.shared.kpis.borrow_mut().publish("LAB", "n_out", "Markers out of range", Some("Lab_Grid!$B$2"), None)?`
  defines `K_LAB_n_out` at a `Ref_KPI` cell (max 10 per workstream; `n/a` until a formula is
  given). The global dashboard reads only `K_*` names; wrap in `IFERROR(..., "n/a")`.
* **Theme** (`infra/theme.rs`): palette and status colours (optimal / ref-ok / out / critical,
  UNVERIFIED orange, PLACEHOLDER grey), number formats `theme::nf`, tab colour by sheet prefix
  (`Lab_ Cln_ Sup_ Age_ Bod_ Gen_ Sch_ Data_ Calc_ Ref_ Params`), banners
  (`disclaimer`, `dx_risk`, `awaiting`, `unverified`), `note_source`, conditional formats
  (`cf_three_color`, `cf_two_color`, `cf_data_bar`, `cf_icons`, `cf_formula`, `cf_status_quilt`),
  `sparkline`, `hidden_base_stack` (waterfall / Gantt), `dyn_formula` for Excel 365
  dynamic arrays. `write_formula` adds `_xlfn.` prefixes to XLOOKUP/LET/etc. itself.
* **Tests.** `workbook::build_with(&[...fake modules], ...)` builds a workbook from a custom
  module list (see `workbook::tests`). `tests/recalc.rs` recalculates the generated workbook in
  headless LibreOffice and fails on any formula error; run it after adding formulas. The one exception is
  `#N/A` from an explicit `NA()` (a chart-only column whose gaps must not plot as 0; see `bod::plot_col`).

## The shared data layer: read it, do not rebuild it

Every build writes three layers (code: `src/infra/data.rs`, `refs.rs`, `calc.rs`). Views read
them by table / column / defined name; they never re-fetch labs or rebuild lab statistics.

**`Data_*` sheets: one Excel Table each, long format, written by Rust.** Table (sheet) -> columns:

| table | rows | columns |
|---|---|---|
| `tblLabs` (`Data_Labs`) | every measurement, by date then marker | `panel_date, panel_id, marker_id, value_raw, unit_raw, value, lab, method, fasting` |
| `tblMarkers` | every catalog marker, measured-but-uncatalogued ones, plus `uacr` | `marker_id, name, system, unit, ref_lo, ref_hi, opt_lo, opt_hi, blueprint_lo, blueprint_hi, dir, critical_lo, critical_hi, weight, cvi, cva` |
| `tblPanels` | one per lab-draw date | `panel_id, date, age_at_draw, lab, fasting, weight_at_panel, n_markers` |
| `tblDaily` | one per date with weight or intake | `date, weight, kcal, protein_g, energy_out_kcal, other` |
| `tblWearable` | empty (placeholder) | `date, hrv, rhr, recovery, strain, sleep_hours, sleep_efficiency, sleep_deep_h, sleep_rem_h` |
| `tblSupp`, `tblProducts`, `tblNutrition`, `tblRoutine`, `tblWorkout`, `tblVax`, `tblPhys`, `tblProviders`, `tblSizes`, `tblResources`, `tblEvidence` | the store's tables | see `TableDef.headers` in `data.rs` |
| `tblGenome` | empty (placeholder) | `rsid, gene, k1_genotype, k2_genotype, concordant` |
| `tblProfile` | one row of formulas | `DOB, sex, height_cm, ancestry` |
| `tblSuppTags` | one blank row per supplement, user-owned | `supp_id, name, h_* (9 hallmarks), s_* (7 SENS), m_* (10 mechanism loadings 0-3), indirectness, rung, evidence_grade` |
| `tblRegimes`, `tblSuppIngredients` | empty (placeholders) | see `data.rs` |

Facts to rely on:
* `tblLabs.panel_id` is `P<yyyymmdd>` and blank for body / vitals measurements (weight, BMI, biological age);
  `tblPanels` lists lab draws only.
* **Units:** `tblLabs.value` and the `tblMarkers` bounds are canonical units (weight in kg). `tblDaily.weight` and
  `tblPanels.weight_at_panel` are **pounds**, as in the legacy `Weight & Age`. `Ref_Units` has the conversion factor
  per marker and raw unit.
* Booleans are the strings `YES` / `NO` (blank = unknown). Dates are real Excel dates.
* `tblMarkers.dir`: `hi` (an upper bound matters), `lo`, `band`; blank when the marker has no range.
  `critical_*`, `blueprint_*`, `weight`, `cvi`, `cva` are blank until someone fills them from a source.
* Lab tables and `Calc_*` exist only when `biomarker` answered; without it they are absent, so guard views with
  `SheetModule::available` (`d.labs.is_some()`).
* `tblSupp.start_date` / `stop_date` exist but are empty (the store does not track them yet): GATE-DATES views degrade to
  placeholders with `theme::awaiting`.

**Reading a table.** In Rust, get ranges from the table definition so a moved column cannot break you:

```rust
use crate::infra::data::{LABS, MARKERS};
let n = LABS.nrows(cx.d);                          // data rows (at least 1)
let marker = LABS.range_n(n, "marker_id");         // Data_Labs!$C$2:$C$749
let value  = LABS.range_n(n, "value");
ws.write_formula(r, c, Formula::new(format!("=AVERAGEIFS({value},{marker},$A2)")))?;
```
Structured references (`=SUM(tblLabs[value])`) also work in Excel and in the LibreOffice recalculation test.

**`Calc_*` sheets: formulas only, read these instead of recomputing.**
* `Calc_Labs` (row-aligned with `tblLabs`): `panel_id, marker_id, value, mrow, ref_lo, ref_hi, opt_lo, opt_hi, in_ref, in_opt, status`.
* `Calc_Marker` (row-aligned with `tblMarkers`): `marker_id, n, first_date, last_date, last, previous, mean, sd, median, min, max,
  slope_per_year, dir_sign, status, z_latest, distance, pct_in_opt, pct_in_ref, days_since_last`.
* `Calc_Panel` (row-aligned with `tblPanels`): `panel_id, date, age, n_measured, pct_in_ref, pct_in_opt`.
* Status code: 0 optimal, 1 reference-ok but not optimal, 2 out of reference, 3 beyond critical. A marker with no optimal range
  is optimal when inside its reference. Calc assumes at most one value per marker and day.
* Names: `N_Panels`, `Latest_Panel`, `Prev_Panel`, plus KPIs `K_DATA_n_panels`, `K_DATA_latest_panel`, `K_DATA_n_measurements`.
  Row-aligned means: to read marker `m`, `INDEX(Calc_Marker!$E$2:$E$n, MATCH(m, Calc_Marker!$A$2:$A$n, 0))`.

**`Ref_*` lookups (tables, `tblRef*`):** `Ref_Systems` (marker -> organ system), `Ref_Units`, `Ref_Stance` (supporting 1 / neutral 0 / warning -1,
heuristic), `Ref_TimeSlots`, `Ref_Hallmarks` (9), `Ref_SENS` (7), `Ref_Mechanisms` (10), `Ref_Direction`. The hallmark and SENS lists are
**UNVERIFIED** (typed from memory; the source column says what to check them against). Rule tables you own (vaccine, screening,
interaction, GRADE, UL/DRI, PGx allele) must use the columns in `refs::RULE_HEADERS`: `id, rule, value, unit, source, source_state, note`.

**Adding a table or column.** Add a `TableDef` (and a row builder) in `data.rs` and list it in `data::all()`; do not create a second
copy of an entity in your own module. Table names must be unique across the workbook (the display sheets use `tblView*`).
