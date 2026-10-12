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
  headless LibreOffice and fails on any formula error; run it after adding formulas.

Not built here (planned data/calc layers): `Data_*`/`Calc_*` sheets, `tblLabs`-style long tables.
