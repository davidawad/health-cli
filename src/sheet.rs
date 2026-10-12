//! Reading an xlsx workbook (read-only) into plain cell grids.
//!
//! Cells are reduced to text, numbers and dates. For a formula cell calamine
//! returns the cached value (for Google Sheets exports that is also what a
//! `__xludf.DUMMYFUNCTION("...")` wrapper evaluated to), and [`Grid::has_formula`]
//! says the cell was computed, so importers can store inputs and skip derived
//! columns. Grid coordinates are absolute: `rows[0][0]` is A1 even when the used
//! range starts later, so column letters and row numbers match Excel.

use std::path::Path;

use calamine::{open_workbook_auto, Data, Reader};
use chrono::NaiveDate;

use crate::error::{AppError, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Empty,
    Number(f64),
    Text(String),
    Date(NaiveDate),
}

impl Cell {
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Empty => true,
            Self::Text(t) => t.trim().is_empty(),
            _ => false,
        }
    }

    /// Text as an importer sees it: numbers without a spurious `.0`, dates as ISO.
    pub fn text(&self) -> String {
        match self {
            Self::Empty => String::new(),
            Self::Number(n) => fmt_number(*n),
            Self::Text(t) => t.trim().to_string(),
            Self::Date(d) => d.format("%Y-%m-%d").to_string(),
        }
    }

    pub fn number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            Self::Text(t) => t.trim().parse().ok(),
            _ => None,
        }
    }
}

/// Shortest decimal for a cell number (`5` not `5.0`, no binary noise).
pub fn fmt_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        let s = format!("{n:.10}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

#[derive(Debug, Clone)]
pub struct Grid {
    pub name: String,
    pub rows: Vec<Vec<Cell>>,
    formulas: Vec<Vec<bool>>,
}

static EMPTY: Cell = Cell::Empty;

impl Grid {
    pub fn new(name: &str, rows: Vec<Vec<Cell>>) -> Self {
        Self { name: name.to_string(), rows, formulas: Vec::new() }
    }

    pub fn cell(&self, row: usize, col: usize) -> &Cell {
        self.rows.get(row).and_then(|r| r.get(col)).unwrap_or(&EMPTY)
    }

    /// Whether the cell holds a formula (its value is then derived).
    pub fn has_formula(&self, row: usize, col: usize) -> bool {
        self.formulas.get(row).and_then(|r| r.get(col)).copied().unwrap_or(false)
    }

    pub fn height(&self) -> usize {
        self.rows.len()
    }

    /// Cell at `(row, col)` as trimmed text (`None` when blank).
    pub fn text(&self, row: usize, col: usize) -> Option<String> {
        let c = self.cell(row, col);
        (!c.is_empty()).then(|| c.text())
    }

    pub fn number(&self, row: usize, col: usize) -> Option<f64> {
        self.cell(row, col).number()
    }

    /// Literal (non-formula, non-blank) cells of a row, except in `skip` columns.
    pub fn literals(&self, row: usize, skip: &[usize]) -> usize {
        (0..self.rows.get(row).map_or(0, Vec::len))
            .filter(|c| !skip.contains(c) && !self.cell(row, *c).is_empty() && !self.has_formula(row, *c))
            .count()
    }

    /// Formula cells of a row, except in `skip` columns.
    pub fn formula_count(&self, row: usize, skip: &[usize]) -> usize {
        (0..self.formulas.get(row).map_or(0, Vec::len))
            .filter(|c| !skip.contains(c) && self.has_formula(row, *c))
            .count()
    }
}

/// What a worksheet row is, judged on column A (the row label) and the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// Column A empty.
    Blank,
    /// Only a label: a category / section heading.
    Header,
    /// Label plus formulas only (totals, sums): derived, never stored.
    Summary,
    /// Label plus at least one literal cell.
    Data,
}

/// Classify row `r`. Formulas in `ignore_formulas` columns (e.g. a per-row
/// "qualifies?" formula that also sits on heading rows) do not count.
pub fn classify(g: &Grid, r: usize, ignore_formulas: &[usize]) -> RowKind {
    if g.text(r, 0).is_none() {
        return RowKind::Blank;
    }
    let skip: Vec<usize> = std::iter::once(0).chain(ignore_formulas.iter().copied()).collect();
    match (g.literals(r, &[0]), g.formula_count(r, &skip)) {
        (0, 0) => RowKind::Header,
        (0, _) => RowKind::Summary,
        _ => RowKind::Data,
    }
}

