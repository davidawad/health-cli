//! Helpers shared by the Cln_* views: lab lookups over the data layer, a small
//! table writer whose formulas name their columns (`[TG]/[HDL]`), the sheet
//! header block and chart builders.

use rust_xlsxwriter::{
    Chart, ChartLine, ChartLineDashType, ChartMarker, ChartType, Format, Formula, TableStyle, Worksheet,
};

use crate::error::Result;
use crate::infra::calc::{MARKER_HEADERS, MARKER_TITLE};
use crate::infra::data::{LABS, MARKERS, PANELS};
use crate::infra::theme;
use crate::workbook::{add_table, col, range, Cx, Data};

/// The workstream tag of the KPIs (`K_CLN_*`).
pub const WS: &str = "CLN";

/// Formula fragments (no leading `=`) that read `tblLabs`, `tblMarkers` and `Calc_Marker`.
pub struct Lab {
    lm: String,
    ld: String,
    lv: String,
    nm: u32,
}

impl Lab {
    pub fn new(d: &Data) -> Self {
        let nl = LABS.nrows(d);
        Self {
            lm: LABS.range_n(nl, "marker_id"),
            ld: LABS.range_n(nl, "panel_date"),
            lv: LABS.range_n(nl, "value"),
            nm: MARKERS.nrows(d),
        }
    }

    /// Value of `marker` drawn on the date in cell `date`; `""` when it was not measured.
    pub fn at(&self, marker: &str, date: &str) -> String {
        self.at_x(&format!("\"{marker}\""), date)
    }

    /// [`Lab::at`] with the marker given as an expression (a cell or a name).
    pub fn at_x(&self, m: &str, date: &str) -> String {
        let (lm, ld, lv) = (&self.lm, &self.ld, &self.lv);
        format!("IF(COUNTIFS({lm},{m},{ld},{date})=0,\"\",SUMIFS({lv},{lm},{m},{ld},{date}))")
    }

    /// Value of `marker` on the latest draw on or before the date in cell `date` within `days`.
    pub fn near(&self, marker: &str, date: &str, days: &str) -> String {
        let (lm, ld, lv) = (&self.lm, &self.ld, &self.lv);
        let last = format!("MAXIFS({ld},{lm},\"{marker}\",{ld},\"<=\"&{date},{ld},\">=\"&({date}-{days}))");
        format!("IF({last}=0,\"\",SUMIFS({lv},{lm},\"{marker}\",{ld},{last}))")
    }

    /// A `Calc_Marker` column (`last`, `n`, `last_date`, ...) of `marker`; `""` when unknown.
    pub fn calc(&self, marker: &str, header: &str) -> String {
        let c = MARKER_HEADERS.iter().position(|h| *h == header).expect("Calc_Marker column") as u16;
        let (ids, vals) = (range(MARKER_TITLE, 0, 2, 1 + self.nm), range(MARKER_TITLE, c, 2, 1 + self.nm));
        format!("IFERROR(INDEX({vals},MATCH(\"{marker}\",{ids},0)),\"\")")
    }

    /// A numeric `tblMarkers` attribute (`ref_lo`, `opt_hi`, ...) of `marker`; `""` when blank.
    pub fn bound(&self, marker: &str, header: &str) -> String {
        let (ids, vals) = (MARKERS.range_n(self.nm, "marker_id"), MARKERS.range_n(self.nm, header));
        let x = format!("INDEX({vals},MATCH(\"{marker}\",{ids},0))");
        format!("IFERROR(IF(ISNUMBER({x}),{x},\"\"),\"\")")
    }

    /// Absolute ranges of `tblLabs` marker / date / value, for views that scan the table.
    pub fn ranges(&self) -> (&str, &str, &str) {
        (&self.lm, &self.ld, &self.lv)
    }
}

/// Lab panels the per-panel tables list (at least one row).
pub fn n_panels(d: &Data) -> u32 {
    PANELS.nrows(d)
}

/// Template: the date of the panel on this row (`{p}` = Excel row of `tblPanels`).
pub fn panel_date() -> String {
    let p = "{p}";
    let c = col(PANELS.col("date"));
    format!("=IF({s}!${c}${p}=\"\",\"\",{s}!${c}${p})", s = PANELS.sheet)
}

/// Template: age at the panel on this row (`tblPanels.age_at_draw`, needs `DOB`).
pub fn panel_age() -> String {
    let p = "{p}";
    let c = col(PANELS.col("age_at_draw"));
    format!("=IFERROR(IF(ISNUMBER({s}!${c}${p}),{s}!${c}${p},\"\"),\"\")", s = PANELS.sheet)
}

/// `IF(COUNT(cells)<n,"",expr)`: blank unless every input is a number.
pub fn need(cells: &[&str], expr: &str) -> String {
    format!("=IFERROR(IF(COUNT({})<{},\"\",{expr}),\"\")", cells.join(","), cells.len())
}

/// A column of a view table: header, formula template (blank = an input column) and format.
pub struct TCol<'a> {
    pub head: &'a str,
    pub f: String,
    pub fmt: Option<&'a Format>,
}

