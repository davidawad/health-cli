//! Workstream W7 (SCH): schedules, routine, vaccines, prevention and the visit pack.
//!
//! One module per `Sch_*` sheet (SCH-01..14 in `docs/ideas/MASTER.md`; the
//! id -> sheet -> status table is `docs/ideas/coverage-sch.md`) plus the two rule
//! tables this workstream owns, `Ref_Vax` and `Ref_Screen` ([`rules`]). The views read
//! the shared data layer (`tblVax`, `tblPhys`, `tblProviders`, `tblSupp`, `tblProducts`,
//! `tblRoutine`, `tblLabs`, `tblPanels`, `tblMarkers`, `Calc_Marker`) and own only their
//! entry cells (blue): visit dates, planned draws, last purchases. Every constant is a
//! `Params` row ([`params`]); headline numbers are `K_SCH_*` KPIs.
//!
//! Layout of every sheet: title (row 1), ideas + sources (row 2), the disclaimer
//! (row 3), a status banner (row 4), content from row 6 ([`TOP`]).

use rust_xlsxwriter::{DataValidation, Format, TableStyle, Worksheet};

use crate::error::Result;
use crate::infra::data::TableDef;
use crate::infra::theme;
use crate::workbook::{add_table, col, Cx, SheetModule};

pub mod chelate;
pub mod dosing;
pub mod gantt;
pub mod low;
pub mod medrec;
pub mod monitor;
pub mod morning;
pub mod params;
pub mod pillload;
pub mod pomr;
pub mod predraw;
pub mod provider;
pub mod refill;
pub mod retest;
pub mod rules;
pub mod screen;
pub mod vax;
pub mod visit;

/// KPI workstream key (`K_SCH_*`).
pub const WS: &str = "SCH";
/// First content row (zero-based) under the header block.
pub const TOP: u32 = 5;
/// Months in the forward calendars (vaccine ladder, provider calendar, cash forecast).
pub const MONTHS: u16 = 12;

/// Every SCH sheet in tab order (SCH-01..14), then the rule tables.
pub fn modules() -> Vec<Box<dyn SheetModule>> {
    vec![
        Box::new(vax::Vax),
        Box::new(screen::Screen),
        Box::new(provider::Provider),
        Box::new(monitor::Monitor),
        Box::new(retest::Retest),
        Box::new(predraw::PreDraw),
        Box::new(dosing::Dosing),
        Box::new(pillload::PillLoad),
        Box::new(gantt::Gantt),
        Box::new(refill::Refill),
        Box::new(chelate::Chelate),
        Box::new(morning::Morning),
        Box::new(visit::Visit),
        Box::new(pomr::Pomr),
        Box::new(medrec::MedRec),
        Box::new(low::Low),
        Box::new(rules::RuleSheet(&rules::REF_VAX)),
        Box::new(rules::RuleSheet(&rules::REF_SCREEN)),
    ]
}

