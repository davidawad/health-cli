//! Workstream W4 (AGE): aging clocks, longevity frameworks and fitness.
//!
//! One module per `Age_*` sheet (AGE-01..18 in `docs/ideas/MASTER.md`; the
//! id -> sheet -> status table is `docs/ideas/coverage-age.md`). The views read
//! the shared data layer (`tblLabs`, `tblPanels`, `tblSupp`, `tblSuppTags`,
//! `tblEvidence`, `Calc_Marker`, `Ref_*`) and own only their entry tables
//! (`tblAge*`), which hold what David types: DEXA scans, VO2 tests, workout and
//! sauna logs. Every constant is a `Params` row (see [`params`]); headline
//! numbers are `K_AGE_*` KPIs.
//!
//! Layout of every sheet: title (row 1), ideas + sources (row 2), the
//! disclaimer (row 3), a status banner (row 4: placeholder / unverified /
//! gate), content from row 6 ([`TOP`]).

use rust_xlsxwriter::{DataValidation, Format, TableStyle, Worksheet};

use crate::error::Result;
use crate::infra::theme;
use crate::workbook::{add_table, col, Cx, SheetModule};

pub mod claims;
pub mod decade;
pub mod decathlon;
pub mod dexa;
pub mod epi;
pub mod hallmarks;
pub mod horsemen;
pub mod params;
pub mod pheno;
#[cfg(test)]
mod pheno_check;
pub mod protocols;
pub mod sauna;
pub mod sens;
pub mod strength;
pub mod vendor;
pub mod vo2;
pub mod volume;
pub mod whatif;
pub mod z2tools;
pub mod zone2;

/// KPI workstream key (`K_AGE_*`).
pub const WS: &str = "AGE";
/// First content row (zero-based) under the header block.
pub const TOP: u32 = 5;
/// Blank rows in a dated entry log.
pub const LOG_ROWS: u32 = 200;
/// Blank rows in a short entry table.
pub const ENTRY_ROWS: u32 = 30;

/// Every AGE sheet, in tab order (AGE-01..18).
pub fn modules() -> Vec<Box<dyn SheetModule>> {
    vec![
        Box::new(pheno::Pheno),
        Box::new(vendor::Vendor),
        Box::new(whatif::WhatIf),
        Box::new(claims::Claims),
        Box::new(epi::Epi),
        Box::new(hallmarks::Hallmarks),
        Box::new(sens::Sens),
        Box::new(protocols::Protocols),
        Box::new(horsemen::Horsemen),
        Box::new(dexa::Dexa),
        Box::new(decathlon::Decathlon),
        Box::new(vo2::Vo2),
        Box::new(zone2::Zone2),
        Box::new(z2tools::Z2Tools),
        Box::new(strength::Strength),
        Box::new(volume::Volume),
        Box::new(sauna::Sauna),
        Box::new(decade::Decade),
    ]
}

