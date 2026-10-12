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
