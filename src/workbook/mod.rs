//! The Health Dashboard workbook generator.
//!
//! One module per sheet or view under [`sheets`]; each implements
//! [`SheetModule`] and is listed once in [`sheets::registry`]. Adding a view
//! is: write a module, add one line to the registry. A module gets its own
//! worksheet and the shared [`Cx`] (data + formats) and nothing else, so
//! views stay independent. Cross-sheet references go through small `pub fn`s
//! on the module that owns the layout (for example [`sheets::supplements::cost_range`]),
//! never through hard-coded cell addresses in the referring module.
//!
//! Output rules: Excel-native formulas only (no Google-only `FILTER` or
//! `__xludf.DUMMYFUNCTION`), real Excel tables, conditional formats, data
//! validation, charts. Derived values (`$/yr`, weekly dose, BMI, ages,
//! totals) are formulas; the store only holds inputs.

pub mod labs;
pub mod sheets;

use std::path::Path;

use chrono::NaiveDate;
use rust_xlsxwriter::{
    DocProperties, Format, FormatAlign, FormatBorder, Table, TableColumn, TableStyle, Workbook, Worksheet, XlsxError,
};

use crate::db::Db;
use crate::error::{AppError, Result};
use crate::output::Record;
use crate::store;

impl From<XlsxError> for AppError {
    fn from(e: XlsxError) -> Self {
        Self::general(format!("xlsx: {e}"))
    }
}

/// Everything a sheet may draw on: the store's tables plus (optionally) labs
/// and genetics fetched from the sibling tools.
#[derive(Debug, Default)]
pub struct Data {
    pub facts: Vec<Record>,
    pub predispositions: Vec<Record>,
    pub supplements: Vec<Record>,
    pub sources: Vec<Record>,
    pub evidence: Vec<Record>,
    pub products: Vec<Record>,
    pub nutrition_targets: Vec<Record>,
    pub nutrition: Vec<Record>,
    pub routine: Vec<Record>,
    pub exercises: Vec<Record>,
    pub exercise_alts: Vec<Record>,
    pub vaccines: Vec<Record>,
    pub vaccine_doses: Vec<Record>,
    pub physicals: Vec<Record>,
    pub providers: Vec<Record>,
    pub sizes: Vec<Record>,
    pub resources: Vec<Record>,
    pub notes: Vec<Record>,
    pub labs: Option<labs::Labs>,
    pub kits: Option<Vec<labs::Kit>>,
    /// Date stamped in the workbook's "generated" note.
    pub generated_on: Option<NaiveDate>,
}

impl Data {
    /// Load every table of the store.
    pub fn load(db: &Db) -> Result<Self> {
        Ok(Self {
            facts: store::all(db, "profile_fact")?,
            predispositions: store::all(db, "predisposition")?,
            supplements: store::supplements(db)?,
            sources: store::all(db, "source")?,
            evidence: store::all(db, "evidence_ref")?,
            products: store::all(db, "product")?,
            nutrition_targets: store::all(db, "nutrition_target")?,
            nutrition: store::all(db, "nutrition_item")?,
            routine: store::all(db, "routine_step")?,
            exercises: store::all(db, "exercise")?,
            exercise_alts: store::all(db, "exercise_alt")?,
            vaccines: store::all(db, "vaccine")?,
            vaccine_doses: store::all(db, "vaccine_dose")?,
            physicals: store::all(db, "physical_check")?,
            providers: store::all(db, "provider")?,
            sizes: store::all(db, "size")?,
            resources: store::all(db, "resource_link")?,
            notes: store::all(db, "note")?,
            labs: None,
            kits: None,
            generated_on: Some(chrono::Utc::now().date_naive()),
        })
    }

    /// A base fact's value by key (`height`, `birthdate`, ...).
    pub fn fact(&self, key: &str) -> Option<&Record> {
        self.facts.iter().find(|f| f.get("key").and_then(|k| k.as_str()) == Some(key))
    }
}

/// Cell formats shared by all sheets.
pub struct Formats {
    pub header: Format,
    pub bold: Format,
    pub title: Format,
    pub money: Format,
    pub money0: Format,
    pub date: Format,
    pub wrap: Format,
    pub url: Format,
    pub pct: Format,
    pub dec1: Format,
    pub dec2: Format,
    pub int: Format,
    pub note: Format,
    pub section: Format,
    pub good: Format,
    pub warn: Format,
    pub bad: Format,
    pub neutral: Format,
}