fn open(path: &Path) -> Result<calamine::Sheets<std::io::BufReader<std::fs::File>>> {
    open_workbook_auto(path).map_err(|e| AppError::io(format!("reading spreadsheet {}: {e}", path.display())))
}

/// Names of every sheet in the workbook, in order.
pub fn sheet_names(path: &Path) -> Result<Vec<String>> {
    Ok(open(path)?.sheet_names())
}

/// Read a sheet by exact (case-insensitive) name, or `None` when it does not exist.
pub fn read(path: &Path, sheet: &str) -> Result<Option<Grid>> {
    let mut wb = open(path)?;
    let Some(name) = wb.sheet_names().into_iter().find(|n| n.eq_ignore_ascii_case(sheet)) else {
        return Ok(None);
    };
    let range = wb.worksheet_range(&name).map_err(|e| AppError::invalid(format!("sheet '{name}': {e}")))?;
    let formula = wb.worksheet_formula(&name).map_err(|e| AppError::invalid(format!("sheet '{name}': {e}")))?;
    let pad = |start: Option<(u32, u32)>| start.map_or((0, 0), |(r, c)| (r as usize, c as usize));
    let (r0, c0) = pad(range.start());
    let mut rows: Vec<Vec<Cell>> = vec![Vec::new(); r0];
    rows.extend(
        range.rows().map(|row| std::iter::repeat_n(Cell::Empty, c0).chain(row.iter().map(convert)).collect::<Vec<_>>()),
    );
    let (fr0, fc0) = pad(formula.start());
    let mut formulas: Vec<Vec<bool>> = vec![Vec::new(); fr0];
    formulas.extend(formula.rows().map(|row| {
        std::iter::repeat_n(false, fc0).chain(row.iter().map(|f| !f.trim().is_empty())).collect::<Vec<_>>()
    }));
    Ok(Some(Grid { name, rows, formulas }))
}

fn convert(d: &Data) -> Cell {
    match d {
        Data::Empty | Data::Error(_) => Cell::Empty,
        Data::Int(i) => Cell::Number(*i as f64),
        Data::Float(f) => Cell::Number(*f),
        Data::Bool(b) => Cell::Text(b.to_string()),
        Data::String(s) if s.trim().is_empty() => Cell::Empty,
        Data::String(s) => Cell::Text(s.clone()),
        Data::DateTime(dt) if dt.is_datetime() => {
            let (y, m, d, ..) = dt.to_ymd_hms_milli();
            NaiveDate::from_ymd_opt(i32::from(y), u32::from(m), u32::from(d))
                .map_or_else(|| Cell::Number(dt.as_f64()), Cell::Date)
        }
        Data::DateTime(dt) => Cell::Number(dt.as_f64()),
        Data::DateTimeIso(s) => s
            .get(..10)
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
            .map_or_else(|| Cell::Text(s.clone()), Cell::Date),
        Data::DurationIso(s) => Cell::Text(s.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_print_without_noise() {
        assert_eq!(fmt_number(5.0), "5");
        assert_eq!(fmt_number(0.1 + 0.2), "0.3");
        assert_eq!(fmt_number(-1.25), "-1.25");
    }

    #[test]
    fn rows_classify_by_label_literals_and_formulas() {
        let mut g =
            Grid::new("t", vec![vec![Cell::Text("Head".into())], vec![Cell::Text("Item".into()), Cell::Number(1.0)]]);
        assert_eq!(classify(&g, 0, &[]), RowKind::Header);
        assert_eq!(classify(&g, 1, &[]), RowKind::Data);
        assert_eq!(classify(&g, 2, &[]), RowKind::Blank);
        g.rows.push(vec![Cell::Text("Total".into()), Cell::Number(9.0)]);
        g.formulas = vec![vec![], vec![], vec![false, true]];
        assert_eq!(classify(&g, 2, &[]), RowKind::Summary);
        assert_eq!(classify(&g, 2, &[1]), RowKind::Header, "ignored formula columns do not count");
    }
}
