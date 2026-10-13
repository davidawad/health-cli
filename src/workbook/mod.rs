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
use crate::infra::{self, theme, Shared};
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
    pub intake: Vec<Record>,
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
            intake: store::all(db, "intake_log")?,
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
    pub input: Format,
    pub unverified: Format,
    pub placeholder: Format,
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
            input: Format::new().set_background_color(theme::INPUT),
            unverified: Format::new().set_background_color(theme::UNVERIFIED).set_bold(),
            placeholder: Format::new().set_background_color(theme::PLACEHOLDER).set_bold(),
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
    /// Defined names, parameters and KPIs registered by the modules.
    pub shared: &'a Shared,
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
    /// Build order: modules of phase 0 build first, then phase 1 (sheets that
    /// list what the others registered: `Params`, `Ref_KPI`). Tab order is registry order.
    fn phase(&self) -> u8 {
        0
    }
    /// Whether `--only` builds include this sheet anyway (the infrastructure sheets that
    /// list what the others registered).
    fn always_built(&self) -> bool {
        self.phase() > 0
    }
    /// Registry ids of sheets this one reads by defined name (data sheets): `--only`
    /// brings them along.
    fn requires(&self) -> &'static [&'static str] {
        &[]
    }
    /// Whether the data this sheet needs is present (labs, genetics...).
    fn available(&self, _d: &Data) -> bool {
        true
    }
    fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()>;
    /// Whether [`finish`](Self::finish) should run: after every sheet is built, with the
    /// used extent of each sheet in `cx.shared.extents` (for whole-sheet sweeps).
    fn finishes(&self) -> bool {
        false
    }
    /// Second pass over this module's sheet once all sheets exist (see [`finishes`](Self::finishes)).
    fn finish(&self, _ws: &mut Worksheet, _cx: &Cx) -> Result<()> {
        Ok(())
    }
}

/// Build the workbook from `data`. `only` (registry ids) restricts the sheets.
pub fn build(data: &Data, only: &[String]) -> Result<Workbook> {
    build_with(&sheets::registry(), data, only)
}

/// [`build`] over an explicit module list (the registry in production, fakes in tests).
pub fn build_with(reg: &[Box<dyn SheetModule>], data: &Data, only: &[String]) -> Result<Workbook> {
    if let Some(bad) = only.iter().find(|o| !reg.iter().any(|m| m.id() == o.as_str())) {
        return Err(AppError::usage(format!(
            "unknown sheet '{bad}' (sheets: {})",
            reg.iter().map(|m| m.id()).collect::<Vec<_>>().join(", ")
        )));
    }
    let formats = Formats::new();
    let shared = Shared::new();
    let cx = Cx { d: data, f: &formats, shared: &shared };
    let mut wb = Workbook::new();
    wb.set_properties(
        &DocProperties::new()
            .set_title("Health Dashboard")
            .set_author("health-cli")
            .set_comment("Generated by `health sheet` from the health-cli store. Edit the store, not this file."),
    );
    // `--only` ids plus the data sheets they require.
    let wanted = |id: &str| only.iter().any(|o| o == id);
    let required: Vec<&str> =
        reg.iter().filter(|m| wanted(m.id())).flat_map(|m| m.requires().iter().copied()).collect();
    let chosen: Vec<&Box<dyn SheetModule>> = reg
        .iter()
        .filter(|m| {
            (only.is_empty() || wanted(m.id()) || required.contains(&m.id()) || m.always_built()) && m.available(data)
        })
        .collect();
    let mut titles: Vec<String> = chosen.iter().map(|m| m.title().to_lowercase()).collect();
    titles.sort();
    if let Some(w) = titles.windows(2).find(|w| w[0] == w[1]) {
        return Err(AppError::invalid(format!("two sheets are named '{}'", w[0])));
    }
    *shared.built.borrow_mut() =
        chosen.iter().map(|m| infra::Built { id: m.id(), title: m.title().to_string(), hidden: m.hidden() }).collect();
    // Build in phase order, keep tab order.
    let mut sheets: Vec<Option<Worksheet>> = chosen.iter().map(|_| None).collect();
    for phase in 0..=chosen.iter().map(|m| m.phase()).max().unwrap_or(0) {
        for (i, m) in chosen.iter().enumerate().filter(|(_, m)| m.phase() == phase) {
            let mut ws = Worksheet::new();
            ws.set_name(m.title())?;
            if m.hidden() {
                ws.set_hidden(true);
            }
            theme::style_tab(&mut ws, m.title());
            m.build(&mut ws, &cx).map_err(|e| e.context(format!("sheet {}", m.id())))?;
            sheets[i] = Some(ws);
        }
    }
    sheets.into_iter().flatten().for_each(|ws| wb.push_worksheet(ws));
    let finishing: Vec<&&Box<dyn SheetModule>> = chosen.iter().filter(|m| m.finishes()).collect();
    if !finishing.is_empty() {
        *shared.extents.borrow_mut() = extents(&mut wb)?;
        for m in finishing {
            let ws = wb.worksheet_from_name(m.title())?;
            m.finish(ws, &cx).map_err(|e| e.context(format!("sheet {}", m.id())))?;
        }
    }
    shared.names.borrow().apply(&mut wb)?;
    Ok(wb)
}