impl Formats {
    pub fn new() -> Self {
        let base = || Format::new().set_align(FormatAlign::Top);
        Self {
            header: Format::new()
                .set_bold()
                .set_font_color("FFFFFF")
                .set_background_color("1F3864")
                .set_border(FormatBorder::Thin)
                .set_text_wrap(),
            bold: Format::new().set_bold(),
            title: Format::new().set_bold().set_font_size(14),
            money: base().set_num_format("$#,##0.00"),
            money0: base().set_num_format("$#,##0"),
            date: base().set_num_format("yyyy-mm-dd").set_align(FormatAlign::Left),
            wrap: base().set_text_wrap(),
            url: Format::new().set_font_color("0563C1").set_underline(rust_xlsxwriter::FormatUnderline::Single),
            pct: base().set_num_format("0%"),
            dec1: base().set_num_format("0.0"),
            dec2: base().set_num_format("0.00"),
            int: base().set_num_format("0"),
            note: Format::new().set_italic().set_font_color("7F7F7F"),
            section: Format::new().set_bold().set_background_color("D9E2F3"),
            good: Format::new().set_background_color("B7E1CD"),
            warn: Format::new().set_background_color("FCE8B2"),
            bad: Format::new().set_background_color("F4C7C3"),
            neutral: Format::new().set_background_color("E7E6E6"),
        }
    }
}

impl Default for Formats {
    fn default() -> Self {
        Self::new()
    }
}

/// What a sheet module is given.
pub struct Cx<'a> {
    pub d: &'a Data,
    pub f: &'a Formats,
}

/// One sheet or view of the workbook.
pub trait SheetModule: Sync {
    /// Stable registry name (`--only supplements`).
    fn id(&self) -> &'static str;
    /// Tab title. Cross-sheet formulas refer to it through the owning module's `TITLE`.
    fn title(&self) -> &'static str;
    fn hidden(&self) -> bool {
        false
    }
    /// Whether the data this sheet needs is present (labs, genetics...).
    fn available(&self, _d: &Data) -> bool {
        true
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()>;
}

/// Build the workbook from `data`. `only` (registry ids) restricts the sheets.
pub fn build(data: &Data, only: &[String]) -> Result<Workbook> {
    let reg = sheets::registry();
    if let Some(bad) = only.iter().find(|o| !reg.iter().any(|m| m.id() == o.as_str())) {
        return Err(AppError::usage(format!(
            "unknown sheet '{bad}' (sheets: {})",
            reg.iter().map(|m| m.id()).collect::<Vec<_>>().join(", ")
        )));
    }
    let formats = Formats::new();
    let cx = Cx { d: data, f: &formats };
    let mut wb = Workbook::new();
    wb.set_properties(
        &DocProperties::new()
            .set_title("Health Dashboard")
            .set_author("health-cli")
            .set_comment("Generated by `health sheet` from the health-cli store. Edit the store, not this file."),
    );
    for m in reg.iter().filter(|m| (only.is_empty() || only.iter().any(|o| o == m.id())) && m.available(data)) {
        let ws = wb.add_worksheet();
        ws.set_name(m.title())?;
        if m.hidden() {
            ws.set_hidden(true);
        }
        m.build(ws, &cx).map_err(|e| e.context(format!("sheet {}", m.id())))?;
    }
    Ok(wb)
}

// ------------------------------------------------------------------ helpers

/// Sheet name as written in a formula (quoted when it needs it).
pub fn q(title: &str) -> String {
    if title.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        title.to_string()
    } else {
        format!("'{}'", title.replace('\'', "''"))
    }
}

/// `A`, `B`, ... `AA` for a zero-based column index.
pub fn col(c: u16) -> String {
    rust_xlsxwriter::utility::column_number_to_name(c)
}

/// Absolute column range `'Sheet'!$C$2:$C$40` (rows are 1-based, as in Excel).
pub fn range(title: &str, c: u16, first_row: u32, last_row: u32) -> String {
    format!("{}!${}${}:${}${}", q(title), col(c), first_row, col(c), last_row)
}

/// A table over `rows` data rows starting under `header_row`, with
/// `headers` as its columns. Always has at least one data row (Excel requires it).
pub fn add_table(
    ws: &mut Worksheet,
    name: &str,
    header_row: u32,
    first_col: u16,
    headers: &[&str],
    rows: usize,
    style: TableStyle,
) -> Result<()> {
    let columns: Vec<TableColumn> = headers.iter().map(|h| TableColumn::new().set_header(*h)).collect();
    let table = Table::new().set_name(name).set_style(style).set_columns(&columns);
    let last_row = header_row + rows.max(1) as u32;
    ws.add_table(header_row, first_col, last_row, first_col + headers.len() as u16 - 1, &table)?;
    Ok(())
}

pub fn set_widths(ws: &mut Worksheet, widths: &[f64]) -> Result<()> {
    widths.iter().enumerate().try_for_each(|(i, w)| ws.set_column_width(i as u16, *w).map(|_| ()))?;
    Ok(())
}

/// Write the workbook to `path` atomically (temp file in the same directory,
/// then rename), owner-only on Unix.
pub fn save(wb: &mut Workbook, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("xlsx.tmp");
    wb.save(&tmp)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(&tmp, path).map_err(|e| AppError::io(format!("writing {}: {e}", path.display())))
}