pub fn c<'a>(head: &'a str, f: impl Into<String>) -> TCol<'a> {
    TCol { head, f: f.into(), fmt: None }
}

pub fn cf<'a>(head: &'a str, f: impl Into<String>, fmt: &'a Format) -> TCol<'a> {
    TCol { head, f: f.into(), fmt: Some(fmt) }
}

/// Where a view table landed.
pub struct Placed {
    pub sheet: &'static str,
    pub heads: Vec<String>,
    pub c0: u16,
    /// Zero-based header row, first and last data rows.
    pub hr: u32,
    pub first: u32,
    pub last: u32,
}

impl Placed {
    pub fn col(&self, h: &str) -> u16 {
        self.c0 + self.heads.iter().position(|x| x == h).unwrap_or_else(|| panic!("no column {h}")) as u16
    }
    /// Absolute data range of a column (`'Cln_X'!$C$7:$C$12`).
    pub fn range(&self, h: &str) -> String {
        range(self.sheet, self.col(h), self.first + 1, self.last + 1)
    }
    /// Cell of column `h` on the last data row.
    pub fn last_cell(&self, h: &str) -> String {
        format!("{}!${}${}", crate::workbook::q(self.sheet), col(self.col(h)), self.last + 1)
    }
    /// Chart area of a column.
    pub fn area(&self, h: &str) -> (&'static str, u32, u16, u32, u16) {
        let c = self.col(h);
        (self.sheet, self.first, c, self.last, c)
    }
}

/// Expand a template. `[H]` -> this row's cell of column `H`; `[[H]]` -> the whole data
/// range of `H`; `[^H]` -> `H` from the first data row down to this row; `[<H]` -> `H`
/// from the first data row to the row above (the header row on the first row);
/// `{r}` -> this Excel row; `{p}` -> the Excel row of the matching `Data_*` row.
/// A `[` right after a name (`tblX[col]`) is a structured reference and is kept.
pub fn expand(heads: &[String], c0: u16, t: &str, r: u32, p: u32, span: (u32, u32)) -> String {
    let (first, last) = span;
    let mut out = String::with_capacity(t.len() + 32);
    let mut rest = t;
    while let Some(i) = rest.find('[') {
        out.push_str(&rest[..i]);
        // `tblX[col]`: a structured reference, copied through unchanged.
        if out.ends_with(|ch: char| ch.is_ascii_alphanumeric() || ch == '_') {
            let j = rest[i..].find(']').map(|j| i + j).unwrap_or_else(|| panic!("unclosed [ in {t}"));
            out.push_str(&rest[i..=j]);
            rest = &rest[j + 1..];
            continue;
        }
        let whole = rest[i..].starts_with("[[");
        let open = if whole { i + 2 } else { i + 1 };
        let j = rest[open..].find(']').map(|j| open + j).unwrap_or_else(|| panic!("unclosed [ in {t}"));
        let key = &rest[open..j];
        let (mode, h) = match key.chars().next() {
            Some(m @ ('^' | '<')) => (m, &key[1..]),
            _ => (if whole { '=' } else { ' ' }, key),
        };
        let k = heads.iter().position(|x| x == h).unwrap_or_else(|| panic!("template {t}: no column {h}"));
        let l = col(c0 + k as u16);
        out.push_str(&match mode {
            '=' => format!("${l}${first}:${l}${last}"),
            '^' => format!("${l}${first}:${l}{r}"),
            '<' => format!("${l}${}:${l}{}", first.min(r - 1), r - 1),
            _ => format!("${l}{r}"),
        });
        rest = &rest[j + if whole { 2 } else { 1 }..];
    }
    out.push_str(rest);
    out.replace("{r}", &r.to_string()).replace("{p}", &p.to_string())
}

/// Write an Excel table `name` with header row `hr` at column `c0` and `n` formula rows.
pub fn table(
    ws: &mut Worksheet,
    sheet: &'static str,
    name: &str,
    hr: u32,
    c0: u16,
    cols: &[TCol],
    n: u32,
) -> Result<Placed> {
    let heads: Vec<String> = cols.iter().map(|x| x.head.to_string()).collect();
    let hs: Vec<&str> = heads.iter().map(String::as_str).collect();
    add_table(ws, name, hr, c0, &hs, n as usize, TableStyle::Light9)?;
    let n = n.max(1);
    for i in 0..n {
        let r0 = hr + 1 + i;
        for (k, x) in cols.iter().enumerate() {
            if x.f.is_empty() {
                continue;
            }
            let f = expand(&heads, c0, &x.f, r0 + 1, i + 2, (hr + 2, hr + 1 + n));
            let cc = c0 + k as u16;
            if let Some(stripped) = f.strip_prefix('=') {
                let fo = Formula::new(format!("={stripped}"));
                match x.fmt {
                    Some(fm) => ws.write_formula_with_format(r0, cc, fo, fm).map(|_| ())?,
                    None => ws.write_formula(r0, cc, fo).map(|_| ())?,
                }
            } else {
                match (x.f.parse::<f64>(), x.fmt) {
                    (Ok(v), Some(fm)) => ws.write_number_with_format(r0, cc, v, fm).map(|_| ())?,
                    (Ok(v), None) => ws.write_number(r0, cc, v).map(|_| ())?,
                    (Err(_), _) => ws.write_string(r0, cc, &f).map(|_| ())?,
                }
            }
        }
    }
    for (k, x) in cols.iter().enumerate() {
        ws.set_column_width(c0 + k as u16, (x.head.len() as f64 + 3.0).clamp(9.0, 24.0))?;
    }
    Ok(Placed { sheet, heads, c0, hr, first: hr + 1, last: hr + n })
}