/// Banner kinds for row 4.
pub enum Banner<'a> {
    /// PLACEHOLDER: the view waits on this data.
    Awaiting(&'a str),
    /// A constant or rule table here is UNVERIFIED pending this reference.
    Unverified(&'a str),
    /// A plain note.
    Note(&'a str),
}

/// Title, ideas + sources line, disclaimer and banner; registers the SCH parameters.
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
    ws.set_column_width(0, 26)?;
    Ok(())
}

/// Section label at `row`, column `c`.
pub fn section(ws: &mut Worksheet, cx: &Cx, row: u32, c: u16, text: &str) -> Result<()> {
    ws.write_string_with_format(row, c, text, &cx.f.section)?;
    Ok(())
}

/// A column of a view table: typed (`In`, `Date`), or a formula of (1-based Excel row, data index).
pub enum Col<'a> {
    In(&'a str),
    /// A typed date (input fill, date format).
    Date(&'a str),
    F(&'a str, Box<dyn Fn(u32, u32) -> String + 'a>),
}

impl<'a> Col<'a> {
    /// A formula column; `f(r, i)` gets the 1-based Excel row and the 0-based data index.
    pub fn f(h: &'a str, f: impl Fn(u32, u32) -> String + 'a) -> Self {
        Self::F(h, Box::new(f))
    }
    fn header(&self) -> &str {
        match self {
            Self::In(h) | Self::Date(h) | Self::F(h, _) => h,
        }
    }
}

/// An Excel table `name` with its header at zero-based `hr`, `rows` body rows.
pub fn table(ws: &mut Worksheet, cx: &Cx, name: &str, hr: u32, c0: u16, cols: &[Col], rows: u32) -> Result<()> {
    let headers: Vec<&str> = cols.iter().map(Col::header).collect();
    let date = input_date();
    add_table(ws, name, hr, c0, &headers, rows as usize, TableStyle::Medium4)?;
    for i in 0..rows.max(1) {
        let r = hr + 1 + i;
        for (j, c) in cols.iter().enumerate() {
            let cc = c0 + j as u16;
            match c {
                Col::In(_) => ws.write_blank(r, cc, &cx.f.input).map(|_| ())?,
                Col::Date(_) => ws.write_blank(r, cc, &date).map(|_| ())?,
                Col::F(_, f) => ws.write_formula(r, cc, f(r + 1, i).as_str()).map(|_| ())?,
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

/// A cell of a data table `def`, data row `i`: `Data_Supp!$B$2`.
pub fn dcell(def: &TableDef, header: &str, i: u32) -> String {
    format!("{}!${}${}", def.sheet, col(def.col(header)), TableDef::row(i as usize))
}

/// `Calc_Marker!$G$5`: column `header` of marker row `i` (row-aligned with `tblMarkers`).
pub fn marker_cell(header: &str, i: u32) -> String {
    use crate::infra::calc::{MARKER_HEADERS, MARKER_TITLE};
    let c = MARKER_HEADERS.iter().position(|h| *h == header).expect("Calc_Marker column") as u16;
    format!("{MARKER_TITLE}!${}${}", col(c), i + 2)
}

/// `IF(src="","",src)`: mirror a cell without turning blanks into 0.
pub fn mirror(src: &str) -> String {
    format!("=IF({src}=\"\",\"\",{src})")
}

/// Drop-down of `items` on a zero-based block of one column.
pub fn list(ws: &mut Worksheet, r0: u32, c: u16, r1: u32, items: &[&str]) -> Result<()> {
    ws.add_data_validation(r0, c, r1, c, &DataValidation::new().allow_list_strings(items)?)?;
    Ok(())
}

/// Write `label` and a formula in the next column, with an optional number format.
pub fn kv(ws: &mut Worksheet, cx: &Cx, r: u32, c: u16, label: &str, formula: &str, nf: Option<&str>) -> Result<()> {
    ws.write_string_with_format(r, c, label, &cx.f.bold)?;
    match nf {
        Some(n) => ws.write_formula_with_format(r, c + 1, formula, &Format::new().set_num_format(n))?,
        None => ws.write_formula(r, c + 1, formula)?,
    };
    Ok(())
}

/// Apply a number format to zero-based rows `r0..=r1` of column `c`.
pub fn fmt_rows(ws: &mut Worksheet, r0: u32, r1: u32, c: u16, nf: &str) -> Result<()> {
    let f = Format::new().set_num_format(nf);
    for r in r0..=r1 {
        ws.set_cell_format(r, c, &f)?;
    }
    Ok(())
}

/// Publish a `K_SCH_*` KPI pointing at a cell of `title`.
pub fn kpi(cx: &Cx, name: &str, label: &str, title: &str, r: u32, c: u16, nf: Option<&str>) -> Result<()> {
    let f = format!("IFERROR(IF({0}=\"\",\"n/a\",{0}),\"n/a\")", cell(title, r, c));
    cx.shared.kpis.borrow_mut().publish(WS, name, label, Some(&f), nf)?;
    Ok(())
}

/// Due-date status of the date in `due` (blank stays blank): overdue / due soon / current.
pub fn due_status(due: &str, soon_days: &str) -> String {
    format!(
        "IF(NOT(ISNUMBER({due})),\"\",IF({due}<Today,\"overdue\",IF({due}<=Today+{soon_days},\"due soon\",\"current\")))"
    )
}

/// Colour a status column: overdue / due soon / current (and the extra `amber` words).
pub fn cf_status(ws: &mut Worksheet, r0: u32, c: u16, r1: u32, amber: &[&str]) -> Result<()> {
    let tl = format!("{}{}", col(c), r0 + 1);
    theme::cf_formula(ws, r0, c, r1, c, &format!("={tl}=\"overdue\""), theme::CRITICAL)?;
    theme::cf_formula(ws, r0, c, r1, c, &format!("={tl}=\"due soon\""), theme::OUT_OF_REF)?;
    theme::cf_formula(ws, r0, c, r1, c, &format!("={tl}=\"current\""), theme::OPTIMAL)?;
    for w in amber {
        theme::cf_formula(ws, r0, c, r1, c, &format!("={tl}=\"{w}\""), theme::REF_OK)?;
    }
    Ok(())
}

/// First day of the month `k` months from the current one.
pub fn month_start(k: u16) -> String {
    format!("(EOMONTH(Today,{})+1)", i32::from(k) - 1)
}

/// Formula (no `=`): 1-based index of the last keyword of `kw` found in `text`
/// (case-insensitive, blank keywords ignored), 0 when none matches.
pub fn match_idx(kw: &str, text: &str) -> String {
    format!("SUMPRODUCT(MAX(ISNUMBER(SEARCH({kw},{text}))*({kw}<>\"\")*(ROW({kw})-MIN(ROW({kw}))+1)))")
}

/// Age today in whole years from `DOB` ("" when unknown).
pub const AGE_NOW: &str = "IF(ISNUMBER(DOB),DATEDIF(DOB,Today,\"y\"),\"\")";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sch_sheet_is_prefixed_and_has_an_id() {
        let m = modules();
        assert_eq!(m.iter().filter(|x| x.title().starts_with("Sch_")).count(), 16, "16 Sch_ sheets");
        for x in &m {
            assert!(x.title().starts_with("Sch_") || x.title().starts_with("Ref_"), "{}", x.title());
            assert!(x.id().starts_with("sch-") || x.id().starts_with("ref-"), "{}", x.id());
        }
    }

    #[test]
    fn helpers_write_absolute_and_relative_refs() {
        assert_eq!(rng("Sch_X", 1, 5, 9), "Sch_X!$B$6:$B$10");
        assert_eq!(cell("Sch_X", 0, 0), "Sch_X!$A$1");
        assert_eq!(at(2, 7), "C7");
        assert_eq!(dcell(&crate::infra::data::SUPP, "name", 0), "Data_Supp!$B$2");
        assert_eq!(month_start(0), "(EOMONTH(Today,-1)+1)");
        assert!(due_status("D7", "Sch_Soon_d").contains("D7<Today"));
        assert!(match_idx("K", "B2").starts_with("SUMPRODUCT(MAX(ISNUMBER(SEARCH(K,B2))"));
    }
}