/// Banner kinds for row 4.
pub enum Banner<'a> {
    /// PLACEHOLDER: the view waits on this data.
    Awaiting(&'a str),
    /// A constant or table here is UNVERIFIED pending this reference.
    Unverified(&'a str),
    /// A plain note.
    Note(&'a str),
}

/// Title, ideas + sources line, disclaimer and banner; registers the AGE parameters.
pub fn head(ws: &mut Worksheet, cx: &Cx, title: &str, ideas: &str, banner: Banner) -> Result<()> {
    params::ensure(cx)?;
    let f = cx.f;
    ws.write_string_with_format(0, 0, title, &f.title)?;
    ws.write_string_with_format(1, 0, ideas, &f.note)?;
    theme::disclaimer(ws, 2, f)?;
    match banner {
        Banner::Awaiting(what) => theme::awaiting(ws, 3, what, f)?,
        Banner::Unverified(what) => theme::unverified(ws, 3, 0, what, f)?,
        Banner::Note(text) => ws.write_string_with_format(3, 0, text, &f.note).map(|_| ())?,
    }
    ws.set_column_width(0, 28)?;
    Ok(())
}

/// Section label at `row`.
pub fn section(ws: &mut Worksheet, cx: &Cx, row: u32, text: &str) -> Result<()> {
    ws.write_string_with_format(row, 0, text, &cx.f.section)?;
    Ok(())
}

/// A column of a view table: typed by David (`In`), or a formula of the 1-based Excel row.
pub enum Col<'a> {
    In(&'a str),
    /// A typed date (input fill, date format).
    Date(&'a str),
    F(&'a str, Box<dyn Fn(u32) -> String + 'a>),
}

impl<'a> Col<'a> {
    pub fn f(h: &'a str, f: impl Fn(u32) -> String + 'a) -> Self {
        Self::F(h, Box::new(f))
    }
    fn header(&self) -> &str {
        match self {
            Self::In(h) | Self::Date(h) | Self::F(h, _) => h,
        }
    }
}

/// An Excel table `name` with its header at zero-based `hr`, `rows` body rows: input
/// columns get the input fill, formula columns their formula on every row.
pub fn table(ws: &mut Worksheet, cx: &Cx, name: &str, hr: u32, c0: u16, cols: &[Col], rows: u32) -> Result<()> {
    let headers: Vec<&str> = cols.iter().map(Col::header).collect();
    let date = input_date();
    add_table(ws, name, hr, c0, &headers, rows as usize, TableStyle::Medium2)?;
    for i in 0..rows.max(1) {
        let r = hr + 1 + i;
        for (j, c) in cols.iter().enumerate() {
            let cc = c0 + j as u16;
            match c {
                Col::In(_) => ws.write_blank(r, cc, &cx.f.input).map(|_| ())?,
                Col::Date(_) => ws.write_blank(r, cc, &date).map(|_| ())?,
                Col::F(_, f) => ws.write_formula(r, cc, f(r + 1).as_str()).map(|_| ())?,
            }
        }
    }
    Ok(())
}

/// Input fill with a date format.
pub fn input_date() -> Format {
    Format::new().set_background_color(theme::INPUT).set_num_format(theme::nf::DATE)
}

/// Absolute range of zero-based column `c` over zero-based rows `r0..=r1` of `title`.
pub fn rng(title: &str, c: u16, r0: u32, r1: u32) -> String {
    crate::workbook::range(title, c, r0 + 1, r1 + 1)
}

/// Absolute single cell of `title` (zero-based row and column).
pub fn cell(title: &str, r: u32, c: u16) -> String {
    crate::infra::names::cell_ref(title, r, c)
}

/// Relative A1 reference to zero-based column `c` on 1-based Excel row `r`.
pub fn at(c: u16, r: u32) -> String {
    format!("{}{r}", col(c))
}

/// Absolute A1 reference on this sheet.
pub fn abs(c: u16, r0: u32) -> String {
    format!("${}${}", col(c), r0 + 1)
}

/// Drop-down of `items` on a zero-based block.
pub fn list(ws: &mut Worksheet, r0: u32, c: u16, r1: u32, items: &[&str]) -> Result<()> {
    ws.add_data_validation(r0, c, r1, c, &DataValidation::new().allow_list_strings(items)?)?;
    Ok(())
}

/// Write `label` and a formula in the next column, with a number format.
pub fn kv(ws: &mut Worksheet, cx: &Cx, r: u32, c: u16, label: &str, formula: &str, nf: Option<&str>) -> Result<()> {
    ws.write_string_with_format(r, c, label, &cx.f.bold)?;
    match nf {
        Some(n) => ws.write_formula_with_format(r, c + 1, formula, &Format::new().set_num_format(n))?,
        None => ws.write_formula(r, c + 1, formula)?,
    };
    Ok(())
}

/// Publish an AGE KPI pointing at a cell of `title`.
pub fn kpi(cx: &Cx, name: &str, label: &str, title: &str, r: u32, c: u16, nf: Option<&str>) -> Result<()> {
    let f = format!("IFERROR(IF({0}=\"\",\"n/a\",{0}),\"n/a\")", cell(title, r, c));
    cx.shared.kpis.borrow_mut().publish(WS, name, label, Some(&f), nf)?;
    Ok(())
}

/// Monday of the week `k` weeks before the current one.
pub fn week_start(k: u32) -> String {
    format!("(Today-WEEKDAY(Today,2)+1-7*{k})")
}

/// A decline rate typed as percent (10) or fraction (0.10), as a fraction; blank stays blank.
pub fn as_fraction(name: &str) -> String {
    format!("IF(ISNUMBER({name}),IF({name}>1,{name}/100,{name}),\"\")")
}

/// The last numeric value of a range ("" when none).
pub fn last_num(range: &str) -> String {
    format!("IFERROR(LOOKUP(2,1/ISNUMBER({range}),{range}),\"\")")
}

/// A `Calc_Marker` column range (row-aligned with `tblMarkers`).
pub fn calc_marker(d: &crate::workbook::Data, header: &str) -> String {
    use crate::infra::calc::{MARKER_HEADERS, MARKER_TITLE};
    let c = MARKER_HEADERS.iter().position(|h| *h == header).expect("Calc_Marker column") as u16;
    crate::workbook::range(MARKER_TITLE, c, 2, 1 + crate::infra::data::MARKERS.nrows(d))
}

/// `Calc_Marker.<header>` of the marker whose slug is in `slug_ref` ("" when absent).
pub fn marker_stat(d: &crate::workbook::Data, header: &str, slug_ref: &str) -> String {
    format!("IFERROR(INDEX({},MATCH({slug_ref},{},0)),\"\")", calc_marker(d, header), calc_marker(d, "marker_id"))
}

/// Age today in years from `DOB`.
pub const AGE_NOW: &str = "IF(ISNUMBER(DOB),(Today-DOB)/365.25,\"\")";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_age_sheet_is_prefixed_and_has_an_id() {
        let m = modules();
        assert_eq!(m.len(), 18, "AGE-01..18");
        for x in &m {
            assert!(x.title().starts_with("Age_"), "{}", x.title());
            assert!(x.id().starts_with("age-"), "{}", x.id());
        }
    }

    #[test]
    fn helpers_write_absolute_and_relative_refs() {
        assert_eq!(rng("Age_X", 1, 5, 9), "Age_X!$B$6:$B$10");
        assert_eq!(cell("Age_X", 0, 0), "Age_X!$A$1");
        assert_eq!(at(2, 7), "C7");
        assert_eq!(abs(3, 4), "$D$5");
        assert!(as_fraction("D").contains("D/100"));
    }
}