/// Title, disclaimer and one line per idea; returns the first free row (after a blank one).
pub fn head(ws: &mut Worksheet, cx: &Cx, title: &str, lines: &[&str]) -> Result<u32> {
    ws.write_string_with_format(0, 0, title, &cx.f.title)?;
    theme::disclaimer(ws, 1, cx.f)?;
    for (i, l) in lines.iter().enumerate() {
        ws.write_string_with_format(2 + i as u32, 0, *l, &cx.f.note)?;
    }
    Ok(3 + lines.len() as u32)
}

/// A neutral pattern-flag banner ([DX-RISK] ideas).
pub fn dx_banner(ws: &mut Worksheet, row: u32, pattern: &str, cx: &Cx) -> Result<()> {
    ws.write_string_with_format(row, 0, theme::dx_risk(pattern), &cx.f.warn)?;
    Ok(())
}

/// A section heading.
pub fn section(ws: &mut Worksheet, row: u32, text: &str, cx: &Cx) -> Result<()> {
    ws.write_string_with_format(row, 0, text, &cx.f.section)?;
    Ok(())
}

/// Text formula wrapping a dx pattern: `IF(cond, "<neutral wording>", "")`.
pub fn dx_if(cond: &str, pattern: &str) -> String {
    format!("=IF({cond},\"{}\",\"\")", theme::dx_risk(pattern).replace('"', "'"))
}

/// Line chart over a view table: `cat` on the x axis, one series per column.
/// Columns whose header starts with `line` are drawn dashed (reference lines).
pub fn line_chart(t: &Placed, title: &str, cat: &str, series: &[&str], y: &str) -> Chart {
    let mut ch = Chart::new(ChartType::Line);
    for s in series {
        let ser = ch.add_series().set_name(*s).set_categories(t.area(cat)).set_values(t.area(s));
        if s.starts_with("line") {
            ser.set_format(
                rust_xlsxwriter::ChartFormat::new().set_line(ChartLine::new().set_dash_type(ChartLineDashType::Dash)),
            );
        } else {
            ser.set_marker(ChartMarker::new().set_automatic());
        }
    }
    ch.title().set_name(title);
    ch.y_axis().set_name(y);
    ch.x_axis().set_num_format(theme::nf::DATE);
    ch.set_width(620).set_height(300);
    ch
}

/// XY scatter of `y` against `x` with a linear trendline.
pub fn scatter(t: &Placed, title: &str, x: &str, y: &str, trend: bool) -> Chart {
    let mut ch = Chart::new(ChartType::Scatter);
    let s = ch.add_series().set_name(y).set_categories(t.area(x)).set_values(t.area(y));
    if trend {
        s.set_trendline(rust_xlsxwriter::ChartTrendline::new().set_type(rust_xlsxwriter::ChartTrendlineType::Linear));
    }
    ch.title().set_name(title);
    ch.x_axis().set_name(x);
    ch.y_axis().set_name(y);
    ch.legend().set_hidden();
    ch.set_width(480).set_height(300);
    ch
}

/// Publish a `K_CLN_*` KPI.
pub fn kpi(cx: &Cx, name: &str, label: &str, formula: &str, fmt: Option<&str>) -> Result<()> {
    cx.shared.kpis.borrow_mut().publish(WS, name, label, Some(&format!("IFERROR({formula},\"n/a\")")), fmt)?;
    Ok(())
}

/// Define a sheet-level name at a cell of this sheet.
pub fn name_cell(cx: &Cx, name: &str, sheet: &str, row: u32, c: u16) -> Result<()> {
    cx.shared.names.borrow_mut().define(name, &crate::infra::names::cell_ref(sheet, row, c), "cln")
}

/// Fill the cells of column `h` that contain `text`.
pub fn cf_text(ws: &mut Worksheet, t: &Placed, h: &str, text: &str, fill: &str) -> Result<()> {
    let c = t.col(h);
    let tl = format!("{}{}", col(c), t.first + 1);
    theme::cf_formula(ws, t.first, c, t.last, c, &format!("=ISNUMBER(SEARCH(\"{text}\",{tl}))"), fill)
}

/// An empty input table (one blank row; Excel grows it as rows are typed) read through
/// structured references (`tblX[col]`), so formulas keep up with the rows added.
pub fn input_table(ws: &mut Worksheet, cx: &Cx, name: &str, hr: u32, c0: u16, heads: &[&str]) -> Result<()> {
    add_table(ws, name, hr, c0, heads, 1, TableStyle::Light1)?;
    for i in 0..heads.len() as u16 {
        ws.write_blank(hr + 1, c0 + i, &cx.f.input)?;
    }
    Ok(())
}