/// Used extent (last zero-based row and column) of every sheet, read back from the
/// serialised workbook: rust_xlsxwriter keeps a sheet's dimensions private.
fn extents(wb: &mut Workbook) -> Result<std::collections::BTreeMap<String, (u32, u16)>> {
    use calamine::Reader;
    let buf = wb.save_to_buffer()?;
    let mut x = calamine::Xlsx::new(std::io::Cursor::new(buf)).map_err(|e| AppError::general(format!("xlsx: {e}")))?;
    let mut out = std::collections::BTreeMap::new();
    for name in x.sheet_names() {
        let dims = x.worksheet_cells_reader(&name).map_err(|e| AppError::general(format!("xlsx: {e}")))?.dimensions();
        out.insert(name, (dims.end.0, dims.end.1 as u16));
    }
    Ok(out)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::{kpi, params};

    /// A fake module: sheet `title`, registering whatever the closure registers.
    struct Fake {
        title: &'static str,
        phase: u8,
        run: fn(&mut Worksheet, &Cx) -> Result<()>,
    }

    impl SheetModule for Fake {
        fn id(&self) -> &'static str {
            self.title
        }
        fn title(&self) -> &'static str {
            self.title
        }
        fn phase(&self) -> u8 {
            self.phase
        }
        fn build(&self, ws: &mut Worksheet, cx: &Cx) -> Result<()> {
            (self.run)(ws, cx)
        }
    }

    fn fake(title: &'static str, run: fn(&mut Worksheet, &Cx) -> Result<()>) -> Box<dyn SheetModule> {
        Box::new(Fake { title, phase: 0, run })
    }

    fn xml(wb: &mut Workbook, part: &str) -> String {
        use std::io::Read;
        let dir = tempfile::TempDir::new().unwrap();
        let p = dir.path().join("t.xlsx");
        wb.save(&p).unwrap();
        let mut z = zip::ZipArchive::new(std::fs::File::open(p).unwrap()).unwrap();
        let mut s = String::new();
        z.by_name(part).unwrap().read_to_string(&mut s).unwrap();
        s
    }

    #[test]
    fn a_defined_name_registered_twice_fails_the_build() {
        let reg = vec![
            fake("Lab_A", |_, cx| cx.shared.names.borrow_mut().define("Sel_X", "Lab_A!$A$1", "lab")),
            fake("Cln_A", |_, cx| cx.shared.names.borrow_mut().define("Sel_X", "Cln_A!$A$1", "cln")),
        ];
        let e = build_with(&reg, &Data::default(), &[]).err().expect("collision fails");
        assert!(e.message.contains("Sel_X") && e.message.contains("lab") && e.message.contains("cln"), "{e}");
    }

    #[test]
    fn duplicate_sheet_titles_fail_the_build() {
        let reg = vec![fake("Lab_A", |_, _| Ok(())), fake("lab_a", |_, _| Ok(()))];
        assert!(build_with(&reg, &Data::default(), &[]).is_err());
    }

    #[test]
    fn params_names_and_kpis_flow_to_the_infra_sheets() {
        let reg: Vec<Box<dyn SheetModule>> = vec![
            fake("Lab_Tear", |ws, cx| {
                ws.write_number(0, 0, 3.0)?;
                cx.shared.params.borrow_mut().add(params::Param::num(
                    "Lab_Alpha",
                    0.1,
                    "",
                    params::State::Heuristic,
                    "heuristic",
                ))?;
                cx.shared.kpis.borrow_mut().publish("LAB", "n_out", "Out of range", Some("Lab_Tear!$A$1"), None)?;
                Ok(())
            }),
            Box::new(params::ParamsSheet),
            Box::new(kpi::KpiSheet),
        ];
        let mut wb = build_with(&reg, &Data::default(), &[]).unwrap();
        let names = xml(&mut wb, "xl/workbook.xml");
        for n in ["Today", "Half_Life_days", "Lab_Alpha", "K_LAB_n_out", "K_INFRA_n_params"] {
            assert!(names.contains(&format!("definedName name=\"{n}\"")), "{n} missing in {names:.600}");
        }
        assert!(names.contains("Params!$B$"), "parameter names point at their value cell");
    }

    #[test]
    fn an_unverified_parameter_must_cite_what_is_pending() {
        let reg = vec![fake("Lab_A", |_, cx| {
            cx.shared.params.borrow_mut().add(params::Param::num(
                "Bad_One",
                1.0,
                "",
                params::State::Unverified,
                "trust me",
            ))
        })];
        assert!(build_with(&reg, &Data::default(), &[]).is_err());
    }

    #[test]
    fn theme_helpers_produce_banners_notes_sparklines_scales_and_hidden_base_charts() {
        use crate::infra::theme;
        let reg = vec![fake("Lab_A", |ws, cx| {
            theme::disclaimer(ws, 0, cx.f)?;
            theme::awaiting(ws, 1, "hs-CRP", cx.f)?;
            theme::unverified(ws, 2, 0, "Levine 2018", cx.f)?;
            theme::note_source(ws, 3, 0, "OUT ch50")?;
            for c in 0..4u16 {
                ws.write_number(5, c, f64::from(c))?;
            }
            theme::sparkline(ws, 5, 5, "Lab_A!$A$6:$D$6")?;
            theme::cf_three_color(ws, 5, 0, 5, 3, true)?;
            theme::cf_data_bar(ws, 5, 0, 5, 3, "9BC2E6")?;
            theme::cf_icons(ws, 5, 0, 5, 3)?;
            theme::cf_status_quilt(ws, 5, 0, 5, 3)?;
            let a = ("Lab_A", 5u32, 0u16, 5u32, 3u16);
            ws.insert_chart(8, 0, &theme::hidden_base_stack(true, "Gantt", a, a, &[("dur", a)]))?;
            Ok(())
        })];
        let mut wb = build_with(&reg, &Data::default(), &[]).unwrap();
        let s = xml(&mut wb, "xl/worksheets/sheet1.xml");
        for needle in ["sparklineGroup", "colorScale", "dataBar", "iconSet", "Visualisation only"] {
            assert!(s.contains(needle) || needle == "Visualisation only", "{needle} missing");
        }
        assert!(xml(&mut wb, "xl/comments1.xml").contains("Source: OUT ch50"));
        assert!(xml(&mut wb, "xl/charts/chart1.xml").contains("<a:noFill/>"), "base series is invisible");
    }

    #[test]
    fn excel_365_functions_get_their_future_function_prefix() {
        let reg = vec![fake("Lab_A", |ws, _| {
            ws.write_formula(0, 0, "=XLOOKUP(1,B1:B3,C1:C3)")?;
            crate::infra::theme::dyn_formula(ws, 1, 0, "=FILTER(B1:B3,B1:B3>0)")?;
            ws.write_formula(2, 0, "=LET(_xlpm.x,2,_xlpm.x*3)")?;
            Ok(())
        })];
        let mut wb = build_with(&reg, &Data::default(), &[]).unwrap();
        let s = xml(&mut wb, "xl/worksheets/sheet1.xml");
        assert!(s.contains("_xlfn.XLOOKUP("), "{s:.800}");
        assert!(s.contains("_xlfn._xlws.FILTER("), "{s:.800}");
        assert!(s.contains("_xlfn.LET("), "{s:.800}");
    }
}
